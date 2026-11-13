## For `docs/src/content/docs/design-system.md`

## Popover

A panel beside the element that opens it, over the window, holding anything: the size of a layer, a few settings, a form. A press outside it or Escape closes it.

```rust
use lurq::{
  components::{Placement, Row},
  ui::{Button, ButtonVariant, Field, Input, Popover},
};

// anchor: ElementRef, open: Signal<bool>, width: Signal<String>
let toggle = open.clone();
Row::new()
  .child(
    Row::new().ref_element(anchor.clone()).child(
      Button::new("Dimensions")
        .variant(ButtonVariant::Outline)
        .on_click(move |_| toggle.update(|open| *open = !*open)),
    ),
  )
  .child(
    Popover::new(anchor.clone(), open.clone())
      .title("Dimensions")
      .description("Set the size of the layer.")
      .placement(Placement::BottomStart)
      .child(Field::new("Width", Input::new(width).full_width(true)).full_width(true)),
  );
```

| Builder | |
| --- | --- |
| `new(anchor, open)` | The element it is shown beside, and the app's `Signal<bool>` of whether it is shown |
| `title(&str)`, `description(&str)` | The header: what it is about, and under it what it is for. Without either there is no header |
| `child(..)`, `children(..)` | What it holds, one under another, as wide as the panel |
| `placement(Placement)` | The side of the anchor it is on and the edge it lines up with. `BottomStart` by default |
| `width(..)` | How wide the panel is, over the recipe's 292 |
| `offset(..)` | How far it is from the anchor, along the side it is on: `SpacingSize::Xs` (4) by default |
| `collision(CollisionStrategy)` | What it does where there is no room on its side: `FlipThenClamp` by default |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the panel |

- It is the engine's `Popup` with the panel dressed by the recipe: it is placed against its anchor in every frame, goes to the other side where there is no room, and closes on Escape and on a press outside it, which sets the signal to `false`. A press of the anchor is not outside it, so a button in the anchor that turns the signal opens and closes it. Declare it always and drive it with the signal: lurq only knows the anchor of a popup it can see.
- The app makes the element that opens it and gives it the anchor with `ref_element`; a button of the design system is put in a row that has the anchor.
- A theme given to a subtree reaches a popover declared in it.
- Parts: `PopoverPart::Root` (the panel), `Header` (the title over the description), `Title`, `Description`, `Body` (what it holds). It has no states of its own.
- In the design the panel is 292 wide in `Popover` and `PopoverForeground`, with an edge of 1 in `Border`, the `Lg` radius and the `Md` shadow, 16 of padding and 14 between the header and the body. The title is in the body's type at medium weight, the description in 13 and `MutedForeground`, 4 under it; what the body holds is 14 apart.
- To what reads the tree the panel is a `dialog` called by its title.
- A renderer builds the panel alone: where it is shown and when stays the popover's. It starts from `view.panel()`, the panel with its style and nothing in it, and has `view.header()` and `view.children`.

## For `CHANGELOG.md`

- `ui::Popover`: a panel beside the element that opens it, over the window, with a title, a description and anything in it; the engine's `Popup` dressed by a recipe, open while the app's signal says so, closed by Escape or a press outside it.

## Decided

- The popover is a builder over `components::Popup`, not a component of its own: what is open is the app's `Signal<bool>`, and the popup already places, flips, clamps and dismisses.
- The renderer of a theme builds the panel only (`PopoverRenderer` takes a `PopoverView` and returns the panel); `Popover::build` wraps whatever it returns in the `Popup`, so a theme's other anatomy cannot lose the placement or the dismissal.
- The offset from the anchor is not on the board (the panel is drawn alone). It is the theme's `SpacingSize::Xs`, 4, as the gallery's own popups have it, and is applied away from the anchor on whichever side it is placed: `(0, offset)` over or under it, `(offset, 0)` beside it. A recipe has no field for it, so it is a builder prop and not a part style.
- The board's title is 14 medium and its description 13 regular: the title is `TypographyStyle::Body` at `FontWeight::Medium`, the description `TypographyStyle::Description` at 13.
- The board has nothing but the header and a `Field` in the panel, 14 apart: the body is a column 14 apart, so what is added under the field is as far from it as the field is from the header.
- There is no `PopoverState`: the board draws the panel in one state, and `state(name, on)` is there for a theme's own.
- `width` is laid under what `style` and `rules` say of the root, so a rule of the app for the root's width wins over it.

## Not done

- No footer: the board's popover has none. The `Tour` popover on the board (Guidance) has one, with a count and Back and Next; it is not this component.
- No enter or exit animation. The design system plan says popovers fade and slide in, but the board draws no motion for this one and a part style has no animation; a renderer can give the panel `.animation(..)` and `.exit_animation(..)`.
- Focus does not move into the popover when it opens, and Tab is not kept in it: the engine's `Popup` does neither.
- No arrow: the board's desktop popover has none. The tablet one (`Tablet/Popover`) does, with a list of rows; it is not built.
- A button of the design system cannot be given an `ElementRef`, so the anchor is a row around it. A `Button::anchor(ElementRef)` (or `ref_element`) would let the button be the anchor, and then the trigger of the popup the engine knows (a press of another dropdown's trigger switches to it); it belongs to `Button`.
- Not built: `Hover Card`, `Tray Popover`, `Teaching Tip`, `Popconfirm`, as asked.

## Differs from the board

- The board draws the panel alone, with no anchor: where it is shown, the side it is on and the 4 between it and the anchor are the engine's and this component's, not the board's.
- The board's title and description have no wrapping width of their own; here they are as wide as the panel, and a long description wraps in it.
