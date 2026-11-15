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
- **What is on the stage is built on the stage.** `Spec::shown` is a `Shown`: `Shown::of(component)` for a builder,
  which becomes an element in the stage's theme (the stage can be in the other appearance, and a text input's
  color or a slider's track is read when it is built), and `Shown::built(|ctx| ..)` for a scene of more than one
  thing. An element the page has already built keeps the page's colors.
- **The size under the stage is the component's.** A component is given to the stage bare: no column of the page's
  to make it full width (`fill: true` gives it the stage's width to fill, and it stands in the middle where it keeps
  a width of its own). A scene that must hold more than the component (a column that gives a skeleton its width,
  what a separator is between) puts `playground.measure(component)` around the component in it and gives the
  scene as `Shown::of(scene).measured()`.
- **A signal the component is bound to is counted with `playground.watch(ctx, &signal, |value| ..)`**, which
  counts changes only: a control that sets its value as it is built is no event. Reset empties the log.
- **Nothing a playground opens is left open.** A dialog that nothing in it closes (a progress dialog with no
  cancel) is closed by the stage after a moment: the window is never behind a veil with no way out.
- **A prose string is one line of source or ends its lines with `\`.** A string continued on the next line
  without it keeps that line's indentation, and the page shows a hole in the sentence.
- **What a batch may use now**: `BackgroundColor::CurrentForeground`; a width in percent run by a keyframe or a
  transition; `Transition::border_dash()`; `PartStyle::corners`; `.stops_inside(true)`; `ui::Badge`, `ui::Tag`,
  `ui::Chip`, `ui::Avatar`, `ui::Card`, `ui::Separator`, `ui::Progress`, `ui::Spinner`, `ui::Skeleton` in what your
  component is made of, and in the gallery's pages in place of anything drawn by hand.

## Props: one way to say each thing

A QA pass of 2026-11-14 read every builder and found one meaning under several names, one name with several types,
and props a family has but one of it lacks. These are the rules every component follows; a new component is checked
against them before its handoff. Nothing of `ui` was released when they were set, so a rename is a rename and not
an alias.

**What a thing is called**

- **The value changed: `on_change`**, told the new value of what the component holds: `on_change(bool)`,
  `on_change(usize)`, `on_change(f64)` for a plain value, `on_change(&T)` for a value of the app's type,
  `on_change(&[T])` for a set (the whole new set, not the one that was pressed). Every component that holds a value
  has it, bound to a signal or not. It is not called for what is already so (a press of the chosen segment).
- **A command was run: `on_select(Fn())`**, on an item of a list of commands: a menu's item, a palette's command,
  a link of a sidebar or a navbar, a breadcrumb's step, a toolbar's command. Not for a value.
- **A press: `on_click(impl IntoMouseEventHandler)`**, given the event, on what is a button, a link or a pressed
  card. A button made from `(label, handler)` takes `Fn()`.
- **An edit of text: `on_input(&TextInputEvent)`**, before the edit, which it may refuse. What a filter is asked
  for after an edit is `on_query(&str)`, beside `query(Signal<String>)`. `on_submit` is Enter in a field that has
  something to submit: Input, SearchField, Combobox.
- **While it moves and when it ends: `on_change` and `on_change_end`**, on what is dragged or stepped: Slider,
  RangeSlider, NumberInput. (A splitter's are `on_resize` and `on_resize_end`: it holds no value.)
- **Is this one on**: `checked` for a box that is ticked, `on` for a switch or a toggle, `selected` for one of
  several, `current` for a place in a sequence (a step, a page, the page a link goes to). The prop and the state
  have the same word: `selected(bool)` and `State::Selected`.
- **Open**: `open` everywhere, never `expanded`. `open(..)` is the app's signal or value; a value that only starts
  it is `default_open(bool)`; one that forces what a pointer would open is `force_open(Option<bool>)`. What hears
  it change is `on_open_change(bool)`.
- **A person closed it**: `on_dismiss` for a panel the app opens (Dialog, Sheet, Drawer, Popover, Menu,
  ContextMenu, CommandPalette, TeachingTip, Toggletip): Escape, a press outside, Tab out of a menu, its close
  button; not a choice or an answer. `on_open_change(open)` hears every change a person makes, a choice too, and
  is called after `on_dismiss`; a component with several panels tells which is open (`Option<usize>`). Neither is
  called when the app sets the signal. A panel every close of which is an answer (a message box) has no
  `on_dismiss`. `on_close` is for a message in the flow whose close control was pressed (Alert, Banner, Toast, Tag
  has `on_remove`). An action of its own keeps its name (`on_cancel`, `on_clear`).
- **What a press outside does**: `dismiss_on_veil(bool)` where there is a veil, `dismiss_on_outside_press(bool)`
  where there is none.
- **Where the focus goes in a question**: on the confirming answer, or on Cancel where the confirming answer is
  destructive.
- **Has a close control**: `close_button(bool)`. A message shows it once `on_close` is given. `clearable(bool)`
  is the control that empties a field.
- **The accessible name: `label(&str)`**, on every component that is a thing to tooling. Never `named`. Where
  the component draws its label, the label is its name. An entry of a menu that is a heading is `heading`.
- **The text under a title: `description`**; a part `Description`. Never `message`. The words of a tip are `text`.
- **The tone: `variant(impl Into<Name>)`**, with the component's enum `XVariant`. The tones are `Neutral`,
  `Info`, `Success`, `Warning`, `Destructive`, in that order where an enum has them; never `Error`, `Information`
  or `kind`.
- **Wrong: `invalid(bool)`**, state `Invalid`. `error` is the text a field shows.
- **Where an anchored panel stands: `placement(Placement)`**. The edge of a container a panel is on is `side`.
  How far it is from its anchor, along the side it is on, is `offset(impl Into<SpacingValue>)`, the theme's
  spacing; a shift across that side is `cross_offset(..)`. Never a pair of pixels.
- **Words lurq writes itself**: every one can be said otherwise: `<control>_label(&str)` for the name of a
  control (`close_label`, `clear_label`, `more_label`, `previous_label`), `<thing>_text(&str)` for a sentence or a
  word a part draws (`empty_text`, `loading_text`, `separator_text`). One method for one string, also where the
  word shown and the name read are two, or the name follows a state. A sentence with numbers in it takes a
  function: `count_text(Fn(rows, selected) -> String)`. A component's default name ("Main") is set with `label`.
- **A handler of a list told which item**: `on_select(Fn(&Id))` on the list (a breadcrumb told the step followed),
  beside each item's own `on_select(Fn())`.
- **What it holds**: a list in the constructor where the component is its items (`Tabs::new(value, tabs)`,
  `ButtonGroup::new(buttons)`), and `noun(item)` with `nouns(items)` to add to it (`item`/`items`, `step`/`steps`,
  `pane`/`panes`); every singular has its plural. `child`/`children` are for free `impl Into<Element>` content only.
- **Before and after**: `icon` is before the text; `trailing_icon` after it. An element is `leading(..)` and
  `trailing(..)`, which also name what stands at the ends of a bar. A picture or a mark a component is given
  is `media(..)`.
- **At most N shown: `max_shown`**. `max` is the greatest value of a number.
- **Which way: `orientation(Orientation)`** with `ui::Orientation`, of which `SliderOrientation`,
  `TabsOrientation` and the rest are aliases. No `horizontal(bool)`. Its states are `Horizontal` and `Vertical`,
  one of which is on.

**Types and order**

- **The signal is the first argument of `new`** where the component cannot be without it:
  `Popover::new(open, anchor)`, `Menu::new(open, anchor)`. A signal the component can keep for itself is a builder
  method (`Sidebar::open(signal)`, `Navbar::open(signal)`). Whether a panel is open is not a value in the sense of
  "bound and not": an overlay needs no `controlled` form.
- **Bound and not: `new(signal, ..)` and `controlled(value, .., on_change)`**, on every component that holds a
  value but a text input, which is always a signal. No type that is "a signal or a value". A component bound to
  `Signal<T>` of one of several is bound to `Signal<Option<T>>` too where nothing chosen is a state it has.
- **A handler's type is checked when the app is compiled, never when it is called.** A component that holds a
  value of the app's type is generic over it, with `()` as the default so that the theme's calls read as before
  (`Tabs<T = ()>`, `resolve::<Tabs>`): `on_change(|value: &T| ..)` of another type does not compile. No builder
  erases the type and checks a handler against it at run time. A component that holds another one (an input
  group's select, a toolbar's) is generic too or takes it as an element.
- **The type parameter is the value the component holds**: `Select<T>`, `Select<Option<T>>`, `ToggleGroup<T, [T]>`,
  `Calendar<DateRange>`, and `on_change(Fn(&V))` is told that value: one, one or none, or the whole new set. The
  forms: bound, `new(Signal<T>, ..)`, `optional(Signal<Option<T>>, ..)` and `multiple(Signal<Vec<T>>, ..)`; not
  bound, `controlled(value, .., on_change)`, `controlled_optional(..)` and `controlled_multiple(..)`. A component
  that has one form only (a group of checkboxes is always any number) calls it `new` and `controlled`.
- **A second value of a component is named**: `sort(signal)`, `sort_controlled(value, on_sort_change)`,
  `on_sort_change(..)`; the first keeps the plain words.
- **Bound, the signal is set first and the handlers are called after**, so a handler that reads the signal sees
  the new value. No handler is called when the app sets the signal, nor for a press that changes nothing.
  `on_change_end` is called only where the value changed between the start and the end of the gesture.
- **A component shown and not operated** (the steps of a wizard the app drives) has a form that takes the value
  alone and tells nothing, `at(value, ..)`: `Steps::at(current, steps)`. It is not `new(Signal::new(..))`.
- **Every `on_*` adds**: a second handler is called after the first, never in its place. The app's handlers
  are called before the component acts on the event; what a handler refuses (`prevent_default`) the component
  does not act on.
- **`action(..)` sets the one action** (a second call takes its place); `actions(..)`/`action` of a component
  that has several adds, and that component's docs say so. A button made of a label and a handler is
  `button(label, handler)`.
- **A panel that closes on a choice closes first and then calls the handler**, so a handler that opens a dialog
  does not have its focus taken back by the closing panel.
- **`size(impl Into<Name>)`** with the component's own `XSize`; a component that has another's sizes names an
  alias (`ToggleSize = ButtonSize`, `SelectSize = TextBoxSize`). The default is `Md` unless the board draws
  another; where the board's default is no step (a dialog of 420), the docs say so. A length is `length`, `width`,
  `height`, not `size`; `Icon::size` is a length, since an icon has no recipe. A shape is `shape(XShape)`.
- **`width` and `height` take `impl Into<Length>`.** A number of a value (`min`, `max`, `step`) takes
  `impl Into<f64>`. A text takes `&str`.
- **`icon(impl Into<Name>)`.** A component with an icon of its own takes `impl Into<IconChoice>`
  (`ui::IconChoice`): a name, or `None` for no icon. No `icon_hidden`. An icon for the open form is `open_icon`.
- **`build(theme)` and `From<X> for Element` on every component.** One that keeps something between renders
  (timers, what the pointer is on, a measured size) also has `mount(ctx)`, and that is where it is kept: the app
  is not handed a helper to hold for it. Converted into an element on its own it is the still form of itself.

**What every one of a kind has**

- **Every themed builder**: `style(part, ..)`, `rules(&recipe)`, `state(name, on)`, `id(..)`, `key(..)`, and
  `force(..)` where something in it is pressed or takes the focus: a component of items draws it on the item Tab
  would stop on. Every builder of an item that is an element: `id`, `key`, `disabled` (a column, a token of a
  text and a panel of links are not).
- **Every typed control**: `label`, `placeholder` where it has one, `size`, `invalid`, `warning`, `valid`,
  `read_only`, `disabled`, `full_width`, `name` (with `form`), and is a `FieldControl`.
- **Every choice control**: the value and `on_change`, `bind(Signal<bool>)` for one bound to a signal, `label`,
  `disabled`, `invalid` where a field can hold it; every group of them `label`, `orientation`, `disabled`,
  `invalid`.
- **Every overlay**: `label`, `inline(bool)`, `on_dismiss`, `dismiss_on_escape(bool)`, and
  `dismiss_on_veil(bool)` or its press outside; an anchored one `placement`, `offset`, `collision`.

**Parts and states**

- `Root` is the component's box where it is declared. A floating panel is `Panel`. What is typed in is `Control`;
  what is pressed to open is `Trigger`.
- `Header` is the row at the top that holds the text and a close control; `Text` the column of `Title` and
  `Description`; `Body` where `child(..)` goes; `Footer` the row of actions.
- A leaf text is `Label`, on one line or wrapping. A line between things is `Separator`. What says "this opens"
  is `Chevron`, with what it opens before it where a component has several (`DetailChevron`). The well an icon
  stands in is `Mark`. The group of buttons in a `Footer` that holds more than buttons is `Actions`. The rail of a
  value is `Track` and its filled part `Fill`. A part's name in a recipe is snake case.
- A state that says the component has a part is the noun of the part (`Icon`, `Description`, `Footer`), not a
  participle; a state that is a look or a mode keeps its word (`Framed`, `Bordered`, `Masked`). An item's own state
  is resolved on the item's part (`Disabled`), not on the root (`ItemDisabled`); where the component's own state of
  that name reaches the item's part too, the item's has a word of its own (`Unavailable`, `OptionDisabled`).
- A component that wraps another lists the inner one's states it shows in its own enum.

## What the eighth batch and the QA pass taught

- **A text has a rule for when it does not fit**, and the test of a component has a text longer than its room: a
  title and a description wrap, a label on one line ends in an ellipsis, what stands beside it (a shortcut, an
  action, a close button) keeps its room. A component whose page shows only the board's short words was cut the
  first time a playground was given a sentence.
- **Where the focus goes is part of every way out.** Closing by a choice, by Escape, by Tab, by the control that
  clears: each returns the focus to what opened it, or says where (`ActionBar::return_focus`). The engine keeps a
  starting point when the focus is dropped, so the next Tab goes on from there; a test that blurs and presses Tab
  does not start at the top of the window.
- **A row follows the density; a control keeps its size.** A recipe of a row-like part says
  `Recipe::row_density(selector, part, board_height)`, with a test that its height follows the theme's density.
- **Two things on one anchor are two layers.** A tooltip and a menu on one button: what looks a layer up by its
  anchor finds the first.
- **A session that moves another component's code** (a selection model shared by the tree and the listbox) merges
  the base first and often: the other session is changing that code too.
- **The checks of a session are four**: `cargo check -p lurq` with no features (what is behind `router` or `form`
  is used only behind it: a navbar's link could not go to a path without a router, and the library did not
  compile without the feature), `cargo check -p lurq --features form,router --tests`,
  `cargo check -p gallery --features mcp` and `cargo check -p gallery --features devtools`. A value kept in a
  signal is `DevtoolsInspectable` with the last. At the merge the ui tests run with `mcp` too: the tests of what
  tooling reads (roles, names, `a11y_checked`) are behind it.
- **What a batch may use now**: `FocusGroup::pages`, `shift_extends`, `panel_row`; `ColumnTracks` and
  `ScrollBoth::fill_width`; `KeyboardEvent::focus_key`; `core::Date`, `YearMonth`, `DateRange`, `DateLocale`;
  `Recipe::row_density`; `ui::Navbar`, `ui::NavigationMenu`, `ui::Table`, `ui::List`, `ui::ListItem`,
  `ui::Listbox`, `ui::DataList`, `ui::Calendar`, `ui::DatePicker`, `ui::DateRangePicker`.

## What the eleventh batch taught

- **A family is built on a ground one session builds first.** Twelve charts would have had twelve legends: the
  ground (`ui/chart/`) and its first user were built and reported with the family's names before the others
  started, and the others were told to change it only by adding and to keep the first user's tests passing.
- **A piece of the engine that two briefs need has one owner named in both.** Two sessions each built
  `Canvas::paint` for their own component, as two had each built a row's least length a batch before; a brief
  that sends a session to the canvas, the layout or the runtime says who builds what the others will also want.
- **The first painted frame is the settled one.** What measures its own room and then lays itself out (a
  splitter, a toolbar that overflows, a timeline that folds) measures before the paint
  (`ElementRef::observe_rect_before_paint`), and its test compares the first frame with the next
  (`Stage::first_frame`, `Stage::frame`). A jump of one frame is a defect Ivan sees.
- **The merge compiles the browser's build and checks the docs' links every time**:
  `cargo check -p gallery --target wasm32-unknown-unknown`, and the docs build with `scripts/check-docs.py`.
  The engine's clock is `crate::time::Instant`, never `std`'s.
- **What moves, moves in every form of the component.** A category that opens with a motion in an accordion
  opens with it in a property grid and a tree; a chart whose bars move to new values has lines and slices
  that do. One of a family animated and the others not reads as a fault.
- **A session stopped in the middle of its work is told to read `git status` first.** The limit of the model
  cut four turns with work uncommitted, one inside a merge; nothing was lost, since each began again from
  what its worktree held and not from its memory of it.
- **What a batch may use now**: `ui::chart` (`ChartSeries`, `chart_props!`, `ChartPlot`, `ChartLegend`,
  `ChartTooltip`, `LinearScale::nice`), `Canvas::paint` and `paint_moving`, `Tree::set_software_canvases` and
  `Stage::software` for a test that reads a canvas's pixels, `lurq::code`, `ui::CodeBlock`, `ui::Blockquote`,
  `ui::Highlight`, the type roles `Heading1` to `Heading6`, `Prose` and `Article`, `TextStyle::case`,
  `ElementRef::observe_rect_before_paint`, `Stage::first_frame`.

## What the tenth batch taught

- **A change of a rule the engine shares runs every suite before it is committed.** A session's own filtered tests
  passed while its change to the order of Tab broke another family's; whoever touches `tab_navigation.rs`, the
  layout engine or the runtime runs `--test ui_tests` whole with `mcp`, and `input_tests`, `layout_tests` and
  `runtime_tests` whole, and pins its own case beside the one it could have broken.
- **What stands for something of the window takes it from the window.** A title bar's icon is the window's icon
  unless the app says another; a glyph of the theme is what is drawn when there is none, not the default an app
  must replace.
- **A test by hand closes what it opened in the same script**, and the session looks at the screens before it ends
  its turn: no window of a test is topmost, and none outlives the script (two were left on Ivan's monitors).
- **The merge runs every test binary that has tests, `markdown_tests` with the `markdown` feature among them.**
  A rule of the engine (what a scroll holds is drawn on a whole device pixel) left one markdown test failing,
  unseen until a session ran that binary for its own work: the routine had named only the suites the batches
  touch.
- **macOS is compiled at the merge**: `cargo check -p lurq --target aarch64-apple-darwin`, and a session that
  writes behind a macOS `cfg` runs it itself.
- **What a report lists under "Not done" goes back to its owner at the merge**, with the board's node or the rule
  that settles it; the merge is pushed first, so the rest of the batch does not wait for it.
- **What a batch may use now**: `.captures_keys(bool)`; `Accelerator::from_key_event`, `parse`, `matches`;
  `.form_value(..)`; `on_copy` and `on_paste` on what is not a text input; `Gradient::corners`;
  `ScrollHorizontal::fill_width`; `lurq::eyedropper::EyeDropper`; `WindowHandle::show_system_menu()`;
  `ui::WindowChrome`, `ui::WindowTabs`, `ui::DocumentTabs`, `ui::StatusBar`, `ui::CommandBar`, `ui::FindBar`,
  `ui::ShortcutRecorder`, `ui::FloatingPanel`, `ui::PanelHeader`, `ui::PropertyGrid`, and the Data Grid's
  `ColumnLayout`, group rows and summaries.

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

## A narrow window

The gallery's window goes down to 320 wide, and the desktop system holds together there: nothing is cut, runs past
its card or lies over something else. It is not the phone's design (touch controls, sheets, a tab bar), which has
components of its own.

- **`cargo test -p gallery narrow` is run after a page or a shared piece of the gallery is added or changed.** It
  lays every page out with no window at 1440, 390 and 320 and fails when something runs past the box that holds it
  at a narrow width and not at 1440 (`crates/gallery/src/narrow.rs`). `NARROW_PAGES=/button,/table` lays those
  pages out alone; in Git Bash put `MSYS_NO_PATHCONV=1` before it, or the routes become paths and nothing is
  checked. `python scripts/gallery-narrow.py` walks the running gallery for the same and for more (what lies over
  what, a control squeezed under 28), with pictures of what it finds (`--shots <folder>`): it is run at the merge of
  a batch, with a look at the pictures.
- **The width of a recipe is the most a component takes**: `s.width(260.0).max_width(Length::Fill)`. A component
  with a width has a test in `crates/lurq/tests/ui/narrow_room.rs`, which lays it out in a row of 200.
- **A greatest width is a share of something.** `max_width(FILL)` holds a box in what holds it only where every box
  between the two has a width or a greatest width of its own: a row or a column with no width is as wide as what
  is in it, and a share of that is nothing. A root that only holds the control and its panel has one too.
- **A scene of a page**: a wrapper of a fixed width gets `.max_width(FILL)`; a row of examples `.width(FILL).wrap()`;
  a text beside a key or a button stands in `Column::new().flex(1.0).min_width(0.0)`, where it wraps; what cannot
  be narrower (a table's bar, a window's frame, a wizard's steps) scrolls sideways in its card,
  `kit::fit(key, self.frame.stage_room(), least, scene)`. `kit::phone()` says the window is under 640, for a scene
  that stacks there. A table's flexible column has a `min_width`, so that a narrow table scrolls and no cell is cut.
- **What is mounted keeps what it measured.** A splitter, a dock layout or a grid first laid out in a narrow card
  and then given its width by `fit` keeps the sizes of the first layout: its `key` says whether it scrolls, so the
  one in the scroll is mounted anew.
- **Nothing is hidden and no text is made smaller to fit**, and the wide form stays as it is: a change for a narrow
  window is a greatest width, a wrap that a wide window never reaches, or a branch on `phone()` or on a measured
  room.

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
