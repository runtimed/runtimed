"use client";

/**
 * ipycanvas Canvas widget.
 *
 * Renders an HTML <canvas> element and processes drawing commands sent from
 * Python via the ipycanvas binary protocol. Drawing commands are sent to a
 * singleton CanvasManagerModel which never renders (_view_name: null).
 * Store-level routing (ensureManagerRouting) subscribes to the manager's
 * messages, parses switchCanvas targets, and re-emits to each canvas's
 * comm_id — no React component mounting required.
 *
 * @see https://ipycanvas.readthedocs.io/
 */

import { useCallback, useEffect, useRef } from "react";
import { cn } from "@/lib/utils";
import type { WidgetComponentProps } from "../widget-registry";
import {
  parseModelRef,
  useWidgetModelValue,
  useWidgetStoreRequired,
  type WidgetStore,
} from "../widget-store-context";
import { COMMANDS, getTypedArray, processCommands } from "./ipycanvas-commands";

// === Store-Level Manager Routing ===

// Ref-counted subscriptions for CanvasManagerModel message routing.
// CanvasManagerModel is a headless widget (_view_name: null) — it never
// renders, so routing can't live in a React component. Each CanvasWidget
// that references a manager calls ensureManagerRouting() in its effect.
const managerRouting = new Map<
  string,
  { refCount: number; unsubscribe: () => void }
>();

/**
 * Walk a command structure and collect switchCanvas target IDs.
 * Calls setTarget as a side effect so subsequent messages without
 * switchCanvas route to the last known target.
 */
function collectSwitchCanvasTargets(
  commands: unknown,
  targets: Set<string>,
  setTarget: (id: string) => void,
): void {
  if (!Array.isArray(commands) || commands.length === 0) return;

  if (Array.isArray(commands[0])) {
    for (const sub of commands) {
      collectSwitchCanvasTargets(sub, targets, setTarget);
    }
  } else {
    const cmdIndex = commands[0] as number;
    if (COMMANDS[cmdIndex] === "switchCanvas") {
      const args = commands[1] as string[] | undefined;
      const ref = args?.[0] ?? "";
      const targetId = ref.startsWith("IPY_MODEL_") ? ref.slice(10) : ref;
      setTarget(targetId);
      targets.add(targetId);
    }
  }
}

/**
 * Ensure store-level routing is active for a CanvasManagerModel.
 *
 * Subscribes to the manager's custom messages, parses switchCanvas to
 * determine target canvases, and re-emits to each target's comm_id.
 * Ref-counted: first CanvasWidget creates the subscription, last tears it down.
 */
function ensureManagerRouting(
  store: WidgetStore,
  managerId: string,
): () => void {
  const existing = managerRouting.get(managerId);
  if (existing) {
    existing.refCount++;
    return () => {
      existing.refCount--;
      if (existing.refCount === 0) {
        existing.unsubscribe();
        managerRouting.delete(managerId);
      }
    };
  }

  let currentTarget: string | null = null;

  const unsubscribe = store.subscribeToCustomMessage(
    managerId,
    (content, buffers) => {
      if (!buffers || buffers.length === 0) return;

      try {
        const metadata = content as { dtype: string };
        const typedArray = getTypedArray(buffers[0], metadata);
        const jsonStr = new TextDecoder("utf-8").decode(typedArray);
        const commands = JSON.parse(jsonStr);

        const targets = new Set<string>();
        collectSwitchCanvasTargets(commands, targets, (id) => {
          currentTarget = id;
        });

        if (targets.size === 0 && currentTarget) {
          targets.add(currentTarget);
        }

        const rawBuffers = buffers.map((dv) => dv.buffer as ArrayBuffer);
        for (const targetId of targets) {
          store.emitCustomMessage(targetId, content, rawBuffers);
        }
      } catch (err) {
        console.warn("[ipycanvas] Manager routing error:", err);
      }
    },
  );

  managerRouting.set(managerId, { refCount: 1, unsubscribe });

  return () => {
    const state = managerRouting.get(managerId);
    if (!state) return;
    state.refCount--;
    if (state.refCount === 0) {
      state.unsubscribe();
      managerRouting.delete(managerId);
    }
  };
}

// === CanvasWidget ===

export function CanvasWidget({ modelId, className }: WidgetComponentProps) {
  const { store, sendCustom } = useWidgetStoreRequired();

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const ctxRef = useRef<CanvasRenderingContext2D | null>(null);
  // Track an async command processing chain so commands execute in order
  const processingRef = useRef<Promise<void>>(Promise.resolve());

  const width = useWidgetModelValue<number>(modelId, "width") ?? 200;
  const height = useWidgetModelValue<number>(modelId, "height") ?? 200;
  const sendClientReady =
    useWidgetModelValue<boolean>(modelId, "_send_client_ready_event") ?? true;
  const imageData = useWidgetModelValue<Uint8ClampedArray | null>(
    modelId,
    "image_data",
  );
  const canvasManagerRef =
    useWidgetModelValue<string>(modelId, "_canvas_manager") ?? null;

  // Initialize 2D context
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    ctxRef.current = canvas.getContext("2d");
  }, []);

  // Draw image_data when it changes (for restoring canvas state)
  useEffect(() => {
    if (!imageData || !ctxRef.current || !canvasRef.current) return;

    const blob = new Blob([imageData.buffer as ArrayBuffer]);
    const url = URL.createObjectURL(blob);
    const img = new Image();
    img.onload = () => {
      ctxRef.current?.drawImage(img, 0, 0);
      URL.revokeObjectURL(url);
    };
    img.src = url;
  }, [imageData]);

  // Set up manager routing + subscribe to own messages + send client_ready.
  // Order matters: routing first so manager messages get re-emitted to our
  // comm_id, then subscribe so we receive them, then client_ready.
  useEffect(() => {
    // Set up manager routing (ref-counted, no-ops if already active)
    let cleanupRouting: (() => void) | undefined;
    if (canvasManagerRef) {
      const managerId = parseModelRef(canvasManagerRef);
      if (managerId) {
        cleanupRouting = ensureManagerRouting(store, managerId);
      }
    }

    // Subscribe to messages routed to this canvas's comm_id
    const unsubscribe = store.subscribeToCustomMessage(
      modelId,
      (content, buffers) => {
        const canvas = canvasRef.current;
        const ctx = ctxRef.current;
        if (!canvas || !ctx || !buffers || buffers.length === 0) return;

        // Chain command processing to maintain order
        processingRef.current = processingRef.current.then(async () => {
          try {
            // First buffer is the JSON-encoded commands
            const metadata = content as { dtype: string };
            const typedArray = getTypedArray(buffers[0], metadata);
            const jsonStr = new TextDecoder("utf-8").decode(typedArray);
            const commands = JSON.parse(jsonStr);

            // Remaining buffers are binary data for batch operations
            const dataBuffers = buffers.slice(1);

            await processCommands(
              ctx,
              commands,
              dataBuffers,
              canvas,
              modelId,
              true,
            );
          } catch (err) {
            console.warn("[ipycanvas] Error processing commands:", err);
          }
        });
      },
    );

    if (sendClientReady) {
      sendCustom(modelId, { event: "client_ready" });
    }

    return () => {
      unsubscribe();
      cleanupRouting?.();
    };
  }, [store, modelId, canvasManagerRef, sendClientReady, sendCustom]);

  // Mouse event helpers
  const getCoordinates = useCallback(
    (event: React.MouseEvent<HTMLCanvasElement>) => {
      const canvas = canvasRef.current;
      if (!canvas) return { x: 0, y: 0 };
      const rect = canvas.getBoundingClientRect();
      return {
        x: (canvas.width * (event.clientX - rect.left)) / rect.width,
        y: (canvas.height * (event.clientY - rect.top)) / rect.height,
      };
    },
    [],
  );

  const onMouseMove = useCallback(
    (event: React.MouseEvent<HTMLCanvasElement>) => {
      sendCustom(modelId, { event: "mouse_move", ...getCoordinates(event) });
    },
    [modelId, sendCustom, getCoordinates],
  );

  const onMouseDown = useCallback(
    (event: React.MouseEvent<HTMLCanvasElement>) => {
      canvasRef.current?.focus();
      sendCustom(modelId, { event: "mouse_down", ...getCoordinates(event) });
    },
    [modelId, sendCustom, getCoordinates],
  );

  const onMouseUp = useCallback(
    (event: React.MouseEvent<HTMLCanvasElement>) => {
      sendCustom(modelId, { event: "mouse_up", ...getCoordinates(event) });
    },
    [modelId, sendCustom, getCoordinates],
  );

  const onMouseOut = useCallback(
    (event: React.MouseEvent<HTMLCanvasElement>) => {
      sendCustom(modelId, { event: "mouse_out", ...getCoordinates(event) });
    },
    [modelId, sendCustom, getCoordinates],
  );

  const onWheel = useCallback(
    (event: React.WheelEvent<HTMLCanvasElement>) => {
      sendCustom(modelId, {
        event: "mouse_wheel",
        x: event.deltaX,
        y: event.deltaY,
      });
      event.preventDefault();
    },
    [modelId, sendCustom],
  );

  const onKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLCanvasElement>) => {
      event.preventDefault();
      event.stopPropagation();
      sendCustom(modelId, {
        event: "key_down",
        key: event.key,
        shift_key: event.shiftKey,
        ctrl_key: event.ctrlKey,
        meta_key: event.metaKey,
      });
    },
    [modelId, sendCustom],
  );

  return (
    <canvas
      ref={canvasRef}
      width={width}
      height={height}
      tabIndex={0}
      className={cn("block", className)}
      data-widget-id={modelId}
      data-widget-type="Canvas"
      onMouseMove={onMouseMove}
      onMouseDown={onMouseDown}
      onMouseUp={onMouseUp}
      onMouseOut={onMouseOut}
      onBlur={() => sendCustom(modelId, { event: "mouse_out", x: 0, y: 0 })}
      onWheel={onWheel}
      onKeyDown={onKeyDown}
    />
  );
}

// === CanvasManagerWidget ===

/**
 * Stub for CanvasManagerModel. The actual routing happens at the store
 * level via ensureManagerRouting(), set up by each CanvasWidget.
 * CanvasManagerModel has _view_name: null and is never rendered.
 */
export function CanvasManagerWidget(_props: WidgetComponentProps) {
  return null;
}
