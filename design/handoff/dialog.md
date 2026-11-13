## Changed outside the component's own files

- `crates/lurq/src/app/ctx.rs`: `Modal::takes_focus(bool)`, off by default, and the field `takes_focus` of
  `ModalSpec` that carries it (set in `into_spec`, copied in `clone_with_node`). A builder has no `Ctx` to call
  `ctx.focus` with, and the engine's modal moved no focus, so the dialog could not take the focus as it opens nor
  give it back as it closes without it.
- `crates/lurq/src/app/runtime/modal_focus.rs` (new): `Tree::sync_modal_focus`. A modal that takes the focus is
  told from another by the node that declares it. As one opens, what had the focus is remembered and the focus
  goes to the first stop of Tab of the topmost modal; as it closes, the focus goes back to what had it, unless the
  focus is on a node still in the tree (the app or a press put it somewhere).
- `crates/lurq/src/app/runtime.rs`: `mod modal_focus;`, two fields of `Tree` (`modal_focus`,
  `built_focus_modals`) with their initial values, the modal branch of `build_overlays_from_layout_index` records a
  modal that takes the focus, `sync_overlay_host_from_layout` keeps what it built (and clears it with no root), and
  the layout pass calls `sync_modal_focus()` after `return_focus_to_anchor` and before the app's focus request, so
  that a request of the app's wins.
- `crates/lurq/src/app/runtime/overlay_layers.rs`: `OverlayLayers::focus_modals`.
- For `docs/src/content/docs/modals.md`, under `## Behavior`, after the line on Tab: "`Modal::takes_focus(true)`
  moves the focus into the modal as it opens, to its first stop of Tab, and back to what had it as it closes, as a
  browser's `<dialog>` opened with `showModal()` does. A focus the app or a press has put somewhere else by then
  stays there. The focus follows the last input's modality, so a modal opened from the keyboard shows the ring on
  its first control. Off by default; `ui::Dialog` turns it on."

## For `docs/src/content/docs/design-system.md`

## Dialog

A panel over the window that asks one thing and waits for the answer: delete a workspace, accept the terms, name a new file.

```rust
use lurq::ui::{Button, ButtonVariant, Dialog, DialogSize};

// open: Signal<bool>, the app's.
let cancel = open.clone();
Dialog::new(open.clone())
  .title("Delete workspace?")
  .description("This removes every window layout and setting stored for this workspace. It cannot be undone.")
  .footer(Button::new("Cancel").variant(ButtonVariant::Outline).on_click(move |_| cancel.set(false)))
  .footer(Button::new("Delete").variant(ButtonVariant::Destructive));

// A body that scrolls between a header and a footer that stay, with a close button.
Dialog::new(terms_open.clone())
  .scrollable(true)
  .close_button(true)
  .title("Terms of use")
  .children(paragraphs)
  .footer(Button::new("Decline").variant(ButtonVariant::Outline))
  .footer(Button::new("Accept"));
```

| Builder | |
| --- | --- |
| `new(Signal<bool>)` | Shown while the signal holds `true`. The dialog does not keep it: Escape, a press on the veil and the close button set it to `false` |
| `title(&str)`, `description(&str)` | What it asks, and more of it under the title. The title is what the dialog is called |
| `child(element)`, `children(..)` | The body: a form, a list, paragraphs |
| `footer(element)` | Adds to the row at the end of the dialog, the least action first and the main one last. The app puts `Button`s there |
| `size(DialogSize)`, `width(f32)` | `Sm` 360, `Md` 480, `Lg` 640, `Full` the window; 420 without one. Never wider than the window |
| `scrollable(bool)` | The body scrolls between a header and a footer that stay, each set off by a line |
| `close_button(bool)` | A ghost button with the close icon at the end of the header |
| `dismiss_on_veil(bool)`, `dismiss_on_escape(bool)` | Whether a press on the veil, and Escape, close it. Both on by default |
| `inline(bool)` | The panel where it is declared, without the veil and without taking the window: for a gallery, a screenshot, a test |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's; `key` tells two dialogs of one component apart |

- It is the engine's `Modal` with `target(Root)` and `takes_focus(true)`: declared anywhere, it takes no room there and covers the whole window while it is open. As it opens the focus goes to its first control (the close button where it has one), and as it closes back to what had it, the button that opened it. Nothing under the veil takes the pointer, and Tab and Shift+Tab go round the dialog's own controls.
- A press on the panel goes no further: only a press on the veil itself closes the dialog.
- The panel is never larger than the window less the veil's margin of 16. A dialog is as high as its content while that fits, and its body gives way and scrolls when it does not, in either form; `style(DialogPart::Panel, |s| s.height(300.0))` gives it a height of its own.
- Parts: `DialogPart::Veil`, `Panel`, `Header` (the heading and the close button), `Heading` (the title over the description), `Title`, `Description`, `Body`, `Footer`, `Close` (laid over the theme's button), `Separator` (the lines of a scrollable dialog). States: `DialogState::Scrollable`. Sizes: `DialogSize::Sm`, `Md`, `Lg`, `Full`.
- In the design the panel is 420 wide in `Popover` with an edge of `Border`, corners of `radius-xl` and the `lg` shadow, padded 24 with 20 between its heading, its body and its footer; the veil is `Scrim`. The title is 16/600, the description 8 under it in 14 of `MutedForeground`, and the footer's buttons stand at its end 8 apart. The scrollable one has a header of 56 (its title 20 from the edge, the close button 12 from it) and a footer of 60, the body padded 20 and 16 between them. The close button is 28 with an icon of 16.
- To what reads the tree the panel is a `dialog` called by its title.
- A renderer that draws another dialog builds its panel from `view.panel()`, the parts from `view.header()`, `view.body()`, `view.footer()`, `view.close()` and `view.separator()`, and lays it over the window with `view.modal(panel)`; `view.closer()` closes it.

## For `CHANGELOG.md`

- `ui::Dialog`: a panel over a veil that covers the window, the engine's `Modal` dressed by a recipe, with a title, a description, a body, a footer of buttons, a close button, sizes, and a form whose body scrolls between a header and a footer that stay; it takes the focus as it opens and gives it back as it closes, with the new `Modal::takes_focus`.

## Decided

- A builder, not a mounted component: what is open is the app's `Signal<bool>`, and the engine's `Modal` (`open(signal)`, `target(Root)`) does the rest. Escape is the `Modal`'s; Tab is kept inside by `tab_navigation.rs` as for any modal.
- The focus is the engine's (`Modal::takes_focus`, above), not the dialog's: a builder cannot move it, and every modal of an app may want the same.
- The veil is a column as large as the window with the panel in its middle; a press on it closes the dialog, and the panel stops the propagation of a press so that one on the panel never reaches the veil. With `dismiss_on_veil(false)` the veil has no handler and still takes the pointer.
- The panel has `max_width` and `max_height` of 100% of the veil's room, and the veil has a margin of 16, so that a dialog is never larger than the window. The body is a `ScrollVertical` with `flex_full(1, 1, None)` in both forms: it takes the panel's height beyond the header and the footer, and gives way and scrolls when the content needs more than the window has.
- A scroll container clips at its edges, and the theme's focus ring stands 4 outside a button. So that a button at the edge of the body keeps its ring, 4 of the panel's padding of 24 and gap of 20 are the header's, the body's and the footer's own: the panel is padded 20 with 12 between its parts, and each part is padded 4. What is drawn is where the board has it.
- The lines of the scrollable dialog are a part of their own (`Separator`, 1 high in `Border`) rather than edges of the header and the footer: `PartStyle` has a border on every side or a line along the bottom, not one along the top.
- The close button is a `ui::Button` (ghost, `ButtonSize::Sm`, `icons::CLOSE`, icon alone) over which the dialog's `Close` part is laid with `rules_for(Close, ButtonPart::Root)`: 28 square, `radius-md`, its icon the `Sm` button's 16. It is called "Close".
- Without a size the panel is 420, the board's dialog; the sizes are the widths the board's card names (sm 360, md 480, lg 640, full).
- No animation in or out: the board draws none, and the engine has no keyframes for one that a builder could name (`Presence` needs keyframes registered on the `Tree`, as `SPIN` is in `AnimationEngine::new`).
- The footer is not a `FocusGroup`: its buttons are separate answers, not one choice or one bar, and Tab stops at each as a browser's dialog does.
- The tests give the stage's window the size of the page (`stage.tree.resize(400, 300)`): a dialog covers the window, and the headless stage's window is otherwise larger than the 400 by 300 it lays the page out in.
- `inline(true)` draws the panel alone where the dialog is declared, for the gallery's drawing of the board's two dialogs and for tests.
- The body has 10 between its children in both forms; the board shows 10 only in the scrollable one.

## Not done

Nothing.

## Differs from the board

- The board's scrollable dialog is 440 by 300. Here it is as wide as any dialog (420 without a size), and as high as its content up to the window; 300 is given with `style(DialogPart::Panel, |s| s.height(300.0))`, as the gallery does.
- The board's body of the scrollable dialog is padded 16 at the top, 6 at the end and nothing at the bottom, with its own scrollbar (`Scrollbar`, 8 wide, `ring`) in the 6. Here it is padded 20 at both sides and 16 at the top and the bottom, so that its last line has room when it is scrolled to the end, and the scrollbar is the engine's own, drawn over the content.
- The footer's buttons are the app's: the board draws them 36 high in the dialog and 32 high, padded 14, in the scrollable one. The dialog does not size them.
- The board's title is `popover-foreground` in the dialog and `foreground` in the scrollable one; both are `PopoverForeground` here (the same color in both appearances of the design).
- The veil's margin of 16 around the panel is not on the board.
- The board's plain dialog has no close button; here one can be asked for, at the end of the header, level with the top of the title.
