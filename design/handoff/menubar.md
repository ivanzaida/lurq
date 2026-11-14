# Menubar

## Changed outside the component's own files

- `crates/lurq/src/node/focus_group.rs`: `FocusGroup` has a field more, `menu_row`, set in `FocusGroup::new` and by
  the new `pub fn menu_row(bool)`. The TreeView session of this batch adds to `FocusGroup` too: the two fields and
  their lines in `new` sit next to each other, and both are kept.
- `crates/lurq/src/app/runtime/menu_row.rs` (new): `Tree::note_alt_press`, `Tree::menu_row_key`,
  `Tree::menu_row_mnemonic`, `Tree::menu_row_alt`, `Tree::press_in_row`, `Tree::close_layers_from`,
  `Tree::focus_place`, `Tree::first_menu_row`, and the free functions `menu_row_around`, `find_menu_rows`,
  `entry_at`.
- `crates/lurq/src/app/runtime.rs`:
  - `mod menu_row;`.
  - `Tree` has two fields more, `alt_alone: bool` and `menu_row_return: Option<FocusTarget>`, after `typeahead`, set
    in `Tree::new`.
  - `Tree::dispatch_key_down`: calls `self.note_alt_press(&key, ctrl, meta)` after `note_key_press_modality`, and has a
    branch more in the chain of defaults, after the focus group's keys and typeahead and before Enter and Space:
    `else if !ctrl && !meta && self.menu_row_key(&key, alt, click_modifiers)`.
  - `Tree::dispatch_key_up`: after the handlers, an Alt released with nothing pressed since it went down, and not
    prevented by a handler, calls `self.menu_row_alt()`.
  - `Tree::dispatch_mouse_down`: a press sets `alt_alone` to `false`.
- `crates/lurq/src/app/runtime/overlay_layers.rs`: `path_to_ref` is `pub(super)` (was private), for `menu_row`.
- `crates/lurq/src/app/runtime/typeahead.rs`: `label` is `pub(super)` (was private), for `menu_row`.
- `crates/lurq/src/ui/menu.rs`:
  - `impl From<MenuItem> for MenuEntry`, so that a menubar's `menu(title, entries)` takes items and separators in one
    list.
  - `MenuItem` and `MenuItemView` have a field more, `autofocus`; `MenuItem::autofocus(bool)` (pub(crate)) sets it and
    `MenuItemView::root` gives the root `.autofocus(true)` for it: the item the focus goes to as its menu opens (a
    menubar's menu opened with Up opens at its last item).
  - `MenuItem::is_disabled()` (pub(crate)).
- `crates/lurq/tests/input/tab_order/menu_row.rs` (new), registered in `tests/input/tab_order/mod.rs`: the engine's
  tests of a row of menus, in `input_tests`. Written, not run.
- No keyframes.

## For `docs/src/content/docs/design-system.md`

## Menubar

The menus of an app in a row, File, Edit, View, each opened from its title, as the title bar of a window holds them on Windows and Linux.

```rust
use lurq::ui::{Menu, MenuEntry, MenuItem, Menubar, icons};

// open: Signal<Option<usize>>, the menu open now; export: ElementRef, exporting: Signal<bool> for a submenu.
Menubar::new(open)
  .menu("File", [
    MenuItem::new("New").icon(icons::FILE).shortcut("Ctrl+N").on_select(move || new_file()).into(),
    MenuItem::new("Open…").shortcut("Ctrl+O").on_select(move || open_file()).into(),
    MenuEntry::Separator,
    MenuItem::new("Export as").submenu(Menu::new(export, exporting).items(["PNG", "SVG", "PDF"].map(MenuItem::new))).into(),
    MenuEntry::Separator,
    MenuItem::new("Quit").shortcut("Ctrl+Q").on_select(move || quit()).into(),
  ])
  .menu("Edit", [
    MenuItem::new("Undo").icon(icons::UNDO).shortcut("Ctrl+Z").on_select(move || undo()),
    MenuItem::new("Redo").shortcut("Ctrl+Y").disabled(true),
  ])
  .menu("View", [MenuItem::new("Show grid").checked(grid.get()).on_select(move || toggle_grid())])
  .menu("Help", [MenuItem::new("About")])
  .title_disabled("Help", !online.get());
```

| `Menubar` | |
| --- | --- |
| `new(open)` | Which menu is open, by its index among the bar's: the app's `Signal<Option<usize>>`. A press of a title sets it; choosing an item, a press outside and Escape set it to `None`; the app may set it too |
| `menu(title, entries)` | A menu called `title`, after the bar's others, holding [menu](#menu) items: `MenuItem`s, with `MenuEntry::Separator` and `MenuEntry::Label` between them (`.into()` each where the list mixes them) |
| `title_disabled(title, bool)` | The menu of that title cannot be opened: its title is dimmed, and the pointer, the arrows and Alt pass it by |
| `disabled(bool)` | No menu of the bar can be opened |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the bar's; its titles are `{id}-0`, `{id}-1` and on, and each menu's panel `{id}-0-menu` |

- Its menus are [menus](#menu), with all their items have: icons, shortcuts, checks, radio items, submenus, disabled and destructive items, separators and labels. One is open at a time.
- The pointer: a press of a title opens its menu, and a press of it again closes it. While a menu is open the pointer coming onto another title opens that one, with no press. Choosing an item, a press outside and Escape close the menu.
- The keyboard: the bar is one stop of Tab (a [focus group](../focus-navigation/#focus-groups) of one stop). Left and Right move between the titles and go round at the ends, a letter goes to the title that starts with it, and a disabled title is passed by. Down, Enter and Space open the focused title's menu with the focus on its first item, Up on its last that can be chosen. In an open menu Left and Right close it and open the menu beside it, at its first item: Left in a submenu closes the submenu first, Right on an item with a submenu opens it, and Right on one without goes to the next title from a submenu too. Escape closes the menu and gives the focus back to its title.
- Alt: pressed and let go alone, Alt puts the focus on the first title and underlines the first letter of each title; Alt alone again gives the focus back to what had it, closing a menu that was open. The letters show while Alt is held down too. Alt with a title's first letter opens its menu from wherever the focus is, and from a menu of the bar opens the other; several titles with one first letter are gone round. On a Mac, where Option with a letter types a character and the app's menus are the system's, Alt does neither.
- Under the pointer, focused from the keyboard, or with its menu open, a title is filled with `Highlight`, the board's `accent`. In the design the bar is 36 tall in `Background`, padded 4, with the medium radius and an edge of 1 in `Border`, its titles 2 apart; a title is 28 tall, padded 10 at its sides, with the small radius, its text 13/500 in `Foreground`. A disabled title is dimmed. A menu opens under its title from its start, 4 below it.
- The engine's part is `FocusGroup::menu_row(true)`, which the bar's row has: a row of elements each of which opens a menu hung from it when it is pressed. While one of those menus has the focus, the Left and Right that nothing in the menu takes close it, with the menus opened from it, and press the element beside it; Alt with a letter and Alt alone act on the first such row of the window. A bar of your own, of buttons and [menus](#menu), gets the same keys from it.
- The bar keeps, for the signal it is given, a signal for each menu kept as one with the app's, the anchor each menu hangs from, and what Alt shows, while it is in the tree.
- Parts: `MenubarPart::Bar`, `Title` (what is pressed, and what is lit), `Label` (its text). States: `MenubarState::Open` (the title's menu is open) and `Disabled` (the bar, or the title, cannot be opened); a title is resolved in the states of the bar and its own.
- To what reads the tree the bar is a `menubar`, each title a `menuitem` called by its text with `a11y_haspopup` `menu` and `a11y_expanded`.
- A renderer for `Menubar` gets a `MenubarView`: `view.bar()` is the row with its style, its focus group and its handlers; `view.title(index)` a title's behavior with its style, `view.label(index)` its text, and `view.menu(index)` its menu, hung from the title, to put after it; `view.title_part(index, part)` is the style of a part of one title. `view.render()` builds the bar.

On a Mac an app's menus belong in the system's menu bar, which `App::set_menu_bar` fills (see [native menus](../window-lifecycle-menus/#building-a-native-menu)): there `app.menu_bar_support()` is `MenuBarSupport::Native`, and the app shows no `Menubar`. The native model routes a command by its id (`MenuAction::new(id, label)`, `app.on_menu_activate`), where a `MenuItem` calls a handler and has no id; and it has no check or radio item. So one list feeds both from the app's own description of its commands, built into each:

```rust
use lurq::app::{Accelerator, Menu as NativeMenu, MenuAction, MenuBarSupport};
use lurq::ui::{MenuItem, Menubar};

struct Command { id: &'static str, label: &'static str, key: Option<&'static str> }
const FILE: [Command; 2] = [
  Command { id: "new", label: "New", key: Some("n") },
  Command { id: "open", label: "Open…", key: Some("o") },
];

if app.menu_bar_support() == MenuBarSupport::Native {
  // menu_bar: the app's MenuBar, its application menu as it likes
  menu_bar.menus = vec![NativeMenu {
    title: "File".into(),
    items: FILE.iter().map(|c| {
      let action = MenuAction::new(c.id, c.label);
      match c.key { Some(key) => action.accelerator(Accelerator::command(key)), None => action }.into()
    }).collect(),
  }];
  app.set_menu_bar(menu_bar);
  app.on_menu_activate(move |id| run(id));
} else {
  // In the render, the same commands as a drawn bar.
  let file = FILE.map(|c| {
    let item = MenuItem::new(c.label).on_select(move || run(c.id));
    match c.key { Some(key) => item.shortcut(&format!("Ctrl+{}", key.to_uppercase())), None => item }
  });
  Menubar::new(open).menu("File", file);
}
```

## For `CHANGELOG.md`

- `ui::Menubar`: the menus of an app in a row, File, Edit, View, of the `MenuItem`s a `Menu` has. A press of a title opens its menu, and with one open the pointer on another title opens that one; the bar is one stop of Tab, Left and Right move between its titles and hand an open menu to the one beside it, Down, Enter and Space open a menu at its first item and Up at its last, Alt alone puts the focus on the bar with the titles' first letters underlined, and Alt with a letter opens that title's menu. Which menu is open is the app's `Signal<Option<usize>>`. The engine's part is `FocusGroup::menu_row`, which a bar of buttons and menus of an app's own can have too. See [Menubar](docs/src/content/docs/design-system.md#menubar).

## Decided

- **Which menu is open is a `Signal<Option<usize>>` the app gives, and the bar is a builder, not a mounted component.** A `Menu` takes a `Signal<bool>` and an anchor, and both must be the same from one build to the next: the runtime knows a menu that has just opened (and moves the focus into it) by its anchor, and a `Popup`'s open signal is what Escape, a press outside and a chosen item set. So the bar keeps, for the app's signal, a `Signal<bool>` and an `ElementRef` for each menu, in a map keyed by the app's signal and held while the bar's handlers are in the tree, as `ContextMenu` keeps its point and anchor. Two watchers keep each menu's signal and the app's as one: a menu that closes itself sets the app's to `None`, and the app's opens the menu it names and closes the others. A mounted component (as `Toasts`' host) would keep the same things with a `Ctx` and no map, but its props would have to be the menus, and a `MenuItem` holds handlers and a submenu and is not `Clone`: the app would build the bar's items in a component of their own, or the bar would need its items twice. The builder stands among the others where the app's title bar is built, and the app can open a menu and know which is open, which the Toasts shape would give through a controller of its own.
- **The open menu handed to its neighbour and Alt are the engine's, as `FocusGroup::menu_row`**, not the bar's handlers. An open menu is a layer of the overlay host, out of the bar's subtree: the bar's handlers do not know that the focus is in its menu, and a key reaches the bar before it reaches the menu, so the bar cannot tell a Left the submenu takes from one it leaves. The runtime acts after every handler and only on what none prevented, so a submenu's Left and an item's Right stay theirs with no change to `Menu`. Alt alone moves the focus, which a builder cannot do.
- **A title's mnemonic is its first letter.** The engine matches Alt with a letter against the label of each element of the row, as typeahead does, so it needs nothing of the bar; a title has no `&` to mark another letter. Two titles with one first letter are gone round, the second press of Alt with it going to the next, as Windows does.
- **The pointer coming onto a title opens it only while another menu is open**, as every desktop menubar does; at rest it lights the title.
- **A title opens on a press and release (a click), as every control of the design system does, not on the press alone.** Press, drag onto an item and release does not choose the item: `ui::Menu` chooses an item on a click, not on a release after a press elsewhere, so it is not built here.
- **Alt with a title's letter leaves the focus on the title under its menu**: the engine focuses the title before it presses it, so Escape gives the focus back to the title, as from Down or Enter.
- **On a Mac Alt does nothing for the row.** Option with a letter types a character there, and the app's menus are in the system's menu bar.
- **One list for the drawn bar and the native menu is the app's own list of commands, built into each, and not a conversion of the bar's.** What the native model (`lurq::app::MenuBar`) lacks for a conversion from `ui::MenuItem`s: an item there is routed by a string id through `on_menu_activate`, where a `MenuItem` has a handler and no id of its own (its `id` is the element's, for tests); there is no check or radio item (`MenuAction` has no `checked`); no icon; no label of a group (`MenuEntry::Label`); and a shortcut is an `Accelerator` of a key and modifiers where a `MenuItem`'s is text it only shows. With a command id and a checked state on `MenuAction` and an id on `MenuItem`, a `Menubar` could give the native bar its menus.
- **A disabled bar or title** dims the title (the menu item's disabled opacity) and takes no press, no pointer, no arrow and no Alt; the board draws neither, and the menu's disabled item is the pattern.

## Not done

Nothing.

## Differs from the board

- The board draws the bar at rest with File filled in `$accent`, the state of a title whose menu is open (or under the pointer); it draws no open menu, no disabled title and no underlined letter. The open menu is the board's Menu, under its title; a disabled title is dimmed as a disabled menu item is; the letter is underlined with the font's underline in the text's color.
- The board's `Window Chrome` and its variants hold no menubar: where a bar stands in a title bar is the app's, in `WindowChrome`'s title bar or under it.
- The board's `Menu Button` (`wbAkK`, a button with a label and a chevron, used in the table toolbar and the small controls) and `Edit Menu` (`Y2mnD`, a row of Cut, Copy, Paste, Select All) are not part of the bar: the first is an outline `ui::Button` with a trailing chevron that opens a `ui::Menu`, as the Menu page's dropdown does, and the second is `ContextMenu::bar`, already built.
