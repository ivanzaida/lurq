# Radio and RadioGroup: hand-off

Built on `claude/ds-radio-c2a2a4`, cut from `feat/design-system`. Checked with
`cargo check -p lurq --features form,router --test ui_tests` and `cargo check -p gallery`; no test was run.

## Outside the component's own files

Nothing in shared code. Only the three `mod.rs` files got lines:

- `crates/lurq/src/ui/mod.rs`: `mod radio;` and the `pub use radio::{..}`.
- `crates/lurq/tests/ui/mod.rs`: `mod radio;`.
- `crates/gallery/src/pages/mod.rs`: `mod radio;` and the page "Radio" in the group Inputs, before Switch: the
  board's heading is "Checkbox, radio and switch", so Checkbox goes before Radio when it lands.

## For `docs/src/content/docs/design-system.md`, after `## Switch` (or between Checkbox and Switch)

````markdown
## Radio

One of a few choices that exclude each other, all of them in sight: the density of a list, the plan of an
account. `RadioGroup` is the choice; `Radio` is one radio, for a layout a group does not make.

```rust
use lurq::ui::{Choice, Radio, RadioGroup};

// The radio whose value the signal has is chosen; a press of another sets it.
RadioGroup::new(density, [(Density::Comfortable, "Comfortable"), (Density::Compact, "Compact")]);

// A choice that cannot be made, and a group the app holds the choice of: none until one is pressed.
RadioGroup::new(plan, [Choice::new("free", "Free"), Choice::new("enterprise", "Enterprise").disabled(true)]);
let set = answer.clone();
RadioGroup::controlled(answer.get(), [(true, "Yes"), (false, "No")], move |yes| set.set(Some(yes)))
  .invalid(answer.get().is_none());

// One radio alone.
Radio::new("Compact").chosen(true).on_choose(|| println!("compact"));
```

| `RadioGroup` | |
| --- | --- |
| `new(signal, choices)` | The radio whose value the signal has is chosen, and a press of another sets the signal |
| `controlled(Option<T>, choices, handler)` | The app holds the choice: none while it is `None`; the handler is told the value of each radio pressed that was not chosen |
| `invalid(bool)` | Every radio's edge in the destructive color: the choice was made wrongly, or not at all |
| `disabled(bool)` | The whole group takes no input. `Choice::disabled` takes one choice out |
| `style(part, ..)`, `id`, `key` | As a [toggle group](#toggle-group) has them |

| `Radio` | |
| --- | --- |
| `chosen(bool)` | Whether it is the choice made. The radio does not keep it: the app says so |
| `on_choose(handler)` | Called on a press while the radio is not chosen |
| `label_hidden(bool)` | The mark alone. The label is still what the radio is called |
| `invalid(bool)` | The edge in the destructive color |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the mark |

- A press of the mark or of the label chooses the radio; a press of the one chosen does nothing, so a choice
  once made is not taken back. Tab reaches each radio of a group in turn, Space and Enter press it, and the
  focus ring is the mark's. Arrow keys do not move the choice within a group yet.
- Parts: `RadioPart::Root` (the mark and the label, side by side), `Mark` (what is pressed), `Dot`, `Label`.
  States: `RadioState::Chosen`, `Invalid`, `Disabled`. A group's parts: `RadioGroupPart::Root` (the radios
  one under the other) and `Item`, laid over the theme's recipe for each radio's root.
- In the design the mark is 16 and round, in `Background` with an edge of 1 inside it in `Input`; chosen, the
  edge is `Primary` and a dot of 8 in `Primary` is in its middle. The edge of an invalid one is
  `Destructive`. The label is 8 beside the mark in the body's type, and a group's radios are 12 apart. A
  disabled radio is dimmed with its label.
- A choice's icon is not drawn: a radio is its mark and its label.
- To what reads the tree (`lurq_snapshot` of the MCP server) the mark is a `radio` called by its label, with
  `a11y_checked` saying whether it is chosen, and a group is a `radiogroup`.
- A renderer that draws another radio starts from `view.mark()`, the behavior with the mark's style, and puts
  in it what it likes; `view.dot()` is the default dot.
````

## For `CHANGELOG.md`, under Added, after `ui::Switch`

```markdown
- `ui::Radio` and `ui::RadioGroup`: one of a few choices that exclude each other. `RadioGroup::new(signal, choices)` is bound to a `Signal<T>` like `ToggleGroup::new`, `RadioGroup::controlled` takes the choice from the app (`None` for none yet); radios are one under the other, 12 apart, with `invalid` and `disabled` for the whole group and `Choice::disabled` for one. A radio is a round mark with a dot while it is chosen and a label that is pressed as the mark is; a press of the one chosen does nothing. The parts `Root`, `Mark`, `Dot`, `Label` and the states `Chosen`, `Invalid`, `Disabled` for a recipe, and `RadioView::mark()` and `dot()` for a renderer.
```

## Decided

- The two are in one file, `ui/radio.rs`, as a group and its item: `RadioGroup` builds a `Radio` for each choice
  with the rules of its `Item` part over the radio's root, the way `ToggleGroup` builds toggles.
- The press handler of a radio is `on_choose(Fn())`, not `on_change(Fn(bool))` as `Switch` has: a radio is
  only ever asked to become chosen. It is not installed on a chosen radio, so the app's handler and the
  signal are never told of a press that changes nothing.
- `controlled` takes `Option<T>` (where `ToggleGroup::controlled` takes an iterator of values): a group of radios
  has one choice or none, never several.
- `invalid` is built though the task named four states: the board draws "radio invalid" (an edge in
  `$destructive`) beside "radio disabled", and a form needs it for a choice that must be made.
- The focus ring is `focus_ring()` on the mark, as `Switch` has it on its track. The board draws no focused
  radio; it draws a focused checkbox with a stroke of 2 in `$ring`, which is the theme's ring.
- The mark's edge changes color over the theme's fast duration (`Transition::border_color()`, with what
  `fading` gives). The dot is there only while the radio is chosen, so it appears at once.
- Every radio of a group is a stop of Tab. With arrow keys a group would be one stop (the chosen radio) and the
  arrows would move the choice; without them, Tab is the only way the keyboard reaches the others.

## Not done

- **Arrow keys within the group.** A key event reaches every node whether it is focused or not, and a component
  cannot yet tell that the focus is inside it (`design/component-playbook.md`), so Up/Down/Left/Right moving
  the choice, and a group being one stop of Tab, wait for the engine to say where the focus is.
- A group in a row. The board draws radios only one under the other (Toggles, "Radios"), so there is no
  `orientation`; an app that wants a row lays out `Radio`s itself.
- `Radio Card` (`IhFJO`, Advanced Inputs), as asked.
- The tests do not assert `a11y_role` / `a11y_checked`: the tree a test has (`ElementHandle`) does not show an
  element's descriptions. `crates/lurq/tests/ui/radio.rs` has 10 tests, none run yet.
- A form field: `RadioGroup` is not bound to `form::Field` or its validation. Whoever builds `Field` decides
  whether a group goes in it as Input does.

## Differs from the board

- On the board a disabled radio in the States row is dimmed on its own (opacity 0.5 on the mark); here, as
  `Switch` and the board's disabled checkbox row ("Managed by policy") do, the radio is dimmed with its label.
