# Steps: hand-off

`ui::Steps` and `ui::Step`, from the board's `Steps` (`rzgls`) in Navigation of Components / light (`Ovt1D`,
"Progress"), used once in Components / dark (`KqNY7`). The board draws one row of three steps (done, current, to
come) and the lines between them, and nothing else of it: no description, error, column, hover, focus or disabled.
The Wizard window of the Examples (`QgDXO`) draws steps in a column, without lines. `Mobile/Step Progress` (`lbQqq`)
was read and not built. What lurq has for what the board does not draw is under "Decided" and "Differs from the
board".

Files: `crates/lurq/src/ui/steps.rs`, `crates/lurq/tests/ui/steps.rs`, `crates/gallery/src/pages/steps.rs` (page
"Steps", group Navigation), and lines in the three `mod.rs` files. Nothing in shared code or the engine was changed.

## For `docs/src/content/docs/design-system.md`

## Steps

The stages of a process in order, with one current: those before it are done, those after it are to come.

```rust
use lurq::ui::{Step, Steps, StepsOrientation};

// Read, not operated: the app moves the current step itself.
Steps::new(1, ["Account", "Workspace", "Finish"]);

// A wizard that lets a person go back to a step that is done. at: Signal<usize>, which a press sets.
Steps::new(&at, [
  Step::new("Account").description("Name and email"),
  Step::new("Billing").description("Card and address").error(billing_failed),
  Step::new("Finish"),
])
.orientation(StepsOrientation::Vertical)
.label("Sign up")
.on_select(|index| println!("back to step {index}"));
```

| Builder | |
| --- | --- |
| `Steps::new(current, steps)` | `current` is the index of the current step, from 0, or a `Signal<usize>` (or `&Signal<usize>`) that a press of a step sets. Past the last step every step is done. A step is a `&str`, a `String` or a `Step` |
| `orientation(StepsOrientation)` | `Horizontal` (the default): in a row, a line between each step and the next. `Vertical`: in a column, a line down from each step's mark to the next |
| `on_select(Fn(usize))` | Called with the index of a step pressed, after a signal given as `current` is set. Makes the steps that are done pressable |
| `pressable(bool)` | Makes the steps that are done pressable with no handler: for steps bound to a signal |
| `ahead(bool)` | Where steps are pressed, a step to come is pressed too: a process whose steps can be taken in any order |
| `label(&str)` | What the list of steps is called to what reads the tree |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the step Tab would enter at in the state. With an `id`, each step has the id `<id>-<index>`, its mark `<id>-<index>-marker`, and the line after it `<id>-<index>-line` |
| `Step::new(title)` | A step. `description(&str)` a line under the title, `icon(name)` an icon of the theme's set in place of its number until it is done, `error(bool)` that it has gone wrong |

- Steps are read, not operated, unless the app says they are pressed (`on_select` or `pressable`). Then a step that is done is pressed to go back to it, and a step to come only with `ahead(true)`; the current step selects nothing. The steps are one stop of Tab, entered at the current step (at the last one pressed where every step is done); the arrow keys of the orientation, Home and End move between the current step and those that are pressed, and Space and Enter press. Steps that are only read take no press and no focus.
- Parts: `StepsPart::Root` (the steps and the lines), `Step` (a step: what is pressed and focused), `Marker` (its round mark), `Number`, `Icon` (the check, the cross, or the step's own icon), `Body` (the title and the description), `Title`, `Description`, `Rail` (in a column, the mark with the line down from it), `Connector` (what holds a line) and `Line`. Each step's parts are styled in that step's states: `StepsState::Done`, `Current` or `Upcoming`, and `Error`, `Pressable`; the whole has `Disabled`. The orientation is the recipe's variant (`StepsOrientation::Horizontal`, `Vertical`).
- In the design the mark is 24 and round: in `Primary` with a check of 14 in `PrimaryForeground` once the step is done, in `Primary` with its number (12, medium) while it is current, and with a 1 px edge in `Input` and the number in `MutedForeground` while it is to come. The title is 8 beside the mark in the label's type (13, medium), `Foreground`, and `MutedForeground` while the step is to come. A line of 1 runs from 12 after one step to 12 before the next, in `Primary` up to the current step and in `Border` after it; in a row the lines share what the steps leave of the width. A description is under the title in the caption's 12, `MutedForeground`. A step in error has a cross on `Destructive` and its title in `Destructive`, whatever else it is.
- A pressed step has a veil of `StateHover` under the pointer and of `StatePressed` held down; the ring of the focus is around the step, which is padded 4 for it. Disabled steps are dimmed and take nothing.
- To what reads the tree (`lurq_snapshot` of the MCP server) the root is a `list` (called by `label`), each step a `listitem`, or a `button` where it is pressed, called by its place, title and state ("Step 2 of 4, Billing, current", with ", error" where it is in error); the current step has `a11y_current` `step`.
- A renderer that draws other steps has `view.steps` (each step with its `status`, whether it is `pressable` and its resolved recipe), `view.step_part(index, part)`, `view.step(index, content)` (a step with its behavior and its name to tooling), `view.marker(index)`, `view.body(index)`, `view.connector(index)` and `view.step_name(index)`. A bar of segments, as the mobile skin draws progress, is such a renderer: a `Line` of each step, and the current step's title under them.

## For `CHANGELOG.md`

- `ui::Steps`: the stages of a process in order with one current, the steps before it done with a check and a line in the primary color up to it; a description, an icon or an error on a step, in a row or in a column, and steps that are done pressed to go back to them, with Tab and the arrow keys.

## Decided

- **A builder.** The current step is the app's, an index or a `Signal<usize>`, as `ToggleGroup` takes its value; nothing else is kept.
- **Pressed only where the app says so.** A wizard decides whether a person may go back, so steps are read, not operated, until `on_select` or `pressable(true)` is given; then the steps that are done are pressed, and steps to come only with `ahead(true)`. The current step is focusable in steps that are operated, as where Tab enters them, but a press of it selects nothing: the process is already there.
- **One stop of Tab** (`FocusGroup::horizontal()` or `vertical()` by the orientation, `one_stop(true)`, the current step its `focus_entry`), as a toolbar or a list of tabs: a wizard's steps are one bar, and the arrow keys move along them. The focus does not select (`activate` is off): going back to a step is a decision, not where the focus passes.
- **Each step resolved in its own case.** A step's state decides its mark, its title and the line after it, so the recipe is resolved once for each step with that step's states (and once for the whole). The line after a step is styled in that step's states: `Done` makes it `Primary`, which is the board's line up to the current step.
- **What the mark shows**: a cross (`icons::CLOSE`) where the step is in error, whatever else it is; a check where it is done; the step's icon where it has one; its number otherwise. A step's own icon gives way to the check once the step is done, so that what is done reads the same in every step.
- **A step is padded 4**, with the radius `Md`: room for the ring of its focus and for the veil of the pointer. The whole is 8 from its lines instead of the board's 12, so the line is still 12 from the mark and the title. The root does not clip, so the ring of a step at either end is drawn.
- **The title is as high as the mark** (24), its text level with the mark's middle: a description then goes under the title and beside nothing, in a row and in a column alike, and the line in a row (in a connector of 32, the step with its padding) stays level with the mark.
- **In a column** each step's mark has a line running down from 4 under it to the bottom of the step, as long as the step's text is high (12 at least: two steps without descriptions are 24 apart). The last step has none.
- **In a row the whole is as wide as it is given** (`Length::Fill`): the lines take what the steps leave, as the board's lines are `fill_container`. Steps in a row whose parent has no width lay out with lines of nothing.
- **Names to tooling** say the place, the title and the state in words ("done", "current", "to come", and ", error"), so a reader that does not know `a11y_current` still hears where the process is.
- A description is in the caption's type (12) and `MutedForeground`, and the error in `Destructive`, `DestructiveForeground` on the mark: the roles a description and an error have elsewhere in the system (`Field`).
- **The mobile skin's `Mobile/Step Progress`** (a bar of four segments of 4, `Primary` up to and with the current step and `Secondary` after it, gap 4, and "Step 2 of 4 · Workspace" under it in 13 `MutedForeground`) is a renderer and a recipe of its own over these parts: a segment is the `Line` of each step drawn for every step and not between two, which a recipe alone cannot do (it styles nodes, it does not change how many there are). `StepsView` gives that renderer everything it needs; nothing of it was built, as the brief says.

## Not done

Nothing. The tests in `crates/lurq/tests/ui/steps.rs` were written and compiled, not run, as the batch's flow has it.

## Differs from the board

- The board draws no description, error, icon in place of a number, column, hover, focus or disabled: what lurq has for them is under "Decided".
- A step is padded 4 for its ring and its veil, and the whole is 8 from its lines: the marks, titles and lines are where the board has them relative to each other, but the whole is 4 longer at each end than the board's 460 holds them.
- The Wizard window of the Examples draws steps in a column with marks of 22, numbers of 11 at 600, the current title at 600, 18 between steps and no lines. lurq's column is the component's: marks of 24, the board's weights, and a line down from each mark to the next.
