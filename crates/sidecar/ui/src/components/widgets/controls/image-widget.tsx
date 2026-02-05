"use client";

/**
 * Image widget - displays images from the kernel.
 *
 * Maps to ipywidgets ImageModel.
 */

import { cn } from "@/lib/utils";
import { Label } from "@/components/ui/label";
import type { WidgetComponentProps } from "../widget-registry";
import { useWidgetModelValue } from "../widget-store-context";

export function ImageWidget({ modelId, className }: WidgetComponentProps) {
  const value = useWidgetModelValue<string>(modelId, "value") ?? "";
  const format = useWidgetModelValue<string>(modelId, "format") ?? "png";
  const width = useWidgetModelValue<string>(modelId, "width") ?? "";
  const height = useWidgetModelValue<string>(modelId, "height") ?? "";
  const description = useWidgetModelValue<string>(modelId, "description");

  if (!value) {
    return null;
  }

  // Construct data URL from base64 value
  // If already a data URL or regular URL, use as-is
  const src =
    value.startsWith("data:") ||
    value.startsWith("http://") ||
    value.startsWith("https://") ||
    value.startsWith("/")
      ? value
      : `data:image/${format};base64,${value}`;

  // Build style object for width/height
  const style: React.CSSProperties = {};
  if (width) style.width = width;
  if (height) style.height = height;

  return (
    <div
      className={cn("inline-flex items-start gap-3", className)}
      data-widget-id={modelId}
      data-widget-type="Image"
    >
      {description && (
        <Label className="shrink-0 pt-1 text-sm">{description}</Label>
      )}
      <img
        src={src}
        alt={description || "Widget image"}
        className="block max-w-full h-auto"
        style={{ objectFit: "contain", ...style }}
      />
    </div>
  );
}

export default ImageWidget;
