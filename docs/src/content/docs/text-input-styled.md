---
title: Styled text inputs
description: Bold, italic, underline, inline code, strikethrough and links edited in place in a TextInput, with headings, lists, quotes and code blocks as kinds of its paragraphs, and pictures as characters of its text.
---

A `TextInput` bound to a spans signal edits styled text: the value stays the plain `Signal<String>`, and a
`Signal<Vec<StyledSpan>>` holds the styled runs over it as UTF-8 byte ranges. The input draws each run with the
theme's text marks, moves the spans with every edit, keeps them in its undo history and writes them back. The app
owns the model: it loads one by setting both signals and hears every change the input makes. A rich editor (a
block editor that stores Markdown, for example) builds on this, and the design system's
[`RichTextEditor`](../design-system/#rich-text-editor) is one. The input itself parses and writes no Markdown; a
[`RichText`](#one-value-richtext) reads a document from Markdown and writes it.

Styled inputs come with the `markdown` feature. Without it none of this API exists and a `TextInput` does no span
work at all.

## Binding the spans

```rust
use lurq::{
  components::{SpanStyle, StyledChange, StyledEdit, StyledSpan, TextInput, TextInputController},
  core::Signal,
};

fn paragraph(value: Signal<String>, spans: Signal<Vec<StyledSpan>>, controller: TextInputController) -> TextInput {
  TextInput::new(value)
    .spans(spans)
    .controller(controller)
    .multiline()
    .on_styled_change(|change: &StyledChange| {
      // What changed, and the spans the input now holds.
      if let StyledEdit::Text { replaced, .. } = change.edit() {
        let _ = (replaced, change.inserted_text());
      }
      let _ = change.spans();
    })
}

let value = Signal::new("Read the docs".to_owned());
let spans = Signal::new(vec![
  StyledSpan::new(0..4, SpanStyle::new().bold()),
  StyledSpan::new(9..13, SpanStyle::new().link("https://example.com")),
]);
```

A `SpanStyle` has `bold`, `italic`, `underline`, `code`, `strikethrough` and an optional `link` target. Spans the input reports
are canonical: sorted, disjoint, styled only (plain text has no span), never splitting a character, with equal
neighbours merged. `StyledText::new` puts any spans in that form, and `StyledText::runs` walks the whole text as
runs of one style, plain runs included, which is the walk a Markdown writer needs.

`on_styled_change` runs after every change the input makes with a `StyledChange`: the edit (`StyledEdit::Text`
with the replaced range and the inserted length, `Style` with the restyled range, or `Replaced` for undo and redo),
the spans the input now holds, shared rather than copied, and the text on demand (`with_text`, `inserted_text`,
or `to_styled_text` for an owned copy). Nothing of the text is copied unless the handler asks, so an app can keep
its own model in step with an edit at a time.

To load a model, set the value and the spans signals; the input lays out again and does not report the write. A
spans signal that does not match the value is clamped to it.

## Typing and editing

Caret moves, selection by pointer and keyboard, word and line moves, double and triple click, deleting and input
method composition work as in a plain input; styles do not change them. Typed text takes the style of the text
before the caret, with two exceptions: a link does not grow past its end, and at the start of a line the character
after the caret lends its marks (not its link). Typing over a selection takes the style of its first character. A
composition is drawn in the style it will commit in.

## Styling commands

The controller carries what a toolbar or a shortcut calls; each is one undo step with the text edits around it:

- `toggle_mark(TextMark::Bold)` turns a mark on over the selection unless all of it has it, then off. With nothing
  selected it toggles the mark of the next typed text, which is how Ctrl+B works while typing. `set_mark` sets it.
- `set_link(Some(target))` links the selection; `set_link(None)` unlinks it. With nothing selected it changes the
  whole link the caret is in.
- `set_pending_style(Some(style))` sets the whole style of the next typed text at the caret.
- `selection_style()` reports, per mark, whether all, some or none of the selection has it (`MarkPresence`), and
  the link target when the selection is one link; with nothing selected, the style typing would give.
- `styled_text()` and `selected_styled_text()` read the model and the selection with its spans.

- `undo()` and `redo()` take the last edit back and make it again, as Ctrl+Z and Ctrl+Y do, and `can_undo()` and
  `can_redo()` say whether there is one: what an editor's undo button runs and is enabled by. They work on a plain
  input too.

lurq binds no shortcut to these: handle Ctrl+B in the input's `on_key_down` and call `prevent_default()`.

```rust
use lurq::{app::events::KeyboardEvent, components::TextMark};

let shortcuts = controller.clone();
input.on_key_down(move |event: KeyboardEvent| {
  let command = if cfg!(target_os = "macos") { event.meta } else { event.ctrl };
  if command && event.key.eq_ignore_ascii_case("b") {
    let _ = shortcuts.toggle_mark(TextMark::Bold);
    event.prevent_default();
  }
})
```

A toolbar button that should leave the keyboard focus in the input is built with `focusable(false)`. The
controller keeps the selection when focus moves anyway.

## Paragraphs

A second signal gives a styled input kinds of paragraphs: a `Signal<Vec<ParagraphKind>>` with one kind for each hard
paragraph of the value (the text between two line breaks), and a `ParagraphStyles` that says how each kind is drawn.

```rust
use lurq::{
  app::theme::PaletteColor,
  components::{ParagraphKind, ParagraphMarker, ParagraphStyle, ParagraphStyles, StyledSpan, TextInput},
  core::Signal,
  layout::text_style::FontWeight,
};

let value = Signal::new("Release notes\nBackdrop blur follows clips.\nFaster shaping".to_owned());
let spans = Signal::new(Vec::<StyledSpan>::new());
let kinds = Signal::new(vec![ParagraphKind::Heading(2), ParagraphKind::Body, ParagraphKind::Bullet]);
let styles = ParagraphStyles::new()
  .gap(10.0)
  .with(
    ParagraphKind::Heading(2),
    ParagraphStyle::new().font_size(22.0).weight(FontWeight::SemiBold).line_height(1.2),
  )
  .with(
    ParagraphKind::Bullet,
    ParagraphStyle::new().indent(21.0).marker(ParagraphMarker::Dot {
      size: 5.0,
      gap: 10.0,
      color: PaletteColor::MutedForeground.into(),
    }),
  );
let editor = TextInput::new(value).spans(spans).paragraphs(kinds, styles).multiline();
```

A `ParagraphKind` is `Body`, `Heading(1..=6)`, `Bullet`, `Ordered`, `Quote` or `Code`. A `ParagraphStyle` gives a
kind its type over the input's own (`font_size` or `font_size_scale`, `weight`, `line_height`, `font_family`,
`color`), the room before its text (`indent`) and what stands in that room (`marker`): a `Dot` centred on the first
row, a `Number` (the item's place in its run of numbered items, with a full stop) or a `Bar` down all its rows. A
`background` is a fill behind the paragraph across the input, and `joined(true)` leaves no room between two
paragraphs of the kind, as between the lines of a block of code. A run of paragraphs of one kind has one bar and
one fill. `ParagraphStyles::gap` is the room between two paragraphs. A mark is drawn over the type of the paragraph
it is in: bold in a heading is the heading's size.

The input keeps the kinds in step with the text. A paragraph that an edit splits (Enter, a paste of several lines)
gives its kind to both halves; paragraphs that an edit joins (Backspace at the start of one, a selection deleted)
take the kind of the first. The kinds are part of the undo history, and of what `on_styled_change` reports:
`StyledChange::paragraphs()` is what each paragraph is now, and an edit of the kinds alone is
`StyledEdit::Paragraphs { range }`, the indices of the paragraphs it changed. The app loads a document by setting
the three signals; a kinds signal shorter or longer than the text has paragraphs is read as one kind for each,
`Body` where none was given.

The controller's commands:

- `set_paragraphs(kind)` makes every paragraph the selection touches a `kind` (the one the caret is in with nothing
  selected), as one undo step.
- `selected_paragraphs()` is what each paragraph the selection touches is, for a toolbar to show, and
  `caret_paragraph()` the kind and the bytes of the paragraph the caret is in.
- `rich_text()` is the text, its spans and its kinds as one [`RichText`](#one-value-richtext).

What an editor does at the keys is the app's (or the design system's editor's): Enter in an empty item ending the
list, a paragraph after a heading. The input's own rule is the split and the join above.

## Objects

A styled input holds things that are not text, a picture among its words: each is one character of the value,
U+FFFC (`OBJECT_CHARACTER`), and a record beside it. A third signal holds the records, a
`Signal<Vec<InlineObject>>`, and a function builds what is drawn for one.

```rust
use lurq::{
  components::{InlineObject, StyledSpan, TextInput},
  core::Signal,
  ui::Picture,
};

let value = Signal::new("The lake: \u{FFFC}".to_owned());
let spans = Signal::new(Vec::<StyledSpan>::new());
// The character at byte 10 is a picture: where its pixels are, what it shows, how wide the document has it.
let objects = Signal::new(vec![InlineObject::new(10, "photos/lake.jpg").label("The lake at dusk").width(320)]);
let editor = TextInput::new(value)
  .spans(spans)
  .objects(objects, |object| {
    Picture::from_file(object.source.as_ref())
      .label(&object.label)
      .width(object.width.unwrap_or(320) as f32)
      .into()
  })
  .multiline();
```

An `InlineObject` has `at`, the byte of its character, `source` and `label`, two texts the app reads as it likes (a
picture's path and its alternative text), and `width`, in whole pixels, where the document says one. The element
`objects` is given back for it is a child of the input:

- It is laid out in the room the text has, and the size it takes is its box in the row. The box stands on the
  baseline, the row is as high as its tallest box needs, and the text keeps its line at the foot of the row. A box
  that does not fit the rest of a row goes to the next, as a word does. One wider than its row (less its
  paragraph's indent) is as wide as the row: an element that fits itself to its room (a picture keeps its shape)
  is as high as it then says, and any other is cut to a box of the shape it had.
- To the caret it is one character. The arrows go past it, a press on it selects it, Backspace after it (and Delete
  before it) selects it and the next press deletes it, and a selection takes it whole. The caret beside it is as
  high as the text's line, not as the box. While the input has the focus, a selected object is under a veil of the
  selection's color.
- It scrolls with the text and is cut by the input's box. The pointer and the focus do not reach what it holds: a
  press on it is a press on the text.
- An element declared as a component (`ui::Picture`, `Element::component::<C>(props)`) is mounted by the component
  that builds the input and kept across its renders, by what it shows: a picture loads once, whatever is typed
  before it.

The input keeps the records in step with the text. An edit before an object moves its `at`; one that deletes its
character deletes its record; undo brings both back, with the source, the label and the width it had. The records
are part of what `on_styled_change` reports (`StyledChange::objects()`, and `to_rich_text()` has them). The app
loads a document by setting the signals; a record whose byte holds no `OBJECT_CHARACTER` is dropped, and a
character with no record is a character like any other. `element` is called while the input is built, for each
record the signal holds then, and the signal is read there: a component that builds the input builds it again
when the objects change. Until it does, a child whose object is gone is not drawn, and the others keep their
places.

The controller's commands:

- `insert_object(&object)` puts an object in place of the selection, or at the caret, as one undo step, and
  leaves the caret after it. `false` for an input without objects.
- `objects()` is the records the text holds now, and `rich_text()` has them.

Objects need `spans` and are meant for a multiline input. In a masked input there are none.

Kinds are drawn where the input wraps its lines and its text is not justified: in a single-line input, one that
does not wrap or one whose text is justified, every paragraph is drawn as body text, and the kinds are still kept,
edited and reported. A justified row is stretched to the input's width, which is known only once every paragraph is
laid out, and the input lays out a paragraph at a time. A row of a paragraph with a kind is as high as its line
height says, whatever the font's own line is. An empty input that shows its placeholder draws it in the kind of its
first paragraph: its type, after its indent and its mark.

## One value: `RichText`

`RichText` holds what the signals hold as one value: `text`, `spans`, `paragraphs` and `objects`.
`RichText::new` puts the first three in the form an input keeps and `with_objects(objects)` adds the fourth,
`plain(text)` is body text with no marks, `blocks()` walks the paragraphs with their kinds, `plain_text()` is the
text with the label of each object in place of its character, and `word_count()` counts the words: an object is
no word, and stands between two.

```rust
use lurq::components::{ParagraphKind, RichText};

let notes = RichText::from_markdown("## Release notes\n\nBackdrop blur **follows clips**.\n\n- Faster shaping");
assert_eq!(notes.paragraphs, [ParagraphKind::Heading(2), ParagraphKind::Body, ParagraphKind::Bullet]);
assert_eq!(notes.to_markdown(), "## Release notes\n\nBackdrop blur **follows clips**.\n\n- Faster shaping");
```

`from_markdown` reads what a styled input can hold: paragraphs, headings, flat lists, quotes and code blocks, with
bold, italic, strikethrough, inline code, links, `<u>` underlines and images. A line break inside a paragraph
starts a new one, a nested list is flattened, and a table, a rule and other raw HTML are left out. `to_markdown`
writes a document that `from_markdown` reads back as the same one: marks nested, numbered items numbered, what
would read as markup escaped, an underline as `<u>…</u>`.

An image is an [object](#objects) where it stands, alone in its paragraph or among words: `![label](source)`, or
`<img src="source" alt="label" width="320">` for one with a width, which Markdown has no other way to say. Both are
read, and each is written as it was. An image inside a link keeps the link, and one inside bold words stays inside
them. In a line of a code block or a run of inline code, where Markdown reads no image, an object is written as
its label.

```rust
use lurq::components::{InlineObject, RichText};

let notes = RichText::from_markdown("The lake ![at dusk](photos/lake.jpg) from the north shore.");
assert_eq!(notes.text, "The lake \u{FFFC} from the north shore.");
assert_eq!(notes.objects, [InlineObject::new(9, "photos/lake.jpg").label("at dusk")]);
assert_eq!(notes.plain_text(), "The lake at dusk from the north shore.");
assert_eq!(notes.to_markdown(), "The lake ![at dusk](photos/lake.jpg) from the north shore.");
```

## Copy, paste and links

- `copy_format(f)` sets what Ctrl+C and Ctrl+X put on the clipboard: `f` gets the styled selection and returns the
  text, such as Markdown. Without it the plain text is copied.
- A paste inserts the clipboard's text in the typing style. To paste styled text, handle `on_paste`, parse the
  text, call `controller.replace_selection_styled(&styled)` and `prevent_default()`.
- `rich_clipboard(true)` (feature `markdown`) keeps the formatting of an input with kinds of paragraphs through the
  clipboard, as the design system's rich text editor does. A copy or a cut writes the selection's Markdown, its marks,
  its objects and the kinds of the paragraphs it touches, beside its text. The system's clipboard holds one text: it
  is given the Markdown where the selection has a mark, an object or a paragraph that is not body text, and the plain
  text otherwise, so another app or another lurq process reads the Markdown as it is written. An object is there as
  the image Markdown writes, `![label](source)`, and in the plain text as its label: the clipboard stays text, and
  the pixels of a picture are not put on it. A paste of that Markdown into an input with objects is the object
  again, with its record; a picture the system's clipboard holds as pixels is not read. This process keeps the plain text beside
  it, and while the clipboard still holds what it wrote, an input of the app without the option pastes the plain
  text. A paste with Markdown beside its text, or whose text reads as Markdown (`RichText::reads_as_markdown`: a line
  with a block mark, `#`, `-`, `1.`, `>` or a fence, or a mark Markdown reads, `**bold**`, `` `code` ``, a link), is
  inserted with its marks and kinds as one undo step: what is copied from one editor is pasted into another, in this
  app or another, as it was. Any other text, and every paste into a line of code, is plain text. A single pasted
  paragraph goes into the line at the caret and leaves it its kind unless the line is empty; of several, the last
  gives its kind to the text after the caret. `controller.replace_selection_rich(&rich)` does the same from the
  app.
- `on_link_click(f)` runs when a link is clicked with Ctrl (Cmd on macOS) held, with its target and whole range;
  the app opens it. A plain click places the caret.

## Look

The theme's `ThemeTextMarks` (`theme.set_text_marks`) holds one `TextMarkStyle` per mark: font family, size scale,
weight, style, features, a color and a background fill (palette roles or colors), and an underline or a line
through. A run with several marks applies code, bold, italic, underline, strikethrough, then link. The defaults are
a bold weight, an italic style, a line under, the `monospace` family at 0.92 over the `SurfaceRaised` role, a line
through, and the `Accent` role underlined.

## Limits

- A masked input is plain, and a styled input ignores `tokens`: what is one thing to its caret is an
  [object](#objects).
- A single-line input lays an object out in its one row, which is as high as the input's line: a box higher than
  that rises over it. Objects are for a multiline input.
- Code fills have no horizontal padding.
- A multiline input lays out a paragraph at a time and keeps each one, so an edit lays out only the paragraph it
  changed. A long left-to-right paragraph is also shaped in chunks of a few hundred bytes, cut after spaces where
  its own words choose, so an edit shapes only the chunk it changed; its rows are still laid out again. Paragraphs
  with right-to-left text or tabs are shaped whole. `benches/styled_input.rs` measures typing, caret moves,
  selection drags and style toggles on 4,000 characters as one paragraph and as 50.
- Span edits copy the input's span list (linear in its spans, not in its text).
