# Menu: hand-off

`ui::Menu` and `ui::MenuItem`, from the board's `Menu` (`iPsCT`) and `Menu Item` (`q4kDx`) in "Select & Menu" of
Components / light (`Ovt1D`): the board's "Menu · checked, radio, disabled, submenu" (`NKwJ3`) for the checked and
radio items, the disabled one and the submenu, and its "Dropdown menu" example (`RxliJ`) for the icons, the group
label and the destructive item.

## Changed outside the component's own files

Changes to the engine, each what a menu needs of it, each tested by `crates/lurq/tests/ui/menu.rs` (run with
`ui_tests menu`, `popover`, `dialog`, `tooltip` and `input_tests`):

- **`Overlay::takes_focus(bool)` and `Popup::takes_focus(bool)`**, off by default, shared with `ui::Popover`,
  which built the mechanism (see `design/handoff/popover.md`, where it was `move_focus`; the two were made one and
  the name kept is the one beside `Modal::takes_focus`). As the overlay opens the focus goes to the first element
  in it that takes the focus; Tab goes through its elements right after its anchor's; as it closes with the focus
  inside, the focus goes back. From the menu it took where the focus goes back to: to what had the focus when the
  overlay opened (`Tree::overlay_focus_returns`, kept by anchor in `Tree::note_focus_returns`, read by
  `return_focus_to_anchor` in `app/runtime/overlay_layers.rs`), where that is still in the tree, and only else to
  the anchor. A split button's anchor is the whole group: its focus goes back to the chevron that opened the menu,
  not the group's first button. An app's focus request in the same pass wins, and the ring shows after a key and
  not after a press of the pointer, as before.
- **`KeyboardEvent::focus_within`** (`app/events/keyboard_event.rs`, `runtime.rs`): a key event reaches every
  node's `on_key_down`; this says whether the keyboard's focus is on the node whose handler is called or inside it.
  `fire_keyboard_recursive` and `fire_keyboard_up_recursive` take the path to the focused node and set it. The item
  of a submenu opens it on Right, and the submenu's panel closes it on Left, only while the focus is on them.
- **`FocusGroup::follow_pointer(bool)`** (`node/focus_group.rs`, `runtime/hover.rs`, `runtime.rs`), off by
  default: while the focus is in such a group, the pointer coming onto one of its elements moves the focus there (in
  the pointer's modality, so with no ring of its own), so that what the pointer and the keyboard are on are one. It
  never takes the focus into the group from outside, and only an element the pointer has just come onto takes it
  (`hover::pointer_focus`), from a pointer move (`dispatch_mouse`) or a relayout under a still pointer
  (`refresh_hover_at_pointer`).
- With it: **an arrow key that moves the focus in a group that follows the pointer takes the hover away** until the
  pointer moves (`Tree::hover_held_by_keys`, set in `focus_group_key`, cleared by `track_pointer`, read by
  `refresh_hover_at_pointer`), so that the item the keys moved to is the only one lit, as in a native menu.

`docs/src/content/docs/focus-navigation.md` could say, in Focus Groups: `follow_pointer(bool)` | the pointer coming
onto an element of the group moves the focus there, while the focus is in the group; and the Popups section of
`modals.md`: `takes_focus(true)` moves the focus into a popup as it opens and back as it closes.

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
- The keyboard: a menu opened from the keyboard (Enter or Space on its button) has its first item focused and lit; opened by the pointer it has the focus there too, unlit. Up and Down move between its items and go round at the ends, Home and End go to the first and the last, and a disabled item is passed by. Enter or Space chooses the focused item; the arrows only move the focus. Escape, or choosing, closes it and gives the focus back to its button. Tab stops in an open menu once (the panel is a [focus group](../focus-navigation/#focus-groups) of one stop).
- The pointer and the keyboard are on one item: the pointer coming onto an item moves the focus there, and an arrow key moves the light off the item under a pointer that holds still.
- Under the pointer or focused from the keyboard an item is highlighted: filled with `Highlight`, the board's `accent`. An item whose submenu is open stays highlighted.
- A submenu opens beside its item, its first item level with it, under the pointer, on a press of the item, or on Right, Enter or Space while the item is focused, with the focus on its first item; the pointer on another item of the menu closes it, and so do Left and Escape while the focus is in it, giving the focus back to its item. Choosing in it closes it and every menu it hangs from. A press in the menu while a submenu is open closes the submenu and still reaches what it pressed.
- Parts of the menu: `MenuPart::Panel`, `Separator` (the room around the line), `SeparatorLine`, `Label`; state `MenuState::Submenu`. Parts of an item: `MenuItemPart::Root` (what is pressed), `Icon`, `Label`, `Shortcut`, `Check` (the check or the dot), `Chevron`. States of an item: `MenuItemState::Checked` (checked, or the chosen radio), `Radio`, `Destructive`, `Disabled`, `Expanded` (its submenu is open).
- A menu is as wide as its widest item, and at least as wide as the design says; never as wide as the window. In the design the panel is at least 230 wide (a submenu 160), in `Popover` with `PopoverForeground`, padded 4, with the large radius, an edge of 1 in `Border` and the medium shadow. An item is 32 tall, padded 8, with the small radius and 8 between what it holds: an icon of 16 in `MutedForeground`, the label in the body's type, and at the end the shortcut in the mono type at 11 in `MutedForeground`, the check or the dot of 16, and the chevron of 16 in `MutedForeground`. A separator is a line of 1 in `Border` with 4 above and below; a group's label is 11/600 in `MutedForeground`, 8 in from the side. A destructive item is in `Destructive`, its icon too; a disabled one is dimmed.
- To what reads the tree the panel is a `menu`; an item a `menuitem`, a `menuitemcheckbox` or a `menuitemradio` (these two with `a11y_checked`) called by its label; an item with a submenu has `a11y_haspopup` `menu` and `a11y_expanded`; a separator is a `separator`.
- A renderer for `Menu` gets a `MenuView`: `view.panel()` is the panel with its style and its focus group, `view.item(item)`, `view.separator()` and `view.label(text)` build what it holds, and `view.render()` hangs it from the anchor. A renderer for `MenuItem` starts from `view.root()`, the behavior with the root's style, and puts in it `view.icon()`, `view.label()`, `view.shortcut()`, `view.check()`, `view.chevron()` or what it likes; `view.submenu()` is the submenu to put after it. Choosing it still closes its menu.

## For `CHANGELOG.md`

- `ui::Menu` and `ui::MenuItem`: a list of commands in a panel over the window, opened from an anchor, with icons, shortcuts, separators, group labels, checked, radio, destructive and disabled items and submenus; it takes the focus as it opens and gives it back as it closes, the arrow keys move between its items and Right and Left into and out of a submenu, and choosing one, a press outside and Escape close it (`Menu::new(anchor, open)`, `MenuItem::on_select`, `MenuItem::submenu`). For it the engine has `Popup::takes_focus`, `FocusGroup::follow_pointer` and `KeyboardEvent::focus_within`.

## Decided

- **Two themed components, `menu` and `menu_item`**, as the board has two (`Menu`, `Menu Item`): a theme styles or
  renders an item without touching the panel. The menu builds its items in its own theme and hands each the signals
  it closes and the submenus of its siblings (`MenuItem::in_menu`, crate-private).
- **The engine's `Popup`, not a component of its own.** `Popup::new(anchor, panel).open(open)` already closes on
  Escape and on a press outside, consumes that press, follows its anchor and is painted in the theme of the subtree
  it was declared in. The menu adds what choosing an item does, the gap of 4 from the anchor (along the placement's
  axis), the keyboard (a `FocusGroup`) and its anatomy. A builder is enough: what is open is the app's signal.
- **The panel is a vertical focus group of one stop that follows the pointer**, and the popup takes the focus. Tab
  reaches the menu once and leaves it from inside, as a menu does; the arrows move between the items. `activate` is
  off: the choice does not follow the focus, Enter or Space presses the focused item as any button. A menu opened by
  the pointer has the focus on its first item, unlit, so a first Down goes to the second, where a native menu's first
  Down goes to the first. The highlight stays the pointer's hover and the keyboard's `focus_visible`, so an inline
  menu, whose focus is outside it, is still lit under the pointer.
- **No part grows to fill its row.** An item holds its icon and label at its start and its shortcut, mark and chevron
  at its end, in two rows the item's width keeps apart (`Justify::SpaceBetween`), and a separator's line has no
  width of its own, stretched by its room: the panel stretches them to its width, which is its widest item's or its
  least width. A child with `flex(1.0)` grows into all the room its row is offered, which in an overlay is the
  window's: the first build had the label and the separator's line grow, and its menus were as wide as the window.
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

- **Typing a letter does not go to the item it starts.** The engine has no text to match a stop of a focus group
  against: an item's `a11y_name` is kept on its node only with the `devtools` or `mcp` feature
  (`NodeUpdate::describe`), and its text is a label beside an icon that is a glyph of text too. The smallest change:
  `FocusGroup::typeahead(bool)` in `focus_group_key`, matching a single printable key, from the stop after the
  focused one and round, against a label the node keeps in every build (`describe("a11y_name", ..)` stored always,
  or a `focus_label` of its own). The menu turns it on in one line.

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
