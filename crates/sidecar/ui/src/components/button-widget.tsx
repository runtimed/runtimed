"use client";

/**
 * Button widget - renders a clickable button.
 *
 * Maps to ipywidgets ButtonModel.
 */

import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import type { WidgetComponentProps } from "@/lib/widget-registry";
import {
  useWidgetModelValue,
  useWidgetStoreRequired,
} from "@/lib/widget-store-context";

// Map ipywidgets button_style to shadcn variants
const STYLE_MAP: Record<
  string,
  "default" | "destructive" | "secondary" | "outline"
> = {
  primary: "default",
  success: "default", // Could use a custom green variant
  info: "secondary",
  warning: "secondary", // Could use a custom yellow variant
  danger: "destructive",
  "": "outline",
};

export function ButtonWidget({ modelId, className }: WidgetComponentProps) {
  const { sendCustom } = useWidgetStoreRequired();

  // Subscribe to individual state keys
  const description = useWidgetModelValue<string>(modelId, "description") ?? "";
  const disabled = useWidgetModelValue<boolean>(modelId, "disabled") ?? false;
  const buttonStyle =
    useWidgetModelValue<string>(modelId, "button_style") ?? "";
  const icon = useWidgetModelValue<string>(modelId, "icon");
  const tooltip = useWidgetModelValue<string>(modelId, "tooltip");

  const handleClick = () => {
    // Send click event to kernel
    sendCustom(modelId, { event: "click" });
  };

  const variant = STYLE_MAP[buttonStyle] ?? "outline";

  return (
    <Button
      variant={variant}
      disabled={disabled}
      onClick={handleClick}
      title={tooltip}
      className={cn(className)}
      data-widget-id={modelId}
      data-widget-type="Button"
    >
      {icon && <span className="mr-1">{icon}</span>}
      {description || "Button"}
    </Button>
  );
}

export default ButtonWidget;
