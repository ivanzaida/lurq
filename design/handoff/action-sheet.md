# Action Sheet

`ui::ActionSheet`, from the board's `Mobile/Action Sheet` (`mBvxg`, 344 wide) in the Mobile frames, and its use under
a long press of the phone's picture (`rOZRH`, Mobile Picture `VmTdR`). Branch `claude/picture`.

## Changed outside the component's own files

- `crates/lurq/src/ui/mod.rs`, `crates/lurq/tests/ui/mod.rs`, `crates/gallery/src/pages/mod.rs`: the lines of
  `action_sheet`; the page is in `Group::Surfaces`, beside the sheet and the dialog: the gallery has no group for
  the phone.
- `ui::MenuItem`: `label_of`, `is_destructive`, `select_handlers` (crate-private): the sheet's options are menu
  items.
- `ui::Picture` uses it for its actions on a phone (`design/handoff/picture.md`).

## For `docs/src/content/docs/design-system.md`

## Action Sheet

On a phone, the actions on something, risen from the bottom of the screen over a scrim, and Cancel under them.

```rust
use lurq::ui::{ActionSheet, MenuItem};

// open: Signal<bool>. An option does what it does and closes the sheet.
ActionSheet::new(open.clone(), [
  MenuItem::new("Save to library").on_select(save),
  MenuItem::new("Duplicate").on_select(duplicate),
  MenuItem::new("Delete photo").destructive(true).on_select(delete),
])
.title("Photo options");
```

| Builder | |
| --- | --- |
| `new(open, items)`, `item(MenuItem)` | Open while the signal says so, with its options: what an item says, whether it destroys something or is disabled, and what choosing it does. An icon, a shortcut and a submenu of an item are not shown |
| `title(&str)` | What the sheet is about, over its options |
| `cancel_label(&str)` | What Cancel says: "Cancel" |
| `on_dismiss(Fn())` | Told when it is closed with no option: Cancel, a press on the scrim, Escape |
| `inline(bool)` | The sheet where it is declared, open or not, without the scrim: for a gallery, a screenshot, a test |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. Its options are `<id>-option-<n>`, Cancel `<id>-cancel` |

- It rises from the bottom as the scrim fades in, and goes down as it fades out. It takes the focus as it opens and
  gives it back as it closes; the arrow keys move among the options, and Enter and Space choose.
- Parts: `ActionSheetPart::Scrim`, `Root`, `Options`, `Title`, `Option`, `Label`, `Cancel`, `CancelLabel`. States:
  `ActionSheetState::Destructive`, `Disabled`.
- In the design the options are a card in `Card` with a line of `Border` and the large corners, 8 in from the
  screen's sides and bottom: the title 13 in `MutedForeground`, 12 over and under; options of 56 with a line over
  each and their words 17 in the middle, in `Foreground` or `Destructive`; Cancel 8 under them, a card of its own,
  17 semibold. The scrim is `Scrim`.
- To what reads the tree it is a `dialog` called by its title, its options and Cancel `button`s.

## For `CHANGELOG.md`

- `ui::ActionSheet` is a component of the design system: a phone's choice among a few actions, risen from the
  bottom over a scrim, with Cancel under them; a long press of a picture on a phone brings its own.

## Decided

- The options are `MenuItem`s: an app that has a menu for a pointer has its sheet for a finger with the same items,
  and a picture shows its menu's items in it.
- It is the engine's `Modal` over the window, as the dialog and the sheet are: Escape, the scrim, the focus. Over
  the window it is a component that keeps clear of what the system draws over the screen's lower edge
  (`Ctx::safe_area`); drawn in place it is built as it is.
- Seen in the gallery's window at 390 by 844: as the board draws it, and an option closes it.

## Not done

- Nothing.

## Differs from the board

- Nothing known.
