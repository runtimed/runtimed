"use client";

/**
 * ipycanvas Canvas widget.
 *
 * Renders an HTML <canvas> element and processes drawing commands sent from
 * Python via the ipycanvas binary protocol. The CanvasManagerWidget receives
 * all commands, parses switchCanvas routing, and re-emits to each target
 * canvas's comm_id. Each CanvasWidget subscribes only to its own messages.
 *
 * @see https://ipycanvas.readthedocs.io/
 */

import { useCallback, useEffect, useRef } from "react";
import { cn } from "@/lib/utils";
import type { WidgetComponentProps } from "../widget-registry";
import {
  useWidgetModelValue,
  useWidgetStoreRequired,
} from "../widget-store-context";
import { COMMANDS, getTypedArray, processCommands } from "./ipycanvas-commands";

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

  // Subscribe to custom messages routed to this canvas by the CanvasManagerWidget,
  // then send client_ready. Subscribe first so replayed commands are received.
  useEffect(() => {
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

    return unsubscribe;
  }, [store, modelId, sendClientReady, sendCustom]);

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
 * Walk a command structure and collect switchCanvas target IDs.
 * Updates currentTargetRef as a side effect so subsequent messages
 * without switchCanvas route to the last known target.
 */
function collectSwitchCanvasTargets(
  commands: unknown,
  currentTargetRef: React.MutableRefObject<string | null>,
  targets: Set<string>,
): void {
  if (!Array.isArray(commands) || commands.length === 0) return;

  if (Array.isArray(commands[0])) {
    // Batch: array of commands
    for (const sub of commands) {
      collectSwitchCanvasTargets(sub, currentTargetRef, targets);
    }
  } else {
    // Single command: [cmdIndex, args?, nBuffers?]
    const cmdIndex = commands[0] as number;
    if (COMMANDS[cmdIndex] === "switchCanvas") {
      const args = commands[1] as string[] | undefined;
      const ref = args?.[0] ?? "";
      const targetId = ref.startsWith("IPY_MODEL_") ? ref.slice(10) : ref;
      currentTargetRef.current = targetId;
      targets.add(targetId);
    }
  }
}

/**
 * Dispatcher widget for CanvasManagerModel.
 *
 * The manager receives ALL drawing commands from Python, parses switchCanvas
 * to determine the target canvas, and re-emits each message to that canvas's
 * comm_id. This isolates canvases from each other — no shared routing state.
 */
export function CanvasManagerWidget({ modelId }: WidgetComponentProps) {
  const { store } = useWidgetStoreRequired();
  const currentTargetRef = useRef<string | null>(null);

  useEffect(() => {
    const unsubscribe = store.subscribeToCustomMessage(
      modelId,
      (content, buffers) => {
        if (!buffers || buffers.length === 0) return;

        try {
          // Parse commands to find switchCanvas targets
          const metadata = content as { dtype: string };
          const typedArray = getTypedArray(buffers[0], metadata);
          const jsonStr = new TextDecoder("utf-8").decode(typedArray);
          const commands = JSON.parse(jsonStr);

          // Walk commands, update currentTarget on switchCanvas,
          // collect all unique target IDs in this message
          const targets = new Set<string>();
          collectSwitchCanvasTargets(commands, currentTargetRef, targets);

          // If no switchCanvas in this message, route to current target
          if (targets.size === 0 && currentTargetRef.current) {
            targets.add(currentTargetRef.current);
          }

          // Re-emit to each target canvas's comm_id
          const rawBuffers = buffers.map((dv) => dv.buffer as ArrayBuffer);
          for (const targetId of targets) {
            store.emitCustomMessage(targetId, content, rawBuffers);
          }
        } catch (err) {
          console.warn("[ipycanvas] Manager routing error:", err);
        }
      },
    );

    return unsubscribe;
  }, [store, modelId]);

  return null;
}
