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

// A disabled button shows its tooltip too: it can say why it is disabled.
let export = Button::new("Export").disabled(true);
let export = Tooltip::new("Nothing to export yet").mount(ctx, export);
```

| Builder | |
| --- | --- |
| `mount(ctx, element)` | Puts the tooltip on the element it describes and returns that element |
| `shortcut(&str)` | The keys of a shortcut, after the text in a chip of their own |
| `side(TooltipSide)` | `Top` (the default), `Bottom`, `Left` or `Right` of the element, centered on it |
| `arrow(bool)` | A triangle from the box to the element |
| `delay(Duration)` | How long the pointer rests before it shows: `TOOLTIP_DELAY`, 500 ms, unless this says otherwise |
| `open(Option<bool>)` | `Some(true)` shows it whatever the pointer and the focus do, `Some(false)` never: for a gallery, a test, a tour |
| `disabled(bool)` | It never shows, and the element is as it would be without it |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of what shows |

- It shows after the pointer has rested on the element for its delay, and at once within `TOOLTIP_SKIP_DELAY` (300 ms) of the pointer leaving another: along a toolbar each button's shows at once, and the one before goes, so one tooltip of the pointer shows at a time.
- Focus from the keyboard shows it at once: the focus of the element's root, or, where the root takes none (a row with a button in it), of the first thing in it that does.
- The pointer can be moved onto the tooltip to read it: it goes 100 ms after the pointer has left both the element and the tooltip. It also goes when the focus leaves, when the element is pressed, and on Escape; after a press or Escape it stays away until the pointer comes again or the focus leaves.
- Escape is the tooltip's before anything under it: the tooltip is the top layer that takes Escape, so the key closes the tooltip and not the dialog or the popup it is shown in.
- A disabled element shows its tooltip: the pointer does not hover a disabled element, so the tooltip puts it in a row of its own that the pointer does hover.
- It is kept in the window: where there is no room on its side it is on the other one, its arrow turned with it, and where the window pushes it along, its arrow still points at the middle of the element. It takes no focus.
- To what reads the tree (`lurq_snapshot` of the MCP server) the element is described by the text (`a11y_description`), and called by it (`a11y_name`) when nothing else names it, as an icon-only button is named by its label already. What shows is a `tooltip`. An element with an element ref of its own keeps it, and the tooltip is anchored to it.
- It is a mounted component, not a builder that goes where an element goes: it keeps timers for its delay and its grace, an anchor that stays the same from one render to the next, and whether the pointer has shown it, and a builder has no `Ctx` to keep them in. Converted into an element on its own, a `Tooltip` is its box as it shows, anchored to nothing: for a gallery, a test or a host of an app's own.
- Parts: `TooltipPart::Root` (the box and its arrow, and the room between them and the element), `Content` (the box), `Label`, `Shortcut`, `Arrow`. The side is the recipe's variant (`TooltipSide`), and the states are `TooltipState::Arrow`, `Shortcut` and `Multiline` (the text wraps, known once it is laid out).
- In the design the box is in `Primary` with a radius of `Md`, its text in `PrimaryForeground` in the caption's 12, 10 from its sides and 6 from its top and bottom, and no wider than 200; a text that wraps has 8 above and below it, in lines of 1.4. A shortcut is 8 after the text in a chip 16 high, 4 from its sides, with a radius of 3, in a white of 15%, its keys in the mono family in 10. The arrow is 12 along the box and 6 out from it, in the box's color. The tooltip stands 6 from the element: the root's padding on the side's axis.
- The arrow is a square in the box's color turned by 45°, its middle on the edge of the `Arrow` part that meets the box; the part clips it to the half that points away (`TooltipView::arrow()`), moved along the box by `TooltipView::aim`.
- A renderer builds what shows, not when: it is given a `TooltipView` with the text, the shortcut, the side the tooltip is on, whether it has an arrow and its aim; `view.content()` is the default box and `view.arrow()` the default arrow.

## For `CHANGELOG.md`

- `ui::Tooltip`: a short text beside an element while the pointer rests on it or the keyboard's focus is on it, mounted on that element with `mount(ctx, element)`. After `TOOLTIP_DELAY`, at once within `TOOLTIP_SKIP_DELAY` of another and for the keyboard's focus (of the element or the first thing in it that takes it); the pointer can be moved onto it; gone after leaving, on a press and on Escape, which it takes before what is under it; shown on a disabled element too; kept in the window, its arrow pointing at the element. `shortcut`, `side` (`TooltipSide`), `arrow`, `delay`, `open` and `disabled`, the parts `Root`, `Content`, `Label`, `Shortcut`, `Arrow` and the states `Arrow`, `Shortcut`, `Multiline` for a recipe, and `TooltipView::content()`, `arrow()` and `aim` for a renderer. `PartStyle` has a `max_width`.

## Decided

- **A mounted component, not a builder that wraps the element.** What a tooltip keeps, timers for its delay and its grace, an anchor (`ElementRef`) that is the same in every render so the overlay finds the element, and whether the pointer has shown it, needs a `Ctx`. `Popup` keeps its open state in an app's `Signal`, but a signal per tooltip would put the delay, the hover and the focus on every app; the engine's own tooltip of Markdown links is a component for the same reason. So `Tooltip` is a builder of what it is, and `mount(ctx, element)` mounts the host (`TooltipHost`, private) with the element as its child, as `Draggable::mount` does. The `From<Tooltip> for Element` that every component has draws the box alone, for a gallery and a playground's window of its own, whose `make` has no `Ctx`.
- **Where the overlay is declared.** The host puts the handlers, the anchor and the overlay on the element's root with `decorate_root`, so they stay when the element is a component that renders again by itself. The overlay is a child of that root: the engine finds a layer among the children of a container that are laid out (`collect_overlay_index_recursive` zips `layout.children` with `node.children()`), and a logical wrapper lays out only its first child, so a wrapper beside the element would hide it. A root that is not a row, a column, a stack or a grid (a text) is put in a row of its own first, and so is a disabled one, which the pointer does not hover (the runtime hovers the elements a disabled element is in).
- **The delay**: 500 ms, the engine's `link_tooltip_delay`; 300 ms in which the next tooltip skips it, from the pointer leaving the element of a shown one. Kept per thread (a window's tooltips are all on its thread), as is the one tooltip of the pointer that shows, which the next one puts away.
- **The grace of 100 ms** after the pointer leaves the element or the tooltip: the time to cross the 6 between them. The tooltip is hit-tested (`HitTestBehavior::ContentOnly`) so the pointer on it is known; it stands beside the element, so it covers nothing of it unless the window has room on neither side.
- **Escape** goes through the runtime's own dismissal: the overlay is open by a signal (`allowed`) with `dismiss_on_escape`, and the runtime turns the top such layer off on Escape and takes the key. A key handler on the element would have reached every node and closed the dialog under the tooltip too. A press of the element turns `allowed` off as well; the pointer coming again and the focus leaving turn it on.
- **The focus**: of the root where it takes the focus; else an element ref is put on the first node in the element that does (or its own is read), and the blur that turns `allowed` on goes there.
- **The side it is on, and the aim of the arrow.** The overlay flips and clamps with `FlipThenClamp` and does not say where it put the tooltip, so the host reads it back: an `observe_rect` on what shows compares its center with the element's, turns the arrow (the side is the recipe's variant) and moves it along the box toward the element's middle, never closer to the box's end than the arrow is long. Both show on the next pass.
- **A text that wraps** is known from the label's height (more than 1.6 of its font size, an `observe_rect` on it) and puts the tooltip in `Multiline`: 8 above and below, lines of 1.4. A wrapped text stays taller than that and a line never is, so it does not swing.
- **The room between the tooltip and the element** (6) is the root's padding on both sides of the side's axis, not an overlay offset: it is then a number of the recipe a theme can change, and the tooltip stays centered. It costs 6 more room before it flips.
- **The arrow** is a square turned by 45° in a part that clips it, as the engine has no triangle; its color is the `Content` part's background, so a theme that recolors the box recolors the arrow.
- **The name to tooling**: the semantics of the MCP server read `a11y_name` only. The element gets `a11y_description` always and `a11y_name` when it has none (only where `devtools` or `mcp` keeps what is described).
- The shortcut's chip is the board's `#FFFFFF26`, a color of the board's own, not a role: no role is a white of 15%, and the board's dark frame uses the same chip on its light box.

## Not done

- **The focus inside an element that is a component of its own.** Where the element's root takes no focus, the element ref is put on the first node in it that does. If that node is the output of a component in the element that then renders again by itself, its new node has no ref until the host renders again, and its keyboard focus shows no tooltip meanwhile. Stands in the way: the runtime tells an element ref of its own node's focus only. Smallest change: an `ElementRef::focus_visible_within()` (and its signal) that the runtime sets on the refs of the focused node's ancestors where it sets focus-visible (`focus_visible::set_node_focus_visible`, walking `focused_path`); the host would read it on its anchor and put nothing on a descendant.
- **A disabled element that fills its row.** A disabled element is put in a row of its own because the pointer does not hover it; that row is as wide as the element's content, so a disabled element whose width is `Fill` is as wide as its content while it is disabled. Stands in the way: the runtime's hover (`runtime/hover.rs`) takes a disabled hit off the hits (`take_disabled_hit`), so no handler of it runs. Smallest change: run a disabled node's `on_mouse_enter` / `on_mouse_leave` (not its hover style, not its cursor) as it is taken off, so the host can put its handlers on the element itself.

## Differs from the board

- The board does not say how far an arrow's tip is from the element; here it is the same 6 as the box without an arrow, so a tooltip with an arrow stands 6 further out.
- The board draws no shadow on a tooltip, and there is none; its layering ladder's `z-tooltip` (1500) is not a number here: an overlay is a layer over the window, over what is in it.
