/**
 * Built-in widget components for @jupyter-widgets/controls.
 *
 * This module registers all built-in widget components with the widget registry.
 * Import this module to enable rendering of standard ipywidgets.
 */

import { registerWidget } from "../widget-registry";

// Import widget components
import { IntSlider } from "./int-slider";
import { FloatSlider } from "./float-slider";
import { IntProgress } from "./int-progress";
import { FloatProgress } from "./float-progress";
import { ButtonWidget } from "./button-widget";
import { CheckboxWidget } from "./checkbox-widget";
import { TextWidget } from "./text-widget";
import { TextareaWidget } from "./textarea-widget";
// Selection widgets
import { DropdownWidget } from "./dropdown-widget";
import { RadioButtonsWidget } from "./radio-buttons-widget";
import { SelectMultipleWidget } from "./select-multiple-widget";
import { ToggleButtonWidget } from "./toggle-button-widget";
import { ToggleButtonsWidget } from "./toggle-buttons-widget";

// Import layout widget components
import { VBoxWidget } from "./vbox-widget";
import { HBoxWidget } from "./hbox-widget";
import { BoxWidget } from "./box-widget";
import { GridBoxWidget } from "./gridbox-widget";
import { AccordionWidget } from "./accordion-widget";
import { TabWidget } from "./tab-widget";

// Register all widgets with their model names
registerWidget("IntSliderModel", IntSlider);
registerWidget("FloatSliderModel", FloatSlider);
registerWidget("IntProgressModel", IntProgress);
registerWidget("FloatProgressModel", FloatProgress);
registerWidget("ButtonModel", ButtonWidget);
registerWidget("CheckboxModel", CheckboxWidget);
registerWidget("TextModel", TextWidget);
registerWidget("TextareaModel", TextareaWidget);
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

// Re-export components for direct use
export { IntSlider } from "./int-slider";
export { FloatSlider } from "./float-slider";
export { IntProgress } from "./int-progress";
export { FloatProgress } from "./float-progress";
export { ButtonWidget } from "./button-widget";
export { CheckboxWidget } from "./checkbox-widget";
export { TextWidget } from "./text-widget";
export { TextareaWidget } from "./textarea-widget";
// Selection widgets
export { DropdownWidget } from "./dropdown-widget";
export { RadioButtonsWidget } from "./radio-buttons-widget";
export { SelectMultipleWidget } from "./select-multiple-widget";
export { ToggleButtonWidget } from "./toggle-button-widget";
export { ToggleButtonsWidget } from "./toggle-buttons-widget";

// Re-export layout widgets
export { VBoxWidget } from "./vbox-widget";
export { HBoxWidget } from "./hbox-widget";
export { BoxWidget } from "./box-widget";
export { GridBoxWidget } from "./gridbox-widget";
export { AccordionWidget } from "./accordion-widget";
export { TabWidget } from "./tab-widget";
