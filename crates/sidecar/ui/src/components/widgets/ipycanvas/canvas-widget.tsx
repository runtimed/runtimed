"use client";

/**
 * ipycanvas Canvas widget.
 *
 * Renders an HTML <canvas> element and processes drawing commands sent from
 * Python via the ipycanvas binary protocol. Commands arrive as custom messages
 * on the CanvasManagerModel and are executed on the canvas 2D context.
 *
 * @see https://ipycanvas.readthedocs.io/
 */

import { useCallback, useEffect, useRef } from "react";
import { cn } from "@/lib/utils";
import type { WidgetComponentProps } from "../widget-registry";
import { parseModelRef } from "../widget-store";
import {
  useWidgetModelValue,
  useWidgetStoreRequired,
} from "../widget-store-context";
import { getTypedArray, processCommands } from "./ipycanvas-commands";

// === CanvasWidget ===

export function CanvasWidget({ modelId, className }: WidgetComponentProps) {
  const { store, sendCustom } = useWidgetStoreRequired();

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const ctxRef = useRef<CanvasRenderingContext2D | null>(null);
  // Track an async command processing chain so commands execute in order
  const processingRef = useRef<Promise<void>>(Promise.resolve());
  // Track which canvas the manager last targeted via switchCanvas.
  // In non-caching mode (the default, without hold_canvas()), each drawing
  // command is a separate custom message. We need to persist the switchCanvas
  // target across messages so each canvas only processes its own commands.
  const activeCanvasRef = useRef<string | null>(null);

  const width = useWidgetModelValue<number>(modelId, "width") ?? 200;
  const height = useWidgetModelValue<number>(modelId, "height") ?? 200;
  const canvasManagerRef =
    useWidgetModelValue<string>(modelId, "_canvas_manager") ?? null;
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

  // Subscribe to custom messages on the CanvasManagerModel, then send client_ready.
  // These must be in the same effect so the subscription is active before
  // Python replays drawing commands in response to client_ready.
  useEffect(() => {
    if (!canvasManagerRef) return;

    const managerModelId = parseModelRef(canvasManagerRef);
    if (!managerModelId) return;

    // Subscribe FIRST
    const unsubscribe = store.subscribeToCustomMessage(
      managerModelId,
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

            // Determine if this canvas is the active target based on the
            // last switchCanvas we saw. Start inactive — a canvas should not
            // draw until switchCanvas explicitly targets it.
            const isActive = activeCanvasRef.current === modelId;

            const result = await processCommands(
              ctx,
              commands,
              dataBuffers,
              canvas,
              modelId,
              isActive,
            );

            // Persist switchCanvas target across messages
            if (result.switchedTo !== null) {
              activeCanvasRef.current = result.switchedTo;
            }
          } catch (err) {
            console.warn("[ipycanvas] Error processing commands:", err);
          }
        });
      },
    );

    // THEN send client_ready — subscription is now active so
    // replayed commands from Python will be received
    if (sendClientReady) {
      sendCustom(modelId, { event: "client_ready" });
    }

    return unsubscribe;
  }, [canvasManagerRef, store, modelId, sendClientReady, sendCustom]);

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
 * Headless widget for CanvasManagerModel.
 * The manager coordinates drawing commands but has no visual representation.
 * Command processing is handled via custom message subscriptions from CanvasWidget.
 */
export function CanvasManagerWidget(_props: WidgetComponentProps) {
  return null;
}
