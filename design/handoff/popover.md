## Changed outside the component's own files

- **The engine: focus into an overlay and back** (`app/ctx.rs`, `components/popup.rs`, `app/runtime.rs`,
  `app/runtime/overlay_layers.rs`). `Overlay::move_focus(bool)` and `Popup::move_focus(bool)`, `false` by
  default: when such an overlay opens, the focus goes to the first element in it that takes the focus; when it
  closes with the focus inside it, the focus goes back to its anchor. The return is the runtime's own
  `return_focus_to_anchor`, which an overlay hidden with its anchor already used. The runtime keeps the anchors
  of the open overlays that move the focus (`Tree::overlay_focus_movers`, from
  `OverlayLayers::focus_movers`, which `build_overlays_from_layout_index` fills, hidden or drawn): one that is
  new this pass gets the focus (`Tree::opened_overlay_focus`, applied after `return_focus_to_anchor`, before
  an app's focus request, which wins); one that is gone while the focus was in its layer gives it back
  (`Tree::focused_overlay_closed`, beside `focused_overlay_hidden`). A layer that leaves with an exit animation
  already loses the focus (`release_input_in_leaving_layers`), so the return works with a fade. `Modal` is not
  covered: a dialog would need the same flag on `ModalSpec` and the same two lines in its arm.
- **`PartStyle::enter` and `PartStyle::exit`** (`ui/part_style.rs`, `PartAnimation` exported from `ui`): a
  keyframe animation of a part as it appears and as it leaves, with its duration and easing as the theme's
  roles (`PartAnimation::new(keyframes, MotionDuration, MotionEasing)`), applied with `NodeUpdate::animation`
  and `NodeUpdate::exit_animation`; in `is_empty` and `merge` like every field.
- **Built-in keyframes `ui::FADE_IN` (`0xFF01`) and `ui::FADE_OUT` (`0xFF02`)** (`ui/shared.rs`, exported from
  `ui`), the opacity from 0 to 1 and from 1 to 0, registered in `AnimationEngine::new` beside `SPIN` (`0xFF00`)
  (`animation/animation.rs`). A toast, a menu or a dialog that fades can play them.
- `ui/mod.rs`, `tests/ui/mod.rs`, `pages/mod.rs`: the lines of the popover, and the exports above.

## For `docs/src/content/docs/design-system.md`

## Popover

A panel beside the button that opens it, over the window, holding anything: the size of a layer, a few settings, a form. A press outside it or Escape closes it.

```rust
use lurq::{
  components::Placement,
  ui::{Button, ButtonVariant, Field, Input, Popover},
};

// anchor: ElementRef, open: Signal<bool>, width: Signal<String>
Popover::new(anchor.clone(), open.clone())
  .trigger(Button::new("Dimensions").variant(ButtonVariant::Outline))
  .title("Dimensions")
  .description("Set the size of the layer.")
  .placement(Placement::BottomStart)
  .child(Field::new("Width", Input::new(width).full_width(true)).full_width(true));
```

| Builder | |
| --- | --- |
| `new(anchor, open)` | The element it is shown beside, and the app's `Signal<bool>` of whether it is shown |
| `trigger(..)` | The button that opens and closes it: put before it, given the anchor, and pressed to turn the signal |
| `title(&str)`, `description(&str)` | The header: what it is about, and under it what it is for. Without either there is no header |
| `child(..)`, `children(..)` | What it holds, one under another, as wide as the panel |
| `placement(Placement)` | The side of the anchor it is on and the edge it lines up with. `BottomStart` by default |
| `width(..)` | How wide the panel is, over the recipe's 292 |
| `offset(..)` | How far it is from the anchor, along the side it is on: `SpacingSize::Xs` (4) by default |
| `collision(CollisionStrategy)` | What it does where there is no room on its side: `FlipThenClamp` by default |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the panel |

- It is the engine's `Popup` with the panel dressed by the recipe: it is placed against its anchor in every frame, goes to the other side where there is no room and is kept in the window, and closes on Escape and on a press outside it, which sets the signal to `false`. Declare it always and drive it with the signal: lurq only knows the anchor of a popup it can see.
- With a trigger, the trigger's own node is the anchor: a press of it, or Enter and Space on it, opens and closes the popover, and while another popup or select is open one press of it closes that one and opens this one. It tells what reads the tree `a11y_haspopup` and `a11y_expanded`. Without a trigger the app opens it, and the anchor is any element of its own (`ref_element`); a press of the anchor is not a press outside it.
- As it opens the focus goes to the first thing in it that takes the focus (`Popup::move_focus`), and as it closes with the focus inside, back to the trigger. It does not keep Tab in it: it is not modal.
- It fades in over the theme's `Base` duration in the `Enter` curve, and out over `Fast` in the `Exit` curve (`PartStyle::enter`, `PartStyle::exit` on the root, `FADE_IN` and `FADE_OUT`). A fade is not movement, so reduced motion leaves it.
- A theme given to a subtree reaches a popover declared in it.
- Parts: `PopoverPart::Root` (the panel), `Header` (the title over the description), `Title`, `Description`, `Body` (what it holds). It has no states of its own.
- In the design the panel is 292 wide in `Popover` and `PopoverForeground`, with an edge of 1 in `Border`, the `Lg` radius and the `Md` shadow, 16 of padding and 14 between the header and the body. The title is in the body's type at medium weight, the description in 13 and `MutedForeground`, 4 under it; what the body holds is 14 apart.
- To what reads the tree the panel is a `dialog` called by its title.
- A renderer builds the panel alone: where it is shown, when, its trigger and its focus stay the popover's. It starts from `view.panel()`, the panel with its style and nothing in it, and has `view.header()` and `view.children`.

## For `CHANGELOG.md`

- `ui::Popover`: a panel beside the button that opens it, over the window, with a title, a description and anything in it, which takes the focus as it opens, gives it back as it closes, and fades in and out; `Popup::move_focus` and `Overlay::move_focus`; `PartStyle::enter` and `PartStyle::exit` with `ui::FADE_IN` and `ui::FADE_OUT`.

## Decided

- The popover is a builder over `components::Popup`, not a component of its own: what is open is the app's `Signal<bool>`, and the popup already places, flips, clamps and dismisses. What a builder cannot do without a `Ctx`, move the focus, the engine now does for an overlay that asks (`move_focus`).
- `trigger` takes any element and puts the anchor, a press that turns the signal, and `a11y_haspopup` / `a11y_expanded` on its own root node, so a `ui::Button` is the anchor itself and the engine's switching of dropdowns works for it. `Button` is not changed.
- The renderer of a theme builds the panel only (`PopoverRenderer` takes a `PopoverView` and returns the panel); `Popover::build` wraps whatever it returns in the `Popup` with the trigger, so a theme's other anatomy cannot lose the placement, the dismissal or the focus.
- The offset from the anchor is not on the board (the panel is drawn alone). It is the theme's `SpacingSize::Xs`, 4, as the gallery's own popups have it, and is applied away from the anchor on whichever side it is placed: `(0, offset)` over or under it, `(offset, 0)` beside it. A recipe has no field for it, so it is a builder prop and not a part style.
- The motion is the board's (Foundations, Motion): popovers take the `base` duration, things arriving the `enter` curve, things leaving the `exit` curve, and exits are quicker than entrances, so the exit is `fast`. The board draws no movement for a popover, only the fade; a slide would also be wrong after a flip, since an overlay does not tell which side it ended on.
- The board's title is 14 medium and its description 13 regular: the title is `TypographyStyle::Body` at `FontWeight::Medium`, the description `TypographyStyle::Description` at 13.
- The board has nothing but the header and a `Field` in the panel, 14 apart: the body is a column 14 apart, so what is added under the field is as far from it as the field is from the header.
- There is no `PopoverState`: the board draws the panel in one state, and `state(name, on)` is there for a theme's own.
- `width` is laid under what `style` and `rules` say of the root, so a rule of the app for the root's width wins over it.
- With nothing in it that takes the focus, the focus stays on the trigger as it opens: the panel itself is not made a stop of Tab.

## Not done

- Tab from the last thing in an open popover goes to the first stop of the page, not to the stop after its trigger: the window's order of Tab puts every open overlay after the page (`docs/.../focus-navigation.md`, Tab Scope). Smallest change: in `app/runtime/tab_navigation.rs`, order the stops of an overlay layer whose overlay moves the focus right after the stops of its anchor, in the window scope.

## Differs from the board

- The board draws the panel alone, with no anchor: where it is shown, the side it is on and the 4 between it and the anchor are the engine's and this component's, not the board's.
- The board's title and description have no wrapping width of their own; here they are as wide as the panel, and a long description wraps in it.
