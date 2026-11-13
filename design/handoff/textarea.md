# Textarea: handoff

Built on `claude/ds-textarea-cc1162`. Files: `crates/lurq/src/ui/textarea.rs`, `crates/lurq/tests/ui/textarea.rs`,
`crates/gallery/src/pages/textarea.rs`, and a line or an entry each in the three `mod.rs` files. Nothing in shared
code (`shared.rs`, `part_style.rs`, `recipe.rs`, the engine) was changed.

## For `docs/src/content/docs/design-system.md`

Put after `## Switch`:

````markdown
## Textarea

Text of more than one line, typed into a box: a bio, a note, what happened.

```rust
use lurq::ui::Textarea;

// bio: Signal<String>
Textarea::new(bio.clone()).placeholder("A few words about you");

// Two rows while it holds less, four before its text scrolls; not accepted.
Textarea::new(report.clone()).rows(2, 4).invalid(true);
```

| Builder | |
| --- | --- |
| `new(Signal<String>)` | What it holds. What is typed is written into the signal, and what the app writes there shows |
| `placeholder(&str)` | What it shows while it holds nothing |
| `rows(min, max)` | The rows it shows at least, and at most before its text scrolls inside it. 3 and 8 unless told (`TEXTAREA_MIN_ROWS`, `TEXTAREA_MAX_ROWS`) |
| `invalid(bool)` | What it holds is not accepted |
| `on_change(handler)` | Called on every edit with the text it holds once the edit applies |
| `label(&str)` | What it is called, to what reads the tree. It draws nothing: a field's label is above it |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box |

- It is the engine's multiline `TextInput`: as high as its text between its rows, its own scroll bar past them, Enter
  a new line. Tab reaches it wherever it is, as every control of the design system; a disabled one is passed by and
  takes no click.
- Parts: `TextareaPart::Root` (the box, which is the text input), `Text` (the type and color of what it holds),
  `Placeholder`. States: `TextareaState::Invalid`, `Disabled`.
- In the design the box is as wide as its room, in `Background` with an edge of 1 in `Input`, the medium radius,
  padded 12 across and 8 down; its text is in the body's type with a line of 1.5, in `Foreground`, and the
  placeholder in `MutedForeground`. Under the pointer the edge is `Ring`; focused, a ring of 2 is drawn inside the
  box, over the edge. Invalid, the edge and the ring are `Destructive`. Disabled, it is at the theme's disabled
  opacity on `Muted`.
- A text input draws in its own text style and does not take the foreground of what it is in, so the `Text` and
  `Placeholder` parts' foregrounds are resolved to colors of the theme when the textarea is built
  (`TextareaView::text_style`).
- To what reads the tree it is a `textbox`, `a11y_multiline`, with `a11y_invalid` and the name `label` gives it.
- A renderer that draws another textarea starts from `view.input()`, the behavior in the box's style, and puts what
  it likes around it: a count of characters, a grip.
````

## For `CHANGELOG.md`

- `ui::Textarea`: text of more than one line in the design's box, bound to a `Signal<String>`, with a placeholder,
  the rows it shows at least and at most, invalid and disabled states, and a page in the gallery.

## Decided

- **The box is the text input.** No node around it: the engine's `TextInput` takes the box's background, edge,
  radius and padding itself, and is the node that is focused, so the ring is on it, where the board draws it.
- **The ring of the focus is inside the box**, a line of 2 at an offset of -2 (`Outline`), as the board draws an
  input's focus (`$ring`, 2, an inner stroke). It is not the theme's `focus_ring` (2, outside, offset 2) that
  `Switch` and the buttons use. If the batch decides every field takes the theme's ring, it is one rule in the recipe.
- **Invalid wins over the pointer and the focus**: an invalid textarea keeps the destructive edge under the pointer,
  and its ring is destructive. The board draws neither case.
- **Rows are props, not the recipe's**: a part style has no rows. Three at least and eight at most by default.
- **`on_change(&str)`** besides the signal, called with the text after the edit (the engine's `on_input` gives
  the text before it); the gallery counts it.
- **Text in the body's role** with a line of 1.5 set over it, so a theme that changes the body's size changes it.

## Not done

- **No read-only state.** The board draws a read-only input (`$muted` fill, `$border` edge); the engine's
  `TextInput` has no read-only mode (it is editable or disabled). It needs an engine change first.
- **No valid state.** The board draws one for an input (`$success` edge and a check icon), not for a textarea; an
  app can add one with `.state("valid", true)` and a rule.
- **No count of characters and no resize grip**: the board draws neither on the textarea. Its "18 / 40 characters"
  is a field's description, which is `Field`'s (built by the Input session). A theme's renderer can add a count (the
  test does).
- **No `name` for a form.** `TextInput::name` is behind the `form` feature; the textarea is bound to a signal as the
  engine's is, and a `Field`/form integration should decide how the design system's inputs join a form.
- **No sizes.** The board draws an input at 32, 36 and 40 high; a textarea has one size.
- What it has in common with `ui::Input` (the edge, the ring inside, the placeholder's color, resolving a part's
  foreground into a `TextStyle`) is written here twice by design: whoever merges the batch brings it together.

## Differs from the board

- **Its height is a number of rows.** The board's master is 88 high (and 64 in `Field bio`), which is not a whole
  number of lines of 21 with the padding of 16. The textarea is three rows (79) while it holds less, and grows.
- **The master is 260 wide; the textarea is as wide as its room** (`Length::Fill`), as the board's instances are
  (`fill_container` in the field). In a parent as wide as its content it needs a width (`.style(TextareaPart::Root,
  |s| s.width(260.0))`).
- **Under the pointer the edge is `Ring`**, from the board's hovered input; the board draws no hovered textarea.
- The tests assume an inside edge takes no room in layout (the box is rows × 21 + 16 high); they were not run.
