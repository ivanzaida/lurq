# Pictures inside a rich text document: handover

For the session that builds pictures inside `ui::RichTextEditor`'s document: a picture that is a character of the
text, that the caret goes past, that is selected, deleted, undone, copied and pasted with the words round it, and
that is shown by `ui::Picture`. Ivan, 2026-11-16: done properly, in the engine, not as blocks between separate inputs.

Written from a reading of `feat/design-system` at `7c5df75b`. Line numbers move: search for the names.

## Where to work

- Branch `feat/design-system` (draft PR 120). Cut a branch and a worktree of your own from it; the coordinating
  session merges it. Do not push, do not merge into `feat/design-system` yourself.
- Read first, whole: `design/component-playbook.md`, the sections `## Rich Text Editor` and `## Text Editing` of
  `docs/src/content/docs/design-system.md`, `docs/src/content/docs/text-input-styled.md`,
  `docs/src/content/docs/text-input-tokens.md`.
- The Pencil board, read only, through the Pencil MCP, uri `file:///H:/projects/lurq/pencil/lurq-design-system.pen`
  in every call (never a worktree's copy, never Read or Grep a `.pen` file): the Rich Text Editor is `KEQoV` in
  `Components / light`, section Forms & Fields; find its dark twin and every instance. If the board draws a picture
  in the document (its size, a selected picture's ring, a caption, handles), that is the look; if it draws none, the
  decisions are yours and the report lists them.

## What exists, and what it does not do

| Piece | Where | What it does today |
| --- | --- | --- |
| The editor | `crates/lurq/src/ui/rich_text_editor.rs` | One styled `TextInput` for the whole document, a toolbar, keys, Markdown shortcuts, `RichTextController` (`insert_text`, `insert_link`, `toggle_mark`, `set_paragraphs`, `undo`, `redo`). The image tool only calls the app's `on_image` (about line 546); the docs say a picture is the app's to place outside the document. |
| The value | `crates/lurq/src/components/styled_text/rich_text.rs` | `RichText { text, spans, paragraphs }`, `from_markdown`, `to_markdown`, `blocks`, `reads_as_markdown`. A Markdown image is read as its alt text linked to its source (`MarkdownInline::Image`, about line 315): the picture is lost. |
| Styled spans and kinds of paragraphs | `components/styled_text.rs`, `styled_text/paragraphs.rs`, `styled_text/ops.rs` | `StyledSpan`, `SpanStyle`, `ParagraphKind`, `ParagraphStyles`; how an edit moves spans and kinds. |
| The input's state | `crates/lurq/src/node/node_kind/text_input/` | `mod.rs` (the state; undo counts about line 516), `editing.rs` (edits; `undo` about line 341; replacing a selection with styled text about line 31), `styled.rs` (spans kept in every undo snapshot, about line 79), `tokens.rs` (tokens, the atomic ones, hit tests on a token), `token_affinity.rs`, `geometry.rs`. |
| Tokens | `crates/lurq/src/components/text_input_tokens.rs` | `InputToken { range, style, atomic }` and `InputTokenStyle` (padding, height, border, icon, looks). An **atomic** token is already one thing to the caret: the caret never rests in it, the arrows go past it whole, a press selects it all, Backspace after it selects it and the next deletes it. Released in 0.62.0, `atomic` with it. |
| Laying the input out | `crates/lurq/src/layout/layout_engine/input_layout.rs` (about line 16) | **One path or the other**: a styled input (`styled_input_styles`, behind `markdown`) is laid out from its spans and kinds, and returns before the app's tokens are looked at; an input with tokens and no styles goes through `layout_input_tokens`. A styled input has no tokens, so no atomic ranges. |
| The decorated layout | `layout_engine/styled_input_layout.rs`, `layout_engine/input_token_layout.rs` (`layout_decorated_input`, `token_style`), `app/glyph_engine/input_tokens/` (`mod.rs`: `TokenStyle`, `TokenChip`; `rows.rs`, `pieces.rs`, `geometry.rs`, `paint.rs`, `paragraphs.rs`) | Shapes a whole paragraph, gives every range a `TokenStyle` (its text style, a padding, a fill, lines, a chip with its own end padding and height), and gives back the carets, the rows and the paint. A chip's room is its text's width plus paddings: there is no range of a width and a height of its own with no glyph. |
| An element at a byte | `TextInput::anchor_at(anchor, byte)` in `text_input_tokens.rs` (about line 190) | Puts an `ElementRef` of no size where the caret before a byte stands, as the layout lays the text out: an overlay can hang from it (a mention's list does). |
| Child nodes placed on a text's boxes (read only) | `crates/lurq/src/layout/layout_engine/inline_links.rs`, `node/inline_link.rs`, `NodeKind::InlineLink` | A read-only rich text paragraph has its links as **child nodes**, placed after layout on the boxes of their spans (`SpanBox` from `app/glyph_engine/span_boxes.rs`), painted and hit there, following the wrap. This is the precedent for an input whose pictures are child nodes. |
| Pictures | `crates/lurq/src/ui/picture.rs`, `lurq::resources` (`Source`, the app's `Loader`), `Element::component::<C>(props)` | A picture from a file, bytes, a resource or an address, with its loading and error states; a declared component keeps its live nodes when its parent renders again. |
| Read-only Markdown with images | `crates/lurq/src/components/markdown/mod.rs` (`render_inline_flow`, about line 692) | Not an inline box either: a paragraph with an image is broken into a wrapping row of texts and images. |
| The clipboard | `TextInput::copy_format` (`components/text_input.rs` about line 262), the editor's copy | A copy of formatted text puts its Markdown on the OS clipboard as plain text, and a paste of Markdown into the editor restores kinds and marks. Text only. |

So the gap is four things, all in the editable input: a range that is a box of a given size and has no glyph; that
range being atomic in a **styled** input; the input holding child nodes placed on those boxes; and the value, undo
and the clipboard carrying the picture.

## What to build

1. **The value.** A picture is one character of `text`, U+FFFC (object replacement), and a record beside it:
   `RichText` gains `objects` (or `pictures`): for each, its byte, its source (what `ui::Picture` takes), its label
   (alt text) and its size as the document holds it (a width, or none for the line's width; the ratio is the
   picture's). `from_markdown` reads `![label](source)` into one; `to_markdown` writes it back; `plain()` gives the
   label in its place; `blocks()` and `word_count` say what they do with it. A picture alone in its paragraph and a
   picture among words are both kept as written. Unit tests of the round trip first: they need no engine.
2. **The engine: a range that is a box.** In `app/glyph_engine/input_tokens`, a `TokenStyle` (or its chip) can be a
   box of a width and a height with no glyph of its own: the row it is in is as high as the tallest box in it
   (check `rows.rs`), the box stands on the baseline (or on what the board draws), the caret beside it is as high
   as the text's line, not the box, and a box wider than the row is made as wide as the row, keeping its ratio.
   Selection paints over a selected box (a veil, or the board's ring). Tests in `tests/input/` beside the token
   tests: the carets either side of a box, a row's height, wrapping a box to the next row, a box wider than the
   room, hit testing on it.
3. **Atomic ranges in a styled input.** `input_layout.rs` must lay out a styled input's spans **and** its objects
   (or any app token) together, and the state's atomic logic (`tokens.rs`, `token_affinity.rs`, the caret's
   movement, Backspace and Delete, a press, a selection) must see the objects of a styled input as it sees an
   atomic token. Keep a plain input with tokens exactly as it is (0.62.0 released it): `tests/input/` whole must
   still pass. A styled input's edits already move spans and kinds (`styled_text/ops.rs`); they move the objects'
   bytes too, and an edit that deletes U+FFFC deletes its record.
4. **The input holds child nodes.** As a read-only paragraph holds its links: the `TextInput` node has a child per
   object, laid out at the object's box and placed on it after the input's layout, moved with the input's scroll,
   cut by its clip, painted in order with the text and the selection. The child is the app's element for the
   object; the editor gives a `ui::Picture` declared with `Element::component` so that it keeps its load across
   renders. A press on a picture selects it in the text (it does not reach the picture's own handlers unless the
   picture is selected and the board says something about a selected picture's bar). Read `inline_links.rs` before
   designing this, and make what you build serve any element, not pictures alone (a mention's chip or a formula
   could use it later). Tests in `tests/runtime/` or `tests/input/`: a child placed where its box is after an edit
   before it, after a scroll, after a resize that wraps the row; a child of a deleted object gone; hit testing.
5. **Undo, redo and the clipboard.** The styled snapshot of the undo history (`text_input/styled.rs`) keeps the
   objects with the spans, so that undoing a deletion brings a picture back with its source and size. A copy of a
   selection with a picture writes its Markdown `![label](source)` (the clipboard stays text); a paste of such
   Markdown into the editor restores it; a paste into a plain input is the text as it is. An image on the OS
   clipboard (a bitmap) is not part of this unless the engine reads one already: say what it does.
6. **The editor.** `RichTextController::insert_picture(source, label)` (and a size if the board has one) at the
   caret, replacing the selection; the image tool calls `on_image`, and an app answers by calling the controller
   (a doc example opens `lurq::files::FileDialog` and inserts the file chosen). A picture's loading and broken
   states are `ui::Picture`'s. Remove the sentence of the docs and the rustdoc that says a document holds text
   only. The gallery's `/rich-text-editor` page shows a document with a picture in a paragraph of its own and one
   among words, and the playground can insert one.

Order: 1, then 2 and 3, then 4, then 5 and 6. Commit at each step with its tests, so that a stop leaves a whole
tree.

## Rules

- **Releases.** `InputToken`, `InputTokenStyle`, `atomic` and `styled_text.rs` exist on `master` (0.62.0): a change
  that breaks a program written against it (`git show master:<path>` to check) goes under `Breaking:` in
  `CHANGELOG.md` and into `docs/src/content/docs/migration-v0-63.md` in the same commit. `components::RichText`,
  `ui::RichTextEditor` and paragraph kinds only ever lived on this branch: changing them is not a break.
- The engine's clock is `crate::time::Instant`, never `std`'s (the browser build panics on it).
- Tests do not read the machine's clock: `Tree::set_animation_clock`, `Tree::set_press_clock`,
  `Tree::set_timer_clock` (`Stage::presses_at`, `Stage::own_timers`).
- **No real pointer or keyboard input, ever** (Ivan is at the machine): a gallery of your own, copied to
  `target/debug/gallery-<your name>.exe`, started with `MSYS_NO_PATHCONV=1` from Git Bash, driven only through its
  MCP server (`H:\projects\lurq\.claude\worktrees\design-system\target\qa\q.py` and `qa.py`: copy them, set `q.LOG`
  to your log), and closed by its own name in the same script. **Never send Ctrl+C or Ctrl+X through MCP: it writes
  Ivan's real clipboard.** Use the MCP `paste` action, and the tests, for the clipboard.
- Do not build `--release`. If a build fails for space, delete your own `target/debug/incremental`.

## Checks before each commit

- `cargo check -p lurq`; `cargo check -p lurq --features form,router --tests`;
  `cargo check -p lurq --features form,router,canvas,markdown --tests`; `cargo check -p gallery --features devtools,mcp`;
  `cargo check -p gallery --target wasm32-unknown-unknown`; `cargo check -p lurq --target aarch64-apple-darwin`.
- Whole suites: `cargo test -p lurq --features form,router,mcp,canvas,markdown --test ui_tests`; with
  `--features form,router,canvas,markdown`: `--test input_tests`, `--test layout_tests`, `--test runtime_tests`,
  `--lib`; `--features form,router,markdown --test markdown_tests`; `cargo test -p gallery` (the guard of a phone's
  width over every page).
- `rustfmt --edition 2024 <the files you changed>`, then `git status`: rustfmt follows `mod` lines into files you
  did not touch; `git checkout --` those.

## Done

The editor holds pictures as the board draws them (or as you decided and listed); every step above has its tests;
docs in `design-system.md` (Rich Text Editor), `text-input-styled.md` and `text-input-tokens.md`; the changelog;
a look in your own gallery at inserting, selecting, deleting, undoing and pasting a picture, in dark and light. The
report says what an app writes, what the engine gained, what you decided where the board is silent, what differs
from the board, and "Not done" empty or exactly what and why. An earlier estimate put this at two thousand lines or
so over four parts of the engine; if a part turns out larger, say so before going on with it.
