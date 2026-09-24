---
title: Styling And Events
description: Visual modifiers, state styles, cursors, inputs, event handlers, scroll, and drag and drop.
---

# Styling And Events

Most visual and input behavior is expressed as chainable modifiers on typed components.

Use [Theme](../theme/) roles for shared app semantics such as palette colors, text variants, radii, spacing, border sizes, shadows, and compound form controls. Use concrete values for isolated one-off visuals.

## Visual Modifiers

```rust
use lurq::{components::Rect, node::color::Color};

Rect::new(120.0, 40.0)
  .background("#2563eb")
  .rounded(8.0)
  .border_inside(1.0, Color::from_hex("#1d4ed8"))
```

Common visual modifiers:

| Modifier | Purpose |
| --- | --- |
| `.background(color)` | Background color from a concrete color or `PaletteColor`. |
| `.background_gradient(gradient)` | Linear, radial, or conic gradient fill. See [Gradients](#gradients). |
| `.rounded(radius)` | Uniform corner radius from `f32` or `RadiusSize`. |
| `.corner_radius_*` | Per-corner radius from `f32` or `RadiusSize`. |
| `.border_inside(width, color)` | Border inside the element bounds from a concrete width or `BorderSize`. |
| `.border_center(width, color)` | Border centered on the element edge from a concrete width or `BorderSize`. |
| `.border_outside(width, color)` | Border outside the element bounds from a concrete width or `BorderSize`. |
| `.box_shadow(shadow)` | Drop or inset shadows from a `ShadowStyle` role, a `BoxShadow`, or a list. See [Box Shadows](#box-shadows). |
| `.opacity(value)` | Draw opacity. |
| `.clip()` | Clip descendants to this element. |
| `.overflow_visible()` | Allow descendants to paint outside this element. |

Translucent colors (`"#000000a6"`, `.opacity(...)`, anti-aliased edges, text, images) blend on the sRGB-encoded
channels, as CSS and design tools do: a `#000000a6` scrim over `#eeeeee` shows `#535353`. Both native backends render
through a non-sRGB target to get this; the devtools screenshot renderer blends the same way.

## Gradients

`.background_gradient(...)` fills an element with a CSS-like gradient. It is separate from `.background(color)`; if both are set, the gradient paints the fill. Gradients respect the element's rounded corners, clipping, and `.opacity(...)` just like a solid background.

```rust
use lurq::{components::Rect, node::{Gradient, GradientStop}};

Rect::new(240.0, 120.0)
  .rounded(12.0)
  .background_gradient(Gradient::linear(135.0, ["#ff0080", "#7928ca"]))
```

Three kinds are supported on both the wgpu and dx12 backends:

| Constructor | Description |
| --- | --- |
| `Gradient::linear(angle_deg, stops)` | Linear gradient. `angle_deg` follows CSS: `0` points up, increasing clockwise (`90` is to the right). The line is sized so `0%`/`100%` reach the box corners. |
| `Gradient::radial(stops)` | Radial gradient, farthest-corner. Defaults to an ellipse fitted to the box; call `.circle()` for a circle. |
| `Gradient::conic(from_deg, stops)` | Conic gradient sweeping clockwise from `from_deg` at the top. |

### Color Stops

Stops accept anything that converts into a color (hex strings, `Color`, or a `PaletteColor`), so theme palette colors work inside gradients. A bare color is auto-positioned; use `GradientStop::at(color, position)` for an explicit position in `0.0..=1.0`.

```rust
use lurq::node::{Gradient, GradientStop};

// Auto-spaced: first at 0.0, last at 1.0, middle evenly distributed.
Gradient::linear(90.0, ["#f00", "#0f0", "#00f"]);

// Explicit positions and a palette color (lurq::app::theme::PaletteColor).
Gradient::linear(90.0, [
  GradientStop::at("#000", 0.0),
  GradientStop::at(PaletteColor::Accent, 0.4),
  GradientStop::at("#fff", 1.0),
]);
```

Omitted positions follow the CSS rules: the first defaults to `0.0`, the last to `1.0`, and runs of omitted stops are spread evenly between their defined neighbors. Colors are interpolated in linear space (CSS interpolates in sRGB, so midpoints are lighter than a browser's); the result then blends over what is below like any other translucent color.

### Center And Shape

```rust
use lurq::node::Gradient;

// Radial circle centered in the top-left quadrant.
Gradient::radial(["#fff", "#1e293b"]).circle().center(0.25, 0.25);

// Conic starting from 45 degrees, centered.
Gradient::conic(45.0, ["#f43f5e", "#8b5cf6", "#06b6d4", "#f43f5e"]);
```

`.center(x, y)` moves the radial/conic origin; coordinates are normalized `0.0..=1.0` within the element (default `(0.5, 0.5)`).

## Box Shadows

`.box_shadow(...)` gives an element CSS-like `box-shadow`s. It takes a `ShadowStyle` [theme role](../theme/#shadow), which is what app UI should use, or concrete `BoxShadow` values for one-off visuals:

```rust
use lurq::{
  app::theme::ShadowStyle,
  components::Rect,
  node::BoxShadow,
};

// A theme elevation.
Rect::new(240.0, 120.0).background("#ffffff").rounded(12.0).box_shadow(ShadowStyle::Md);

// offset_x, offset_y, blur, color; spread and inset are optional.
Rect::new(240.0, 120.0)
  .background("#ffffff")
  .rounded(12.0)
  .box_shadow([
    BoxShadow::new(0.0, 1.0, 2.0, "#0f172a1f"),
    BoxShadow::new(0.0, 12.0, 32.0, "#0f172a40").spread(-4.0),
  ]);

// An inset well.
Rect::new(240.0, 40.0)
  .background("#ffffff")
  .box_shadow(BoxShadow::new(0.0, 2.0, 6.0, "#0000004d").inset());
```

A `BoxShadow` follows CSS and design tools such as Figma and Pencil:

| Field | Meaning |
| --- | --- |
| `offset_x`, `offset_y` | Moves the shadow, in logical pixels. |
| `blur` | CSS blur radius: a Gaussian with a standard deviation of `blur / 2`. `0` is a hard edge. |
| `spread` | Grows the shadow shape (negative shrinks it) before the blur. Corner radii grow and shrink with it as CSS specifies, so square corners stay square. |
| `color` | A `Color`, hex string, or `PaletteColor` role. |
| `inset` | Paints inside the element instead of beneath it (`.inset()`). |

How shadows paint:

- A list paints first on top. Outer shadows paint beneath the element's background; inset shadows above the background and below the border and the children, inside the padding box (within an inside or centered border).
- An outer shadow follows the element's corner radii and paints only outside its box, so it never shows through a translucent background.
- Shadows never change layout and are not hit-tested: a click on a shadow goes to whatever is under it.
- Shadows take the element's `.opacity(...)` and transform, and its clip: an ancestor that clips its children clips their shadows too. Containers clip by default, so give a shadow room with padding, or call `.overflow_visible()` on the containers it should escape (as with any child that paints outside its parent).
- Values scale with the display like every other length.
- Hover, active, and focus styles can change the shadow, for example to raise a card on hover with `.hovered(|style| style.box_shadow(ShadowStyle::Lg))`; `BoxShadowValue::none()` removes it.

Both native backends evaluate the blurred rounded rect analytically in the quad shader (a closed-form `erf` along one axis, eight samples along the other), so a shadow is one more instance in the quad pipeline: no offscreen pass and no blur texture. wgpu and DX12 render the same pixels; the devtools screenshot renderer uses the same formula on the CPU.

## Hover, Active, And Focus Styles

State styles merge into the base style while the node is hovered, active, or focused.

```rust
use lurq::{components::Text, node::CursorIcon};

Text::new("Save")
  .padding_horizontal(12.0)
  .padding_vertical(8.0)
  .background("#2563eb")
  .rounded(6.0)
  .cursor(CursorIcon::Pointer)
  .hovered(|style| style.background("#3b82f6"))
  .active(|style| style.background("#1d4ed8"))
  .focused(|style| style.border_inside(1.0, "#93c5fd".into()))
```

State styles can affect layout if they change frame, padding, or flex. That is supported, but it can force relayout when interaction state changes.

Use `ctx.interaction()` when component code needs to read the current interaction state:

```rust
let interaction = ctx.interaction();
let hovered = interaction.is_hovered();
```

Attach the state to an element with `.interactive(interaction)` if you need to observe that element's state from component code.

## Mouse Events

```rust
use lurq::app::events::MouseEvent;

Rect::new(100.0, 40.0)
  .on_mouse_down(|event: MouseEvent| println!("down {:?}", event.button))
  .on_mouse_up(|event: MouseEvent| println!("up at {}, {}", event.x, event.y))
  .on_click(|event: MouseEvent| println!("click target {:?}", event.target_id))
  .on_dblclick(|event: MouseEvent| println!("double click {:?}", event.target_id))
  .on_mouse_move(|event: MouseEvent| println!("move {}, {}", event.x, event.y))
  .on_mouse_enter(|| println!("enter"))
  .on_mouse_leave(|| println!("leave"))
```

`MouseEvent` includes `x`, `y`, `button`, `kind`, and `target_id`. See [Event Control](#event-control) for `prevent_default()` and propagation methods.

Each `on_*` modifier appends a handler for that rendered node, so a node can have multiple handlers for the same event. Inline closures are render output: when the node is rendered again, the rendered handler list should replace the previous list.

Use a stable `EventHandler` when you need to remove the exact handler later:

```rust
use lurq::{app::events::MouseEvent, node::EventHandler};

let handler = EventHandler::new(|event: &MouseEvent| {
  println!("click target {:?}", event.target_id);
});

let node = Rect::new(100.0, 40.0)
  .on_click(handler.clone())
  .off_click(handler);
```

Use `ctx.on_click_outside` with an element ref when a component needs to react to clicks outside one of its own nodes:

```rust
let menu_ref = ctx.element_ref();
let open = self.open.clone();
ctx.on_click_outside(menu_ref.clone(), move |_| open.set(false));

Column::new()
  .ref_element(menu_ref)
  .child(Text::new("Menu"))
```

The hook listens for left clicks outside the referenced element's measured bounds. It is removed automatically when the component stops calling it during render.

## Keyboard And Focus

Keyboard events go to the focused node.

Inside a component, request focus with `ctx.focus(&field_ref)`, where `field_ref` is a retained `core::ElementRef` attached through `.ref_element(field_ref.clone())`. The request is applied after the render is reconciled, including when a newly mounted route creates the field. The last request wins; a ref absent from the resulting tree is ignored. `field_ref.focused()` subscribes the rendering component to focus changes; `field_ref.focus_signal()` exposes the same state for observation.

Retained input value signals, element refs, explicit IDs, keys and component slots keep focus attached to the same control across sibling insertion/reordering. Removing the focused control emits its `on_blur` callbacks and clears its ref, including when the whole tree is dropped. Use explicit keys or IDs for otherwise anonymous reorderable controls.

```rust
use lurq::app::events::KeyboardEvent;

Text::new("Focusable")
  .on_focus(|| println!("focused"))
  .on_blur(|| println!("blurred"))
  .on_key_down(|event: KeyboardEvent| {
    println!("key={} code={} shift={}", event.key, event.code, event.shift);
  })
```

`KeyboardEvent` includes `key`, `code`, `shift`, `ctrl`, `alt`, and `target_id`.

User `on_key_down` handlers run before built-in keyboard defaults, so they can block text editing, focused-button activation, select navigation, modal or popup Escape dismissal, and similar defaults:

```rust
use lurq::app::events::KeyboardEvent;

TextInput::new(value)
  .on_key_down(|event: KeyboardEvent| {
    if event.key == "Tab" {
      event.prevent_default();
    }
  })
```

## Text Selection

Plain text is not selectable by default. Opt in with `.selectable(true)`:

```rust
lurq::components::Text::new("Drag, double-click, or triple-click this text")
  .selectable(true)
```

Selectable text supports pointer drag ranges, double-click word selection, and triple-click line selection. Multiline and wrapped text render one selection highlight per selected row. Selection is visual-coordinate aware, so text inside transformed parents can still be selected from the painted position.

With the `clipboard` feature enabled, `Ctrl+C` and `Ctrl+Insert` copy the current selectable text selection to the system clipboard.

## Scroll

Wrap content in one of the scroll components:

```rust
use lurq::{
  app::events::ScrollEvent,
  components::{Column, ScrollVertical, Text},
};

ScrollVertical::new(
  Column::new()
    .spacing(8.0)
    .child(Text::new("Row 1"))
    .child(Text::new("Row 2")),
)
.on_scroll(|event: ScrollEvent| println!("delta: {}, {}", event.delta_x, event.delta_y))
```

Set the default scrollbar style on the theme:

```rust
use lurq::{layout::scrollbar::{ScrollBarStyle, ScrollBarVisibility}, node::color::Color};

app.theme().set_scrollbar(ScrollBarStyle {
  visible: ScrollBarVisibility::Auto,
  width: 7.0,
  thumb_color: Color::from_hex("#64748b"),
  thumb_radius: 4.0,
  ..ScrollBarStyle::default()
});
```

Override the scrollbar on a specific scroll component:

```rust
use lurq::{layout::scrollbar::{ScrollBarStyle, ScrollBarVisibility}, node::color::Color};

ScrollVertical::new(content)
  .scrollbar(ScrollBarStyle {
    visible: ScrollBarVisibility::Auto,
    width: 7.0,
    thumb_color: Color::from_hex("#64748b"),
    thumb_radius: 4.0,
    ..ScrollBarStyle::default()
  })
  .scrollbar_hovered(|style| style.with_thumb_color(Color::from_hex("#94a3b8")))
```

`.scrollbar_hovered(...)` receives the effective style, so it applies to either the theme default or the component override.

`ScrollEvent` includes `x`, `y`, `delta_x`, `delta_y`, `phase`, and `target_id`.

Scroll handlers run before the default scroll movement. Call `prevent_default()` to observe a wheel/scroll event without moving the scroll container:

```rust
use lurq::app::events::ScrollEvent;

ScrollVertical::new(content)
  .on_scroll(|event: ScrollEvent| {
    event.prevent_default();
  })
```

## Event Control

`MouseEvent`, `KeyboardEvent`, and `ScrollEvent` share the same control methods:

| Method | Effect |
| --- | --- |
| `event.prevent_default()` | Blocks runtime default behavior for that event. |
| `event.default_prevented()` | Returns whether a handler already prevented the default. |
| `event.stop_propagation()` | Stops later handlers for the same dispatched event path. |
| `event.propagation_stopped()` | Returns whether propagation has been stopped. |
| `event.stop_immediate_propagation()` | Stops later handlers on the current node and later nodes. |
| `event.immediate_propagation_stopped()` | Returns whether immediate propagation has been stopped. |

Propagation control and default-action control are separate. Use `stop_propagation()` when another handler should not see the event. Use `prevent_default()` when handlers may still run, but the runtime should not perform the event's built-in action.

```rust
use lurq::app::events::MouseEvent;

Rect::new(100.0, 40.0)
  .on_click(|event: MouseEvent| {
    event.stop_propagation();
  })
```

Common defaults that can be prevented include:

- focusing an input from mouse down,
- text input editing from key down,
- focused button activation from `Enter` or `Space`,
- single-line text input submit or blur on `Enter`,
- select keyboard navigation,
- modal or popup Escape dismissal,
- form submit from buttons or keyboard,
- popup outside-click dismissal,
- scroll container movement.

Public capture-phase handlers are not part of the general event API yet. The current model keeps dispatch simple: handlers receive the event, can stop later dispatch with propagation methods, and can block runtime defaults with `prevent_default()`.

## Inputs

Inputs are controlled by signals.

```rust
let checked = ctx.signal(false);
let volume = ctx.signal(50);
let name = ctx.signal(String::new());

Column::new()
  .child(lurq::components::Checkbox::new(checked.clone()))
  .child(lurq::components::Slider::new(volume.clone()).range(0, 100))
  .child(lurq::components::TextInput::new(name.clone()).placeholder("Name"))
```

Input updates write back to their signals, which rerenders the owning component.

`TextInput::on_input` runs before a built-in text edit is applied. The event carries the input's `Signal<String>` as `event.value` and the key that caused the edit as `event.keyboard`. Mutate the signal directly for custom input behavior, and call `event.prevent_default()` to cancel the built-in edit for that action:

```rust
use lurq::app::events::TextInputEvent;

TextInput::new(command.clone())
  .on_input(|event: TextInputEvent| {
    if event.keyboard.key == "Tab" {
      event.value.set("/play ".to_owned());
      event.prevent_default();
    }
  })
```

### Checkbox Styling

Checkboxes accept normal element modifiers such as `.size()`, `.background()`, `.border_inside()`, `.rounded()`, `.cursor()`, `.hovered()`, and `.focused()`. Generic `.background()` styles the unchecked box. Checked visuals use checkbox-specific styles so the checked state can have its own color or indicator.

```rust
use lurq::{components::Checkbox, core::Signal, node::color::Color};

let enabled = Signal::new(true);

Checkbox::new(enabled)
  .size(20.0, 20.0)
  .background("#ffffff")
  .border_inside(1.0, Color::from_hex("#94a3b8"))
  .rounded(4.0)
  .checked_box(|style| {
    style
      .background("#2563eb")
      .border_inside(1.0, Color::from_hex("#1d4ed8"))
      .rounded(4.0)
  })
  .box_hovered(|style| style.border_inside(1.0, Color::from_hex("#38bdf8")))
  .checked_box_hovered(|style| style.background("#1d4ed8"))
```

With the `image` feature enabled, checked boxes can render an indicator image centered inside the box:

```rust
use lurq::{components::Checkbox, images::ImageData};

let check = ImageData::from_file("assets/check.png").unwrap();

Checkbox::new(enabled)
  .checked_box(|style| {
    style
      .background("#16a34a")
      .indicator_image(check)
      .indicator_size(12.0, 12.0)
      .indicator_contain()
  })
```

With `image` and `resources`, the indicator can come from the app resource loader:

```rust
Checkbox::new(enabled)
  .checked_box(|style| style.indicator_image("ui/check.png").indicator_size(12.0, 12.0))
```

### Text Input Editing

Plain `Text` can align content inside its own box:

```rust
use lurq::{layout::Alignment, node::dimension::Dimension};

Text::new("No endpoints yet")
  .width(Dimension::Pct(100.0))
  .text_align(Alignment::Center)
```

`TextInput` keeps editing state internally while the string value remains signal-owned. Clicking focuses the input and places the caret. Dragging selects a range; double-click selects a word; triple-click selects a line. Multiline inputs support vertical caret movement and per-row selection highlights.

Single-line inputs can align value and placeholder text inside their content box:

```rust
use lurq::layout::text_style::TextAlign;

TextInput::new(endpoint.clone())
  .placeholder("Connect to an endpoint to get started.")
  .single_line()
  .text_align(TextAlign::Center)
```

Password inputs can hide their contents with `.mask()`, which renders a bullet (`•`, U+2022) for each character instead of the typed text. Use `.mask_char(...)` for a custom mask character, such as `.mask_char('\u{25cf}')` for a WinUI-style heavy dot (`●`) or `.mask_char('*')` for the previous default. Use `.unmask()` to clear masking:

```rust
TextInput::new(password.clone())
  .placeholder("Password")
  .single_line()
  .mask()

TextInput::new(pin.clone())
  .single_line()
  .mask_char('#')

TextInput::new(visible_secret.clone())
  .mask()
  .unmask()
```

Masking changes displayed text and built-in node inspection. The signal value, clipboard copy/cut, and caret and selection behavior all operate on the real text.

Built-in MCP and DevTools inspection returns the displayed mask and `masked=true`, including tree text, lookup/find data, shape details and set-value replies. Underlying editing/form values remain intact. Typed `TextInputHandle::mask()` and `is_masked()` expose the configuration; direct application access to `value()` still returns the real value. An empty masked field can still display its ordinary placeholder.

Mask glyphs use the normal text shaping and font fallback chain. If the selected font lacks U+2022, the shaper searches the loaded and system fallback fonts for the bullet. If no available fallback contains it, the font's missing-glyph marker may appear; Lurq does not substitute `*`. Load a font containing U+2022 or choose a supported custom mask in that case.

Keyboard editing supports character insertion, `Backspace`, `Delete`, arrow keys, `Home`, `End`, `Ctrl+A`, `Ctrl+Z`, `Ctrl+Y`, and `Ctrl+Shift+Z`. Hold `Shift` with movement keys to extend the selection; hold `Ctrl` with horizontal movement to jump by words.

With the `clipboard` feature enabled, text inputs also support `Ctrl+C`, `Ctrl+X`, `Ctrl+V`, `Ctrl+Insert`, `Shift+Insert`, and `Shift+Delete`. Without `clipboard`, those shortcuts do not read or write the system clipboard.

### Slider Styling

`Slider::new` uses `Signal<i32>`. Pointer input maps the track position into the range, and the default keyboard step is `1`. Use `Slider::new_f32` with `Signal<f32>` and `.range_f32(min, max)` for fractional values; `.step(value)` controls snapping and keyboard increments.

```rust
let gain = lurq::core::Signal::new(0.5_f32);
lurq::components::Slider::new_f32(gain).range_f32(0.0, 1.0).step(0.05);
```

The slider frame still accepts normal modifiers like `.width()`, `.height()`, `.cursor()`, and `.focused()`. Track and thumb visuals are styled separately with `SliderPartStyle`.

```rust
use lurq::{components::Slider, core::Signal, node::color::Color};

let value = Signal::new(68);

Slider::new(value)
  .range(0, 100)
  .width(260.0)
  .height(34.0)
  .track(|style| {
    style
      .size(220.0, 2.0)
      .background("#334155")
      .rounded(1.0)
      .border_center(1.0, Color::from_hex("#64748b"))
  })
  .track_hovered(|style| {
    style
      .height(4.0)
      .background("#475569")
      .border_center(1.0, Color::from_hex("#93c5fd"))
  })
  .thumb(|style| {
    style
      .size(12.0, 12.0)
      .background("#f97316")
      .rounded(6.0)
      .border_inside(2.0, Color::from_hex("#0f172a"))
  })
  .thumb_hovered(|style| {
    style
      .size(14.0, 14.0)
      .background("#fb923c")
      .rounded(7.0)
      .border_inside(2.0, Color::from_hex("#f8fafc"))
  })
```

The track and thumb support width, height, background color, border, corner radius, image backgrounds, and hover overrides. Corner radius accepts `f32` or `RadiusSize`. Hover dimensions are included in the slider's preferred size, so a larger hover thumb does not resize surrounding layout when the pointer enters.

The thumb is centered on the track line, not on the slider frame. A `2px` track with a `10px` or `14px` thumb keeps the thumb vertically centered on that thin track.

Image-backed slider parts use the same `image` feature as node background images:

```rust
use lurq::{components::Slider, images::ImageData};

let track = ImageData::from_file("assets/track.png").unwrap();
let thumb = ImageData::from_file("assets/thumb.png").unwrap();

Slider::new(value)
  .track(|style| style.height(2.0).background_image(track).background_cover())
  .thumb(|style| style.size(16.0, 16.0).background_image(thumb).background_cover())
```

With `image` and `resources`, pass resource paths instead:

```rust
Slider::new(value)
  .track(|style| style.background_image("ui/slider-track.png").background_cover())
  .thumb(|style| style.background_image("ui/slider-thumb.png").background_cover())
```

## Programmatic Interaction

Nodes tagged with `.id("...")` can be driven imperatively from integration code and tests, browser-DOM style. Look the node up on `Tree` and use the handle's universal actions or a typed downcast:

```rust
Column::new()
  .child(TextInput::new(email.clone()).id("email"))
  .child(Checkbox::new(agree.clone()).id("agree"))
  .child(Button::new("Save").id("save").on_click(on_save))
```

```rust
// DOM el.click(): fires the node's own on_click at its bounds center
// without hit-testing, focuses focusable nodes, submits for submit buttons.
tree.get_element_by_id_mut("save").unwrap().click();

// DOM el.value = x: writes the backing signal and clamps the caret,
// but does NOT fire on_input handlers.
tree.get_element_by_id_mut("email").unwrap()
  .as_text_input().unwrap()
  .set_value("ada@example.com");

// Widget default actions live on the typed handles.
tree.get_element_by_id_mut("agree").unwrap().as_checkbox().unwrap().toggle();
```

`focus()` and `blur()` route through the tree's focus machinery and fire the node's own `on_focus`/`on_blur` handlers. Typed handles exist for `TextInput`, `Checkbox`, `Slider`, and `Select`; downcasting a different node kind returns `None`.

These operations write signal-backed widget state, so they behave like real user input from the app's perspective — minus the event side effects called out above. For pointer-fidelity interaction (hover, capture, hit testing), drive `tree.mouse_down` / `tree.mouse_up` instead, composing coordinates from the handle's `bounds().center()`. See [Runtime And Retained Tree](../retained_nodes/#ids-and-classes) for the lookup and mutation contract.

## Drag And Drop

Use high-level DnD components when you want draggable nodes and drop zones.

```rust
use lurq::components::{
  DragContainer, DragContainerProps, Draggable, DraggableProps, DropZone, DropZoneProps, Rect, Stack,
};

let card = Draggable::mount(
  ctx,
  DraggableProps::new().on_drag_end(|event| {
    println!("drop result: {:?}", event.drop_result);
  }),
  Rect::new(64.0, 64.0)
    .background("#2563eb")
    .absolute_position(24.0, 24.0),
);

let zone = DropZone::mount(
  ctx,
  DropZoneProps::new().on_drop(|event| {
    println!("source {:?} dropped on {:?}", event.source_id, event.target_id);
  }),
  Rect::new(160.0, 100.0)
    .background("#16a34a33")
    .absolute_position(180.0, 80.0),
);

DragContainer::mount(
  ctx,
  DragContainerProps::new(),
  Stack::new()
    .size(420.0, 240.0)
    .child(zone)
    .child(card),
)
```

`DragContainerProps::new()` bounds descendant draggables to the container surface. Use `DragBounds::None` when the draggable should not be constrained.

Low-level node drag handlers are also available: `.on_drag_start`, `.on_drag_move`, `.on_drag_end`, and `.on_drop`.
