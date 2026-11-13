## Changed outside the component's own files

- `crates/lurq/src/ui/part_style.rs`: `PartStyle::max_width: Option<Length>` and its setter, in `is_empty`, `merge`
  and `apply` (`NodeUpdate::max_width`), each beside `min_width`. The tooltip's box is no wider than 200, where a
  longer text wraps; no field said so. A state style still ignores it, as it ignores every size.
- `crates/lurq/src/app/ctx.rs`: `Timeout::set_duration` (`pub(crate)`) is no longer behind `feature = "markdown"`.
  The tooltip's delay is a prop, and the host's one timer takes it on every render.

## For `docs/src/content/docs/design-system.md`

## Tooltip

A short text beside an element while the pointer rests on it or the keyboard's focus is on it: what an icon button does, and the keys that do it too.

```rust
use lurq::ui::{Button, ButtonVariant, Tooltip, TooltipSide};

// In a component's render: a tooltip is mounted on the element it describes, and returns that element.
let copy = Button::new("Copy").variant(ButtonVariant::Ghost).icon("copy").icon_only(true);
let copy = Tooltip::new("Copy").shortcut("Ctrl+C").mount(ctx, copy);

let zoom = Button::new("Zoom in").icon("zoom-in").icon_only(true);
let zoom = Tooltip::new("Zoom in").side(TooltipSide::Bottom).arrow(true).mount(ctx, zoom);
```

| Builder | |
| --- | --- |
| `mount(ctx, element)` | Puts the tooltip on the element it describes and returns that element. The pointer and the focus are the element's root's: give it the control, not a row the control is in |
| `shortcut(&str)` | The keys of a shortcut, after the text in a chip of their own |
| `side(TooltipSide)` | `Top` (the default), `Bottom`, `Left` or `Right` of the element, centered on it. Where there is no room it is on the other side, its arrow turned with it |
| `arrow(bool)` | A triangle from the box to the element |
| `delay(Duration)` | How long the pointer rests before it shows: `TOOLTIP_DELAY`, 500 ms, unless this says otherwise |
| `open(Option<bool>)` | `Some(true)` shows it whatever the pointer and the focus do, `Some(false)` never: for a gallery, a test, a tour |
| `disabled(bool)` | It never shows, and the element is as it would be without it |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of what shows |

- It shows after the pointer has rested on the element for its delay, and at once within `TOOLTIP_SKIP_DELAY` (300 ms) of another tooltip going: the pointer moving along a toolbar shows each button's at once. Focus from the keyboard shows it at once. It goes when the pointer leaves, the focus leaves, the element is pressed or Escape is pressed; after a press or Escape it stays away until the pointer comes again or the focus leaves.
- It takes neither the pointer nor the focus: it is a layer over the window (an `Overlay` anchored to the element, `HitTestBehavior::None`) that the pointer goes through, and nothing in it is a stop of Tab.
- To what reads the tree (`lurq_snapshot` of the MCP server) the element is described by the text (`a11y_description`), and called by it (`a11y_name`) when nothing else names it, as an icon-only button is named by its label already. What shows is a `tooltip`.
- It is a mounted component, not a builder that goes where an element goes: it keeps a timer for its delay, an anchor that stays the same from one render to the next, and whether the pointer has shown it, and a builder has no `Ctx` to keep them in. Converted into an element on its own, a `Tooltip` is its box as it shows, anchored to nothing: for a gallery, a test or a host of an app's own.
- Parts: `TooltipPart::Root` (the box and its arrow, and the room between them and the element), `Content` (the box), `Label`, `Shortcut`, `Arrow`. The side is the recipe's variant (`TooltipSide`), and the states are `TooltipState::Arrow` and `Shortcut`.
- In the design the box is in `Primary` with a radius of `Md`, its text in `PrimaryForeground` in the caption's 12, 10 from its sides and 6 from its top and bottom, and no wider than 200. A shortcut is 8 after the text in a chip 16 high, 4 from its sides, with a radius of 3, in a white of 15%, its keys in the mono family in 10. The arrow is 12 along the box and 6 out from it, in the box's color. The tooltip stands 6 from the element: the root's padding on the side's axis.
- The arrow is a square in the box's color turned by 45°, its middle on the edge of the `Arrow` part that meets the box; the part clips it to the half that points away (`TooltipView::arrow()`).
- A renderer builds what shows, not when: it is given a `TooltipView` with the text, the shortcut, the side the tooltip is on and whether it has an arrow; `view.content()` is the default box and `view.arrow()` the default arrow.

## For `CHANGELOG.md`

- `ui::Tooltip`: a short text beside an element while the pointer rests on it or the keyboard's focus is on it, mounted on that element with `mount(ctx, element)`. After `TOOLTIP_DELAY`, at once within `TOOLTIP_SKIP_DELAY` of another going and for the keyboard's focus; gone on leaving, a press or Escape; the pointer goes through it. `shortcut`, `side` (`TooltipSide`, flipped where there is no room), `arrow`, `delay`, `open` and `disabled`, the parts `Root`, `Content`, `Label`, `Shortcut`, `Arrow` and the states `Arrow`, `Shortcut` for a recipe, and `TooltipView::content()` and `arrow()` for a renderer. `PartStyle` has a `max_width`.

## Decided

- **A mounted component, not a builder that wraps the element.** What a tooltip keeps, a timer for its delay, an anchor (`ElementRef`) that is the same in every render so the overlay finds the element, and whether the pointer has shown it, needs a `Ctx`. `Popup` keeps its open state in an app's `Signal`, but a signal per tooltip would put the delay, the hover and the focus on every app; the engine's own tooltip of Markdown links is a component for the same reason. So `Tooltip` is a builder of what it is, and `mount(ctx, element)` mounts the host (`TooltipHost`, private) with the element as its child, as `Draggable::mount` does. The `From<Tooltip> for Element` that every component has draws the box alone, for a gallery and a playground's window of its own, whose `make` has no `Ctx`.
- **Where the overlay is declared.** The host puts the handlers, the anchor and the overlay on the element's root with `decorate_root`, so they stay when the element is a component that renders again by itself. The overlay is a child of that root: the engine finds a layer among the children of a container that are laid out (`collect_overlay_index_recursive` zips `layout.children` with `node.children()`), and a logical wrapper lays out only its first child, so a wrapper beside the element would hide it. A root that is not a row, a column, a stack or a grid (a text) is put in a row of its own first.
- **The delay**: 500 ms, the engine's `link_tooltip_delay`; 300 ms in which the next tooltip skips it. The skip is kept per thread (a window's tooltips are all on its thread), set when a shown tooltip goes. Keyboard focus shows at once, as the Markdown link's tooltip does.
- **Escape and a press** put it away until the pointer comes again or the focus leaves (`on_blur`), so a press of a button does not leave its tooltip over what the press opened, and Escape does not bring it back while the pointer is still. A key reaches every node, so Escape puts away a tooltip whose element is not focused too, which is what the pointer's tooltip wants.
- **The side it is on.** The overlay flips with `FlipThenClamp` and does not say where it put the tooltip, so the host reads it back: an `observe_rect` on what shows compares its center with the element's and turns the arrow (the side is the recipe's variant) on the next pass. The first frame of a flipped tooltip has the arrow of the side it was given.
- **The room between the tooltip and the element** (6) is the root's padding on both sides of the side's axis, not an overlay offset: it is then a number of the recipe a theme can change, and the tooltip stays centered. It costs 6 more room before it flips.
- **The arrow** is a square turned by 45° in a part that clips it, as the engine has no triangle; its color is the `Content` part's background, so a theme that recolors the box recolors the arrow.
- **The name to tooling**: the semantics of the MCP server read `a11y_name` only. The element gets `a11y_description` always and `a11y_name` when it has none (only where `devtools` or `mcp` keeps what is described).
- The shortcut's chip is the board's `#FFFFFF26`, a color of the board's own, not a role: no role is a white of 15%, and the board's dark frame uses the same chip on its light box.

## Not done

- Hovering the tooltip does not keep it: the pointer goes through it, and leaving the element hides it (WCAG 1.4.13 asks that a tooltip can be hovered). With `HitTestBehavior::None` the tooltip never covers what is under it, which the board's tooltips over toolbars want; keeping it would need a hit-testable layer and a grace delay on leaving.
- Escape does not tell the element that the key was taken: an Escape that puts a tooltip away also reaches whatever else listens for it (a dialog it is in closes too).
- Where the overlay clamps the tooltip along the element (near a window's edge), the arrow stays in the middle of the box, not at the element.
- A tooltip on an element that is not focusable itself (a row with a button in it) shows for the pointer but not for the keyboard: the focus read is the element root's.
- The element's root gets the anchor as its element ref, replacing one the app gave it.
- `Toggletip`, `Teaching Tip` and `Hover Card` of the board, as the task said.
- Tests are written and compile; none was run.

## Differs from the board

- A text that wraps has 6 above and below it and the type's own line height; the board's multi-line tooltip has 8 and a line height of 1.4. Whether the text wraps is not known before it is laid out, and one recipe for both keeps the one-line tooltip, the common one, exact (27 high).
- The board does not say how far an arrow's tip is from the element; here it is the same 6 as the box without an arrow, so a tooltip with an arrow stands 6 further out.
- The board draws no shadow on a tooltip, and there is none; its layering ladder's `z-tooltip` (1500) is not a number here: an overlay is a layer over the window, over what is in it.
