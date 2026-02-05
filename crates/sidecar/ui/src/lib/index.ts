import { registerWidget } from "./widget-registry";
import { AccordionWidget } from "@/components/accordion-widget";
import { BoxWidget } from "@/components/box-widget";
import { ButtonWidget } from "@/components/button-widget";
import { CheckboxWidget } from "@/components/checkbox-widget";
import { ColorPicker } from "@/components/color-picker";
// Selection widgets
import { DropdownWidget } from "@/components/dropdown-widget";
import { FloatProgress } from "@/components/float-progress";
import { FloatRangeSlider } from "@/components/float-range-slider";
import { FloatSlider } from "@/components/float-slider";
import { GridBoxWidget } from "@/components/gridbox-widget";
import { HBoxWidget } from "@/components/hbox-widget";
import { HTMLWidget } from "@/components/html-widget";
import { IntProgress } from "@/components/int-progress";
import { IntRangeSlider } from "@/components/int-range-slider";
// Import widget components
import { IntSlider } from "@/components/int-slider";
import { RadioButtonsWidget } from "@/components/radio-buttons-widget";
import { SelectMultipleWidget } from "@/components/select-multiple-widget";
import { TabWidget } from "@/components/tab-widget";
import { TextWidget } from "@/components/text-widget";
import { TextareaWidget } from "@/components/textarea-widget";
import { ToggleButtonWidget } from "@/components/toggle-button-widget";
import { ToggleButtonsWidget } from "@/components/toggle-buttons-widget";
// Import layout widget components
import { VBoxWidget } from "@/components/vbox-widget";

// Register all widgets with their model names
registerWidget("IntSliderModel", IntSlider);
registerWidget("FloatSliderModel", FloatSlider);
registerWidget("IntRangeSliderModel", IntRangeSlider);
registerWidget("FloatRangeSliderModel", FloatRangeSlider);
registerWidget("IntProgressModel", IntProgress);
registerWidget("FloatProgressModel", FloatProgress);
registerWidget("ButtonModel", ButtonWidget);
registerWidget("CheckboxModel", CheckboxWidget);
registerWidget("TextModel", TextWidget);
registerWidget("TextareaModel", TextareaWidget);
registerWidget("HTMLModel", HTMLWidget);
registerWidget("ColorPickerModel", ColorPicker);
// Selection widgets
registerWidget("DropdownModel", DropdownWidget);
registerWidget("RadioButtonsModel", RadioButtonsWidget);
registerWidget("SelectMultipleModel", SelectMultipleWidget);
registerWidget("ToggleButtonModel", ToggleButtonWidget);
registerWidget("ToggleButtonsModel", ToggleButtonsWidget);

// Register layout widgets
registerWidget("VBoxModel", VBoxWidget);
registerWidget("HBoxModel", HBoxWidget);
registerWidget("BoxModel", BoxWidget);
registerWidget("GridBoxModel", GridBoxWidget);
registerWidget("AccordionModel", AccordionWidget);
registerWidget("TabModel", TabWidget);

export { AccordionWidget } from "@/components/accordion-widget";
export { BoxWidget } from "@/components/box-widget";
export { ButtonWidget } from "@/components/button-widget";
export { CheckboxWidget } from "@/components/checkbox-widget";
export { ColorPicker } from "@/components/color-picker";
// Selection widgets
export { DropdownWidget } from "@/components/dropdown-widget";
export { FloatProgress } from "@/components/float-progress";
export { FloatRangeSlider } from "@/components/float-range-slider";
export { FloatSlider } from "@/components/float-slider";
export { GridBoxWidget } from "@/components/gridbox-widget";
export { HBoxWidget } from "@/components/hbox-widget";
export { HTMLWidget } from "@/components/html-widget";
export { IntProgress } from "@/components/int-progress";
export { IntRangeSlider } from "@/components/int-range-slider";
// Re-export components for direct use
export { IntSlider } from "@/components/int-slider";
export { RadioButtonsWidget } from "@/components/radio-buttons-widget";
export { SelectMultipleWidget } from "@/components/select-multiple-widget";
export { TabWidget } from "@/components/tab-widget";
export { TextWidget } from "@/components/text-widget";
export { TextareaWidget } from "@/components/textarea-widget";
export { ToggleButtonWidget } from "@/components/toggle-button-widget";
export { ToggleButtonsWidget } from "@/components/toggle-buttons-widget";
// Re-export layout widgets
export { VBoxWidget } from "@/components/vbox-widget";
