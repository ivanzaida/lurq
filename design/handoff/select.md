# Select: hand-off

`ui::Select`, one value chosen from a list that opens under a trigger (board: `fte2x` Select, with `iPsCT` Menu and
`q4kDx` Menu Item for its list, in "Select & Menu" of `Components / light`).

## Changed outside the component's own files

The engine's select paints its trigger itself, from a `SelectPartStyle`, and that style had no ring. The board's
focus and open look is a line of 2 inside the box in `Ring`, the one `ui::Input` has (an outline of 2 at an offset
of -2): drawn as a thicker border it would be a different look from the input's, and an inset shadow cannot be
said by a recipe's `outline`. So, as the slider's session did for its thumb:

- `node/select_style.rs`: `SelectPartStyle` has an `outline: Option<Outline>` and a builder `outline(Outline)`,
  merged like its other fields.
- `layout/layout_engine/select_quads.rs`: `push_select_trigger_quads` keeps the resolved trigger's outline as a
  ring through `push_ring` (the slider's change made it), so `trigger_focused` and `trigger_open` can be a ring.
  The menu and the option rows do not paint an outline.

Nothing else of the engine changed. The engine's own `Select::style_with(|s| s.trigger_focused(SelectPartStyle::new().outline(..)))` now works too, which `styling-events.md` could say.

## For `docs/src/content/docs/design-system.md`

## Select

One value chosen from a list that opens under a trigger shaped like an input: a theme, a time zone, a page size.

```rust
use lurq::{components::SelectOption, ui::{Select, SelectSize}};

// theme: Signal<String>
Select::new(theme.clone(), [
  ("system".to_owned(), "System"),
  ("light".to_owned(), "Light"),
  ("dark".to_owned(), "Dark"),
]);

// Nothing chosen until the signal has an option's value; an option can have a detail, or be disabled.
Select::new(zone.clone(), [
  SelectOption::new("Europe/Kyiv".to_owned(), "Kyiv").detail("UTC+2"),
  SelectOption::new("Asia/Tokyo".to_owned(), "Tokyo").disabled(true),
])
.placeholder("Select a time zone")
.size(SelectSize::Sm)
.invalid(zone.get().is_empty());
```

| Builder | |
| --- | --- |
| `new(Signal<T>, options)` | What it is bound to, and its options: `(value, label)` pairs or `SelectOption`s (a detail line, an element before or after the label, disabled). The option whose value the signal has is chosen; choosing another sets the signal |
| `placeholder(&str)` | What the trigger says while no option is chosen |
| `size(SelectSize)` | `Sm` 32, `Md` 36 (the default), `Lg` 40 high, with text of 13, 14, 15: an input's sizes |
| `invalid(bool)` | What is chosen is wrong, or nothing is: the edge and the ring in the destructive color |
| `full_width(bool)` | As wide as what holds it, in place of 260 |
| `label(&str)` | What it is called, to what reads the tree. The field it is in shows its label |
| `name(..)` | The name of its value in a form, the chosen option's label (feature `form`) |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the select's node. `state(SelectState::Open, true)` draws the trigger as it is while open, for a gallery |

- It is the engine's [`Select`](../styling-events/#select) dressed by the recipe, not a second one: a press, or Enter, Space and the arrows from the keyboard, open the list under it (over it where there is no room); the arrows, Home, End and the first letters move through it, skipping disabled options; Enter or a press chooses, and Escape closes. Tab reaches it wherever it is (`tab_index(0)`); a disabled one does not open and Tab passes it by.
- It stands beside an [input](#input): the same box, sizes, edge and ring. The ring is a line of 2 inside the box in `Ring`, shown from the keyboard and while the list is open.
- Parts: `SelectPart::Trigger` (the select's node and box: its size, edge, fill, padding, ring, opacity, cursor, and the type of what it says), `Value` and `Placeholder` (laid over the trigger's type and color), `Chevron` (its side and color), `Menu` (the panel: fill, edge, corners, shadow, padding; its gap is the room under the trigger), `Item` (an option: height, padding, corners, type, color; `hovered` is the item under the pointer or the keys; its gap is the room before the check), `Check` (the chosen item's mark), `Detail`. States: `SelectState::Open`, `Invalid`, `Disabled`, `FullWidth`, and of an item `Chosen` and `ItemDisabled`. Sizes: `SelectSize::Sm`, `Md`, `Lg`.
- In the design the trigger is 260 by 36, `Background` in an edge of 1 in the `Input` color, `radius-md`, the text 12 from the edge in the body's type, the placeholder and a chevron of 16 in `MutedForeground`; `Ring` under the pointer; `Destructive` while invalid, under the pointer, in the focus and open; disabled is half opaque on `Muted`. The list is 4 under it and as wide, on `Popover` in an edge of `Border`, `radius-lg`, with the theme's middle shadow and 4 of padding. An item is 32 high, its text 8 in, `radius-sm`, in the body's type, on `Highlight` under the pointer or the keys; the chosen one has a check of 16 in `Foreground` at its end, 8 after the label; a disabled one is at 0.45.
- The engine paints the trigger, the panel and the items itself, from its `SelectStyle`, not from nodes the recipe styles: of the trigger it takes the fill, the edge, the corners, the shadow, the ring and the padding, at rest, `hovered`, `focus_visible` and in the state `Open`; of an item, the fill, edge, corners, padding, height (at least), type, color and opacity, at rest, `hovered` and in the states `Chosen` and `ItemDisabled`. A transition is not painted, and the room between the value and the chevron is the engine's 8.
- To what reads the tree (`lurq_snapshot` of the MCP server) it is a `combobox`, called by its label where it has one.
- A renderer that draws another select starts from `view.select_style()`, the engine's `SelectStyle` the recipe makes, changes what it likes (another chevron, a menu as wide as its widest option), and builds it with `view.select(style)`, the engine's select bound to the value with the trigger's size, opacity and cursor.

## For `CHANGELOG.md`

- `ui::Select`: one value chosen from a list that opens under a trigger shaped like an input. The engine's select dressed by a recipe, bound to a `Signal<T>` with `(value, label)` pairs or `SelectOption`s, with the sizes of an input, `placeholder`, `invalid`, `full_width` and `disabled`, the parts `Trigger`, `Value`, `Placeholder`, `Chevron`, `Menu`, `Item`, `Check`, `Detail` and the states `Open`, `Invalid`, `Disabled`, `FullWidth`, `Chosen`, `ItemDisabled` for a recipe, and `SelectView::select_style()` and `select(style)` for a renderer. The engine's `SelectPartStyle` has an `outline`: a ring around a select's trigger.

## Decided

- The builder is not generic: `Select::new(value, options)` takes the signal and the options together, as
  `RadioGroup::new` does, and keeps the engine's select to be built (a closure over `Signal<T>`), so `Select`,
  `SelectView` and the theme's renderer are one type for every `T`. The options are the engine's `SelectOption<T>`,
  re-used rather than a `Choice<T>`: they have the detail line, the elements beside the label and the disabled
  flag the engine draws.
- What the board draws as the chosen item, the highlight's fill and a check, is two things: the highlight is the
  item under the pointer or the keys (the board's Menu draws `accent` on such an item), the check is the chosen
  one. On opening, the keys' highlight is on the chosen item, which is the board's picture.
- The item under the keys looks as the one under the pointer (the engine does that when `option_highlighted` is
  not set); a recipe has no separate interaction for it.
- The open look of the trigger is a state of the recipe (`Open`), resolved apart and given to the engine as
  `trigger_open`: the whole trigger as the open case has it, merged last, so it wins over the hover and the focus.
- `Menu`'s gap is the room between the trigger and the list (`menu_gap`), and `Item`'s gap the room before the
  check and beside an option's elements: the parts have no other use for a gap.
- The text of the trigger is the trigger's type (as the input's control has it), with `Value` and `Placeholder`
  over it; the sizes change the trigger's height and font size, as the input's.
- The disabled option is at the board's 0.45, a number of the board's; the whole select disabled is the theme's
  `OpacityLevel::Disabled` (0.5), which the board also draws.
- No `on_change`: the select is bound to the app's signal, as the engine's is. An app that wants to be told
  watches the signal (the gallery's playground does, with `ctx.watch`).
- The engine's select is a combobox to the MCP tree already; the recipe adds only `a11y_name` from `label`.

## Not done

- Groups in the list with a label over each and separators between them (the board's "Groups · disabled option ·
  description"): the engine's select has a flat list of options.
- The board's trailing text of an option ("beta", "soon" in the mono type): an option's `trailing(element)` can
  carry it, but the recipe has no part for it, and the gallery does not show it.
- `Combobox`, `Multi Select`, `Listbox`, `Cascader`, `Tree Select`: not asked for. `components::Select::multiple`
  exists in the engine; a multi select of the design system would be another component.
- The panel's height is the engine's most, 240, then it scrolls; the board does not say. A recipe cannot change
  it (`PartStyle` has no greatest height); a renderer can, with `select_style().max_menu_height(..)`.
- A select in a `Field` is not named by the field's label (the builder has no `Ctx`; the same holds for an input).
- Tests were written, not run (the batch is tested when merged). Most likely to need a look:
  `a_press_opens_the_list_under_it_and_a_press_of_an_item_chooses_it` (the exact place of the panel, of the
  items' text and of the check: the panel's edge and fill may be one quad or two, and the check's glyph is found
  by the theme's character for `check`), and `the_keys_open_the_list_move_through_it_and_choose`.
- The gallery's hand-made `select` in `crates/gallery/src/kit.rs` (its playground panels) is left as it was.

## Differs from the board

- The room between the value and the chevron is the engine's 8, which is the board's gap too, but it is not the
  recipe's to change.
- The trigger's ring while open is the focus ring of the input (an outline of 2 inside the box); the board draws a
  stroke of 2, which looks the same.
- An item's corners are `radius-sm` on every item; the engine rounds the first and the last item to the panel's
  corners only where an item has no radius of its own, so this does not change the board's look.
- The check is the theme's `check` icon in the color the recipe gives (`Foreground`), drawn by the engine in a slot
  of 16 that every item keeps at its end, so a label does not reach the panel's padding even where the item is not
  chosen; the board lets the label run there.
- The cursor over the trigger is a pointer, the default while disabled; the board does not say.
