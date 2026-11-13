# Checkbox: handoff

`ui::Checkbox`, from the board's `Checkbox` (`dhuPb`) in "Checkbox, radio and switch" of Components / light
(`Ovt1D`). `Checkbox Card` (`lCj7S`) is not built.

## Outside this component's files

Nothing but the three registrations: `mod checkbox` and its `pub use` in `crates/lurq/src/ui/mod.rs`,
`mod checkbox` in `crates/lurq/tests/ui/mod.rs`, and the page in `crates/gallery/src/pages/mod.rs` (group
Inputs, before Switch). No shared code was changed.

## For `docs/src/content/docs/design-system.md` (after `## Switch`)

~~~markdown
## Checkbox

A choice that is made or not, among others a form takes at once: send crash reports, keep me signed in, a row of a list picked.

```rust
use lurq::ui::Checkbox;

Checkbox::new("Send crash reports")
  .checked(reports.get())
  .on_change(move |checked| reports.set(checked));

// The box over a list of which some are picked: a press of it picks them all.
Checkbox::new("Select all")
  .checked(picked == files.len())
  .indeterminate(picked > 0 && picked < files.len());

// The box alone, in a row of a table that has the text already.
Checkbox::new("Select blur.rs").label_hidden(true).checked(row.get());
```

| Builder | |
| --- | --- |
| `checked(bool)` | Whether it is checked. The checkbox does not keep it: the app says so |
| `indeterminate(bool)` | Some of what it stands for is checked: a dash, whatever `checked` says |
| `on_change(handler)` | Called on a press with what the checkbox is to be: checked if it was not, or was indeterminate |
| `invalid(bool)` | What it holds is wrong: the box is edged in the destructive color |
| `label_hidden(bool)` | The box alone. The label is still what the checkbox is called |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box |

- A press of the box or of the label asks for the other state; a press of an indeterminate one asks for checked. Tab reaches the box, Space and Enter press it, and the focus ring is the box's.
- Parts: `CheckboxPart::Root` (the box and the label, side by side), `Box` (what is pressed), `Indicator` (the check or the dash), `Label`. States: `CheckboxState::Checked` (checked and not indeterminate), `Indeterminate`, `Invalid`, `Disabled`.
- In the design the box is 16 with the small radius, on the `Background` with an inner edge of 1 in `Input`; checked or indeterminate it is filled and edged in `Primary`, with the theme's `check` or `minus` icon of 12 (`IconSize::Xs`) in `PrimaryForeground`. An invalid box is edged in `Destructive`, checked or not. A disabled checkbox is dimmed with its label, and its box is `Muted` while it is empty. The label is 8 beside the box in the body's type.
- To what reads the tree (`lurq_snapshot` of the MCP server) the box is a `checkbox` called by its label, with `a11y_checked` `true`, `false` or `mixed`.
- A renderer that draws another checkbox starts from `view.control()`, the behavior with the box's style, and puts in it what it likes; `view.indicator()` is the default mark, `None` while the checkbox is empty.
- `lurq::components::Checkbox` is the engine's node, which keeps its value in a `Signal` and draws its mark from an image; `lurq::ui::Checkbox` is the design system's and does not use it.
~~~

## For `CHANGELOG.md`

~~~markdown
- `ui::Checkbox`: the design system's checkbox, checked, indeterminate, invalid and disabled, with its label beside it; its state is the app's (`checked`, `indeterminate`, `on_change`).
~~~

## Decided

- **A pressable box of its own, not the engine's `Checkbox` dressed.** A recipe cannot style every part of
  the engine's node: its mark is only an image (`CheckboxStyle::indicator_image`, under `raster`), so the
  theme's check glyph and its color cannot be given to it; it has no indeterminate state and no dash; its
  styles take colors, not the theme's roles, and have no transitions; it has no label; and it keeps its
  value in a `Signal` it toggles itself, while the design system's state is the app's. So the checkbox is
  built as `Switch` is: `shared::pressable` for the box, `shared::glyph` with `icons::CHECK` /
  `icons::MINUS` for the mark.
- An indeterminate checkbox draws the dash whatever `checked` says, is not in the `Checked` state, and a
  press asks for `true`, as browsers do.
- Invalid and checked: the board draws invalid only empty. The destructive edge is kept over the primary
  fill, so that the box still says it is wrong once it is checked (the `Invalid` rule is written after the
  `Checked` one, at the same rank).
- Disabled and checked: the board's "Managed by policy" row is a checked box at half opacity, and its
  "disabled off" box is muted. A recipe has no case for a state that does not hold, so the disabled rule
  mutes the box and two rules of `Checked + Disabled` and `Indeterminate + Disabled` fill it again.
- `invalid(bool)` is a prop of the checkbox, not only a name for `state(..)`, because the board draws it.

## Not done

- No hover or pressed style: the board draws none for the checkbox, and `Switch` has none either.
- No group of checkboxes (a `Signal<Vec<T>>` of the picked ones, as `ToggleGroup` has): the board has none,
  and the gallery's "Over a list" section builds one from plain checkboxes.
- Tests are written (`crates/lurq/tests/ui/checkbox.rs`) and compile; per the playbook they were not run.

## Differs from the board

- **The focus ring** is the theme's (`focus_ring()`: 2 wide, 2 from the box), as `Switch`'s is; the board
  draws a stroke of 2 right on the box's edge, with no gap.
- **The weight of the mark** is the icon font's: the board overrides the check's stroke to 1 in its 12 px
  instance; the font's `check` and `minus` are drawn at the weight of every other icon.
- The board dims a disabled box with an opacity on the box itself ("disabled off" in States); the build
  dims the root, box and label together, as the board's "Managed by policy" row does. Alone, the result is
  the same.
