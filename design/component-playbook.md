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
5. `design/handoff/<name>.md`, a new file with exactly these headings in this order, which a script reads:
   `## For `docs/src/content/docs/design-system.md`` (under it the section as it is to stand in the docs,
   starting at its own `## Name` heading, in the form of the `## Switch` section; no fence around the whole
   of it), `## For `CHANGELOG.md`` (one line starting with `- `), `## Decided`, `## Not done`,
   `## Differs from the board`. A change outside your own files is named under a first heading
   `## Changed outside the component's own files`. Whoever integrates the batch puts these in place.

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

## "Not done" is work, not a record

What you leave undone is done by whoever merges the batch, before the batch counts as merged: nothing is kept as a known gap. So build everything the board draws for your component and everything a person would expect of it (the keyboard, a disabled state, a group of it where the board lays several out as one). Put under `## Not done` only what you could not build, each with what stands in the way and the smallest change that would remove it: "needs the engine to ..." is a task for the merge. Other components of the board that your brief told you to leave out are not "not done": do not list them.

The engine has what earlier batches needed: `FocusGroup` (`docs/src/content/docs/focus-navigation.md`, Focus Groups) for arrow keys within a group, with Tab stopping in it once; use it wherever the board has several things that are one choice or one bar.

## A component that must keep something

A builder has no `Ctx`. What is open, what is chosen, what is typed is the app's: a `Signal` or an
`ElementRef` the builder is given (`Popup::new(anchor, content).open(signal)` is the engine's own example).
Where that is not enough, a timer of its own, a list it keeps, a host that shows what is sent to it from
anywhere, the component is a mounted `Component` (`crates/lurq/src/app/component.rs`, `ctx.mount::<C>(props)`)
with the recipe and the parts of any other; say in the handoff why a builder would not do.

## What the first batch taught

- The engine lays the style of the pointer over the style of the focus (`Node::state_style`: focused, then
  focus-visible, then hovered, then pressed). A focus drawn by a property the hover sets too is lost under
  the pointer: draw the focus as an outline (`focus_ring()`, or `Outline::new(2.0, color).offset(-2.0)` for
  a ring inside a box), not as a border.
- A text input is focus-visible however it was focused; a button only from the keyboard.
- In a test, compare a position that is a sum of fractions with a tolerance, and find the text of a
  multiline input by its start: it is not painted a quad for each line of the value.
- In `pages/mod.rs` add your `mod` line and your `Page { .. }` entry each on lines of their own, the entry
  after the last one of your group: every session of the batch adds one there.

## What the second batch taught

- **A layer is painted only inside the root's box.** The headless stage lays a scene out with loose constraints,
  so its root is as small as its content, and a popup, a modal, the list of a select, a tooltip or a toast that
  falls outside it is not painted and not found. A test's scene with a layer has a root that takes a size of its
  own (`Column::new().size(400.0, 300.0)`), as a window gives its root the whole window. Most of the failures of
  the batch were this.
- **Two sessions built the same change of the engine** under two names, and it had to be made one afterwards. Your
  brief says which session owns which kind of engine change. What you need of another's kind you write under
  `## Not done` with the smallest change that would give it; you do not build it.
- **Built-in keyframes have identifiers of their own**: `0xFF00` to `0xFF02` are the spin and the two fades
  (`ui::SPIN`, `ui::FADE_IN`, `ui::FADE_OUT`), `0xFF11` to `0xFF14` the toast's. Take the next free ones after the
  last registered in `AnimationEngine::new` (`crates/lurq/src/animation/animation.rs`) and name them in the
  handoff: two with one identifier overwrite each other silently.
- **A panel in a layer is as wide as its content or as the recipe says, never as the layer offers**: a part with a
  growing child fills the window. Give the panel a width or a `max_width`, and test its width.
- **When the merge reports your tests failing you run them yourself**, your own file only
  (`cargo test -p lurq --features form,router --test ui_tests <name>`), and fix the component where it is the
  component: a test is rewritten to assert what a person would see, not loosened until it passes.

What the engine and `lurq::ui` have since, for you to use and not build again: `FocusGroup` with `typeahead`,
`follow_pointer` and `.focus_label(..)`; `Overlay::takes_focus` and `Modal::takes_focus`; `.focuses(&ElementRef)`
(a press of one element focuses another); `ElementRef::focus_visible_within()`; a disabled element hearing the
pointer come and go; `PartStyle` with a padding for each side, `max_width`, `enter` and `exit`; `FieldControl`
(a control a `Field` names, judges and disables: implement it for a control a form holds); `ui/text_box.rs` (what
a box that is typed in or chosen from has); `ui::Menu`, `ui::Popover`, `ui::Tooltip` for what your component
opens; `FocusGroup::none()` around what a group holds beside what its keys move between; a transition of `Height`
between a number and none, for what opens with the height of its content (`ui::Collapsible` is one); the focus
staying in a group when the element it is on is disabled; `PartStyle::start_line`; a modal over the whole window
starting under the window's own title bar.

## What the third batch taught

- **A line of one part under a line of another**: a part's `underline` is drawn inside its box, over what the part
  holds where the part clips. A line a child must draw over (the selected tab's over the bar's) is on a box that is
  under the children, a first child of a `Stack`, and not on the box that holds them.
- **A text beside something else in a row is as wide as it is** and is cut, not wrapped: put it in a column of the
  room it has, with a width that fills (`Column::new().width(fill).child(text.width(fill))`).
- **The handoff's section for the docs is plain markdown**, with no fence around the whole of it, of backticks or
  of tildes: a `~~~` left after a section turned the rest of the docs page into one code block, and no test saw it.
- **A transition runs from the value the element had while it had the transition.** An element drawn first with
  no transition and then with one and another value is at the other value at once: the toast's timer never ran. What
  is to move has its transition from its first render, of no duration where it must stand still.
- **A part that clips inside a rounded part is cut to a plain rectangle**: the clip of the inner one replaces the
  round corners of the outer where they meet. What lies along the edge of a rounded panel (a rail at its bottom, a
  picture at its top) is `overflow_visible()` and is cut by the panel alone.
- **A scrolling part does not grow**: `flex_full(0.0, 1.0, None)`, so that it gives way when there is too little
  room and is as high as its content when there is enough. One that grows makes its panel as high as the window.

## What the fourth batch taught

- **A stack and a column give a child no more than their own width**; only a row lays a child out as wide as it
  is, from its start. What is centred on a point whatever its size is in a row at the point and moved back by
  half of itself: `.offset_by_size(-0.5, -0.5)`. What has no size paints nothing of what is in it.
- **A test reads what moves where its move has it.** Keyframes registered in phase run on the process's clock, so a
  turning glyph is at another angle in every run of the suite: assert what holds at any moment (its distance from
  the middle it turns about), or a difference between two moments, never one pose.
- **A playground offers a control only for what it changes**: a prop that one form of the component has is there
  while that form is chosen. A component that is read and not operated gives `event: ""`, and the stage says so.
  One that holds the app's own buttons (an outcome's actions, a fieldset's) is operated by them: their presses are
  recorded as `on_click("Deploy")`, and nothing on a stage is pressed without the Events tab hearing of it.
- **What a person tells apart, the app's value tells apart.** A token and what is being typed look different, so
  they are two things to the app: a list and a draft, each with its own event, not one text that holds both.
- **The stage of a playground cuts nothing off**, and a page's own wrappers must not either where a shadow, a ring
  or a badge reaches past the component: `overflow_visible()` on what holds it.
- **What a batch may use now**: `BackgroundColor::CurrentForeground`; a width in percent run by a keyframe or a
  transition; `Transition::border_dash()`; `PartStyle::corners`; `.stops_inside(true)`; `ui::Badge`, `ui::Tag`,
  `ui::Chip`, `ui::Avatar`, `ui::Card`, `ui::Separator`, `ui::Progress`, `ui::Spinner`, `ui::Skeleton` in what your
  component is made of, and in the gallery's pages in place of anything drawn by hand.

## What the seventh batch taught

- **A scene made for a stage fills it.** The stage scrolls what is larger than it, but a scene of a fixed width
  wider than the stage at a window of 1280 by 800 is a scene made wrong: give it `fill: true` and let what is in
  it share the stage's width.
- **An app's own part moved onto a component keeps its look.** The component is the board's; where the app's part
  looked another way (the gallery's side bar on the canvas, the board's on `Muted`), the app lays a recipe over the
  component, and the brief says which look is kept.
- **A color the engine draws is the theme's.** A scroll bar's thumb and a selection were literals of the engine and
  wrong on a dark theme: nothing lurq draws has a color that a palette does not give it.
- **A session writes the docs of what it gave the engine**, in the page an app's author looks in
  (`focus-navigation.md`, `layout.md`, `styling-events.md`), with a line of the changelog for each, and not only in
  its handoff: the handoff is deleted at the merge.
- **What a batch may use now**: `FocusGroup::nested`, `menu_row` and `FocusRows`; `.inert(bool)`;
  `.interaction_group(true)` with `.within_style(..)`; a row's drop marker; a held drag; `ui::Sidebar`,
  `ui::Menubar`, `ui::Toolbar`, `ui::ActionBar`, `ui::TreeView`, `ui::Splitter`, and `TokenInput::list`.

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
- A key event reaches every node, focused or not, so a component does not handle the arrow keys itself:
  `.focus_group(FocusGroup::..)` on what holds its items makes the runtime move the focus within it.
- A builder has no `Ctx`: no signals, no element refs that outlive a render, no focus requests. State is the
  app's, given by `on(bool)` / a `Signal` and told back by a handler, as `Switch` and `ToggleGroup` do.
- A press of the pointer reaches a node with `on_click`; `describe("a11y_role", ..)`, `"a11y_name"` and
  `"a11y_checked"` say what the node is to tooling that reads the tree.
