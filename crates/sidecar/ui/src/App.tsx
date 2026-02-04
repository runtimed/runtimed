import { useState, useEffect, useCallback, useRef } from "react";
import { MediaRouter } from "@/components/media-router";
import {
  AnsiStreamOutput,
  AnsiErrorOutput,
} from "@/components/ansi-output";
import type {
  JupyterMessage,
  JupyterOutput,
  MimeBundle,
  MimeMetadata,
} from "./types";
import {
  isDisplayData,
  isExecuteResult,
  isStream,
  isError,
  isClearOutput,
} from "./types";
import { cn } from "@/lib/utils";

interface OutputCellProps {
  output: JupyterOutput;
  index: number;
}

function OutputCell({ output, index }: OutputCellProps) {
  if (output.output_type === "stream") {
    return (
      <AnsiStreamOutput
        text={output.text}
        streamName={output.name}
        className="px-4"
      />
    );
  }

  if (output.output_type === "error") {
    return (
      <AnsiErrorOutput
        ename={output.ename}
        evalue={output.evalue}
        traceback={output.traceback}
        className="px-4"
      />
    );
  }

  // display_data or execute_result
  return (
    <div className="output-cell" data-index={index}>
      {output.execution_count != null && (
        <div className="px-4 py-1 text-xs text-muted-foreground font-mono">
          Out[{output.execution_count}]:
        </div>
      )}
      <div className="px-4">
        <MediaRouter
          data={output.data as Record<string, unknown>}
          metadata={output.metadata}
        />
      </div>
    </div>
  );
}

export default function App() {
  const [outputs, setOutputs] = useState<JupyterOutput[]>([]);
  const [kernelStatus, setKernelStatus] = useState<string>("unknown");
  const outputAreaRef = useRef<HTMLDivElement>(null);

  // Convert message to output format
  const messageToOutput = useCallback(
    (message: JupyterMessage): JupyterOutput | null => {
      if (isDisplayData(message)) {
        return {
          output_type: "display_data",
          data: message.content.data as MimeBundle,
          metadata: message.content.metadata as MimeMetadata,
        };
      }

      if (isExecuteResult(message)) {
        return {
          output_type: "execute_result",
          data: message.content.data as MimeBundle,
          metadata: message.content.metadata as MimeMetadata,
          execution_count: message.content.execution_count,
        };
      }

      if (isStream(message)) {
        return {
          output_type: "stream",
          name: message.content.name,
          text: message.content.text,
        };
      }

      if (isError(message)) {
        return {
          output_type: "error",
          ename: message.content.ename,
          evalue: message.content.evalue,
          traceback: message.content.traceback,
        };
      }

      return null;
    },
    []
  );

  // Handle incoming Jupyter messages
  const handleMessage = useCallback(
    (message: JupyterMessage) => {
      console.log("[sidecar] Received message:", message.header.msg_type, message);

      // Decode base64 buffers if present
      if (message.buffers && Array.isArray(message.buffers)) {
        message.buffers = message.buffers.map((b64) => {
          if (typeof b64 === "string") {
            const binary = atob(b64);
            const bytes = new Uint8Array(binary.length);
            for (let i = 0; i < binary.length; i++) {
              bytes[i] = binary.charCodeAt(i);
            }
            return bytes.buffer;
          }
          return b64;
        });
      }

      // Handle clear_output
      if (isClearOutput(message)) {
        if (message.content.wait) {
          // TODO: handle wait flag (clear on next output)
        } else {
          setOutputs([]);
        }
        return;
      }

      // Handle status updates
      if (message.header.msg_type === "status") {
        const status = (message as { content: { execution_state: string } }).content.execution_state;
        setKernelStatus(status);
        return;
      }

      // Convert to output and append
      const output = messageToOutput(message);
      if (output) {
        setOutputs((prev) => {
          // Merge consecutive stream outputs of the same type
          if (
            output.output_type === "stream" &&
            prev.length > 0
          ) {
            const lastOutput = prev[prev.length - 1];
            if (
              lastOutput.output_type === "stream" &&
              lastOutput.name === output.name
            ) {
              return [
                ...prev.slice(0, -1),
                {
                  ...lastOutput,
                  text: lastOutput.text + output.text,
                },
              ];
            }
          }
          return [...prev, output];
        });
      }
    },
    [messageToOutput]
  );

  // Register global message handler
  useEffect(() => {
    // @ts-expect-error - globalThis.onMessage is set by Rust
    globalThis.onMessage = handleMessage;

    return () => {
      // @ts-expect-error - cleanup
      delete globalThis.onMessage;
    };
  }, [handleMessage]);

  // Auto-scroll to bottom on new outputs
  useEffect(() => {
    if (outputAreaRef.current) {
      outputAreaRef.current.scrollTop = outputAreaRef.current.scrollHeight;
    }
  }, [outputs]);

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="sticky top-0 z-10 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="flex h-10 items-center justify-between px-4">
          <h1 className="text-sm font-medium">Kernel Sidecar</h1>
          <div className="flex items-center gap-2">
            <div
              className={cn(
                "h-2 w-2 rounded-full",
                kernelStatus === "idle" && "bg-green-500",
                kernelStatus === "busy" && "bg-amber-500",
                kernelStatus === "starting" && "bg-blue-500",
                kernelStatus === "unknown" && "bg-gray-400"
              )}
            />
            <span className="text-xs text-muted-foreground capitalize">
              {kernelStatus}
            </span>
          </div>
        </div>
      </header>

      {/* Output Area */}
      <main
        ref={outputAreaRef}
        className="max-w-4xl mx-auto py-4 space-y-2"
      >
        {outputs.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-20 text-muted-foreground">
            <p className="text-sm">Waiting for outputs...</p>
            <p className="text-xs mt-1">
              Execute code in your notebook to see results here
            </p>
          </div>
        ) : (
          outputs.map((output, index) => (
            <div
              key={index}
              className="rounded-lg border bg-card py-3 shadow-sm"
            >
              <OutputCell output={output} index={index} />
            </div>
          ))
        )}
      </main>
    </div>
  );
}
