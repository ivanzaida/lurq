# Menu: hand-off

`ui::Menu` and `ui::MenuItem`, from the board's `Menu` (`iPsCT`) and `Menu Item` (`q4kDx`) in "Select & Menu" of
Components / light (`Ovt1D`): the board's "Menu · checked, radio, disabled, submenu" (`NKwJ3`) for the checked and
radio items, the disabled one and the submenu, and its "Dropdown menu" example (`RxliJ`) for the icons, the group
label and the destructive item.

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
  .child(Button::new("View").variant(ButtonVariant::Ghost).on_click(move |_| toggle.update(|open| *open = !*open)));

Menu::new(anchor, open)
  .item(MenuItem::new("Undo").icon(icons::UNDO).shortcut("Ctrl+Z").on_select(move || undo()))
  .item(MenuItem::new("Redo").shortcut("Ctrl+Y").disabled(true))
  .separator()
  .item(MenuItem::new("Show grid").checked(grid.get()).on_select(move || toggle_grid()))
  .separator()
  .label("Zoom")
  .item(MenuItem::new("Fit to window").radio(zoom.get() == Zoom::Fit).on_select(move || fit()))
  .item(MenuItem::new("100%").radio(zoom.get() == Zoom::Actual).on_select(move || actual()))
  .separator()
  // export: ElementRef, exporting: Signal<bool>: the submenu hangs from its item.
  .item(MenuItem::new("Export as").submenu(Menu::new(export, exporting).items(["PNG", "SVG", "PDF"].map(MenuItem::new))))
  .item(MenuItem::new("Delete").icon(icons::TRASH).destructive(true).on_select(move || delete()));
```

| `Menu` | |
| --- | --- |
| `new(anchor, open)` | What it hangs from (an `ElementRef`) and whether it is open: the app's `Signal<bool>`, or a `bool` that holds it open or closed |
| `item(MenuItem)`, `items(..)` | Commands, after what the menu holds already |
| `separator()` | A line that ends one group of items and starts the next |
| `label(&str)` | A text over the items after it: what they are about |
| `placement(Placement)` | Where the panel opens beside its anchor, 4 from it. `BottomStart` by default, `RightStart` for a submenu |
| `inline(bool)` | The panel drawn where the menu is in the tree, open or not, and not over the window: for a gallery, a test, a list of commands in a page |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the panel |

| `MenuItem` | |
| --- | --- |
| `icon(name)` | An icon before the label, by its name in the theme's set |
| `shortcut(&str)` | The keys that do the same, written at the end: "Ctrl+Z". The item only shows them; the app binds the keys |
| `checked(bool)` | Whether what it turns on is on: a check at its end |
| `radio(bool)` | One of several of which one is chosen: a dot at the end of the chosen one |
| `submenu(Menu)` | A menu the item opens beside it, with a chevron at its end. The submenu's anchor is given to the item |
| `destructive(bool)` | It removes or cannot be undone: drawn in the destructive color, icon and all |
| `on_select(handler)` | Called when it is chosen, before its menu closes |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the item |

- The menu does not keep whether it is open: the app does, in the signal it gives it, and the anchor's own handler opens it. Choosing an item calls the item's handler and sets the signal to `false`; a press outside the panel and Escape do too (the engine's `Popup`). Declare the menu always, open or not, beside its anchor: lurq then knows the anchor is its trigger.
- The keyboard: Tab stops in an open menu once (the panel is a [focus group](../focus-navigation/#focus-groups), one stop), Up and Down move between its items and go round at the ends, Home and End go to the first and the last, and a disabled item is passed by. Enter or Space chooses the focused item; the arrows only move the focus. An open popup comes after the page in the order of Tab.
- Under the pointer or focused from the keyboard an item is highlighted: filled with `Highlight`, the board's `accent`. An item whose submenu is open stays highlighted.
- A submenu opens beside its item, its first item level with it, under the pointer or on a press of the item (Enter or Space from the keyboard); the pointer on another item of the menu closes it. Choosing in it closes it and every menu it hangs from. A press in the menu while a submenu is open closes the submenu and still reaches what it pressed.
- Parts of the menu: `MenuPart::Panel`, `Separator` (the room around the line), `SeparatorLine`, `Label`; state `MenuState::Submenu`. Parts of an item: `MenuItemPart::Root` (what is pressed), `Icon`, `Label`, `Shortcut`, `Check` (the check or the dot), `Chevron`. States of an item: `MenuItemState::Checked` (checked, or the chosen radio), `Radio`, `Destructive`, `Disabled`, `Expanded` (its submenu is open).
- In the design the panel is at least 230 wide (a submenu 160), in `Popover` with `PopoverForeground`, padded 4, with the large radius, an edge of 1 in `Border` and the medium shadow. An item is 32 tall, padded 8, with the small radius and 8 between what it holds: an icon of 16 in `MutedForeground`, the label in the body's type, and at the end the shortcut in the mono type at 11 in `MutedForeground`, the check or the dot of 16, and the chevron of 16 in `MutedForeground`. A separator is a line of 1 in `Border` with 4 above and below; a group's label is 11/600 in `MutedForeground`, 8 in from the side. A destructive item is in `Destructive`, its icon too; a disabled one is dimmed.
- To what reads the tree the panel is a `menu`; an item a `menuitem`, a `menuitemcheckbox` or a `menuitemradio` (these two with `a11y_checked`) called by its label; an item with a submenu has `a11y_haspopup` `menu` and `a11y_expanded`; a separator is a `separator`.
- A renderer for `Menu` gets a `MenuView`: `view.panel()` is the panel with its style and its focus group, `view.item(item)`, `view.separator()` and `view.label(text)` build what it holds, and `view.render()` hangs it from the anchor. A renderer for `MenuItem` starts from `view.root()`, the behavior with the root's style, and puts in it `view.icon()`, `view.label()`, `view.shortcut()`, `view.check()`, `view.chevron()` or what it likes; `view.submenu()` is the submenu to put after it. Choosing it still closes its menu.

## For `CHANGELOG.md`

- `ui::Menu` and `ui::MenuItem`: a list of commands in a panel over the window, opened from an anchor, with icons, shortcuts, separators, group labels, checked, radio, destructive and disabled items and submenus; the arrow keys move between its items, and choosing one, a press outside and Escape close it (`Menu::new(anchor, open)`, `MenuItem::on_select`, `MenuItem::submenu`).

## Decided

- **Two themed components, `menu` and `menu_item`**, as the board has two (`Menu`, `Menu Item`): a theme styles or
  renders an item without touching the panel. The menu builds its items in its own theme and hands each the signals
  it closes and the submenus of its siblings (`MenuItem::in_menu`, crate-private).
- **The engine's `Popup`, not a component of its own.** `Popup::new(anchor, panel).open(open)` already closes on
  Escape and on a press outside, consumes that press, follows its anchor and is painted in the theme of the subtree
  it was declared in. The menu adds what choosing an item does, the gap of 4 from the anchor (along the placement's
  axis), the keyboard (a `FocusGroup`) and its anatomy. A builder is enough: what is open is the app's signal.
- **The panel is a vertical focus group of one stop.** Tab reaches the menu once, at its first item, and leaves it
  from inside, as a menu does; the arrows move between the items. `activate` is off: the choice does not follow the
  focus, Enter or Space presses the focused item as any button.
- **A radio item is the app's choice, as a checked one is**: `radio(chosen)` draws the board's `dot-circle` at the
  end of the chosen one and nothing at the others (the board reserves no room for the mark), and `on_select` tells
  the app. No group of radio items keeps a `Signal` (`Chosen::one`): the app's handler sets it, as for `checked`.
- **A submenu is a `Menu` given to its item**, with its own anchor and open signal, as every menu has: the item puts
  the anchor on itself, opens the submenu on a press and on the pointer's entering it, and closes the open submenus
  of the other items of its menu when the pointer enters it. The submenu's items close the submenu and every menu it
  hangs from. The submenu's popup lets a press in the menu through (`OutsidePress::PassThrough`), so a press on
  another item of the menu closes the submenu and presses that item, and its anchor is not a trigger that switches
  dropdowns (`switch_on_anchor_press(false)`). It opens beside the item back by the panel's padding, so its panel's
  edge is 4 from the menu's and its first item is level with the item; the default renderer puts it in a column
  with the item, where it takes no room.
- `Menu::new` takes `impl Into<OpenState>`, as `Popup::open` does: a `bool` holds a menu open for a gallery or a
  test, and then choosing an item calls its handler and closes nothing.
- **`inline(bool)`** draws the panel in place: the board draws its menus standing, the gallery's "Checked, radio,
  disabled, submenu" section does the same, and a test of the anatomy does not depend on where the popup is placed.
- **Highlighted is the pointer's and the keyboard's**: `hovered` and `focus_visible` of the item's root, and the
  `Expanded` state, fill it with `Highlight`. They set the same property to the same role, so the pointer's style
  does not take the keyboard's away. `force(Interaction::Hovered)` draws an item highlighted for a gallery.
- The panel's text is `PopoverForeground`, the role paired with its surface; the board writes the item's label in
  `$foreground`. Both are the same color in the light and the dark palette.
- The width is a least width (`min_width(230)`, a submenu's 160), not the board's fixed width: a menu with longer
  items grows. `.style(MenuPart::Panel, |s| s.min_width(..))` gives one its own.
- `ui::Button` takes no `ElementRef`, so the anchor is an element around the button (`Row::new().ref_element(..)`).
  In the split button the whole `ButtonGroup` is the anchor and the menu opens under it from its end
  (`Placement::BottomEnd`): a button inside a group cannot be wrapped.

## Not done

- **The first item is not focused when the menu opens, and the focus does not go back to the anchor when it
  closes.** Opened from the keyboard (Enter on its button), a menu shows no highlighted item until Tab reaches it,
  and once an item is chosen with Enter the focus is nowhere, so the next Tab starts from the top of the window.
  A builder cannot ask for focus: only `Ctx::focus(&ElementRef)` can, and an event handler has no `Ctx`. The
  smallest change is in the engine's overlays, with no new API for the menu but one flag:
  `Overlay::focus_on_open(bool)` (and `Popup::focus_on_open`), which, when the overlay's layer is shown and was not
  in the last layout, focuses the stop of Tab inside it that a one-stop focus group enters at (the
  `.focus_entry(true)` one, or the first), in the current input modality, so that it is highlighted after a key and
  not after a press of the pointer. Its other half is to give the focus back to the anchor when a layer that held
  it closes: `Tree::return_focus_to_anchor` (`app/runtime/overlay_layers.rs`) does that already for a layer that
  is hidden because its anchor is out of view (`focused_overlay_hidden`); it should run as well for a layer that
  is gone because its overlay closed. With the flag the menu sets it on its popup, and on its submenus, in one
  line each. A popover or a dialog opened from the keyboard is likely to want the same.
- **Right and Left do not open and close a submenu, and typing a letter does not go to the item it starts.** A
  key event reaches every node, focused or not, so the item with a submenu cannot tell that Right was pressed
  while it was the one focused (`KeyboardEvent::target_id` is the focused node, but a builder does not know its
  item's node id across renders); and going to an item by its letter needs a focus request. The smallest change:
  `on_focused_key_down`, a key handler that runs only while the node or one inside it is focused (the web's
  `keydown` bubbling from its target); with it the item opens its submenu on Right and the submenu's panel closes it
  on Left. Opening a submenu from the keyboard works today with Enter or Space on its item; its items are then
  reached by Tab, as the submenu's layer comes after the menu's, until `focus_on_open` puts the focus in it.
- Hover does not move the focus: an item focused from the keyboard and another under the pointer are both
  highlighted at once, where a native menu has one. Moving the focus on hover needs a focus request too (the same
  `Ctx::focus`), or an engine rule that a focus group's stop takes the focus when the pointer enters it.
- Tests are written (`crates/lurq/tests/ui/menu.rs`) and compile; per the playbook they were not run. The ones that
  depend most on the engine: the widths (an item as wide as a panel whose width is a least width, through
  `align_items(Stretch)`), the submenu's place (4 beside the menu, its first item level with its item), the pointer
  entering an item opening and closing submenus, and a submenu's items not being stops of the menu's focus group (a
  submenu is declared inside the menu's panel; if the engine counts the stops of a layer declared inside a group as
  the group's, Down from "Export as" reaches "PNG" while it is open, which is no harm, and the test of the arrows
  has no submenu).

## Differs from the board

- **The label of a group** has 5 above and under it; the board's reusable menu has 6 above and 4 under (`padding
  [6, 8, 4, 8]`): a part has one vertical padding. The height is the same. The board's "Dropdown menu" example draws
  its label 12/500 with 6 on both sides; the build follows the reusable component (11/600).
- **A disabled item** is dimmed by the theme's `OpacityLevel::Disabled` (0.5), as every disabled control of the
  design system is; the board's is 0.45.
- **The gap from the anchor** (4) is the board's gap between an open select and its options; the board draws no
  menu with its trigger. The board lays its submenu 4 beside the menu bottom-aligned with it, for room; the build
  levels its first item with the item it hangs from, as menus do.
- The weight of the icons, the check, the dot and the chevron is the icon font's; the board sets their stroke to
  1.333 in its 16 px instances.
