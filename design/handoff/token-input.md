# Token Input and Mentions: hand-off

`ui::TokenInput` and `ui::Mentions`, from the board's `Token Input` (`tEIxy`) and `Mentions` (`O8zhK8`) in Components /
light (`Ovt1D`), and where they are used: the token input in Inputs, "Multi-line & tokens" (`K195w8`, beside the
textarea), and the mentions in Selection (`ReJPy`, beside the multi select: "Thanks for the review @iv" in a focused
editor of 280 by 64, and under it a list of 220 of three people, Ivan Zaida highlighted). The Examples frame (`ZB05g`)
has no composer or prompt box that uses either: nothing there has a token or a mention in its text, so nothing was
read from it. The board draws no token under the pointer or selected, no mention once chosen, no token input on
several rows and no sizes; those were built in the manner of what it does draw (see Differs from the board).

Files: `crates/lurq/src/ui/token_input.rs`, `crates/lurq/src/ui/mentions.rs`, `crates/lurq/tests/ui/token_input.rs`
(both components), `crates/gallery/src/pages/token_input.rs` and `mentions.rs` (pages "Token Input" and "Mentions",
group Inputs), `design/handoff/token-input.md`, and lines in the three `mod.rs` files.

## Changed outside the component's own files

This session owns the engine's tokens in a text input (`crates/lurq/src/components/text_input_tokens.rs`), and every
change of the engine below is of that kind, with one small accessor on the text input's controller.

### What the engine's tokens were, before this batch

What `lurq::ui` builds on, and does not write again:

- **What a token is**: `InputTokens::new(source, Vec<InputToken>)` is a validated list of decorations of one exact
  string: each `InputToken` a nonempty UTF-8 byte range of it, none overlapping, none holding CR or LF (typed errors
  otherwise), with an `InputTokenStyle`. `TextInput::tokens(tokens)` hands it to the input. The value stays a plain
  `String`: a token is a range of the text, not an object in it, and no label replaces its bytes.
- **How it is inserted**: it is not, by the engine. The app parses its own text in every render and passes the ranges
  for the exact current value; the input decorates them only while its value is that source, and an edit makes the
  old list stop decorating at once until the next render passes a new one. Text is put at the caret by a
  `TextInputController` (`replace_selection`, one undo step through `on_input`), which a picker uses.
- **Deleted, selected, copied**: as text. The caret goes into a token, a click places it inside, typing or deleting
  makes it partial, and the app's parser then says what it is. Copy, cut, paste, undo and persistence see the raw
  bytes. During an IME composition and in a masked input tokens are not drawn.
- **Its look**: `InputTokenStyle::new(foreground, background)` with `padding` (one horizontal value at both ends,
  reserved as measured advance on every visual fragment), `radius` and `typography`; colors as palette roles or
  colors, padding and radius as theme steps or numbers. A token is drawn as a rounded fill per visual fragment (a
  wrapped token has one per row), as high as its type's line, with its glyphs in the foreground, shaped with the
  whole paragraph. No edge, no icon, no height of its own, no look under the pointer or in a selection.
- **What tooling and a form read**: the input's value, the plain text, tokens and all; nothing of the tokens.

### What changed in the engine

- `crates/lurq/src/components/text_input_tokens.rs`:
  - `InputTokenStyle` takes what a component resolves from its recipe, all of it optional and none of it changing a
    style that does not set it: `padding_end(..)` (the room after the text, apart from `padding`'s before it),
    `height(f32)` (the fill's own height, centred where the line's would be), `border(width, color)` (a line inside
    the fill), `text_style(TextStyle)` (the token's whole type in place of a typography role: family, size, weight,
    line height; its color is still the foreground), `icon(InputTokenIcon)`, `hovered(InputTokenStyle)` and
    `selected(InputTokenStyle)` (the looks it takes under the pointer and while the selection covers all of it).
  - `InputTokenIcon::new(IconGlyph)` with `size`, `color`, `gap` and `removes(bool)`: a glyph at the end of the token,
    after its text, in room its end padding keeps for it, drawn at the end of its last fragment. A click on an icon that
    `removes` takes the token away: its text, with the space after it where it starts the text or follows a space, is
    deleted as an edit through `on_input`, one step of the undo.
  - `InputToken::atomic(bool)` (default `false`, so every existing use is as it was): an atomic token is one thing to the
    caret. The caret never rests inside it (Left, Right, word moves, Up and Down go to its edges, the way they move);
    a press or a click on it, or a double click, selects all of it; a drag that takes part of it takes all of it;
    Backspace with the caret at its end (Delete at its start), nothing selected, selects it, and the next deletes the
    selection. Its text is still the value's.
  - `InputTokenIcon` is exported from `lurq::components` beside the others.
- `crates/lurq/src/app/glyph_engine/input_tokens/mod.rs`, `geometry.rs`, `rows.rs`, `rows/padding.rs`: the glyph
  engine's `TokenStyle` has a `chip: TokenChip` (end padding, height, edge, icon; all empty by default) and
  `padding_end()`; the padding of a row and the cost a row break counts take the end's own padding; a fragment knows
  the token it fills (`InputFragment::token`, with the edge), and a fill of its own height is centred on the line. The
  internal tests that write `TokenStyle` out field by field (`crates/lurq/tests/input/text_input/chunked_shaping.rs`,
  `crlf_geometry.rs`, `rtl_geometry.rs`, `scroll_geometry.rs`, `source.rs`, `token_paint.rs`) and
  `layout/layout_engine/styled_input_layout.rs` add `chip: Default::default()`, nothing else.
- `crates/lurq/src/layout/layout_engine/input_token_layout.rs`: a token's style is resolved by `token_style`, from the
  look the token is in (selected over hovered over its own); `input_token_quads.rs` paints a token's edge as an inside
  border of its fill and its icon as a text quad at the end of its last fragment.
- `crates/lurq/src/node/node_kind/text_input/tokens.rs`, `editing.rs`, `mod.rs`: the token under a point
  (`token_at_point`, from the painted fragments), the hovered token (`hover_token_at_point`, kept only where its style
  has a hovered look, and drawn again on a change), `selection_bounds`, keeping the caret and the anchor out of atomic
  tokens (`keep_out_of_tokens`, called at the end of every caret move and pointer selection), selecting an atomic token
  beside the caret before Backspace and Delete, and removing a token by its icon. `TextInputInner` has `hovered_token`.
- `crates/lurq/src/app/runtime.rs`: a pointer move tells the text input under it where the pointer is in its text (the
  same coordinates a press uses), and the input it was over before that it left; leaving the window tells it too. One
  field, `token_hover`. Over the icon of a token that a click takes away, the cursor is the pointer
  (`hovered_cursor`), as over a button; elsewhere in the input it stays the input's.
- `crates/lurq/src/components/text_input_controller.rs` and `node_kind/text_input/controller_access.rs`:
  `TextInputController::caret_point(byte) -> InputCaretPoint { x, y, height }`, where the caret before a byte stood
  when the input was last laid out, from the start of its text's box; `InputCaretPoint` is exported. This is no area
  another session owns (it neither masks, filters what is typed, steps a value nor moves the focus), and it is a read
  of what layout already keeps.
- An anchor at a byte, set as the input is laid out: `TextInput::anchor_at(element_ref, byte)` (in
  `text_input_tokens.rs`, kept in `TextInputInner::byte_anchor`) has the layout publish `element_ref` at the place of
  the caret before `byte`, one wide and as high as the text's band, inside the padding and past the scroll.
  `LayoutEngine::index_input_anchor` (`input_token_quads.rs`, called from `collect_overlay_index_recursive` in
  `layout_engine.rs` beside `index_node_layout`) pushes it into the overlay index where every element's rectangle is
  published, and updates the ref's own bounds; an overlay placed in that pass hangs from it. Nothing of overlay
  placement changed. `Mentions` hangs its list from it, so the list stands at the trigger at the first paint after a
  paste or a value the app sets, as after a key.
- `node_kind/text_input/geometry.rs`: the input keeps the hovered token across a render, as it keeps the caret, until
  the pointer moves.
- `node_kind/text_input/editing.rs`: a press on a token's removing icon selects nothing, so its release is a click
  (a release that leaves a selection is the end of a drag and makes no click).
- What `Mentions` would share with the Combobox session's list: its list is a `Popup` that takes no focus, with rows of
  its own (36 high, an avatar or icon and a label, the active one on `Highlight`), because the keys stay in the text
  and a row must not take the focus. A list filtered by typing that never takes the focus from the input it hangs from,
  with rows that have a leading avatar, is what both have; if the combobox mode of the engine's select can hang its
  list from an `ElementRef` of the app's (here a mark at the trigger) and take a leading element per row, `Mentions`
  could use it in place of its own rows.
- `docs/src/content/docs/text-input-tokens.md` says tokens "are editable ranges, not atomic words" and lists only
  padding, fill, radius and typography. Whoever integrates the batch replaces the paragraph that starts "Ranges must be
  nonempty" from its fourth sentence ("Tokens are editable ranges ...") with: "By default tokens are editable ranges,
  not atomic words: a click can place the caret inside `{{task.title}}`, and typing/deletion can make it a partial or
  invalid expression. `InputToken::atomic(true)` makes one a single thing to the caret: arrows go past it whole, a click
  selects it, a drag takes all of it, and Backspace after it (Delete before it) selects it so that the next deletes it.
  Existing ranges stop decorating immediately when the source changes. Reparse on the next render; omitted/invalid
  expressions remain ordinary text." And after the paragraph on padding: "A style can also give the end its own padding
  (`padding_end`), the fill its own height (`height`, centred on the line; give the type a line height that leaves
  room), a line inside the fill (`border`), the whole type (`text_style`), an icon after the text
  (`InputTokenIcon`, whose `removes(true)` makes a click on it delete the token as one undoable edit through
  `on_input`), and the looks it takes under the pointer (`hovered`) and while the selection covers it (`selected`).
  `TextInputController::caret_point(byte)` says where a byte stood in the last layout, and `TextInput::anchor_at(element,
  byte)` has the layout put an element at a byte as it lays the input out, for an overlay hung from the text."

## For `docs/src/content/docs/design-system.md`

## Token Input

A box of text in which what the app recognizes stands as a token: tags, recipients, the variables of a template.

```rust
use lurq::ui::{TagVariant, Token, TokenInput};

// Tags: every word with a space after it is a token.
TokenInput::new(tags.clone())
  .tokens(|text| Token::separated(text, ' '))
  .placeholder("Add tag");

// A template on rows, its variables tokens with no cross.
TokenInput::new(template.clone())
  .rows(2, 6)
  .tokens(|text| {
    text
      .match_indices("{{name}}")
      .map(|(at, name)| Token::new(at..at + name.len()).variant(TagVariant::Info).removable(false))
      .collect()
  });
```

| Builder | |
| --- | --- |
| `new(Signal<String>)` | The text, tokens and all; what is typed is set on it, and what the app sets on it is shown |
| `tokens(impl Fn(&str) -> Vec<Token>)` | How the app finds its tokens in the text, called in every render. Tokens that overlap one before them, hold a line break or are not on characters are left out |
| `Token::new(range)` | A token over a byte range of the text: atomic and removable by default |
| `Token::variant(name)` | A `TagVariant` (`Neutral`, `Info`, `Success`, `Warning`, `Destructive`), `MENTION`, or a variant the theme's recipe names |
| `Token::removable(bool)` | The cross at its end, which a click takes it away by |
| `Token::atomic(bool)` | One thing to the caret (the default), or text drawn as a token that the caret goes into |
| `Token::separated(text, char)` | The items of a list typed with a separator: each item with a separator after it |
| `rows(min, max)` | Text on several rows, between the two counts, tokens wrapping with the words; one line without it |
| `placeholder(&str)` | What it says while it is empty |
| `size(TokenInputSize)` | `Sm` 32, `Md` 36 (the default), `Lg` 40 high, with tokens of 20, 24 and 28 |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | The edge in the destructive, warning or success color, as an input's |
| `read_only(bool)` | Focused, selected and copied, not changed; its tokens have no cross |
| `full_width(bool)` | As wide as what holds it, in place of 260 |
| `on_input(handler)` | Called before an edit, taking a token away by its cross too; `prevent_default()` refuses it |
| `controller(TextInputController)` | Kept by the app: reads the selection and puts text at it, a token chosen from a picker |
| `label(&str)`, `name(..)`, `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As an [input](#input) has them. `id` is the id of the control |

- It is the engine's `TextInput` with `TextInput::tokens`: what it holds is text, and a token is a range of that text
  the app's function finds in every render, so an edit that changes a token's text makes it what the function then
  says of it. Copying, pasting, undo, a form and tooling see the text: to what reads the tree it is a `textbox` whose
  value is the text, each token as its own words.
- A token is atomic: Left and Right go past it whole, a click selects all of it, Backspace after it selects it and the
  next deletes it. A click on its cross deletes it with the space after it, as one edit `on_input` hears and undo
  brings back. With `Token::separated`, deleting the space after the last token makes it text again, to be edited.
- The box is an input's (one line) or, with `rows`, a textarea's: the same sizes, judgments, read only and disabled,
  and a [field](#field) names and judges it as it does an input.
- Parts: `TokenInputPart::Root` (its width), `Control` (the box: edge, fill, height, padding, the type of the text),
  `Placeholder`, `Token` (fill, edge, radius, height, the room before the text and after the cross, and the gap
  between them), `TokenLabel` (the token's type and color), `TokenIcon` (the cross: its side and color). States:
  `TokenInputState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`, `Multiline`, and a token's own
  `TokenHovered` and `TokenSelected`. A token's parts are resolved with its variant and its state as the case, so a
  recipe styles a variant's tokens with `.variant(..)` and their looks with `.state(TokenInputState::TokenHovered, ..)`.
- In the design the box is 260 by 36, an input's, padded 6 at its sides. A token is 24 high, `Secondary`, `radius-sm`,
  8 before its text and 6 after its cross; its text is 13 in `SecondaryForeground` and its cross 12 in
  `MutedForeground`, 4 after the text. A colored variant is in the muted fill of its color with its text and cross in
  the color, as a tag is. Under the pointer a token is `SecondaryHover` (a colored one keeps its fill) and its cross
  `Foreground`; covered by the selection it is `Primary` with its text and cross in `PrimaryForeground`. On rows the
  box is padded 6 down too and its lines are 28 high, so a token has room around it.
- A renderer is given a `TokenInputView`: `view.tokens` (what was found), `view.control()` (the engine's input with
  the tokens drawn in the recipe's style), `view.token_style(recipe, cross)` (how the engine draws a token of a
  resolved case), `view.root(control)`.

## Mentions

Text in which a trigger opens a list of what can be mentioned, and what is chosen stands as a token: a comment, a
message, a prompt.

```rust
use lurq::ui::{Avatar, Mention, Mentions, MentionsController, icons};

// Kept by the app, with the value, from one render to the next.
let mentions = MentionsController::new();

Mentions::new(comment.clone(), &mentions)
  .trigger('@', vec![
    Mention::new("Ivan Zaida").avatar(Avatar::new("Ivan Zaida")),
    Mention::new("Ivy Chen").avatar(Avatar::new("Ivy Chen")),
  ])
  .trigger('#', |typed: &str| teams_starting_with(typed))
  .on_mention(|text| println!("mentioned {text}"))
  .placeholder("Write a comment");
```

| Builder | |
| --- | --- |
| `new(Signal<String>, &MentionsController)` | The text, and what the field keeps between renders: the app keeps both |
| `trigger(char, options)` | What a character at the start of a word offers: a `Vec<Mention>`, narrowed to the offers whose label or text has what is typed after it (without regard to case), or a function of what is typed. At most 8 are shown |
| `Mention::new(label)` | An offer: `text(&str)` is what it puts in the field (the trigger and the label by default), `avatar(Avatar)` or `icon(name)` before its label |
| `on_mention(handler)` | Called with the text of an offer as it is chosen |
| `tokens(..)` | Other tokens of the app's in the text, as a [token input](#token-input) finds them |
| `rows(min, max)` | 2 and 8 unless told |
| `full_width(bool)` | As wide as what holds it, in place of 280 |
| `input_rules(&recipe)` | Rules laid over the token input it is |
| `placeholder`, `size`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `name`, `on_input`, `state`, `style`, `rules`, `force`, `id`, `key` | As a token input has them |

- Typing a trigger at the start of the text or after a space opens the list under it while it offers something. Up and
  Down move the offer that Enter (or Tab) chooses, a click chooses one, the pointer moves the active one, and Escape
  puts the list away while the caret stays in that word, so that Enter is a new line again. The keys stay in the
  field the whole time: the list takes no focus, and a press on it does not blur the field.
- Choosing puts the offer's text and a space in place of the trigger and what was typed, as one edit through
  `on_input`. From then on that text is a mention wherever it stands as a word of its own: a token of the variant
  `MENTION`, atomic, with no cross. `MentionsController::mention(text)` makes a text a mention without choosing it
  (what a message loaded with mentions has), and `mentioned()` lists them.
- `MentionsController` holds the field's `TextInputController` (`input()`), the active offer, the trigger Escape put
  away and the mentions; a builder has no `Ctx`, and these must outlive a render, as the engine's controller does.
- It is a [token input](#token-input) on rows: to tooling a `textbox` whose value is the text with each mention as the
  offer's text, with `a11y_haspopup` `listbox` and `a11y_expanded`; the list is a `listbox` of `option`s, the active
  one `a11y_selected`.
- Parts: `MentionsPart::Root`, `Control` (laid over the token input's control: its padding, line and least height),
  `List`, `Option`, `OptionAvatar`, `OptionIcon`, `OptionLabel`; the mentions themselves are the token input's parts in
  the variant `MENTION`. States: `MentionsState::Open`, `Active` (an offer's own).
- In the design the box is 280 wide and at least 64 high, padded 12 across and 8 down, its text 14 with lines of 1.5.
  The list hangs 4 under the trigger: 220 wide, `Popover`, `radius-lg`, an edge of 1 in `Border`, the medium shadow,
  padded 4. An offer is 36 high, padded 8, `radius-sm`, an avatar of 24 (or an icon of 16 in `MutedForeground`) 8
  before its label in the body's type; the active one, and the one under the pointer, are on `Highlight`. A mention is
  in the text's type on `InfoMuted`, its text in `Info`, 20 high and padded 4.

## For `CHANGELOG.md`

- `ui::TokenInput`: a box of text, on one line or on rows, in which the tokens the app finds stand as chips in the colors of a tag, with a cross that takes one away as an undoable edit; atomic to the caret and to Backspace, and a `textbox` of the plain text to tooling. The engine's input tokens have what it needs: `InputTokenStyle::padding_end`, `height`, `border`, `text_style`, `icon(InputTokenIcon)`, `hovered` and `selected`, `InputToken::atomic`, `TextInput::anchor_at` (an element the layout puts at a byte, for an overlay) and `TextInputController::caret_point`.
- `ui::Mentions` and `MentionsController`: a token input in which a trigger (`@`, `#`, `/`) opens a list of `Mention`s under it, narrowed to what is typed or found by a function, chosen with Up, Down and Enter or a click and put away with Escape; what is chosen stands as an atomic mention token.

## Decided

- **A token is a range of the text, as the engine's token is.** The board's token input could be read as a list of tags
  beside a box for the next one; the brief binds it to the engine's tokens, and a mention has to stand in the middle of
  prose, so both are text with tokens in it. What a token input of tags holds is "rust wgpu ", which a form sends and
  tooling reads as it is; `Token::separated` makes each item with a separator after it a token, so the item being typed
  is text until its separator.
- **The look of a token is resolved from the recipe in the theme it is built in and given to the engine as values**
  (`token_style`), with one resolve for each variant and each of rest, hovered and selected, kept for every token of
  that variant. The engine takes a whole `TextStyle` for the token's type, so weight and family reach it too.
- **Tokens are atomic by default** in the design system (the engine's default stays editable): a chip is one thing.
  Backspace selects before it deletes, so that a person sees what goes; a removable token has its cross for the
  pointer. A cross takes the space after its token too, so taking a word from a list leaves one space.
- **Hovered and selected are looks of a token, not states of the box**, resolved as cases of the recipe with
  `TokenHovered` and `TokenSelected`; the engine picks the look as it lays out, from where the pointer is and what the
  selection covers. A token under the pointer only lights where its style has a hovered look, so an engine user who
  sets none pays nothing.
- **The selected token is `Primary`**, the color of what is chosen elsewhere in the system; the selection's own band
  is drawn over it as over any text.
- **Mentions keeps what must outlive a render in a `MentionsController` the app holds**, as the engine asks an app to
  hold a `TextInputController`: the caret's place is read from the input's controller, the active offer and the
  dismissed trigger are kept in it, and a signal in it renders the field again when a key moves the offer or the caret
  without changing the text. A mounted component would have needed a `Ctx` where it is built, and `FieldControl`
  builds without one.
- **The list hangs from the trigger, not the caret**, so that it stays still while the query is typed. Its anchor is an
  element the input's layout puts at the trigger's byte (`TextInput::anchor_at`).
- **The list's rows are the mentions' own**, in a recipe that has a menu item's values (36 high as the board draws,
  padding 8, gap 8, `radius-sm`, `Highlight`): `ui::MenuItem` is a focusable button that would take the focus from
  the text, and has no avatar.
- **A mention is a token of the variant `MENTION` in the token input's recipe**, not a part of the mentions', so a
  theme styles it where it styles the other tokens. It is `InfoMuted` with `Info` text: the board draws no chosen
  mention, and the informative color is what a reference reads as.
- **A static list is narrowed by "contains", without regard to case**, on the label and on the text, and shows 8 at
  most (`MENTIONS_LIMIT`); a function is given what is typed and its offers are shown as it gives them.

## Not done

Nothing.

## Differs from the board

- **The room between tokens** is the width of the space between them in the text (about 4 at 14), where the board's
  row has a gap of 6: a token is a range of the text and what separates two is text.
- **What the board does not draw** was built in the manner of what it does: a token under the pointer, a selected
  token, a chosen mention, the token input on rows (its lines 28 high, padded 6 down), the sizes (an input's heights
  with tag heights for the tokens: 20, 24, 28, text 12, 13, 14, crosses 10, 12, 16), and a colored token (a tag's
  muted fills).
- **The mentions box is a token input on rows**: at least 64 high as the board draws it, and it grows with its text to
  8 rows, where the board's editor is a fixed 64.
- **The list hangs 4 under the trigger**, where the board's hangs 4 under the editor at its left: the brief puts it
  under the caret, and an editor of several rows would leave it far from the word.
