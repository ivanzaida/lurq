# Message Box, Popconfirm, Progress Dialog: hand-off

`ui::MessageBox`, `ui::Popconfirm` and `ui::ProgressDialog`, from the board's `Message Box` (`v6DOl`), `Popconfirm`
(`znPAr`) and `Progress Dialog` (`HwgAn`) in Components / light (`Ovt1D`). The message box is drawn in Desktop Dialogs
› Message boxes as a warning with "Don't ask again" and as a destructive question (`tApxf`: a trash in the destructive
well, a Delete button in the destructive fill), and the same frame's Prompts draws "Unsaved changes · three choices"
(`eecVr`), a message box with a third, plain answer at the start of its footer. `Mobile/Alert Dialog` (`miZtO`) was
read for what a renderer must be able to build from a message box's view. The popconfirm is drawn once, at rest
(States, `LmHgz`); the progress dialog once, in Desktop Dialogs › Progress (`ZHQYe`).

Files: `crates/lurq/src/ui/message_box.rs`, `popconfirm.rs`, `progress_dialog.rs`; `crates/lurq/tests/ui/message_box.rs`,
`popconfirm.rs`, `progress_dialog.rs`; `crates/gallery/src/pages/message_box.rs`, `popconfirm.rs`,
`progress_dialog.rs` (pages "Message Box", "Popconfirm", "Progress Dialog", group Surfaces); this file; lines in the
three `mod.rs` files. No keyframes taken.

## Changed outside the component's own files

- **Engine: an element the focus goes to first** (`autofocus`). `crates/lurq/src/node/node.rs`: `Node` has a field
  more, `autofocus: bool` (in `Node::new`, the clone, `NodeUpdate::autofocus`, `Node::is_autofocus`), and
  `crates/lurq/src/node/dsl.rs` sets it in its literal; `crates/lurq/src/components/mod.rs`: every element has
  `.autofocus(bool)`. `crates/lurq/src/app/runtime/tab_navigation.rs`: a `Stop` says whether it is marked, and with
  nothing focused (a modal opening, which `sync_modal_focus` asks for, or Tab in a window with no focus) the first
  marked stop is taken in place of the first. `crates/lurq/src/app/runtime/overlay_layers.rs`: an overlay that takes
  the focus (`Overlay::takes_focus`) gives it to a marked element in it before its first focusable one. Needed
  because a message box's focus goes to its default answer, which is last in the tree, and a builder has no `Ctx`
  to focus it; the size of `Node` stays within its budget (the field sits beside two other `bool`s).
- **`ui::Dialog`** (`crates/lurq/src/ui/dialog.rs`): `on_dismiss(handler)`, what runs after a press of the veil, the
  close button or Escape closed the dialog (`DialogView::dismisser()`; a dialog with one handles Escape on its
  panel while the focus is in it and turns the modal's own Escape off, so that Escape never closes it without
  running it); `alert(bool)`, an `alertdialog` and not a `dialog` to tooling; `named(&str)` and `described(&str)`,
  the panel's `a11y_name` and `a11y_description` for a dialog whose title is drawn by what it holds. A message box
  needs all four: its cancel answer is what dismissing it gives, and its title and message are in its body beside
  its icon, where the dialog's header has no room for an icon.
- **`ui::Popover`** (`crates/lurq/src/ui/popover.rs`): `alert(bool)`, `named(&str)`, `described(&str)`, as the
  dialog has them, for the popconfirm's question beside its icon.
- **Engine: what runs as an overlay is dismissed.** `Overlay::on_dismiss(handler)` and `Popup::on_dismiss`
  (`app/ctx.rs`, `components/popup.rs`; `OverlaySpec::on_dismiss` and `ctx::OverlayDismiss`, the handler's type), run
  after a press outside the overlay or Escape set its open signal to `false`. The functions touched, on the dismiss
  path only: in `app/ctx.rs` the `Overlay` and `OverlaySpec` structs, `Overlay::new`, `Overlay::on_dismiss` (new),
  `Overlay::into_spec` and `OverlaySpec::clone_with_node`; in `app/runtime.rs` the `OverlayDismissEntry` struct (a
  field `on_dismiss`), `build_overlays_from_layout_index` (the overlay's and the modal's `HiddenDismiss` and
  `OverlayDismissEntry`, a modal's with none), `dismiss_top_overlay_on_escape` (runs it) and, for a left press, in
  `dispatch_mouse_with_click_target` (the loop over `overlay_dismiss_signals`, which now holds pairs); in
  `app/runtime/overlay_layers.rs` the `HiddenDismiss` struct and `OverlayLayers::hide`; in
  `app/runtime/outside_press.rs` `consume_outside_press` and `overlay_dismiss_signals_at` (which returns each signal
  with its handler). `ui::Popover::on_dismiss` hands it on; a popconfirm gives it its cancel handler.
- **The icon font: `help-circle`**, a question mark in the circle of `info`'s weight, for `MessageBoxKind::Question`.
  `design/icons/svg/help-circle.svg` (the ring of `info.svg`, the hook and the dot of a stroke of 2 with round ends,
  outlined), its codepoint U+E099 in `design/icons/codepoints.json`, `crates/lurq/assets/lurq-icons.ttf` and
  `crates/lurq/src/ui/icons.rs` (`icons::HELP_CIRCLE`) built by `python scripts/build-icon-font.py`. The board has no
  `Icon/help-circle`: a `--from-board` build keeps the source and names it as gone from the board.
- `crates/lurq/src/ui/message_box.rs` has, for the progress dialog too, `disclosure` and `disclosure_recipe`
  (`pub(crate)`): the board's "Details", a chevron and a label that open a text under them; and `autofocus`, which
  marks the first focusable node of an element. `ui::Collapsible` was not used: its indicator is at the end of its
  header and 16, where the board's "Details" has its chevron of 14 before the label.

## For `docs/src/content/docs/design-system.md`

## Message Box

A dialog that asks one question or says one thing and waits for the answer: replace a file, save the changes, delete projects.

```rust
use lurq::ui::{MessageBox, MessageBoxKind, MessageBoxRole, icons};

MessageBox::new(open.clone())
  .kind(MessageBoxKind::Warning)
  .title("Replace existing file?")
  .message("“theme.json” already exists in this folder. Replacing it overwrites its contents.")
  .checkbox("Don't ask again", again.clone())
  .answer("Cancel", MessageBoxRole::Cancel, || {})
  .answer("Replace", MessageBoxRole::Default, move || replace_file());

// A question that cannot be undone: the focus goes to Cancel.
MessageBox::new(open.clone())
  .icon(icons::TRASH)
  .title("Delete 3 projects?")
  .message("This cannot be undone.")
  .answer("Cancel", MessageBoxRole::Cancel, || {})
  .answer("Delete", MessageBoxRole::Destructive, move || delete_projects());
```

| Builder | |
| --- | --- |
| `MessageBox::new(Signal<bool>)` | Shown while the signal holds `true`. The message box does not keep it: every answer sets it to `false` before what the answer does runs |
| `kind(MessageBoxKind)` | `Information` (the default, `info`), `Question` (`help-circle`), `Warning` (`warning`), `Error` (`alert-circle`): the icon in the well and their colors. A theme may add kinds by name |
| `icon(&str)` | An icon of the theme's set in place of the kind's: `trash` for a question that deletes |
| `title(&str)`, `message(&str)` | What it asks or says, and more of it in the muted color. The title is what it is called, the message what describes it |
| `detail(&str, Signal<bool>)` | A text behind "Details" under the message, open while the signal holds `true`; `detail_label(&str)` names the trigger |
| `checkbox(&str, Signal<bool>)` | A checkbox at the start of the footer: "Don't ask again" |
| `answer(&str, MessageBoxRole, handler)` | An answer. `Default`: filled, Enter gives it. `Cancel`: outlined, Escape and a press of the veil give it. `Destructive`: in the destructive fill. `Plain`: outlined, at the start of the footer |
| `width(f32)` | A width of its own over the board's 400 |
| `inline(bool)` | The panel where it is declared, without the veil: for a gallery, a screenshot, a test |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [dialog](#dialog) has them. `id` is the id of the panel |

- It is a [dialog](#dialog): the veil, the focus going in as it opens and back as it closes, and Tab going round its controls are the dialog's. As it opens the focus goes to the default answer, or to the cancel one where an answer is destructive.
- Enter gives the default answer wherever the focus is, on the checkbox too, unless it is on another button, which Enter presses. Escape and a press of the veil give the cancel answer where there is one and do nothing where there is none.
- The plain answers stand at the start of the footer, after the checkbox, in their order; at its end the cancel answer stands first and the others after it, whatever order they were given in.
- Parts: `MessageBoxPart::Veil` and `Panel` (laid over the dialog's), `Content`, `Body`, `IconWell`, `Icon`, `Text`, `Title`, `Message`, `DetailTrigger`, `DetailIcon`, `DetailLabel`, `DetailPanel`, `Detail`, `Footer`, `Option`, `Checkbox` and `CheckboxLabel` (laid over the checkbox's root and label), `Actions`, and for each role a part laid over the button's root and one over its label: `DefaultButton`, `DefaultLabel`, `CancelButton`, `CancelLabel`, `DestructiveButton`, `DestructiveLabel`, `PlainButton`, `PlainLabel`. Variants: the kinds. States: `MessageBoxState::Destructive` (an answer is destructive: the icon in the destructive colors), `DetailOpen`.
- So a theme gives the two buttons of a message box fonts or fills of their own without touching a button anywhere else: `theme.patch_recipe::<MessageBox>(|rules| rules.base(MessageBoxPart::DefaultLabel, |s| s.font_family("Georgia")).base(MessageBoxPart::CancelLabel, |s| s.font_family(FontFamily::Mono)))`.
- In the design the panel is 400 wide, padded 20 with 16 between the body and the footer; the well is 36 and round, in the kind's muted color (`InfoMuted`, `Secondary` for a question, `WarningMuted`, `DestructiveMuted`), its icon 18 in the kind's color; the title is 15/600, 14 after the well, over the message of 13 in `MutedForeground`, 4 apart; the buttons are 32 high, padded 14, 8 apart; the checkbox's label is 13. "Details" is a chevron of 14 in the muted color 4 before its label of 13, and the detail opens 8 under it, in the mono type of 11 on the `Muted` fill, with its height over the theme's base duration.
- To what reads the tree it is an `alertdialog` called by its title and described by its message.
- A renderer that draws another message box (an iOS alert: the text in the middle, the answers as cells side by side) has the pieces: `view.content()` (with Enter), `view.title()`, `view.message()`, `view.find(role)`, `view.press(index)` (it closes the box and runs the answer), `view.button(index)` (the default renderer's button, which takes the focus first where it should), and `view.dialog(content)`, which puts what it built in the dialog with Escape, the veil and what tooling is told.

## Popconfirm

A question in a small panel hung from what was pressed, answered by one of two buttons: delete a file, leave a channel.

```rust
use lurq::ui::{Button, ButtonVariant, Popconfirm};

Popconfirm::new(anchor.clone(), open.clone())
  .trigger(Button::new("Delete file").variant(ButtonVariant::Outline))
  .title("Delete this file?")
  .description("It will be moved to the trash.")
  .destructive(true)
  .confirm("Delete", move || delete_file())
  .cancel("Keep", || {});
```

| Builder | |
| --- | --- |
| `Popconfirm::new(anchor, Signal<bool>)` | Hung from `anchor`, shown while the signal holds `true`. Keep the anchor from one render to the next, as for a [popover](#popover) |
| `trigger(element)` | A button that asks: it is the anchor, and a press of it turns the signal |
| `title(&str)`, `description(&str)` | The question, and more of it in the muted color |
| `icon(&str)` | An icon of the theme's set before the title, in place of the warning sign |
| `confirm(&str, handler)`, `cancel(&str, handler)` | The two buttons: their labels ("Yes" and "No" when not given) and what each runs after it closes the popconfirm. Escape and a press outside run the cancel one |
| `destructive(bool)` | The confirm button and the icon in the destructive color |
| `placement(Placement)`, `offset(..)` | Where it is beside its anchor: under it at its start, 4 from it, by default |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [popover](#popover) has them. `id` is the id of the panel |

- It is a [popover](#popover): kept in the window, gone to the other side where there is no room, the focus back on the anchor as it closes. As it opens the focus goes to the cancel button, so that Enter or Space does not confirm what was not read.
- Escape and a press outside close it and run what the cancel button runs.
- Parts: `PopconfirmPart::Panel` and `Body` (laid over the popover's root and body), `Content`, `Message`, `Icon`, `Text`, `Title`, `Description`, `Actions`, `CancelButton`, `CancelLabel`, `ConfirmButton`, `ConfirmLabel`. States: `PopconfirmState::Destructive`.
- In the design the panel is 250 wide, padded 12 with 12 between the message and the buttons, in the popover's colors, edge, large radius and medium shadow; the icon is the warning sign of 16 in `Warning`, 8 before the title of 13/500 over the description of 12 in `MutedForeground`, 2 apart; the buttons are 28 high, padded 10, their labels 13, 8 apart at the end.
- To what reads the tree it is an `alertdialog` called by its title and described by its description.
- A theme's renderer builds what the panel holds from `PopconfirmView` (`message()`, `cancel()`, `confirm()`, `actions()`, `content()` with Escape, `confirmer()`, `canceller()`); where the panel is shown, and when, stays the popover's.

## Progress Dialog

A dialog over the window while a piece of work runs: what is being done, how far it is, and a way to stop it.

```rust
use lurq::ui::{ProgressDialog, icons};

ProgressDialog::new(open.clone())
  .icon(icons::COPY)
  .title("Copying 128 items")
  .message("from Downloads to projects/lurq/assets")
  .value(copied.get())
  .max(128.0)
  .format(|done, all| format!("{done} of {all}"))
  .status("about 2 min left")
  .detail(&current_file.get(), details.clone())
  .action("Pause", move || pause())
  .on_cancel(move || stop());
```

| Builder | |
| --- | --- |
| `ProgressDialog::new(Signal<bool>)` | Shown while the signal holds `true`. It closes when the app sets it to `false`: Escape and a press of the veil do not close it |
| `icon(&str)`, `title(&str)`, `message(&str)` | What kind of work it is, what is being done (what it is called), and a line more of it in the muted color |
| `value(f64)`, `max(f64)` | How far the work is, a fraction of one unless `max` says otherwise. Without a value its end is not known, and the bar runs a segment across it |
| `format(Fn(f32, f32) -> String)` | Says how much is done under the start of the bar: "42 of 128". A percentage when not given |
| `status(&str)` | What the app says of the rest, under the end of the bar |
| `detail(&str, Signal<bool>)` | A text behind "Details" at the start of the footer, open under the footer while the signal holds `true`; `detail_label(&str)` names the trigger |
| `action(&str, handler)` | A button before the cancel one: "Pause". It does not close the dialog |
| `on_cancel(handler)`, `cancel_label(&str)` | A cancel button, which has the focus as the dialog opens and which Escape presses. It does not close the dialog: the app does once the work has stopped |
| `width(f32)`, `inline(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [message box](#message-box) has them |

- It is a [dialog](#dialog) with its veil, its focus and its Tab, which nothing but the app closes.
- Parts: `ProgressDialogPart::Veil` and `Panel` (laid over the dialog's), `Content`, `Header`, `Icon`, `Text`, `Title`, `Message`, `Progress` (laid over the [progress](#progress)'s root), `Meta`, `Count`, `Status`, `Footer`, `DetailTrigger`, `DetailIcon`, `DetailLabel`, `DetailPanel`, `Detail`, `Actions`, `ActionButton`, `ActionLabel`, `CancelButton`, `CancelLabel`. States: `ProgressDialogState::Indeterminate`, `DetailOpen`.
- In the design the panel is 420 wide, padded 20 with 12 between its parts; the icon is 20, 12 before the title of 15/600 over the message of 12 in `MutedForeground`, 2 apart; the bar is the progress's of 6 across the panel; what is done and what is left are in the mono type of 11 in `MutedForeground`; the footer, 4 further down, has "Details" at its start and the outlined buttons of 32, padded 14, 8 apart, at its end.
- To what reads the tree it is a `dialog` called by its title and described by its message, with the `progressbar` in it.

## For `CHANGELOG.md`

- `ui::MessageBox`: a dialog that asks one question or says one thing, with an icon by kind (information, question, warning, error), a title, a message, a detail behind "Details", a "Don't ask again" checkbox, and answers by role (default, cancel, destructive, plain) whose buttons the message box's recipe styles apart from buttons elsewhere; Enter gives the default answer, Escape and the veil the cancel one, and the focus goes to the default answer or, for a destructive question, to cancel. Every element has `.autofocus(bool)`, where the focus goes first as the modal or the overlay it is in opens; `ui::Dialog` has `on_dismiss`, `alert`, `named` and `described`.
- `ui::Popconfirm`: a question in a small panel hung from what was pressed, over `ui::Popover`, with a confirm and a cancel button, the focus on cancel as it opens, Escape and a press outside that cancel, and a destructive look; `ui::Popover` has `alert`, `named`, `described` and `on_dismiss`, and `Overlay` and `Popup` have `on_dismiss`, what runs after a press outside or Escape closed them.
- `ui::ProgressDialog`: a dialog that nothing but the app closes while work runs, with what is being done, a `ui::Progress` with a value or without one, how much is done and what is left, a detail behind "Details", buttons of the app's, and a cancel button that Escape presses.

## Decided

- A message box is a `ui::Dialog` with its own body: the dialog's header holds a heading and a close button, and the board puts the title and the message beside an icon. The dialog is told the title and the message for tooling (`named`, `described`) and is an `alertdialog` (`alert`).
- An answer closes the message box first and then runs, so that an answer may open the box again with the same signal.
- Enter is the default answer while the focus is nowhere in the box or on its checkbox (the checkbox's Enter does not check it: Space does); on any other button Enter presses that button. Drawn inline, a box does not take the page's Enter.
- The answers are placed by role, not by the order they are given in: plain ones at the start, cancel first at the end. A renderer that wants another order (Windows puts OK first) places them itself.
- The kind is the recipe's variant, so a theme adds kinds by name; the icon of a kind the theme names is the information sign unless `icon` is given. A box with a destructive answer has its well in the destructive colors whatever its kind (the board's destructive box).
- The detail and the checkbox take the app's signals, as the open state does: a builder has no `Ctx` to keep them.
- The progress dialog's cancel button runs the app's handler and does not close the dialog: the work stops in the app's time, and the app closes it. Escape presses cancel, as Windows' progress dialogs do; without `on_cancel` Escape does nothing.
- A popconfirm's default placement is the popover's (under the anchor at its start); the board does not show one.
- Of the panel's padding 4 are the inner part's (the dialog's body, the popover's body), room for the ring of a focused button that the inner part would clip; the sum is the board's 20 and 12.

## Not done

Nothing.

## Differs from the board

- The board draws two kinds of message box (warning, and a destructive question). The information, question and error kinds, their icons and their colors are decided here: `info` on `InfoMuted`/`Info`, `help-circle` (drawn here, after `info`) on `Secondary`/`Foreground`, `alert-circle` on `DestructiveMuted`/`Destructive`.
- The board's three-choice prompt has an app icon in a primary square of radius 8 and a title with a line height of 1.35; here every message box has the kind's round well, and the gallery draws that prompt as a question with `icons::BOX` and a width of 440.
- The board's message box has no detail. Its "Details" is the progress dialog's (a chevron of 14 and 13 of text); the detail it opens, in the mono type of 11 on the muted fill, is decided here.
- The board's progress dialog shows "Details" closed only: what it opens, under the footer, is decided here.
- The board's destructive box shows a trash: here the kind's icon stays unless the app gives `icon(icons::TRASH)`, as the gallery does.
- The mobile alert (`miZtO`) is not a component of its own: it is what a renderer builds from a message box's view, which the tests do.
