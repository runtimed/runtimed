/**
 * Built-in widget components for @jupyter-widgets/controls.
 *
 * This module registers widget components with the widget registry.
 * Import this module to enable rendering of standard ipywidgets.
 */

import { registerWidget } from "../widget-registry";

// Import widget components
import { IntSlider } from "./int-slider";

// Register widgets with their model names
registerWidget("IntSliderModel", IntSlider);

// Re-export components for direct use
export { IntSlider } from "./int-slider";
