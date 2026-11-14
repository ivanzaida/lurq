# Toolbar and Action Bar: hand-off

`ui::Toolbar` and `ui::ActionBar`, from the board's `Toolbar` (`onKbC`), `Overflow Menu` (`p4lLjx`), `Action Bar`
(`yh6CV`), `Command Bar` (`oCaxH`) and `Table Toolbar` (`nX63b`) in Components / light (`Ovt1D`), where they are used (the
toolbar and the overflow menu in Shell & Toolbars › Tools, `nr26g`; the action bar in Overlays › Contextual, `Eux0O`;
the command bar in Desktop Shell › Bars, `bOlEj`; the table toolbar in Data › Table toolbar), and `Mobile/Selection Bar`
(`rTRZ5`) and `Mobile/Bottom App Bar` (`tpI2p`), read for the parts a theme restyles into them.

Files: `crates/lurq/src/ui/toolbar.rs`, `crates/lurq/src/ui/action_bar.rs`, `crates/lurq/tests/ui/toolbar.rs`,
`crates/lurq/tests/ui/action_bar.rs`, `crates/gallery/src/pages/toolbar.rs` and `action_bar.rs` (pages "Toolbar" and
"Action Bar", group Navigation), `design/handoff/toolbar.md`, and lines in the three `mod.rs` files.

## Changed outside the component's own files

This session owns measuring which children of a row fit and moving the rest into a menu. The engine already drops the
children of a single-line row that do not fit (`ShrinkLimit::Drop`, last first within a shrink order) and reports a
dropped child's element refs at no size; what it lacked is the child that stands for what was dropped.

- **Engine: `.drop_marker(bool)`** on every component (the builder macro in `components/mod.rs`), the field
  `ShrinkRule::marker` (`layout/layout_kind.rs`) and `NodeUpdate::drop_marker` (`node/node.rs`). A child of a single-line
  `Row`/`Column` marked so takes its room only while another child of the line is dropped, and is dropped itself while
  none is: the line gives way without the marker's room first, and only when that drops (or collapses) a child does it
  give way again with the marker in it. In `layout/layout_engine/flex_shrink.rs`, `shrink_outcomes` does this around the
  old body, now `give_way(line, results, overflow, occupied)`; a line with no marker gives way exactly as before.
  `has_dirty_shrunk_child` also counts a dirty marker, whose zero size was the line's choice. Devtools' snapshot shows
  "drop marker" (`app/devtools/snapshot.rs`). Tested in `tests/ui/toolbar.rs`
  (`a_line_drops_its_marker_while_it_drops_nothing`); an engine test of its own in `layout_engine` would be welcome at the
  merge but is not needed.
- `crates/lurq/src/ui/button.rs`: `Button::command()` (`pub(crate)`), the button's label, icon, icon-only, disabled
  (or loading), id and its press as a `ToolbarCommand`; the press calls its `on_click` handlers with a click as Enter
  makes it (`toolbar::keyboard_click()`).
- `crates/lurq/src/ui/toggle.rs`: `Toggle::command()` (`pub(crate)`), checked while on; choosing it calls `on_change`
  with the other state.
- `crates/lurq/src/ui/toggle_group.rs`: `ToggleGroup::commands()` (`pub(crate)`), one checked command per toggle.

No keyframes were added: the action bar comes in and leaves with the toast's (`0xFF11` to `0xFF14`, a fade and a move of
16), named again in `action_bar.rs` by their identifiers.

## For `docs/src/content/docs/design-system.md`

## Toolbar

The tools of what is in front of a person, in a row: the marks of a text, the actions on a table, the commands of a document.

```rust
use lurq::ui::{Button, Toggle, Toolbar, ToolbarGroup, ToolbarItem, ToolbarVariant, icons};

Toolbar::new()
  .label("Formatting")
  .child(ToolbarItem::from(Toggle::new("Bold").icon(icons::BOLD).icon_only(true).on(bold.get())).shortcut("Ctrl+B"))
  .child(Toggle::new("Italic").icon(icons::ITALIC).icon_only(true).on(italic.get()))
  .child(Toolbar::separator())
  .child(Button::new("Link").icon(icons::LINK).icon_only(true).on_click(link))
  .child(Toolbar::spacer())
  .child(ToolbarItem::from(Button::new("Search").icon(icons::SEARCH).icon_only(true)).stays(true))
  .overflow(true)
  .mount(ctx);

// A command bar's groups: a label under each, and the group goes into the menu whole.
Toolbar::new()
  .variant(ToolbarVariant::Plain)
  .child(ToolbarGroup::new().label("Clipboard").child(Button::new("Paste").icon(icons::PASTE)))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `Toolbar::new()` | An empty bar on a surface, its controls of the small size |
| `child(impl Into<ToolbarItem>)`, `children(..)` | A `Button`, a `Toggle`, a `ToggleGroup`, a `Select`, a `ToolbarGroup`, `Toolbar::separator()`, `Toolbar::spacer()`, an `Element`, or a `ToolbarItem` made of any of them |
| `mount(ctx)` | Puts the bar in the tree of `ctx`'s component: it keeps the menu of what does not fit and the control the focus left. `build(theme)` and `From<Toolbar> for Element` draw it once with nothing kept: no tooltips, no menu |
| `variant(ToolbarVariant)` | `Surface` (the default): the background's fill and an edge. `Plain`: neither. `Floating`: the popover's fill, an edge and the large shadow |
| `size(ButtonSize)` | The size of its buttons and toggles: `Sm` by default, so the bar is 40 high |
| `overflow(bool)` | Where the bar is narrower than its controls, those that do not fit go into a menu behind a More button at its end, the last first. Without it they are cut off |
| `label(&str)` | What the bar is called, to what reads the tree |
| `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. The More button has the id with `-more` after it, its menu `-menu` |

| `ToolbarItem` | |
| --- | --- |
| `shortcut(&str)` | Keys shown in the tooltip of an icon alone and beside its item in the menu |
| `stays(bool)` | The control never goes into the menu |
| `as_given(bool)` | The button or toggle keeps its own variant, size and style over the bar's: the outline Export of a table's bar |
| `ToolbarItem::element(el)`, `in_menu(ToolbarCommand)` | Any element: it stays in the bar unless it has a command to stand for it in the menu |

- The bar gives its buttons its size and makes them ghost ones, and its toggles its size, unless an item is `as_given`. A button or toggle that is its icon alone is called by its label and has a tooltip (below it) of its label and its shortcut.
- It is one stop of Tab: Tab enters it at the control the focus last left (the first, the first time), and leaves it at once. Left and Right move between its controls, the toggles of a group in it among them; Home and End go to the first and the last; a disabled one is passed by. A field or a select in it keeps its own keys while it is focused.
- With `overflow`, a control that does not fit drops out of the bar whole, the last first, and the More button (`more-horizontal`, after a line) appears at the end; its menu has an item for each control that went, in the order of the bar: its label, its icon, its shortcut, a check for a toggle (each toggle of a group, after a line), and its press. A labelled group goes whole, under its label. A select or a field stays unless it has `in_menu`. The bar learns what went from the layout (the engine's `ShrinkLimit::Drop` and `drop_marker`), and its menu follows it a pass later.
- Parts: `ToolbarPart::Root`, `Item` (laid over each button's and toggle's root), `Slot` (a control with the line before it, 2 apart), `Separator` (laid over a separator's root), `Spacer`, `Group`, `GroupItems`, `GroupLabel`, `More`. State: `ToolbarState::Overflowing`. Variants: `ToolbarVariant::Plain`, `Surface`, `Floating`.
- In the design the bar is padded 4, its controls 2 apart; a button is a ghost one of 32 with an icon of 16; a toggle that is on has the fill of what is picked out; a line of 20 in the border's color stands between two groups with 4 on either side. On a surface it has the background's fill, an edge of 1 in the border's color and the middle radius. A group of a command bar is padded 10 with a line at its end and its label of 10 in the muted foreground under it. A focused control has its ring inside it, where the bar does not cut it off.
- To what reads the tree it is a `toolbar` called by its label, with `a11y_orientation`; the More button says `a11y_haspopup` and `a11y_expanded`.
- A renderer that draws another bar gets a `ToolbarView`: `view.slot(index)` is a control as the bar lays it out (dropping when it does not fit), `view.more()` the More button with its menu, `view.menu()` the menu alone, `view.dropped` which slots went.

## Action Bar

The toolbar of a selection: over the list whose rows are selected, it says how many are, offers what can be done to them, and clears them.

```rust
use lurq::{components::Stack, ui::{ActionBar, Button, icons}};

Stack::new()
  .child(list)
  .child(
    ActionBar::new(selected.get().len())
      .action(Button::new("Move").icon(icons::FOLDER_IMPORT).on_click(move_them))
      .action(Button::new("Duplicate").icon(icons::COPY).on_click(duplicate))
      .destructive_action(Button::new("Delete").icon(icons::TRASH).on_click(delete))
      .on_clear(move || selected.set(Vec::new()))
      .escape_clears(true),
  );
```

| Builder | |
| --- | --- |
| `ActionBar::new(count)` | The bar of a selection of `count`: shown while it is more than none |
| `text(&str)` | What it says, in place of "3 selected" |
| `action(Button)`, `destructive_action(Button)` | An action on what is selected: a round ghost button; one that destroys in the destructive color |
| `on_clear(handler)` | What clearing does: the round button with a cross at the end (none without it), and Escape where it clears |
| `escape_clears(bool)` | Escape clears the selection while the bar is shown, wherever the focus is, unless what has it took the key |
| `placement(ActionBarPlacement)` | `Bottom` (the default) or `Top` of the room it is given |
| `clear_label(&str)` | What the clear button is called: "Clear selection" |
| `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. The clear button has the id with `-clear` after it |

- It is as large as the room it is given and takes no press itself: put it over what it acts on, in a `Stack`. The bar stands 16 from the bottom (or the top) of that room, in the middle; it comes in with a fade and a move of 16 from that edge, and leaves the same way as the count goes to none.
- It is a toolbar to what reads the tree, called by what it says, and one stop of Tab whose Left and Right move between its buttons.
- Parts: `ActionBarPart::Root`, `Bar`, `Count`, `Separator` (laid over a separator's root), `Action`, `ActionIcon`, `ActionLabel`, `Destructive` (laid over a button's root, icon and label), `Clear`, `ClearIcon`. State: `ActionBarState::Empty`. Variants: `ActionBarPlacement::Bottom`, `Top`.
- In the design the bar is 44 high, of the popover's color, round at its ends, with an edge of 1 in the border's color and the large shadow, padded 16 before its count and 6 after its clear button, its parts 4 apart. The count is 13/600; a line of 20 with 6 on either side sets it apart; an action is 32 high, padded 10, its icon 14 and its label 13, 6 apart; the clear button is a round 32 with a cross of 15. A phone's selection bar is the same parts restyled: across the top, 56 high, in the fill of what is picked out.

## For `CHANGELOG.md`

- `ui::Toolbar`: a row of tools that is one stop of Tab, with groups, lines, a spacer, tooltips of its icons and their shortcuts, and with `overflow(true)` a menu at its end of the controls that do not fit, the last first; the engine's `drop_marker` lays out a line's More button only while something is dropped.
- `ui::ActionBar`: the toolbar of a selection, with its count, its actions and its clear button, coming up over the list it acts on.

## Decided

- The toolbar is a mounted component (`Toolbar::mount(ctx)`), as the tooltip is: the menu of what does not fit has to be open or closed, it is made of what the last layout dropped, the bar returns the focus to the control it left, and its icon buttons carry tooltips, none of which a builder can keep or mount. `build(theme)` draws the bar once with nothing kept, for a window of its own.
- What does not fit is measured by the engine, in the layout, not by the bar: each control that may go is `ShrinkLimit::Drop` in its order, the More button is the line's `drop_marker`. So the bar never flickers as the window is resized; only the menu's content follows the layout, a pass later, through the element refs of the dropped controls (which are at no size).
- A line before a control is part of that control's slot, so that it goes with it: a bar never ends in a line before its More button with nothing after it. The More button has a line of its own before it, as the board draws it.
- Where Tab enters the bar is remembered by the bar (a signal of its host, set by the focus of each stop) and given to the engine as `focus_entry` on that stop. A one-stop focus group that remembers where it was left would serve every group; it is the focus group's kind of change, not this session's, and the bar does not need it.
- The items of a toggle group are among the bar's controls: its own focus group is taken off inside a toolbar, so the bar's arrow keys move through its toggles and on.
- A select or a field stays in the bar unless the app gives a `ToolbarCommand` to stand for it: a select's options would need a submenu with its own open state per select, and the bar cannot read them out of a `ui::Select`.
- Buttons of the bar are ghost ones of its size unless `as_given`: a `ui::Button` cannot tell a variant it was given from its default, so a bar that kept "what it was told" would keep every default primary.
- The tooltips of the bar stand below it: a toolbar is mostly at the top of what it works on. A focused control has its ring inside it (`Item`'s focus-visible outline of 2, offset −2): the bar clips what does not fit, and a ring outside would be cut by its edge.
- The action bar is a builder: what it shows is the app's count, and it comes and goes with the presence of its root, keyed. Escape clears only with `escape_clears`, because a key event reaches the bar whatever has the focus.

## Not done

Nothing.

## Differs from the board

- The command bar's tall commands (an icon of 20 over a label of 11 in a box of 56 by 52) and its stacked small ones (Cut, Copy at 24 high) are not built: a labelled group holds ordinary buttons and toggles of the bar's size. They are buttons of another shape; a `ButtonShape` or a renderer of the button would draw them. The gallery's command bar uses the extra-small size (28 with icons of 14, as its format group has them) and `ui::Tabs` restyled to 32 for its tabs.
- The table toolbar's controls are 30 high on the board (its density group, its menu button, Export); the gallery's are the small size, 32, with the bar restyled to 52 high, padded 12, 8 apart.
- The board's overflow menu is 180 wide; `ui::Menu` is at least 230, as the menu of the board's menu component is.
- The phone's selection bar on the board has the clear button first, then the count, then icons of 22 and a More button; the gallery restyles the action bar's parts, which keep their order (the count, the actions, the clear button), and has no More.
