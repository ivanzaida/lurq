# Speed Menu

`ui::MediaSpeedMenu`, from the board's `Media/Speed Menu` (`XUFEp`) in the section `Media` (`gr6ym`, dark `tdnDU`) of
`Components / light` (`Eu33V`), checked against `ui::Menu` and the board's `Menu Item` (`q4kDx`). Branch
`claude/ds-media-controls-14820a`, cut from `claude/ds-media`.

## Changed outside the component's own files

- The lines of the three `mod.rs` files, named in `play-button.md`. Nothing else.

## For `docs/src/content/docs/design-system.md`

## Speed Menu

The menu a player's speed is chosen from: the speeds under the heading "Playback speed", the one in use checked at the end of its row.

```rust
use lurq::{
  components::Placement,
  ui::{MediaSpeedMenu, media_rate_text},
};

// open: Signal<bool>, set by a press of the speed button; speed_button: ElementRef; speed: Signal<f32>.
MediaSpeedMenu::new(open.clone(), speed_button.clone(), speed.get())
  .on_change(move |rate| speed.set(rate as f32));

// From a bar at the bottom of a picture, opening up from the button's end, with the app's own speeds.
MediaSpeedMenu::new(open.clone(), speed_button.clone(), speed.get())
  .rates(&[0.25, 0.5, 1.0, 1.5, 2.0, 3.0])
  .placement(Placement::TopEnd);
```

| Builder | |
| --- | --- |
| `new(open, anchor, rate)` | Open while the signal is true, hung from the anchor, with `rate` checked |
| `on_change(handler)` | Called with the speed chosen, after the menu has closed, where it is another than the one in use. A second handler is called after the first |
| `on_dismiss(handler)`, `on_open_change(handler)` | As a [menu](#menu) has them: a person closed it other than by choosing; a person closed it, a choice too, after `on_dismiss` |
| `rates(&[f32])` | The speeds offered, in place of `MEDIA_RATES` (0.5×, 0.75×, Normal, 1.25×, 1.5×, 2×). Each is named by `media_rate_text` |
| `heading_text(&str)` | The words over the speeds: "Playback speed" |
| `label(&str)` | The menu's name to tooling: its heading unless told |
| `placement(Placement)`, `offset(..)`, `collision(..)`, `inline(bool)` | As a [menu](#menu) has them |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's; `force` draws the speed in use |

- It is a [menu](#menu): choosing a speed, Escape, a press outside and Tab close it, Up and Down move between the speeds, Enter chooses. Choosing the speed in use closes it and tells `on_change` nothing.
- Parts: `MediaSpeedMenuPart::Panel` and `Label` (laid over a menu's `Panel` and `Label`), `Item`, `ItemLabel`, `Check` (laid over a menu item's `Root`, `Label` and `Check`). State: `MediaSpeedMenuState::Inline`.
- In the design it is the menu's panel 168 wide (where a menu is at least 230), with the group label and a menu item for each speed; the check is at the end of the row of the speed in use. Over a picture it stays themed: it is the system's own surface, and opaque.
- A renderer that draws another speed menu starts from `view.menu()`, the menu with its speeds, or `view.item(rate)` for one speed.

## For `CHANGELOG.md`

- `ui::MediaSpeedMenu` is a component of the design system: the menu's panel 168 wide with the heading "Playback speed" and the speeds, the one in use checked at the end of its row; `on_change` is told only for another speed; it has a menu's `placement`, `offset`, `collision`, `inline`, `on_dismiss`, `on_open_change` and `label`, and `MediaSpeedMenuPart`, `MediaSpeedMenuState`, `MediaSpeedMenuView` and `MediaSpeedMenuRenderer` are new.

## Decided

- The panel is 168 wide whatever its speeds: `width`, `min_width` and `max_width` of 168 in the recipe, over the menu's least 230. A speed named longer than the room is cut by the menu item, as any menu's label is.
- The speed in use is told by a match within `f32::EPSILON`: a player holds its rate as an `f32` of the same list.
- `force` draws the speed in use, the item the focus would be on for a person who knows the menu: a menu focuses its first item as it opens, which the menu decides.

## Not done

- Nothing.

## Differs from the board

- Nothing: the board's speed menu is the menu's shell and menu items, which lurq's menu draws to the board.
