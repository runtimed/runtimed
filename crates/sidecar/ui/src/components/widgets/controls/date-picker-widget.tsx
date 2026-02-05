"use client";

/**
 * DatePicker widget - renders a date input field.
 *
 * Maps to ipywidgets DatePickerModel.
 */

import { useCallback } from "react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import type { WidgetComponentProps } from "../widget-registry";
import {
  useWidgetModelValue,
  useWidgetStoreRequired,
} from "../widget-store-context";

// Convert Date object or ISO string to YYYY-MM-DD format for input
function toDateString(value: Date | string | null): string {
  if (!value) return "";
  const date = typeof value === "string" ? new Date(value) : value;
  if (Number.isNaN(date.getTime())) return "";
  return date.toISOString().split("T")[0];
}

export function DatePickerWidget({ modelId, className }: WidgetComponentProps) {
  const { sendUpdate } = useWidgetStoreRequired();

  // Subscribe to individual state keys
  const value =
    useWidgetModelValue<Date | string | null>(modelId, "value") ?? null;
  const min = useWidgetModelValue<Date | string | null>(modelId, "min") ?? null;
  const max = useWidgetModelValue<Date | string | null>(modelId, "max") ?? null;
  const description = useWidgetModelValue<string>(modelId, "description");
  const disabled = useWidgetModelValue<boolean>(modelId, "disabled") ?? false;

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const newValue = e.target.value;
      if (newValue) {
        // Send as ISO date object compatible format
        sendUpdate(modelId, { value: new Date(newValue).toISOString() });
      } else {
        sendUpdate(modelId, { value: null });
      }
    },
    [modelId, sendUpdate],
  );

  return (
    <div
      className={cn("flex items-center gap-3", className)}
      data-widget-id={modelId}
      data-widget-type="DatePicker"
    >
      {description && <Label className="shrink-0 text-sm">{description}</Label>}
      <Input
        type="date"
        value={toDateString(value)}
        min={toDateString(min)}
        max={toDateString(max)}
        disabled={disabled}
        onChange={handleChange}
        className="w-40"
      />
    </div>
  );
}

export default DatePickerWidget;
