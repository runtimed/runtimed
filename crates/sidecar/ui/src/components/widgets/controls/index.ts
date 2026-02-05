import { registerWidget } from "../widget-registry";
import { AccordionWidget } from "./accordion-widget";
import { BoxWidget } from "./box-widget";
import { ButtonWidget } from "./button-widget";
import { CheckboxWidget } from "./checkbox-widget";
import { ColorPicker } from "./color-picker";
// Selection widgets
import { DropdownWidget } from "./dropdown-widget";
import { FloatProgress } from "./float-progress";
import { FloatRangeSlider } from "./float-range-slider";
import { FloatSlider } from "./float-slider";
import { GridBoxWidget } from "./gridbox-widget";
import { HBoxWidget } from "./hbox-widget";
import { HTMLWidget } from "./html-widget";
import { IntProgress } from "./int-progress";
import { IntRangeSlider } from "./int-range-slider";
// Import widget components
import { IntSlider } from "./int-slider";
import { RadioButtonsWidget } from "./radio-buttons-widget";
import { SelectMultipleWidget } from "./select-multiple-widget";
import { TabWidget } from "./tab-widget";
import { TextWidget } from "./text-widget";
import { TextareaWidget } from "./textarea-widget";
import { ToggleButtonWidget } from "./toggle-button-widget";
import { ToggleButtonsWidget } from "./toggle-buttons-widget";
// Import layout widget components
import { VBoxWidget } from "./vbox-widget";

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

export { AccordionWidget } from "./accordion-widget";
export { BoxWidget } from "./box-widget";
export { ButtonWidget } from "./button-widget";
export { CheckboxWidget } from "./checkbox-widget";
export { ColorPicker } from "./color-picker";
// Selection widgets
export { DropdownWidget } from "./dropdown-widget";
export { FloatProgress } from "./float-progress";
export { FloatRangeSlider } from "./float-range-slider";
export { FloatSlider } from "./float-slider";
export { GridBoxWidget } from "./gridbox-widget";
export { HBoxWidget } from "./hbox-widget";
export { HTMLWidget } from "./html-widget";
export { IntProgress } from "./int-progress";
export { IntRangeSlider } from "./int-range-slider";
// Re-export components for direct use
export { IntSlider } from "./int-slider";
export { RadioButtonsWidget } from "./radio-buttons-widget";
export { SelectMultipleWidget } from "./select-multiple-widget";
export { TabWidget } from "./tab-widget";
export { TextWidget } from "./text-widget";
export { TextareaWidget } from "./textarea-widget";
export { ToggleButtonWidget } from "./toggle-button-widget";
export { ToggleButtonsWidget } from "./toggle-buttons-widget";
// Re-export layout widgets
export { VBoxWidget } from "./vbox-widget";
