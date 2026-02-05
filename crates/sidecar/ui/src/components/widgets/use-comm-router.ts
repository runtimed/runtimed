"use client";

/**
 * Comm Protocol Router Hook
 *
 * Handles routing of Jupyter comm messages between the kernel and widget store.
 * Provides both inbound message handling and outbound message construction.
 *
 * @see https://jupyter-widgets.readthedocs.io/en/latest/examples/Widget%20Low%20Level.html
 */

import { useCallback } from "react";
import type { WidgetStore } from "./widget-store";
import { applyBufferPaths } from "./buffer-utils";

// === Message Types ===

export interface JupyterMessageHeader {
  msg_id: string;
  msg_type: string;
  username?: string;
  session?: string;
  date?: string;
  version?: string;
}

export interface JupyterCommMessage {
  header: JupyterMessageHeader;
  parent_header?: JupyterMessageHeader | null;
  metadata?: Record<string, unknown>;
  content: {
    comm_id?: string;
    target_name?: string;
    data?: {
      state?: Record<string, unknown>;
      method?: string;
      content?: Record<string, unknown>;
      buffer_paths?: string[][];
      [key: string]: unknown;
    };
  };
  buffers?: ArrayBuffer[];
  channel?: string | null;
}

/**
 * Function type for sending messages to the kernel.
 */
export type SendMessage = (msg: JupyterCommMessage) => void;

// === Hook Types ===

export interface UseCommRouterOptions {
  /** Function to send messages to the kernel */
  sendMessage: SendMessage;
  /** Widget store instance */
  store: WidgetStore;
}

export interface UseCommRouterReturn {
  /** Handle incoming Jupyter comm messages */
  handleMessage: (msg: JupyterCommMessage) => void;
  /** Send a state update to the kernel */
  sendUpdate: (
    commId: string,
    state: Record<string, unknown>,
    buffers?: ArrayBuffer[],
  ) => void;
  /** Send a custom message to the kernel */
  sendCustom: (
    commId: string,
    content: Record<string, unknown>,
    buffers?: ArrayBuffer[],
  ) => void;
  /** Close a comm channel */
  closeComm: (commId: string) => void;
}

// === Message Construction Helpers ===

// Session ID for this sidecar instance (stable across messages)
const SESSION_ID = crypto.randomUUID();

/**
 * Create a complete Jupyter message header.
 */
function createHeader(msgType: string): JupyterMessageHeader {
  return {
    msg_id: crypto.randomUUID(),
    msg_type: msgType,
    username: "sidecar",
    session: SESSION_ID,
    date: new Date().toISOString(),
    version: "5.3",
  };
}

/**
 * Create a comm_msg for state updates.
 */
function createUpdateMessage(
  commId: string,
  state: Record<string, unknown>,
  buffers?: ArrayBuffer[],
): JupyterCommMessage {
  return {
    header: createHeader("comm_msg"),
    parent_header: null,
    metadata: {},
    content: {
      comm_id: commId,
      data: {
        method: "update",
        state,
        buffer_paths: [],
      },
    },
    buffers: buffers ?? [],
    channel: "shell",
  };
}

/**
 * Create a comm_msg for custom messages.
 */
function createCustomMessage(
  commId: string,
  content: Record<string, unknown>,
  buffers?: ArrayBuffer[],
): JupyterCommMessage {
  return {
    header: createHeader("comm_msg"),
    parent_header: null,
    metadata: {},
    content: {
      comm_id: commId,
      data: {
        method: "custom",
        content,
      },
    },
    buffers: buffers ?? [],
    channel: "shell",
  };
}

/**
 * Create a comm_close message.
 */
function createCloseMessage(commId: string): JupyterCommMessage {
  return {
    header: createHeader("comm_close"),
    parent_header: null,
    metadata: {},
    content: {
      comm_id: commId,
    },
    buffers: [],
    channel: "shell",
  };
}

// === Hook Implementation ===

/**
 * Hook for routing Jupyter comm protocol messages.
 *
 * Handles:
 * - Inbound: comm_open, comm_msg (update/custom), comm_close
 * - Outbound: sendUpdate, sendCustom, closeComm
 *
 * @example
 * const { handleMessage, sendUpdate, sendCustom, closeComm } = useCommRouter({
 *   sendMessage: (msg) => kernel.send(msg),
 *   store: widgetStore,
 * });
 *
 * // Route incoming messages
 * kernel.onMessage((msg) => handleMessage(msg));
 *
 * // Send updates back to kernel
 * sendUpdate(commId, { value: 42 });
 */
export function useCommRouter({
  sendMessage,
  store,
}: UseCommRouterOptions): UseCommRouterReturn {
  /**
   * Handle incoming Jupyter comm messages.
   * Routes to appropriate store methods based on message type.
   */
  const handleMessage = useCallback(
    (msg: JupyterCommMessage) => {
      const msgType = msg.header.msg_type;
      const commId = msg.content.comm_id;

      if (!commId) return;

      switch (msgType) {
        case "comm_open": {
          // Get state and apply buffer paths if present
          let state = msg.content.data?.state || {};
          const bufferPaths = msg.content.data?.buffer_paths;

          if (bufferPaths && msg.buffers) {
            state = { ...state };
            applyBufferPaths(state, bufferPaths, msg.buffers);
          }

          store.createModel(commId, state, msg.buffers);
          break;
        }

        case "comm_msg": {
          const data = msg.content.data;
          const method = data?.method;

          if (method === "update" && data?.state) {
            // Apply buffer paths to state update
            let state = data.state;
            const bufferPaths = data.buffer_paths;

            if (bufferPaths && msg.buffers) {
              state = { ...state };
              applyBufferPaths(state, bufferPaths, msg.buffers);
            }

            store.updateModel(commId, state, msg.buffers);
          } else if (method === "custom") {
            // Dispatch custom message to widget handlers
            const content = (data?.content as Record<string, unknown>) || {};
            store.emitCustomMessage(commId, content, msg.buffers);
          }
          break;
        }

        case "comm_close": {
          store.deleteModel(commId);
          break;
        }
      }
    },
    [store],
  );

  /**
   * Send a state update to the kernel.
   * Also applies optimistic update to local store for immediate UI response.
   */
  const sendUpdate = useCallback(
    (
      commId: string,
      state: Record<string, unknown>,
      buffers?: ArrayBuffer[],
    ) => {
      // Optimistic update: apply locally first for responsive UI
      store.updateModel(commId, state, buffers);
      // Then send to kernel
      sendMessage(createUpdateMessage(commId, state, buffers));
    },
    [sendMessage, store],
  );

  /**
   * Send a custom message to the kernel.
   */
  const sendCustom = useCallback(
    (
      commId: string,
      content: Record<string, unknown>,
      buffers?: ArrayBuffer[],
    ) => {
      sendMessage(createCustomMessage(commId, content, buffers));
    },
    [sendMessage],
  );

  /**
   * Close a comm channel.
   * Sends comm_close to kernel and removes model from store.
   */
  const closeComm = useCallback(
    (commId: string) => {
      sendMessage(createCloseMessage(commId));
      store.deleteModel(commId);
    },
    [sendMessage, store],
  );

  return {
    handleMessage,
    sendUpdate,
    sendCustom,
    closeComm,
  };
}
