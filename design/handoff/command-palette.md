# Command Palette and Context Menu: hand-off

`ui::CommandPalette` and `ui::ContextMenu`, from the board's `Command Palette` (`v4CHi`), `Context Menu` (`ZvlY5`),
`Edit Menu` (`Y2mnD`) and `Overflow Menu` (`p4lLjx`) in Components / light (`Ovt1D`), where they are used (the palette in
Overlays › Panels, `jNAcP` and `ZfFTA`; the context menu in Shell & Toolbars › Tools, `C92rUr`; the edit menu in Desktop
Tools › Text & media, `d8775`; the overflow menu in Shell & Toolbars › Tools, `nuMay`), the Menu Item (`q4kDx`) and the
Kbd (`zGoRU`) they are built of, and `Mobile/Context Menu` (`GO5Be`) and `Tablet/Context Menu` (`GJks9`), read for the
parts a theme would restyle into them.

Files: `crates/lurq/src/ui/command_palette.rs`, `crates/lurq/src/ui/context_menu.rs`,
`crates/lurq/tests/ui/command_palette.rs`, `crates/lurq/tests/ui/context_menu.rs`,
`crates/gallery/src/pages/command_palette.rs` and `context_menu.rs` (pages "Command Palette" and "Context Menu", group
Surfaces & overlays), `design/handoff/command-palette.md`, and lines in the three `mod.rs` files.

## Changed outside the component's own files

This session owns a popup placed at a point of the window rather than at an element; the engine change is of that kind.

- **Engine: `Overlay::at(Option<(f32, f32)>)` and `Popup::at(..)`**: a point of the window, in logical pixels, that the
  overlay stands at in place of its anchor. It opens down and toward the end from the point, `offset` away, and on the
  other side of the point along an axis where the window has no room that way (and the other side has); the collision
  strategy's clamp then keeps it inside the window. The placement is not used. The anchor is still what the overlay
  hangs from: a press on it is not outside it, the focus goes back to it, it is hidden while the anchor is out of view.
  Files: `app/ctx.rs` (field `at` on `Overlay` and `OverlaySpec`, in `clone_with_node` and `into_spec`, the builder),
  `components/popup.rs` (the builder), `app/runtime.rs` (`OverlayPlacing::at`, set in `build_overlay_node`;
  `place_overlay` calls the new `point_position` in place of `resolve_overlay_collision` and `overlay_position` when
  it is set). Tests in `crates/lurq/tests/runtime/overlay.rs` (`overlay_at_a_point_*`, three), written and not run.
- `crates/lurq/src/ui/menu.rs`: `Menu::context(point)` (`pub(crate)`) and the field it sets on `Menu` and `MenuView`:
  the menu's popup stands at the point (`Popup::at`) with no offset, or, for `None` (opened from the keyboard), at its
  anchor as a menu does; either way its anchor is no trigger of a dropdown (`switch_on_anchor_press(false)`). A menu
  that is not a context menu is built as before.
- `crates/lurq/src/ui/part_style.rs`: `PartStyle::max_height` (the field after `max_width`, its setter, in `is_empty`,
  `merge` and `apply`), mirroring `max_width`: the most height of the palette's list, past which it scrolls. If another
  session of the batch added the same, the merge keeps one.
- `crates/gallery/src/shell.rs`: the gallery's search is the command palette. The header's field is now what opens it
  (a press, Enter or Space; it looks as the field did), Ctrl+K opens it, and its commands are the pages in the side
  bar's groups, found by name and by their `about` words; running one goes to the page. The side bar is no longer
  filtered, and the shell keeps `search_open` in place of the search's text, field, focus and the `find` counter.
  **Merge note**: the empty-state session also changes this file (`keys(find_chord())` becomes
  `Kbd::shortcut("Mod+K").platform(KeyPlatform::current())`, `find_chord` removed). Take this session's
  `search_field` and put their `Kbd` in place of `keys(find_chord())` in it.

No keyframes were added.

## For `docs/src/content/docs/design-system.md`

## Command Palette

A search over the window that finds a command or a place by typing: every command of an editor, the pages of an app, the files of a project.

```rust
use lurq::ui::{CommandItem, CommandPalette, icons};

CommandPalette::new(open.clone())
  .placeholder("Type a command or search")
  .group("Suggestions", [
    CommandItem::new("Toggle dark mode").icon(icons::MOON).shortcut("Ctrl+D").keywords(["theme"]).on_select(toggle),
    CommandItem::new("Open DevTools").icon(icons::TERMINAL).shortcut("F12").on_select(devtools),
  ])
  .group("Files", files.iter().map(|file| CommandItem::new(&file.name).icon(icons::FILE).detail(&file.folder)))
  .footer(true);

// Commands an app fetches as a person types: it keeps every one it was given.
CommandPalette::new(open.clone())
  .query(query.clone())
  .on_input(move |typed| fetched.set(search(typed)))
  .filter(|_, _| true)
  .loading(searching.get())
  .items(fetched.get().iter().map(|hit| CommandItem::new(&hit.title)));
```

| Builder | |
| --- | --- |
| `CommandPalette::new(open)` | Shown while the app's `Signal<bool>` holds `true`. Running a command, Escape on an empty input and a press on the veil set it to `false` |
| `placeholder(&str)` | What the input says while nothing is typed: "Type a command or search" |
| `item(CommandItem)`, `items(..)` | Commands with no group |
| `group(&str, items)` | Commands under a label: "Suggestions". A group with nothing shown for what is typed is not shown |
| `filter(Fn(&CommandItem, &str) -> bool)` | Which commands are shown while something is typed. By default (`matches_every_word`) those whose label, detail or keywords hold every word typed, without regard to case, in the order the app gave them |
| `on_input(Fn(&str))` | Called with what the input will hold at each edit: for an app that fetches its commands |
| `query(Signal<String>)` | Where the app keeps what is typed. Without one the palette keeps it while it is open, and it is empty when it opens again |
| `loading(bool)` | The list ends with a spinner and "Loading…" |
| `empty(&str)` | What the list says when nothing is found: "No results" |
| `footer(bool)` | A row at the bottom, under a line, of the keys that work in it: ↑ ↓ to move, ↵ to run, Esc to close |
| `inline(bool)` | The panel drawn where it is declared, open or not, without the veil: for a gallery or a test |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's |
| `CommandItem::new(label)` | A command: what it does, or where it goes |
| `icon(name)`, `detail(&str)`, `shortcut(&str)` | An icon of 16 before the label; a second text after it, muted; the keys that do the same at the end, as a [menu](#menu) writes them (the app binds them) |
| `keywords([..])` | Words that find it and are not shown |
| `disabled(bool)`, `on_select(Fn())`, `id`, `key` | Dimmed, and passed by; called when it is run, before the palette closes; the row's id and key |

- It stands near the top of the window over the veil, as the engine's `Modal`: the focus goes to its input as it opens and back to what had it as it closes, and Tab stays inside it, as a [dialog](#dialog)'s does.
- The focus stays in the input. Down and Up move the highlight through the rows that are shown, across groups and round at the ends, past a disabled one, and scroll the list to it; the pointer moves it too, and a row shows it as a menu item shows the keyboard's focus. Enter runs the highlighted command and closes the palette; a click on a row does the same. Escape clears what was typed, and closes the palette when nothing is. Typing puts the highlight on the first row it shows.
- To what reads the tree it is a `dialog` holding a `combobox` (the input, `expanded`, with the highlighted command's label as its `a11y_activedescendant`) and a `listbox` of `option`s with `a11y_selected`.
- Parts: `CommandPalettePart::Veil`, `Panel`, `Search` (the row at the top), `SearchIcon`, `Input` (laid over the theme's input), `Key` (the Escape at the end of the search row and the keys of the footer), `Results` (what scrolls: its most height), `List` (the room around the rows), `GroupLabel`, `Row` (laid over a menu item's root), `Detail`, `Empty`, `Status`, `Footer`, `FooterHint`. The rows are a [menu](#menu)'s items: a theme's rules for `MenuItem` style them too. States: `CommandPaletteState::Loading`, `Empty`.
- In the design the panel is 420 wide on `Popover` in an edge of 1 in `Border`, `radius-xl`, the large shadow, 72 from the top of the window over `Scrim`. The search row is 44 high, 14 in, with 10 between the magnifier of 16 in `MutedForeground`, the input in the body's type (no edge or fill of its own) and the key, and a line of `Border` under it. The list is padded 4 and scrolls past 320; a group's label is 12/500 in `MutedForeground`, padded 6 and 8; a row is a menu item, 32 high on `Highlight` while it is highlighted. A key is 20 high, padded 6, 4 round, on `Muted` in an edge of `Border`, in the mono type at 11 in `MutedForeground`.
- A renderer that draws another palette has `view.panel()`, `view.search()`, `view.input()`, `view.key(label)`, `view.results()`, `view.group_label(label)`, `view.row(index)`, `view.empty_state()`, `view.status()`, `view.footer_row()`, `view.veil(panel)` and `view.modal(panel)`, with `view.shown()`, `view.command(index)` and `view.highlighted`.

## Context Menu

A [menu](#menu) of what can be done with an element, opened where the element is pressed with the secondary button or held with a finger: a file in a list, a node of a canvas, selected text.

```rust
use lurq::ui::{ContextMenu, MenuItem, icons};

ContextMenu::new(file_row, open.clone())
  .item(MenuItem::new("Open").icon(icons::FOLDER_OPEN).on_select(open_file))
  .item(MenuItem::new("Rename").icon(icons::PENCIL).shortcut("F2").on_select(rename))
  .separator()
  .item(MenuItem::new("Delete").icon(icons::TRASH).destructive(true).on_select(delete));

// The board's edit menu: the commands of selected text in one row.
ContextMenu::new(editor, editing.clone())
  .bar(true)
  .items(["Cut", "Copy", "Paste", "Select All"].map(MenuItem::new));
```

| Builder | |
| --- | --- |
| `ContextMenu::new(target, open)` | The element the menu is for, which lays out as it would without it, and the app's `Signal<bool>`: what asks for the menu sets it, and choosing an item, a press outside and Escape clear it |
| `item(MenuItem)`, `items(..)`, `separator()`, `label(&str)` | As a [menu](#menu) has them: icons, shortcuts, checks, radios, submenus, a destructive item |
| `bar(bool)` | The commands in one row, as the board's edit menu over selected text has them. Left and Right move between them; a separator is a line down the row, and a label is left out |
| `disabled(bool)` | The element has no menu now |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's |

- It opens at the pointer on a press of the secondary button on the element (and on Control and a press on macOS), at the finger on a long press, and under the element from its start on the Menu key or Shift+F10 while the focus is in the element. A press asks for the menu of the innermost element that has one; asked for again on its element, it moves to the new point.
- It stays inside the window: where there is no room down or toward the end of the point, it opens up or back from it. One context menu is open at a time: asking for another closes the first.
- It is a [menu](#menu): the focus goes to its first item as it opens and back to what had it as it closes, Up and Down move between the items, a letter goes to the item it starts, Enter chooses, a submenu opens beside its item. To what reads the tree it is a `menu`, and the element says it has one (`a11y_haspopup`).
- Parts: `ContextMenuPart::Panel` (laid over the menu's panel), `Bar` (the panel of a bar), `BarItem` and `BarLabel` (laid over a menu item's root and label in a bar); the menu's and its items' parts are a menu's. State: `ContextMenuState::Bar`.
- In the design the menu is a menu at least 220 wide, its corner at the pointer. The bar is a row on `Popover` padded 3, `radius-lg`, in an edge of `Border` with the large shadow; its items are 28 high, padded 10, their labels 13.
- The engine places it: `Popup::at(Some((x, y)))` (`Overlay::at`) stands a popup at a point of the window and keeps it inside, which an app may use for a popup of its own.

## For `CHANGELOG.md`

- `ui::CommandPalette` and `ui::CommandItem`: a search over the window that finds a command or a place by typing, near the top over a veil, with groups, icons, details, shortcuts and keywords, a filter of every word (or the app's), the focus kept in its input while Down and Up move a highlight round the rows, Enter running and Escape clearing then closing, an empty and a loading list, a footer of its keys, and commands an app fetches; the gallery's search is one.
- `ui::ContextMenu`: a menu opened at the pointer by a secondary press, at the finger by a long press, or at its element by the Menu key and Shift+F10, kept inside the window, one open at a time, with a bar for the edit menu of selected text; the engine's `Overlay::at` and `Popup::at` stand a popup at a point of the window.

## Decided

- **The palette's rows are a menu's items drawn by the palette**: it resolves the theme's `MenuItem` recipe (with the palette's `Row` laid over the root) and draws the root, the icon, the label and the shortcut with those parts, so rows look as menu items do and a theme's menu item rules reach them. It does not use `MenuItem`'s own renderer, whose root is a button and a stop of Tab: the focus is to stay in the input. The highlight is the menu item's focus style, forced on the highlighted row; the hover style is taken off the rows, so that the pointer and the keys never highlight two rows at once.
- **The input is `ui::Input`** in its view (`Input::view`, `InputView::control`, `render_with_end`), with the palette's `Input` part laid over the control: no edge, no fill and no ring of its own, as the board's search row has none. The control takes the palette's keys and says to tooling it is a `combobox`.
- **What a palette keeps (what is typed, the highlight, where its list is scrolled) is kept for its open signal**, while the palette is in the tree, as a combobox keeps what is typed for its value: a map of the crate, keyed by the signal, whose entry goes when nothing holds it. The input's handlers hold it, so it goes when the palette closes, and the palette opens empty. A mounted component would have needed a `Ctx` at the call site, which `CommandPalette::new(open)` has not. The context menu keeps the point it was asked for in the same way, held by its element's handlers.
- **The default filter keeps the app's order.** Every word typed must be in the label, the detail or the keywords; there is no ranking, because no rank of "best" would be right for every app, and a palette whose rows move about as one types is harder to aim at. An app that ranks gives `filter(|_, _| true)` and its commands in its order.
- **Typing puts the highlight on the first row** the new text shows, from the input's `on_input`, which knows the commands; when the app's commands change under it, the highlight is the nearest row that can be run.
- **Escape is the palette's**, not the modal's (`Modal::dismiss_on_escape(false)`): it clears first and closes second.
- **Near the top, not in the middle**: the veil stands the panel 72 from its top, so it does not jump as the list grows and shrinks under typing.
- **The context menu takes the app's open signal**, as `Menu::new(anchor, open)` does, and wraps its element: `ContextMenu::new(target, open)` reads beside `Menu`, and an `.context_menu(..)` on every element would have needed a change to every builder of the engine. Its items are given as a menu's (`item`, `items`, `separator`, `label`), so a submenu, a check or a radio is the same `MenuItem` it is in a menu.
- **The point is the engine's to place, the context menu's to keep**: `Overlay::at` places an overlay at a point, flips it on each axis where there is no room and clamps it, and is otherwise an anchored overlay; the context menu remembers where it was asked for. From the keyboard there is no point, and it hangs under its element as a menu under its button.
- **One context menu at a time is the context menu's**: opening one closes the one opened before it (a thread-local of the last open signal, as the tooltip has for its last). The engine does not close a popup on a secondary press outside it (its tests say so), and that was not changed: a secondary press on another element with a menu closes the first by opening its own; one elsewhere leaves it open, as a press of the secondary button leaves any popup open in lurq.
- **The right press opens it on its release** (`on_mouse_click(Right)`), as Windows does, and stops there, so an element with a menu inside another with one opens its own.
- **The edit menu is a context menu's bar**, not a component of its own: the same items and opening, laid in a row with Left and Right. **The overflow menu is a `Menu` under a ghost icon button** with a panel of 180, shown so on the gallery page: nothing in it is not a menu's and a button's.
- **The board's mobile and tablet context menus** (`GO5Be`, `GJks9`) are a menu's parts restyled: items of 48 or 44 with labels of 17 or 16 and a line between them (`MenuItemPart::Root`, `Label`, `underline`), a panel of 250 with corners of 14 and no padding (`ContextMenuPart::Panel`, `MenuPart::Panel`), icons of 20 or 18 at the end (a `MenuItem` renderer that puts `view.icon()` after `view.label()`). A theme does that at its breakpoints; the design's recipe is the desktop's.

## Not done

- **The keys are not `ui::Kbd`**: the Escape at the end of the search row and the keys of the footer are drawn by the palette's own `Key` part, in the board's Kbd style (20 high, padded 6, 4 round, `Muted` in an edge of `Border`, mono 11 in `MutedForeground`). `ui::Kbd` is on the empty-state session's branch, not on this one's base. At the merge: make `CommandPaletteView::key(label)` return `Kbd::new(label).build(self.theme())` and drop the `Key` part and its recipe rule (or keep the part as rules laid over the Kbd's root, `self.recipe.rules_for(CommandPalettePart::Key, KbdPart::Root)`, if a palette is to style its keys apart).

## Differs from the board

- **What the board does not draw was built in its manner**: the board's palette has the search row, one group label and three rows. The second text of a row is after its label, 12 in `MutedForeground`; the empty state is the empty text, centered, in the body's type in `MutedForeground` with 24 above and under it; the loading row is a combobox's (32 high, 8 in, a spinner of 16 and "Loading…" at 13, muted); the footer is 36 high, 12 in, under a line, its keys as the search row's key with their words at 12 in `MutedForeground`; the veil is the dialog's `Scrim`, with the panel 72 from the top.
- **Group labels are 12/500**, as the palette draws them, not the menu's 11/600.
- **The palette's shadow is the theme's large one**, and the context menu's the menu's medium one, as the board's two layers of shadow are drawn by those steps elsewhere.
- **The mobile context menu's preview** (the card over the menu in `GO5Be`) is not drawn: it is the app's element, shown by the app.
- **The edit menu's labels are in the foreground at 13** and its items have the menu item's highlight; the board's "Copy" is on `Accent`, which is the menu's highlight role here (`Highlight`).
