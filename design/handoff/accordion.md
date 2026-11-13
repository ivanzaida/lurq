# Accordion and Collapsible: hand-off

`ui::Accordion` (with `ui::AccordionItem`) and `ui::Collapsible`, from the board's `Accordion` (`HAue9`) and
`Collapsible` (`xlFCT`) in Components / light (`Ovt1D`), Navigation › Lists (`qWv07`, `Do5dr`); each is used once more
in Components / dark (`XfK30` in Navigation, `DhAgR` in Data Display). The board draws an accordion of three items,
the first open, and a collapsible open on three boxes; it draws no hover of an accordion's header, no focus and nothing
disabled. What lurq has for what the board does not draw is under "Decided" and "Differs from the board".

Files: `crates/lurq/src/ui/collapsible.rs` (the collapsible, and the view an item of an accordion is built with),
`crates/lurq/src/ui/accordion.rs`, `crates/lurq/tests/ui/accordion.rs`, `crates/gallery/src/pages/accordion.rs` (page
"Accordion", group Navigation, with a section for the collapsible alone), and lines in the three `mod.rs` files.

## Changed outside the component's own files

The engine, so that a part opens to and closes from the height of its content (this batch's engine change of its kind:
the Accordion session owns it):

- `crates/lurq/src/animation/interpolate.rs`: a new `AnimatableValue::Fit { px, content }`, a height of `px` and
  `content` times the height the element's content gives it. `read_target` reads a `Height` the content decides (no
  height, or `Auto`) as `Fit { px: 0, content: 1 }` where it read nothing before, and `lerp` mixes `Float` and `Fit`
  into a `Fit`. A transition of `Height` between a height in pixels and none therefore runs, both ways.
- `crates/lurq/src/node/node.rs`: `Node::effective_frame` lays a node with a `Fit` override out with no height, and a
  new `Node::fit_height()` returns the override.
- `crates/lurq/src/layout/layout_engine.rs`: `layout_node_box`, after the node is laid out as high as its content,
  gives it `px + content × that height`, within its constraints. What is in it stays where it was laid out; the node
  clips the rest where it clips.
- What changes for other code: a node with `Transition::height()` or `Transition::all()` whose height goes from a number
  to none, or from none to a number, now animates between them where it changed at once. A node whose height stays
  none has `Fit` both times, so no run starts when only its content grows. Reduced motion ends it at once, as every
  `Height`. `AnimatableValue` is public and has a variant more: a `match` on it outside lurq needs an arm for it.
- `docs/src/content/docs/animation-transforms.md` needs a line under "Animatable Properties" or under the
  `Transition::height()` row: "A height the content decides is one too: a transition of `Height` between a height in
  pixels and none runs to, and from, the height of the content, which the layout reads as it goes."

Nothing in `shared.rs`, `part_style.rs` or `recipe.rs` was changed. No keyframes were registered: the content fades in
and out with `ui::FADE_IN` and `ui::FADE_OUT`.

## For `docs/src/content/docs/design-system.md`

## Accordion

Headers one under the other, each of which opens what is under it: questions and their answers, the sections of a long
form, settings in groups. One item is open at a time, or any number.

```rust
use lurq::ui::{Accordion, AccordionItem};

// One open at a time, or none: open is a Signal<Option<u8>>, which a press sets.
Accordion::new(open, [
  AccordionItem::text(0, "Is lurq GPU-accelerated?", "Yes. Rendering goes through wgpu."),
  AccordionItem::text(1, "Does it support dark mode?", "Every theme has two appearances."),
  AccordionItem::new(2, "Can I bring my own theme?", theme_form).disabled(locked),
]);

// Any number open: sections is a Signal<Vec<&str>>.
Accordion::multiple(sections, [
  AccordionItem::text("billing", "Billing", "Card, address and invoices."),
  AccordionItem::text("shipping", "Shipping", "Where the order goes."),
]);
```

| Builder | |
| --- | --- |
| `Accordion::new(Signal<Option<T>>, items)` | One item open at a time: the one whose value the signal has, or none. A press of another opens it and closes the one that was open; a press of the open one closes it |
| `Accordion::multiple(Signal<Vec<T>>, items)` | Any number open: those whose values the signal has. A press opens an item, or closes it |
| `Accordion::controlled(open, items, press)` | The app holds what is open: the items of `open` are, and `press` is told the value of each header pressed |
| `AccordionItem::new(value, label, content)`, `AccordionItem::text(value, label, body)` | An item: its value, what its header says, and what it shows while it is open, an element or a text. `disabled(bool)` on an item |
| `disabled(bool)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `disabled` is every header's |

- Each header is a stop of Tab, as a web accordion's is, and Space and Enter press it; Up and Down move the focus from
  one header to the next and round, Home and End to the first and the last. A disabled header is passed by.
- What an item holds opens and closes with its height, over the theme's base duration, and fades in and out with it:
  the panel's height goes from none to its content's, which the layout reads as it goes. Where motion is reduced it is
  there at once. While an item is closed what it held is not there: not painted, and the controls in it are no stops
  of Tab.
- Parts: `AccordionPart::Root` (the items), `Item` (an item, with the line under it), `Trigger` (its header: what is
  pressed and focused), `Label`, `Indicator` (the chevron), `Panel` (what opens and closes) and `Content` (what the
  panel holds, in its padding). Each item's parts are styled in that item's states, `AccordionState::Open` and
  `Disabled` (`AccordionState` is `CollapsibleState`).
- In the design each item has a line of 1 in `Border` under it. Its header is 14 above and under its label, which is
  in the body's type (14) at medium weight in `Foreground`, with a chevron of 16 in `MutedForeground` at its end, 12
  from the label: down while the item is closed, up while it is open. What it opens is in the body's type in
  `MutedForeground` at a line height of 1.5, 14 above the line. A disabled header is dimmed; the ring of the focus is
  around the header.
- To what reads the tree (`lurq_snapshot` of the MCP server) a header is a `button` called by its label, with
  `a11y_expanded` saying whether its item is open.
- A renderer that draws another accordion (`AccordionRenderer`) is given `view.items`, each a `CollapsibleView` in the
  accordion's recipe, and `view.root()`, the root with the behavior of the keyboard.

## Collapsible

One header and what it opens, with no others around it: the crates of a workspace, the details of an order, options a
person rarely needs.

```rust
use lurq::ui::Collapsible;

Collapsible::new("3 crates in this workspace", crates)
  .open(open.get())
  .on_change(move |next| open.set(next));

Collapsible::text("Why is this here?", "Because the build needs it.");
```

| Builder | |
| --- | --- |
| `Collapsible::new(label, content)`, `Collapsible::text(label, body)` | A header that says `label`, and what it shows while it is open: an element or a text |
| `open(bool)` | Whether what it holds is shown. The collapsible does not keep it: the app says so |
| `on_change(handler)` | Called on a press of the header with what the collapsible is to be: open if it was closed |
| `icons(closed, open)` | The icons of the indicator, closed and open, by their names in the theme's set; `chevrons-vertical` both |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the header |

- The whole header is pressed; Tab reaches it, and Space and Enter press it. What it holds opens and closes as an
  accordion's item does.
- Parts: `CollapsiblePart::Root`, `Trigger`, `Label`, `Indicator`, `Panel`, `Content`. States: `CollapsibleState::Open`,
  `Disabled`.
- In the design the label is in the label's type (13, medium), `Foreground`; at the end of the header the
  `chevrons-vertical` icon of 16 is in a ghost button of 32 that `Highlight` fills under the pointer; what it opens is 8
  under the header. A disabled header is dimmed, and what it opens, which is the app's, is not.
- To what reads the tree the header is a `button` called by its label, with `a11y_expanded`.
- A renderer that draws another collapsible (`CollapsibleRenderer`) starts from `view.trigger()`, the behavior with the
  header's style, and `view.panel()`, what opens and closes with what it holds; `view.label()` and `view.indicator()`
  are the default ones.

## For `CHANGELOG.md`

- `ui::Accordion` and `ui::Collapsible`: headers that open what is under them, one at a time or any number, bound to a signal or held by the app, with Tab and the arrow keys; what opens grows to the height of its content and back, as a transition of `Height` between a number and none now does.

## Decided

- **Builders.** What is open is the app's: a `Signal<Option<T>>` or a `Signal<Vec<T>>`, or `controlled` with a list
  and a handler, as `ToggleGroup` holds its choices (through `shared::Chosen`); a collapsible is `open(bool)` and
  `on_change`, as `Switch::on`. Nothing is kept by the component, so no mounted `Component` was needed.
- **One open, and none.** `new` takes a `Signal<Option<T>>`, so that every item can be closed: the board draws one open
  and does not forbid none, and a press of the open item closing it is what a person expects of the header they just
  opened.
- **An accordion's item is a collapsible** in the accordion's recipe: `CollapsibleView` builds both, the item's root
  being the accordion's `Item` part. The accordion has a recipe of its own (`Themed for Accordion`), since the board's
  two differ in everything but their behavior.
- **The height.** The panel is of height 0 with a transition of `Height` over the theme's base duration, and of no
  height (`Length::Auto`) while open; the engine's new `Fit` value makes the transition run to and from the content's
  height. The panel clips, and has `.presence()`; its content is keyed and has `FADE_OUT` as its exit animation over the
  same duration, so what closes stays (only painted, no Tab, no press) while the panel closes over it, and is gone
  after. The content enters with `FADE_IN`, also on the first render of an open item.
- **Tab stops at each header**, with `FocusGroup::vertical()` (not one stop) on the accordion's root, as the brief and
  the WAI pattern say.
- **The ring of the focus** is `focus_ring()` on the header, outside it, with `radius-sm` on an accordion's header and
  `radius-md` on a collapsible's; the root and the item draw outside themselves (`overflow_visible`) so that it shows.
- **A disabled collapsible** dims its header only: what it opens is the app's, with controls that judge their own state.
  Its ghost button does not light under the pointer.
- **The collapsible's content** has no style of the board's (the board's boxes are the app's content): the body's type,
  8 under the header. `Collapsible::text` gives a text in it.

## Not done

- **The arrow keys stop at the controls inside an open item too.** `FocusGroup` moves between every stop of Tab in the
  group (`tab_navigation::group_stops`), so a button inside an open panel is between its header and the next one for Up
  and Down, where a web accordion's arrows move between headers alone. Focus groups are the Tabs session's; the
  smallest change is a mark on an element that keeps it and what is in it out of the group around it (as
  `.focus_group(..)` on the content would, if a group with no keys of its own were allowed: `FocusGroup::none()`, whose
  `step` takes no key), which the panel's content then has.

## Differs from the board

- **The collapsible's whole header is pressed**, not only its ghost button: the brief asks for a header that is pressed,
  and a header of a label and a button of 32 is easier to hit whole. The button is drawn as the board draws it, and
  lights under the pointer where the pointer is over it.
- **Hover, focus and disabled** are not on the board. An accordion's header has no hover (the board draws none, and the
  chevron and the line say it is pressed); the focus is the theme's ring around the header; a disabled header is half
  opaque.
- **The width** is the app's: the board's accordion is 360 and its collapsible 280; lurq's are as wide as what holds
  them gives them, and the gallery and the tests give them the board's.
- **The chevron of an open item** is the `chevron-up` glyph in place of the down chevron turned, as the board draws it;
  there is no turning of it between the two.
