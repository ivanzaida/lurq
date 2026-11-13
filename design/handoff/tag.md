# Tag and Chip: hand-off

Built on `claude/ds-tag-f16e15`: `ui::Tag` (with the board's Pill as `Tag::pill(true)`), `ui::Chip` and
`ui::ChipGroup`. Files of their own: `crates/lurq/src/ui/tag.rs`, `crates/lurq/src/ui/chip.rs`,
`crates/lurq/tests/ui/tag.rs`, `crates/gallery/src/pages/tag.rs`, this file.

## Changed outside the component's own files

- `crates/lurq/src/mcp/tools/inspect.rs` (`semantic_state`): a node with `a11y_pressed` reports `state.pressed`
  (`true` or `false`), as one with `a11y_selected` reports `state.selected`. Eight lines after the `a11y_selected`
  block. A chip says whether it is pressed this way.
- `crates/lurq/src/ui/mod.rs`: `mod chip;`, `mod tag;` and their two `pub use` lines.
- `crates/lurq/tests/ui/mod.rs`: `mod tag;`.
- `crates/gallery/src/pages/mod.rs`: `mod tag;` and the `Page` entry "Tag" (`/tag`, `Group::Data`), after Steps: it
  is the first page of the Data group, so the other display pages' entries go beside it at the merge.
- Nothing in `shared.rs`, `part_style.rs`, `recipe.rs` or the engine; no keyframes.

## For `docs/src/content/docs/design-system.md`

## Tag

A label on a thing, a keyword or a category, that the app may let a person take away.

```rust
use lurq::ui::{Tag, TagSize, TagVariant, icons};

Tag::new("design");
Tag::new("in review").variant(TagVariant::Info).icon(icons::CIRCLE);

// With the button that takes it away: the app holds the list.
Tag::new("rust").on_remove(move || topics.update(|all| all.retain(|t| t != "rust")));

// Round, with no line: the token of a box that holds several values.
Tag::new("React").pill(true).on_remove(move || remove("React"));
```

| Builder | |
| --- | --- |
| `variant(TagVariant)` | `Neutral` (the default), `Info`, `Success`, `Warning`, `Destructive`, or a variant the theme's recipe names |
| `size(TagSize)` | `Sm` (20 high), `Md` (24, the default), `Lg` (28) |
| `icon(name)` | An icon before the text, by its name in the theme's set |
| `on_remove(handler)` | Gives the tag its remove button, and is called on a press of it |
| `pill(bool)` | Round at both ends, with no line around it |
| `disabled(bool)` | Dimmed; its remove button is not pressed and Tab passes it by |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the root's; `force` draws the remove button |

- A tag does not keep whether it is there: the app holds its list, and `on_remove` tells it which one a person took away. A tag with no handler is read only: it has no button and is no stop of Tab.
- The remove button is a button of its own, called "Remove" and the tag's text ("Remove rust") to what reads the tree. Tab reaches it, and a press, Enter, Space, Backspace and Delete press it; Backspace and Delete only while the focus is on it, with no modifier held.
- Parts: `TagPart::Root`, `Icon`, `Label`, `Remove` (the button), `RemoveIcon` (the cross: its side). States: `TagState::Removable` (it has the button), `Pill`, `Disabled`.
- In the design a tag is 24 high, padded 8, with the corners of `RadiusSize::Sm`, in `Secondary` with a line of `Border` inside it, its text 12 and medium in `SecondaryForeground`. A removable one is padded 4 at its end, where a button of 16 (corners of 3) holds a cross of 11 in `MutedForeground`, 4 after the text; under the pointer the button is filled `StateHover` and its cross is `Foreground`. `Info`, `Success`, `Warning` and `Destructive` are in the muted fill of their color, line and fill alike, with the text in the color. A pill is round, has no line, is padded 10 and 6, and its cross is 12.
- Several tags side by side are an app's row: `Row::new().spacing(6.0).wrap()`, 6 apart as the board lays them out.
- A renderer that draws another tag starts from `view.remove()`, the button with its behavior and its cross, `None` for a tag with no handler.

## Chip

A tag that is pressed: a filter or a choice that is on or off. Open issues only, a language among others.

```rust
use lurq::ui::{Chip, ChipGroup, Choice};

Chip::new("Rust").selected(rust.get()).on_change(move |on| rust.set(on));
// Bound to the app's signal: a press turns it.
Chip::new("Open").bind(open);

// Any number chosen: the signal's values. One chosen: `ChipGroup::new(kind, [..])`.
ChipGroup::multiple(languages, [
  Choice::new("rust", "Rust"),
  Choice::new("swift", "Swift"),
  Choice::new("cobol", "Cobol").disabled(true),
]);
```

| Builder | |
| --- | --- |
| `selected(bool)` | Whether it is chosen. The chip does not keep it: the app says so |
| `on_change(handler)` | Called on a press with what the chip is to be: chosen if it was not |
| `bind(Signal<bool>)` | Chosen while the signal is true, which a press turns |
| `icon(name)` | An icon before the text while it is not chosen; a chosen chip has the check there |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them |

| `ChipGroup` | |
| --- | --- |
| `ChipGroup::new(signal, choices)` | Exactly one chosen: the signal's value. A press of another sets it |
| `ChipGroup::multiple(signal, choices)` | Any number chosen: the signal's values. A press adds a value or takes it out |
| `ChipGroup::controlled(selected, choices, press)` | The app holds what is chosen, and `press` is told each value pressed |
| `disabled(bool)`, `style(part, ..)`, `id`, `key` | `ChipGroupPart::Item` lays rules over each chip's root |

- Tab reaches a chip, Space and Enter turn it, and the focus ring is the chip's. A group is one stop of Tab: the arrow keys move between its chips and Space or Enter turns the one the focus is on. A choice is a [`Choice`](#toggle-group), as a toggle group's: its icon is the chip's.
- To what reads the tree a chip is a `button` called by its text, with `a11y_pressed` saying whether it is chosen; `lurq_inspect` reports it as `state.pressed`.
- Parts: `ChipPart::Root` (what is pressed), `Icon`, `Label`; `ChipGroupPart::Root`, `Item`. States: `ChipState::Selected`, `Leading` (it has an icon before its text: the check or its own), `Disabled`.
- In the design a chip is 28 high and round, padded 12, its text 13 and medium. One not chosen is in `Background` with a line of `Input` and its text in `Foreground`, `Highlight` under the pointer; a chosen one is in `Primary`, line and fill, its text in `PrimaryForeground`, with a check of 14 before it, 6 from it and padded 8 at its start, and in `PrimaryHover` under the pointer. A disabled chip is dimmed. The chips of a group are 6 apart and go on to the next line where the row has no room.
- A renderer that draws another chip starts from `view.root()`, the behavior with the root's style; `view.leading()` is the icon the default renderer puts before the text.

## For `CHANGELOG.md`

- `ui::Tag` and `ui::Chip`: a tag is a label on a thing in five colors and three sizes, with an icon, round as a pill, and a remove button called "Remove" and its text that Tab reaches and Backspace or Delete press; a chip is a tag that is pressed, on or off, bound to a `Signal<bool>` or told of a press, and `ui::ChipGroup` makes several one choice of one or of any number, with one stop of Tab and the arrow keys; `lurq_inspect` reports a node's `a11y_pressed` as `state.pressed`.

## Decided

- **What the board means by the three.** Tag (`HY236`) is a label: the board's "Tags · removable" row has three removable ones and a read-only one, with no remove button and padded 8 at both ends; the table toolbar uses it for the filters that are set ("Status: Passing", info). Chip (`zhKGK`) is a pressed choice: the "Chips · selectable" row has one chosen (filled, with a check) and others not (outlined), one of them disabled. Pill (`Su09I`) is used once, in "Bits" of Advanced Inputs: a removable label round at both ends with no line, with its cross after the text. It is a tag drawn another way, not a component of its own, so it is `Tag::pill(true)`: a state and not a variant, so that a pill has the colors and the sizes of a tag.
- **`ToggleGroup` is not what the board draws for chips**: it is a bar inside one line with square toggles and lines between them. `ChipGroup` is a row of whole chips 6 apart that wraps; it is built as `ToggleGroup` is (`Choice`, `shared::Chosen`, `FocusGroup::horizontal().one_stop(true)`, `rules_for(Item, ChipPart::Root)`) and builds `Chip`s.
- **Tags side by side are an app's row**, not a component: they are not one choice, the keyboard already reaches each remove button, and the board's "Tag group" is a row with a gap of 6.
- **The remove button is the close-button pattern of `ToastView::close`**: `shared::pressable` with `icons::CLOSE` inside, `a11y_role` `button`, a name. Its name is "Remove " and the tag's text, so that a list of removable tags reads as what each button does. Backspace and Delete are an `on_key_down` on the button that acts only on `event.focus_within` and with no modifier (Shift included), since a key reaches every node.
- **The tag itself is not focusable.** Only its button is a stop of Tab: a read-only tag has nothing to do, and a tag that is a link or a filter to open is a `Chip` or a `Link`.
- **Variants are a tag's colors**: the board draws `Neutral` and `Info`; `Success`, `Warning` and `Destructive` follow `Info` with the muted roles the palette has for them. The colored ones have a line of their own fill, as the board's info tag has (`stroke: $info-muted`), so they are as wide as the neutral one.
- **`force` on a tag draws its remove button**, the only part the pointer and the keyboard act on.
- **A chip has no sizes and no variants**: the board draws one of each. Its own icon (`Chip::icon`) gives way to the check while it is chosen, so a chosen chip reads the same with or without one.
- **The chip's hover** is a quiet button's (`Highlight`, `HighlightForeground`) while it is not chosen and the primary button's (`PrimaryHover`, line and fill) while it is; a press lays the `StatePressed` veil, as the toggle does.
- **`Chip::bind(Signal<bool>)`** reads the signal as the chip is built and sets it on a press, beside `selected` and `on_change`, which it is made of.
- The root of a tag and of a chip group lets what is in it paint outside it (`overflow_visible`), so that the ring of a remove button or of a chip at the edge is drawn whole.
- The remove button's corners are 3, the board's number, not a radius of the theme (the smallest is 4).

## Not done

- **The focus after a tag is taken away.** When a person takes a tag away from the keyboard, the app takes the tag out of its list and its remove button, which had the focus, leaves the tree: the focus is then on nothing, and the next Backspace takes nothing away. A person removing tags one after the other expects the focus to go on to the next tag's button. A builder cannot move the focus. The smallest change: the engine, when the focused node leaves the tree in a pass, gives the focus to the stop of Tab that took its place in the order (the next one, or the one before where it was the last) instead of to nothing, at least inside a `FocusGroup` that asks for it (`FocusGroup::keep_focus(true)`); the tags of an app's row would then sit in such a group.

The tests in `crates/lurq/tests/ui/tag.rs` were written and compiled, not run, as the batch's flow has it.

## Differs from the board

- The board draws the tag at 24 high only (and 26 in the table toolbar's filters, a height of that toolbar's, which a rule there can give: `.style(TagPart::Root, |s| s.height(26.0))`). `TagSize::Sm` (20, text 11, button 14, cross 10, padded 6) and `Lg` (28, text 13, icon 14, button 18, cross 12, padded 10, gap 6) are lurq's.
- The board draws no icon and no dot on a tag; the icon is lurq's, 12 (14 at `Lg`), in the color of the text. No dot was built.
- The board draws the remove button only at rest: its hover (`StateHover`, cross in `Foreground`), its press (`StatePressed`) and its ring are lurq's. So are a disabled tag (dimmed), and the chip's hover, press and ring.
- The board's pill draws its cross of 12 straight in the row, with a stroke of 1 against the tag's 0.917; lurq's is a button of 12 with round corners holding the cross of the icon font, so that it can be pressed and have a ring.
- The Multi select's tokens ("Token Rust" in Advanced Inputs) are drawn neither as Tag nor as Pill: the corners of a tag, no line, padded 8 and 6, a cross of 12. The Multi select is not in this batch; it can draw them as `Tag::new(..).on_remove(..)` with a rule that takes the line away, or as a pill.
- `Mobile/Chip` (32 high, padded 14, text 14, outlined) is the mobile skin's and was not built: it is a recipe over the same parts.
- The board's chip groups are one line; lurq's go on to the next line where they have no room.
