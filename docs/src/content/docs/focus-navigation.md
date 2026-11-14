---
title: Focus And Keyboard Navigation
description: Which elements take focus, how Tab moves between them, modal focus traps, focused and focus-visible styles, and testing focus.
---

# Focus And Keyboard Navigation

One element has focus at a time. It receives keyboard events, shows its focused style, and is where Tab and Shift+Tab start. It is focused whatever moved focus there: a click, Tab, `ctx.focus(&element_ref)`, or `ElementHandle::focus()`. Whether it also shows a focus ring for keyboard users is a second state, [focus-visible](#focus-visible), like CSS `:focus-visible`.

## Focusable Elements

Buttons, text inputs, checkboxes, sliders, selects, and any element with a `tab_index` are focusable. A click on one focuses it, and a click on its content (a button's label) focuses the nearest focusable ancestor. A press where nothing can take focus (empty space, a plain row, a label) blurs the focused element, button or input, like HTML.

- `.focusable(true)` makes any element focusable by click and by request. It does not put the element in the Tab order outside a form; add `.tab_index(0)` for that.
- `.focusable(false)` keeps an element from ever taking focus: not by click, not by Tab, not by `ctx.focus`. A click on it does not change focus either: whatever had focus keeps it. Its click handlers still run, and a checkbox still toggles. Use it for toolbar buttons that must leave focus in an editor.
- `.focuses(&element_ref)` makes a click on an element focus another, as a `<label for>` focuses its control: the click focuses the first focusable element under the one the ref is on, as `ctx.focus(&element_ref)` does, once the click's handlers have run. A builder that has no `Ctx` can use it: it makes an `ElementRef::new()`, puts it on what holds the control with `.ref_element(..)` (and a `key`, so that a ref made anew in each render does not make that node another one), and names it on the label. A click on something under the element that takes the focus itself (a link in the label) focuses that instead, a click whose default a handler prevented focuses nothing, and a disabled target is not focused.

```rust
use lurq::components::Button;

// Clicking it runs the handler; focus stays in the text editor.
Button::new("Bold").focusable(false).on_click(|_| toggle_bold());
```

## Tab Order

`tab_index` follows HTML `tabindex`:

| Element location | `tab_index` not set | `tab_index(0)` | `tab_index(n > 0)` | `tab_index(-1)` |
| --- | --- | --- | --- | --- |
| Inside a form | in Tab order | in layout order | first, by number | skipped |
| Outside any form | not in Tab order | in layout order | first, by number | skipped |

- Positive values come first, in ascending order; equal values keep tree order. Then come `tab_index(0)` elements (and, inside forms, controls without a tab index) in tree order.
- `tab_index(-1)` only removes an element from the Tab order. A click still focuses it, as in HTML.
- A button is one stop: its content is never a separate stop.
- Setting a tab index makes the element focusable, unless it is `focusable(false)`.

Outside forms, only elements that opt in with `tab_index(0)` or higher are stops. Give toolbar, sidebar, and list buttons `tab_index(0)` to make them reachable by keyboard:

```rust
use lurq::components::{Button, Row};

Row::new()
  .child(Button::new("New").tab_index(0).on_click(|_| new_file()))
  .child(Button::new("Open").tab_index(0).on_click(|_| open_file()));
```

### Inert Elements

`.inert(true)` takes an element and everything in it out of the focus and the pointer, while it is still painted: a splitter's closed pane, a panel on its way out.

```rust
use lurq::components::Column;

Column::new().inert(!open).child(panel_content);
```

- Tab and Shift+Tab pass by everything in it, and so do a focus group's arrows and typeahead.
- A focus request for anything in it (`ctx.focus`, `ElementHandle::focus`) is refused, and a press does not focus it.
- No pointer reaches it or anything in it: no press, click, hover or drop. Hit testing passes through it, so what is under it gets them.
- When it becomes inert with the focus inside it, the focus leaves it as it leaves an element taken out of the tree: to the stop of the focus group it was in that takes its place (the next one, or the last), or to nothing.
- Unlike `disabled(true)`, it sets no disabled state on what is in it: no disabled style is drawn, and controls in it look as they did.
- An element ref makes its element inert with nothing rendered again: `ElementRefMut::set_inert(bool)`. It takes effect at the next pass, which it asks for. An element built inert stays inert whatever its ref says.

```rust
use lurq::{components::Column, core::ElementRefMut, node::dimension::Dimension};

let pane = ElementRefMut::new();
let panel = Column::new().ref_element(pane.clone()).child(panel_content);

// Closed by a drag, with no render: no width, and nothing in it takes the focus or the pointer.
pane.set_size(Some(Dimension::Px(0.0)), None);
pane.set_inert(true);
```

## Focus Groups

`.focus_group(FocusGroup::..)` on an element makes what is in it a group the arrow keys move the focus within, while the focus is on one of them: the buttons of a toolbar, the choices of a radio group, the items of a menu.

```rust
use lurq::{
  components::{Button, Row},
  node::FocusGroup,
};

// Tab reaches the toolbar once; Left and Right move within it.
Row::new()
  .focus_group(FocusGroup::horizontal().one_stop(true))
  .child(Button::new("Cut").tab_index(0))
  .child(Button::new("Copy").tab_index(0))
  .child(Button::new("Paste").tab_index(0));
```

| `FocusGroup` | |
| --- | --- |
| `horizontal()`, `vertical()`, `both()` | Which arrows move the focus: Left and Right, Up and Down, or all four (Left and Up go back, Right and Down go on) |
| `wrap(bool)` | Whether the focus goes from the last to the first and back. It does unless told otherwise |
| `activate(bool)` | What the focus is moved to is pressed as well, as Space would press it: the choice follows the focus, as in a radio group |
| `one_stop(bool)` | Tab stops in the group once, on the element marked `.focus_entry(true)` (the chosen one) or the first, and from inside the group Tab leaves it |
| `follow_pointer(bool)` | While the focus is in the group, the pointer coming onto one of its elements moves the focus there, with no ring, and an arrow key takes the hover off the element under a pointer that holds still: what the pointer and the keyboard are on is one, as in a menu |
| `typeahead(bool)` | Typing a letter moves the focus to the element whose label starts with it. Letters typed within a second of one another are one prefix, the same letter again goes on to the next element that starts with it, and a pause starts again |
| `menu_row(bool)` | The group is a row of menus, as the File, Edit and View of a menubar: each of its elements opens a menu hung from it. With one of those menus open, Left and Right go to the menu beside it, and Alt reaches the row from anywhere. See [a row of menus](#a-row-of-menus) |
| `panel_row(bool)` | The group is a row of panels, as a site's navigation menu: a row of menus whose elements are links as well as buttons that open a panel. Left and Right from an open panel focus the element beside it and open its panel, pressing no link, and Alt is not its. See [a row of panels](#a-row-of-panels) |
| `nested(bool)` | The group is a tree: Right and Left open, close and go between levels, and `*` opens the elements beside the focused one. Its elements say their level with `.focus_level(n)` and whether they are open with `.focus_expanded(open, toggle)`. See [a nested list](#a-nested-list) |
| `pages(bool)` | In a vertical group, PageUp and PageDown move the focus by a page: to the farthest element no more than the height of the scroll the group is in away, and at least one on. A group in no scroll is one page: the keys go to its ends. See [pages and ranges](#pages-and-ranges) |
| `shift_extends(bool)` | A key that moves the focus with Shift held presses what it moves to with Shift, as a press of the pointer with Shift would: in a list of which any number are selected, Shift with the arrows, Home, End and the page keys selects a range. See [pages and ranges](#pages-and-ranges) |
| `FocusGroup::none()` | No key moves the focus within it, and what is in it is out of the group around it: that group's arrows pass it by, and do nothing while the focus is in it. For what a group holds beside what its keys move between, as the content of an accordion's open item is between two headers |

- The keys move between what Tab would stop at inside the element (a `tab_index` of `0` or more, or a control of a form), in the order of the tree; a disabled or dropped one is passed by. Home and End go to the first and the last.
- A key that is the group's is taken even where it moves nothing, at an end of a group that does not wrap. A handler that prevents the key's default keeps the group from acting on it.
- An element that is disabled while it has the focus loses it. In a group the focus goes to the element before it in the group, or the one after it, and not out of the group: the next button of a pagination disables itself on the last page, and the keyboard is still there.
- An element that leaves the tree while it has the focus takes the focus with it, except in a group that stays: there the focus goes to the group's stop that took its place, the next one or the last. A tag taken away with Backspace leaves the focus on the next tag's button, and a row deleted from a list on the next row.
- Tab stops at a button and not at what is in it, which is part of the button. `.stops_inside(true)` on a button makes Tab stop at what takes the focus inside it too, after the button itself: a card that is pressed as a whole and has buttons of its own.
- In a group that is one stop, the stops inside the stop the group keeps are kept with it: Tab from a focused row of a table goes into its buttons and then out of the group, and Shift+Tab from one of them goes back to its row. While the focus is on such a button the group's stop is the row it is in. The arrow keys move between the rows, not into what they hold; a group whose stops have nothing inside them is one stop as before.
- A text input, a slider and a select keep the arrow keys for themselves while they are focused. With Ctrl, Alt or the system key held an arrow is not the group's.
- The innermost group around the focused element that takes the key acts: a vertical menu inside a horizontal bar takes Up and Down, the bar Left and Right.
- An element's label for typeahead is what `.focus_label(..)` gives it, or, for a button with one text in it, that text; an element with neither is not reached by typing. A menu item's label is its label alone, not its shortcut. A text input, a slider and a select in the group keep the letters, and Space still presses what has the focus.
- A key event reaches every node's `on_key_down`, focused or not. `KeyboardEvent::focus_within` says whether the focus is on the node whose handler runs or inside it: a handler that acts only for what has the focus (a menu item opening its submenu on Right) checks it.
- A key's handler can move the focus to an element of its group by the element's key with `event.focus_key(key)`. Once the handlers have run and what they changed is rendered, the focus goes to the stop of the innermost group around the focused element whose `.key(..)` is `key`, from the keyboard (it shows its ring), and is scrolled into view. The key's default is prevented, so the group does not move the focus as well. This is for a group whose elements are reached by what the app knows of them, not by their order. The days of a calendar are one: Up is the same day a week back, PageDown the same day a month on, and either may be in a month that is built only once the key has shown it. A key no stop of the group has, or a group with nothing focused in it, moves nothing.

```rust
use lurq::{
  app::events::KeyboardEvent,
  components::{Button, Row},
  node::FocusGroup,
};

// Each day says where the keys take the focus from it: Right to the next day, by its key.
let day = |n: u32| {
  Button::new(&n.to_string())
    .tab_index(0)
    .key(format!("day-{n}"))
    .on_key_down(move |event: KeyboardEvent| {
      if event.focus_within && event.key == "ArrowRight" {
        event.focus_key(format!("day-{}", n + 1));
      }
    })
};
let week = Row::new()
  .focus_group(FocusGroup::horizontal().one_stop(true))
  .child(day(1))
  .child(day(2));
```

### Pages And Ranges

A list in a scroll, a listbox or a table's rows, moves by pages with `pages(true)`, and one of which any number are selected extends its selection with Shift and the keys with `shift_extends(true)`. Neither counts rows of its own: the group moves the focus, and the row it moves to is pressed as the pointer would press it.

```rust
use lurq::{
  app::events::MouseEvent,
  components::{Button, Column, ScrollVertical},
  node::FocusGroup,
};

// One stop; Up, Down, Home, End, PageUp and PageDown through the rows; Shift with them a range.
let rows = Column::new()
  .focus_group(
    FocusGroup::vertical()
      .one_stop(true)
      .wrap(false)
      .pages(true)
      .shift_extends(true),
  )
  .with_children((0..200).map(|row| {
    Button::new(&format!("Row {row}"))
      .tab_index(0)
      .on_click(move |event: MouseEvent| {
        // event.shift: from where the selection started to this row; event.ctrl: this row in or out;
        // event.from_keyboard: Space, which adds or takes out the focused row.
        let _ = (row, event.shift, event.ctrl, event.from_keyboard);
      })
  }));
let list = ScrollVertical::new(rows).max_height(320.0);
```

- A page is what the innermost scroll that runs down around the focused element shows: PageDown goes to the farthest element whose top is no more than the scroll's height below the focused one's, or to the next one where even that is farther. It does not go round.
- With `shift_extends`, the press after a move with Shift is a click with Shift held, from the keyboard: the element's `on_click` reads `event.shift` and selects from where the selection started to it. Without Shift a key only moves the focus, unless the group `activate`s, where the choice follows the focus; a letter of typeahead presses too in a group that `activate`s.
- `ui::Listbox` and `ui::TreeView` select this way, with one model: a plain press selects one row alone, Ctrl (Cmd) with a press adds or takes out one, Shift with a press or a key selects from the last selected row to it and keeps that row last, so that the next range starts there, Space adds or takes out the focused row, and Ctrl+A selects every row that takes input. A disabled row is passed by and never selected. What is selected is the app's: a `Vec` whose last item is where a range starts.
- In a group of [rows a list builds in part](#rows-a-list-builds-in-part) both hold as well: the row the keys move to is pressed once it is built and focused.

### A Row Of Menus

`FocusGroup::menu_row(true)` makes a group a row of menus: each of its elements opens a menu hung from it when it is pressed, a popup whose anchor is the element, as `ui::Menu` is. It is what a menubar's titles are (`ui::Menubar` has it), and what a bar of your own of buttons and menus takes to have a menubar's keys.

```rust
use lurq::{
  components::{Button, Column, Row},
  core::{ElementRef, Signal},
  node::FocusGroup,
  ui::{Menu, MenuItem},
};

// file, edit: ElementRef; file_open, edit_open: Signal<bool>. A press of a title opens its menu and closes the other.
let title = |name: &str, anchor: &ElementRef, open: &Signal<bool>, other: &Signal<bool>| {
  let (open, other) = (open.clone(), other.clone());
  Button::new(name)
    .tab_index(0)
    .ref_element(anchor.clone())
    .on_click(move |_| {
      other.set(false);
      open.update(|open| *open = !*open);
    })
};
Row::new()
  .focus_group(FocusGroup::horizontal().one_stop(true).typeahead(true).menu_row(true))
  .child(
    Column::new()
      .child(title("File", &file, &file_open, &edit_open))
      .child(Menu::new(file_open.clone(), file.clone()).item(MenuItem::new("New")).item(MenuItem::new("Open"))),
  )
  .child(
    Column::new()
      .child(title("Edit", &edit, &edit_open, &file_open))
      .child(Menu::new(edit_open.clone(), edit.clone()).item(MenuItem::new("Undo")).item(MenuItem::new("Redo"))),
  );
```

- On the row itself it is a group as any other: Left and Right move between its elements, and Enter or Space presses one, which opens its menu with the focus in it.
- While a menu hung from an element of the row has the focus, Left and Right that nothing in the menu takes close it, with every menu opened from it, and press the element before or after the one it hangs from, going round at the ends as the group wraps: that element opens its menu, which takes the focus. A key a handler in the menu prevents stays the menu's: a submenu's Left closes the submenu, and an item's Right opens its own submenu. From a submenu, Right on an item with none goes on to the next element of the row.
- Alt pressed and let go with nothing pressed between puts the focus on the first row of menus of the window, at its entry (the element marked `.focus_entry(true)`, or the first that takes the focus); Alt alone again, from the row or a menu of it, closes the menu and gives the focus back to what had it before. A key or a press of the pointer while Alt is down makes its release no Alt alone, and a handler that prevents the release's default keeps it from the row.
- Alt with a letter presses the element of that row whose label starts with it (its `focus_label`, or the one text of a button), from wherever the focus is: it opens its menu, closing the one that was open. Where several start with the letter, the next press goes on to the next. A letter that starts no element is not the row's.
- A disabled row, or a disabled element of it, is passed by: the row with no element to stop at is not reached by Alt.
- On a Mac, Alt (Option) does neither: Option with a letter types a character there, and an app's menus belong in the system's menu bar (see [native menus](../window-lifecycle-menus/#building-a-native-menu)).

### A Row Of Panels

`FocusGroup::panel_row(true)` makes a group a row of panels: a [row of menus](#a-row-of-menus) whose elements are links as well as buttons that open a panel hung from them, as a site's navigation menu is (`ui::NavigationMenu` has it, and so the links of `ui::Navbar`). It sets `menu_row` too, and keeps what a row of menus does with two differences, which a row of links needs.

```rust
use lurq::{
  components::{Button, Column, Popup, Row},
  core::{ElementRef, Signal},
  node::FocusGroup,
};

// docs: ElementRef; docs_open: Signal<bool>; docs_panel: what the panel holds; go_to_blog: what the link does.
let (open, toggle) = (docs_open.clone(), docs_open.clone());
Row::new()
  .focus_group(FocusGroup::horizontal().panel_row(true))
  .child(
    Column::new()
      .child(
        Button::new("Docs")
          .tab_index(0)
          .ref_element(docs.clone())
          .on_click(move |_| open.update(|open| *open = !*open))
          // How the row opens it from the keyboard, as a nested list does.
          .focus_expanded(docs_open.get(), move |open| toggle.set(open)),
      )
      .child(Popup::new(docs.clone(), docs_panel).open(docs_open.clone()).takes_focus(true)),
  )
  .child(Button::new("Blog").tab_index(0).on_click(move |_| go_to_blog()));
```

- On the row itself it is a group as any other, and Tab still stops at each of its elements unless the group has `one_stop(true)`: a site's links are each a stop, as a browser's are.
- While a panel hung from an element of the row has the focus, Left and Right that nothing in the panel takes close it, as in a row of menus, and then focus the element before or after the one it hangs from, from the keyboard, without pressing it: going past a link does not follow it. An element that says with `.focus_expanded(open, toggle)` how it opens is asked to open (`toggle(true)`), and opens its panel; a link, which says nothing, is only focused.
- Alt is a menubar's, not a row of panels': neither Alt alone nor Alt with a letter reaches it, so a navbar before a menubar in the tree does not take the window's Alt from it.

### A Nested List

`FocusGroup::nested(true)` on a vertical group makes it a tree: its elements stand at levels, and those that open and close say so. Each element gives its level with `.focus_level(n)` (1 at the top; an element with none is at 1), and one that opens gives whether it is open and what asks it to change with `.focus_expanded(open, toggle)`. What an element holds is the elements after it at a higher level, up to the next one at its level or lower, so the group may be one flat list of rows, as a tree view's is, or elements inside elements, as a sidebar's sections. `ui::TreeView` is one.

```rust
use lurq::{
  components::{Button, Column},
  core::Signal,
  node::FocusGroup,
};

// open: Signal<bool>, whether the folder is open.
let set = open.clone();
let mut tree = Column::new()
  .focus_group(FocusGroup::vertical().nested(true).one_stop(true).typeahead(true))
  .child(
    Button::new("src")
      .tab_index(0)
      .focus_level(1)
      .focus_expanded(open.get(), move |to| set.set(to)),
  );
if open.get() {
  tree = tree
    .child(Button::new("lib.rs").tab_index(0).focus_level(2))
    .child(Button::new("main.rs").tab_index(0).focus_level(2));
}
tree = tree.child(Button::new("Cargo.toml").tab_index(0).focus_level(1));
```

- Right opens a closed element (it calls its toggle with `true`), and on an open one goes to the first element in it. Left closes an open element, and on a closed one or one that does not open goes to the element it is in: the nearest one before it at a lower level. `*` opens every closed element at the focused one's level in the element it is in.
- Opening and closing is the app's: the keys call the toggle, and what the element holds comes and goes with the next render. What a closed element holds is not in the tree, so Up, Down, Home, End and typeahead go through the elements that are shown.
- A nested list in a sidebar, whose items hold their children in a column under the button (or in a `ui::Collapsible`'s panel), takes the same calls: the level of each item's button, and `focus_expanded` on those that open. Children kept in the tree while their item closes (to animate their height out) are still stops until they leave: put them in a `FocusGroup::none()` while closing.
- Only a vertical group is nested: Right and Left are its cross keys. A group that is not nested leaves Right, Left and `*` to what is around it.

### Rows A List Builds In Part

A virtual list of thousands of rows builds those near its view; the rest are not in the tree, so they are no stops for a group's keys. `.focus_rows(rows)` on the element of the group gives the group the rows themselves, an `Arc` of an `impl FocusRows`, and each row the list builds says which it is with `.focus_row(index)`. The group's keys then step over the rows, built or not. `TreeView::mount(ctx)` is one.

```rust
use std::sync::Arc;

use lurq::{
  components::{Button, Column},
  core::Signal,
  node::{FocusGroup, FocusRows},
};

/// A thousand rows, of which the list builds those from `start` on.
struct Rows {
  start: Signal<usize>,
}

impl FocusRows for Rows {
  fn count(&self) -> usize {
    1000
  }

  fn label(&self, row: usize) -> Option<String> {
    Some(format!("Row {row}"))
  }

  fn page(&self) -> usize {
    20
  }

  // Build the row and bring it into view: the focus goes to it once a layout has it.
  fn reveal(&self, row: usize) {
    self.start.set(row.saturating_sub(5));
  }
}

// rows: Arc<Rows>, the same one from one render to the next.
let start = rows.start.get();
let group: Arc<dyn FocusRows> = rows.clone();
Column::new()
  .focus_group(FocusGroup::vertical().one_stop(true).typeahead(true).wrap(false))
  .focus_rows(group)
  .with_children((start..(start + 30).min(1000)).map(|row| {
    Button::new(&format!("Row {row}")).tab_index(0).focus_row(row)
  }));
```

| `FocusRows` | |
| --- | --- |
| `count()` | How many rows there are |
| `label(row)` | What typeahead matches the row by |
| `enabled(row)` | Whether the row takes the focus; a disabled one is passed by. All do unless told otherwise |
| `level(row)`, `expanded(row)`, `set_expanded(row, open)` | In a [nested](#a-nested-list) group: the row's level, whether it is open, and what asks it to open or close |
| `page()` | How many rows PageUp and PageDown move by: as many as the view holds. 10 unless told otherwise |
| `reveal(row)` | Asks for the row to be built and brought into view: a virtual list's [`reveal_key`](../layout/#scroll), with `reveal_request` to ask for one row again |

- Up, Down, Home, End, PageUp and PageDown, the keys of a nested group, and typeahead go to any row. A row that is built has the focus at once; one that is not is asked for with `reveal`, and has the focus after the layout that has it.
- Keys pressed while a row is awaited go on from that row, not from the row that had the focus: holding Down runs down the list. A press of the pointer, Tab and a focus the app asks for (`Ctx::focus`) end the wait.
- PageUp and PageDown are a group's only where it has rows or [pages](#pages-and-ranges); a group with neither leaves them to what scrolls.
- In a group that `activate`s, or that has `shift_extends` and Shift is held, the row the keys or a letter move to is pressed as well, as an element of a group that builds every one is: at once where it is built, and otherwise once the layout that has it has focused it. `Listbox::mount(ctx)` chooses that way over thousands of options.
- The group is known by its `Arc`: give the same one at every render, as a component that keeps it in a field does.

A row may hold several stops, as the cells of a grid do: each says its row and its column, `.focus_row(row)` and
`.focus_column(column)`, and each row's stops are a horizontal group of their own. Left, Right, Home and End move
within the row; Up, Down, PageUp and PageDown go to the stop of the same column in the row they go to (or the
nearest before it, where that row has fewer), built or not; Ctrl+Home and Ctrl+End go to the first stop of the
first row and the last stop of the last. `ui::DataGrid` is one.

```rust
Column::new()
  .focus_group(FocusGroup::vertical().one_stop(true).wrap(false))
  .focus_rows(rows)
  .with_children((start..end).map(|row| {
    Row::new()
      .focus_group(FocusGroup::horizontal().wrap(false))
      .with_children((0..3).map(move |column| {
        Button::new(&format!("{row}:{column}")).tab_index(0).focus_row(row).focus_column(column)
      }))
  }));
```

## Tab Scope

Tab and Shift+Tab move through one scope and wrap around at its ends:

1. **A modal is open**: the topmost open modal (see [Modal Focus Trap](#modal-focus-trap)).
2. **Otherwise**: the whole window, including open popups and overlays after the page, in tree order. The stops of a popup or overlay that takes the focus (`takes_focus(true)`, see [Popups](../modals/#popups-and-outside-presses)) come right after its anchor's instead, as a web popover's come after its invoker's: Tab from the trigger goes into it, and Tab from its last stop goes on to what follows the trigger. Where the anchor is in a focus group Tab stops in once, they come after the group.

`WindowChrome`'s title bar and resize zones are a layer over the page, but they are window decoration, not a modal: they never become the Tab scope, and stops placed in the title bar follow the page's stops in the window order. A `Modal` declared in the chrome's content or passed to `WindowChrome::overlay` is a real modal and traps Tab as usual.

Forms do not have a scope of their own. A form's controls are stops without a tab index, in tree order among the scope's other stops, and the form is passed through like in a browser: Tab from a toolbar button reaches the form's first field, Tab from its last control moves on to the next stop after the form, and Shift+Tab from its first control goes back to the stop before it. A form cycles only when it is all its scope contains, for example the only content of a modal.

When the focused element is not a stop itself (a clicked button without a tab index, or a text input outside a form), Tab continues from its place in the tree, like a browser: Tab goes to the next stop after it, Shift+Tab to the previous one. With nothing focused, Tab goes on from where the focus was last (see [Where Tab Goes On From](#where-tab-goes-on-from)), and where it was never anywhere, to the first stop, and Shift+Tab to the last.

### Where Tab Goes On From

A browser's sequential focus navigation starting point: when the focus is dropped, the place it was is kept, and the next Tab or Shift+Tab goes on from there in the Tab order, not from the top of the window. However the focus goes, its place stays:

- **A key blurs it**: Enter in a text field that is not in a form, or Escape in one. Tab goes to the stop after the field, Shift+Tab to the one before it.
- **What had it leaves the tree**: Tab goes to what took its place, the first stop after where it was. Inside a focus group that is still there the focus moves to the group's next stop at once (see [Focus Groups](#focus-groups)); this is for what has no group to stay in.
- **What had it is disabled or made inert**, or an element around it is: Tab goes on from where that element is.
- **A modal closes with nothing to give the focus back to**: what opened it has gone, or nothing had the focus as it opened. Tab goes on from where the focus was as it opened.
- **`ctx.blur()`** or **`ElementHandle::blur()`**.

A press on something that takes no focus (a label, a plain row, empty space) blurs what has the focus and puts the place there: Tab goes to the first stop after what was pressed, Shift+Tab to the last before it, as a press on a page's text does in a browser. A press on nothing at all starts Tab from the top again. Anything that gives the focus to an element (a click, Tab, `ctx.focus`) forgets the place.

```rust
use lurq::components::{Button, Column, Text, TextInput};

Column::new()
  .child(Button::new("Back").tab_index(0))
  .child(TextInput::new(name.clone()).tab_index(0))
  .child(Text::new("Your name as it is shown to others."))
  .child(Button::new("Save").tab_index(0));
// Type in the field and press Enter: the field is left, and Tab goes on to Save, not to Back.
// Press the line of text: Tab goes to Save as well, and Shift+Tab to the field.
```

An open menu or select does not keep Tab: Tab closes a `ui::Menu` (and the menus it hangs from) or a `Select`'s list, and goes on from what opened it, as it would from there closed.

Tab with no stops in scope outside a modal is not handled, so it reaches other keyboard defaults. An `on_key_down` handler that calls `prevent_default()` on Tab replaces traversal entirely (see [Keyboard And Focus](../styling-events/#keyboard-and-focus)).

Tab traversal does not need the `form` feature; without it there are no forms, and only elements with a tab index are stops.

## Modal Focus Trap

An open `Modal` confines Tab and Shift+Tab to itself, whether or not it contains a form:

- Focus behind the modal is never a stop. When focus is still on the page behind it (the button that opened it), the next Tab moves into the modal.
- Stops inside the modal follow the same rules: `tab_index(0)` or higher, and form controls inside a form in the modal. A form in a modal cycles inside the modal, together with the modal's other stops.
- A modal without stops consumes Tab and keeps focus where it is, so Tab never reaches the page behind it.
- With nested or stacked modals, the topmost one traps. Modals stack in the order they opened, not in the order of the tree: a second modal opened over a first, wherever it is declared, takes the focus (with `takes_focus(true)`), and as it closes the focus goes back into the first. See [Behavior](../modals/#behavior).
- When the modal closes, the window scope applies again.

Pointer input is not trapped; a click on the page behind a `Parent`- or element-targeted modal still focuses what it hits.

## Scrolling Focus Into View

When Tab or Shift+Tab, `ctx.focus(&element_ref)` or `ElementHandle::focus()` moves focus to an element inside a scroll container, every scroll container around it scrolls by the smallest amount that shows the element, innermost first, on each axis the container scrolls, like a browser's `element.focus()`. An element already in view does not move anything; one taller or wider than the viewport is aligned to the viewport start. A focus request scrolls in the pass that applies it, so the frame drawn next already shows the element. Focus moved by a click does not scroll: what was clicked is already on screen, and moving it under the pointer would be a jump.

What sticks at an edge of a scroll container (`.sticky(..)`, see [Sticky elements](../layout/#sticky-elements)) is left out of the viewport an element is scrolled into, as the web's `scroll-padding` would leave it: a row the arrow keys move to is shown under a table's header that stays at the top, not behind it, and clear of a first column that stays at the start. Only the sticky elements at that edge are left out, by their inset and size, and not one the element is in itself.

To scroll an element into view without focusing it, use `ctx.scroll_into_view(&element_ref)` in a component, or `ElementHandle::scroll_into_view()` on a handle from `Tree::get_element_by_id_mut` (see [Scroll](../layout/#scroll)).

## Focus-Visible

Focus-visible is the keyboard-only part of focus, like CSS `:focus-visible`: a button focused by Tab shows a focus ring, the same button focused by a click does not. The focused element is focus-visible when the last input that could move focus was the keyboard:

- **Keyboard**: Tab and Shift+Tab, and any key press other than a bare modifier (Shift, Control, Alt, Super/Command) made without Control, Alt or Super/Command held. Arrow keys, Space, Enter, Escape and letters count, so pressing a key on a clicked button shows its ring, as in a browser; a shortcut such as Ctrl+C or Cmd+C does not.
- **Pointer**: any mouse button press in the window, including one that only closes a popup, and `ElementHandle::click()`. It hides the ring of the element that keeps focus.
- **Focus requests** (`ctx.focus(&element_ref)`, `ElementHandle::focus()`) inherit the last input's modality: a request made from a key handler shows the ring, one made from a click handler does not. Before any input the modality is pointer, so an app that focuses an element at startup shows no ring until the user touches the keyboard.
- **Text inputs** are focus-visible whenever they have focus, clicked or not, as browsers treat text fields: the caret alone does not show which field takes the keys.

Synthetic input (`lurq::app::synthetic_input`, the MCP `lurq_interact` tool) goes through the same mouse and key entry points and follows the same rules. The rules are the same on Windows and macOS.

Read the state with `InteractionState::is_focus_visible()` (on a node given `.interactive(state)`), `core::ElementRef::focus_visible()` and its reactive `focus_visible_signal()`, or `Tree::focus_visible()` for the focused element. The focused state (`is_focused()`, `focused()`, `focus_signal()`) is unchanged and stays true for any focus.

## Focused Styles

Every element accepts a focused state style, merged over the base style while it has focus, and a focus-visible style, merged over the focused style while it is focus-visible. Both sit under the hovered and active styles, so a hovered border paints over a focus ring:

```rust
use lurq::{
  app::theme::{BorderSize, PaletteColor},
  components::Button,
};

// A ring for keyboard focus only; a click focuses the button without it.
Button::new("Save")
  .tab_index(0)
  .border_inside(BorderSize::Sm, PaletteColor::Border)
  .focus_visible(|style| style.border_inside(BorderSize::Sm, PaletteColor::Ring));
```

A [disabled](../styling-events/#disabled) element and everything in it take no focus: Tab passes them by and focus requests on them are ignored.

Use `.focused(...)` / `.focused_style(...)` for a style on any focus, and `.focus_visible(...)` / `.focus_visible_style(...)` for a focus ring. A ring that sits outside the control and moves nothing is an [outline](../styling-events/#outline): `.focus_visible(|style| style.outline(Outline::new(2.0, PaletteColor::Ring).offset(2.0)))`.

Controls whose visuals are drawn from part styles have part-level focus styles, layered like their hovered styles. They show while the control is focus-visible, like the native focus ring of a browser's checkbox, range input and select: a click or a drag does not show them, Tab or a key press does. They change paint only; a width or height in them is ignored so focus never moves layout:

| Control | Focus ring (focus-visible) |
| --- | --- |
| Button, text input, any element | `.focus_visible(...)` / `.focus_visible_style(...)` |
| `Checkbox` | `.box_focused(...)` / `.box_focused_style(...)` |
| `Slider` | `.thumb_focused(...)` / `.thumb_focused_style(...)` |
| `Select` | `SelectStyle::trigger_focused(...)` |

For a checkbox, slider or select that should mark every focus, put a `.focused(...)` style on the control itself.

The compound form controls take their focus border from the form theme: `form.input.border_focus`, `form.checkbox.border_focus`, `form.slider.thumb_border_focus`, and `border_focus` on both `form.button` roles, all `PaletteColor::Ring` by default. The buttons, the checkbox and the slider draw it while focus-visible; the text input draws it for any focus. See [Form Theme](../theme/#form-theme).

## Testing Focus

`Tree::focused_element()` returns the element that has focus, like `document.activeElement`: the control itself, not the wrapper that owns its `on_focus` handler. `Tree::focus_visible()` tells whether it is [focus-visible](#focus-visible). The MCP `lurq_read_tree` tool lists a focus-visible node's states as `focused,focus-visible`, and `lurq_inspect` reports `"focus_visible": true`.

`Tree::pass_headless(&mut app)` runs a pass without a window: it rebuilds components and lays out the tree, overlays and modals included, so hit testing, focus, and Tab work, but it draws nothing and never calls the render engine. It needs no window handle and no `unsafe`:

```rust
use lurq::{
  app::{App, Tree},
  components::{Button, Column, Modal, Root},
};

let mut tree = Tree::new();
tree.set_root(
  Column::new()
    .child(Button::new("Toolbar").id("toolbar").tab_index(0))
    .child(
      Modal::new(
        Column::new()
          .child(Button::new("Cancel").id("cancel").tab_index(0))
          .child(Button::new("Confirm").id("confirm").tab_index(0)),
      )
      .target(Root),
    ),
);
tree.pass_headless(&mut App::new());

tree.key_down("Tab".into(), "Tab".into(), false, false, false);
assert_eq!(tree.focused_element().and_then(|element| element.id()), Some("cancel"));
```

Call `tree.resize(width, height)` before the pass to choose the viewport (800×600 by default). Run another `pass_headless` after state changes that re-render (opening a signal-backed modal) and after keyboard scrolling, before reading bounds. See [Testing](../testing/).

## Giving The Focus Up

`ctx.blur()` takes the focus from whatever has it after the current render: a text input being typed into is
left, and a phone's keyboard goes away. A request to focus made in the same render wins.
