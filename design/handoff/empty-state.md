# Kbd, Empty State and Result: hand-off

`ui::Kbd` (with `ui::InlineCode`), `ui::EmptyState` and `ui::Outcome` (the board's Result), from the board's `Kbd`
(`zGoRU`), `Inline Code` (`P3Ytv`), `Empty State` (`KLVG2`) and `Result` (`EvvSZ`) in Components / light (`Ovt1D`),
read with where they are used: the keys of the search field, the command palette, the shortcut recorder, the ghost
completion, the Markdown document and `Tablet/Shortcut Overlay` (`zSCTi`); the shortcuts written as text in the menus
and the tooltip's own key; the empty table of `Table/Empty` (`Th2wb`) and `Mobile/Empty State` (`nesN1`); the
result's states error, warning, info and not found (`Q5tNfy`, `M57HEo`).

Files: `crates/lurq/src/ui/kbd.rs` (`Kbd` and `InlineCode`), `crates/lurq/src/ui/empty_state.rs`,
`crates/lurq/src/ui/outcome.rs`, `crates/lurq/tests/ui/kbd.rs`, `empty_state.rs`, `outcome.rs`,
`crates/gallery/src/pages/kbd.rs`, `empty_state.rs`, `outcome.rs` (pages "Kbd", "Empty State" and "Result", group
Data), this file, and lines in the three `mod.rs` files.

## Changed outside the component's own files

No change to the engine or to shared code of `lurq::ui`.

- `crates/gallery/src/kit.rs`: `keys` removed, as the kit says its parts go when their component lands. Its uses now
  build `ui::Kbd`: `shell.rs` (the search field's keys, `Kbd::shortcut("Mod+K").platform(KeyPlatform::current())`,
  which replaces `find_chord`), and the lists of keys on the pages of the number input (`Kbd::new`), the pagination
  and the slider (`Kbd::shortcut`, their `"Shift Tab"` now written `"Shift+Tab"`, a key each). A sibling that still
  calls `kit::keys` will not compile after the merge: use `Kbd::new(key)` or `Kbd::shortcut(keys)`.

## For `docs/src/content/docs/design-system.md`

## Kbd

A key of the keyboard, the keys of a shortcut pressed together, or steps pressed one after another: what a person presses to do something.

```rust
use lurq::ui::{InlineCode, Kbd, KbdSize, KbdVariant, KeyPlatform};

Kbd::new("Esc");

// A key each. `Mod` is Ctrl, and ⌘ on a Mac.
Kbd::shortcut("Mod+K").platform(KeyPlatform::current());

// Pressed one after another: G, then D; or two shortcuts.
Kbd::sequence(["G", "D"]);
Kbd::sequence(["Ctrl+K", "Ctrl+S"]).size(KbdSize::Lg);

// As a menu writes it: one text, no box. "⌘K" on a Mac.
Kbd::shortcut("Mod+K").variant(KbdVariant::Plain);

// A short text of code in a run of text.
InlineCode::new("ctx.theme()");
```

| Builder | |
| --- | --- |
| `Kbd::new(&str)` | One key, written as it is given: "Ctrl", "Page Up", "/" |
| `Kbd::shortcut(&str)` | The keys of a shortcut, parted by `+`, a box each. A `+` after another, or alone, is the key of that name: `Ctrl++` |
| `Kbd::sequence(steps)` | Steps pressed one after another, each a key or a shortcut, with "then" between them |
| `Kbd::accelerator(&Accelerator)` | The keys of an accelerator of the app's menu: its modifiers, then its key |
| `platform(KeyPlatform)` | `Standard` (words, the default) or `Mac` (⌃ ⌥ ⇧ ⌘, ↩ ⌫ ⇥ ⌦, the modifiers first in a Mac's order). `KeyPlatform::current()` is the platform the app is built for |
| `variant(KbdVariant)` | `Default` (a box for each key) or `Plain` (each step as one text in the muted color, with no box: "Ctrl+K", "⌘K") |
| `size(KbdSize)` | `Sm` 16 high at 10, `Md` 20 at 11, `Lg` 24 at 12 |
| `separator(&str)` | What stands between the steps of a sequence: "then" unless told |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [badge](#badge) has them. `id` is the id of the root |

- Keys are named as lurq names them in a `KeyboardEvent` (`Escape`, `ArrowUp`, `Backspace`, `PageUp`) or as a person writes them, in any case and with spaces or none: `Esc`, `Del`, `Page Up`. `Mod` is Ctrl, and ⌘ on a Mac; `Meta`, `Cmd`, `Super` and `Win` are ⌘ on a Mac. Arrows are drawn as ↑ ↓ ← →. A single letter is drawn in capitals; a key lurq does not name is drawn as it is written (`Up` stays `Up`).
- On a Mac the modifiers are glyphs and stand first, in the order `Accelerator::display` writes them: ⌃ ⌥ ⇧ ⌘. In words they keep the order they are given in.
- Keys are read and not operated: they take no press, no focus and no stop of Tab. To what reads the tree (`lurq_snapshot` of the MCP server) they are one `text`, called by their keys in words: "Ctrl+K", "Cmd+Shift+P", "G then D".
- Parts: `KbdPart::Root` (the row of keys, its gap), `Key` (a key's box), `Label` (the text on a key, or of a plain step), `Separator` (what stands between steps). Variants: `KbdVariant`. Sizes: `KbdSize`. States: `KbdState::Sequence`, `Mac`.
- In the design a key is 20 high, padded 6, on `Muted` with an edge of 1 in `Border` and the radius `Sm`, its text 11 in the family of code in `MutedForeground`. The keys of a shortcut are 4 apart with nothing between them; large ones are 24 high, padded 7, at 12 and 6 apart, as the list of an app's shortcuts has them. Plain is the text of a menu's shortcut: 11 in the family of code in `MutedForeground`.
- A renderer that draws other keys has `view.steps` (each step's keys, with what is drawn on each and what it is called), `view.root()`, `view.key_cap(key)`, `view.label(text)`, `view.between()` and `view.written(step)` (a step as one text).
- `InlineCode::new(&str)` is a short text of code in a run of text: 13 in the family of code in `Foreground`, on `Muted` with the radius `Sm`, padded 5 across and 1 above and below. It is not cut, and is `code` to what reads the tree, called by its text. Parts: `InlineCodePart::Root`, `Label`.

## Empty State

What stands where content would be when there is none, and what to do about it: no results, no windows yet, all caught up.

```rust
use lurq::ui::{Button, ButtonSize, ButtonVariant, EmptyState, EmptyStateSize, Length, icons};

EmptyState::new("No windows yet")
  .description("Open a window to see it listed here.")
  .icon(icons::INBOX)
  .action(Button::new("New window").icon(icons::PLUS).variant(ButtonVariant::Outline));

// In a table or a list: the icon alone, no edge; centred in all of the room the list would have.
EmptyState::new("No crates match")
  .description("Check the spelling or clear the filter.")
  .icon(icons::SEARCH_CLOSE)
  .size(EmptyStateSize::Sm)
  .height(Length::Fill)
  .action(Button::new("Clear filter").variant(ButtonVariant::Outline).size(ButtonSize::Sm));
```

| Builder | |
| --- | --- |
| `EmptyState::new(&str)` | What is empty, in a line. It is what the empty state is called |
| `description(&str)` | Why, or what to do, under the title |
| `icon(name)` | An icon of the theme's set over the title, in its well |
| `media(element)` | An element of the app's in place of the icon and its well: an illustration, a picture |
| `action(element)` | Something to do under the text; called again, a second one after it, the main one last |
| `size(EmptyStateSize)` | `Sm` (the icon alone, 22, everything 10 apart), `Md` (a well of 40, in an edge), `Lg` (a round well of 64, a title of 18) |
| `bordered(bool)` | An edge around it. `Md` has one unless told; `Sm` and `Lg` have none |
| `height(Length)` | A height of its own, in which what it holds is centred: `Length::Fill` for all of the room a parent that stretches it gives |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [badge](#badge) has them. `id` is the id of the root |

- It is as wide as the room it is given and as high as what is in it, all of it centred; the title and the description wrap in the width there is. Its actions are the app's elements, as large as the place has them: a small button in a table.
- It is read; what it gives to do is operated as itself. To what reads the tree it is a `status`, called by its title, with its description as `a11y_description`.
- Parts: `EmptyStatePart::Root`, `Media` (the well), `Icon`, `Text` (the title over the description), `Title`, `Description`, `Actions`. Sizes: `EmptyStateSize`. States: `EmptyStateState::Bordered`.
- In the design the medium one is padded 32, with 16 between the well, the text and the actions, in an edge of 1 in `Border` with the radius `Xl`; the well is 40 on `Muted` with the radius `Lg`, its icon 20 in `Foreground`; the title is 14/500 in `Foreground`, the description 4 under it at 13 in `MutedForeground`; the actions are 8 apart. The small one is the empty table's: padded 32 and 16, everything 10 apart, the icon 22 in `MutedForeground` with no well. The large one is the phone's: padded 32 and 24, a round well of 64 with an icon of 28, a title of 18/600 and a description of 15, 6 apart.
- A renderer that draws another empty state has `view.root()`, `view.media()` (the app's element or the icon in its well), `view.text()` and `view.actions()`.

## Result

What a page or a dialog says when something is over: payment received, build failed, nothing here. It is `ui::Outcome` in the code, so that `use lurq::ui::*` does not hide `std::result::Result`.

```rust
use lurq::ui::{Button, ButtonVariant, Outcome, OutcomeVariant};

Outcome::new(OutcomeVariant::Success, "Build succeeded")
  .description("412 tests passed in 38 seconds.")
  .action(Button::new("View log").variant(ButtonVariant::Outline))
  .action(Button::new("Deploy"));

// On a page of its own, or in a dialog: no box.
Outcome::new(OutcomeVariant::NotFound, "Nothing here")
  .description("This page was moved or never existed.")
  .bordered(false)
  .action(Button::new("Home"));
```

| Builder | |
| --- | --- |
| `Outcome::new(OutcomeVariant, &str)` | What came of it, and its title, which is what it is called. `Success`, `Error`, `Warning`, `Info` or `NotFound` (a 404) |
| `description(&str)` | More of it, under the title |
| `icon(name)` | Another icon of the theme's set in the mark, in the variant's colors |
| `media(element)` | An element of the app's in place of the mark |
| `extra(element)` | What stands under the text and over the buttons, as wide as the outcome: a summary, a list |
| `action(element)` | A button at the bottom, after those there: the least first, the main one last |
| `bordered(bool)` | A box of its own with a fill and an edge, as the design draws it: true unless told |
| `width(Length)` | A width of its own in place of the design's 300 |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [badge](#badge) has them. `id` is the id of the root |

- Everything in it is centred; its title and description wrap in its width. With no box it has no padding either, and stands where what holds it puts it: a column that centres it on a page, the body of a [dialog](#dialog).
- It is read; its buttons are operated as themselves. To what reads the tree it is a `status`, called by its title, with its description as `a11y_description`.
- Parts: `OutcomePart::Root`, `Mark`, `Icon`, `Text`, `Title`, `Description`, `Extra`, `Actions`. Variants: `OutcomeVariant` (`OutcomeVariant::icon()` is the icon each is marked with). States: `OutcomeState::Bordered`.
- In the design it is 300 wide, padded 24 with 16 between the mark, the text and the buttons, on `Background` in an edge of 1 in `Border` with the radius `Lg`. The mark is a circle of 48 in the variant's muted color, its icon 24 in the variant's color: a check in `Success`, a cross in `Destructive`, a warning in `Warning`, an `i` in `Info`, and a crossed magnifier in `Foreground` on `Muted` for not found. The title is 16/600 in `Foreground`, the description 4 under it at 13 in `MutedForeground`; the buttons are 8 apart.
- A renderer that draws another outcome has `view.root()`, `view.mark()`, `view.text()`, `view.extra()` and `view.actions()`.

## For `CHANGELOG.md`

- `ui::Kbd`: a key, the keys of a shortcut (`Kbd::shortcut("Mod+K")`), steps pressed one after another (`Kbd::sequence`) or an accelerator of the app's menu, in words or with a Mac's glyphs in a Mac's order (`KeyPlatform`), in a box each or as the plain text a menu writes, in three sizes; and `ui::InlineCode`, a short text of code on a muted fill. The gallery's `kit::keys` is gone for it.
- `ui::EmptyState`: what stands where content would be when there is none, with an icon in a well or the app's element, a title, a description and one or two actions, in three sizes (a table's, the default in an edge, a phone's), centred in the room it is given.
- `ui::Outcome`, the design's Result: what a page or a dialog says when something is over, in five variants (success, error, warning, info, not found) with their marks and colors, the app's icon or element, what the app puts under the text, and its buttons, in a box or on a page.

## Decided

- **`Outcome`, not `Result`.** A `Result` in `lurq::ui` would hide `std::result::Result` wherever `lurq::ui::*` is imported, and `ResultView` is the renderer's view of something else already. `Outcome` says what it is (what came of something) and keeps the usual names: `OutcomeVariant`, `OutcomePart`, `OutcomeView`. The gallery's page and the docs keep the board's name, Result, and say what it is called in the code.
- **`InlineCode` is a component of its own, in `kbd.rs`.** The board's inline code is a look of its own (a muted fill, a radius, a padding), not the mono family alone; it is the same kind of thing as a key (a small text read in a run of text), so it is built beside it.
- **The words of a key are lurq's.** Keys are matched by the names `KeyboardEvent` gives (`ArrowUp`, `Escape`, `Backspace`, `PageUp`) and by the usual ways of writing them, and a Mac's glyphs and order are those of `Accelerator::display` (⌃ ⌥ ⇧ ⌘). `Accelerator` itself writes one string with no room between keys; `Kbd::accelerator` draws its keys a box each. `Mod`, the primary modifier, is ⌘ on a Mac and Ctrl elsewhere, as editors write it, so an app writes a shortcut once. The platform is the app's to ask for (`KeyPlatform::current()`); unasked, keys are in words, so what a test or a document shows does not change with the machine.
- **A sequence says "then" between its steps**, in the muted color at the size of the keys. The board draws no sequence; a word reads better than an arrow and is what tooling is told. `separator(..)` changes it, for another language too.
- **Small keys are the tooltip's size** (16, padded 4, at 10) in the key's own colors. The tooltip's key is a look of its own on the dark tooltip (white at 15%, the whole shortcut in one box); `ui::Tooltip` is left as it is.
- **`ui::Menu` keeps writing its shortcuts as text.** A later change: `MenuItemView::shortcut` builds `Kbd::shortcut(keys).variant(KbdVariant::Plain)`, with the menu's platform (`KeyPlatform::current()`, or one the menu is given), styled by the menu's `Shortcut` part through `.rules(..)` so that a theme's rules for the menu still apply. It draws what the menu draws now, 11 in the family of code in `MutedForeground`, and on a Mac writes ⌘K for `Mod+K`. The same holds for the command palette's rows, which are a menu's.
- **An empty state's edge goes with its size.** The board's empty state (`KLVG2`) has an edge, and the empty table's and the phone's have none: the medium size has one, the others not, and `bordered(..)` says otherwise. It is as wide as the room it is given rather than the board's 320, since it stands where content would be; a height given to it centres what it holds in it.
- **An empty state's actions and an outcome's buttons are the app's elements**, not labels the component makes buttons of: an app chooses their variants and sizes (the board's are outline, primary, small in a table, large on a phone), and a link or a menu can stand there.
- **An outcome is a box of 300 by default**, as the board draws it; on a page or in a dialog it is `bordered(false)` (no fill, no edge, no padding). Its variant's icon can be another (`icon`), in the variant's colors; the whole mark can be the app's element (`media`).
- **Both are a `status` to tooling**, called by the title and described by the description; neither takes the focus.

## Not done

Nothing.

## Differs from the board

- The board's empty state is 320 wide; here it is as wide as the room it is given. The board's result is 300 wide, as here.
- The board's empty table has its button 30 high and padded 14; the gallery shows it with `ButtonSize::Sm`, 32 high and padded 12. The phone's button is 200 wide; the gallery's is as wide as its label.
- The board draws no sequence of keys, no small key in the key's colors, no plain key and no Mac's glyph other than ⌘ and ⌫; they are decided above. Its tooltip's key is left to the tooltip.
- The board's glyphs are drawn with a stroke of 1.667 (20), 1.833 (22), 2 (24) and 2.333 (28); the theme's icon font has its own weight.
