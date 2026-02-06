/**
 * Store-level routing for ipycanvas CanvasManagerModel.
 *
 * CanvasManagerModel is a headless widget (_view_name: null) that receives
 * ALL drawing commands from Python. This module subscribes to each manager's
 * custom messages, parses switchCanvas targets, and re-emits to each target
 * canvas's comm_id — isolating canvases from each other.
 *
 * Same pattern as link-subscriptions.ts for LinkModel/DirectionalLinkModel.
 *
 * Usage:
 *   const cleanup = createCanvasManagerRouter(store);
 *   // ... later, to tear down all routing:
 *   cleanup();
 *
 * WidgetStoreProvider calls this automatically. For non-React integrations
 * (e.g. iframe isolation), call createCanvasManagerRouter directly.
 */

import { COMMANDS, getTypedArray } from "./ipycanvas/ipycanvas-commands";
import type { WidgetStore } from "./widget-store";

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
 * Set up message routing for a single CanvasManagerModel.
 * Returns a cleanup function to tear down the subscription.
 */
function setupManagerRouting(
  store: WidgetStore,
  managerId: string,
): () => void {
  let currentTarget: string | null = null;

  return store.subscribeToCustomMessage(managerId, (content, buffers) => {
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
  });
}

/**
 * Create a canvas manager router that monitors the store for
 * CanvasManagerModel widgets and routes their messages to target canvases.
 *
 * Returns a cleanup function that tears down all active routing.
 *
 * Called automatically by WidgetStoreProvider. For non-React integrations
 * (e.g. iframe isolation), call this directly after creating the store.
 */
export function createCanvasManagerRouter(store: WidgetStore): () => void {
  const activeRoutes = new Map<string, () => void>();
  let lastSize = -1;

  function scan() {
    const models = store.getSnapshot();

    if (models.size === lastSize) return;
    lastSize = models.size;

    models.forEach((model, id) => {
      if (activeRoutes.has(id)) return;

      if (model.modelName === "CanvasManagerModel") {
        activeRoutes.set(id, setupManagerRouting(store, id));
      }
    });

    for (const [id, cleanup] of activeRoutes) {
      if (!models.has(id)) {
        cleanup();
        activeRoutes.delete(id);
      }
    }
  }

  const unsubscribe = store.subscribe(scan);
  scan();

  return () => {
    unsubscribe();
    activeRoutes.forEach((cleanup) => cleanup());
    activeRoutes.clear();
  };
}
