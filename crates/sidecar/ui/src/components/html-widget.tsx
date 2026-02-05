"use client";

/**
 * HTML widget - renders arbitrary HTML content.
 *
 * Maps to ipywidgets HTMLModel.
 */

import { cn } from "@/lib/utils";
import { Label } from "@/components/ui/label";
import type { WidgetComponentProps } from "@/lib/widget-registry";
import { useWidgetModelValue } from "@/lib/widget-store-context";

export function HTMLWidget({ modelId, className }: WidgetComponentProps) {
  const value = useWidgetModelValue<string>(modelId, "value") ?? "";
  const description = useWidgetModelValue<string>(modelId, "description");
  const placeholder = useWidgetModelValue<string>(modelId, "placeholder");

  // Show placeholder if value is empty
  const displayValue = value || placeholder || "";

  return (
    <div
      className={cn("flex items-start gap-3", className)}
      data-widget-id={modelId}
      data-widget-type="HTML"
    >
      {description && <Label className="shrink-0 text-sm">{description}</Label>}
      <div
        className="widget-html-content"
        dangerouslySetInnerHTML={{ __html: displayValue }}
      />
    </div>
  );
}

export default HTMLWidget;
