# Menu: hand-off

`ui::Menu` and `ui::MenuItem`, from the board's `Menu` (`iPsCT`) and `Menu Item` (`q4kDx`) in "Select & Menu" of
Components / light (`Ovt1D`), with the board's "Dropdown menu" example (`RxliJ`) for the icons, the group label and
the destructive item. `Menubar`, `Context Menu`, `Overflow Menu`, `Navigation Menu` and `Menu Button` are not built.

## For `docs/src/content/docs/design-system.md`

## Menu

A list of commands in a panel over the window, opened from an anchor: the actions of a file, the account's menu, how a split button does what it does.

```rust
use lurq::{
  components::Row,
  ui::{Button, ButtonVariant, Menu, MenuItem, icons},
};

// anchor: ElementRef, open: Signal<bool>. The button the menu hangs from opens and closes it.
let toggle = open.clone();
Row::new()
  .ref_element(anchor.clone())
  .child(Button::new("File").variant(ButtonVariant::Ghost).on_click(move |_| toggle.update(|open| *open = !*open)));

Menu::new(anchor, open)
  .label("File")
  .item(MenuItem::new("Rename").icon(icons::PENCIL).shortcut("F2").on_select(move || rename()))
  .item(MenuItem::new("Show hidden files").checked(hidden.get()).on_select(move || toggle_hidden()))
  .item(MenuItem::new("Redo").shortcut("Ctrl+Y").disabled(true))
  .separator()
  .item(MenuItem::new("Delete").icon(icons::TRASH).destructive(true).on_select(move || delete()));
```

| `Menu` | |
| --- | --- |
| `new(anchor, open)` | What it hangs from (an `ElementRef`) and whether it is open: the app's `Signal<bool>`, or a `bool` that holds it open or closed |
| `item(MenuItem)`, `items(..)` | Commands, after what the menu holds already |
| `separator()` | A line that ends one group of items and starts the next |
| `label(&str)` | A text over the items after it: what they are about |
| `placement(Placement)` | Where the panel opens beside its anchor, 4 from it. `BottomStart` by default |
| `inline(bool)` | The panel drawn where the menu is in the tree, open or not, and not over the window: for a gallery, a test, a list of commands in a page |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the panel |

| `MenuItem` | |
| --- | --- |
| `icon(name)` | An icon before the label, by its name in the theme's set |
| `shortcut(&str)` | The keys that do the same, written at the end: "Ctrl+Z". The item only shows them; the app binds the keys |
| `checked(bool)` | Whether what it turns on is on: a check at its end. An item given this is a `menuitemcheckbox` |
| `destructive(bool)` | It removes or cannot be undone: drawn in the destructive color, icon and all |
| `on_select(handler)` | Called when it is chosen, before its menu closes |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the item |

- The menu does not keep whether it is open: the app does, in the signal it gives it, and the anchor's own handler opens it. Choosing an item calls the item's handler and sets the signal to `false`; a press outside the panel and Escape do too (the engine's `Popup`). Declare the menu always, open or not, beside its anchor: lurq then knows the anchor is its trigger.
- An item is chosen by a click, or by Enter or Space while it is focused. While the menu is open Tab reaches its items, after the page's own stops (an open popup is after the page in the order of Tab); a disabled item is passed by.
- Under the pointer or focused from the keyboard an item is highlighted: filled with `Highlight`, the board's `accent`.
- Parts of the menu: `MenuPart::Panel`, `Separator` (the room around the line), `SeparatorLine`, `Label`. Parts of an item: `MenuItemPart::Root` (what is pressed), `Icon`, `Label`, `Shortcut`, `Check`. States of an item: `MenuItemState::Checked`, `Destructive`, `Disabled`. A menu has no states of its own.
- In the design the panel is at least 230 wide, in `Popover` with `PopoverForeground`, padded 4, with the large radius, an edge of 1 in `Border` and the medium shadow. An item is 32 tall, padded 8, with the small radius and 8 between what it holds: an icon of 16 in `MutedForeground`, the label in the body's type, and at the end the shortcut in the mono type at 11 in `MutedForeground` and the check of 16. A separator is a line of 1 in `Border` with 4 above and below; a group's label is 11/600 in `MutedForeground`, 8 in from the side. A destructive item is in `Destructive`, its icon too; a disabled one is dimmed.
- To what reads the tree the panel is a `menu`, an item a `menuitem` (a `menuitemcheckbox` with `a11y_checked` when it was given `checked`) called by its label, and a separator a `separator`.
- A renderer for `Menu` gets a `MenuView`: `view.panel()` is the panel with its style, `view.item(item)`, `view.separator()` and `view.label(text)` build what it holds, and `view.render()` hangs it from the anchor. A renderer for `MenuItem` starts from `view.root()`, the behavior with the root's style, and puts in it `view.icon()`, `view.label()`, `view.shortcut()`, `view.check()` or what it likes; choosing it still closes its menu.

## For `CHANGELOG.md`

- `ui::Menu` and `ui::MenuItem`: a list of commands in a panel over the window, opened from an anchor, with icons, shortcuts, separators, group labels, checked, destructive and disabled items; choosing one, a press outside and Escape close it (`Menu::new(anchor, open)`, `MenuItem::on_select`).

## Decided

- **Two themed components, `menu` and `menu_item`**, as the board has two (`Menu`, `Menu Item`): a theme styles or
  renders an item without touching the panel. The menu builds its items in its own theme and hands each the signal
  it closes (`MenuItem::closing`, crate-private).
- **The engine's `Popup`, not a component of its own.** `Popup::new(anchor, panel).open(open)` already closes on
  Escape and on a press outside, consumes that press, follows its anchor and is painted in the theme of the subtree
  it was declared in. The menu adds only what choosing an item does, the gap of 4 from the anchor (offset along the
  placement's axis, so a left or right placement keeps it too) and its anatomy. A builder is enough: what is open is
  the app's signal.
- `Menu::new` takes `impl Into<OpenState>`, as `Popup::open` does: a `Signal<bool>` is the usual, a `bool` holds it
  open (or closed) for a gallery or a test, and then choosing an item calls its handler and closes nothing.
- **`inline(bool)`** draws the panel in place: the board draws its menus standing, the gallery's "Checked, disabled
  and grouped" section does the same, and a test of the anatomy does not depend on where the popup is placed.
- **Highlighted is the pointer's and the keyboard's**, not a state the app sets: `hovered` and `focus_visible` of
  the item's root both fill it with `Highlight`. Both set the same property to the same role, so the pointer's style
  does not take the keyboard's away (the playbook's warning is about different values). `force(Interaction::Hovered)`
  draws an item highlighted for a gallery.
- The panel's text is `PopoverForeground`, the role paired with its surface; the board writes the item's label in
  `$foreground`. Both are the same color in the light and the dark palette.
- The width is a least width (`min_width(230)`) and not the board's fixed 230: a menu with longer items grows. The
  board's other menus are 160 to 240 wide; `.style(MenuPart::Panel, |s| s.min_width(160.0))` gives one its own.
- An item given `checked(false)` draws nothing at its end (the board reserves no room for the check) but is still a
  `menuitemcheckbox` to what reads the tree.
- `ui::Button` takes no `ElementRef`, so the anchor is an element around the button (`Row::new().ref_element(..)`).
  In the split button the whole `ButtonGroup` is the anchor and the menu opens under it from its end
  (`Placement::BottomEnd`): a button inside a group cannot be wrapped.

## Not done

- **Arrow keys, Home, End and typing a letter to move through the items** are not built: a key event reaches every
  node, and the menu cannot tell that the focus is inside it (the playbook says why). For the same reason opening the
  menu does not move the focus into it, and closing it does not give the focus back to the button it hangs from:
  a builder has no `Ctx` to ask for focus. Tab and Shift+Tab reach the items, after the page's own stops.
- Hover does not move the focus: an item focused from the keyboard and another under the pointer are both
  highlighted at once, where a native menu has one.
- The board's radio item (`100%` with a `dot-circle` in its "checked, radio, disabled, submenu" menu) and the
  submenu (`Export as` with a chevron, opening `PNG / SVG / PDF` beside it) are not built: the brief named neither.
  A submenu needs a menu that opens a second popup from an item and keeps both open while the pointer moves between
  them; a radio item is `checked` with another glyph and a group's choice (`Chosen::one`).
- No pressed style: the board draws none, and the menu closes on the press.
- The gallery page is in the group Surfaces & overlays after the last page of Inputs; where it goes among the other
  overlays of the batch is the integrator's to say.
- Tests are written (`crates/lurq/tests/ui/menu.rs`) and compile; per the playbook they were not run. The ones that
  depend most on the engine are the widths (an item as wide as a panel whose width is a least width, through
  `align_items(Stretch)`) and the panel 4 under its anchor.

## Differs from the board

- **The label of a group** has 5 above and under it; the board's reusable menu has 6 above and 4 under (`padding
  [6, 8, 4, 8]`): a part has one vertical padding. The height is the same. The board's "Dropdown menu" example draws
  its label 12/500 with 6 on both sides; the build follows the reusable component (11/600).
- **A disabled item** is dimmed by the theme's `OpacityLevel::Disabled` (0.5), as every disabled control of the
  design system is; the board's is 0.45.
- **The gap from the anchor** (4) is the board's gap between an open select and its options; the board draws no
  menu with its trigger.
- The weight of the icons and of the check is the icon font's; the board sets their stroke to 1.333 in its 16 px
  instances.
