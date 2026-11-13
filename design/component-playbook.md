# Building one component of the design system

For a session that is given one component (or two that go together) to build on a branch of its own, while
other sessions build its siblings. Read this first, then the reference implementation, then the board.

## What you are given, and where things are

- **The design**: `pencil/lurq-design-system.pen`, read with the Pencil MCP server (`mcp__pencil__execute`
  with `Get`, `Print`, `TakeScreenshot`, `Export`; `mcp__pencil__read_skill` path `execute.md` for its API).
  Use the URI `file:///H:/projects/lurq/pencil/lurq-design-system.pen`: that file is open in the user's
  Pencil. **Read only: never `Insert`, `Update`, `Replace`, `Move`, `Delete` or `SetVariables`.** The board
  is the authority for what the component is: its sizes, gaps, roles of color, states. Read the reusable
  component by its id, find where it is used (`Get` with a visitor for `n.type === "ref" && n.ref === id`),
  and read the states the board draws around it. Variables (`$primary`, `$input`, `$radius-full`) are the
  theme's roles: `design/tokens/variables.tsv` maps them.
- **The reference implementation, to copy the shape of**: `Switch`.
  - `crates/lurq/src/ui/switch.rs`: the builder, `SwitchPart` and `SwitchState` with `names!`, `SwitchView`
    with `part()`, the behavior (`track()`), `render()`, and `impl Themed` with the recipe.
  - `crates/lurq/tests/ui/switch.rs`: tests on the headless `Stage` (`crates/lurq/tests/ui/stage.rs`).
  - `crates/gallery/src/pages/switch.rs`: its page, with a playground.
  - `Toggle` (`ui/toggle.rs`) and `Button` (`ui/button.rs`) are the references for sizes and variants;
    `ToggleGroup` and `SegmentedControl` for a component that holds choices bound to a `Signal`.
- **The machinery**: `crates/lurq/src/ui/recipe.rs` (`Recipe`, `Selector`, `Case`, `ResolvedPart`),
  `part_style.rs` (`PartStyle`: every field a part can be styled with), `shared.rs` (`pressable`, `fading`,
  `glyph`, `Choice`, `Chosen`), `theme.rs` (`Themed`). `docs/src/content/docs/design-system.md` explains
  them. `design/design-system-plan.md` sections 14.5 to 14.8 say what was decided and why.
- **What the engine already has**: `crates/lurq/src/components/` (`TextInput`, `Checkbox`, `Slider`,
  `Select`, `Popup`, `Tabs`, ...). A component of the design system that the engine has a node for is that
  node dressed by a recipe, not a second implementation of its behavior. Read the engine component and its
  docs page in `docs/src/content/docs/` before deciding the anatomy.

## What a component is

1. `crates/lurq/src/ui/<name>.rs`: a builder that converts into an `Element` in the theme of what renders
   it; an enum of parts and one of states (and of variants or sizes where the board has them); a `View` a
   renderer is given; `impl Themed` with the board's recipe. Every color, size, radius and duration is a
   role or a step of the theme, or a number the board gives, in the recipe: none in the renderer.
2. Registered in `crates/lurq/src/ui/mod.rs` (`mod`, `pub use`).
3. `crates/lurq/tests/ui/<name>.rs`, registered in `crates/lurq/tests/ui/mod.rs`: what it paints at rest
   against the board's numbers (sizes, fills by role, positions), each state, what a press or a key does,
   that a disabled one takes nothing, the focus ring from the keyboard (press Tab, then act, and assert on
   what acted: a ring alone proves nothing), and that a theme's renderer or rule changes it.
4. `crates/gallery/src/pages/<name>.rs`, registered in `crates/gallery/src/pages/mod.rs` in the group the
   board has it in: a `Playground` first (every prop a control), then sections with `self.frame.demo(n,
   stage, call, code)` whose `code` is the Rust that builds exactly what the stage shows, then
   `props_table(&rows, &[required], self.frame.room())`.
5. `design/handoff/<name>.md`, a new file: the section for `docs/src/content/docs/design-system.md` (in
   the form of its `## Switch` section), one line for `CHANGELOG.md`, and a short "decided / not done /
   differs from the board" list. Whoever integrates the batch puts these in place.

## Rules

- **Do not edit** `CHANGELOG.md`, `design/design-system-plan.md`, `docs/`, the board, or another
  component's files. The three `mod.rs` files get your lines added and nothing else changed. A change the
  component needs in shared code (`shared.rs`, `part_style.rs`, `recipe.rs`, the engine) is made as small
  as it can be and named first in the handoff file.
- **Do not run tests.** The batch is tested when all of it is merged. Make it compile and nothing more:
  `cargo check -p lurq --features form,router --test ui_tests` and `cargo check -p gallery`. No
  `cargo build`, `cargo test`, `cargo clippy`, and no running the gallery: several sessions compile at once
  on one machine.
- Format what you changed: `rustfmt --edition 2024 --config-path rustfmt.toml <files>`.
- Commit on your branch when it compiles. Do not push, do not open a pull request, do not merge.
- A defect you find in the engine or in another component is written in the handoff file, not fixed, unless
  your component cannot be built without the fix.
- Code, comments, the handoff file and the commit message are in English. Comments say what the code cannot
  say for itself, in the plain manner of the files around them.

## What the engine does that surprises

- A container clips what is in it (`Overflow::Hidden` is the default): a shadow or a ring outside a part
  needs `.overflow_visible()` on what holds it, or room.
- A plain `components::Button` is a stop of Tab only in a form or with a `tab_index`; `shared::pressable`
  gives a control of the design system `tab_index(0)`. Use it for what is pressed.
- A state style (`hovered`, `pressed`, `focus_visible` in a recipe) changes what is painted, never a size.
- A part cannot be styled by the state of another part's node: the ring of the focus is on the node that is
  focused. Choose which part is the focusable one by where the board draws the ring.
- A transition runs from one value to another, not from none: a part that slides has an offset in both
  states (`SwitchView::thumb`).
- `Length::Fill` as a height in a parent whose height is its content's is nothing: use
  `align_items(Stretch)`.
- A key event reaches every node, focused or not: a component cannot yet tell that the focus is inside it,
  so arrow keys within a group are not built. Say so in the handoff if the board implies them.
- A builder has no `Ctx`: no signals, no element refs that outlive a render, no focus requests. State is the
  app's, given by `on(bool)` / a `Signal` and told back by a handler, as `Switch` and `ToggleGroup` do.
- A press of the pointer reaches a node with `on_click`; `describe("a11y_role", ..)`, `"a11y_name"` and
  `"a11y_checked"` say what the node is to tooling that reads the tree.
