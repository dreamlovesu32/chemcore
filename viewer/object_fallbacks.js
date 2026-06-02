// Legacy scene-object renderer facade.
// Current maintained products should render from Rust/WASM core primitives instead.

export { renderLineObject } from "./legacy_line_renderer.js";
export { renderTextObject } from "./legacy_text_renderer.js";
export { renderShapeObject } from "./legacy_shape_renderer.js";
