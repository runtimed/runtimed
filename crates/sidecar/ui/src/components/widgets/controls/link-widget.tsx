"use client";

/**
 * Link widgets — frontend-only property synchronization.
 *
 * LinkModel: bidirectional sync (widgets.jslink)
 * DirectionalLinkModel: one-way sync (widgets.jsdlink)
 *
 * These are invisible widgets that synchronize property values between
 * two other widgets entirely on the frontend, without kernel round-trips.
 * They render no visible UI (return null).
 */

import { useEffect } from "react";
import type { WidgetComponentProps } from "../widget-registry";
import { parseModelRef } from "../widget-store";
import {
  useWidgetModelValue,
  useWidgetStoreRequired,
} from "../widget-store-context";

/**
 * Parse a link source/target tuple from widget state.
 * The state arrives as: ["IPY_MODEL_<id>", "attribute_name"]
 * Returns [modelId, attrName] or null if malformed.
 */
function parseLinkTarget(tuple: unknown): [string, string] | null {
  if (
    !Array.isArray(tuple) ||
    tuple.length !== 2 ||
    typeof tuple[0] !== "string" ||
    typeof tuple[1] !== "string"
  ) {
    return null;
  }
  const modelId = parseModelRef(tuple[0]);
  if (!modelId) return null;
  return [modelId, tuple[1]];
}

/**
 * DirectionalLinkWidget — one-way property synchronization (source → target).
 *
 * When the source widget's attribute changes, the target widget's attribute
 * is updated to match. Changes to the target do NOT propagate back.
 */
export function DirectionalLinkWidget({ modelId }: WidgetComponentProps) {
  const { store } = useWidgetStoreRequired();

  const source = useWidgetModelValue<[string, string]>(modelId, "source");
  const target = useWidgetModelValue<[string, string]>(modelId, "target");

  useEffect(() => {
    const src = parseLinkTarget(source);
    const tgt = parseLinkTarget(target);
    if (!src || !tgt) return;

    const [sourceModelId, sourceAttr] = src;
    const [targetModelId, targetAttr] = tgt;

    let keyUnsub: (() => void) | undefined;
    let globalUnsub: (() => void) | undefined;
    let isSetUp = false;

    function setup() {
      if (isSetUp) return;
      if (!store.getModel(sourceModelId) || !store.getModel(targetModelId)) {
        return;
      }
      isSetUp = true;

      // Initial sync: read source value, write to target
      const sourceModel = store.getModel(sourceModelId);
      if (sourceModel) {
        const currentValue = sourceModel.state[sourceAttr];
        if (currentValue !== undefined) {
          store.updateModel(targetModelId, { [targetAttr]: currentValue });
        }
      }

      // Subscribe to source changes, propagate to target
      keyUnsub = store.subscribeToKey(sourceModelId, sourceAttr, (newValue) => {
        store.updateModel(targetModelId, { [targetAttr]: newValue });
      });

      // Clean up global listener once setup is complete
      if (globalUnsub) {
        globalUnsub();
        globalUnsub = undefined;
      }
    }

    // Try immediate setup
    setup();

    // If models aren't ready yet, wait via global subscription
    if (!isSetUp) {
      globalUnsub = store.subscribe(() => setup());
    }

    return () => {
      globalUnsub?.();
      keyUnsub?.();
    };
  }, [store, source, target]);

  return null;
}

/**
 * LinkWidget — bidirectional property synchronization (source ↔ target).
 *
 * Changes to either widget's attribute propagate to the other.
 * Uses a synchronous guard to prevent infinite loops (store subscriptions
 * fire synchronously during updateModel).
 */
export function LinkWidget({ modelId }: WidgetComponentProps) {
  const { store } = useWidgetStoreRequired();

  const source = useWidgetModelValue<[string, string]>(modelId, "source");
  const target = useWidgetModelValue<[string, string]>(modelId, "target");

  useEffect(() => {
    const src = parseLinkTarget(source);
    const tgt = parseLinkTarget(target);
    if (!src || !tgt) return;

    const [sourceModelId, sourceAttr] = src;
    const [targetModelId, targetAttr] = tgt;

    let keyUnsubs: (() => void)[] = [];
    let globalUnsub: (() => void) | undefined;
    let isSetUp = false;
    let isSyncing = false;

    function setup() {
      if (isSetUp) return;
      if (!store.getModel(sourceModelId) || !store.getModel(targetModelId)) {
        return;
      }
      isSetUp = true;

      // Initial sync: source → target
      const sourceModel = store.getModel(sourceModelId);
      if (sourceModel) {
        const currentValue = sourceModel.state[sourceAttr];
        if (currentValue !== undefined) {
          isSyncing = true;
          store.updateModel(targetModelId, { [targetAttr]: currentValue });
          isSyncing = false;
        }
      }

      // Source → Target
      keyUnsubs.push(
        store.subscribeToKey(sourceModelId, sourceAttr, (newValue) => {
          if (isSyncing) return;
          isSyncing = true;
          store.updateModel(targetModelId, { [targetAttr]: newValue });
          isSyncing = false;
        }),
      );

      // Target → Source
      keyUnsubs.push(
        store.subscribeToKey(targetModelId, targetAttr, (newValue) => {
          if (isSyncing) return;
          isSyncing = true;
          store.updateModel(sourceModelId, { [sourceAttr]: newValue });
          isSyncing = false;
        }),
      );

      // Clean up global listener once setup is complete
      if (globalUnsub) {
        globalUnsub();
        globalUnsub = undefined;
      }
    }

    // Try immediate setup
    setup();

    // If models aren't ready yet, wait via global subscription
    if (!isSetUp) {
      globalUnsub = store.subscribe(() => setup());
    }

    return () => {
      globalUnsub?.();
      keyUnsubs.forEach((unsub) => unsub());
    };
  }, [store, source, target]);

  return null;
}
