---
title: Source-backed input tokens
description: Retained insertion at the selection and inline decorations over a plain String.
---

`TextInputController` edits the input's own selection through its normal `on_input` and undo path.
`InputTokens` decorates validated UTF-8 ranges of the current raw `Signal<String>`; it does not replace
that value with embedded objects or display labels.

## Retained picker insertion

Keep one controller in the component for each stable keyed input. Construct the field on every render,
then request ordinary runtime focus after closing the picker. A picker callback can replace the retained
selection even after focus moved to the picker:

```rust
use lurq::{
  app::ctx::Ctx,
  components::{InputEditError, TextInput, TextInputController},
  core::Signal,
};

struct Editor {
  source: Signal<String>,
  controller: TextInputController,
  return_focus: Signal<bool>,
}

impl Editor {
  fn field(&self, ctx: &Ctx) -> TextInput {
    let field = TextInput::new(self.source.clone())
      .key("instruction-prompt")
      .controller(self.controller.clone())
      .multiline();
    if self.return_focus.get() {
      self.controller.focus(ctx);
    }
    field
  }

  fn choose_variable(&self, placeholder: &str) -> Result<bool, InputEditError> {
    let accepted = self.controller.replace_selection(placeholder)?;
    if accepted {
      // Close the app's picker, then request focus in the next render.
      self.return_focus.set(true);
    }
    Ok(accepted)
  }
}
```

Call edits from normal accepted application callbacks, not while constructing a speculative field.
The controller does not override the runtime's retained identity, focus rules, or synthetic-input consent.
Changing the field key/id follows normal input reconciliation. A controller observes its live state weakly,
so a removed input cannot be edited. Do not attach one controller to several simultaneous fields.

`selection()` reports the caret and optional anchor as raw UTF-8 byte offsets. `select(anchor, caret)`
rejects positions outside the value or inside a UTF-8 code point. `replace_selection()` returns `false`
when an `on_input` handler prevents the edit or an empty replacement has no selection. An accepted edit
is one undo step and follows the same validation as keyboard insertion. Its event uses the editor command
code `InsertText` and has no physical key target. A masked input refuses controller metadata and edits;
an active IME composition returns `Composing` and keeps the preedit intact.

## Measured raw-source pills

Parse known expressions in the application and supply the exact current source and its ranges:

```rust
use lurq::{
  app::theme::{PaletteColor, RadiusSize, SpacingSize, TypographyStyle},
  components::{InputToken, InputTokens, InputTokenStyle, TextInput},
  core::Signal,
};

fn field(source: Signal<String>) -> Result<TextInput, lurq::components::InputTokenError> {
  let raw = source.get();
  let token_style = InputTokenStyle::new(
    PaletteColor::extra("variable_text"),
    PaletteColor::extra("variable_fill"),
  )
    .padding(SpacingSize::extra("variable_padding"))
    .radius(RadiusSize::extra("variable_radius"))
    .typography(TypographyStyle::extra("variable_font"));
  let tokens = raw.find("{{task.title}}")
    .map(|start| vec![InputToken::new(start..start + "{{task.title}}".len(), token_style)])
    .unwrap_or_default();
  let decorations = InputTokens::new(&raw, tokens)?;
  Ok(TextInput::new(source).tokens(decorations).multiline())
}
```

Set those roles in the application's theme. A token style can use a different fill/text role for an
unavailable variable; validation and the variable parser remain application responsibilities.
Padding is horizontal in logical pixels. The line pitch is the largest participating text style's
line height; each wrapped token fragment has its own rounded background. Center/right alignment,
clipping, scrolling, selection, pointer hits and caret placement use the same measured raw-source rows.

Ranges must be nonempty, nonoverlapping UTF-8 byte ranges and cannot contain CR or LF. The constructor
sorts them and returns typed errors. Literal CR/LF text around inline tokens remains editable, including
CRLF. Tokens are editable ranges, not atomic words: a click can place the caret inside `{{task.title}}`,
and typing/deletion can make it a partial or invalid expression. Existing ranges stop decorating immediately
when the source changes. Reparse on the next render; omitted/invalid expressions remain ordinary text.

Copy, cut, paste, keyboard navigation, undo/redo and persistence continue to use the raw source bytes.
No replacement label enters clipboard or storage. Decorations are disabled during IME preedit and for
masked inputs. Debug output of `InputTokens` omits source and ranges. The controller never returns
secret source content, and styling a masked field cannot reveal its token glyphs.

This is a decorated plain input, not a rich-text document model. Run shaping uses the existing glyph
engine; token boundaries are explicit shaping boundaries. Paragraph-wide mixed-direction reordering
across token boundaries has not been validated. Platform-native IME and visual parity need separate
native verification; headless editing/layout tests do not establish that result.
