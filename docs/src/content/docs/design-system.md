---
title: Design System
description: The components of lurq::ui and what a theme styles them with. Recipes, part styles, rules by variant, size, density, breakpoint and state, renderers, a bare theme, a theme for a subtree, Icon, Button, Toggle, the groups, SegmentedControl, Link and Fab.
---

# Design System

`lurq::ui` holds the design system's components and what a theme styles them with. It is being built a component at a time: this page describes what is in the crate today. The gallery (`cargo run -p gallery`) shows each component live.

```rust
use lurq::{components::Row, ui::{Button, ButtonVariant}};

Row::new()
  .spacing(8.0)
  .child(Button::new("Cancel").variant(ButtonVariant::Outline))
  .child(Button::new("Save").on_click(|_| save()))
```

A component is a builder that goes where an element goes. It takes the theme of the component being rendered around it, so nothing is passed to it, and what renders it renders again when the theme changes.

## The Three Layers

| Layer | What it says | Where it is set |
| --- | --- | --- |
| Tokens | What blue is, how round `Md` is | The palette and the scales of the [theme](../theme/) |
| Recipes | How each part of a component looks in each case | `theme.patch_recipe::<Button>(...)`, `theme.set_recipe::<Button>(...)` |
| Renderers | What nodes a component is built from | `theme.set_renderer::<Button>(...)`, `theme.set_part_renderer::<Button, _>(...)` |

Each has the design as its default. Most apps change tokens only. A brand changes recipes. A look whose controls are built differently changes renderers.

The gallery shows each level live: its **Styling** page (Getting started › Styling, `cargo run -p gallery -- --route /styling`) changes a palette role, patches a recipe, styles one instance with `.style`, `.rules`, `.state` and `.force`, and gives a part a renderer of its own, each with its code and in the order they are laid. Every component's playground has a **Styling** tab beside its props: the parts and the states the component declares, a part restyled live for the one on the stage (`.style(Part, ..)`) or for every one in the stage's own theme (`theme.patch_recipe::<X>(..)`), and the Rust of both.

## Recipes

A component is made of named **parts**: a button has `Root`, `Label`, `Leading` and `Trailing`. A recipe is a list of rules, and a rule says that in a case a part has a style:

```rust
use lurq::{
  app::theme::{ControlSize, PaletteColor, RadiusSize},
  ui::{ButtonPart, ButtonVariant, Recipe, Selector},
};

let recipe = Recipe::new()
  // Always.
  .base(ButtonPart::Root, |s| s.height(ControlSize::Md).padding_x(16.0).radius(RadiusSize::Md))
  // For a variant.
  .variant(ButtonVariant::Primary, ButtonPart::Root, |s| {
    s.background(PaletteColor::Primary).foreground(PaletteColor::PrimaryForeground)
  })
  // For a variant under the pointer.
  .when(Selector::new().variant(ButtonVariant::Primary).hovered(), ButtonPart::Root, |s| {
    s.background(PaletteColor::PrimaryHover)
  });
```

| Rule | Holds |
| --- | --- |
| `base(part, style)` | Always |
| `variant(variant, part, style)` | For a variant |
| `size(size, part, style)` | For a size |
| `density(density, part, style)` | While the theme is at a [density](../theme/#density) |
| `row_density(selector, part, height)`, `by_density(selector, part, style)` | At each density but the default, by the step a row moves at it (`ROW_STEPS`): a row `height` high at the default, or a style given the step |
| `at(breakpoint, part, style)` | In a window at a [breakpoint](../theme/#breakpoints) or wider |
| `state(state, part, style)` | While the component is in a state: disabled, checked, loading |
| `hovered(part, style)`, `pressed(...)`, `focus_visible(...)` | Under the pointer, held down, focused from the keyboard |
| `within(part, style)` | While the [interaction group](../styling-events/#within-an-interaction-group) the part is in is under the pointer or holds the focus: what a row shows only then |
| `when(selector, part, style)` | In the case a `Selector` describes: several of the above together |

- The rules that hold are laid one over another, the more specific over the less: always, then a variant, then a size, then a density, then a breakpoint (a wider one over a narrower), then states (more of them over fewer). Rules of one rank apply in the order they were written.
- A style a later rule sets replaces the earlier one's field by field; what it leaves alone shows through.
- Variants, sizes, states and parts are names. A component gives its own as enums, which are accepted where a name is, and an app adds any others: `.variant("brand", ...)`, `.state("pinned", ...)`.
- Rules for the pointer and the keyboard become [state styles](../styling-events/) of the node. A hover renders nothing again, and changes only what is painted: no size, padding or text.
- `Recipe::extend(&other)` adds another recipe's rules after this one's.

A part that shows only while the element it is in is pointed at or focused, as the action at the end of a tree's row,
is styled `within`: the component makes that element an interaction group (`.interaction_group(true)`), and the rule
becomes the part's `within_style`. `Selector::within()` joins it with a variant or a state, `PartStyles::within` is what
the rules gave it, and a part drawn with a forced interaction shows it too.

```rust
use lurq::{
  app::theme::Density,
  ui::{Recipe, TreeViewPart},
};

// Hidden at rest, shown in its row's light, and always under a finger, which points at nothing.
let rules = Recipe::new()
  .base(TreeViewPart::Reveal, |s| s.opacity(0.0))
  .within(TreeViewPart::Reveal, |s| s.opacity(1.0))
  .density(Density::Touch, TreeViewPart::Reveal, |s| s.opacity(1.0));
```

### Breakpoints

A rule `at(Breakpoint::Md, ...)` holds at that breakpoint and at every wider one; a rule with no breakpoint is for every width, the narrowest included. This is the order [`Responsive`](../theme/#responsive-values) values resolve in.

```rust
use lurq::{app::theme::Breakpoint, ui::{Button, ButtonPart}};

// A taller button on a narrow window, the design's from the medium breakpoint up.
theme.patch_recipe::<Button>(|rules| {
  rules
    .base(ButtonPart::Root, |s| s.height(48.0))
    .at(Breakpoint::Md, ButtonPart::Root, |s| s.height(36.0))
});
```

A component is built again when the window's width crosses a breakpoint, not at every resize. `ui::current_breakpoint()` is the breakpoint of the window what is being built is in.

### Density

A theme is at one [density](../theme/#density), and a rule `density(Density::Compact, ...)` holds while it is at that one. A component is resolved at the density of the theme it is built in, and is built again when `theme.set_density(..)` changes it.

```rust
use lurq::{app::theme::Density, ui::{Button, ButtonPart}};

// A theme whose buttons are as high as a row is: lower in a compact tool, higher under a finger.
theme.patch_recipe::<Button>(|rules| {
  rules
    .density(Density::Compact, ButtonPart::Root, |s| s.height(Density::Compact))
    .density(Density::Touch, ButtonPart::Root, |s| s.height(Density::Touch))
});
theme.set_density(Density::Compact);
```

The design's recipe for `Button` has no rule for a density: a button has its sizes, and so have the other controls (an input, a select's box). The design's rows have: an item of a menu, a sidebar or a command palette, an option of a select, a combobox or a multi select, a row of a tree view, a tab, an accordion's header. A row moves from its height at the default by `ROW_STEPS`, 8 lower at compact and 8 and 12 higher at comfortable and touch, as a row of 36 is 28, 44 and 48; a row the design draws 32 high is 24, 40 and 44. `Recipe::row_density(selector, part, height)` writes those rules for a part of `height` at the default in the case of `selector`, and `by_density(selector, part, |style, step| ..)` any other: an accordion's header moves its padding. `Selector::density(..)` joins a density with a variant, a size or a state in one rule.

### The Touch Form

Some components have a form dressed for a finger: the [lightbox](#lightbox)'s phone viewer, the [picture](#picture)'s phone form, the [video player](#video-player)'s touch form. Which one they take is the person's pointer and nothing else: under a finger (`Pointer::Coarse`) the finger's, under a mouse, a trackpad or a pen (`Pointer::Fine`) the pointer's. `ui::touch_form()` says it for what is being built, and is what each of them reads; an app's own component with a touch form reads it too. The pointer starts as the platform says (`Coarse` on iOS and Android, `Fine` on a desktop, in the browser what `(pointer: coarse)` says) and follows the last input unless the app pins it: see [A Finger Or A Mouse](../styling-events/#a-finger-or-a-mouse). A component's own prop for its form (`variant(..)`, `touch(bool)`) wins over it.

Neither the window's width nor the theme's density is asked. A narrow window of a desktop is a narrow room, and a component lays itself out in it (labels to an ellipsis, a row into a column, the lightbox's level giving way), but it keeps the pointer's form: a phone's chrome in a narrow desktop window would take away the buttons a mouse needs. A [density](#density) is how large rows are, at any pointer. A rule for a [breakpoint](#breakpoints) changes the layout, never the form. A lightbox that is open keeps the form it opened in until it closes, so the finger that lands on it does not change it under itself; the app setting the pointer reaches it at once.

```rust
use lurq::{app::Pointer, ui::{Lightbox, LightboxVariant}};

// A kiosk with a touch screen: every component takes its touch form.
app.set_pointer(Pointer::Coarse);

// One lightbox in the phone's viewer whatever the pointer.
Lightbox::new(open, pictures).variant(LightboxVariant::Phone);

// An app's own component.
let swipes = lurq::ui::touch_form();
```

## Part Styles

`PartStyle` is the style of one part: a patch in which every field is optional. Every component uses this one type.

| Builder | Sets |
| --- | --- |
| `background(color)`, `foreground(color)` | The fill, and the color of the text and icons in the part and under it |
| `border(border)`, `radius(radius)` | A border on every side; the corners |
| `underline(border)` | A line along the bottom of the part only: what a link has under the pointer |
| `start_line(border)` | A line along the start side of the part only: what marks the selected tab of a list that runs down the side |
| `shadow(shadow)`, `veil(color)` | Shadows; a color laid over the fill and under the content (`StateHover`, `StatePressed`) |
| `outline(outline)`, `focus_ring()` | An outline; the theme's focus ring as the outline |
| `opacity(value)` | A number, or a level of the theme's (`OpacityLevel::Disabled`) |
| `padding_x(spacing)`, `padding_y(spacing)`, `gap(spacing)` | Padding across and down, and the room between the part's children |
| `padding_top(spacing)`, `padding_bottom(spacing)`, `padding_start(spacing)`, `padding_end(spacing)` | The padding of one side, laid over `padding_y` or `padding_x`: 6 above and 4 under a label. Start is the left, end the right. `paddings()` reads the four as they come out: top, end, bottom, start |
| `width(length)`, `height(length)`, `min_width(length)`, `min_height(length)`, `max_width(length)`, `square(length)` | A `Length`: pixels, a step of the control, density or icon scale, `Length::Fill`. `max_width` is the most the part grows to |
| `typography(role)` | Every field of the part's text at once |
| `font_family(family)`, `font_size(px)`, `weight(weight)`, `font_style(style)`, `line_height(ratio)`, `letter_spacing(px)` | One field of the text, over the role's |
| `transition(PartTransition::new(Transition::background_color(), MotionDuration::Fast))` | A transition, with a duration and an easing of the theme's |
| `enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))`, `exit(PartAnimation::new(FADE_OUT, ..))` | Keyframes the part plays as it appears and as it leaves, with a duration and an easing of the theme's. A part that leaves is kept while it plays where what holds it keeps leaving parts: an overlay's layer, or a child with an id or a key of an element with `presence()`. `ui::FADE_IN` and `ui::FADE_OUT` are an opacity from nothing to whole and back, which every window has |
| `cursor(icon)` | The cursor over the part |

Colors, radii, spacings and shadows are roles of the theme or concrete values, as everywhere in lurq.

A part's `foreground` is the color of its text over what its typography role says: where a theme's role names a color (its `TextStyle::color` is not the default), a part that sets a `foreground`, at rest or in any of its states, still draws its text in that foreground, and a part that sets one only in a state draws its text in the role's color at rest. Where the part sets no foreground, the role's color stands, and a role that names none takes the foreground of what the text is in.

A part's text is in one of the theme's two families or in one by name:

```rust
use lurq::ui::{ButtonPart, FontFamily};

rules
  .base(ButtonPart::Label, |s| s.font_family("Georgia"))
  .variant("code", ButtonPart::Label, |s| s.font_family(FontFamily::Mono))
```

## What A Theme Changes

```rust
use lurq::{app::theme::RadiusSize, ui::{Button, ButtonPart, ButtonVariant}};

// Every primary button is a pill.
app.theme().patch_recipe::<Button>(|rules| {
  rules.variant(ButtonVariant::Primary, ButtonPart::Root, |s| s.radius(RadiusSize::Full))
});
```

| | |
| --- | --- |
| `theme.patch_recipe::<C>(\|rules\| ...)` | Lays rules over the component's recipe. What it adds wins where both style a case. |
| `theme.set_recipe::<C>(recipe)` | Gives the component a recipe in place of the design's: nothing of the design's is left under it. |
| `theme.set_renderer::<C>(renderer)` | Builds other nodes for the same behavior. |
| `theme.set_part_renderer::<C, R>(part, renderer)` | Builds one part; the rest is still built by the renderer it had. |

The same is on a [`ThemeDefinition`](../theme/#themes-as-one-value), in its `ui` field: `definition.ui.patch::<Button>(...)`, `set_recipe`, `set_renderer`, `set_part_renderer`, `reset_recipe`.

What a theme resolved for a component in a case it keeps until the theme changes, so a list of a thousand buttons resolves the recipe once.

### A Bare Theme

`ThemeDefinition::bare()` has the design's tokens and no recipe. Every part of every component comes out unstyled, and every component still works: a bare button is its label, pressed by a click and by the keyboard. It is the base for a look that shares nothing with the design's.

```rust
use lurq::{
  app::theme::{RadiusSize, ThemeDefinition},
  node::color::Color,
  ui::{Button, ButtonPart, Recipe},
};

let mut look = ThemeDefinition::bare();
look.ui.set_recipe::<Button>(
  Recipe::new()
    .base(ButtonPart::Root, |s| {
      s.height(44.0)
        .padding_x(24.0)
        .radius(RadiusSize::Full)
        .background(Color::from_hex("#111827"))
        .foreground(Color::from_hex("#f9fafb"))
    })
    .hovered(ButtonPart::Root, |s| s.background(Color::from_hex("#374151"))),
);
app.theme().apply(look);
```

### Renderers

A recipe restyles a component and cannot change what it is made of. A renderer builds its nodes. It is given the component's view: what the component was given, the style of each of its parts, and its behavior ready to be dressed.

```rust
use std::sync::Arc;
use lurq::{components::{Column, Rect}, ui::{Button, ButtonPart, ButtonRenderer, ButtonView}};

// A button whose label is in capitals with a bar under it.
let renderer: ButtonRenderer = Arc::new(|view: ButtonView| {
  let label = view.part(ButtonPart::Label).text(&view.label.to_uppercase());
  view.root().child(Column::new().child(label).child(Rect::new(20.0, 2.0))).into()
});
app.theme().set_renderer::<Button>(renderer);
```

- `view.root()` is the element that is pressed, focused and disabled, with the style of the root part and nothing in it yet. The behavior is lurq's: a renderer cannot leave it out.
- `view.part(part)` is the style of a part; `.part(&style)` on any element gives the element that style, and `style.text("...")` is a text in the part's typography.
- A renderer for one part leaves the rest to the renderer the component had:

```rust
use lurq::ui::ButtonPartRenderer;

let label: ButtonPartRenderer = Arc::new(|view| view.part(ButtonPart::Label).text(&format!("» {}", view.label)).into());
app.theme().set_part_renderer::<Button, ButtonPartRenderer>(ButtonPart::Label, label);
```

### One Instance, One Place, One Subtree

```rust
// One instance, over what the theme gives it.
Button::new("Wide").style(ButtonPart::Root, |s| s.padding_x(40.0));

// The buttons in the places of a component: what a message box says of its confirm button.
let confirm = Recipe::new().base(ButtonPart::Label, |s| s.font_family("Georgia"));
Button::new("Confirm").rules(&confirm);

// A subtree in a theme of its own: a dense inspector in a window that is not.
let inspector = ctx.in_theme(&dense, |ctx| inspector_panel(ctx));

// An element and what is in it painted with another theme: a dark side bar in a light window.
Column::new().theme(&dark).background(PaletteColor::Background).child(nav);
```

- The order, later over earlier: the theme's recipe, the rules of the place (`.rules(...)`), the instance's own (`.style(...)`).
- Every builder of `lurq::ui` is `Styled`: a trait with `rules`, `state` and `style`, so what styles any component (a helper of the app's over several kinds, the gallery's playground) is written once: `fn pill<C: Styled>(c: C, part: impl Into<Name>) -> C { Styled::style(c, part, |s| s.radius(RadiusSize::Full)) }`.
- `Styled::describe(name, value)` says a state of any component to tooling that reads the tree, the DevTools and the MCP tree, as an element's own `describe` does: `Callout::new(..).describe("tone", "warning")`, `Progress::new(0.4).describe("share", "40")`. It is said on the node of the component's root part, where its id is, when the renderer gives that node the part's style, so a theme's renderer says it too; it comes before what the component says of itself there (its `a11y_role`, its `a11y_name`), and a reader that takes the first value of a name takes the app's. The instance's `Recipe` carries it (`Recipe::describe`), laid as its `rules` are. Like an element's `describe`, it says nothing unless a tooling feature (`devtools`, `mcp`) is on.
- `ctx.in_theme(&theme, |ctx| ...)` is the theme of everything the closure builds and mounts, and returns it as an `Element`. Components mounted in it keep the theme when they render again on their own, the one the closure returns as its root too (`ctx.in_theme(&theme, |ctx| ctx.mount::<Panel>(props))`): what it paints is in the theme after a render of its own as at its first.
- The theme reaches the whole of the subtree. A component built in it takes its recipe and its renderers, the sizes, type, opacity and motion of its parts, its density and its icons from that theme; and the roles left on the nodes, a color of the palette, a radius, a border width, a shadow, a spacing role, a typography style, the scroll bar and the caret, are laid out and painted with that theme's values. A subtree in a dark theme inside a light window is dark.
- `.theme(&theme)` on any element does the second half alone: the element and what is in it are laid out and painted with the values of that theme. What was built outside `in_theme` keeps the recipes it was built with, so this is for plain elements, and `in_theme` is for components.
- A change of the subtree's theme while the app runs lays the window out again, as a change of the window's theme does, and is there at once: a transition in the subtree runs between the colors of its theme, and not from the window's to its.
- What is declared in the subtree and drawn over the window, the content of a `Popup`, a `Modal`, the menu of a `Select`, is laid out and painted with the subtree's theme too.
- `ui::current_theme()` is the theme of what is being built. Outside a render there is none, and it is a neutral one that follows nothing: build components where they are rendered.

### Ids Of Parts

A component given an id keeps it on its root, and names the node of each of its other parts after it: `x-` and the part's name as its `*Part` enum writes it in a recipe, underscores and all. What finds a node by id (`Tree::get_element_by_id`, a test, DevTools, an agent over MCP) reaches a badge's label or a progress's fill with no id written for it.

```rust
use lurq::ui::{Badge, Button, DataList};

// status, status-dot, status-label
Badge::new("Online").dot(true).id("status");
// about, about-term, about-value, about-term-2, about-value-2
DataList::new().text("Version", "0.63.0").text("License", "MIT").id("about");
// save, and its spinner save-spinner, the spinner's indicator save-spinner-indicator
Button::new("Saving…").loading(true).id("save");
```

- A part of which there are several is told apart by its key where it has one: a day of a calendar by its date (`due-calendar-day-2026-11-20`), a weekday by its English short name (`x-weekday-mon`, the same in every locale), the buttons of a calendar's sides `x-nav-previous` and `x-nav-next`, an item, an option, a tab or a segment by its key or value. Where it has none, by the order it is built in: `x-term`, `x-term-2`, `x-term-3`.
- A node that has an id keeps it. An item the app gave an id `y` is `y`, and names its own parts after it (`y-label`, `y-chevron`), whether the component has an id or not.
- A component built inside another and given no id of its own is named after what it stands for there, and names its own parts after that: the spinner of a button `save-spinner`, the calendar of a date picker `due` `due-calendar` (not after the panel it stands in), a dialog's close button `x-close`.
- A component given no id gives its parts none, and the ids are the same at every render of the same content, so a node keeps its state from one to the next.
- A renderer of a theme gets the same: what it gives `.part(&view.part(..))` is named, `view.recipe.part_id(part)` is the id of the next node of a part for an element built otherwise, and `style.keyed(key)` names one by its key.

## Icon

A glyph of the theme's set, by name.

```rust
use lurq::{
  app::theme::{IconSize, PaletteColor},
  ui::{Icon, icons},
};

Icon::new(icons::SEARCH);                             // 16 high, in the foreground of what it is in
Icon::new(icons::SETTINGS).size(IconSize::Lg);        // a step of the theme's icon sizes
Icon::new(icons::CHECK).color(PaletteColor::Success); // a color of its own
Icon::new(icons::SPINNER).spin(true);                 // a spinner
Icon::new(icons::SPINNER).spin(true).spin_motion(SpinMotion::stepped(24)); // in 24 poses a second
```

| Builder | |
| --- | --- |
| `Icon::new(name)` | A constant of `ui::icons`, or a name the theme added |
| `size(size)` | An [`IconSize`](../theme/#icon-size) (`Md`, 16, by default) or a number of logical pixels. The icon is a square of that side |
| `color(color)` | A color in place of the foreground of what it is in |
| `spin(bool)` | It turns once in `ui::SPIN_PERIOD`, a second |
| `spin_motion(SpinMotion)` | How it turns, in place of the theme's: [in steps, and still where motion is reduced](#spin-motion) |
| `id(id)`, `key(key)` | Its id and key |

- An icon names no color: it is drawn in the `.foreground(..)` of what it is in, as text is, so the icon of a button is the color of its label.
- `ui::icons` has a constant for each of the 171 icons of lurq's font (`icons::CHEVRON_DOWN` is `"chevron-down"`), `icons::ALL` lists them with their characters, and `icons::FAMILY` is the family of the font. Every app has the font loaded, hinted: at 16 and 20 pixels its strokes land on whole pixels of the display.
- The theme says what a name is drawn with ([`ThemeIcons`](../theme/#icons)). A theme gives a name another glyph, or adds names of its own, from a font the app loads; lurq's components name the icons they draw, so they change with it.
- A name the theme has no glyph for takes its room and draws nothing.
- A name the theme draws with an SVG ([`IconSvg`](../theme/#icons), `theme.set_icon_svg(name, IconSvg::stroked(svg))`) is that SVG at the icon's size: a stroked one's lines and a filled one's shapes in the icon's color (its `color`, or the foreground of what it is in), a colored one in its own colors, a brand's mark. It turns where it spins. SVG is drawn with the feature `canvas`; without it, or where the source is no SVG, it takes its room and draws nothing.
- Most icons are lines. `icons::STAR_FILL` is a solid star, the shape a [rating](#rating) lights: a glyph that is filled where `icons::STAR` is drawn.
- Spinning icons are in phase: spinners that appear at different moments turn together. `ui::SPIN` is the keyframes of the turn, registered in every window; keyframe identifiers from `0xFF00` up are lurq's own.

### Spin Motion

How a spinning icon and a [spinner](#spinner) (its ring, its dots, its bars) move: a `SpinMotion`.

```rust
use lurq::ui::{Icon, SpinMotion, Spinner, SpinnerKind, icons};

// 24 poses a second and none between, and still where the system asks for reduced motion.
let motion = SpinMotion::stepped(24).still_when_reduced(true);
Icon::new(icons::SPINNER).spin(true).spin_motion(motion);
Spinner::new().kind(SpinnerKind::Dots).motion(motion);

// Every spinner that is not told its own, the ring of a loading button too.
app.theme().set_spin_motion(motion);
```

- `SpinMotion::SMOOTH`, the default, moves at every frame the display has, so the window draws every frame while a spinner is shown.
- `SpinMotion::stepped(per_second)` holds each pose until the next, `per_second` of them a second, and asks for no frame between: a ring of 24 turns 15 degrees at each of 24 steps of its second, and the window draws 24 frames a second whatever the number of spinners, for they all move on one clock. A period that is no whole number of steps is taken in the nearest number of them (the dots' 1.2 seconds are 28 steps at 24, 14 there and 14 back), and the dots and the bars start on a step, so that all of them change at the same moment.
- `.still_when_reduced(true)` leaves them in their pose at rest where motion is reduced (`Theme::reduced_motion`): the ring unturned, the dots and the bars as the design draws them still. Unless told so, a spinner moves under reduced motion too: one that stood still would say nothing, where nothing else says that something is under way.
- `Theme::set_spin_motion(motion)` (and `UiDefinition::set_spin_motion`) is the motion of every spinning icon and spinner given none of its own; `Icon::spin_motion` and `Spinner::motion` are one's own.

```rust
use lurq::{app::theme::IconGlyph, ui::{Icon, icons}};

// The app loads its font, and the theme draws from it.
app.load_font(include_bytes!("brand-icons.ttf").to_vec());
theme.set_icon(icons::SEARCH, IconGlyph::new("brand icons", '\u{F002}'));
theme.set_icon("rocket", IconGlyph::new("brand icons", '\u{F135}'));
Icon::new("rocket");
```

## Button

An action a press starts: a click, or Enter or Space while it is focused.

```rust
use lurq::ui::{Button, ButtonShape, ButtonSize, ButtonVariant, icons};

Button::new("Delete")
  .variant(ButtonVariant::Destructive)
  .size(ButtonSize::Sm)
  .icon(icons::TRASH)
  .on_click(|_| delete());

// An icon alone: a square, or a circle. Its label is still what it is called.
Button::new("Settings").variant(ButtonVariant::Ghost).icon(icons::SETTINGS).icon_only(true);
Button::new("Add").icon(icons::PLUS).icon_only(true).shape(ButtonShape::Pill);

// While what it started is under way.
Button::new("Saving…").loading(saving.get());
```

| Builder | |
| --- | --- |
| `variant(variant)` | `Primary` (the default), `Secondary`, `Outline`, `Ghost`, `Destructive`, or a variant the theme's recipe names |
| `size(size)` | `Xs` 28, `Sm` 32, `Md` 36 (the default), `Lg` 40, `Xl` 44, or a size the recipe names |
| `shape(shape)` | `ButtonShape::Default`, the theme's radius for a control, or `Pill`, rounded all the way |
| `icon(name)`, `trailing_icon(name)` | An [icon](#icon) before and after the label, by its name in the theme's set. It is drawn in the color of the label, at the size the button's size has for it |
| `icon_only(bool)` | The icon alone: a square as wide as it is high, a circle with `ButtonShape::Pill`. The label is not drawn. An icon with an empty label and nothing else beside it is drawn so too |
| `loading(bool)` | A spinner turns in the place of the icon, or before the label, and a click, Enter and Space do nothing until it is over |
| `description(text)` | A second line under the label that says more of what the button does. The button is as high as the two lines, with an icon of 20 beside them |
| `leading(element)`, `trailing(element)` | Any element before and after the label, where an icon by name is not enough |
| `disabled(bool)` | It takes no input: no press, hover or focus, and Tab passes it by. It is dimmed to the theme's `OpacityLevel::Disabled` |
| `full_width(bool)` | As wide as what it is in |
| `fill_label(bool)` | The label (and its description) takes the room the button has: what is before it stands at the button's start, what is after it (a `trailing_icon`, a `trailing` element) at its end, as a select's chevron does. Seen on a button wider than its content (`full_width`, a width of the app's). State `ButtonState::FillLabel` |
| `description_wraps(bool)` | The description goes on to the next lines in the room the button has, in place of one line cut with an ellipsis: an answer that says what a press of it leads to. State `ButtonState::DescriptionWraps` |
| `on_click(handler)` | A click, and Enter and Space while it is focused |
| `on_focus(Fn())` | Called when the button takes the focus: Tab, the arrows of a group it is in, the app, or a press of a button Tab stops at (one that is not, `tab_stop(false)`, no press focuses). What is said beside it may follow the button the keyboard is on. A second handler is called after the first |
| `on_open_aside(handler)` | Called, given the press, when a person asks for what the button opens beside what is open, in place of pressing it: the middle button, or a press with Ctrl held (Cmd on macOS), and Enter with it while it is focused. An app with tabs opens the place in a new one, and one the person switches to where the press's `shift` is held. Such a press is the handler's alone: no `on_click`, no focus, no submit, and nothing around the button hears it. See [pointer-only pressables](#pointer-only-pressables) |
| `tab_stop(bool)` | `false`: Tab passes the button by and a press leaves the focus where it was; focused by the app it is pressed by Enter and Space as before. See [pointer-only pressables](#pointer-only-pressables) |
| `autofocus(bool)` | The focus goes to it as the [dialog](#dialog), the popover or the menu it is in opens, in place of their first stop of Tab: a dialog's main action, which Enter then does. It is a stop of Tab as it was, in its place |
| `state(name, on)` | A state the theme's recipe names |
| `style(part, \|s\| ...)`, `rules(&recipe)` | One instance over the theme; the rules of the place it is in |
| `force(Some(Interaction::Hovered))` | Drawn hovered, pressed or focused with nothing pointing at it: for a gallery, a screenshot, a test. A component made of items (segments, tabs, rows, titles, links) draws it on one item, the one Tab would stop on, as it can only be |
| `id(id)`, `key(key)`, `submit(bool)` | Its id and key; with the `form` feature, the button that submits the form it is in |
| `ref_element(element)`, `focuses(&ElementRef)` | The ref of its element, which an app focuses or measures; and what a press of it gives the focus to, in place of taking it itself: the text a button of an editor's bar works on |
| `mount(ctx)`, `tooltip(Tooltip)` | The button with its [tooltip](#tooltip), kept in `ctx`: an icon-only button with a label has its label as its tooltip, as a [toolbar](#toolbar) gives the icons in it; `tooltip(..)` gives it one of the app's in place of that (the keys of a shortcut, another side, a tooltip for a button with words). A button with neither is built as it is. In a toolbar the button is given as it is, and the toolbar puts its tooltip on it |

| Size | Height | Padding | Label | Icon |
| --- | --- | --- | --- | --- |
| `Xs` | 28, `Density::Compact` | 10 | 12 | 14 |
| `Sm` | 32, `ControlSize::Sm` | 12 | 13 | 16 |
| `Md` | 36, `ControlSize::Md` | 16 | 13 | 16 |
| `Lg` | 40, `ControlSize::Lg` | 20 | 13 | 18 |
| `Xl` | 44, `Density::Comfortable` | 24 | 15 | 20 |

| Part | |
| --- | --- |
| `ButtonPart::Root` | The button itself: its fill, size, padding, corners, focus ring |
| `ButtonPart::Label` | Its text: 13/500 in the design |
| `ButtonPart::Icon` | An icon given by name: its side (`square(..)`), and a color of its own if the part names one |
| `ButtonPart::Spinner` | What turns while the button is loading |
| `ButtonPart::Description` | The line under the label: 12, at the theme's muted opacity, so that it is quieter than the label on any fill |
| `ButtonPart::Leading`, `ButtonPart::Trailing` | The elements put before and after the label |

States a recipe styles: `ButtonState::Disabled`, `FullWidth`, `Loading`, `IconOnly`, `Pill`, `Description` (it has a description), `FillLabel` and `DescriptionWraps`, and hovered, pressed and focus-visible.

- In the design a button under the pointer has its variant's hover fill (`PrimaryHover`, `SecondaryHover`, `DestructiveHover`, and `Highlight` for an outline or a ghost button), a button held down has the `StatePressed` veil over that, and one focused from the keyboard has the theme's focus ring. A primary button held down has the veil over its `Primary` fill: over `PrimaryHover` the veil is lost (black on near black in the light palette, white on near white in the dark), and a press would look like a hover. The board draws a pressed button in its hover fill.
- A button is as wide as its label where it has room. In a room narrower than it (a fixed width, `full_width` in something narrow, a panel that cannot grow, a line of a row that wraps) it is never wider than what holds it: it keeps its padding and its icon, and its label (and its description) is cut with an ellipsis. The board draws no button with a long label; the ellipsis is what every one-line label of the design system does. A toggle does the same. An icon alone keeps its square.
- A loading button shows three quarters of itself. It stays where the focus is and is not pressed: a form it submits is not submitted again. Its label is the app's to change.
- A theme's renderer for `ButtonPart::Label`, `ButtonPart::Icon` or `ButtonPart::Spinner` builds that part in place of the default one: dots that pulse where the design turns a ring.
- A button that is an icon alone is still called by its label: tooling that reads the tree (DevTools, the MCP tools) names it so.
- The design's close button, menu button and compound button are this button: `Button::new("Close").variant(ButtonVariant::Ghost).size(ButtonSize::Xs).icon(icons::CLOSE).icon_only(true)`, `Button::new("Export").variant(ButtonVariant::Outline).trailing_icon(icons::CHEVRON_DOWN)`, and a button with a `description`.
- In a [toolbar](#toolbar) a button is a ghost one of the bar's size (unless its item is `as_given`), and an icon alone has a tooltip of its label. Where the bar is too narrow for it, it goes into the bar's menu as an item of its label, its icon, its id with `-item` after it, disabled where the button is disabled or loading; choosing the item calls the button's `on_click` handlers with the click Enter would make (`from_keyboard`, at 0, 0).

### Pointer-only pressables

Many desktop apps reach their buttons, rows, tabs and links with the pointer and with their own shortcuts, and keep Tab for the fields of a form: a press of a toolbar's button leaves the focus in the text being typed. A pressable of the design system is a stop of Tab unless it is told `tab_stop(false)`, which a `Button`, a `ListItem`, a `Tag` (its remove button) and, once for all their items, `Tabs`, a `SegmentedControl`, a `Breadcrumb` and `WindowTabs` (its tabs, its add button and its More button) take:

```rust
use lurq::ui::{Breadcrumb, Button, ButtonVariant, Tab, Tabs, icons};

Button::new("Bold").variant(ButtonVariant::Ghost).icon(icons::BOLD).icon_only(true).tab_stop(false);
Tabs::new(view, [Tab::new("board", "Board"), Tab::new("list", "List")]).tab_stop(false);
Breadcrumb::new(["Projects", "Orchester", "KON-12"]).tab_stop(false);
```

- **Tab passes it by**, forward and back; Tab from it, where it has the focus, goes on to the next stop as from anything else.
- **A press with the pointer leaves the focus where it was**: in the field being typed in, or nowhere. The press does what it does.
- **It still takes the focus it is given**: `Ctx::focus`, an element ref, an agent through MCP. Focused, Enter and Space press it, a tag's Backspace and Delete take the tag away, and the arrow keys of the tabs or the segments it is one of move along them as before.
- **It is the same to what reads the tree**: its role, its name and its states do not change.

The engine's elements have the two halves apart, for an app's own control: `tab_stop(false)` passes an element by while a press still focuses it, as a browser's `tabindex="-1"` does, and `focus_on_press(false)` keeps a press from moving the focus.

A press can also ask for a place **beside what is open**, as a link in a browser does: `on_open_aside(handler)` on a `Button`, a `ListItem`, a `Tab`, a `BreadcrumbStep`, a `SidebarItem` and a `MenuItem` (whose menu closes, as after a choice). The middle button, or the primary one with Ctrl held (Cmd on macOS), and Enter with the key while it is focused, call the handler in place of the press: the button's `on_click` is not called, the tab is not selected, the step is not followed, the row's path is not gone to, nothing takes the focus, no form is submitted, and what is around the control (a row that opens its own page) does not hear the press. A middle press starts no scroll that follows the pointer. The handler is given the press, so an app tells a tab that is switched to (Shift held) from one that opens behind (`SidebarItem`'s is given nothing). A control with no such handler takes these presses as it did.

```rust
use lurq::{app::events::MouseEvent, ui::ListItem};

ListItem::new("KON-12 Tasks list")
  .on_click(move |_| open(route.clone(), Where::Here))
  .on_open_aside(move |press: MouseEvent| open(route.clone(), if press.shift { Where::NewTab } else { Where::Behind }));
```

## Toggle

A button that stays on until it is pressed again: bold in an editor, a filter that holds.

```rust
use lurq::ui::{Toggle, icons};

Toggle::new("Bold")
  .icon(icons::BOLD)
  .on(bold.get())
  .on_change(move |on| bold.set(on));
```

| Builder | |
| --- | --- |
| `on(bool)` | Whether it is on. The toggle does not keep it: the app says so |
| `bind(Signal<bool>)` | On while the signal is true, which a press turns; `on_change` handlers are called after it is set |
| `on_change(handler)` | Called on a press with what the toggle is to be: on if it was off |
| `icon(name)`, `icon_only(bool)` | An icon before the label; the icon alone, a square. An icon with an empty label is drawn so too |
| `leading(element)` | An element before the label, where an icon by name is not enough: the dot of a recording. It stands before an icon that is given too |
| `size(size)` | A `ToggleSize` (a `ButtonSize`): a toggle is as high as a button of that size |
| `focuses(&ElementRef)` | What a press of it gives the focus to, in place of taking it itself: the text a toggle of an editor's bar works on. It is still a stop of Tab |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them |

Parts: `TogglePart::Root`, `Label`, `Icon`. States: `ToggleState::On`, `Disabled`, `IconOnly`. In the design a toggle has no fill until the pointer is on it or it is on, and then the `Highlight` fill; it is padded 10.

In a [toolbar](#toolbar) a toggle takes the bar's size, and an icon alone has a tooltip of its label. Where the bar is too narrow for it, it goes into the bar's menu as a checked item, on while the toggle is: its label, its icon and its id with `-item` after it. Choosing the item calls `on_change` with the other state, as a press would.

## Button Group

Buttons that belong together, joined into one shape.

```rust
use lurq::ui::{Button, ButtonGroup, ButtonGroupVariant, icons};

ButtonGroup::new([Button::new("Years"), Button::new("Months"), Button::new("Days")]).label("Period");

// An action and the choice of how to do it: a split button.
ButtonGroup::new([Button::new("Merge").on_click(|_| merge())])
  .variant(ButtonGroupVariant::Primary)
  .button(Button::new("How to merge").icon(icons::CHEVRON_DOWN).icon_only(true).on_click(|_| choose()));
```

| Builder | |
| --- | --- |
| `ButtonGroup::new(buttons)` | The buttons, in order |
| `button(button)`, `buttons(buttons)` | Buttons after the ones it has |
| `variant(variant)` | `Outline` (the default): one line around the group and a line between its buttons, which have no fill. `Primary`, `Secondary`, `Destructive`: buttons in that fill, a pixel apart |
| `size(size)` | The `ButtonGroupSize` (a `ButtonSize`) of every button in it |
| `label(&str)` | What the group is called to what reads the tree, a `group` |
| `disabled(bool)` | Every button in it is disabled. State: `ButtonGroupState::Disabled` |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a button has them. `force` draws the first button |

- The group gives its buttons their variant and their size and squares their corners: the corners of the group are the round ones. A button keeps the rest of what it was given: its icon, what a press does, whether it is disabled.
- Parts: `ButtonGroupPart::Root`, the group; `Item`, what the group says of each button, laid over the theme's recipe for a button's root (padded 14 in the design); `Separator`, the line between two.
- A split button is a filled group of two: the action, and a button that opens a [menu](#menu) hung from the group (`Menu::new(open, anchor).placement(Placement::BottomEnd)`, the group in an element that carries the anchor's ref).

## Toggle Group

Toggles that are one choice: one of them is on, or any number are.

```rust
use lurq::ui::{Choice, ToggleGroup, icons};

// One of them is on: the one with the signal's value.
ToggleGroup::new(align.clone(), [(Align::Left, "Left"), (Align::Center, "Center"), (Align::Right, "Right")]);

// Any number are: those with the values the signal has.
ToggleGroup::multiple(marks.clone(), [
  Choice::new(Mark::Bold, "Bold").icon(icons::BOLD).icon_only(true),
  Choice::new(Mark::Italic, "Italic").icon(icons::ITALIC).icon_only(true),
]);
```

| Constructor | |
| --- | --- |
| `ToggleGroup::new(signal, choices)` | Exactly one is on: a press of another sets the signal |
| `ToggleGroup::multiple(signal, choices)` | A `Signal<Vec<T>>`: a press adds a value, or takes it out |
| `ToggleGroup::controlled(value, choices, on_change)` | The app holds the one that is on: a press of another tells `on_change` (`Fn(&T)`) the value it is to be, and a press of the one that is on tells nothing |
| `ToggleGroup::controlled_multiple(values, choices, on_change)` | The app holds those that are on: a press tells `on_change` (`Fn(&[T])`) the whole set it is to be |
| `on_change(handler)` | Called as a person changes what is on, with the new value: `\|align: &Align\|` for a group of one, `\|marks: &[Mark]\|` for one of any number. Not called where the app sets the signal. Each one adds |
| `label(&str)` | What the group is called to what reads the tree, a `group`; a [field](#field)'s label where it has none |
| `orientation(Orientation)` | Along a row (the default), or down a column, with the arrow keys of that way |
| `invalid(bool)` | Its line is in the destructive color; a field's judgment sets it |
| `size(size)`, `disabled(bool)` | For the whole group: a `ToggleGroupSize` (a `ButtonSize`) |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a button has them. `force` draws the toggle Tab stops on |

The group is generic over its values: `new` and `controlled` make a `ToggleGroup<T>`, whose `on_change` is told `&T`, and `multiple` and `controlled_multiple` a `ToggleGroup<T, [T]>`, told `&[T]`; the compiler checks what a handler is told. `T` is `()` where nothing names it, as in the theme's `resolve::<ToggleGroup>`. A choice is a value and a label, `(value, "Label")`, or a `Choice` with an icon, the icon alone, or disabled. Tab stops in the group once; the arrow keys move between its toggles, and Space or Enter turns the one the focus is on. Parts: `ToggleGroupPart::Root`, `Item` (laid over a toggle's root) and `Separator`. States: `ToggleGroupState::Horizontal`, `Vertical`, `Disabled`, `Invalid`.

In a [toolbar](#toolbar) the group takes the bar's size and is no stop of its own: its toggles are among the bar's controls, and the bar's Left and Right move through them and on. Where the bar is too narrow for it, the whole group goes into the bar's menu, after a line, as a checked item for each toggle, on while it is; choosing one presses that toggle.

## Segmented Control

One of a few choices that are all in view, side by side on a track.

```rust
use lurq::ui::SegmentedControl;

SegmentedControl::new(period.clone(), [(Period::Day, "Day"), (Period::Week, "Week"), (Period::Month, "Month")]);

// The app holds what is chosen, and is told what it is to be.
SegmentedControl::controlled(theme.preferred_appearance(), [(None, "System"), (Some(Light), "Light")], |look| {
  theme.set_preferred_appearance(*look)
})
.label("Appearance");
```

| Builder | |
| --- | --- |
| `SegmentedControl::new(signal, choices)` | The chosen segment is the one with the signal's value; a press of another sets it |
| `SegmentedControl::controlled(value, choices, on_change)` | The app holds the value: a press of another segment tells `on_change` (`Fn(&T)`) the value it is to be, and a press of the chosen one tells nothing |
| `on_change(handler)` | Called as a person chooses another segment, with its value (`\|period: &Period\|`), checked by the compiler: the control is a `SegmentedControl<T>`. Not called where the app sets the signal. Each one adds |
| `label(&str)` | What the control is called to what reads the tree, a `group`; a [field](#field)'s label where it has none |
| `invalid(bool)` | The track has a line in the destructive color; a field's judgment sets it |
| `size(size)` | A `SegmentedControlSize` (a `ButtonSize`): the track is as high as a button of that size |
| `full_width(bool)` | As wide as what it is in, its segments sharing the width |
| `tab_stop(bool)` | `false`: Tab passes its segments by and a press chooses one and leaves the focus where it was; the arrows move along them once the app focuses one. See [pointer-only pressables](#pointer-only-pressables) |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a button has them. `force` draws the segment Tab stops on: the chosen one, else the first that is not disabled |

- The control is one choice: Tab stops in it once, at the chosen segment; Left and Right move the focus to the next segment and choose it, Home and End the first and the last, and a click chooses one.
- A choice is a `(value, label)` or a `ui::Choice`: `icon(name)`, `icon_only(bool)` and `disabled(bool)`, and of its own `id(..)` (the segment's id, after which its parts are named: `<id>-label`, `<id>-count`), `count(n)` (a number after the label, in the part `Count`, 12 in `MutedForeground`), `state(name, on)` (a state of the theme's recipe on that segment alone, resolved over the control's and `Selected`: a choice that warns) and `element_ref(ElementRef)` (on the segment, for a popup it opens to hang from). A [radio group](#radio) draws the same; a toggle group and a chip group take a choice's `id` and `state`.
- Parts: `SegmentedControlPart::Root`, the track (`Muted`, padded 3, its segments 2 apart); `Segment`; `Label`; `Icon`; `Count`. States: `Selected`, of a segment (a `Background` surface with a small shadow); `Disabled`; `FullWidth`; `Invalid`.

## Link

Text that goes somewhere when it is pressed. It has no box, so it sits in a line of text or beside one.

```rust
use lurq::ui::Link;

Link::new("Read the docs").external(true).on_click(move |_| navigator.push("/docs"));
// A page on the web: a press opens it in the browser.
Link::new("lurq.dev").href("https://lurq.dev").external(true);
```

| Builder | |
| --- | --- |
| `on_click(handler)` | A click, and Enter and Space while it is focused. Where the link goes is what its press does |
| `href(&str)` | The address a press opens in the browser ([`open_url`](../ctx/#opening-an-address)), after the `on_click` handlers are told, unless one of them calls `event.prevent_default()`. An app with a router gives none, and goes to its route in `on_click` |
| `trailing_icon(name)`, `external(bool)` | An icon after the text; `external(true)` is the arrow of a link that leaves the app, where it has no trailing icon of its own |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `force(..)`, `id`, `key` | As a button has them |

Parts: `LinkPart::Root`, `Label` (14/500), `Icon` (14, four after the text). While the pointer is on the link or the focus is in it, the label has an `underline` (a `within` rule: the root is an interaction group): a straight line under the text alone, not under the icon. The root's own edge would curl up at its ends, rounded as it is for the focus ring.

## Fab

A floating action button: the one thing a view is for, raised over it.

```rust
use lurq::ui::{Fab, icons};

Fab::new("Compose", icons::COMPOSE).on_click(|_| compose());
Fab::new("Compose", icons::COMPOSE).extended(true); // with its label
```

- It is a [button](#button) that is an icon alone, round, 48 across, with an icon of 20 and the theme's large shadow, and takes what a button takes: `variant`, `on_click`, `disabled`, `loading`.
- Parts: `FabPart::Root`, `Icon`, `Label`, each laid over the theme's recipe for the same part of a button. States: `FabState::Extended`, `Disabled`, `Loading` (a button's own rules dim it and turn its spinner). `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id` and `key` as a [button](#button) has them.
- Where it floats is the app's to say: it is an element, put in a corner of a `Stack`. A container clips what is in it, so give its shadow room, or let the container show what is outside it (`.overflow_visible()`).

## Switch

A setting that is on or off and takes effect at once: dark mode, sync in the background.

```rust
use lurq::ui::Switch;

// Bound to the app's signal, which a press turns.
Switch::new("Dark mode").bind(dark.clone());

// The app holds it, and is told of a press.
Switch::new("Reduce motion")
  .on(motion.get())
  .on_change(move |on| motion.set(on));

// The track alone, in a row of settings that has the text already.
Switch::new("Sync in background").label_hidden(true).bind(sync.clone());
```

| Builder | |
| --- | --- |
| `on(bool)` | Whether it is on. The switch does not keep it: the app says so |
| `bind(Signal<bool>)` | On while the signal is true, which a press turns: `on` and a handler that sets the signal |
| `on_change(handler)` | Called on a press with what the switch is to be: on if it was off. Bound, it is called after the signal is set, and not when the app sets it. A second handler is called after the first |
| `invalid(bool)` | What it holds is wrong, where a field says so: the track is edged in the destructive color |
| `label_hidden(bool)` | The track alone. The label is still what the switch is called |
| `whole(bool)` | The track and the label are one control: the root is what is pressed, focused and ringed, one stop of Tab, and the id is on it, the parts named after it (`x-track`, `x-thumb`, `x-label`). State `SwitchState::Whole` |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the track, or of the whole where it is `whole` |

- A press of the track or of the label asks for the other state. Tab reaches the track, Space and Enter press it, and the focus ring is the track's; a whole switch's are the root's, round the track and the label, with the small radius.
- A press of a switch or of its label is the switch's alone: a row around it that is pressed (a list item, a row of settings with its own press) is not pressed by it.
- Parts: `SwitchPart::Root` (the track and the label, side by side), `Track` (what is pressed), `Thumb`, `Label`. States: `SwitchState::On`, `Invalid`, `Disabled`, `Whole`.
- In the design the track is 36 by 20 and round, in the `Input` color and in `Primary` while it is on; the thumb is 16, in `Background`, 2 from the edge; the label is 8 beside the track in the body's type. A disabled switch is dimmed with its label. The board draws no invalid switch: an invalid one has an edge of 1 inside its track in `Destructive`, as an invalid checkbox's box has. A [field](#field) with an error makes its switch invalid.
- The thumb slides: the default renderer moves it by an offset, how far is `SwitchView::travel()` (the track's width less its padding and the thumb), and the recipe gives the thumb a transition of `Transition::offset_x()` over the theme's fast duration. A theme with reduced motion moves it at once.
- To what reads the tree (`lurq_snapshot` of the MCP server) the track is a `switch` called by its label, with `a11y_checked` saying whether it is on.
- A renderer that draws another switch starts from `view.track()`, the behavior with the track's style, and puts in it what it likes; `view.thumb()` is the default thumb. Where `view.whole` is true it starts from `view.whole_root()`, the behavior with the root's style, and puts the track and the label in it.

## Input

One line of text that a person types: a name, an email address, a search.

```rust
use lurq::ui::{Input, InputSize, icons};

Input::new(name.clone()).placeholder("Display name");

Input::new(query.clone()).placeholder("Search components").icon(icons::SEARCH).size(InputSize::Sm);

// What the app checked: the edge says so.
Input::new(email.clone()).invalid(!email_ok);
```

| Builder | |
| --- | --- |
| `new(Signal<String>)` | What the input shows; what is typed is set on it, and what the app sets on it is shown |
| `placeholder(&str)` | What it says while it is empty |
| `label(&str)` | What it is called, to what reads the tree. It draws nothing: a [field](#field) gives it its own label |
| `icon(name)` | A glyph of the theme's set at the start of the box |
| `size(InputSize)` | `Sm` 32, `Md` 36 (the default), `Lg` 40 high, with text of 13, 14, 15 |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | What the app judged of what it holds: the edge in the destructive, warning or success color. One is shown, in that order |
| `read_only(bool)` | It is focused, selected and copied, and every edit is refused |
| `full_width(bool)` | As wide as what holds it, in place of 260 |
| `error_icon(name)` | An icon before the description while the field is invalid (its error, or a description of an invalid field): 14, 6 before the words, in their color, centred on them (part `Icon`). A field that is not invalid draws none |
| `control_end(bool)` | The label and the description on the left, one over the other and 2 apart, taking the row's room (part `Text`), and the control at the row's end, 16 from them and in the middle of their height: a row of settings with its switch or select. The orientation is not read; a fluid field is not drawn so. State `FieldState::ControlEnd`; with `full_width(true)` as wide as what holds it |
| `on_input(handler)` | Called before an edit with the `TextInputEvent`; `prevent_default()` refuses it |
| `on_submit(Fn(&str))` | Called with what it holds when Enter is pressed in it, with no modifier and outside a composition. A second handler is called after the first |
| `completion(&str)`, `squiggles(ranges)` | What would complete what is typed, drawn after it and taken by Tab; and wavy lines under ranges of its text, what the app's spelling checker marks. See [text editing](#text-editing) |
| `name(..)` | The name of its value in a form (feature `form`) |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the control, the box that is typed in |

- It is the engine's `TextInput`, single line, dressed by the recipe: the control *is* the box, so that the ring of its focus is drawn on it. A text input is focus-visible however it was focused, so a click shows the ring too. Tab reaches it wherever it is (`tab_index(0)`).
- Parts: `InputPart::Root` (the place it takes: its width, and the room between the icon and the text), `Control` (the box: edge, fill, height, the type of the value), `Placeholder` (laid over the control's type), `Icon`. States: `InputState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`, `Icon`. Sizes: `InputSize::Sm`, `Md`, `Lg` (`InputSize` is `TextBoxSize`, which a textarea has too).
- In the design the box is 260 by 36, `Background` in an edge of 1 in the `Input` color, `radius-md`, the text 12 from the edge in the body's type, the placeholder in `MutedForeground`. The edge is the only signal of its state: `Ring` under the pointer, a line of 2 in `Ring` inside the box in the focus (an outline, so that the edge under the pointer does not take it away), `Destructive`, `Warning`, `Success` for a judgment (which keeps its color under the pointer and in the focus). Disabled is half opaque on `Muted`; read only is on `Muted` in the `Border` color. The icon is 16 in `MutedForeground` (in `Success` while it is valid), 8 before the text.
- In a [field](#field) it is called by the field's label unless it has a `label` of its own, it is judged as the field is unless the app judged the input itself, and it is disabled where the field is.
- A text input does not take the foreground of its node, so the renderer names the color of the value and of the placeholder from the parts' foregrounds (`InputView::value_style()`, `placeholder_style()`).
- To what reads the tree the control is a `textbox`, with `a11y_invalid`, `a11y_readonly` where it is read only, and the name its label or its field's gives it (`InputView::label`).
- A renderer that draws another input starts from `view.control()`, the bound text input in the control's style; `view.icon()` is the icon laid over its start, and `view.text_start()` how far the text starts from the edge.
- What an input and a textarea have in common is one recipe's worth of rules on the box (its edge and fill, the focus inside it, the placeholder's color, the judgments, read only, disabled, the type of the sizes): a theme that changes one of them for both patches each recipe, by the same part and state names.

## Field

A control with what it is called above it, and under it what it is for or what is wrong with what it holds. The field finishes its control: the label names it, a press of the label focuses it, and an input in a field with an error is invalid without being told.

```rust
use lurq::ui::{Field, Input, Orientation, ShortcutRecorder, Switch, Textarea};

Field::new("Display name", Input::new(name.clone()).full_width(true))
  .description("Shown in the title bar.")
  .required(true);

// The error makes the input invalid: its edge is destructive.
Field::new("Email", Input::new(email.clone()).full_width(true))
  .error("Enter a full email address.");

Field::new("What happened", Textarea::new(story.clone()).full_width(true)).warning(true);

// Any element is a control; the label beside it, where a form lines its labels up.
Field::new("Notifications", Switch::new("Pause notifications")).orientation(Orientation::Horizontal);

// A control that keeps state of its own: the field mounted where it is declared mounts it.
Field::new("Command palette", ShortcutRecorder::new(palette.clone()))
  .description("Opens the palette from anywhere.")
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `new(&str, impl FieldControl)` | Its label and its control: an `Input`, a `Textarea`, another component, any `Element` (an engine node goes in with `.into()`). An empty label draws none above the control and names it nothing: a checkbox with its own label beside it, and the field's description under it |
| `description(&str)` | What the control is for, under it |
| `error(&str)` | What is wrong, in place of the description; the field is invalid |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | The description in the destructive, warning or success color, and the control judged so |
| `disabled(bool)` | Its control takes no input, and a press of its label focuses nothing |
| `required(bool)` | A `*` after the label |
| `optional(bool)`, `optional_text(&str)` | "Optional" after the label, or the app's own words. A required field shows none |
| `info(&str)` | What the label needs explained: the mark of an [info label](#info-label) after the label, 6 after what is before it, with the words in a tooltip above it. The tooltip shows on a field that is mounted (`mount(ctx)`), and the mark is then the field's first stop of Tab; on one converted into an element the mark says the words to tooling alone |
| `orientation(Orientation)` | `Vertical` (the default): the label above the control. `Horizontal`: the label in a column of 110 beside the control and its description; the field is at least the label's column, the gap and the control. States `FieldState::Horizontal` and `Vertical` |
| `full_width(bool)` | As wide as what holds it, in place of 260 |
| `fluid(bool)` | The label inside the box of the control, over what it holds: a [fluid field](#fluid-field) |
| `Field::unlabeled(impl FieldControl)` | A field with no label: the control alone with what the field says under it. The control keeps the name it has of its own (its hidden label, its `label`); no control takes an empty name from a field. Beside its label (`Horizontal`) the field keeps the label's column, empty, so that its control is in line with the others of the form |
| `below(element)` | An element of the app's under the description, after those given before, one under another, 4 apart (part `FieldPart::Below`): rows of errors the control found, a count, a link |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the field's root |
| `mount(ctx)` | The field built where it is declared, its control mounted: see below. Converting it (`.into()`) builds it with its control as it is |

- A control that keeps state of its own between renders, which outside a field the app mounts, is mounted by a field the app mounts (`Field::mount(ctx)`): the field tells it what it says, then mounts it (`FieldControl::mount_in_field`), and it is the same control in the field as outside it, with the field's label, description and error. These are the [shortcut recorder](#shortcut-recorder) (it records), the [date field](#date-field) and the [time field](#time-field) (they keep what is half typed), the [date picker](#date-picker), the [date range picker](#date-range-picker) and the [time picker](#time-picker) (they open), the [month picker and the year picker](#month-picker) (they keep the year shown), the [password input](#password-input) (its eye turns), the [rating](#rating) (the stars under the pointer are lit), the [file upload](#file-upload) and the [drop zone](#drop-zone) (a drag over them shows) and the [listbox](#listbox) (a virtual list). A field converted into an element builds its control as it is converted, which keeps nothing: it shows its value and takes the field's words, but does none of that. Any other control is the same converted or mounted. A mounted field goes in a [fieldset](#fieldset) as a child: `.child(field.full_width(true).mount(ctx))`.
- A field is at least as wide as its control: one wider than the recipe's 260 that does not fill the field (an [input OTP](#input-otp) of six cells) widens it, and the label and the description stay 260. Beside the label the field is at least the label's column of 110, the gap of 12 and the control, and the description wraps in the 238 the recipe's 360 leaves it. A description longer than the field wraps. Beside the label, a control lower than an input (a switch) stands in the middle of the label's height, level with it.
- A field's control is a `FieldControl`: what the field tells it (`InField`: the label, the judgment, whether it is disabled, the description) it takes as far as it has the prop, and only where the app did not say it of the control itself: an input with a label of its own keeps it, and one the app judged keeps that judgment whatever the field says. A control of an app's own implements `FieldControl` and passes what it is told to the control in it; one that keeps state of its own implements `mount_in_field(self, &InField, &mut Ctx)` too, which tells it the same and mounts it (by default it is `in_field`). Any `Element` takes none of it.

| Control | Takes from a field |
| --- | --- |
| `Input`, `Textarea`, `Select` | Its name (the field's label), its judgment (invalid, warning, valid), disabled |
| `Checkbox`, `Radio` | Its name where its own label is hidden (the label a person reads is then the field's), invalid, disabled |
| `Switch` | Its name where its own label is hidden, disabled |
| `CheckboxGroup`, `RadioGroup` | Its name (`label`, on the group), invalid on every box or radio, disabled |
| `Slider`, `RangeSlider` | Its name, disabled |
| `SegmentedControl`, `ToggleGroup` | Disabled; each segment or toggle keeps its own name |

- The field's description, or its error while it has one, is the control's description to what reads the tree (`a11y_description`), whatever the control: the field puts it on each outermost node in the control that says what it is (`a11y_role`) or takes the focus, so a radio group is described and not each radio in it. `lurq_inspect` reports it as `description`.
- A press of the label focuses the control: the first node under what holds it that takes the focus (`.focuses(&ElementRef)` on the label, the ref on the row that holds the control). A control that is disabled is not focused, and a disabled field's label focuses nothing. A renderer that holds the control another way puts `view.focus` on what holds it.
- An input or a textarea that is to fill the field says so (`Input::full_width`, `Textarea::full_width`).
- **In a form.** The form controls of `lurq::components` (feature `form`) are these components bound to a form's control: `FormTextInput` is a field holding an input (a textarea where it is multiline), `FormCheckboxInput` a field with no label above a checkbox with its label beside it, `FormSliderInput` and `FormRangeSliderInput` a field holding a slider as wide as it, and `FormPrimaryButton` and `FormSecondaryButton` a primary and an outline button as wide as what holds them. The form's error is the field's error, a required control has the field's mark, a disabled one (or one of a form that is submitting) is disabled, and every one of them is as wide as what holds it. They are styled by these recipes and no other: see [Forms](../forms/#form-controls).
- Parts: `FieldPart::Root`, `Label` (the label and its mark), `Required`, `Optional`, `Body` (the control and the description, where the label is beside them), `Description`, `Icon` (the error's), `Text` (the label and the description, where the control is at the end), `Below` (what the app put under the description). States: `FieldState::Invalid`, `Warning`, `Valid`, `Horizontal`, `FullWidth`, `Fluid`, `ControlEnd`, `Disabled` (no rule of the design's: a theme dims what it likes).
- In the design the field is 260 wide; the label is 6 above the control in a field label's type (13, medium) and `Foreground`, the description 6 under it in a caption's (12) and `MutedForeground`. The `*` is 4 after the label, semibold, in `Destructive`; "Optional" is 11 in `MutedForeground`. Beside the control the label is 110 wide and as high as a control, 12 from it, and the field is 360 wide.

**In a narrow room.** A field whose label stands beside its control puts the label over the control where the
field's own room, not the window's, is narrower than the two need side by side: the label is then as high as its
text, and the control fills the row under it. With room again they stand side by side. The board draws the label beside its control (Fieldset, the horizontal field) and no narrow form of it: the label over the control is the form of its other fields.

### Fluid Field

A field whose label is inside the box of its control, over the value: the design's Fluid Field (`imRnW`), for a form whose fields stand edge to edge. It is a form of the field, `Field::fluid(true)`, for the controls that have a fluid form, which every text control a field holds has: an [input](#input), a [textarea](#textarea), a [select](#select), a [password input](#password-input), a [search field](#search-field), a [number input](#number-input), a [combobox](#combobox), a [multi select](#multi-select), a [date field](#date-field), a [time field](#time-field), a [date picker](#date-picker), a [date range picker](#date-range-picker), a [time picker](#time-picker), a [token input](#token-input), a [mentions field](#mentions) and a [color field](#color-field). A month picker and a year picker are panels of a grid, not boxes that hold a value: they have no fluid form.

```rust
use lurq::ui::{DatePicker, Field, Input, NumberInput, Select, Textarea};

Field::new("Workspace name", Input::new(name.clone())).fluid(true);

Field::new("Notes", Textarea::new(notes.clone()).rows(2, 5))
  .fluid(true)
  .description("Shown to everyone in the workspace.");

Field::new("Theme", Select::new(theme.clone(), [("system", "System"), ("dark", "Dark")]))
  .fluid(true)
  .required(true);

Field::new("Seats", NumberInput::new(seats.clone()).min(1)).fluid(true);

// A control that keeps something between renders is mounted by the field, and has the form there too.
Field::new("Due date", DatePicker::new(due.clone()).clearable(true))
  .fluid(true)
  .mount(ctx);
```

- The control is the box, as wide as the field: on `Muted`, round at its top (6) and square at its bottom, with a line of 1 in `Foreground` along its bottom and no edge on its other sides. The label lies over its top, 12 from its sides and 10 from its top, in a caption's type (12) and `MutedForeground`, on one line, cut with an ellipsis where the box is narrower; the mark of a required or an optional field is after it, and the mark that explains it (`Field::info`) after that, which alone of the label takes the pointer, for its tooltip: a press elsewhere on the label goes to the control. What the control holds is 4 under the label's line and 10 over the bottom: a control of one line is 56 high, a textarea as high as its rows and the 39 around them. The field is 240 wide, or as wide as what holds it (`full_width`).
- The line says what the edge of a box says, where the board is silent: the ring's color under the pointer, `Destructive`, `Warning` or `Success` for the field's judgment (an error makes it destructive, as in any field), `Border` when the control is read only. The focus is a line of 2 inside the box, as an input's is. A disabled one is at half its opacity.
- The description, or the error, is 6 under the box, as any field's. A press of the label goes to the control under it: an input takes the caret, a select opens.
- What stands in the box beside the value stands on the line of the value, under the label, and not in the middle of the box: an input's icon, a password's eye, a search's clear button, a picker's button, a color's swatch. The board draws the form for an input alone; the others follow it, each with what is particular to it:
  - A password input has its eye 12 from the end of the box; its meter and what the meter says are under the box, as wide as it.
  - A search field is padded 12 at its start, so that its icon starts where the label does; its clear button, or the hint of its key while it is empty, is at its end.
  - A number input has its buttons on the line of the number, where its layout has them: each 25 high with the small radius, from the label's line to 4 under the number's, and the lines beside them as high. Stacked, they are one over the other about the number's line, down to the line of the box.
  - A combobox is a select's fluid trigger: what is typed, the cross that clears and the chevrons are on one line.
  - A multi select is at least 56 high. A tag (24) is higher than a line of text, so its box is padded 26 over its tags and 6 under them, and 12 at its start: with one line of tags, or with none, it is 56, and each line more adds a tag and the 6 between lines.
  - A date field and a time field are 56 high, their segments and their glyph on one line, padded 9 at the start so that the first segment's text starts where the label does.
  - A date picker, a date range picker and a time picker are the input's fluid box, with the button that opens the panel, and a clearable date picker's cross, on the line of the value. A time picker, 160 wide on its own, is as wide as the field.
  - A token input is padded 12 and 56 high on one line, its tokens in the middle of the line of its text; on rows it is as high as its rows (28 each) and the 39 around them.
  - A mentions field is a token input on rows: its two rows of 21 and the 39 around them, and higher as its text takes more rows.
  - A color field has its swatch and its eyedropper on the line of the hex; while its picker is open its line is in the ring's color.
- A control says that it has the form with `FieldControl::has_fluid_form`, and takes it when the field tells it `InField::fluid`: a control of an app's own that wraps an input passes both on. A fluid field of a control that has none (a switch, a slider, a month picker) is drawn as any field, its label above the control. The label is always over the value: a fluid field's `orientation` is not read.
- States: `FieldState::Fluid` on the field (its `Label` and its `Root`), and on the control the state `Fluid` of its own enum, named `fluid`: `InputState`, `TextareaState`, `SelectState`, `PasswordInputState`, `SearchFieldState`, `NumberInputState`, `ComboboxState` (a multi select's too), `DateFieldState` (a time field's too), `DatePickerState` (a date range picker's too), `TimePickerState`, `TokenInputState`, `MentionsState` and `ColorFieldState`. Its rules are on the part that is the box: the `Control`, the `Trigger` of a select, a combobox and a multi select, the `Root` of a date field and a time field. A renderer of a field has `view.fluid` and `view.fluid_label()`.
## Info Label

What a field or a setting is called, with a mark after it that explains it: "API key", and that it is stored in the keychain.

```rust
use lurq::{app::ctx::Placement, ui::InfoLabel};

InfoLabel::new("API key").info("Stored in the OS keychain").mount(ctx);

// The tooltip on another side, with an arrow.
InfoLabel::new("Region")
  .info("Where the data of the workspace is kept")
  .placement(Placement::Bottom)
  .arrow(true)
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `new(&str)` | The text of the label |
| `info(&str)` | What the mark explains, in a few words: the text of its tooltip. Without one, or with an empty one, there is no mark |
| `info_label(&str)` | What the mark is called to tooling, in place of "More information" |
| `icon(name)` | The icon of the mark, in place of the "i" |
| `placement(Placement)`, `offset(..)`, `collision(..)`, `arrow(bool)` | Where the tooltip stands, as a [tooltip](#tooltip) has them. Above the label, from its start (`Placement::TopStart`), unless told |
| `force_open(Option<bool>)` | The tooltip shown whatever the pointer and the focus do, or never |
| `mount(ctx)` | The label where it goes, with its tooltip on it. Converting it (`.into()`) builds its still form: the label and the mark, on which nothing shows |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the root's, and the mark is `<id>-mark`; `force` draws the mark under the pointer or focused |

- The explanation is the theme's [tooltip](#tooltip), mounted on the label: it shows after the pointer has rested on the label or on its mark for the tooltip's delay, and at once when the keyboard's focus comes to the mark, which is a stop of Tab; it goes when they leave, on a press, and on Escape. It is a mounted component for that reason alone: `mount(ctx)` puts the tooltip on it.
- A label longer than the room it has ends in an ellipsis on its one line, and the mark keeps its place after it.
- To what reads the tree the mark is an `img` called "More information", or by `info_label`, and described by what it explains (`a11y_description`), in the still form too.
- A [field](#field) draws the same mark after its own label: `Field::new(..).info("…")`. An explanation with a link in it, or one that opens on a press, is a [toggletip](#toggletip)'s.
- Parts: `InfoLabelPart::Root` (the label and the mark, 6 apart), `Label`, `Mark` (the "i": its side, its color, and its ring while it is focused from the keyboard). State: `InfoLabelState::Info` (it has something to explain).
- In the design the label is a field label's text (13, medium) in `Foreground`, and the mark an "i" of 14 in `MutedForeground` 6 after it; under the pointer and focused the mark is in `Foreground`, and focused from the keyboard it has the theme's ring around its circle. The board draws no state of the mark: these are a link's.
- A renderer that draws another label has `view.root()`, `view.mark()` (`None` with nothing to explain) and `view.label`; when the tooltip shows stays the label's.

## Textarea

Text of more than one line, typed into a box: a bio, a note, what happened.

```rust
use lurq::ui::{Textarea, TextareaSize};

// bio: Signal<String>
Textarea::new(bio.clone()).placeholder("A few words about you");

// Two rows while it holds less, four before its text scrolls; wrong.
Textarea::new(report.clone()).rows(2, 4).invalid(true);

// Read, selected and copied, not changed; in small type.
Textarea::new(terms.clone()).read_only(true).size(TextareaSize::Sm);
```

| Builder | |
| --- | --- |
| `new(Signal<String>)` | What it holds. What is typed is written into the signal, and what the app writes there shows |
| `placeholder(&str)` | What it shows while it holds nothing |
| `rows(min, max)` | The rows it shows at least, and at most before its text scrolls inside it. 3 and 8 unless told (`TEXTAREA_MIN_ROWS`, `TEXTAREA_MAX_ROWS`) |
| `size(TextareaSize)` | The type of an input's sizes: `Sm` 13, `Md` 14 (the default), `Lg` 15, each with a line of 1.5. A row is that high |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | What the app judged of what it holds: the edge in the destructive, warning or success color. One is shown, in that order |
| `read_only(bool)` | It is focused, selected and copied, and every edit is refused |
| `on_input(handler)` | Called before an edit with the `TextInputEvent` (`new_value` is what it will hold); `prevent_default()` refuses it. A read-only one makes no edit |
| `squiggles(ranges)` | Wavy lines in `Destructive` under ranges of its text, bytes of it: what the app's spelling checker marks. See [text editing](#text-editing) |
| `full_width(bool)` | As wide as what holds it, in place of 260: in a form, as wide as its field. The state `FullWidth` |
| `name(..)` | The name of its text in a form (feature `form`) |
| `label(&str)` | What it is called, to what reads the tree. It draws nothing: a [field](#field) gives it its own label |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box |

- It is the engine's multiline `TextInput`: as high as its text between its rows, its own scroll bar past them, Enter a new line. Tab reaches it wherever it is, as every control of the design system; a disabled one is passed by and takes no click. A read-only one is a stop of Tab and refuses every edit.
- Parts: `TextareaPart::Root` (the place it takes: its width, and its opacity while disabled), `Control` (the box, which is the text input: its edge, fill and padding, and the type and color of what it holds, as an input's control), `Placeholder` (laid over the control's type). States: `TextareaState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`. Sizes: `TextareaSize::Sm`, `Md`, `Lg` (`TextareaSize` is `TextBoxSize`, an input's sizes).
- In the design the box is 260 wide, as an input is, in `Background` with an edge of 1 in `Input`, the medium radius, padded 12 across and 8 down; its text is in the body's type with a line of 1.5, in `Foreground`, and the placeholder in `MutedForeground`. Under the pointer the edge is `Ring`; focused, a ring of 2 is drawn inside the box, over the edge. Invalid, warning and valid, the edge and the ring are `Destructive`, `Warning` and `Success`. Read only, it is on `Muted` in the `Border` color, as a read-only input. Disabled, it is at the theme's disabled opacity on `Muted`. Small, its text is 13; large, 15; the padding stays.
- In a [field](#field) it is called by the field's label unless it has a `label` of its own, it is judged as the field is unless the app judged it itself, it is disabled where the field is, and a press of the label focuses it.
- A text input draws in its own text style and does not take the foreground of what it is in, so the `Text` and `Placeholder` parts' foregrounds are resolved to colors of the theme when the textarea is built (`TextareaView::text_style`, `placeholder_style`).
- To what reads the tree it is a `textbox`, `a11y_multiline`, with `a11y_invalid`, `a11y_readonly` where it is read only, and the name its label or its field's gives it.
- A renderer that draws another textarea starts from `view.input()`, the behavior in the box's style, and puts what it likes around it: a count of characters, a grip.

## Checkbox

A choice that is made or not, among others a form takes at once: send crash reports, keep me signed in, a row of a list picked.

```rust
use lurq::ui::Checkbox;

// Bound to the app's signal, which a press turns.
Checkbox::new("Keep me signed in").bind(signed_in.clone());

// The app holds it, and is told of a press.
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
| `bind(Signal<bool>)` | Checked while the signal is true, which a press turns: `checked` and a handler that sets the signal |
| `indeterminate(bool)` | Some of what it stands for is checked: a dash, whatever `checked` says |
| `on_change(handler)` | Called on a press with what the checkbox is to be: checked if it was not, or was indeterminate. Bound, it is called after the signal is set, and not when the app sets it. A second handler is called after the first |
| `invalid(bool)` | What it holds is wrong: the box is edged in the destructive color |
| `label_hidden(bool)` | The box alone. The label is still what the checkbox is called |
| `name(..)` | The name it has in a form (feature `form`): checked, it gives the form `on`, as the engine's checkbox does, and nothing while it is not |
| `whole(bool)` | The box and the label are one control: the root is what is pressed, focused and ringed, one stop of Tab, and the id is on it, the parts named after it (`x-box`, `x-indicator`, `x-label`). State `CheckboxState::Whole` |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box, or of the whole where it is `whole` |

- A press of the box or of the label asks for the other state; a press of an indeterminate one asks for checked. Tab reaches the box, Space and Enter press it, and the focus ring is the box's; a whole checkbox's are the root's, round the box and the label.
- A press of a checkbox or of its label is the checkbox's alone: a row around it that is pressed is not pressed by it.
- Parts: `CheckboxPart::Root` (the box and the label, side by side), `Box` (what is pressed), `Indicator` (the check or the dash), `Label`. States: `CheckboxState::Checked` (checked and not indeterminate), `Indeterminate`, `Invalid`, `Disabled`, `Whole`.
- In the design the box is 16 with the small radius, on the `Background` with an inner edge of 1 in `Input`; checked or indeterminate it is filled and edged in `Primary`, with the theme's `check` or `minus` icon of 12 (`IconSize::Xs`) in `PrimaryForeground`. An invalid box is edged in `Destructive`, checked or not. A disabled checkbox is dimmed with its label, and its box is `Muted` while it is empty. The label is 8 beside the box in the body's type.
- To what reads the tree (`lurq_snapshot` of the MCP server) the box is a `checkbox` called by its label, with `a11y_checked` `true`, `false` or `mixed`.
- A renderer that draws another checkbox starts from `view.control()`, the behavior with the box's style, and puts in it what it likes; `view.indicator()` is the default mark, `None` while the checkbox is empty. Where `view.whole` is true it starts from `view.whole_root()`, with the root's style, and puts the box and the label in it.
- `CheckboxGroup<T>` is checkboxes over one list: `CheckboxGroup::new(signal, choices)` with a `Signal<Vec<T>>` checks the boxes whose values the signal has, and a press adds a value or takes it out; `CheckboxGroup::controlled(checked, choices, on_change)` leaves the list to the app, which is told the whole new set a press asks for (`&[T]`: `checked` with the value pressed added, or taken out) and draws what it says next. `on_change(&[T])` is told the whole new set after a press, bound or not, after the signal is set. `orientation(Orientation)` (`Vertical` unless told; `Horizontal` side by side, going on to the next line), `label(&str)` (its name to tooling: it draws nothing, a field gives it its label), `invalid(bool)` and `disabled(bool)` are for the whole group, and `Choice::disabled` takes one box out; `state`, `style`, `rules`, `force` (the first box that is not disabled), `id` and `key` are as a checkbox's. Each box stays its own stop of Tab. Its parts are `CheckboxGroupPart::Root` (12 apart, or 24 side by side) and `Item`, laid over each checkbox's root; its states `CheckboxGroupState::Horizontal` and `Vertical`, one of which is on. A theme's renderer gets a `CheckboxGroupView`: its checkboxes built, in `view.checkboxes`, and `view.render()`. A theme names the group without its type (`patch_recipe::<CheckboxGroup>`), for groups of every `T`.
- `lurq::components::Checkbox` is the engine's node, which keeps its value in a `Signal` and draws its mark from an image; `lurq::ui::Checkbox` is the design system's and does not use it. The engine's is deprecated for an app since 0.63: see [the migration guide](../migration-v0-63/#components-that-moved-to-lurqui).

## Radio

One of a few choices that exclude each other, all of them in sight: the density of a list, the plan of an
account. `RadioGroup` is the choice; `Radio` is one radio, for a layout a group does not make.

```rust
use lurq::ui::{Choice, Radio, RadioGroup};

// The radio whose value the signal has is selected; a press of another sets it.
RadioGroup::new(density, [(Density::Comfortable, "Comfortable"), (Density::Compact, "Compact")]);

// A choice that cannot be made, in a group with nothing selected until a press: a `Signal<Option<T>>`.
RadioGroup::new(plan, [Choice::new("free", "Free"), Choice::new("enterprise", "Enterprise").disabled(true)])
  .on_change(|plan| println!("{plan}"));

// The app holds the choice, and answers each press of another.
let set = answer.clone();
RadioGroup::controlled(answer.get(), [(true, "Yes"), (false, "No")], move |yes| set.set(Some(*yes)))
  .invalid(answer.get().is_none());

// One radio alone.
Radio::new("Compact").selected(true).on_change(|selected| println!("compact: {selected}"));
```

| `RadioGroup<T>` | |
| --- | --- |
| `new(signal, choices)` | The radio whose value the signal has is selected, and a press of another sets the signal: a `Signal<T>`, or a `Signal<Option<T>>` of which `None` selects none |
| `controlled(Option<T>, choices, on_change)` | The app holds the choice: none while it is `None`; `on_change` is told the value of each radio pressed that was not selected, and what is drawn is what the app says next |
| `on_change(handler)` | Called with `&T`, the value of a radio pressed that was not the one selected, after the signal is set. A press of the one selected calls nothing, nor does the app setting the signal. A second handler is called after the first |
| `orientation(Orientation)` | `Vertical` unless told; `Horizontal`, the radios side by side, going on to the next line where there is no room |
| `label(&str)` | What the group is called to tooling. It draws nothing: a [field](#field) gives it its label |
| `invalid(bool)` | Every radio's edge in the destructive color: the choice was made wrongly, or not at all |
| `disabled(bool)` | The whole group takes no input. `Choice::disabled` takes one choice out |
| `whole(bool)` | Each radio pressed as a whole, as `Radio::whole` says; a choice's id is then on its whole. The group is one stop of Tab still |
| choices | A `(value, label)` or a `Choice`, whose `id(..)` is its radio's (its mark's, its parts named after it: `<id>-label`, `<id>-count`), `count(n)` a number after the label, `state(name, on)` a state of the theme's on its radio alone, and `element_ref(..)` a ref to its mark |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the radio Tab would stop on |

| `Radio` | |
| --- | --- |
| `selected(bool)` | Whether it is the one of several selected. The radio does not keep it: the app says so |
| `on_change(handler)` | Called with `true` on a press while the radio is not selected. A press of the one selected calls nothing |
| `label_hidden(bool)` | The mark alone. The label is still what the radio is called |
| `count(n)` | A number after the label, in the part `Count`: 12 in `MutedForeground` |
| `element_ref(ElementRef)` | Refers to the mark once it is laid out; to the whole where it is `whole` |
| `invalid(bool)` | The edge in the destructive color |
| `whole(bool)` | The mark and its label are one control (state `Whole`): what is pressed, focused and ringed is the whole of them, one stop of Tab, a `radio` to tooling; its id is on the whole and the mark, the dot, the label and the count are named after it (`x-mark`, `x-dot`, `x-label`, `x-count`). `RadioView::whole_root()` for a renderer |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the mark, or of the whole |

- A press of the mark or of the label selects the radio; a press of the one selected does nothing, so a choice
  once made is not taken back. A group is one stop of Tab, at the selected radio or the first; the arrow keys
  move the focus to the next radio and select it, Home and End the first and the last, and a disabled one is
  passed by. A radio on its own is a stop of Tab that Space and Enter press. The focus ring is the mark's, or the
  whole's. A press of a radio, its mark, its label or its whole, is its own: a row around it with an `on_click` of its
  own is not pressed by it, disabled or not.
- Parts: `RadioPart::Root` (the mark and the label, side by side), `Mark` (what is pressed), `Dot`, `Label`, `Count`.
  States: `RadioState::Selected`, `Invalid`, `Disabled`, `Whole`. A group's parts: `RadioGroupPart::Root` (the radios
  one under the other, 12 apart) and `Item`, laid over the theme's recipe for each radio's root; its states
  `RadioGroupState::Vertical` and `Horizontal` (side by side, 24 apart), one of which is on. A theme's
  renderer of a group gets a `RadioGroupView`: its radios built, in `view.radios`, and `view.render()`. A theme names
  the group without its type (`patch_recipe::<RadioGroup>`, `set_renderer::<RadioGroup>`), for groups of every `T`.
- In the design the mark is 16 and round, in `Background` with an edge of 1 inside it in `Input`; selected, the
  edge is `Primary` and a dot of 8 in `Primary` is in its middle. The edge of an invalid one is
  `Destructive`. The label is 8 beside the mark in the body's type, and a group's radios are 12 apart. A
  disabled radio is dimmed with its label.
- A choice's icon is not drawn: a radio is its mark and its label.
- To what reads the tree (`lurq_snapshot` of the MCP server) the mark is a `radio` called by its label, with
  `a11y_checked` saying whether it is selected, and a group is a `radiogroup`.
- A renderer that draws another radio starts from `view.mark()`, the behavior with the mark's style, and puts
  in it what it likes; `view.dot()` is the default dot.

## Slider

A value picked along a track, between a least and a greatest: volume, the size of a brush, a price no higher than. A range slider picks two on one track: a price between, the hours a shop is open.

```rust
use lurq::ui::{RangeSlider, Slider, SliderOrientation};

// volume: Signal<i32>
Slider::new(volume.clone()).label("Volume");

// opacity: Signal<f32>: a fraction, in steps of a twentieth.
Slider::new(opacity.clone()).step(0.05).label("Opacity");

// The app holds the value; told each change, and of the end of a drag.
Slider::controlled(volume, move |value| set_volume(value))
  .on_change_end(move |value| save(value));

// From 10 to 50 in steps of 5.
Slider::new(size.clone()).min(10).max(50).step(5);

// Up, the least value at the bottom; 100 high.
Slider::new(level.clone()).orientation(SliderOrientation::Vertical).length(100.0);

// price: Signal<(i32, i32)>, the low value first.
RangeSlider::new(price.clone()).step(5).label("Price");
```

| Builder | |
| --- | --- |
| `Slider::new(Signal<T>)` | The value it is bound to, `T` a `SliderNumber`: `i32`, whole numbers from 0 to 100 in steps of 1, or `f32`, fractions from 0 to 1 in steps of a hundredth |
| `Slider::controlled(value, on_change)` | The app holds the value, an `i32` or an `f32`: the slider shows it, tells `on_change` what a person makes it, and shows what the app answers with. A drag goes on over the renders the answers make |
| `RangeSlider::new(Signal<(T, T)>)`, `RangeSlider::controlled((low, high), on_change)` | The two values of a range slider, the low one first, in the same ranges and steps |
| `on_change(Fn(f64))` | Each change a person makes, as it is made: a drag as it moves, a press on the track, a key. Not what the app sets, nor a new range or step that snaps the value. A range slider's is `Fn((f64, f64))`, the two values. Each call adds |
| `on_change_end(Fn(f64))` | The value when a change is over, where it is another than it was as the change began: the drag let go, the press on the track or the key's step made. A drag taken back to where it began tells nothing. A range slider's is told the two. Each call adds |
| `min(v)`, `max(v)`, `step(v)` | The least value, the greatest and how far apart the values are, from the least. Any number: `min(10)`, `step(0.25)` |
| `orientation(SliderOrientation)` | `Horizontal`, or `Vertical`: 16 wide and 140 high, the least value at the bottom. `SliderOrientation` is `ui::Orientation` |
| `length(f32)` | How long the slider is along its track: its width across, its height up |
| `label(&str)` | What the slider is called, to what reads the tree. The slider shows no text: the field it is in does |
| `name(..)` | The name of its value in a form (feature `form`); a range slider gives its low value and its high one under it |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the slider's node |

- It is the engine's [`Slider`](../styling-events/#slider-styling) dressed by the recipe: a drag of the thumb, a press on the track and the keys set the signal, snapped to the step and kept in the range. Tab reaches it wherever it is (`tab_index(0)`), and the focus ring is the thumb's. A vertical slider is dragged along its height, and the up and down arrows move it.
- The keys: the arrows a step (right and up toward the greatest value), Page Up and Page Down a tenth of the range in whole steps and at least one, Home and End to the least and the greatest. The [engine's slider keys](../styling-events/#slider-keys) are the same for a vertical slider and for each thumb of a range slider.
- A range slider is the engine's slider with two thumbs. A press on the track moves the nearer thumb, a drag the thumb it began on, and neither passes the other, by the pointer or the keys. Each thumb is a stop of Tab, the low one first; the keys move the one focused, and the ring is around it. In a form it is `FormRangeSliderInput` (see [Forms](../forms/)).
- Parts: `SliderPart::Root` (the slider's node: its size, its cursor, its opacity), `Track`, `Fill` (the track from its start to the thumb, or between a range slider's thumbs), `Thumb` (both of a range slider's). States: `SliderState::Disabled`, `Horizontal` or `Vertical` (one of them is on), `Range`. A range slider is drawn by the slider's recipe and renderer in the state `Range`, so a theme that restyles the slider restyles it too, and `rules.state(SliderState::Range, ..)` styles it alone.
- In the design the slider is 240 by 16, and no longer than the room it is given: in a narrower row its track is shorter and its thumb stays on it, inside the slider, at every value; the track is 4 high and round, in `Secondary`; the fill is `Primary`; the thumb is 16, round, in `Background` with an edge of 1.5 in `Primary` inside it. The vertical one is 16 by 140, its track 4 wide and as high as it. A disabled slider is dimmed. `style(SliderPart::Root, |s| s.width(Length::Fill))` makes it as wide as what holds it.
- The engine paints the track, the fill and the thumb, not nodes: of their style it takes the width and the height in pixels, the fill, the corners, the border and the outline (`focus_ring()` too), at rest, under the pointer (`hovered`), and for the thumb focused from the keyboard (`focus_visible`). An opacity, a shadow or a transition of one of those three parts is not painted. On a vertical slider a part's width is its thickness.
- To what reads the tree (`lurq_snapshot` of the MCP server) it is a `slider` called by its label; a range slider's value is `low,high`.
- A renderer that draws another slider starts from `view.slider()`, the engine's slider bound to the value (or the two) with its orientation, the root's style and the length, and `view.dressed(slider)` gives it the recipe's track, fill and thumbs. `view.value()` reads the value, `view.values()` a range slider's two, and `view.is_range()` tells which it is.

## Angle Slider

A dial that sets an angle in degrees, by a drag round it, a press on it or the keys: the rotation of a layer, the direction of a gradient or of a light.

```rust
use lurq::ui::AngleSlider;

// rotation: Signal<f64>, 0 to 360 in whole degrees, going round past 359 to 0.
AngleSlider::new(rotation.clone()).label("Rotation");

// The app holds the angle; told each change, and of the end of a drag.
AngleSlider::controlled(angle, move |angle| preview(angle))
  .on_change_end(move |angle| save(angle));

// A half turn either way of 0, in steps of 15, held at its ends.
AngleSlider::new(tilt.clone()).min(-180).max(180).step(15).wrap(false);
```

| Builder | |
| --- | --- |
| `AngleSlider::new(Signal<f64>)` | The angle the signal holds, in degrees; a drag, a press and the keys set it |
| `AngleSlider::controlled(f64, on_change)` | The app holds the angle: it is told what a person makes it, and what is drawn is what it gives next |
| `on_change(Fn(f64))` | Each change a person makes, as it is made: a drag as it moves, a press, a key. After the signal is set; not when the app sets it, nor for a press that leaves it as it is. Each call adds |
| `on_change_end(Fn(f64))` | The angle when a change is over, where it is another than it was as the change began: the drag let go, the key's step made. A drag taken back to where it began tells nothing. Each call adds |
| `min(v)`, `max(v)` | The range: 0 to 360 unless told. A range of more than a turn is turned round more than once |
| `step(v)`, `page_step(v)` | How far apart the angles are, from the least: 1 unless told, what an arrow key moves it by and what a drag snaps to. What Page Up and Page Down move it by: 15 unless told, in whole steps and at least one |
| `wrap(bool)` | On unless told: where the range is a whole turn it goes round, a step past 359 is 0 and a drag goes on round. Off, or with another range, it stops at its ends, and a drag past an end stays there until it comes back |
| `label(&str)` | What it is called to tooling, "Angle" unless told. A [field](#field) gives it its label |
| `locale(&Locale)` | The words it speaks in, its name, in place of the app's (`App::set_locale(Locale::for_tag("uk"))`, English where it set none), whose words are `Locale::input`. A `label` told by itself stays; the degrees are a sign, the same in every locale |
| `value_text(Fn(f64) -> String)` | What it says the angle is, in its middle and to tooling: "45°" unless told, with the decimals of the step ("22.5°") |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the dial, what is dragged and focused |

- The angle is drawn as on a compass: 0 at the top and growing clockwise, the dot on the ring at it. An angle the app gives is snapped to the step and kept in the range; where it goes round, 370 is 10 and 360 is 0.
- The pointer: a press anywhere on the dial sets the angle it is at (of a range of more than a turn, the turn nearer to the angle it holds; past the ends of a range of less than a turn, the nearer end), and a drag goes on round from there, snapped to the step. A press at the dial's very middle, where a point has no angle, sets nothing and the drag goes on from where it moves. A finger or a pen turns it as the mouse does, and is taken from the page. A press focuses it, so the keys go on from it.
- The keys: Right and Up a step clockwise, Left and Down a step back, Page Up and Page Down by `page_step`, Home to the least and End to the greatest (where it goes round, the last step before it comes round to the least: 359). Each is a change and its end. Tab stops on it once; the ring of the focus is around the circle.
- It tells as a [slider](#slider) does: `on_change` for each change, then `on_change_end` for the end of the drag or the key's step; bound, the signal is set first.
- It is built of boxes: the circle, the text in its middle and the dot, placed at the angle, so the parts are styled as any others are.
- In the design the dial is 64 on `Background` with a ring of 1.5 in `Input` inside it, the angle in the mono type of 13 in `Foreground`, and a dot of 9 in `Primary` inside the ring, touching it. The board draws it at rest: the ring of the focus is around the circle, the pointer is a hand over it, and a disabled dial is dimmed. A dial made larger (`style(AngleSliderPart::Root, |s| s.square(96.0))`) keeps its dot touching its ring.
- To a screen reader it is a slider called by its label, of its range, that says its angle.
- Parts: `AngleSliderPart::Root` (the circle, what is focused), `Value`, `Handle`. State: `AngleSliderState::Disabled`.
- In a [field](#field) it is called by the field's label unless it has one of its own and disabled where the field is.
- A renderer that draws another dial is given an `AngleSliderView`: `view.root()` is the dial with its behavior (the pointer, the keys, the role), `view.value()` and `view.handle()` the text and the dot, `view.reach()` how far the dot is from the middle, `view.text()` what it says, `view.angle` the angle.

## Rating

A number of stars, chosen by a press or by the keys, or shown and not chosen: the quality of a call, how much a person liked a release, what others gave a package.

```rust
use lurq::ui::{Rating, RatingPrecision, RatingSize, icons};

// quality: Signal<f64>, 0 for none. Mounted, the stars up to the one under the pointer are lit as a press would set them.
Rating::new(quality.clone()).label("Call quality").mount(ctx);

// review: Signal<Option<f64>>, None until a rating is given; in halves, and a press of the value takes it back.
Rating::optional(review.clone())
  .precision(RatingPrecision::Half)
  .clearable(true)
  .on_change(|stars| save(stars))
  .mount(ctx);

// Shown and not chosen: an average lights the part of a star.
Rating::at(3.7).label("Average rating").size(RatingSize::Sm);

// Another icon.
Rating::at(3.0).icon(icons::HEART).empty_icon(icons::HEART);
```

| Builder | |
| --- | --- |
| `Rating::new(Signal<f64>)` | The stars the signal holds, `0.0` for none; a press or a key sets it |
| `Rating::optional(Signal<Option<f64>>)` | The same where none is a state of its own: `None` until a rating is given |
| `Rating::controlled(f64, on_change)`, `Rating::controlled_optional(Option<f64>, on_change)` | The app holds the stars: it is told what a press or a key asks for, and what is drawn is what it says next |
| `Rating::at(f64)` | Shown and not chosen from: the stars, the part of one too. It tells nothing and is no stop of Tab |
| `on_change(handler)` | Called with what a press or a key sets (`f64`, or `Option<f64>` for an optional rating), after the signal is set. Not for a press that changes nothing, nor when the app sets the signal. Each call adds |
| `max(u32)` | How many stars there are: 5 unless told |
| `precision(RatingPrecision)` | `Whole` unless told; `Half`, a press of the half of a star under the pointer and the keys choose halves. A value the app gives is drawn as it is |
| `clearable(bool)` | A press of the star that is the value takes it back to none; so do 0, Backspace and Delete |
| `read_only(bool)` | Shown and not chosen from; `at` is a rating that is |
| `size(RatingSize)` | `Sm` stars of 16, 2 apart; `Md` 20 and 4 apart, the board's; `Lg` 24 and 4 apart |
| `icon(name)`, `empty_icon(name)` | The icon of a lit star and of one that is not: `icons::STAR_FILL` unless told |
| `label(&str)` | What it is called to tooling, "Rating" unless told. It draws nothing: a [field](#field) gives it its label |
| `value_text(Fn(Option<f64>, u32) -> String)` | What it says it holds to tooling: "3 of 5 stars", and "No rating" for none, unless told |
| `invalid(bool)`, `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the row of stars, what takes the focus |

- To a screen reader it is a slider, the WAI-ARIA pattern for a rating that is one stop of Tab and takes halves: called by its label, of the range from none (or the first step where it is not clearable) to the number of stars, with its value in words. Tab stops on it once. Left and Down take a step off, Right and Up add one, Home and End go to the least and the most, a digit sets that many stars, and where it is clearable 0, Backspace and Delete set none. The ring of the focus is around the row of stars.
- The pointer: a press of a star sets that many, and in halves the half of the star under the pointer decides. A press focuses the rating, so the keys go on from it. Mounted (`mount(ctx)`), the rating keeps where the pointer is, and the stars up to the one under it are lit as a press would set them; the pointer gone, the value is lit again. Converted into an element, or given to a field, it takes the presses and the keys and lights nothing under the pointer.
- A value between two steps (3.7) is drawn as it is: the empty star, and over it the lit star in a room as wide as the part that is lit, which cuts the rest off.
- In the design the stars are solid, 20 and 4 apart, those lit in `Warning` and the rest in `Input`. The board draws no other state: an invalid rating has its stars that are not lit in `Destructive` at half its opacity, a disabled one is dimmed, and a rating that is chosen from has the pointer's hand over its stars.
- Parts: `RatingPart::Root` (the row of stars, what is focused), `Star` (the room of a star, what a press chooses), `Filled` (a lit star, or the lit part of one), `Empty`. States: `RatingState::Interactive`, `ReadOnly`, `Invalid`, `Disabled`; sizes `RatingSize::Sm`, `Md`, `Lg`.
- In a [field](#field) it is called by the field's label unless it has one of its own, invalid where the field is and disabled where the field is.
- A renderer that draws another rating is given a `RatingView`: `view.root()` is the row with its behavior (the keys, the role, the value), `view.star(index)` a star with its room for a press, `view.lit(index)` how much of a star is lit, `view.stars` the value and `view.shown` what is lit now.

## Signature Pad

A surface a person signs on with a mouse, a pen or a finger: the signature on a delivery, the consent on a form. Behind the `canvas` feature.

```rust
use lurq::ui::{Field, Signature, SignaturePad};

// signature: Signal<Signature>. Each stroke let go, Undo and Clear set it.
SignaturePad::new(signature.clone()).label("Signature of the recipient").mount(ctx);

// The app holds the signature, and is told what a person makes of it.
SignaturePad::controlled(held.clone(), move |signature| save(signature));

// In a field, which names it and judges it.
Field::new("Signature", SignaturePad::new(consent.clone()).placeholder("Sign to accept the delivery"))
  .error("Sign to accept the delivery.");

// The signature as a picture: its own size twice over, in the ink and the width given, on nothing.
let image = signature.get().to_image(2.0, ink, 2.0); // Option<ImageData>
let png = signature.get().to_png(2.0, ink, 2.0); // Option<Vec<u8>>, a PNG file
```

| Builder | |
| --- | --- |
| `SignaturePad::new(Signal<Signature>)` | The signature the signal holds; each stroke, Undo and Clear set it |
| `SignaturePad::controlled(Signature, on_change)` | The app holds the signature: it is told what a stroke, Undo or Clear make of it, and what is drawn is what it gives next |
| `on_change(Fn(&Signature))` | Called after a stroke is let go, after Undo and after Clear, once the signal is set. Not while a stroke is drawn, nor when the app sets the signal. Each call adds |
| `placeholder(&str)` | The words under the line, "Sign above the line" unless told. They stay once it is signed, as the board draws them, and end in an ellipsis where they do not fit; `placeholder("")` leaves the line alone |
| `read_only(bool)` | It shows the signature and takes nothing: no stroke, no buttons, no stop of Tab |
| `full_width(bool)`, `width(..)`, `height(..)` | As wide as what holds it; or a size of its own, 280 by 124 unless told, and no wider than what holds it |
| `label(&str)` | What it is called to tooling, "Signature" unless told. A [field](#field) gives it its label |
| `undo_label(&str)`, `clear_label(&str)` | The names of its two buttons, "Undo" and "Clear" unless told |
| `signed_text(&str)`, `empty_text(&str)` | What it says to tooling it holds: "Signed" and "Not signed" unless told |
| `locale(&Locale)` | The words it speaks in, its name, the words under its line, its buttons' names and what it says it holds, in place of the app's (`App::set_locale(Locale::for_tag("uk"))`, English where it set none), whose words are `Locale::input`. A word told by itself (`placeholder`, `undo_label` and the others) stays |
| `invalid(bool)`, `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the pad, what takes the focus; its canvas is `<id>-surface` and its buttons `<id>-undo` and `<id>-clear` |
| `mount(ctx)`, `build(theme)` | Converted into an element, or mounted, it is the pad that is signed on: it keeps the stroke being drawn. `build` is its still form, which shows the signature and takes nothing |

- **The value.** A `Signature` is the app's: `strokes`, each a `SignatureStroke` of `points` in the order they were drawn, and the `width` and `height` of the surface they were signed on. A `SignaturePoint` is `x` and `y` in logical pixels from that surface's top left, `pressure` from 0 to 1 where the input measures it (a pen, a screen that feels force; `None` for a mouse) and `time` in milliseconds from the first point of its stroke. A stroke of one point is a dot. It is `Clone`, `PartialEq` and inspectable, to keep in a signal and to store. Given back to a pad of another size, it is drawn scaled to fit from its top left, keeping its shape, and a stroke drawn on it joins it in its own pixels.
- **The pointer.** A press draws a stroke where the pointer goes until it is let go, and the stroke is shown as it is drawn; the signature is told only when it is let go. A move of less than three quarters of a pixel from the last point is no point. A stroke stays on the pad: one dragged past its edge goes along the edge. A finger or a pen draws as the mouse does, with its pressure, and is taken from the page: it does not scroll what the pad is in. A second finger while one draws draws nothing, and a touch the system takes away signs nothing.
- **The line.** Each stroke is a curve through the middles between its points (each point bends the line toward it), 2 wide in `Foreground` with round ends and joins. The board's line is even, so the pad's is: its width does not follow the speed or the pressure, which are in the value for an app that draws otherwise.
- **The keys and the focus.** The pad is one stop of Tab, and its Undo and Clear buttons the next two, in its top corner while it is signed. With the pad focused, Backspace takes back the last change, a stroke or a clearing, and Delete takes back every stroke. Its history is undo and redo to lurq ([`on_undo` and `on_redo`](../styling-events/#editing-commands-undo-redo-copy-cut-paste)): Ctrl+Z (Cmd+Z on a Mac) takes back the last change and Ctrl+Shift+Z or Ctrl+Y (Cmd+Shift+Z) does again what was taken back, in a Ukrainian or any other layout, and so do an Edit menu's Undo and Redo (`ctx.edit(EditCommand::Undo)`) with the pad focused. With nothing to take back, or to do again, the command goes on to what holds the pad: a form whose app keeps a history of its own. A new stroke forgets what was taken back, and a signature the app sets forgets what the pad did. A press or a stroke on the pad focuses it, so its keys go on from there; a press of Clear, or of Undo that takes back the last stroke, gives the focus to the pad, as the buttons are gone after it. A press of a button draws nothing under it.
- **A picture.** `Signature::to_image(scale_factor, ink, line_width)` draws it on lurq's CPU canvas ([`CanvasHandle::offscreen`](../canvas/#a-picture-drawn-off-the-screen)) at its own size times `scale_factor`, in `ink` and `line_width` wide, on nothing: an `ImageData` of straight RGBA, the same wherever it is made and with no window. `Signature::to_png(scale_factor, ink, line_width)` is the same picture as the bytes of a PNG file (8-bit RGBA, not premultiplied), to store or to send; the `canvas` feature brings the `png` crate for it, which is Rust alone and builds for the browser. `Signature::paint(&draw, width, height, ink, line_width)` draws it on any canvas, scaled to fit.
- In the design the pad is 280 by 124 on `Background`, its corners `Lg`, an edge of 1 in `Input` inside it. At its bottom, 12 and 16 in, a line of 1 in `Input` and, 6 under it, the words in the mono type of 11 in `MutedForeground`. The board draws it signed and no other state: the ring of the focus is around the pad, Undo and Clear are ghost buttons of the `Xs` size 6 in from its top corner, an invalid pad has its edge in `Destructive`, a disabled one is dimmed, and the pointer is a cross over a pad that is signed on. It is drawn on the canvas, again only when what it shows changes.
- To a screen reader it is a picture (`img`) called by its label and described as signed or not.
- Parts: `SignaturePadPart::Root` (the pad, what is focused), `Surface` (the canvas over the whole pad), `Ink` (the line: its background is the ink, its width the line's), `Guide` (the column at the bottom: its padding and gap), `Baseline`, `Placeholder`, `Actions` (the row of the buttons). States: `SignaturePadState::Empty` or `Signed` (one of them is on), `Drawing`, `Interactive`, `ReadOnly`, `Invalid`, `Disabled`.
- In a [field](#field) it is called by the field's label unless it has one of its own, invalid where the field is and disabled where the field is.
- A renderer that draws another pad is given a `SignaturePadView`: `view.root()` is the pad with its keys and its role, `view.surface()` the canvas that draws and takes the strokes, `view.guide()` the line and its words, `view.actions()` the buttons where they are shown, `view.signature` the value and `view.draft` the points of the stroke being drawn.

## Shortcut Recorder

A box that holds a key combination, which a person sets by pressing it: the shortcut of a command in an app's settings, the key that shows a window.

```rust
use lurq::{app::Accelerator, ui::ShortcutRecorder};

// palette: Signal<Option<Accelerator>>. Mounted, it records.
ShortcutRecorder::optional(palette.clone())
  .label("Command palette")
  // The app refuses a combination with a sentence the box says under it.
  .validate(|pressed| match pressed.to_string().as_str() {
    "Ctrl+Q" => Err("Already used by Quit".to_owned()),
    _ => Ok(()),
  })
  .mount(ctx);

// The app holds it: told what is pressed, it shows what the app says next.
ShortcutRecorder::controlled(saved, move |shortcut| store(shortcut.to_string()));
```

| Builder | |
| --- | --- |
| `ShortcutRecorder::new(Signal<Accelerator>)` | The shortcut the signal holds; a combination pressed and taken sets it |
| `ShortcutRecorder::optional(Signal<Option<Accelerator>>)` | The same where none is a state of its own, which Backspace and the cross at its end give |
| `controlled(Accelerator, on_change)`, `controlled_optional(Option<Accelerator>, on_change)` | The app holds the shortcut: it is told what a combination pressed (or an emptying) asks for, and what is shown is what it says next |
| `on_change(handler)` | Called with `&Accelerator` (`&Option<Accelerator>` for an optional one) after the signal is set. Not for the shortcut it holds already, nor when the app sets the signal. Each call adds |
| `validate(Fn(&Accelerator) -> Result<(), String>)` | The app refuses a combination with a sentence: the box shows what was pressed, edged as wrong, says why under it, and keeps what it held. Each call adds a check |
| `require_modifier(bool)` | A combination needs a modifier held with its key: it does unless told, but a function key is taken alone |
| `platform(KeyPlatform)` | Whose names its keys are drawn with ([`Kbd`](#kbd)): the platform the app runs on unless told |
| `label(&str)` | What it is called to tooling: "Shortcut" unless told. It draws nothing |
| `placeholder(&str)`, `recording_text(&str)`, `clear_label(&str)`, `modifier_text(&str)` | What it says with no shortcut ("Record shortcut") and while it records ("Press shortcut…"), what its cross is called ("Clear shortcut"), and what it says of a combination with no modifier |
| `size(ShortcutRecorderSize)` | A box's sizes: `Sm` 32 high, the board's and the default, `Md` 36, `Lg` 40 |
| `invalid(bool)`, `warning(bool)`, `valid(bool)`, `read_only(bool)`, `disabled(bool)`, `full_width(bool)`, `name(..)` | As an [input](#input) has them. Read only, it does not record and has no cross. Its `name` gives the form its shortcut written as `Ctrl+Shift+P` |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the box's |

- A press of the box, or Enter or Space while it has the focus, starts recording. While it records it takes every key of the window ([`captures_keys`](../styling-events/#taking-every-key)): the app's own shortcuts and the accelerators of its menu are held back, and Tab is recorded with what is held. It shows the modifiers as they are held, and takes the combination when a key that is no modifier goes down ([`Accelerator::from_key_event`](../window-lifecycle-menus/#reactive-updates-and-shortcuts)). Escape gives up, the focus leaving it gives up, and Backspace and Delete with no modifier empty an optional one.
- It records where it is mounted (`mount(ctx)`), which keeps whether it records, the modifiers held and the combination it refused. Converted into an element it shows its shortcut and its cross empties it, but a press does not record. In a [field](#field) the app mounts (`Field::new("Shortcut", recorder).mount(ctx)`) the field mounts it: it records, is called by the field's label, judged as the field is, and disabled where it is, and a combination it refuses says why under it, over the field's description.
- In the design the box is 230 by 32, a box's fill and edge with the medium radius, 8 in at its start and 6 at its end; its keys are [`Kbd`](#kbd)s 4 apart, and the cross at its end is 14 in `MutedForeground`. Recording, its edge is a line of 2 in `Ring`, and a dot of 8 in `Destructive` stands before "Press shortcut…" in `MutedForeground` at 13. A refused combination edges it in `Destructive` and says why 6 under it at 12 in `Destructive`. Under the pointer, judged, read only and disabled it is a box.
- Parts: `ShortcutRecorderPart::Root`, `Control` (the box), `Keys`, `Dot`, `Hint`, `Clear`, `Message`. States: `ShortcutRecorderState::Recording`, `Empty`, `Refused`, `FullWidth`, and a box's `Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`.
- To what reads the tree the box is a `button` called by its label, with `a11y_value` its shortcut in the platform's words ("Ctrl+Shift+P", "Cmd+Shift+P"), `a11y_pressed` while it records, and a refusal as `a11y_invalid` and its `a11y_description`.
- A renderer is given a `ShortcutRecorderView`: `view.control()` is the box with its behavior, `view.content()` what it shows, `view.clear()` the cross, `view.message()` why a combination was refused, `view.kbd(shortcut)` the keys of a shortcut.

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

// Groups with a label over each and a line between them, and a hint at an option's end.
Select::new(platform.clone(), [
  SelectOption::new("windows".to_owned(), "Windows").group("Desktop"),
  SelectOption::new("linux".to_owned(), "Linux"),
  SelectOption::new("ios".to_owned(), "iOS").group("Mobile").hint("beta"),
]);

// Held by the app: it is told of a choice, and the trigger shows what it answers with.
Select::controlled(rows, [(10, "10"), (20, "20"), (50, "50")], move |rows| set_rows(*rows));
```

| Builder | |
| --- | --- |
| `new(Signal<T>, options)` | What it is bound to, and its options: `(value, label)` pairs or `SelectOption`s (`detail`, a second line; `hint`, a note at its end; `leading` and `trailing`, an element before and after the label, which the closed select shows for the chosen option, and `leading_with` and `trailing_with` for one built for the state of its row, as an [icon](#icon) in the color of the label is; `group(label)`, the group it starts; `disabled`). The option whose value the signal has is chosen; choosing another sets the signal |
| `optional(Signal<Option<T>>, options)` | Bound to a value that may be none: `None` is no option's, and the trigger shows the placeholder |
| `controlled(value, options, on_change)` | The app holds the value: a choice is told to `on_change`, and the trigger shows what the app answers with. Two built at one place in the app's code, in a loop, are told apart by their `id` or `key` |
| `on_change(Fn(&T))` | Called with the new value when a person chooses: not when the app sets the signal, and not for the option chosen already. The handler takes the type of the value, `&Option<T>` of an optional one: a `Select<T>` is generic over it, so a handler of another type does not compile. A second handler is called after the first |
| `placeholder(&str)` | What the trigger says while no option is chosen |
| `size(SelectSize)` | `Sm` 32, `Md` 36 (the default), `Lg` 40 high, with text of 13, 14, 15: an input's sizes. `SelectSize` is `TextBoxSize` |
| `read_only(bool)` | What is chosen is read and not changed: the list does not open. It keeps the focus; the trigger is on `Muted` in an edge of `Border`, as a read only input is |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | What the app judged of what is chosen: the edge and the ring in the destructive, warning or success color. One is shown, in that order, as an input shows them |
| `full_width(bool)` | As wide as what holds it, in place of 260 |
| `fit_options(bool)` | Whether its list is as wide as its longest option, with the room of the check, in place of as wide as the trigger: for a trigger as wide as the one value it shows, as a bar's control is. The panel's `min_width` still holds |
| `label(&str)` | What it is called, to what reads the tree. The field it is in shows its label |
| `name(..)` | The name of its value in a form, the chosen option's label (feature `form`) |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the select's node. `state(SelectState::Open, true)` draws the trigger as it is while open, for a gallery |

- It is the engine's [`Select`](../styling-events/#select) dressed by the recipe, not a second one: a press, or Enter, Space and the arrows from the keyboard, open the list under it (over it where there is no room); the arrows, Home, End and the first letters move through it, skipping disabled options and the groups' labels; Enter or a press chooses, and Escape closes. Tab closes it too and goes on to the next stop, as from a closed select. Tab reaches it wherever it is (`tab_index(0)`); a disabled one does not open and Tab passes it by.
- It stands beside an [input](#input): the same box, sizes, edge, ring and judgments. The ring is a line of 2 inside the box in `Ring`, shown from the keyboard and while the list is open.
- Parts: `SelectPart::Trigger` (the select's node and box: its size, edge, fill, padding, ring, opacity, cursor, the type of what it says, and in its gap the room before the chevron), `Value` and `Placeholder` (laid over the trigger's type and color), `Chevron` (its side and color), `Panel` (the panel of the list: fill, edge, corners, shadow, padding; its gap is the room under the trigger, its height the most it is before it scrolls, and its least width how wide it is where its trigger is narrower), `Item` (an option: height, padding, corners, type, color; `hovered` is the item under the pointer or the keys; its gap is the room before the check), `Check` (the chosen item's mark), `Detail`, `Hint`, `Group` (a group's label), `Separator` (the line between groups: its fill, its height, and in its padding the room around it). States: `SelectState::Open`, `Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`; an item's own `Selected` and `Disabled` are resolved on its parts (`Item`, `Check`). Sizes: `SelectSize::Sm`, `Md`, `Lg`.
- In the design the trigger is 260 by 36, `Background` in an edge of 1 in the `Input` color, `radius-md`, the text 12 from the edge in the body's type, the placeholder and a chevron of 16 in `MutedForeground`, 8 apart; `Ring` under the pointer; `Destructive`, `Warning` or `Success` for a judgment, under the pointer, in the focus and open too; disabled is half opaque on `Muted`. The list is 4 under it and as wide, on `Popover` in an edge of `Border`, `radius-lg`, with the theme's middle shadow and 4 of padding. An item is 32 high, its text 8 in, `radius-sm`, in the body's type, on `Highlight` under the pointer or the keys; the chosen one has a check of 16 in `Foreground` at its end, 8 after the label; a disabled one is at 0.45. A hint is 11 in the mono type in `MutedForeground`. A group's label is 11, semibold, in `MutedForeground`, in line with the items' text; a line of 1 in `Border` is between groups, 4 from each.
- Its options follow the [density](../theme/#density): 24, 32 (the design's), 40 and 44 high at compact, default, comfortable and touch, the steps of a row of 36 (28, 36, 44, 48). The trigger is a control and keeps its size.
- The engine paints the trigger, the panel and the rows itself, from its `SelectStyle`, not from nodes the recipe styles: of the trigger it takes the fill, the edge, the corners, the shadow, the ring and the padding, at rest, `hovered`, `focus_visible` and in the state `Open`; of an item, the fill, edge, corners, padding, height (at least), type, color and opacity, at rest, `hovered` and in the states `Selected` and `Disabled`; of a group's label, its padding, fill, type and color; of the separator, its fill, height and padding. A transition is not painted.
- To what reads the tree (`lurq_snapshot` of the MCP server) it is a `combobox`, called by its label where it has one.
- A renderer that draws another select starts from `view.select_style()`, the engine's `SelectStyle` the recipe makes, changes what it likes (another chevron, a menu as wide as its widest option), and builds it with `view.select(style)`, the engine's select bound to the value with the trigger's size, opacity and cursor.

## Popover

A panel beside the button that opens it, over the window, holding anything: the size of a layer, a few settings, a form. A press outside it or Escape closes it.

```rust
use lurq::{
  components::Placement,
  ui::{Button, ButtonVariant, Field, Input, Popover},
};

// open: Signal<bool>, anchor: ElementRef, width: Signal<String>
Popover::new(open.clone(), anchor.clone())
  .trigger(Button::new("Dimensions").variant(ButtonVariant::Outline))
  .title("Dimensions")
  .description("Set the size of the layer.")
  .placement(Placement::BottomStart)
  .child(Field::new("Width", Input::new(width).full_width(true)).full_width(true));
```

| Builder | |
| --- | --- |
| `new(open, anchor)` | The app's `Signal<bool>` of whether it is shown, and the element it is shown beside |
| `trigger(..)` | The button that opens and closes it: put before it, given the anchor, and pressed to turn the signal |
| `title(&str)`, `description(&str)` | The text: what it is about, and under it what it is for. Without either there is no text |
| `child(..)`, `children(..)` | What it holds, one under another, as wide as the panel |
| `placement(Placement)` | The side of the anchor it is on and the edge it lines up with. `BottomStart` by default |
| `width(..)` | How wide the panel is, over the recipe's 292 |
| `offset(impl Into<SpacingValue>)` | How far it is from its anchor, along the side it is on: `SpacingSize::Xs` (4) by default |
| `collision(CollisionStrategy)` | What it does where there is no room on its side: `FlipThenClamp` by default |
| `within(element)` | Kept inside the box of that element (an `ElementRef`), a column it belongs in, as well as inside the window: where its collision flips and clamps it, it does so in that box too (the engine's `Overlay::within`) |
| `placed_by(element)` | Placed by the box of that element in place of its anchor's: at its placement of that box, flipped and clamped there. The anchor stays its trigger and where the focus goes back: a popover over a status bar and at its end that an item of the bar opens (the engine's `Overlay::placed_by`) |
| `at(Option<(f32, f32)>)` | A point of the window it stands at in place of its anchor, its corner there, `offset` away, opening down and toward the end and the other way where there is no room (the engine's `Overlay::at`). The placement is not used |
| `label(&str)`, `described(&str)`, `alert(bool)` | What tooling calls it (its title by default), what it is described by, and whether it is an `alertdialog` |
| `on_dismiss(Fn())` | Runs after a person closed it by a press outside or Escape, not when the app sets the signal. Each handler given runs, in order |
| `dismiss_on_escape(bool)`, `dismiss_on_outside_press(bool)` | Whether Escape, and a press outside it and its anchor, close it: both `true` by default |
| `keeps_focus(bool)` | It leaves the focus where it is as it opens, in the app's field or panel, in place of taking it: a list of choices under a field being typed in, a [calendar](#calendar) whose keys its panel reads (`Calendar::keys(CalendarKeys::Panel)`). A press outside it then goes on to what it was on, as well as closing it. `false` by default |
| `inline(bool)` | Draws the panel where it is declared, open or not, without its trigger and not over the window: for a gallery, a screenshot, a test |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the panel |

- A title or a description longer than the panel wraps inside it.
- It is the engine's `Popup` with the panel dressed by the recipe: it is placed against its anchor in every frame, goes to the other side where there is no room and is kept in the window (and in the element `within` names, as that element was last laid out: it flips where that box has no room on its side, is moved into it, then into the window, which wins where they differ; one wider than the box keeps its start at the box's start), and closes on Escape and on a press outside it, which sets the signal to `false`. Declare it always and drive it with the signal: lurq only knows the anchor of a popup it can see.
- With a trigger, the trigger's own node is the anchor: a press of it, or Enter and Space on it, opens and closes the popover, and while another popup or select is open one press of it closes that one and opens this one. It tells what reads the tree `a11y_haspopup` and `a11y_expanded`. Without a trigger the app opens it, and the anchor is any element of its own (`ref_element`); a press of the anchor is not a press outside it.
- As it opens the focus goes to the first thing in it that takes the focus (`Popup::takes_focus`), and as it closes with the focus inside, back to what had it when it opened: the trigger. Its stops of Tab come right after the trigger's: Tab from its last one goes on to what follows the trigger. It does not keep Tab in it: it is not modal.
- The anchor is kept from one render to the next (a field of the component, `ctx.element_ref()`): one made anew in each render is not the one the focus goes back to.
- It fades in over the theme's `Base` duration in the `Enter` curve, and out over `Fast` in the `Exit` curve (`PartStyle::enter`, `PartStyle::exit` on the root, `FADE_IN` and `FADE_OUT`). A fade is not movement, so reduced motion leaves it.
- A theme given to a subtree reaches a popover declared in it.
- Parts: `PopoverPart::Panel` (the panel), `Text` (the title over the description), `Title`, `Description`, `Body` (what it holds). It has no states of its own.
- In the design the panel is 292 wide in `Popover` and `PopoverForeground`, with an edge of 1 in `Border`, the `Lg` radius and the `Md` shadow, 16 of padding and 14 between the text and the body. The title is in the body's type at medium weight, the description in 13 and `MutedForeground`, 4 under it; what the body holds is 14 apart.
- To what reads the tree the panel is a `dialog` called by its label or its title.
- A renderer builds the panel alone: where it is shown, when, its trigger and its focus stay the popover's. It starts from `view.panel()`, the panel with its style and nothing in it, and has `view.text()` and `view.children`.

## Menu

A list of commands in a panel over the window, opened from an anchor: the actions of a file, the account's menu, how a split button does what it does.

```rust
use lurq::{
  components::Row,
  ui::{Button, ButtonVariant, Menu, MenuItem, icons},
};

// anchor: ElementRef, open: Signal<bool>. The button the menu hangs from opens and closes it.
let toggle = open.clone();
Row::new()
  .ref_element(anchor.clone())
  .child(Button::new("View").variant(ButtonVariant::Ghost).on_click(move |_| toggle.update(|open| *open = !*open)));

Menu::new(open, anchor)
  .item(MenuItem::new("Undo").icon(icons::UNDO).shortcut("Ctrl+Z").on_select(move || undo()))
  .item(MenuItem::new("Redo").shortcut("Ctrl+Y").disabled(true))
  .separator()
  .item(MenuItem::new("Show grid").checked(grid.get()).on_select(move || toggle_grid()))
  .separator()
  .heading("Zoom")
  .item(MenuItem::new("Fit to window").radio(zoom.get() == Zoom::Fit).on_select(move || fit()))
  .item(MenuItem::new("100%").radio(zoom.get() == Zoom::Actual).on_select(move || actual()))
  .separator()
  // export: ElementRef, exporting: Signal<bool>: the submenu hangs from its item.
  .item(MenuItem::new("Export as").submenu(Menu::new(exporting, export).items(["PNG", "SVG", "PDF"].map(MenuItem::new))))
  .item(MenuItem::new("Delete").icon(icons::TRASH).destructive(true).on_select(move || delete()));
```

| `Menu` | |
| --- | --- |
| `new(open, anchor)` | Whether it is open, the app's `Signal<bool>`, and what it hangs from (an `ElementRef`). Choosing an item, a press outside, Escape and Tab set the signal to `false` |
| `controlled(open, anchor, on_open_change)` | A menu the app holds open itself, with a `bool`: a person closing it is told `false`, and what the app's next render says is drawn. `true` and a handler that does nothing hold it open, for a gallery or a test |
| `item(MenuItem)`, `items(..)` | Commands, after what the menu holds already |
| `separator()` | A line that ends one group of items and starts the next |
| `heading(&str)` | A heading over the items after it: what they are about |
| `label(&str)` | The menu's name to what reads the tree |
| `placement(Placement)` | Where the panel opens beside its anchor, 4 from it. `BottomStart` by default, `RightStart` for a submenu |
| `offset(impl Into<SpacingValue>)`, `collision(CollisionStrategy)` | How far the panel is from its anchor, along the side it opens on, in place of the design's 4 (a submenu's first item stays level with its item); and what it does where the window has no room on that side (`FlipThenClamp`, the other side, by default) |
| `on_dismiss(Fn())` | A person closed it other than by a choice: Escape, a press outside it, Tab. Not the app setting the signal. Each handler is called, in order. Tab in a submenu closes it and the menus it hangs from, and tells each one's `on_dismiss`, the submenu's first |
| `on_open_change(Fn(bool))` | Told `false` when a person closes it, by a choice too. Each handler is called, in order |
| `dismiss_on_escape(bool)`, `dismiss_on_outside_press(bool)` | Whether Escape and a press outside close it: both by default |
| `within(element)` | Kept inside the box of that element besides the window, as a [popover](#popover) is; a submenu given nothing of its own is kept where its menu is |
| `placed_by(element)`, `at(Option<(f32, f32)>)` | Placed by the box of another element than its anchor, or at a point of the window, as a [popover](#popover) is; the anchor still opens it |
| `child(element)`, `children(..)` | Elements of the app's among its entries, drawn as given: a foot that says why the items above cannot be chosen, a section of its own, a field. One is no item and does not close the menu; one that takes the focus is reached by the arrows as an item is |
| `inline(bool)` | The panel drawn where the menu is in the tree, open or not, and not over the window: for a gallery, a test, a list of commands in a page |
| `keeps_focus(&MenuKeys)` | It leaves the focus in the app's field as it opens, where a person goes on typing: see [a menu that keeps the focus in a field](#a-menu-that-keeps-the-focus-in-a-field) |
| `page(Signal<Option<usize>>)`, `back_label(&str)` | Which item's [page](#menu) the menu shows in its place, by the item's place among what the menu holds (items, separators and headings counted), `None` for its own items; and what the row back is called before the page's title ("Back"). A menu whose items have pages is given the signal; a person closing the menu sets it back to `None` |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the panel |

| `MenuItem` | |
| --- | --- |
| `icon(name)` | An icon before the label, by its name in the theme's set |
| `leading(element)` | Any element before the label, in place of the icon: an avatar, a mark of a service |
| `content(element)` | All the item shows, the app's own row, in place of its icon, label, detail and end. It is still the menu's item: pressed, reached by the arrows and by typing its label, called by its label, checked, disabled or unavailable, and choosing it closes the menu. Its root is as high as the element and never lower than a row (state `Content`) |
| `detail(&str)` | A second line under the label that says more of the item: an agent's model under its name. The item is as high as its two lines |
| `unavailable(&str)` | It cannot be chosen now, for this reason: disabled, in the state `Unavailable`, with a lock at its end and the reason under its label where it has no detail of its own. What reads the tree is told the reason (`a11y_description`). An empty reason makes it available again |
| `shortcut(&str)` | The keys that do the same, written at the end: "Ctrl+Z". The item only shows them; the app binds the keys |
| `checked(bool)` | Whether what it turns on is on: a check at its end |
| `radio(bool)` | One of several of which one is selected: a dot at the end of the selected one |
| `submenu(Menu)` | A menu the item opens beside it, with a chevron at its end. The submenu's anchor is given to the item |
| `page(items)` | Items the item shows in its menu's place, under a row that goes back with the item's label for a title: a player's "Quality" and its qualities. The item has a chevron at its end and is not chosen itself; choosing an item of the page closes the menu and calls that item's handlers. An item has a page or a submenu, not both |
| `destructive(bool)` | It removes or cannot be undone: drawn in the destructive color, icon and all |
| `on_select(handler)` | Called when it is chosen, after its menu (and the menus it is in) closed, so that a handler that opens a dialog keeps the focus the dialog takes. Each handler is called, in order |
| `on_open_aside(handler)` | Called in place of `on_select` when a person asks for what it stands for beside what is open: the middle button, a press with Ctrl (Cmd on macOS), Ctrl+Enter (Cmd+Enter) on the focused item or on the active item of a menu that keeps the focus in a field. The handler alone runs, given the press (its `shift` asks for a tab that is switched to), and the menu closes, with the menus it is in, as after a choice. An item that is disabled or opens a submenu or a page is asked for nothing. See [pointer-only pressables](#pointer-only-pressables) |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the item |

- The menu does not keep whether it is open: the app does, in the signal it gives it, and the anchor's own handler opens it. Choosing an item sets the signal to `false` and then calls the item's handler; a press outside the panel, Escape and Tab set it too (the engine's `Popup`). Declare the menu always, open or not, beside its anchor: lurq then knows the anchor is its trigger.
- The keyboard: a menu opened from the keyboard (Enter or Space on its button) has its first item focused and lit; opened by the pointer it has the focus there too, unlit. Up and Down move between its items and go round at the ends, Home and End go to the first and the last, and a disabled item is passed by. Typing a letter goes to the item whose label starts with it, letters typed quickly to the one a word starts, the same letter again to the next. Enter or Space chooses the focused item; the arrows only move the focus. Escape, or choosing, closes it and gives the focus back to its button. Tab closes it, and every menu it hangs from, and goes on from its button to the next stop, as Tab in an open [select](#select) does (the panel is a [focus group](../focus-navigation/#focus-groups) of one stop).
- The pointer and the keyboard are on one item: the pointer coming onto an item moves the focus there, and an arrow key moves the light off the item under a pointer that holds still.
- Under the pointer or focused from the keyboard an item is highlighted: filled with `Highlight`, the board's `accent`. An item whose submenu is open stays highlighted.
- A submenu opens beside its item, its first item level with it, under the pointer, on a press of the item, or on Right, Enter or Space while the item is focused, with the focus on its first item; the pointer on another item of the menu closes it, and so do Left and Escape while the focus is in it, giving the focus back to its item. Choosing in it closes it and every menu it hangs from, and gives the focus back to what opened the outermost, as choosing in the menu itself does. A press in the menu while a submenu is open closes the submenu and still reaches what it pressed.
- Parts of the menu: `MenuPart::Panel`, `Separator` (the room around the line), `SeparatorLine`, `Label`; state `MenuState::Submenu`. Parts of an item: `MenuItemPart::Root` (what is pressed), `Icon`, `Leading` (around the app's element), `Label`, `Detail`, `Lock`, `Shortcut`, `Check` (the check or the dot), `Chevron`. States of an item: `MenuItemState::Checked` (a checked item), `Selected` (the selected radio), `Radio`, `Destructive`, `Disabled`, `Open` (its submenu is open), `Detail` (it has a line under its label), `Unavailable` (it cannot be chosen, for a reason; `Disabled` too), `Content` (it shows the app's element), `Active` (the item the keys are on in a menu that keeps the focus in a field, filled with `Highlight`).
- An item with a detail grows to its two lines, at least 32 high with 6 above and below them; its icon or element stands at the top, by the label. The detail is 2 under the label, in 12 of `MutedForeground` in lines of 1.4, and wraps where the menu has no room for it. An unavailable item is dimmed as a disabled one is, and ends in a lock of 16 in `MutedForeground`.
- A menu is as wide as its widest item, and at least as wide as the design says; never as wide as the window. In the design the panel is at least 230 wide (a submenu 160) and, where the design has no longer label, at most 360: a label longer than that gives way with an ellipsis, and the shortcut, the check and the chevron keep their room and the 8 before them. The panel is in `Popover` with `PopoverForeground`, padded 4, with the large radius, an edge of 1 in `Border` and the medium shadow. An item is 32 tall, padded 8, with the small radius and 8 between what it holds: an icon of 16 in `MutedForeground`, the label in the body's type, and at the end the shortcut in the mono type at 11 in `MutedForeground`, the check or the dot of 16, and the chevron of 16 in `MutedForeground`. A separator is a line of 1 in `Border` with 4 above and below; a group's label is 11/600 in `MutedForeground`, 8 in from the side. A destructive item is in `Destructive`, its icon too; a disabled one is dimmed.
- Its items follow the [density](../theme/#density): 24, 32 (the design's), 40 and 44 high at compact, default, comfortable and touch. A context menu's, a menubar's and a command palette's are menu items, and follow it with them.
- To what reads the tree the panel is a `menu`; an item a `menuitem`, a `menuitemcheckbox` or a `menuitemradio` (these two with `a11y_checked`) called by its label; an item with a submenu has `a11y_haspopup` `menu` and `a11y_expanded`; a separator is a `separator`.
- What a menu holds, in order, is a `MenuEntry`: `Item(MenuItem)`, `Separator`, `Heading(text)`, or `Element(element)`, an element of the app's, as `MenuView::entries` has them. A `MenuItem` converts into one with `.into()`, for a list that mixes items with separators and labels, as a [menubar](#menubar)'s `menu(title, entries)` takes.
- A renderer for `Menu` gets a `MenuView`: `view.panel()` is the panel with its style and its focus group, `view.item(item)`, `view.separator()` and `view.heading(text)` build what it holds, and `view.render()` hangs it from the anchor. A renderer for `MenuItem` starts from `view.root()`, the behavior with the root's style, and puts in it `view.icon()` or `view.leading()`, `view.label()`, `view.detail()`, `view.shortcut()`, `view.lock()`, `view.check()`, `view.chevron()` or what it likes; `view.submenu()` is the submenu to put after it. Choosing it still closes its menu.
- A menu of the app's own rows keeps lurq's menu: a `MenuItem` given `content(row)` is the row the app designs, with its label cut to its room, a detail on its line, a hint at its end, and the item is the root around it that the keyboard, the pointer and tooling know. The app's recipe of `menu_item` gives that root its height, its padding and its fill under the pointer; what the row draws for a state (dimmed, the destructive color, a lock) is the app's, and the item still says the state to tooling. Rows that are not commands, a foot, a field, are the menu's `child`ren:

```rust
use lurq::ui::{Menu, MenuItem};

// open: Signal<bool>, anchor: ElementRef; row(..) and foot(..) build the app's elements.
Menu::new(open, anchor)
  .item(MenuItem::new("Opus").content(row("Opus", "the deepest")).checked(true))
  .item(MenuItem::new("Sonnet").content(row("Sonnet", "balanced")).unavailable("Not on this plan"))
  .separator()
  .child(foot("Sonnet is not on this plan."));
```
<a id="a-menu-that-keeps-the-focus-in-a-field"></a>**A menu that keeps the focus in a field.** The commands after a `/`, the people after an `@`, a combobox's list:
the menu opens under a field a person goes on typing in, and the focus stays there (a combobox's
`aria-activedescendant`). `Menu::keeps_focus(&keys)` takes a `MenuKeys`, made once by the app with the field's
`ElementRef`, as a `DateMemory` is. The panel takes no focus, and a press of an item leaves the focus in the field;
one item is the active one (state `MenuItemState::Active`, `a11y_selected` to tooling), the first that can be chosen as
the menu opens and whenever what it holds changes, and the one the pointer comes onto. While the field has the focus
the panel reads its keys: Up and Down move the active item round the items that can be chosen, Enter chooses it,
Ctrl+Enter (Cmd+Enter on macOS) asks for it aside (`MenuItem::on_open_aside`), Escape closes the menu, and every other
key, letters and Home and End among them, is the field's. Enter with no item that can be chosen is the field's too. A
field whose own keys come first (Enter sends the message) forwards each key with `MenuKeys::key_down(&event)`, which
says whether the menu took it; the menu takes a key once. `MenuKeys::active()` is the active item's place among the
items. A press outside the panel, in the field too, closes the menu and goes on to what it was on; a press of the anchor
opens nothing. The items of a page and of a submenu are not reached by these keys.

```rust
use lurq::{
  components::TextInput,
  ui::{Menu, MenuItem, MenuKeys},
};

// field: ElementRef and keys: MenuKeys::new(field.clone()), both made once with the component; open: Signal<bool>.
let menu = Menu::new(open.clone(), field.clone())
  .keeps_focus(&keys)
  .items(commands.iter().map(|command| MenuItem::new(&command.label).on_select(command.run.clone())));
let forward = keys.clone();
let input = TextInput::new(draft.clone()).ref_element(field.clone()).on_key_down(move |event| {
  if forward.key_down(&event) {
    return;
  }
  // The field's own Enter: send.
});
```
- A page is the menu's other list in the same panel, where a submenu is a panel beside it: Right on an item with a page opens it, Left in the page and the row at its top go back, and the focus stays on the row the keyboard was on (the row back and the item that opened the page are one element to the engine). Parts `MenuPart::PageTitle` (13/600) and `PageIcon` (the chevron, in `Foreground`) style the row back, as the board's `Media/Settings Menu/Page` has it.

## Dialog

A panel over the window that asks one thing and waits for the answer: delete a workspace, accept the terms, name a new file.

```rust
use lurq::ui::{Button, ButtonVariant, Dialog, DialogSize};

// open: Signal<bool>, the app's.
let cancel = open.clone();
Dialog::new(open.clone())
  .title("Delete workspace?")
  .description("This removes every window layout and setting stored for this workspace. It cannot be undone.")
  .footer(Button::new("Cancel").variant(ButtonVariant::Outline).on_click(move |_| cancel.set(false)))
  .footer(Button::new("Delete").variant(ButtonVariant::Destructive));

// A body that scrolls between a header and a footer that stay, with a close button.
Dialog::new(terms_open.clone())
  .scrollable(true)
  .close_button(true)
  .title("Terms of use")
  .children(paragraphs)
  .footer(Button::new("Decline").variant(ButtonVariant::Outline))
  .footer(Button::new("Accept"));
```

| Builder | |
| --- | --- |
| `new(Signal<bool>)` | Shown while the signal holds `true`. The dialog does not keep it: Escape, a press on the veil and the close button set it to `false` |
| `title(&str)`, `description(&str)` | What it asks, and more of it under the title. The title is what the dialog is called |
| `child(element)`, `children(..)` | The body: a form, a list, paragraphs |
| `footer(element)` | Adds to the row at the end of the dialog, the least action first and the main one last. The app puts `Button`s there |
| `size(DialogSize)`, `width(impl Into<Length>)` | `Sm` 360, `Md` 480, `Lg` 640, `Full` the window; 420 without one, the board's, which is none of the steps. Never wider than the window |
| `scrollable(bool)` | The body scrolls between a header and a footer that stay, each set off by a line |
| `icon(name)`, `media(element)` | A mark before the title: the icon in a well of 32 (`Muted`, `radius-lg`, the icon 16 in `Foreground`, 12 before the title, which then stands level with its middle), or the app's element in its place, as it is. State `DialogState::Mark` |
| `note(&str)`, `note_icon(name)` | Quiet words at the start of the footer, the buttons at its end: why the action cannot be taken yet, what a key does. 12 in `MutedForeground`, an icon of 14 6 before them; they take the room the buttons leave, and wrap there. State `DialogState::Note` |
| `note_key(keys, words)`, `note_key_worded(cap, words)`, `note_child(element)` | A key cap and the words of what the key does in the note, after its words, in the order given: `note_key("Enter", "Delete").note_key("Esc", "cancels")`. The keys are written as `Kbd::shortcut` writes them for the platform; `note_key_worded` writes the cap as the app gives it, its own word for the key ("Enter", translated). The words are in the note's type, 6 after the cap, and a pair stands 12 after what is before it (part `DialogPart::NoteKey`, ids `<id>-note_key`, `<id>-note_key-2`; each cap is a `Kbd` named by the part `NoteKeyCap`, `<id>-note_key_cap`, `<id>-note_key_cap-2`). `note_child` puts any element of the app's there: it gives way where the note is short and is never wider than it, so a long reason wraps on to the next lines. A note of keys alone is a note |
| `inset_top(f32)` | How much of the top of the window the veil leaves to what is under it, over the title bar of a [window chrome](#window-chrome), which it leaves without being told: a bar the app draws itself. That band takes the pointer as while no dialog is open, and the panel stands in the middle of the rest; the keys are the dialog's |
| `initial_focus(DialogFocus)` | Where the focus goes as it opens: `FirstControl`, the default, to a control marked [`autofocus`](#button) (the main action) else its first stop of Tab; or `Panel`, to the panel itself, a stop of Tab before its first control that shows no ring, so that Enter confirms (`on_confirm`) and presses no button. As it closes the focus goes back to what had it, either way |
| `on_confirm(Fn())` | Called when Enter is pressed while the dialog is open: its main action, which the app runs (and closes the dialog after where it should). Not where the focus is on what Enter does a thing of its own with (a button, which Enter presses; a link; a select; a text input of several lines, where it starts a line), which the engine says in `KeyboardEvent::focus_takes_enter`, nor for an Enter an input method composes or one with Ctrl, Alt or Cmd held. An Enter in a text input of one line confirms. A dialog drawn `inline` confirms only an Enter from its own focus. A second handler is called after the first |
| `close_button(bool)`, `close_label(&str)` | A ghost button with the close icon at the end of the header, and what it is called to tooling: "Close" when not given |
| `dismiss_on_veil(bool)`, `dismiss_on_escape(bool)` | Whether a press on the veil, and Escape, close it. Both on by default |
| `on_dismiss(Fn())` | Runs after a person closed it: a press on the veil, the close button, or Escape wherever the focus is (the engine's `Modal::on_dismiss`). A button of the app's that closes it, or the app setting the signal, does not run it. A second handler runs after the first |
| `label(&str)` | What it is called to tooling, over its title: for a dialog whose title is drawn by what it holds. `described(&str)` is what it is described by |
| `alert(bool)` | An `alertdialog` to tooling, and not a `dialog`: it asks something that has to be answered |
| `inline(bool)` | The panel where it is declared, without the veil and without taking the window: for a gallery, a screenshot, a test |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the close button as under the pointer, pressed or focused; `id` is the panel's; `key` tells two dialogs of one component apart |

- A title or a description longer than the panel wraps inside its padding, beside the close button where there is one; neither is cut.
- It is the engine's `Modal` with `target(Root)` and `takes_focus(true)`: declared anywhere, it takes no room there and covers the whole window while it is open. As it opens the focus goes to its first control (the close button where it has one), or to the control marked `autofocus`, or to its panel (`initial_focus`), and as it closes back to what had it, the button that opened it. Nothing under the veil takes the pointer, and Tab and Shift+Tab go round the dialog's own controls. A dialog or a [command palette](#command-palette) opened while it is open is over it, wherever either is declared: it takes the focus, the keys and the pointer, Escape closes it first, and as it closes the focus goes back into the dialog under it.
- A press on the panel goes no further: only a press on the veil itself closes the dialog.
- The panel is never larger than the window less the veil's margin of 16. A dialog is as high as its content while that fits, and its body gives way and scrolls when it does not, in either form; `style(DialogPart::Panel, |s| s.height(300.0))` gives it a height of its own.
- Parts: `DialogPart::Veil`, `Panel`, `Header` (the mark, the text and the close button), `Mark`, `Icon` (the mark's), `Text` (the title over the description), `Title`, `Description`, `Body`, `Footer`, `Note`, `NoteIcon`, `NoteKey`, `NoteKeyCap` (rules over a cap's `Kbd`), `Close` (laid over the theme's button), `Separator` (the lines of a scrollable dialog). States: `DialogState::Scrollable`, `Mark`, `Note`. Sizes: `DialogSize::Sm`, `Md`, `Lg`, `Full`.
- In the design the panel is 420 wide in `Popover` with an edge of `Border`, corners of `radius-xl` and the `lg` shadow, padded 24 with 20 between its text, its body and its footer; the veil is `Scrim`. The title is 16/600, the description 8 under it in 14 of `MutedForeground`, and the footer's buttons stand at its end 8 apart. The scrollable one has a header of 56 (its title 20 from the edge, the close button 12 from it), higher for a description under the title, and a footer of 60, the body padded 20 and 16 between them: the body is as high as what is in it, and scrolls when the window has no room for all of the dialog. The close button is 28 with an icon of 16.
- To what reads the tree the panel is a `dialog` called by its label or its title.
- A renderer that draws another dialog builds its panel from `view.panel()`, the parts from `view.header()`, `view.mark()`, `view.text()`, `view.body()`, `view.note()`, `view.footer()`, `view.close()` and `view.separator()`, and lays it over the window with `view.modal(panel)`; `view.closer()` closes it, and `view.dismisser()` closes it and runs what `on_dismiss` was given. `view.panel()` carries what Enter confirms with; `view.confirmer()` is that key handler, for a panel built another way.

## Tooltip

A short text beside an element while the pointer rests on it or the keyboard's focus is on it: what an icon button does, and the keys that do it too.

```rust
use lurq::{
  components::Placement,
  ui::{Button, ButtonVariant, Tooltip},
};

// In a component's render: a tooltip is mounted on the element it describes, and returns that element.
let copy = Button::new("Copy").variant(ButtonVariant::Ghost).icon("copy").icon_only(true);
let copy = Tooltip::new("Copy").shortcut("Ctrl+C").mount(ctx, copy);

let zoom = Button::new("Zoom in").icon("zoom-in").icon_only(true);
let zoom = Tooltip::new("Zoom in").placement(Placement::Bottom).arrow(true).mount(ctx, zoom);

// A disabled button shows its tooltip too: it can say why it is disabled.
let export = Button::new("Export").disabled(true);
let export = Tooltip::new("Nothing to export yet").mount(ctx, export);
```

| Builder | |
| --- | --- |
| `mount(ctx, element)` | Puts the tooltip on the element it describes and returns that element |
| `shortcut(&str)` | The keys of a shortcut, after the text in a chip of their own |
| `note(&str)` | A second line under the text that says more of it; the text is then its title, in a medium weight |
| `mono(bool)` | Its text is a path, a command or code: in the theme's mono family |
| `max_width(..)` | The widest the box is before its text wraps, in place of the recipe's 200 |
| `when_cut(&ElementRef)` | It shows only while the text whose ref this is is cut: a label that says itself whole in its tooltip only where it is not whole already |
| `hint(&str)` | With `when_cut`, what it says while the text is whole, in place of nothing; while the text is cut it says its own text. The element is described by the hint |
| `over(&ElementRef)` | Keeps clear of that element, one around the element it describes, on the side it opens on: its distance from the frame's edge, and along it lined up with its element (the engine's `Overlay::clear_of`). A chip's tooltip over the field the chip sits in |
| `hang(&anchor, open)` | Not mounted on an element: the tooltip beside `anchor`, shown while `open` is (a `bool` or a `Signal<bool>`), an element to put among the children of any container. The app shows it and puts it away; the pointer, the focus, a press and Escape do nothing to it, and it takes no pointer |
| `placement(Placement)` | The side of the element it is on and the edge it lines up with: `Top`, centered, by default |
| `offset(..)`, `collision(CollisionStrategy)` | How far it stands from the element in place of the theme's 6, and what it does where there is no room on its side: `FlipThenClamp` by default |
| `arrow(bool)` | A triangle from the box to the element |
| `open_delay(..)`, `close_delay(..)` | A `MotionDuration` of the theme's or a `Duration`: how long the pointer rests before it shows (`TOOLTIP_DELAY`, 500 ms) and how long it stays after the pointer left (100 ms) |
| `force_open(Option<bool>)` | `Some(true)` shows it whatever the pointer and the focus do, `Some(false)` never: for a gallery, a test, a tour |
| `disabled(bool)` | It never shows, and the element is as it would be without it |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of what shows |

- A text too long to have its shortcut beside it wraps across the box, no wider than 200 (or its `max_width`), and the shortcut goes under it. A note wraps in the same box, under its title.
- A tooltip `when_cut(&label)` reads `ElementRef::text_cut()`, what the last layout drew of the label: cut where it was ended by an ellipsis (`TextOverflow::Elipsis`) or by a limit of lines. While the label is whole the pointer and the focus show nothing, and the label is not described by the tooltip to what reads the tree, as it says the same itself. A label cut with an ellipsis, mounted as it is, is cut in the row the tooltip holds it in, which gives way as the label would:

```rust
use lurq::{components::{Text, TextOverflow}, ui::Tooltip};

// label: ElementRef, kept in a field of the component.
let words = Text::new(path).nowrap().text_overflow(TextOverflow::Elipsis).ref_element(label.clone());
let words = Tooltip::new(path).mono(true).max_width(400.0).when_cut(&label).mount(ctx, words);
```
- It shows after the pointer has rested on the element for its open delay, and at once within `TOOLTIP_SKIP_DELAY` (300 ms) of the pointer leaving another: along a toolbar each button's shows at once, and the one before goes, so one tooltip of the pointer shows at a time.
- Focus from the keyboard shows it at once: the focus of the element or of anything in it (a row with a button in it), read by `ElementRef::focus_visible_within()`.
- The pointer can be moved onto the tooltip to read it: it goes its close delay, 100 ms, after the pointer has left both the element and the tooltip. It also goes when the focus leaves, when the element is pressed, and on Escape; after a press or Escape it stays away until the pointer comes again, or until both the pointer and the focus have left.
- Escape is the tooltip's before anything under it: the tooltip is the top layer that takes Escape, so the key closes the tooltip and not the dialog or the popup it is shown in.
- A disabled element shows its tooltip: a disabled element is not hovered, but it hears the pointer come and go, and the tooltip is on it as on any other.
- It is kept in the window: where there is no room on its side it is on the other one, its arrow turned with it, and where the window pushes it along, its arrow still points at the middle of the element. It takes no focus.
- A tooltip the app shows itself, as a toolbar shows its buttons' tooltips the moment the pointer comes to them, is hung beside its anchor and not mounted: `Tooltip::new("Copy").placement(Placement::Bottom).hang(&anchor, shown)` among the button's children. It takes no pointer, so a press that comes with the pointer reaches the button under it.
- To what reads the tree (`lurq_snapshot` of the MCP server) the element is described by the text (`a11y_description`), and called by it (`a11y_name`) when nothing else names it: a text in the element names it (a button by its label), and an icon-only button is named by its label already; an icon alone, a glyph, names nothing. What shows is a `tooltip`. An element with an element ref of its own keeps it, and the tooltip is anchored to it.
- It is a mounted component, not a builder that goes where an element goes: it keeps timers for its delay and its grace, an anchor that stays the same from one render to the next, and whether the pointer has shown it, and a builder has no `Ctx` to keep them in. Converted into an element on its own, a `Tooltip` is its box as it shows, anchored to nothing: for a gallery, a test or a host of an app's own.
- Parts: `TooltipPart::Root` (the box and its arrow, and the room between them and the element), `Panel` (the box), `Label`, `Note`, `Shortcut`, `Arrow`. The side it is on is the recipe's variant (`TooltipSide`: the side of its placement, or the other where there was no room), and the states are `TooltipState::Arrow`, `Shortcut`, `Multiline` (the text wraps, known once it is laid out), `Note` and `Mono`.
- In the design the box is in `Primary` with a radius of `Md`, its text in `PrimaryForeground` in the caption's 12, 10 from its sides and 6 from its top and bottom, and no wider than 200; a text that wraps has 8 above and below it, in lines of 1.4. A note is 2 under the text in 12 of the regular weight in lines of 1.4, the text over it in a medium weight, and the box has 6 above and below them. A mono tooltip's text is in the mono family. A shortcut is 8 after the text in a chip 16 high, 4 from its sides, with a radius of 3, in a white of 15%, its keys in the mono family in 10. The arrow is 12 along the box and 6 out from it, in the box's color. The tooltip stands 6 from the element: the root's padding on the side's axis.
- The arrow is a square in the box's color turned by 45°, its middle on the edge of the `Arrow` part that meets the box; the part clips it to the half that points away (`TooltipView::arrow()`), moved along the box by `TooltipView::aim`.
- A renderer builds what shows, not when: it is given a `TooltipView` with the text, the note, the shortcut, the side the tooltip is on, whether it has an arrow and its aim; `view.panel()` is the default box and `view.arrow()` the default arrow.
- It has no `label`: its text is its name and what describes the element. It has no `on_dismiss` and no `dismiss_on_escape`: the app does not open it, and Escape always puts it away, as content that shows on hover must be dismissible.

## Toast

A short message that something happened, in a corner of the window over everything, that leaves by itself: settings saved, a file moved to the trash, a connection that failed.

```rust
use lurq::ui::{Button, ButtonSize, ButtonVariant, Toast, ToastVariant, Toasts};

// In the root component's create, once, kept in a field of it.
let toasts = Toasts::new();
// In its render, mounted last, so that its layer is over every other.
column.child(self.toasts.mount(ctx));

// Anywhere, with a clone of it.
toasts.show(Toast::new("Settings saved").description("Your theme applies to every open window.").variant(ToastVariant::Success));
toasts.show(
  Toast::new("File moved to trash")
    .description("theme-dark.json")
    .action(Button::new("Undo").variant(ButtonVariant::Outline).size(ButtonSize::Sm).on_click(move |_| restore()))
    .timer(true),
);
```

| Builder | |
| --- | --- |
| `Toast::new(title)`, `description(&str)` | What happened, and more of it under the title |
| `variant(impl Into<Name>)` | A `ToastVariant`: `Neutral` (the default, no icon), `Info`, `Success`, `Warning`, `Destructive`: the icon and its color. A variant a theme adds has no icon of its own |
| `icon(impl Into<IconChoice>)` | An icon of the theme's set in place of the variant's, or none: `icon(None)` |
| `action(Button \| Link)` | What can be done about it, before the close control: the one action, a second takes its place |
| `close_button(bool)`, `on_close(handler)` | Whether it has its close control (it has unless told otherwise), and what a press of it calls; a second handler is called after the first |
| `label(&str)` | What it is called to tooling, in place of its title |
| `timer(bool)` | A rail along the bottom edge that says how long it has left |
| `duration(Duration)`, `persistent(bool)` | How long a `Toasts` shows it (`TOAST_DURATION`, 5 s), or that it stays until it is closed |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the panel; `force` draws the close control |

| `Toasts` | |
| --- | --- |
| `Toasts::new()` | No toasts yet. It needs no `Ctx`: make it once and clone it into whatever shows a toast |
| `show(toast) -> ToastId`, `dismiss(id)`, `clear()` | Shows one in front of the others; takes one away; takes all away |
| `ids()`, `len()`, `is_empty()` | The toasts shown and waiting, the oldest first |
| `mount(ctx)`, `mount_at(ctx, ToastPosition)` | What draws them and keeps their time: at the bottom right, or at `TopLeft`, `Top`, `TopRight`, `BottomLeft`, `Bottom` |

- A toast is a panel like any other component and can be put anywhere. Sent to a `Toasts`, it is drawn by what the `Toasts` mounts, in a layer over the window, clear of its safe area and under the title bar a `WindowChrome` draws: the layer takes a press only on the toasts, and Tab reaches their controls from the page as it reaches any other.
- It leaves after its duration, on a press of its close control or of its action, when a finger throws it to a side or toward the window's edge, and by `dismiss`. While the pointer is on the toasts their time stands still, timers too, and runs on from there when it leaves.
- Several are stacked: the newest in front, an edge of each of two others behind it, on the side away from the window's edge. With the pointer on them they are laid out one over another, 10 apart, the newest nearest the edge: those behind move out from behind the front one, fading in, and move back and fade when the pointer has gone, so nothing jumps and no shadow is left to vanish. At most `TOASTS_SHOWN` (3) are shown, and their time runs, those behind the front one too; the others wait in order.
- A toast comes in from the window's edge, fading in over the theme's slow duration, and leaves toward it over the base duration. One that comes while another is shown comes in over it: the one that was in front stays where it is, and fades under the new one when that is in. With reduced motion it comes and goes at once.
- Parts: `ToastPart::Root` (the panel), `Content` (what is in it above the timer, side by side), `Icon`, `Text` (the title over the description), `Title`, `Description`, `Action`, `Close`, `Timer` (the rail), `Remaining` (what is left of it); and the host's `Viewport` (how far from the window's edges), `List` (the toasts one over another), `BehindNear`, `BehindFar` (the edges behind the front one). State: `ToastState::Action`. Variants: `ToastVariant`.
- In the design the panel is 360 wide, in `Popover` with an edge of `Border`, large corners and the large shadow; 16 inside it are the icon of 16 in the variant's color (`Success`, `Info`, `Warning`, `Destructive`), 12 from the title (14, medium) over the description (13, `MutedForeground`), each going on to the next line where the panel has no room for it, and the close control of 16 in `MutedForeground`. With an action it is 14 high inside, centered on the action, and the title is 2 over the description. The timer is a rail of 2 in `Secondary` with what is left in `Primary`. Toasts are 16 from the window's edges; the edges behind are 24 and 48 narrower than the panel, three quarters and half opaque.
- To what reads the tree the panel is a `status` called by its label or its title, and its close control a `button` called "Close". Tab reaches the action and the close control; Space and Enter press them.
- A renderer that draws another toast starts from the view's parts: `view.icon()`, `view.text()`, `view.action()`, `view.close()` (the close control with its behavior) and `view.timer()`.

## Tabs

A list of tabs of which one is selected, and the panel of the selected one under it or beside it: the sections of a
settings page, the views of a record, the filters over a list.

```rust
use lurq::ui::{Tab, Tabs, TabsOrientation, TabsVariant, icons};

// section: Signal<&'static str>. Selecting a tab sets it.
Tabs::new(section, [
  Tab::new("general", "General").panel(|| general()),
  Tab::new("appearance", "Appearance").icon(icons::SETTINGS).panel(|| appearance()),
  Tab::new("advanced", "Advanced").disabled(true),
])
.label("Settings");

// A row over a line, with counts, the archived set apart.
Tabs::new(memory, [
  Tab::new("all", "All").count(7),
  Tab::new("memory", "Memory").count(9),
  Tab::new("archived", "Archived").count(0).separated(true),
])
.variant(TabsVariant::Line);

// The app holds which is selected, and is told the tab a person selects.
Tabs::controlled(filter, [(0, "Overview"), (1, "Activity")], move |index| set_filter(*index))
  .variant(TabsVariant::Pill)
  .orientation(TabsOrientation::Vertical);

// More tabs than the width holds, kept between renders.
Tabs::new(canvas, [(0, "Canvas"), (1, "Layout"), (2, "Text")])
  .variant(TabsVariant::Line)
  .overflow(true)
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `Tabs::new(Signal<T>, tabs)` | The selected tab is the one with the signal's value; selecting another sets it. A tab is a `(value, &str)` or a `Tab` |
| `Tabs::controlled(value, tabs, on_change)` | The app holds which is selected: the tab of `value`, and `on_change` is told the value of each other tab a person selects |
| `on_change(Fn(&T))` | Told the value of the tab a person selects (a press, the arrow keys, the menu of the overflowing form), after the signal is set. Not what the app sets, nor a press of the selected tab. A second one is called after the first |
| `variant(TabsVariant)` | `Default`: a muted rail with the selected tab lifted out of it. `Line`: a row over a line, the selected tab underlined. `Pill`: round tabs, the selected one in the primary fill |
| `orientation(TabsOrientation)` | `TabsOrientation` is `ui::Orientation`. `Horizontal` (the default): the list along the top, the panel under it. `Vertical`: the list down the start side, the panel beside it |
| `manual(bool)` | The arrow keys move the focus alone, and Enter or Space selects: for panels slow to build. Otherwise the selection follows the focus |
| `label(&str)` | What the list is called to what reads the tree |
| `overflow(bool)` | The overflowing form: the list scrolls between two scroll buttons, and More at its end opens a menu of every tab |
| `note(&str)`, `end(element)` | Quiet words at the end of the list's line (a caption in `MutedForeground`, 16 from the tabs, part `TabsPart::Note`), and an element of the app's after them (8 from them, part `End`): why a tab cannot be opened, a button that acts on the page. They give way first where the line is short, the words ending in an ellipsis; the element is no tab, and the arrow keys pass it by. In the default form they stand in the rail; in the overflowing form at the end of the list that scrolls |
| `reserve_selected_width(bool)` | Each label keeps the room it takes in the selected tab's style, a heavier weight, selected or not: a copy so drawn, no higher than nothing and hidden from tooling, holds it. Selecting another tab then moves none after it, as a browser's tab bar keeps its tabs in place. Off unless told; tabs that give way in a room too narrow (built with nothing kept) keep no room |
| `tab_stop(bool)` | `false`: Tab passes the tabs and the More button by, and a press selects a tab and leaves the focus where it was; the arrows move along them once the app focuses one. The panel is a stop as before. See [pointer-only pressables](#pointer-only-pressables) |
| `mount(ctx)` | Puts the tabs in the tree of the app's component, where the overflowing form keeps where its list is scrolled and whether its menu is open. Converted into an element on their own (`build(theme)`, `Element::from`), the tabs are the still form: scrolled to the start, a menu that does not open |
| `more_label(&str)`, `scroll_back_label(&str)`, `scroll_on_label(&str)` | The words of the overflowing form: what the More button says and is called (it says "More" and is called "More tabs" unless told otherwise), and the names of the scroll buttons, "Scroll back" and "Scroll on" |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `disabled` makes every tab disabled. `force` draws the tab Tab stops on, the selected one, and not the panel. `id` is the root's; in the overflowing form the scroll buttons are `<id>-back` and `<id>-on` and the More button `<id>-more` |
| `Tab::new(value, label)` | A tab. `icon(name)` before the label, `count(n)` after it, `badge(n)` a count of what waits for the person in a pill after them (part `Badge`, its id `<id>-badge`), `disabled(bool)`, `disabled_reason(&str)` disabled and why (see below), `separated(bool)` a separator before it (a rule between groups of tabs), `dot(PaletteColor)` a dot of 6 in a role of the palette after the count and the badge (part `Dot`, `<id>-dot`), `count_pending(bool)` a bone as wide as a digit in the count's place while the count is read (in the part `Count`, `<id>-count`, the bone `CountSkeleton`; the tab says `a11y_busy`), `inert(bool)` a tab a press leaves alone (drawn as it is, selected or not, but no press, menu item or arrow key selects or focuses it; state `TabsState::Inert`), `trailing_icon(name)` an icon after the label, the count, the badge and the dot (part `TrailingIcon`, `<id>-trailing_icon`, 14 square): the lock of a tab not built yet, `at_end(bool)` the tab stands at the end of the line along the top, after the room that grows, the note and the app's element, with the other tabs so placed, as far apart as the list's tabs (a page's Settings after its lists; it is still a tab of the list, its arrow keys reach it after the tabs at the start and the More menu lists it in its order; mounted tabs count it in what the list needs to stand whole; down the side it stands after the others), `panel(Fn() -> Element)` what it shows while selected, `on_open_aside(handler)` called for the middle button or a press with Ctrl (Cmd) in place of selecting it (see [pointer-only pressables](#pointer-only-pressables); its item in the More menu is chosen as before), `id` (its item in the More menu is `<id>-item`), `key` |

- Exactly one tab is selected, and the app holds which. A press of a tab selects it; a press of the selected one or of a disabled one asks for nothing, and tells nothing.
- The list is one stop of Tab, at the selected tab. The arrow keys of the orientation (Left and Right, or Up and Down) select the tab after and the one before, Home and End the first and the last, and a disabled tab is passed by; with `manual(true)` they move the focus and Space or Enter selects. Tab from the list goes to the panel, and on to what is in it.
- A tab disabled with a reason (`Tab::disabled_reason`) is a disabled tab whose reason is its tooltip, shown as the pointer rests on it, and its description to tooling (`a11y_description`). A renderer gets the tab with its tooltip from `view.tab_element(index)` (`view.tab(index)` is the button alone); `TabEntry` has the `reason` and the `badge`.
- The panel of a tab is built only while the tab is selected, keyed by the tab, so one tab's panel never takes another's nodes. What the runtime keeps of a panel's nodes (a scroll offset, a caret, an open select) goes when another tab is selected; what the app keeps in its own signals stays. A panel that is to come back as it was keeps its state in the app: its values in signals made in the app's `create`, its scroll in a `ScrollState` it gives the scroll area (`with_scroll_state`).
- The panel is a stop of Tab itself, so a panel with nothing focusable in it is still reached and read; its ring is the color and width of the focus ring, 4 out from the panel, clear of what runs to its edges.
- Parts: `TabsPart::Root` (the list and the panel), `List`, `Tab` (what is pressed and focused), `Label`, `Icon`, `TrailingIcon`, `Count`, `Badge` (lurq's, the board has none: a pill of `Primary` padded 6 and 1, its count 11/500 in `PrimaryForeground`, the other way round on a selected pill tab), `Dot`, `CountSkeleton` (7 by 10), `Separator`, `Note`, `End`, `Panel`, and in the overflowing form `Bar` (the row of the list, its buttons and More), `ScrollButton`, `ScrollIcon`, `More`, `MoreLabel`, `MoreIcon`. A tab's parts are styled in its states: `TabsState::Selected`, `Disabled`, `Inert`; the whole has `Horizontal` or `Vertical`, and `Overflow`. Variants: `TabsVariant::Default`, `Line`, `Pill`.
- In the design the default list is a rail of `Muted`, `radius-lg`, padded 3, its tabs 2 apart; a tab is 28 high, padded 12, `radius-md`, its label 13/500 in `MutedForeground` and a count of 12 in `MutedForeground` 6 after it; the selected tab is `Background` with the small shadow and its label in `Foreground`. The line list is 44 high over a line of 1 in `Border`, as wide as what holds it, its tabs 28 apart; a label is 14 in `MutedForeground` and a count 13 at 0.7, 8 apart; the selected tab is underlined with 2 in `Foreground`, its label 500 in `Foreground` and its count whole. A separator is 1 by 16 in `Border`. Down the side the line list is at least 180 wide with a line of 1 in `Border` at its start, a tab 32 high, padded 12, its label 13, and the selected tab marked with 2 in `Foreground` at its start. The pills are 30 high, padded 14, round, 4 apart, the label 13/500; the selected one is `Primary` with its label in `PrimaryForeground`. The panel is 16 under the list, or 24 beside it.
- A tab follows the [density](../theme/#density) by the steps of a row, 8 lower at compact, 8 and 12 higher at comfortable and touch, from the design's height of each form: the default's 28 is 20, 36 and 40; a line's 44 (its list with it) is 36, 52 and 56, its 32 standing and its 40 overflowing (the bar and the list with it) the same steps; a pill's 30 is 22, 38 and 42.
- The overflowing form is a row 40 high (over the line in the line variant): the list, its tabs 20 apart with labels of 13, between two scroll buttons 28 wide with chevrons of 14 in `Foreground`, and More at the end, set apart by a line of 1 in `Border`, padded 8 and 4, its label 13 and a chevron of 14 in `MutedForeground`. A scroll button moves the list most of the way it shows; it is for the pointer and no stop of Tab, since the arrow keys scroll the list to the tab they select. More opens a menu of every tab (`ui::Menu`), the selected one marked as a radio and a disabled one not to be chosen; choosing one selects it and scrolls the list the least that shows it.
- A tab under the pointer has its label in `Foreground`; a disabled tab is dimmed and does not light up. The focus ring is the tab's; in the overflowing form it is drawn inside the tab, where the list that scrolls does not cut it off.
- To what reads the tree (`lurq_snapshot` of the MCP server) the list is a `tablist` called by `label`, with `a11y_orientation`; a tab is a `tab` called by its label, with `a11y_selected` (which `lurq_inspect` reports as `state.selected`); the panel a `tabpanel` called by its tab. More has `a11y_haspopup` `menu` and `a11y_expanded`.
- A renderer that draws other tabs has `view.tabs` (each with its label, icon, count, and whether it is selected, disabled, set apart), `view.part(part)`, `view.tab_part(index, part)`, `view.tab(index)` (a tab with its behavior and its name to tooling), `view.list()` (the tabs as one focus group), `view.separator()`, `view.panel()` and `view.bar()` (the overflowing form).
- `lurq::components::Tabs` is another thing: a set of places of an app, each a router with a history of its own kept while it is not shown, with a bar to move between them across the bottom or the top. Tabs of the design system are a choice within a page.

**In a narrow room.** Tabs put in the tree with `mount` measure their room before a frame is painted: while the
list along the top is wider than it, they are their overflowing form (the list between two scroll buttons, More at
its end), scrolled to the selected tab, with no frame of the other form, and the list alone again when it fits.
`overflow(true)` keeps that form whether or not the list fits. Tabs converted into an element on their own cannot
measure: in a room too narrow they end their labels in an ellipsis, every tab still in the room. A theme's renderer
gives the box that is the tabs' room `TabsView::room_ref`. The form is the board's Tabs/Overflow. Its tabs of a phone (Mobile/Scrollable Tabs: 48 high, labels of 15, scrolled with no buttons) are another dress of the same scrolling list, which an app gives with rules; the room does not choose it.

## Accordion

Headers one under the other, each of which opens what is under it: questions and their answers, the sections of a long
form, settings in groups. One item is open at a time, or any number.

```rust
use lurq::ui::{Accordion, AccordionItem};

// One open at a time, or none: open is a Signal<Option<u8>>, which a press sets.
Accordion::optional(open, [
  AccordionItem::text(0, "Is lurq GPU-accelerated?", "Yes. Rendering goes through wgpu."),
  AccordionItem::text(1, "Does it support dark mode?", "Every theme has two appearances."),
  AccordionItem::new(2, "Can I bring my own theme?", theme_form).disabled(locked),
]);

// Any number open: sections is a Signal<Vec<&str>>. Each press is told the whole set open after it.
Accordion::multiple(sections, [
  AccordionItem::text("billing", "Billing", "Card, address and invoices."),
  AccordionItem::text("shipping", "Shipping", "Where the order goes."),
])
.on_change(|open: &[&str]| save(open));
```

| Builder | |
| --- | --- |
| `Accordion::optional(Signal<Option<T>>, items)` | One item open at a time: the one whose value the signal has, or none. A press of another opens it and closes the one that was open; a press of the open one closes it. An accordion has no form of which one must be open, so it has no plain `new` |
| `Accordion::multiple(Signal<Vec<T>>, items)` | Any number open: those whose values the signal has. A press opens an item, or closes it |
| `Accordion::controlled_optional(Option<T>, items, on_change)`, `Accordion::controlled_multiple(open, items, on_change)` | The app holds what is open, one at a time or any number, and `on_change` is told what is to be open after a press of a header: the item or none, or the whole set. The accordion opens and closes nothing itself |
| `on_change(Fn(&Option<T>))`, `on_change(Fn(&[T]))` | Told the accordion's value after a press of a header, once the signal is set: the item open or none for one at a time (an `Accordion<T>`), every item open for any number (an `Accordion<T, [T]>`), checked by the compiler. Not what the app sets. A second one is called after the first |
| `label(&str)` | What the accordion is called, to what reads the tree: a `group` of that name |
| `AccordionItem::new(value, label, content)`, `AccordionItem::text(value, label, body)` | An item: its value, what its header says, and what it shows while it is open, an element or a text. `disabled(bool)`, `id` (its header's) and `key` on an item |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `disabled` is every header's; `state` is every item's; `force` draws the header Tab stops on first, that of the first item that is not disabled |

- Each header is a stop of Tab, as a web accordion's is, and Space and Enter press it; Up and Down move the focus from
  one header to the next and round, Home and End to the first and the last. A disabled header is passed by, and so is
  what an open item holds: a button in it is a stop of Tab between its header and the next, and the arrows are not the
  accordion's while the focus is on it (`FocusGroup::none()` around the content).
- What an item holds opens and closes with its height, over the theme's base duration, and fades in and out with it:
  the panel's height goes from none to its content's, which the layout reads as it goes. Where motion is reduced it is
  there at once. While an item is closed what it held is not there: not painted, and the controls in it are no stops
  of Tab.
- Parts: `AccordionPart::Root` (the items), `Item` (an item, with the line under it), `Trigger` (its header: what is
  pressed and focused), `Label`, `Chevron`, `Panel` (what opens and closes) and `Body` (what the panel holds, in its
  padding). Each item's parts are styled in that item's states, `AccordionState::Open` and
  `Disabled` (`AccordionState` is `CollapsibleState`).
- In the design each item has a line of 1 in `Border` under it. Its header is 14 above and under its label, which is
- A header follows the [density](../theme/#density): 10, 18 and 20 above and under its label at compact, comfortable and touch, the steps of a row from the design's 14.
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

// open: Signal<bool>, which a press of the header turns.
Collapsible::new("3 crates in this workspace", crates).bind(open);

// The app holds whether it is open.
Collapsible::text("Why is this here?", "Because the build needs it.")
  .open(shown)
  .on_open_change(move |next| show(next));
```

| Builder | |
| --- | --- |
| `Collapsible::new(label, content)`, `Collapsible::text(label, body)` | A header that says `label`, and what it shows while it is open: an element or a text |
| `open(bool)` | Whether what it holds is shown. The collapsible does not keep it: the app says so |
| `bind(Signal<bool>)` | Open while the signal is true, which a press of the header turns; the handlers of `on_open_change` are told after it is set |
| `on_open_change(Fn(bool))` | Called on a press of the header with what the collapsible is to be: open if it was closed. Not for what the app sets. A second one is called after the first |
| `icon(..)`, `open_icon(..)` | The icon of the chevron by its name in the theme's set, `chevrons-vertical` unless told otherwise, and the one while it is open, the same unless told otherwise. `icon(None)` takes the chevron away |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the header |

- The whole header is pressed; Tab reaches it, and Space and Enter press it. What it holds opens and closes as an
  accordion's item does.
- Parts: `CollapsiblePart::Root`, `Trigger`, `Label`, `Chevron`, `Panel`, `Body`. States: `CollapsibleState::Open`,
  `Disabled`.
- In the design the label is in the label's type (13, medium), `Foreground`; at the end of the header the
  `chevrons-vertical` icon of 16 is in a ghost button of 32 that `Highlight` fills under the pointer; what it opens is 8
  under the header. A disabled header is dimmed, and what it opens, which is the app's, is not.
- To what reads the tree the header is a `button` called by its label, with `a11y_expanded`.
- A renderer that draws another collapsible (`CollapsibleRenderer`) starts from `view.trigger()`, the behavior with the
  header's style, and `view.panel()`, what opens and closes with what it holds; `view.label()` and `view.chevron()`
  (none where it has no icon) are the default ones.

## Breadcrumb

The way to where one is: a step for each place on it, each but the last a link back there, the last the page one is on. As a path control, the folders of a path in a bar, as a file manager has them.

```rust
use lurq::ui::{Breadcrumb, BreadcrumbSeparator, BreadcrumbStep, BreadcrumbVariant, icons};

Breadcrumb::new([
  BreadcrumbStep::new("Settings").value("/settings"),
  BreadcrumbStep::new("Appearance").value("/settings/appearance"),
])
.step("Theme")
.on_select(|route| go_to(route));

// More steps than four: the first, an ellipsis that opens a menu of the hidden ones, and the last three. Mounted,
// it keeps whether the menu is open.
Breadcrumb::new([BreadcrumbStep::new("Home").icon(icons::HOME).value("/")])
  .steps(["Projects", "lurq", "crates", "ui", "breadcrumb.rs"])
  .max_shown(4)
  .separator(BreadcrumbSeparator::slash())
  .mount(ctx);

// A path control: the parts of a path are its steps, each told as the path as far as it. Mounted, it folds the
// middle of a path longer than its room into the menu of the ellipsis.
Breadcrumb::path("H:\\projects\\lurq\\crates").on_select(|folder| open(folder)).mount(ctx);

// The same bar for steps of the app's own.
Breadcrumb::new([
  BreadcrumbStep::new("Macintosh HD").icon(icons::DRIVE).value("/"),
  BreadcrumbStep::new("projects").icon(icons::FOLDER).value("/projects"),
])
.variant(BreadcrumbVariant::Path);
```

| Builder | |
| --- | --- |
| `new(steps)` | A breadcrumb of these steps, in order. The last is the page one is on: text, not a link. A `&str` is a step of that label. `Breadcrumb::default()` has none |
| `step(impl Into<BreadcrumbStep>)`, `steps(iter)` | Steps, in order, after those it has |
| `Breadcrumb::path(&str)` | A path control: the path of a folder or a file as a bar of its parts, the `Path` variant of `BreadcrumbStep::path`'s steps, called "Path", which overflows. `on_select` is told the path as far as the part pressed |
| `BreadcrumbStep::path(&str)` | The steps of a path, apart at `/` and `\`: a drive (`H:`, whose value is `H:\`), the root (`/`) or a share (`\\nas\media`) first, with the icon of a drive, a `~` with a home's, the rest with a folder's. A path to a file is given the file's icon by the app, on the last step |
| `variant(..)` | `BreadcrumbVariant::Plain` (the default: links in a line of text) or `Path` (segments in a bar, the last in a box of its own), or a variant the theme's recipe names |
| `overflow(bool)` | Mounted, the steps it has no room for go into the menu of the ellipsis, the ones nearest the first first, the first and the last never, and come back as it is given room. With `max_shown` too, no more than that many are shown where there is room for more. It does nothing to a breadcrumb that is not mounted: that one is its still form (see below) |
| `on_select(Fn(&str))` | Called with the value of the step followed, after the step's own handlers |
| `tab_stop(bool)` | `false`: Tab passes the links and the ellipsis by, and a press follows a link and leaves the focus where it was. See [pointer-only pressables](#pointer-only-pressables) |
| `max_shown(usize)` | The most steps shown. With more, the first, an ellipsis and the last `max_shown - 1`; never fewer than two. All of them without it |
| `link_max_width(length)` | The most a link is wide (a number, or a `Length`): a longer name is cut there with an ellipsis, and a tooltip says it whole while it is cut (`Tooltip::when_cut`). The links keep their room, so the page one is on gives way first where the row is short. A rule of `max_width` on `BreadcrumbPart::Link`, and `BreadcrumbView::cut_links`. Mounted, the refs the tooltips read are kept from render to render |
| `mount(ctx)` | Puts it where it goes, keeping whether the menu of the hidden steps is open and what it hangs from, and which steps one that overflows has no room for. Built on its own (`build`, `.into()`), its ellipsis opens nothing unless it is given `menu`, and it drops no step |
| `menu(Signal<bool>, ElementRef)` | Whether the menu of the hidden steps is open, and what it hangs from, where the app keeps them, as for a [menu](#menu). A press of the ellipsis turns it; choosing a step, a press outside and Escape close it |
| `label(&str)`, `more_label(&str)` | What the whole and the ellipsis are called to what reads the tree: "Breadcrumb" and "Show the hidden steps" unless told otherwise |
| `separator(BreadcrumbSeparator)` | What stands between two steps: `BreadcrumbSeparator::chevron()` (the default), `slash()`, `Icon(name)` or `Text(..)` |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the first link, the one Tab stops on first, in the state. The ellipsis's id is the root's with `-ellipsis` after it |
| `BreadcrumbStep::new(label)` | A step. `value(&str)` is what `on_select` is told (its label until then), `icon(name)` an icon before its text, `on_select(Fn())` called when it is followed, `on_open_aside(handler)` called for the middle button or a press with Ctrl (Cmd) on its link in place of following it (see [pointer-only pressables](#pointer-only-pressables)), `disabled(bool)` dimmed and not followed (Tab passes it, and its item in the menu is disabled), `state(name, on)` a state the theme's recipe names for this step alone (a key in mono, a last step drawn as the steps before it: `recipe.when(Selector::new().state("ancestor"), BreadcrumbPart::Page, ..)`), `id` the id of its link, of the page, or of its item in the menu while it is hidden, `key` the key of its link |

- A link is followed by a click, and by Enter or Space while it is focused. The links and the ellipsis are stops of Tab in their order; the page is not. Choosing a hidden step in the menu follows it as a press of its link would.
- Parts: `BreadcrumbPart::Root` (the row), `Link` (a step that is a link: what is pressed, with its icon and text), `Page` (the last step), `Icon` (a step's icon), `Separator` and `SeparatorIcon`, `Ellipsis` (what is pressed to open the menu) and `EllipsisIcon`. States: `BreadcrumbState::Collapsed` (some steps are hidden), `Disabled` (of the whole, and of a step alone: it dims the link, the page, the separators and the ellipsis, so that a step can be dimmed by itself). A step's own states (`BreadcrumbStep::state`) are on the parts of that step alone: its link or page and its icon.
- In the design the steps are 8 apart in the body's 14; the links are in `MutedForeground` and the page in `Foreground` at 500; a chevron of 14 in `MutedForeground` stands between each two. The ellipsis is a square of 24 filled with `Highlight` (the board's `$accent`), with the small radius and an icon of 16 in `Foreground`. Under the pointer a link is in `Foreground`; the ring of the focus is around the link. A step's icon is 14, 4 before its text. A disabled breadcrumb is dimmed, follows nothing and is no stop of Tab.
- The path control of the design (`Path Control`) is the `Path` variant: a bar 28 high on `Muted` with the medium radius, 4 in from its sides, its segments touching. A segment is 22 high with the small radius, 6 in, an icon of 13 in `MutedForeground` 5 before a text of 12 in `Foreground`; a chevron of 12 in `MutedForeground` stands between each two; the last, the folder one is in, is on `Background` in an edge of 1 in `Border`. The board draws no segment under the pointer and no ellipsis in the bar: a segment has the `StateHover` and `StatePressed` veils, and the ellipsis is a square of 22 with an icon of 13.
- **In a room narrower than its steps** a breadcrumb has two forms. The one that folds is `overflow(true)` with `mount(ctx)`: it measures before the frame is painted, so its first frame at a width is the settled one, and where even the first step, the ellipsis and the last have no room, the last is cut with an ellipsis. The still one is every other breadcrumb: one built on its own (`build`, `.into()`), with `overflow` or without, and a mounted one without `overflow`. It keeps nothing of what was measured, so it folds nothing: its root is no wider than its room, its last step gives way and is cut with an ellipsis, and the steps before it keep their widths, so that links wider together than the room run past its end. A breadcrumb in a window a person can narrow is the folding one; a still one is for a place that always has its room, or one that hides by count with `max_shown`.
- A theme gives the separator a renderer of its own with `set_part_renderer::<Breadcrumb, BreadcrumbPartRenderer>(BreadcrumbPart::Separator, ..)`, built once for each place between two steps; or restyles it by the recipe.
- To what reads the tree (`lurq_snapshot` of the MCP server) the root is a `navigation` called "Breadcrumb" (or its `label`), each link a `link` called by its label, the page has `a11y_current` `page`, and the ellipsis, called "Show the hidden steps" (or its `more_label`), says `a11y_haspopup` `menu` and whether it is `a11y_expanded`.
- A renderer that draws another breadcrumb has `view.shown()` (the steps to lay out, by index: all but those over `max_shown`; `view.hidden` is what the ellipsis stands for, those a row dropped among them), `view.step(index)` (a link with its behavior, or the page), `view.separator()`, `view.ellipsis()` (the button with its menu, while some steps are hidden), `view.root()` and `view.follow(index)` (what following a step does).

## Pagination

The pages of a list, one of them the current one: the results of a search, the rows of a table, the posts of a feed.

```rust
use lurq::ui::{Pagination, Select};

// Pages count from 1, as a person reads them. The signal has the current one, and a press sets it.
Pagination::new(page.clone(), 20);

// The footer of a table: 284 rows, so many to a page, with a select of how many.
Pagination::for_items(page.clone(), 284, size.get())
  .compact(true)
  .page_size("Rows per page", Select::new(size.clone(), [(10, "10"), (20, "20"), (50, "50")]));

// The app holds the page: on_change is told the page of each press.
Pagination::controlled(current, 12, move |to| go(to));
```

| Builder | |
| --- | --- |
| `new(Signal<usize>, pages)` | `pages` pages, of which the current one is the signal's, counted from 1 |
| `for_items(Signal<usize>, total, page_size)` | The pages of `total` items, `page_size` to a page |
| `controlled(current, pages, on_change)` | The app holds the page: `current`, counted from 1, and `on_change` told the page of each press |
| `items(total, page_size)` | The pages of so many items, in place of a number of pages; the compact form then says "1–20 of 284" |
| `siblings(usize)` | How many pages are drawn on each side of the current one: 1 unless it is said |
| `compact(bool)` | Previous and next with where the list is, and no pages |
| `page_size(label, Select)` | A select of how many to a page, after its label, at the start; the pagination is then as wide as what holds it |
| `summary_text(&str)` | What is said of where the list is, in place of "1–20 of 284" or "Page 3 of 12": in another language |
| `previous_label(&str)`, `next_label(&str)`, `page_label(&str)` | What previous and next are called, "Previous page" and "Next page", and the word before a page's number, "Page" of "Page 3", unless told otherwise |
| `label(&str)` | What the whole is called to what reads the tree: "Pagination" unless it is said |
| `on_change(handler)` | Called with the page of each press |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the first button that takes input (previous, or the first page that is not the current one) in the state. Its buttons have ids after the pagination's: `{id}-previous`, `{id}-next`, `{id}-page-3` |

- Which pages are drawn is the pagination's: the first and the last always, the current one with its siblings on each side, and an ellipsis where pages are left out. While every page fits in `2 × siblings + 5` every page is drawn; once they do not, that many things are drawn wherever the current page is, so that the row does not change its length as a person goes through it. Near an end the pages run on to that end and take the room of the ellipsis that is not needed there: on page 1 of 20, `1 2 3 4 5 … 20`; on page 10, `1 … 9 10 11 … 20`; on page 20, `1 … 16 17 18 19 20`. An ellipsis always stands for two pages or more. `Pagination::pages_shown(current, pages, siblings)` says what is drawn, as `PageItem::Page(n)` and `PageItem::Ellipsis`.
- A current page past the ends is drawn as the end it is past; the signal is not changed. With no pages, previous and next are drawn, disabled, and nothing between them.
- Previous is disabled on the first page and next on the last. A press of a page goes to it; a press of the current one does nothing.
- Each page, previous and next is a button and a stop of Tab of its own; Left and Right move along the row (`FocusGroup::horizontal()`), from the last on to the first, and Home and End go to its ends. Enter and Space press the button the focus is on. A disabled button is passed by, and one that is disabled while the focus is on it, as next is by Enter on the page before the last, passes the focus to the button before it: the keyboard stays in the pagination.
- The page-size select is the app's `ui::Select`, bound to its own signal; the pagination gives its trigger the board's size over the select's recipe (`PaginationPart::PageSize`, laid over `SelectPart::Trigger`), and calls it by the label beside it.
- To what reads the tree (`lurq_snapshot` of the MCP server) the whole is a `navigation` called "Pagination" (or its `label`), each page a button called "Page 3" (after its `page_label`), and the current one has `a11y_current` "page". Previous and next are called "Previous page" and "Next page" (or their labels).
- Parts: `PaginationPart::Root`, `Nav` (the row of buttons the arrow keys move along), `Item` (laid over each button's root), `Ellipsis`, `EllipsisIcon`, `Summary`, `PageSizeLabel`, `PageSize` (laid over the select's trigger). States: `PaginationState::Compact`, `PageSize`, `Disabled`, and `Current` of an item: the current page's button.
- In the design the pages are ghost buttons of 36, 4 apart, the current one an outline button; previous and next are ghost buttons of 36 with a chevron of 16. The ellipsis is a box of 36 with `more-horizontal` of 16 in `MutedForeground`. A page button is at least 36 wide and 6 from its sides, so that a number of four digits or more still fits. The compact form has outline buttons of 30 for previous and next, 12 after where the list is, in the body's type at 13 in `Foreground`; its page-size select is 72 by 30 with its text at 13, 12 after its label in `MutedForeground`. A disabled button is dimmed as a button is.
- A renderer is given a `PaginationView`: where the list is (`current`, `pages`, `shown`, `summary`), and `view.previous()`, `view.next()`, `view.page(n)`, `view.ellipsis(i)`, `view.summary()`, `view.nav()` (the row of buttons) and `view.page_size()` (its label and the select) built in the recipe's style with their behavior.

**In a narrow room.** A pagination put in the tree with `mount` reads its room before a frame is painted. Narrower
than its buttons, it draws fewer pages on each side of the current one, down to the first, the current and the last
between two ellipses; narrower than those, it says where the list is between previous and next ("Page 6 of 12").
With room again it draws what the app says. Its root wraps: beside a select of how many to a page the pages go on
to a line under it, and a table's footer puts its pagination under its count. The board draws no narrower form than
its compact one (Pagination/Compact, where the list is between previous and next), which is what the last step
is. A pagination that is only built (converted into an element, or `build`) cannot measure: it draws the pages the
app says in any room, and only its root wraps; `mount(ctx)` is the one that measures.

## Steps

The stages of a process in order, with one current: those before it are done, those after it are to come.

```rust
use lurq::ui::{Orientation, Step, Steps};

// Shown, not operated: the app moves the current step itself. page: usize.
Steps::at(page, ["Account", "Workspace", "Finish"]);

// A wizard that lets a person go back to a step that is done: a press sets the signal.
Steps::new(at.clone(), [
  Step::new("Account").description("Name and email"),
  Step::new("Billing").description("Card and address").invalid(billing_failed),
  Step::new("Finish"),
])
.orientation(Orientation::Vertical)
.label("Sign up")
.on_change(|index| println!("back to step {index}"));

// The current step held by the app, which answers a press itself.
Steps::controlled(page, ["Account", "Finish"], move |index| go_to(index));
```

| Builder | |
| --- | --- |
| `Steps::new(current, steps)` | `current` is a `Signal<usize>` that holds the index of the current step, from 0, and that a press of a step sets. Past the last step every step is done. A step is a `&str`, a `String` or a `Step` |
| `Steps::controlled(current, steps, on_change)` | The index held by the app: a press tells `on_change`, and the steps show what the app answers with |
| `Steps::at(current, steps)` | The index of the current step, shown and not operated: the steps of a wizard the app drives. No step is pressed, whatever `pressable` and `on_change` say, and the steps are not a stop of Tab |
| `orientation(Orientation)` | `Horizontal` (the default): in a row, a line between each step and the next. `Vertical`: in a column, a line down from each step's mark to the next. `StepsOrientation` is a name of `ui::Orientation` |
| `on_change(Fn(usize))` | Called with the index of the step a person made current, after a signal given to `new` is set; not when the app sets it. Makes the steps that are done pressable. A second handler is called after the first |
| `pressable(bool)` | Makes the steps that are done pressable with no handler: for steps bound to a signal |
| `ahead(bool)` | Where steps are pressed, a step to come is pressed too: a process whose steps can be taken in any order |
| `label(&str)` | What the list of steps is called to what reads the tree |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the step Tab would enter at in the state. With an `id`, each step has the id `<id>-<index>`, its mark `<id>-<index>-marker`, and the line after it `<id>-<index>-line` |
| `Step::new(title)` | A step. `description(&str)` a line under the title, `icon(name)` an icon of the theme's set in place of its number until it is done, `invalid(bool)` that it has gone wrong, `disabled(bool)` that it takes no press and is dimmed, `id` the id of its element in place of `<id>-<index>`, `key` |

- In a row the steps share the width: where they do not fit, a step gives way before the lines, which are never shorter than 24. The descriptions wrap first, each step down to its title; only when every step is at its title is a title cut with an ellipsis, the last step's first (`ShrinkLimit::ContentFirst`).
- Steps are read, not operated, unless the app says they are pressed (`on_change`, `controlled` or `pressable`). Then a step that is done is pressed to go back to it, and a step to come only with `ahead(true)`; the current step and a disabled one change nothing. The steps are one stop of Tab, entered at the current step (at the last one pressed where every step is done); the arrow keys of the orientation, Home and End move between the current step and those that are pressed, and Space and Enter press. Steps that are only read take no press and no focus.
- Parts: `StepsPart::Root` (the steps and the lines), `Step` (a step: what is pressed and focused), `Marker` (its round mark), `Number`, `Icon` (the check, the cross, or the step's own icon), `Text` (the title and the description), `Title`, `Description`, `Rail` (in a column, the mark with the line down from it), `Connector` (what holds a line) and `Line`. Each step's parts are styled in that step's states: `StepsState::Done`, `Current` or `Upcoming`, and `Invalid`, `Pressable` and `Disabled` (that step alone); the whole has `Disabled`. The orientation is the state `StepsState::Horizontal` or `Vertical`.
- In the design the mark is 24 and round: in `Primary` with a check of 14 in `PrimaryForeground` once the step is done, in `Primary` with its number (12, medium) while it is current, and with a 1 px edge in `Input` and the number in `MutedForeground` while it is to come. The title is 8 beside the mark in the label's type (13, medium), `Foreground`, and `MutedForeground` while the step is to come. A line of 1 runs from 12 after one step to 12 before the next, in `Primary` up to the current step and in `Border` after it; in a row the lines share what the steps leave of the width. A description is under the title in the caption's 12, `MutedForeground`. An invalid step has a cross on `Destructive` and its title in `Destructive`, whatever else it is.
- A pressed step has a veil of `StateHover` under the pointer and of `StatePressed` held down; the ring of the focus is around the step, which is padded 4 for it. Disabled steps are dimmed and take nothing.
- To what reads the tree (`lurq_snapshot` of the MCP server) the root is a `list` (called by `label`), each step a `listitem`, or a `button` where it is pressed, called by its place, title and state ("Step 2 of 4, Billing, current", with ", error" where it is invalid); the current step has `a11y_current` `step`.
- A renderer that draws other steps has `view.steps` (each step with its `status`, whether it is `pressable` and its resolved recipe), `view.step_part(index, part)`, `view.step(index, content)` (a step with its behavior and its name to tooling), `view.marker(index)`, `view.text(index)`, `view.connector(index)` and `view.step_name(index)`. A bar of segments, as the mobile skin draws progress, is such a renderer: a `Line` of each step, and the current step's title under them.

**In a narrow room.** Steps across put in the tree with `mount(ctx)` measure their room before a frame is
painted. Their marks are always whole, and the titles end in an ellipsis first. Under the room the titles need
beside the marks, steps that are only read are the board's progress of a phone (Mobile/Step Progress, the
`progress` state): a segment for each step, 4 high and 4 apart, in the primary color up to the current step, over
a line that says where the process is, "Step 2 of 4 · Billing" (parts `segments`, `segment`, `summary`). Steps a
person presses keep every step to press: the titles stand under their marks (the `stacked` state), and under the
room the marks and their lines need, the steps run down the page; the board draws neither of these two. Steps that
are only built (converted into an element, or `build`) cannot measure: they stay a row, keep their marks whole and
cut their titles.

## Table of Contents

The list of a page's sections, with the one being read marked: "On this page" beside an article, the outline of a document. It follows the scroll of the container the app names, and a press of an entry brings its section to the top.

```rust
use lurq::{core::ElementRef, ui::{TableOfContents, TableOfContentsEntry}};

// In `create`: a ref for each section, and one for what scrolls them.
let (page, theme, palette, roles) = (ElementRef::new(), ElementRef::new(), ElementRef::new(), ElementRef::new());

// In `render`: each section has its ref on it (`.ref_element(theme.clone())`), the scroll has `page`.
TableOfContents::new([("Runtime theme", &theme), ("Palette", &palette)])
  .entry(TableOfContentsEntry::new("Extra roles", &roles).level(2))
  .container(&page)
  .on_change(|index| println!("reading section {index}"))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `TableOfContents::new(entries)` | Its entries: `TableOfContentsEntry`s, or pairs of a label and the ref of a section |
| `entry(..)`, `entries(..)` | Adds to them |
| `mount(ctx)` | The list where it is declared: it follows the scroll and goes to a section when its entry is pressed. Converted (`.into()`) it is still: see below |
| `container(&ElementRef)` | What scrolls the sections, by the ref the app gave it: the reading line is measured from its top. With none, from the top of the window |
| `reading_line(impl Into<f32>)` | How far under the top of the container a section's top is when the section becomes the one read: 24 unless told |
| `current(Signal<usize>)` | The entry marked, the app's: the list sets it as the page scrolls and as an entry is pressed, and marks what the app sets. Without it a mounted list keeps the entry itself |
| `on_change(Fn(usize))` | Another section became the one read, by the scroll or by a press: after the signal is set, and not when the app sets it |
| `on_select(Fn(usize))` | An entry was pressed, before the list goes to its section |
| `title(&str)` | What the list is called over its entries, in place of "On this page"; drawn in capitals. An empty one draws no title |
| `label(&str)` | What it is called to tooling, in place of its title |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the marked entry |

| Entry | |
| --- | --- |
| `TableOfContentsEntry::new(&str, &ElementRef)` | What the section is called, and the ref on the section's element, or on its heading |
| `level(usize)` | How deep the section is: 1 for a section of the page, 2 for a section of a section, which is set in |
| `disabled(bool)` | It takes no input, and the keys pass it by |
| `id`, `key` | Of the entry |

- A mounted list marks the last entry whose section's top has come up to the reading line: at the top of a page the first, and each next one as its heading nears the top of the container. Where the container is scrolled to its end, with the last section all in view and the first gone up out of it, the last entry is marked, though its section's top cannot come up to the line; scrolled back, the entry of the section at the line again. It adds observers of its own to the refs of its sections (`ElementRef::add_rect_observer_before_paint`), so the entry is marked in the frame the scroll is drawn in, and an observer the app registered on one of those refs is kept; a new render replaces the list's observers, which go with it.
- A press of an entry scrolls its section to the start of what scrolls it (`ctx.scroll_into_view_start`, clear of what sticks to the top), and marks the entry. A section near the end of the page cannot come up to the line, the page ends first: its entry is the marked one because it was pressed, until the page is scrolled again.
- Converted into an element on its own it is the still form of itself: it marks what its signal says (the first entry with no signal), follows no scroll, and a press sets the signal and tells the handlers, and scrolls nothing. A list that is to stay in view while the page scrolls is in a container of the app's that sticks (`.sticky(Sticky::top(0.0))`).
- To the keyboard it is one stop of Tab, at the marked entry. Up and Down move between the entries and do not wrap, Home and End go to the first and the last, Enter and Space go to the section; the focus stays on the entry. A label longer than the list is cut with an ellipsis on its one line.
- To what reads the tree it is a `navigation`, called by its label or its title, of `link`s called by their labels; the marked one says it is the current location (`a11y_current`).
- Parts: `TableOfContentsPart::Root` (the width, the room between the entries), `Title`, `Entry` (what is pressed: its height, the line at its start, the room before its label), `Indent` (its width is how far a level sets an entry in), `Label`. States, each an entry's own: `TableOfContentsState::Current`, `Disabled`.
- In the design (`i7m0J`) it is 200 wide, its entries 2 apart under a title in the mono face at 11, in capitals a pixel apart, in `MutedForeground`, 12 in and 6 over the first entry. An entry is 28 high with a line of 1 in `Border` at its start and its label 12 after it, 13 in `MutedForeground`; one of the second level is 12 further in. The marked one has a line of 2 in `Primary` and its label in `Foreground` at medium weight. The board draws no pointer: a label is in `Foreground` under it, and the focus is a line of 2 in the ring's color inside the entry. An entry is a row and follows the density.
- A renderer that draws another list has `view.root()`, `view.title()`, `view.entry(index)`, `view.entry_recipe(index)` and `view.render()`.

## Badge

A short label of a state or a count, read and not operated: beta, passing, 3 unread.

```rust
use lurq::ui::{Badge, BadgeShape, BadgeSize, BadgeVariant, Button, icons};

Badge::new("Beta");
Badge::new("Passing").variant(BadgeVariant::Success).icon(icons::CHECK);
Badge::new("Online").variant(BadgeVariant::Success).dot(true);
Badge::new("v0.60").variant(BadgeVariant::Secondary).shape(BadgeShape::Square);

// A dot alone: what it says is what it is called, not drawn.
Badge::new("Unread").variant(BadgeVariant::Info).dot_only(true);

// A count: "99+" past 99, nothing at all at 0.
Badge::count(unread).label("unread messages");

// On the top end corner of what it is about.
Badge::count(3)
  .label("notifications")
  .on(Button::new("Notifications").icon(icons::BELL).icon_only(true));
```

| Builder | |
| --- | --- |
| `Badge::new(label)` | A badge that says `label` |
| `Badge::count(n)` | A count: `n` in a small round badge, `BadgeVariant::Destructive` and `BadgeSize::Sm` unless told |
| `variant(BadgeVariant)` | `Default` (the primary color), `Secondary`, `Outline` (an edge and no fill), and the statuses `Info`, `Success`, `Warning`, `Destructive` (the status's muted fill and its strong text). A count in `Destructive` is solid |
| `size(BadgeSize)` | `Sm` (18 high, at 11), `Md` (22, at 12, the default), `Lg` (26, at 13) |
| `shape(BadgeShape)` | `Pill` (the default) or `Square`, with the theme's small radius |
| `icon(name)` | An icon of the theme's set before the text |
| `dot(bool)` | A dot in the variant's color before the text; the badge then has an edge and no fill, as the design's "Online" |
| `dot_only(bool)` | A dot alone in the variant's color. Its text is not drawn and is still what it is called |
| `max(usize)` | The most a count shows: past it the maximum and a plus. 99 unless told |
| `show_zero(bool)` | Whether a count of 0 is drawn. It is nothing unless told |
| `label(&str)` | On a count, what it counts, which tooling is told with the number |
| `truncate(bool)` | Gives way where it has less room than its word, down to its dot, its icon and its padding, the word ending in an ellipsis; what reads the tree is told the whole word. Off unless told: a badge keeps its width |
| `on(element)` | Puts the badge on the top end corner of `element` |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the badge's own, not the element's it is on |

- A badge is read and not operated: it takes no press, no focus and no stop of Tab, and has nothing to force.
- On an element (`on`), the badge and the element are one element as large as the element; the middle of the badge is on the point 3 in from the element's top end corner, whatever the badge's width, so a dot of 12 is at 27, -3 on a box of 36 (the design's Indicator), a count of one digit reaches 6 past the corner each way, and a wider one as far to either side of the point. It has a ring of 2 in `Background` that parts it from what it is on. It reaches past the element, so what holds them gives it room or does not clip (`overflow_visible()`). A count that is not drawn leaves the element alone.
- Parts: `BadgePart::Root` (the badge: its fill, edge and height; a dot alone is its root), `Label`, `Icon`, `Dot` (the dot before the text), `Anchor` (what holds the element and the badge on it) and `Corner` (a square at the element's top end corner whose centre the badge's round end is centred on: its width is all that is read of it). States: `BadgeState::Dot`, `DotOnly`, `Count`, `OnCorner`, `Square`. The variant and the size are the recipe's.
- In the design a badge is 22 high and round, padded 8, its text 12/500 (the caption's type at medium), an icon of 12 and 4 before the text; small is 18, padded 6, at 11 with an icon of 10; large is 26, padded 10, at 13 with an icon of 14. With a dot it is in `Background` with a 1 px edge of `Border` and its text in `Foreground`, the dot 7, 6 before the text, the badge padded 8 before and 10 after. A count is at 600 and at least as wide as it is high. A dot alone is 8 (6 small, 10 large), and 12 on a corner (10 small, 14 large).
- To what reads the tree (`lurq_snapshot` of the MCP server) a badge is a `status` called by its text; a count is called by its number with what it counts ("120 unread messages"), the whole number however it is shown.
- A renderer that draws another badge has `view.root()` (the root with its style and its name to tooling), `view.badge()` (the default badge), `view.text` (what is drawn: "99+"), `view.name()`, `view.shown` (false for a count of 0 not drawn), and `view.place(element, badge)` with `view.corner_offset()` to put it on `view.on`.

## Status Label

What a thing is doing, as an icon and a word in one tone, with no fill and no edge: the state of a run in a row of a table, of a step in a page's header.

```rust
use lurq::ui::{StatusLabel, StatusLabelSize, StatusLabelVariant, icons};

StatusLabel::new("Completed").icon(icons::CHECK_CIRCLE).variant(StatusLabelVariant::Success);
// Its icon turns while what it says is under way.
StatusLabel::new("Running · 03 Implement").icon(icons::SPINNER).spin(true);
StatusLabel::new("Queued").icon(icons::HOURGLASS).variant(StatusLabelVariant::Muted).size(StatusLabelSize::Sm);
```

| Builder | |
| --- | --- |
| `StatusLabel::new(label)` | A label that says `label` |
| `icon(name)` | An icon of the theme's set before the words |
| `spin(bool)`, `spin_motion(SpinMotion)` | The icon turns as a spinner does, by the theme's [spin motion](#spin-motion) or its own |
| `variant(StatusLabelVariant)` | `Neutral` (`Foreground`, the default), `Muted` (`MutedForeground`), `Info`, `Success`, `Warning`, `Destructive`, or a tone the theme's recipe names |
| `size(StatusLabelSize)` | `Md` (an icon of 14, words of 12, 6 apart, the default) or `Sm` (12, 11, 4 apart) |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [badge](#badge) has them. `id` is the root's, with `{id}-icon` and `{id}-label` |

- A status label is read and not operated: no press, no focus, no stop of Tab. Its words are one line, cut with an ellipsis where its room is short; the icon keeps its size.
- Where a [badge](#badge) is a filled box, a status label has none: its words and its icon are in the tone's strong color, at medium weight.
- Parts: `StatusLabelPart::Root` (its color and the room between icon and words), `Icon` (its side), `Label`. States: `StatusLabelState::Spinning`.
- To what reads the tree it is a `status` called by its words. A renderer gets a `StatusLabelView` with `root()`, `icon()` (turning where it spins) and `words()`.

## Tag

A label on a thing, a keyword or a category, that the app may let a person take away.

```rust
use lurq::ui::{Tag, TagShape, TagSize, TagVariant, icons};

Tag::new("design");
Tag::new("in review").variant(TagVariant::Info).icon(icons::CIRCLE);

// With the button that takes it away: the app holds the list.
Tag::new("rust").on_remove(move || topics.update(|all| all.retain(|t| t != "rust")));

// Round, with no line: the token of a box that holds several values.
Tag::new("React").shape(TagShape::Pill).on_remove(move || remove("React"));

// A term and its value: the term before the label, as the app writes it.
Tag::new("main").term("branch:").icon(icons::BRANCH);
```

| Builder | |
| --- | --- |
| `variant(TagVariant)` | `Neutral` (the default), `Info`, `Success`, `Warning`, `Destructive`, or a variant the theme's recipe names |
| `size(TagSize)` | `Sm` (20 high), `Md` (24, the default), `Lg` (28) |
| `icon(name)` | An icon before the text, by its name in the theme's set |
| `term(&str)` | A term before the text, `branch:` before `main`, as the app writes it (its punctuation too), quieter than the text. The tag is still called by its text |
| `on_remove(handler)` | Gives the tag its remove button, and is called on a press of it |
| `on_remove_event(Fn(&TagRemoveEvent))` | `on_remove` given what took the tag away: `TagRemoveEvent::Press` (the button's press) or `Key` (Backspace or Delete on it). A tag in a row a press opens calls `event.stop_propagation()`, so the row hears nothing of it. Handlers of both are called in the order they were given |
| `shape(TagShape)` | `Default`, or `Pill`: round at both ends, with no line around it |
| `remove_label(&str)` | What the remove button is called to what reads the tree: "Remove" and the tag's text by default |
| `disabled(bool)` | Dimmed; its remove button is not pressed and Tab passes it by |
| `tab_stop(bool)` | `false`: Tab passes the remove button by, and a press of it takes the tag away and leaves the focus where it was; focused by the app, Enter, Space, Backspace and Delete press it. See [pointer-only pressables](#pointer-only-pressables) |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the root's; `force` draws the remove button |

- A tag does not keep whether it is there: the app holds its list, and `on_remove` tells it which one a person took away. A tag with no handler is read only: it has no button and is no stop of Tab.
- The remove button is a button of its own, called "Remove" and the tag's text ("Remove rust") to what reads the tree, or its `remove_label`; it has the tag's id with `-remove` after it. Tab reaches it, and a press, Enter, Space, Backspace and Delete press it; Backspace and Delete only while the focus is on it, with no modifier held.
- Tags an app lays in a row with a focus group (`Row::new().focus_group(FocusGroup::horizontal())`) keep the focus when one is taken away: it goes to the next tag's button, or to the last, so Backspace takes them one after another. Outside a group the focus leaves with the button.
- Parts: `TagPart::Root`, `Icon`, `Term` (in the label's type at regular weight, `MutedForeground` on a neutral tag and the variant's color on another), `Label`, `Remove` (the button), `RemoveIcon` (the cross: its side). States: `TagState::Remove` (it has the button), `Pill`, `Disabled`.
- In the design a tag is 24 high, padded 8, with the corners of `RadiusSize::Sm`, in `Secondary` with a line of `Border` inside it, its text 12 and medium in `SecondaryForeground`. A removable one is padded 4 at its end, where a button of 16 (corners of 3) holds a cross of 11 in `MutedForeground`, 4 after the text; under the pointer the button is filled `StateHover` and its cross is `Foreground`. Focused from the keyboard, the button has a ring of 2 in `Ring` around its edge, which stands clear of the text before it. `Info`, `Success`, `Warning` and `Destructive` are in the muted fill of their color, line and fill alike, with the text in the color. A pill is round, has no line, is padded 10 and 6, and its cross is 12.
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
| `ChipGroup::controlled(value, choices, on_change)` | The app holds the one chosen: a press of another tells `on_change` (`Fn(&T)`) the value it is to be, and a press of the chosen one tells nothing |
| `ChipGroup::controlled_multiple(values, choices, on_change)` | The app holds those chosen: a press tells `on_change` (`Fn(&[T])`) the whole set it is to be |
| `on_change(handler)` | Called as a person changes what is chosen, with the new value: `&T` for a group of one (a `ChipGroup<T>`), `&[T]` for one of any number (a `ChipGroup<T, [T]>`), checked by the compiler. Not called where the app sets the signal. Each one adds |
| `label(&str)` | What the group is called to what reads the tree, a `group`; a [field](#field)'s label where it has none |
| `invalid(bool)` | Each chip has a line in the destructive color; a field's judgment sets it. State: `ChipGroupState::Invalid` |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | `ChipGroupPart::Item` lays rules over each chip's root. `force` draws the chip Tab stops on |

- Tab reaches a chip, Space and Enter turn it, and the focus ring is the chip's. A group is one stop of Tab: the arrow keys move between its chips and Space or Enter turns the one the focus is on. A choice is a [`Choice`](#toggle-group), as a toggle group's: its icon is the chip's.
- To what reads the tree a chip is a `button` called by its text, with `a11y_pressed` saying whether it is chosen; `lurq_inspect` reports it as `state.pressed`.
- Parts: `ChipPart::Root` (what is pressed), `Icon`, `Label`; `ChipGroupPart::Root`, `Item`. States: `ChipState::Selected`, `Icon` (it has an icon before its text: the check or its own), `Disabled`.
- In the design a chip is 28 high and round, padded 12, its text 13 and medium. One not chosen is in `Background` with a line of `Input` and its text in `Foreground`, `Highlight` under the pointer; a chosen one is in `Primary`, line and fill, its text in `PrimaryForeground`, with a check of 14 before it, 6 from it and padded 8 at its start, and in `PrimaryHover` under the pointer. A disabled chip is dimmed. The chips of a group are 6 apart and go on to the next line where the row has no room.
- A renderer that draws another chip starts from `view.root()`, the behavior with the root's style; `view.shown_icon()` is the icon the default renderer puts before the text.

## Avatar

Who a person is, at a glance: the initials of the name on a fill, a picture in their place, whether the person is there.

```rust
use lurq::{components::Image, ui::{Avatar, AvatarPresence, AvatarShape, AvatarSize, icons}};

Avatar::new("Ivan Zaida");
Avatar::new("Ivan Zaida").image(Image::from_file("ivan.png")).presence(AvatarPresence::Online);
Avatar::new("Ivy Chen").size(AvatarSize::Xl);
// The app's letters; tooling still calls it "Ada King Lovelace".
Avatar::new("Ada King Lovelace").initials("AK");

// What is not a person: an icon in place of the initials, in a square.
Avatar::new("Build bot").icon(icons::TERMINAL).shape(AvatarShape::Square);
```

| Builder | |
| --- | --- |
| `new(&str)` | Who it is. The initials are the first letters of the first word and the last: "Ivan Zaida" is "IZ", "Ivan" is "I". A name with no letters shows the icon of a person |
| `initials(&str)` | The letters the face shows, in place of those taken from the name: the app's own way of making them. The name stays what tooling calls the avatar. Empty letters show the icon of a person |
| `image(Image)` | A picture in place of the initials, which fills the face, cropped to it (`cover`) and clipped to its shape: the engine's image element as the app builds it, from pixels (`Image::new`) or from a file or bytes it decodes (`Image::from_file`, `Image::from_bytes`, feature `image`), with a cache key and a size to decode to where it wants them. Feature `raster` |
| `icon(IconChoice)` | An icon of the theme's set in place of the initials, for what is not a person: a bot, a team. `None` is no icon, not even the person a name with no letters shows |
| `size(AvatarSize)` | `Xs` 20, `Sm` 24, `Md` 32 (the default), `Lg` 40, `Xl` 56, or a size the theme's recipe names |
| `shape(AvatarShape)` | `Circle` (the default), or `Square` with the corners of a control |
| `presence(AvatarPresence)` | The dot at the bottom end of the face: `Online`, `Away`, `Busy`, `Offline`; `None` for none |
| `presence_label(&str)` | What the dot says to tooling after the name, in place of the presence's own word: the app's word, or the word in another language |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the root's |

- While a picture decodes and when it fails (a broken file, a format the decoder does not read), the avatar shows its initials or its icon: they are the image's placeholder and its error view. A picture with clear pixels shows the face's fill through them.
- An avatar is read, not operated: it takes no press and is no stop of Tab. To what reads the tree (`lurq_snapshot` of the MCP server) it is an `img` called by its name, with its presence after it: "Ivan Zaida, away".
- Parts: `AvatarPart::Root` (the face and the dot, as large as the face), `Face` (the shape: side, corners, fill, the ring of a group; it clips what it holds), `Initials`, `Icon` (its width is the icon's side), `Image`, `Presence` (the dot). States: `AvatarState::Square`, and the avatar's `AvatarPresence` under its own name (`.state(AvatarPresence::Away, AvatarPart::Presence, ..)`). Sizes: `AvatarSize`.
- In the design an avatar is a circle of 32 in `Muted`, its initials 12 and medium in `Foreground`; at 20, 24, 40 and 56 they are 9, 10, 14 and 18. A square one has the corners of `RadiusSize::Md`. The icon is half the side, in `MutedForeground`. The dot is 10 on a face of 32 (6, 8, 12 and 14 on the others), at the face's bottom end corner, round, ringed 2 outside it in `Background`: `Success` online, `Warning` away, `Destructive` busy, `MutedForeground` offline.
- The ring of the dot stands outside the avatar's box, as the rings of a group do: what holds an avatar at its edge clips them, as a container clips what is in it, unless it leaves 2 of room or is `overflow_visible()`.
- The dot on its own, with no avatar under it, is `Presence::new(AvatarPresence::Online)`: before a word in a list of who is there, beside a name in a line of text. It is the `AvatarPart::Presence` of the avatar's recipe, in the color of its presence and at the side of the dot of an avatar of its `size(AvatarSize)` (10 unless told), so a theme that restyles an avatar's dot restyles it; it has no ring, with no face to part it from. `style` and `rules` name the avatar's parts, and `label(&str)` says what tooling calls it in place of the presence's word: it is an `img` called "online".
- A person with a name and a line under it (the board's Persona) is a [persona](#persona).
- A renderer that draws another avatar has `view.face()` (the shape in its style, clipped), `view.fallback()` (the initials or the icon), `view.presence_dot()` and `view.label()`; `view.image` is the picture the app gave, to put in the face.

## Avatar group

Avatars laid over one another: who is in a document, who reviews a change.

```rust
use lurq::{
  app::theme::PaletteColor,
  node::border::Border,
  ui::{Avatar, AvatarGroup, AvatarGroupPart, AvatarSize},
};

let people = ["Ivan Zaida", "Anna K.", "Mark R.", "Ivy Chen", "Ivo Kovac", "Lena Quinn"];
// Three, and "+3" for the three it leaves out.
AvatarGroup::new(people.map(Avatar::new)).max_shown(3).label("Reviewers");

// On a card, the ring is the card's color.
AvatarGroup::new(people.map(Avatar::new))
  .size(AvatarSize::Sm)
  .style(AvatarGroupPart::Item, |s| s.border(Border::outside(2.0, PaletteColor::Card)));
```

| Builder | |
| --- | --- |
| `new(avatars)` | The avatars, in order: each lies over the end of the one before it |
| `max_shown(usize)` | The most avatars it shows; after them a last one says how many it leaves out, "+3" |
| `size(AvatarGroupSize)` | The size of every avatar in it: an `AvatarSize` |
| `label(&str)` | What it is called to what reads the tree. Without one, the names it shows and how many more |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- The group gives its avatars its size and what its recipe says of an item; each keeps everything else it was given: its picture, its icon, its presence.
- To what reads the tree it is a `group` called by its label; the count is an `img` called "3 more".
- Parts: `AvatarGroupPart::Root` (the row: how far each avatar lies over the one before, as a gap below nothing), `Item` (laid over each avatar's `Face`: its ring), `More` (laid over the count's `Face`, after `Item`). Sizes: `AvatarSize`.
- In the design avatars of 32 lie 8 over one another, each ringed 2 outside its face in `Background`, the color of the surface behind them; the count is in `Secondary`, its text as the initials. Smaller and larger avatars lie a quarter of their side over each other.
- A renderer that draws another group has `view.avatars` (ready, with the group's size and rules), `view.hidden` and `view.more()`, the count as an avatar.

## Persona

A person in a list, a menu or a header: their avatar with the dot that says whether they are there, their name, and a line under it.

```rust
use lurq::ui::{Avatar, AvatarPresence, AvatarShape, Persona, PersonaSize, icons};

Persona::new("Ivan Zaida").description("Available · Kyiv").presence(AvatarPresence::Online);
Persona::new("Ivy Chen").description("Away · back at 15:00").presence(AvatarPresence::Away).size(PersonaSize::Lg);

// An avatar of the app's own: a picture, an icon, a shape.
Persona::new("Build bot")
  .description("Runs on every push")
  .avatar(Avatar::new("Build bot").icon(icons::TERMINAL).shape(AvatarShape::Square));
```

| Builder | |
| --- | --- |
| `new(&str)` | Who it is: the name beside the avatar, and the avatar's initials |
| `description(&str)` | The line under the name, in the muted color |
| `presence(AvatarPresence)` | The dot at the corner of the avatar; `None` leaves the avatar as it was given |
| `presence_label(&str)` | What the dot says to tooling in place of the presence's own word, as an [avatar](#avatar)'s |
| `avatar(Avatar)` | An avatar in place of the one the name gives, with a picture, an icon or a shape of its own. The persona gives it its size, and its presence where it has one |
| `size(PersonaSize)` | The avatar's: `Xs` 20, `Sm` 24, `Md` 32 (the default), `Lg` 40, `Xl` 56, with the text to match. `PersonaSize` is `AvatarSize` |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the root's |

- A name or a description longer than the room there is ends in an ellipsis, each on its one line; the avatar keeps its size, and the persona is no wider than what holds it.
- A persona is read, not operated: it takes no press and is no stop of Tab. To what reads the tree it is a `group` called by the name and described by the description, with its avatar, an `img`, in it.
- Parts: `PersonaPart::Root` (the avatar and the text, side by side), `Avatar` (what the persona says of its avatar's face, laid over the theme's recipe for it), `Text` (the name over the description), `Name`, `Description`. State: `PersonaState::Description` (it has the line). Sizes: `PersonaSize`.
- In the design a persona is an avatar of 32, 10 before a name in 14 and medium in `Foreground` with a line 1 under it in 12 of `MutedForeground`. The board draws that one size; the others follow the avatar's: at 20 and 24 the name is 12 and 13 over a line of 11, 6 and 8 from the avatar; at 40 and 56 it is 15 and 18 over a line of 13 and 14, 12 and 14 from it.
- The ring of the avatar's dot stands outside the persona's box: what holds a persona at its edge leaves 2 of room or is `overflow_visible()`, as with an [avatar](#avatar).
- A renderer that draws another persona has `view.root()`, `view.text()` and `view.avatar`, the avatar with the persona's size and presence, to build where it likes.

## Card

A surface that holds one thing: a group of settings, a summary of a project, a form of a few fields.

```rust
use lurq::{
  components::Text,
  layout::Alignment,
  node::padding::Padding,
  ui::{Button, ButtonVariant, Card, CardSize, CardVariant},
};

Card::new()
  .title("Appearance")
  .description("How lurq looks on this device.")
  .child(accent_field)
  .child(dark_mode_row)
  .footer(Button::new("Cancel").variant(ButtonVariant::Outline))
  .footer(Button::new("Save"));

// Pressed as a whole: one stop of Tab.
Card::new()
  .variant(CardVariant::Elevated)
  .size(CardSize::Sm)
  .title("lurq")
  .description("A Rust UI toolkit with GPU-backed rendering.")
  .on_click(move |_| open_project());

// Laid out as a column is, from its own builder.
Card::new()
  .padding(Padding::symmetric(16.0, 12.0))
  .gap(4.0)
  .align_items(Alignment::Center)
  .flex(1.0)
  .child(Text::new("1,284"))
  .child(Text::new("Runs this week"));
```

| Builder | |
| --- | --- |
| `title(&str)` | What the card is about, 16/600. It is what the card is called unless it has a label, and wraps in the width the card has |
| `label(&str)` | What the card is called to what reads the tree, in place of its title |
| `description(&str)` | More of it, under the title, 14 in the muted color |
| `action(element)` | At the end of the header, beside the title: a button, a menu, a badge. A card has one: a second call takes its place |
| `child(element)`, `children(..)` | Add to the body, under the header |
| `footer(element)` | Added to a row at the end of the card, under a line: its least action first and its main one last |
| `variant(..)` | `CardVariant::Outline` (the default), `Elevated`, `Filled`, or a variant the theme's recipe names |
| `size(..)` | `CardSize::Md` (the default) or `Sm`, or a size the theme's recipe names |
| `width(..)` | A width of its own. Without one the card is as wide as the room it is given |
| `height(..)`, `min_width(..)`, `max_width(..)`, `min_height(..)`, `max_height(..)` | A height of its own, and the least and the most it is wide and high: a number or another `Length` |
| `padding(..)` | The room inside it around its header and its body, in place of its size's: a `Padding`, a number, or a step of the theme's spacing. The footer keeps its own |
| `gap(..)` | The room between its children, and between its header and its body, in place of its size's |
| `align_items(Alignment)` | How its children are aligned across its width, as a column aligns them |
| `flex(f32)`, `flex_shrink(f32)` | Its share of the room left in a row or a column, and how much of its size it gives up where there is too little |
| `absolute(x, y, width, height)` | Where it stands in the `Stack` that holds it |
| `describe(name, value)` | What its surface says to tooling, beside its role and its name: the state of what it shows |
| `on_click(handler)` | Makes the card pressed as a whole, from the pointer and the keyboard |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the surface |

- Every part is optional: a card with children alone is a padded surface. The header is there where the card has a title, a description or an action; the footer where it has something in it, set off by a line only where there is content over it.
- A card is as wide as the room it is given, as a block is. Cards side by side in a row are each given a share of it: `flex(1.0)` on each, or a grid. The title and the description wrap in the room the action leaves them.
- A card given a height of its own (`height`, or its recipe's on `CardPart::Root`) gives what its header leaves to its body, so the body's children with `flex(1.0)` fill it (figure cards of one height in a row, each `height(118.0).flex(1.0)`), and its footer stays at the bottom. A card as high as what is in it, or given only a least height, gives nothing: a column that grows in lurq's layout takes all the room it may. A card that is a share of a column takes `height(Length::Fill)`. `view.has_height()` tells a renderer.
- What is in a card is clipped to its rounded corners, as what is in any element is to its own box: a table or a picture that reaches the card's edge does not paint over its corners. A control flush with the edge has its focus ring clipped there too, so content keeps its padding or is `overflow_visible()`.
- A card is laid out as a column is, from its own builder, though what it holds is two levels down: what lays out what is in it goes to its content and its body (`padding` to the content, `gap` to the content and the body, `align_items` to the body), and what places it in what holds it goes to its surface (`width`, `height` and their least and most, `flex`, `flex_shrink`, `absolute`, `describe`), on a read card and a pressed one alike. The padding, the gap and the sizes are rules of the card over the theme's recipe, so a renderer finds them in `view.part(..)`; the alignment, the flex, the place and what it describes are laid on the node that `view.body()`, `view.read_root()` and `view.pressed_root()` give.
- A card given `on_click` is pressed as a whole: one stop of Tab, with the focus ring around it, pressed by a click, Enter and Space. The controls in its action and its footer are stops of Tab of their own, after the card, and a press of one of them is that control's and not the card's; a control in its body is pressed with the card, so a pressed card keeps its controls in the action and the footer. A card without `on_click` is read: it is no stop of Tab, and each control in it is a stop of its own.
- Parts: `CardPart::Root` (the surface; what is pressed), `Content` (the header over the body, padded), `Header` (the text and the action), `Text` (the title over the description), `Title`, `Description`, `Action`, `Body` (what the app put in it), `Footer`. States: `CardState::Pressable`, `Disabled`, `Footer` (it has a footer under its content; the line over the footer is the content's `underline`).
- In the design a card is in `Card` with `CardForeground` ink and corners of `radius-xl`. The outlined card has an edge of 1 in `Border` and the small shadow, the elevated one the medium shadow and no edge, the filled one `Muted` and no edge. Its content is padded 24 with 24 between the header and the body; the title is 16/600, the description 6 under it in 14 of `MutedForeground`; what is in the body is 16 apart; the footer is padded 16 and 24 with its buttons 8 apart at its end. A small card is padded 16, with a title of 14 and a description of 13. A pressed card takes an edge of `Ring` and the medium shadow under the pointer, and the small shadow again while it is held down; a disabled one is dimmed.
- A surface of the app's own is a card with its parts styled: `style(CardPart::Content, |s| s.padding_x(14.0).padding_y(14.0))`, `style(CardPart::Root, |s| s.radius(RadiusSize::Lg).shadow(BoxShadowValue::none()))`, and what it holds as the body. The settings card and the stat card of the design are components of their own: a [settings card](#settings-card) and a [stat card](#stat-card).
- To what reads the tree (`lurq_snapshot` of the MCP server) a card is a `group` called by its label or its title, and a pressed card a `button`.
- A renderer that draws another card (`CardRenderer`) starts from `view.read_root()` or `view.pressed_root()`, the surface with its behavior, and puts in it what it likes: `view.content()` (the header over the body), `view.header()`, `view.text()`, `view.action()`, `view.body()` and `view.footer()` are the default ones.

## Expandable Tile

A tile that is pressed to show more of itself: a fact with its details, a figure with where it comes from, a plan with what it includes.

```rust
use lurq::{components::Text, core::Signal, ui::{ExpandableTile, icons}};

// The board's tile, open.
ExpandableTile::text("Renderer", "wgpu 26 on DirectX 12. Frame time 6.9 ms median.").open(true);

// Open while the signal is true, which a press of the tile turns.
let open = Signal::new(false);
ExpandableTile::text("Renderer", "wgpu 26 on DirectX 12.")
  .bind(open.clone())
  .on_open_change(|open| println!("open: {open}"));

// What it always shows, over an element it opens.
ExpandableTile::new("Team", Text::new("Unlimited seats, SSO, audit log."))
  .child(Text::new("$12 a seat"))
  .icon(icons::PLUS)
  .open_icon(icons::MINUS);
```

| Builder | |
| --- | --- |
| `ExpandableTile::new(title, element)` | Its title, and what it shows while it is open |
| `ExpandableTile::text(title, text)` | Its title, and a text it shows while it is open, in the detail's type |
| `child(..)`, `children(..)` | What it always shows, under its header: a figure, a line, a chart |
| `open(bool)` | Whether what it opens is shown |
| `bind(Signal<bool>)` | Open while the signal is true; a press turns it |
| `on_open_change(Fn(bool))` | Told of a press, with what the tile is to be: after the signal is set where it is bound, and not when the app sets it |
| `icon(..)`, `open_icon(..)` | The icon of the chevron (`chevron-down`), and of the open tile's (`chevron-up`, or its `icon` where it was given one); `None` for no chevron |
| `disabled(bool)` | It takes no input |
| `full_width(bool)` | As wide as what holds it, in place of the design's 260 |
| `label(&str)` | What it is called to tooling, in place of its title |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them |

- The whole tile is pressed: by the pointer anywhere on it, and by Enter and Space once Tab has reached it. It does not keep whether it is open: the app says so (`open`), or binds it (`bind`). With neither a handler nor a signal a press does nothing.
- A press of an element the app put in it (what `child` gave it, the element it opens) is that element's alone, and the controls in it are stops of Tab of their own, after the tile. A press of the text of `ExpandableTile::text` is the tile's.
- What it opens opens and closes with its height over the theme's base duration and fades, as a [collapsible](#collapsible)'s does, at once where motion is reduced. While it is closed it is not there: the controls in it are not stops of Tab.
- A long title goes on to the next line, and the chevron stays at the end of its first.
- To what reads the tree it is a `button`, called by its label or its title, which says whether what it opens is open (`a11y_expanded`).
- Parts: `ExpandableTilePart::Root` (what is pressed: the fill, the edge, the padding), `Header`, `Title`, `Chevron`, `Body` (what it always shows), `Panel` (what opens: its height), `Detail` (what the panel holds, and the type of its text). States: `ExpandableTileState::Open`, `Disabled`, `FullWidth`, `Body`.
- In the design (`slE1t`) it is 260 wide on `Background` in an edge of 1 in `Border` with the large radius, padded 16; its title is 14 at medium weight, 8 before a chevron of 16 in `MutedForeground`; what it opens is 10 under the header, a text of 13 on lines of 1.5 in `MutedForeground`. The board draws it open and at rest: closed it has `chevron-down`, under the pointer its edge is in the ring's color, as a pressed [card](#card)'s is, the focus is the theme's ring around it, and what it always shows is 10 under its header.
- A renderer that draws another tile has `view.root()`, `view.header()`, `view.title()`, `view.chevron()`, `view.body()`, `view.panel()` and `view.render()`.

## Scroll Area

Content that scrolls in a room of its own: a dialog's long body, a list in a panel, a table's cells. Its bar is the design's.

```rust
use lurq::ui::{ScrollArea, ScrollAreaOrientation, ScrollAreaScrollbars};

ScrollArea::new()
  .height(184.0)
  .label("Terms of use")
  .children(paragraphs);

// A list with its bar in a lane of its own beside the rows.
ScrollArea::new().width(340.0).height(160.0).gutter(true).children(rows);

// A table's cells, both ways, the bars only under the pointer and while they scroll.
ScrollArea::new()
  .orientation(ScrollAreaOrientation::Both)
  .scrollbars(ScrollAreaScrollbars::Hover)
  .child(cells);
```

| Builder | |
| --- | --- |
| `ScrollArea::new()`, `child(element)`, `children(..)` | What scrolls: one under another, or side by side in an area that scrolls across |
| `orientation(..)` | `ScrollAreaOrientation::Vertical` (the default), `Horizontal`, or `Both`; a `ui::Orientation` is one of the first two |
| `scrollbars(ScrollAreaScrollbars)` | When its bars show: `Auto` (while the content overflows), `Always`, `Hover` (under the pointer and while it scrolls, then they fade), `Scrolling` (only while it scrolls, as a system set to show scroll bars only when scrolling), `Never` |
| `gutter(bool)` | Its bars in a lane of their own beside the content, kept while a bar fades, and not over its edge |
| `fill_width(bool)` | What scrolls across is at least as wide as the area: items that grow share the room there is, and it scrolls sideways only once they need more (a [timeline](#timeline) across) |
| `label(&str)` | A region of that name to what reads the tree, and a stop of Tab whose keys scroll it |
| `scroll_state(ScrollState)` | The app's: where it is scrolled to, read and set |
| `width(..)`, `height(..)`, `max_height(..)` | Its size, `impl Into<Length>`; it is as large as the room it is given without one |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them; `force` draws its ring of focus |

- Its content scrolls with the wheel (smoothly, as every scroll container's), a finger, a drag of the bar's thumb and a press on its track. A bar is drawn over the content's edge unless the area has a `gutter`; content that runs under it, as a text does, is given a padding of the root at its end.
- A scroll area with a label is a stop of Tab with the ring of the focus inside it. With the focus on it, Up and Down scroll it by 40, Left and Right likewise across, PageUp, PageDown, Space and Shift+Space by what it shows, Home and End to its ends: the keys of any scroll container that has the focus itself. Give a label to an area whose content has nothing that takes the focus, so that the keyboard reaches it. One with no label is no stop: the controls in it are.
- To what reads the tree it is a `region` called by its label, and each bar of content that overflows is a scrollbar with its value: `scrollbar_vertical` and `scrollbar_horizontal`, how far it is scrolled from 0 at its start to 100 at its end.
- Its bar is the theme's [`ScrollBarStyle`](../styling-events/#scroll) (`theme.scrollbar()`), shown and placed as the area says: the scroll area adds no look of its own to the bar, and a theme that changes the bar changes it here and on every scroll container.
- In the design (`Scrollbar`) the bar is a lane of 8 with no fill, padded 1, and a thumb of 6 in it, round, in `Ring`. Under the pointer, and while it is dragged, the thumb is `MutedForeground`. The lane is 2 from the edge it runs along and from the ends of the area. The board has the bar beside a list's rows and in a dialog's padding; it draws no bar across, no corner where two meet, and no bar under the pointer or dragged, which are the engine's with the board's numbers: where both bars are there, each ends before the other's lane, and the corner is no bar's.
- Parts: `ScrollAreaPart::Root`. States: `ScrollAreaState::Vertical` and `Horizontal` (both on for an area that scrolls both ways), `Gutter`.
- A renderer that draws another area has `view.content()`, what scrolls laid out its way, `view.bar`, the theme's bar as the area shows it, and `view.render()`.

**Content that gives way first.** `ScrollHorizontal::fit_width(true)` lays content wider than the viewport out
in the viewport's width first, so that a line gives way there by its own rules (its children shrink, or drop out
of it), and scrolls only to what still has no room: a toolbar's controls go into its menu as it narrows, and the
bar scrolls to the ones that stay.

## Separator

A line between things: sections of a page, groups of a toolbar, the two ways to sign in.

```rust
use lurq::ui::Separator;

Column::new()
  .child(github_button)
  .child(Separator::horizontal().label("or"))
  .child(email_button);

// Down a row that centers what is in it: a length of its own.
Row::new()
  .align_items(Alignment::Center)
  .child(undo)
  .child(Separator::vertical().length(20.0))
  .child(bold);
```

| Builder | |
| --- | --- |
| `Separator::new()`, `horizontal()`, `vertical()` | A line across a column (`new` and `horizontal`), or down a row |
| `orientation(Orientation)` | Which way it runs: `Horizontal` (unless told) or `Vertical`. `SeparatorOrientation` is `ui::Orientation` |
| `label(&str)` | A word in the middle, with the line on either side of it |
| `length(f32)` | How long it is: its width across a column, its height down a row |
| `decorative(bool)` | Whether tooling passes it by. On by default |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- A label longer than the room is cut with an ellipsis on one line, and a line of 24 at least stays on either side of it.
- A separator across a column is as wide as the room it is given. One down a row is as high as the row where the row stretches its children (`align_items(Alignment::Stretch)`); in a row that does not, it is given a `length`. An inset is the root's padding: `style(SeparatorPart::Root, |s| s.padding_x(16.0))`.
- Parts: `SeparatorPart::Root` (the line, or the lines and the label), `Line`, `Label`. States: `SeparatorState::Horizontal` and `Vertical`, one of them on, which a recipe styles each way by.
- In the design the line is 1 in `Border`. A label is in the caption's type in `MutedForeground`, 8 from the line on either side.
- A separator is decoration to what reads the tree. One given `decorative(false)` is a `separator` with `a11y_orientation`, called by its label where it has one: for one between parts a person moves between.
- Tabs, the dialog and the menu draw lines of their own from their recipes.

## Spinner

Something is being done, and how far it is is not known: a ring that turns, three dots or five bars.

```rust
use lurq::ui::{LoadingOverlay, Spinner, SpinnerKind, SpinnerSize};

Spinner::new();
Spinner::new().size(SpinnerSize::Lg).kind(SpinnerKind::Dots);
// Beside the text that says what is being done.
Spinner::new().label("Loading results…");

// A veil with a spinner over a part of the page while it loads.
LoadingOverlay::new(loading.get(), panel);

// In the place of a whole page while it starts.
PageLoader::new("Starting lurq").description("Loading your workspace…");
```

| Builder | |
| --- | --- |
| `kind(SpinnerKind)` | `Ring` (the default), `Dots` or `Bars` |
| `size(name)` | `SpinnerSize::Sm` (16, the default), `Md` (24), `Lg` (32); the dots and the bars grow by a half and by twice |
| `variant(name)` | `SpinnerVariant::Current` (the color of the text around it, dots and bars too), `Muted`, `Primary` |
| `label(&str)` | What is being done, drawn 8 beside it, and what it is called |
| `label_hidden(bool)` | The label is its name and is not drawn |
| `decorative(bool)` | No status of its own, where what it is in says what is under way: a button that loads |
| `motion(SpinMotion)` | How the ring, the dots or the bars move, in place of the theme's: [in steps, and still where motion is reduced](#spin-motion) |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- To what reads the tree a spinner is a `status`, called by its label, or "Loading" when it has none.
- The ring is the glyph a [button](#button) turns while it loads, and `Button::loading` draws a `Spinner`: a rule a theme lays over the spinner reaches the button's. It turns with `ui::SPIN` once in `SPIN_PERIOD`. The dots fade one after another with `ui::SPINNER_DOTS`, the bars shrink about their middle one after another with `ui::SPINNER_BARS`; all of them play on the animation clock, so every spinner on screen moves as one. A spinner moves whether or not motion is reduced, unless its [motion](#spin-motion) says to stand still: one that stood still would say nothing.
- A spinner is drawn in the color of the text around it, so a spinner in a button is the color of its label: the ring as text is, and the dots and the bars by the fill `BackgroundColor::CurrentForeground`. Where nothing around it names a color, that is `Foreground`. A variant gives it a color of its own.
- Parts: `SpinnerPart::Root` (the spinner and its label), `Indicator` (the ring as a square of its side, or the row of the dots or of the bars), `Dot`, `Bar`, `Label`. States: `SpinnerState::Dots`, `Bars`.
- In the design the ring is 16, 24 or 32; the dots are 8, 6 apart, in a row 16 high, at 1, 0.6 and 0.3 of their color at rest; the bars are 3 wide and round, 3 apart, 8, 18, 12, 20 and 10 high in a row of 20; the label is 13 in `MutedForeground`.
- `LoadingOverlay::new(loading, content)` lays a veil over `content` while `loading` is true: the background's color at 85 hundredths, with a spinner in its middle (`.spinner(Spinner)` gives one of the app's). Meanwhile no press reaches the content and Tab passes it by. The veil fades in with `ui::FADE_IN` and out with `ui::FADE_OUT` over the theme's base duration. The overlay is as large as its content; its root clips the veil, so a rule that rounds `LoadingOverlayPart::Root` rounds the veil with a rounded content. Parts: `Root`, `Content`, `Veil`, and `Scrim`, whose background and opacity are the veil's fill. State: `LoadingOverlayState::Loading`.
- `PageLoader::new(title)` stands in the place of a whole page, a window or a pane while it starts: a ring of 28 over the title and, with `.description(&str)`, a line that says what is being done, in the middle of the room it is given. It is as large as that room (it fills its width, and its height where what holds it has one) and keeps 24 from its sides; a title or a description longer than the room wraps, centred. It paints no surface: it stands on the page's, and the board's box of 280 by 180 is the page it is drawn on. `.spinner(Spinner)` gives a spinner of the app's in place of the ring, as it is given. To what reads the tree it is one `status`, called by its title and described by its description; the spinner in it is no status of its own. Parts: `PageLoaderPart::Root`, `Spinner` (laid over the spinner's `Indicator`: the ring's side), `Text`, `Title` (14, medium, `Foreground`), `Description` (13, `MutedForeground`, 4 under the title); the ring is 14 over the text. State: `PageLoaderState::Description`. A renderer has `view.root()`, `view.spinner()` and `view.text()`.
- A renderer that draws another spinner starts from `view.indicator()` and `view.label()`; `view.name()` is what it is called.

## Skeleton

A placeholder in the shape of what is loading.

```rust
use lurq::{components::{Column, Row}, ui::{Skeleton, SkeletonAnimation}};

Skeleton::line();
Skeleton::circle(40.0);
Skeleton::block(200.0, 112.0);
Skeleton::paragraph(4).animation(SkeletonAnimation::Shimmer);

// A row of a list, in one status for tooling.
Skeleton::status(
  "Loading people",
  Row::new()
    .spacing(12.0)
    .child(Skeleton::circle(40.0))
    .child(Column::new().flex(1.0).spacing(8.0).child(Skeleton::line()).child(Skeleton::line().width(140.0))),
);
```

| Builder | |
| --- | --- |
| `Skeleton::line()` | A line of text: as wide as the room it has, 12 high |
| `Skeleton::circle(size)` | A face, an icon: round, `size` across |
| `Skeleton::block(width, height)` | An image, a chart, a card. `Length::Fill` is all the room it has |
| `Skeleton::paragraph(lines)` | Lines 10 apart; the one before the last a little shorter, the last shorter still |
| `width(length)`, `height(length)` | The app's size over the shape's. For a paragraph, the width of all of it and the height of each line |
| `min_width`, `max_width`, `min_height`, `max_height`, `radius(..)` | The rest of what an app lays on a bone: its least and most sizes and its corners, over the shape's. For a paragraph, the widths are of all of it, the heights and the radius of each line |
| `flex(factor)`, `flex_shrink(factor)` | Its share of the room left in its row or column, and how much of its size it gives up where there is too little, as an element's. lurq has no margins: a gap or a padding around it says where it stands |
| `animation(SkeletonAnimation)` | `Pulse` (the default), `Shimmer` or `None` |
| `motion(SkeletonMotion)` | How it pulses or shimmers, in place of the theme's (`theme.set_skeleton_motion(..)`): `SkeletonMotion::SMOOTH` (the default), or `SkeletonMotion::stepped(per_second)`, with `.period(duration)` |
| `Skeleton::status(label, content)` | A `status` called `label` around a group of skeletons |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- A skeleton is nothing to tooling that reads the tree: nothing of it is described, so it is left out as any box with no text, id or handler is. What is loading is said once, by `Skeleton::status` around a group of them.
- A skeleton pulses with `ui::SKELETON_PULSE`, from whole to half and back in `SKELETON_PULSE_PERIOD`; one that shimmers has a sheen as wide as itself cross it in `SKELETON_SHIMMER_PERIOD`, from wholly before its start to wholly past its end, whatever its width: in a row three times the skeleton's width centred on it, the room before the sheen grows from nothing to two thirds of the row (`ui::SKELETON_SHIMMER`, a width in percent). Both play on the animation clock, so every skeleton on screen pulses as one, and the sheen crosses all of them at once. Where motion is reduced a skeleton stands still, whatever its motion.
- A `SkeletonMotion` says how they move: smoothly, or in steps (`SkeletonMotion::stepped(20)`), each pose held until the next and no frame asked for between, so that a window with skeletons shown draws 20 frames a second and none more, all of them changing at the same moment; and how long a pulse, or a crossing of the sheen, takes (`.period(..)`, `SKELETON_PULSE_PERIOD` unless told). A theme gives the motion of every skeleton not told its own: `app.theme().set_skeleton_motion(SkeletonMotion::stepped(20).period(Duration::from_millis(1600)))`.
- Parts: `SkeletonPart::Root` (the box of a shape, or the column of a paragraph's lines), `Bone` (a shape), `Sheen` (the fill of its middle; it is as wide as the bone). A theme's renderer is given `SkeletonView` with its `motion`, and builds the root with `view.root()`, which carries the id, the key and the app's `flex`. The shapes are the variants of `SkeletonShape`: `Line`, `Circle`, `Block`, `Paragraph`.
- In the design every shape is in `Muted`; a line is 12 high and `radius-sm`, a circle `radius-full`, a block 200 by 112 and `radius-lg`; a paragraph's lines are 10 apart, its last two 92 and 60 hundredths of its width. The sheen runs from the shape's fill to `SecondaryHover` and back.
- The board's media row and card are built from these and are no components of their own: the gallery's Skeleton page has both, with their code.

## Progress

How far a piece of work is: a file uploading, items being copied. A bar along a rail, the thin bar across the top of a window, or a ring with the value in its middle.

```rust
use lurq::ui::{Progress, ProgressSize, ProgressVariant, RingProgress};

// A fraction of one.
Progress::new(0.62).label("Uploading theme.json").show_value(true).size(ProgressSize::Md);

// 42 of 128 items, said as the app says it.
Progress::new(42).max(128).show_value(true).value_text(|done| format!("{done} of 128"));

// Work whose end is not known.
Progress::indeterminate().label("Connecting");

// The thin bar across the top of a page, first in the page's column.
Progress::new(loaded.get()).top_bar(true);

// The same value around a circle, with it in the middle.
RingProgress::new(0.7).variant(ProgressVariant::Success);
```

| Builder | |
| --- | --- |
| `Progress::new(value)` | How far the work is: a fraction of one, or of the range `min` and `max` say. A value outside the range fills nothing or all of the rail; its text says what it is |
| `Progress::indeterminate()` | Work whose end is not known: a segment crosses the rail again and again |
| `value(n)`, `min(m)`, `max(m)` | How far the work is, and its range: none of it done (0 when not given) and the whole of it (1 when not given), as a [meter](#meter) says them. A range of nothing or less: nothing is done |
| `variant(ProgressVariant)` | The color of what is filled: `Neutral` (primary), `Info`, `Success`, `Warning`, `Destructive`, the tones' order and names as every component's |
| `size(ProgressSize)` | The rail's height: `Sm` 4, `Md` 6, `Lg` 8 (the default). A ring of 40, 56 or 72 |
| `label(&str)` | What the work is, over the start of the rail, and what the progress is called |
| `label_hidden(bool)` | The label is what the progress is called, and is not drawn |
| `show_value(bool)` | The value over the end of the rail, as a percentage or as `value_text` writes it. Off unless told, as the design's bar draws none; none while it is not known. A ring shows it unless told not to, as the design's ring does |
| `value_text(Fn(f64) -> String)` | Writes the value as the app does, given the value as it is: "42 of 128". Not given, its share of the range as a percentage |
| `description(&str)` | The line under the rail: what is left, "2 minutes left". It describes the progress to tooling |
| `top_bar(bool)` | The thin bar across the top of a window or a surface: 3 high, no rail, as wide as what holds it |
| `width(impl Into<Length>)` | How wide the progress is: 240 when not given |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the root |

- A label longer than the rail is cut with an ellipsis on one line, before the value, which is never cut.
- A progress is read, not operated: it takes no focus and no press. What is filled is a share of the rail, and moves to a new value over the theme's slow duration; while motion is reduced it is there at once.
- Work that is not known runs a segment of 90 of 240 across the rail, growing out of its start and leaving at its end, once in `ui::PROGRESS_PERIOD` (1.6 s), in phase with every other such bar. While motion is reduced the segment stands still 70 of 240 from the start of the rail, where no value would be, as the board draws it.
- Parts: `ProgressPart::Root` (the header over the rail), `Header` (the label and the value), `Label`, `Value`, `Track` (the rail), `Fill` (what is filled; the moving segment), `Description` (12 in `MutedForeground`, under the rail, as a meter's). States: `ProgressState::Indeterminate`, `TopBar`.
- In the design the rail is 240 by 8, round, in `Secondary`, filled in `Primary`, or `Success`, `Warning`, `Destructive`. Under a label and in a dialog it is 6, in a row of a list 4. The label is 8 over the rail in the label's type (13, medium) and `Foreground`; the value is at the end in the mono type of 11, `MutedForeground`. The top bar is 3 high with no rail, its fill square where it starts and round 2 where it ends.
- A ring (`RingProgress`) is 72 across, the ring a tenth of that wide, in `Secondary`, with what is done over it in the variant's color from the top and clockwise, and the value in the middle in the label's type of 13 at a weight of 600 (12 and 11 in the rings of 56 and 40). Nothing is painted in its middle, so it sits on any surface. It takes `value`, `min`, `max`, `variant`, `size`, `label` (its name to tooling: the board draws no text beside it), `show_value`, `value_text`, `state`, `style`, `rules`, `id` and `key`. It always knows the value: what does not is a spinner's.
- The ring is two round boxes one over the other: the track, whose inside border is the ring, and the fill, whose inside border, as wide and in the arc's color, is one dash as long as the arc with a gap of the rest of the circle. A border's dashes start at the top of a circle and run clockwise, and their period is fitted to the outline, so a dash and a gap that make the circle are one dash of it. The arc moves round to a new value, and fades in from nothing, over the theme's slow duration (`Transition::border_dash()`, `border_color()`); while motion is reduced it is there at once. Parts: `RingProgressPart::Root`, `Track` (its border is the ring: its width and the color of what is not done), `Fill` (its border's color is the arc's), `Middle` (holds the value, paints nothing), `Value`.
- To what reads the tree (`lurq_snapshot` of the MCP server) a progress and a ring are a `progressbar` called by their label and described by the description, with `a11y_valuenow`, `a11y_valuemin`, `a11y_valuemax` and `a11y_valuetext` (the value as it is written), the numbers written as a [meter](#meter)'s are; one that does not know how far the work is has no value.
- A renderer that draws another progress has `view.fraction()`, `view.value()`, `view.max()`, `view.value_text()`, `view.root()` (the root with its name to tooling), `view.header()` and `view.track()` (the rail with what is filled, or with the moving segment). `view.min()` and `view.description()` (the line under the rail) are there too. A share of something that is not work under way is a [meter](#meter)'s. A ring's renderer has `view.fraction()`, `view.value_text()`, `view.side()`, `view.thickness()`, `view.root()`, `view.track()`, `view.arc()` (the fill with its dash) and `view.middle()`. A value as half a ring is a [gauge](#gauge)'s.

## Meter

How much of a known range a value takes, in segments, and whether that is good: the space a disk has used, a battery's charge, the seats of a plan. Where a [progress bar](#progress) says how far a task has come, a meter says where a value stands.

```rust
use lurq::ui::{Meter, MeterVariant};

// The board's: good under 64, a warning to 96, bad past it.
Meter::new(96)
  .max(128)
  .low(64)
  .high(96)
  .label("Disk space")
  .value_text(|used| format!("{used} of 128 GB"))
  .description("Running low. Free up space soon.");

// A battery: the higher the better.
Meter::new(18).low(20).high(80).optimum(100).label("Battery");

// One tone, no parts.
Meter::new(12).max(20).segments(20).variant(MeterVariant::Info).label("Seats used");
```

| Builder | |
| --- | --- |
| `Meter::new(value)`, `value(..)` | The value, `impl Into<f64>` |
| `min(..)`, `max(..)` | The range, 0 to 100 unless given, as a [progress bar](#progress) and a [slider](#slider) say it |
| `low(..)`, `high(..)` | Where the low part of the range ends and the high part starts: each filled segment is in the tone of its part |
| `optimum(..)` | The best value, `min` unless given: the part it is in is good (`Success`), the part next to it a warning (`Warning`), the far one bad (`Destructive`), as a web page's meter judges them |
| `variant(impl Into<Name>)` | A `MeterVariant` (`Neutral`, `Info`, `Success`, `Warning`, `Destructive`), or a variant a theme adds: the tone of every filled segment of a meter with no `low` and no `high` |
| `segments(usize)` | How many segments the range is cut into, the board's 16 unless given |
| `label(&str)` | What it measures, over the segments: its name to tooling |
| `description(&str)` | The line under the segments: what the value means. In the tone of the value where that is a warning or bad |
| `value_text(Fn(f64) -> String)` | Writes the value as the app does. Not given, "96 of 128" |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- The value fills the segments it reaches, to the nearest: 96 of 128 fills 12 of 16. A value under `min` fills none and one over `max` fills them all; its text says what it is.
- A segment's tone is that of the part of the range its middle is in, so the board's 64 to 96 are the four warning segments 9 to 12.
- To what reads the tree it is a `meter` called by its label and described by its description, with `a11y_valuenow`, `a11y_valuemin`, `a11y_valuemax` and `a11y_valuetext`.
- Parts: `MeterPart::Root`, `Header`, `Label`, `Value`, `Segments`, `Segment`, `Description`. States: `MeterState::Filled` (of a segment). The variant a segment is resolved with is the tone of its part of the range; the root's is the tone of the value.
- In the design the meter is 260 on the board and as wide as what holds it here, its header, its segments and its description 8 apart. The label is 13 at 500 in `Foreground`, the value the mono type at 11 in `MutedForeground` at the end of its row. The 16 segments are 8 high, 2 apart, with a radius of 2: a filled one in the tone of its part, an empty one in `Secondary`. The description is 12, in `Warning` or `Destructive` where the value is, and in `MutedForeground` where it is good.
- A theme's renderer is given `MeterView`: `view.segments` (each a `MeterSegment` with its tone and whether it is filled, and `segment.render()`), the value, its range, its tone and its text, `view.header()`, `view.segments()`, `view.description()` and `view.root(content)`.

## Gauge

A value of a known range as the share of a half ring, its reading in it: the load of a processor, the speed of a link, the heat of a card. Its ring is drawn on the canvas, behind the `canvas` feature.

```rust
use lurq::ui::Gauge;

Gauge::new(70).label("CPU");

// A temperature: bad past 85, its reading as the app writes it.
Gauge::new(86).min(20).max(100).low(60).high(85).label("GPU").value_text(|degrees| format!("{degrees}°"));
```

| Builder | |
| --- | --- |
| `Gauge::new(value)`, `value(..)` | The value, `impl Into<f64>` |
| `min(..)`, `max(..)` | The range, from the ring's start to its end: 0 to 100 unless given, as a [meter](#meter) says it |
| `low(..)`, `high(..)`, `optimum(..)` | The parts of the range, as a [meter](#meter)'s: the value's share is in the tone of its part |
| `color(impl Into<BackgroundColor>)` | The color of the value's share of a gauge with no parts, a role of the palette or a color: `Chart1` unless given |
| `label(&str)` | What it measures, under the ring between its bounds: its name to tooling |
| `value_text(Fn(f64) -> String)` | Writes the reading. Not given, the value's share of the range in percent, "70%" |
| `bound_text(Fn(f64) -> String)` | Writes the bounds under the ring. Not given, as they are: "0", "100" |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- The half ring runs from the least value at its start, the left, to the greatest at its end, and the value takes its share from the start. A value outside the range takes none or all of it; its reading says what it is.
- The ring is painted with [`Canvas::paint`](../canvas/#a-drawing-that-follows-from-a-key), sharp at every scale; its reading, its label and its bounds are text, laid over it and under it. Its canvas holds a `value` [item](../canvas/#describing-what-you-drew) where the value's share ends, with the reading as its text.
- To what reads the tree it is a `meter` called by its label, with `a11y_valuenow`, `a11y_valuemin`, `a11y_valuemax` and its reading as `a11y_valuetext`.
- Parts: `GaugePart::Root`, `Arc` (the canvas: its width and height), `Track` and `Value` (drawn in their part's background, the track as thick as its part's width), `Reading`, `Range`, `Label`, `Bound`. The variant it is resolved with is the tone of the value where it has parts.
- In the design the gauge is 140 wide, a half ring 140 across and 72 high whose inner edge is 0.78 of its outer, so 15.4 thick: the track in `Secondary`, the value's share in `Chart1`, both with square ends. The reading is 20 at 600 in `Foreground` in the ring, its foot 10 over the ring's. Under the ring, 4 from it, the bounds at the ends, the mono type at 11 in `MutedForeground`, and the label between them, 12 at 500 in `Foreground`.
- A theme's renderer is given `GaugeView`: its value, range, share and reading, the bounds' texts, its label, `view.arc()`, `view.range()`, `view.root(content)` and `view.render()`.

## Password Input

An input whose text is masked, with an eye at its end that shows it and hides it again, and under it, where the app says so, a meter of how strong the password is.

```rust
use lurq::ui::{PasswordInput, PasswordStrength};

// password: Signal<String>, shown: Signal<bool>
PasswordInput::new(password.clone())
  .placeholder("Password")
  .bind_reveal(shown.clone());

// A password being chosen, judged by the app as it is typed.
PasswordInput::new(chosen.clone())
  .new_password(true)
  .bind_reveal(shown.clone())
  .strength(judge(&chosen.get()));
```

| Builder | |
| --- | --- |
| `PasswordInput::new(value)` | A password input bound to a `Signal<String>`: what is typed is set on it |
| `reveal(bool)`, `on_reveal(Fn(bool))` | Whether the password is shown as it is; called on a press of the eye with whether it is to be shown |
| `bind_reveal(Signal<bool>)` | Shown while the signal is true, which the eye turns |
| `reveal_button(bool)` | The eye at the end of the box: there unless the app says not |
| `show_label(&str)`, `hide_label(&str)` | What the eye is called to tooling, in place of "Show password" and "Hide password" |
| `mount(ctx)` | The input mounted where it is declared: with no `bind_reveal` and no `on_reveal`, it keeps whether the password is shown, and the eye turns it |
| `strength(Option<PasswordStrength>)` | The meter under the input: `Weak`, `Fair`, `Good`, `Strong` fill 1 to 4 of its bars |
| `strength_text(&str)` | What the meter says, in place of "Weak password" to "Strong password" (`PasswordStrength::text`) |
| `new_password(bool)` | The password is being chosen, not entered: the platform may offer a strong one |
| `input_rules(&recipe)` | Rules over the theme's recipe for the input it is made of |
| `placeholder`, `icon`, `size`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `full_width`, `on_input`, `name` | As an [input](#input) has them |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the control |

- While the password is masked the dots are 1 apart; the placeholder is set as any placeholder, with no spacing of its own.
- It is an [input](#input) in every other way: its box, its sizes, its judgments, a field's label. Whether the password is shown is kept by a `Signal<bool>` bound with `bind_reveal`, by the app with `reveal` and `on_reveal`, or, with neither, by the input mounted with `mount(ctx)`. The eye is drawn unless `reveal_button(false)`; converted into an element with neither a signal nor a handler, the input is its still form, and the eye turns nothing.
- Tab reaches the input and then the eye, which Space and Enter press. To tooling the eye is a `button` called "Show password" or "Hide password", with `a11y_pressed`.
- Masked, the password is not copied or cut, the platform's keyboard keeps it out of its suggestions, and tooling (`lurq_snapshot`, `lurq_set_value` of the MCP server) reads only its dots and `masked=true`. Shown, it is what the screen shows, to tooling too. Shown or not it holds a password to the platform (`ContentKind::Password`, or `NewPassword`), which may offer to fill it in.
- Parts: `PasswordInputPart::Root` (the input, the meter and its message, one under another), `Mask` (the room between the dots), `Toggle` (the eye), `Strength` (the meter), `Bar` (a bar the strength does not reach), `Fill` (one it does), `Description` (what the meter says). States: `PasswordInputState::Revealed`, `Disabled`, `FullWidth`, and the strength by its name (`weak`, `fair`, `good`, `strong`). The box, its text and its placeholder are the input's parts, styled by the theme's recipe for an input or `input_rules`.
- In the design the dots are 1 apart; the eye is 16 in `MutedForeground`, 12 from the end of the box and 8 after the text, in `Foreground` under the pointer; while the password is shown it is an eye with a line through it (`icons::EYE_OFF`), in `Foreground`. The meter is 8 under the box: four bars 4 high and 4 apart, round, in `Input` where the strength does not reach and in its color where it does, `Destructive` for weak, `Warning` for fair, `Success` for good and strong; the message is 8 under it in a caption's type in the same color.
- A renderer that draws another password input has `view.input` (the input's view: `render_with_end` is crate-private, so it lays the eye over `view.control()` itself), `view.control()` (the masked text input with room at its end), `view.toggle()`, `view.strength_meter()` and `view.description()`.

## Search Field

An input for a query: the search icon before the text, a button that empties it, and the hint of the key that reaches it.

```rust
use lurq::ui::SearchField;

// query: Signal<String>
SearchField::new(query.clone())
  .placeholder("Search components")
  .shortcut("/")
  .on_submit(|query| search(query));
```

| Builder | |
| --- | --- |
| `SearchField::new(value)` | A search field bound to a `Signal<String>`, which the app filters by as it changes |
| `shortcut(&str)` | The key that reaches the field, at its end while it is empty: `/`, `Ctrl K`. The app binds the key |
| `on_submit(Fn(&str))` | Called on Enter with what the field holds. The focus stays in the field |
| `ref_element(ElementRef)` | The ref of the field's text input: what the app focuses on the key the hint shows (`ctx.focus(&field)`) |
| `clear_label(&str)` | What the clear button is called to tooling, in place of "Clear search" |
| `clearable(bool)` | The clear button while it holds something, and Escape that empties it: on unless the app says not |
| `icon(impl Into<IconChoice>)` | The icon before the text in place of the search icon, or `None` for none |
| `input_rules(&recipe)` | Rules over the theme's recipe for the input it is made of |
| `placeholder`, `size`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `full_width`, `on_input`, `name` | As an [input](#input) has them |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the control |

- The hint of a key is never wider than 112: a longer one is cut with an ellipsis, and the icon and the text keep their room.
- While it holds something a clear button is at its end, which empties it and takes no focus, so the caret stays in it; Escape does the same while the focus is in it, and keeps the focus there. Escape in an empty field does what it does in any input. A read-only or disabled field has no clear button, and Escape does not empty it.
- The clear button is no stop of Tab: from the keyboard Escape does what it does. The hint shows while the field is empty; the field does not bind the key, which the app does and focuses the field on (`ctx.focus`).
- To tooling the field is a `searchbox` and its button a `button` called "Clear search".
- Parts: `SearchFieldPart::Root` and `Control` (what the field says of the input's root and box: its width, its padding), `Trailing` (what is at the end of the box), `Clear`, `Shortcut` (the hint). States: `SearchFieldState::Filled`, `FullWidth`. The box, its text, its placeholder and its icon are the input's parts.
- In the design the field is 280 by 36, padded 10 at its start and 8 at its end; the search icon is 16 in `MutedForeground`, 8 before the text. The clear button is a circled cross of 16 in `MutedForeground`, `Foreground` under the pointer. The hint is 20 high, padded 6, 4 round on `Muted` in an edge of 1 in `Border`, in the mono type of 11 in `MutedForeground`.
- The field renders again as it fills and empties, to show the clear button or the hint: it reads its signal as it is built.
- A renderer that draws another search field has `view.input`, `view.control()` (the text input with its keys and room at its end), `view.clear()` and `view.shortcut()`.

## Input Group

An input joined with what stands before and after it into one box: a scheme, a domain, a unit, an icon, the button that sends it.

```rust
use lurq::ui::{Button, ButtonVariant, Input, InputAddon, InputGroup, Select, icons};

InputGroup::new(Input::new(site.clone())).leading("https://").trailing(".com");

InputGroup::new(Input::new(email.clone()).placeholder("Email"))
  .leading(InputAddon::icon(icons::MAIL))
  .trailing(Button::new("Invite").variant(ButtonVariant::Ghost).on_click(invite));

InputGroup::new(Input::new(site.clone())).leading(Select::new(scheme.clone(), [("https", "https://"), ("http", "http://")]));
```

| Builder | |
| --- | --- |
| `InputGroup::new(input)` | A group around an [input](#input), which keeps its own value, handlers and id |
| `leading(addon)`, `trailing(addon)` | What stands before and after the input, in order: a text (`&str`), `InputAddon::icon(name)`, a `Button`, a `Select` |
| `label(&str)`, `read_only(bool)`, `name(..)`, `on_submit(..)`, `force(..)` | Said of the input in the group, as an input has them |
| `size(InputSize)` | The input's, and each button's and select's in the group |
| `invalid`, `warning`, `valid`, `disabled`, `full_width`, `on_input` | The input's judgment and the box's edge; nothing in a disabled group takes input; 280 wide or as wide as what holds it |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box |

- The corners of the box are the group's: what is at its ends is round where the box is, every other corner is square. A line is between two neighbours. The input has no edge or fill of its own: the box's edge says what it holds, and turns the ring's color under the pointer. The ring of the focus is the focused part's own, drawn over its neighbours and the box's edge.
- An icon beside the input is in its box: before it, the input's own icon; after it, over the end of the input. An icon away from the input stands on the muted fill as a text does.
- A button and a select keep what they do and their variant; the group gives them its size and its corners, and a select no edge or fill of its own. Each is its own stop of Tab.
- In a [field](#field) the input is called by the field's label and judged as the field is, and the group is disabled where the field is.
- Parts: `InputGroupPart::Root` (the box), `Field` (what the group says of the input's root), `Control` (of the input's box), `Addon` (a text or an icon on the muted fill), `AddonIcon`, `Separator` (the line), `Button` (of a button's root), `Select` (of a select's trigger). States: `InputGroupState::Disabled`, `FullWidth`, and the input's size and judgment by their names.
- In the design the box is 280 by 36, `Background` in an edge of 1 in `Input`, 6 round. A text addon is on `Muted`, padded 10, in 13 and `MutedForeground` (12 small, 14 large); the line between two parts is 1 in `Input`; the input is padded 10. A disabled group is half opaque on `Muted`.
- A renderer that draws another group has `view.input`, `view.before`, `view.after`, `view.addon(addon, index)`, `view.field()` and `view.separator()`.

## Number Input

A number a person types or steps: a quantity, a price, a size in pixels.

```rust
use lurq::ui::{NumberFormat, NumberInput, NumberInputLayout};

// A number there always is, between 1 and 99.
NumberInput::new(quantity.clone()).min(1).max(99).label("Quantity");

// A price with two decimals, its thousands set apart, the buttons stacked at the end.
NumberInput::new(price.clone())
  .min(0.0)
  .step(0.5)
  .precision(2)
  .format(NumberFormat::new().prefix("$").grouping(true))
  .layout(NumberInputLayout::Stacked);

// A number that may be left out: an emptied field is None, not 0.
NumberInput::optional(limit.clone()).placeholder("No limit");

// The app's own value, told back; and saved when Enter, the focus leaving or a step ends a change.
NumberInput::controlled(seats.get(), move |value| seats.set(value)).on_change_end(move |value| save(value));
```

| Builder | |
| --- | --- |
| `NumberInput::new(Signal<f64>)` | Bound to a number there always is: an emptied field takes back the number it had when it is left |
| `NumberInput::optional(Signal<Option<f64>>)` | Bound to a number that may be none: an emptied field sets `None` |
| `NumberInput::controlled(Option<f64>, on_change)` | The app holds the value: it shows it, tells `on_change` each change a person makes, and shows what the app answers with |
| `min(v)`, `max(v)` | The ends of the range, where it has them, any number (`impl Into<f64>`). With no least value a minus can be typed and Home moves the caret; with no greatest End does |
| `step(v)` | How far apart the values a step takes it to are, from the least (from 0 with none): 1 when not given |
| `precision(usize)` | How many decimals the value keeps and shows: as many as the step has when not given. With none no decimal separator is taken |
| `format(NumberFormat)` | How the number is written out of the focus: `prefix`, `suffix`, `grouping`, `separators(decimal, group)`, or `NumberFormat::custom(format, parse)`, the app's own |
| `layout(NumberInputLayout)` | `Split` (the board's: a minus at the start, a plus at the end, the number in the middle), `Stacked` (a chevron up over one down at the end, the number at the start), `Bare` (no buttons) |
| `size(NumberInputSize)` | `Sm` 32, `Md` 36, `Lg` 40 high, the type 13, 14, 15 |
| `placeholder(&str)` | What an empty input says |
| `invalid`, `warning`, `valid`, `read_only` | As an [input](#input) has them. A read-only one is focused and selected, and its keys and buttons change nothing |
| `on_change(Fn(Option<f64>))` | Called with the value each time a person changes it: as it is typed, stepped, put right or emptied. Each call adds |
| `on_change_end(Fn(Option<f64>))` | Called with the value when a change is over, where it is another than before: Enter or the focus leaving after typing, each step of a key, Home and End, and the release of a button held down, however many steps it took. Each call adds |
| `increase_label(&str)`, `decrease_label(&str)` | What the buttons are called to what reads the tree: "Increase" and "Decrease" when not given |
| `name(..)` | The name of its text in a form's data. A form's number is bound with `NumberInput::new(form.number("seats"))` |
| `label`, `full_width`, `disabled`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As an [input](#input) has them. `id` is the id of the box that is typed in |

- What is typed is kept as text while it is typed, and is the value as soon as it is a number. A minus (first, where the range has room below 0), digits and one decimal separator (where the precision keeps decimals) are taken; anything else is not. A number pasted as it is written (`$2,000`) is read and typed as the number. On Enter and when the focus leaves, the text is put right: the number rounded to the precision, kept in the range and written in the format; what is not a number goes back to the value. When the range changes under the value (a `max` set lower than it), the value is taken into the range at once, as a slider's is, in both forms: an input bound to a signal shows it, writes it and tells `on_change`; a controlled one, whose value the app holds, shows the clamped value, tells `on_change` once, as a press of a stepper would, and shows what the app answers with.
- In the focus the number is typed plain, with the decimal separator and nothing around it; out of it, it is written in the format. A locale's decimal separator is the format's: `NumberFormat::new().separators(',', '.')`.
- Up and Down take a step, Page Up and Page Down ten, Home and End go to the least and the greatest value where the range has them. A step from a value between two steps goes to the next one; from an empty field it goes to 0, or to the end of the range nearer to it. A wheel over the input scrolls what it is in and leaves the number as it is.
- The buttons take a step when pressed, and while held another after 400 ms and then every 50 ms. They are not stops of Tab and take no focus: a press leaves the focus on the number being typed. The one toward an end of the range the value is at is disabled and dimmed (states `AtMin`, `AtMax`).
- Parts: `NumberInputPart::Root` (its width), `Control` (the box that is typed in, which holds the ring of the focus), `Placeholder`, `Buttons` (what holds the buttons over the box: its inset and corners), `Decrease`, `Increase`, `Separator` (the line between a button and the number), `Icon`. Variants: `NumberInputLayout`. States: `NumberInputState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`, `AtMin`, `AtMax`.
- In the design the box is 132 by 36, an input's box (the `Background` fill in an edge of 1 in `Input`, 6 round), its number in the middle in the mono type at 14 in `Foreground`. The minus and the plus are 36 wide at the ends, each with a line of 1 in `Input` between it and the number, their glyphs 16; under the pointer a button is in `Highlight`. The ring's color under the pointer, a line of 2 inside the box in the focus, and the judgments' colors are the input's.
- The board's mobile quantity stepper is the number input restyled: rules make the `Control` 44 high, round, in `Secondary` with no edge, the number in the sans type at 16, semibold; the `Buttons` with no inset and round; `Decrease` and `Increase` 44 wide; the `Separator` 0 wide; the `Icon` 20. The gallery's page has them.
- To what reads the tree (`lurq_snapshot` of the MCP server) the box is a `spinbutton` called by its label, with `a11y_valuenow`, `a11y_valuetext` (as it is written), `a11y_valuemin` and `a11y_valuemax`; the buttons are `button`s called "Increase" and "Decrease" (or their labels), with `a11y_disabled` at an end of the range, their ids the input's with `-increase` and `-decrease` after it.
- A renderer that draws another number input has `view.control()` (the box with all of its typing, keys and putting right), `view.button(part, icon, steps)`, `view.separator()`, `view.buttons()` (laid over the box as the layout has them) and `view.buttons_reach()` (how far they reach in from each end).

## Input OTP

A one-time code: a few characters typed one to a cell, the code a message or an authenticator app gives to prove who a person is.

```rust
use lurq::ui::{Field, InputOtp, InputOtpKind};

Field::new(
  "Verification code",
  InputOtp::new(code.clone())
    .groups([3, 3])
    .on_complete(move |code| verify(code)),
)
.description("We sent a code to ivan@lurq.dev.");

// A recovery code of letters and digits, taken as capitals however it is typed.
InputOtp::new(recovery.clone())
  .length(8)
  .groups([4, 4])
  .kind(InputOtpKind::Alphanumeric)
  .uppercase(true);
```

| Builder | |
| --- | --- |
| `InputOtp::new(Signal<String>)` | The code. The cells show what the signal holds, and what is typed is set on it |
| `length(usize)` | How many cells it has: 6 unless it says |
| `groups(sizes)` | The sizes of the groups of cells, from the first: `[3, 3]` puts a dash after the third cell. Cells past the groups are a group of their own |
| `kind(InputOtpKind)` | `Digits` (the default) or `Alphanumeric`. Anything else typed or pasted is not taken |
| `uppercase(bool)` | A letter is taken as a capital |
| `mask(bool)` | A dot for each character in place of it, as a PIN is shown |
| `on_change(impl Fn(&str))` | Called with the code when a person changes it: a character typed, pasted or deleted. Not when the app sets the signal |
| `on_complete(handler)` | Called with the code when an edit fills its last cell. A second handler is called after the first, as every `on_*` is |
| `size(InputOtpSize)` | `Sm`, `Md`, `Lg`: cells of 32 by 36, 36 by 40, 40 by 44, with type of 14, 16, 18 |
| `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `name` | As an [input](#input) has them; `name` is the whole code's in its form |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the input under the cells |

- To a person it is one field. Tab puts the focus on the first empty cell. Typing fills the cell that has the focus and goes on to the next; typing on a filled cell replaces its character. Past the end of a full code what is typed goes nowhere, until a key or a click puts the focus back on a cell: the last cell is not typed over, and `on_complete` is called once as the code fills. Ctrl+A (Cmd+A) selects the whole code: Backspace or Delete then clears it, and a character typed starts it again. Backspace empties the cell that has the focus, or, on an empty one, goes back and empties the one before; Delete empties it and stays. Left and Right move, no further than the first empty cell; Home and End go to the first cell and to the first empty one. A click puts the focus on the cell clicked, or on the first empty one where that is before it.
- A code has no gaps: emptying a cell before the last filled one moves the cells after it up.
- It is a text input, which is always bound to a signal: it has no `controlled` form. `kind` says what the code is
  made of, not a tone: it is not a `variant`.
- A value the app sets is taken as typing takes it: what is not of the code's kind is dropped, letters are made capitals where it says, and no more than its cells are kept, and the signal is set to what is left.
- A paste of a whole code, or more, fills every cell from the first; a shorter one fills the cells from the one that has the focus. What the code does not take of it (spaces, dashes) is dropped, so `123-456` is `123456`.
- To a form, to tooling and to the platform it is one text input: under the cells is the engine's `TextInput`, bound to the code, which draws nothing of its own. It is the one stop of Tab, the one `textbox` to what reads the tree (`lurq_snapshot` of the MCP server), called by its label, with the code as its value, and the one value of its form under its `name`. It says it holds a `ContentKind::OneTimeCode`, so a phone offers the code from a message that just came, and asks for the digits keyboard where the code is of digits. A masked code is a masked input.
- The cell the focus is on, and whether the input has the focus, are kept by the code's signal for as long as its input is in the tree, since a builder keeps nothing between renders: two codes bound to one signal share them.
- Parts: `InputOtpPart::Root` (the row of cells and separators, and the room between them), `Cell`, `Value` (the character, or its dot), `Caret` (in the empty cell that has the focus), `Separator` (the dash between groups). States: `InputOtpState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `Masked`, and on a cell of its own `Active` (the focus is on it) and `Filled`.
- In the design a cell is 36 wide and 40 high, 6 from the next, in the background's fill with an edge of 1 in the `Input` color, `radius-md` round, its character in the mono type at 16 in `Foreground`. The cell the focus is on has a line of 2 inside it in the `Ring` color, and while it is empty a caret 1.5 wide and 18 high in `Foreground`, which blinks unless the theme reduces motion. The dash between groups is the `minus` glyph at 16 in `MutedForeground`, 6 from the cells on either side. A judgment colors every edge and the ring; a disabled code is half opaque on the muted fill, a read-only one on the muted fill in the `Border` color.
- The mobile skin of the board (Mobile/OTP Field) is a rule of the theme: `Root` 8 apart, `Cell` 46 by 56 and `radius-lg`, `Value` in the sans type at 22 and semibold, `Caret` 2 by 24.
- A renderer that draws other cells starts from `view.input()`, the behavior, and lays its own cells over it: `view.characters`, `view.shown(index)` (what a cell shows, its dot where masked), `view.active` (the cell the focus is on, while the code has it), `view.separators`, and `view.cell_part(index, part)` for the style of a part of one cell. `view.cell(index)`, `view.separator()` and `view.cells()` are the default ones. A cell built by `view.cell(index)` is one a click is known to be on.

## Swatch Picker

One color chosen among swatches in a grid: the color of a label, of a highlight, of an event in a calendar.

```rust
use lurq::{node::color::Color, ui::{Swatch, SwatchPicker}};

// Bound to the app's signal, which a choice sets.
SwatchPicker::new(color.clone(), ["#EF4444", "#F97316", "#F59E0B", "#22C55E", "#3B82F6", "#A855F7"])
  .columns(3)
  .label("Label color");

// The app's own values, each with its color: none chosen until a person chooses.
SwatchPicker::optional(label.clone(), [
  Swatch::of(Label::Bug, Color::from_hex("#EF4444")).label("Bug"),
  Swatch::of(Label::Docs, Color::from_hex("#3B82F6")).label("Docs"),
]);

// Held by the app, which is told what is chosen and shows what it answers with.
SwatchPicker::controlled(app.color(), swatches, |color: &Color| app.set_color(*color));
```

| Builder | |
| --- | --- |
| `SwatchPicker::new(value, swatches)` | Bound to a `Signal<T>`: the swatch of its value is the chosen one, and choosing another sets it. `T` is `Color`, or the app's value the swatches carry |
| `optional(Signal<Option<T>>, swatches)` | The same, holding none until a person chooses: Tab then enters at the first swatch |
| `controlled(value, swatches, on_change)` | The app holds the value: a choice is told to `on_change`, and the picker shows what the app answers with. It keeps nothing between renders |
| `on_change(handler)` | Told the value a person chooses (`\|color: &Color\|`, `&T` the compiler checks); not for the swatch chosen already, nor when the app sets the signal. A second handler is called after the first |
| `swatch(swatch)`, `swatches(swatches)` | More swatches after those given: a `Color`, a hex (`"#EF4444"`, which panics where it is no color), or a `Swatch` |
| `Swatch::new(color)`, `Swatch::of(value, color)` | A swatch whose value is its color, or the app's `value`; with `label(&str)` (its hex where it has none), `disabled(bool)`, `id`, `key` |
| `columns(usize)` | How many swatches a row holds: 6, the board's. Up and Down move the focus by as many |
| `size(ColorSwatchSize)` | The swatches: `Sm` 20, `Md` 24 (the board's), `Lg` 32 |
| `framed(bool)` | A panel of its own (the default), or the grid alone, for a popover or a picker that frames it |
| `label(&str)`, `invalid(bool)`, `disabled(bool)` | What it is called to what reads the tree; the panel's edge in `Destructive`; no swatch taking input, each dimmed |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the swatch Tab stops on |

- Exactly one swatch is chosen, ringed in `Foreground`. The grid is one stop of Tab, at the chosen swatch or the first that is not disabled; Left and Right move along a row and on to the next (a [focus group](../focus-navigation/#focus-groups), without going round), Up and Down a row in the same column, Home and End to the first and the last swatch, and a disabled swatch is passed by. A click, Enter or Space chooses; the arrows only move the focus.
- It is a control a [field](#field) holds: the field names it, judges it invalid and disables it.
- Each swatch is a [color swatch](#color-swatch)'s anatomy: its parts and states (`ColorSwatchPart`, `ColorSwatchState::Selected`, `Disabled`) are resolved for each, under the picker's rules for `SwatchPickerPart::Swatch`. A color that is not opaque stands over the swatch's checkerboard.
- Parts: `SwatchPickerPart::Root` (the panel, or the bare grid), `Row` (the room between swatches), `Swatch` (laid over each swatch's root). States: `SwatchPickerState::Framed`, `Invalid`, `Disabled`.
- In the design the panel is in `Popover` with an edge of 1 in `Border`, the large radius and the medium shadow, padded 10, its rows 6 apart; the swatches are 24, the small radius, 6 apart, each with an edge of 1 in `StatePressed` (the board's black at 8%), the chosen one ringed with 2 in `Foreground` outside it. The board's panel is 196 wide, 2 more than its six swatches and its padding take: the picker is as wide as its rows, 194. The board draws no focus, no disabled swatch and no invalid picker: the focus is the theme's ring on the swatch, a disabled swatch is dimmed, and an invalid picker's edge is in `Destructive`.
- To what reads the tree it is a `listbox` called by its label, of `option`s called by their labels (or their colors' hex), with `a11y_selected`, and `a11y_disabled` on one that cannot be chosen.
- A renderer that draws another picker has `view.swatches` (each a `SwatchItem` with its color, label, whether it is chosen or disabled, and its recipe), `view.entry()`, `view.root()` (the grid's focus group with its role) and `view.swatch(index)` (a swatch with its keys and what choosing it does).

## Checkbox Card

A checkbox whose whole card is pressed, with a title, a description and an icon or a price: the add-ons of a plan, the reports a team is sent.

```rust
use lurq::ui::{CheckboxCard, CheckboxCardGroup, Orientation, icons};

// Bound to the app's signal, which a press turns.
CheckboxCard::new("Analytics").description("Share anonymous usage data.").bind(analytics.clone());

// Cards over one list: those whose values the signal has are checked.
CheckboxCardGroup::new(add_ons, [
  ("backups", CheckboxCard::new("Backups").icon(icons::SHIELD).price("$4")),
  ("support", CheckboxCard::new("Priority support").icon(icons::MAIL).price("$20")),
])
.orientation(Orientation::Horizontal)
.on_change(|add_ons| println!("{add_ons:?}"));
```

| Builder | |
| --- | --- |
| `description(&str)` | More of it, under the title in the muted color; what the card is described by to tooling |
| `icon(name)`, `price(&str)` | An icon of 20 before the text; what the choice costs, at the end of the title |
| `checked(bool)` | Whether it is checked. The card does not keep it: the app says so |
| `bind(Signal<bool>)` | Checked while the signal is true, which a press turns: `checked` and a handler that sets the signal |
| `on_change(handler)` | Called on a press with what the card is to be: checked if it was not. Bound, it is called after the signal is set, and not when the app sets it |
| `invalid(bool)`, `full_width(bool)` | The edge in the destructive color; as wide as what holds it in place of 220 |
| `mark(element)` | An element of the app's in place of the icon, in the part `Icon`: a mark in its own colors |
| `content(element)` | An element under the description, in the part `Content`, 8 under it; a second stands 8 under the first. A press in it presses the card too, unless it stops its press |
| `disabled_reason(&str)` | Disabled for a reason: the reason under the description (part `Reason`) and to tooling (`a11y_disabled_reason`), a lock (part `Lock`) in place of the box or the mark |
| `loading(bool)` | What it chose is under way: no press, dimmed, its box or mark kept, `a11y_busy` to tooling (state `Loading`) |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the card |

| `CheckboxCardGroup<T>` | |
| --- | --- |
| `new(Signal<Vec<T>>, [(T, CheckboxCard)])` | The cards whose values the signal has are checked; a press adds a value or takes it out |
| `controlled(checked, [(T, CheckboxCard)], on_change)` | The app holds what is checked and is told the whole new set a press asks for (`&[T]`); what is drawn is what it says next |
| `on_change(handler)` | Called with the whole new set checked (`&[T]`) after a press, after the signal is set |
| `orientation(Orientation)` | `Vertical` unless told; `Horizontal`, side by side, 12 apart and as high as the highest, going on to the next line where there is no room |
| `label(&str)`, `invalid(bool)`, `disabled(bool)` | What the group is called to tooling; every card invalid or disabled |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the card Tab would stop on |

- The card is what is pressed: a click anywhere on it, or Space or Enter while it has the focus. It is the one stop of Tab and the ring of the focus is around it; the box in it is drawn and takes no focus of its own. In a group each card is its own stop.
- Parts: `ChoiceCardPart::Root` (what is pressed), `Icon`, `Text`, `Heading` (the title and the price), `Title`, `Price`, `Description`, `Content`, `Reason`, `Lock`. States: `ChoiceCardState::Checked` (a checkbox card), `Selected` (a radio card), `Invalid`, `Disabled`, `FullWidth`, `Loading`. The box is the checkbox's own `CheckboxPart::Box` and `Indicator`, resolved from the theme's checkbox recipe: a theme that restyles a checkbox restyles the card's box. A group's parts are `ChoiceCardGroupPart::Root` and `Item` (laid over each card's root), its states `ChoiceCardGroupState::Horizontal` and `Vertical`, one of which is on. A theme's renderer of either group gets a `ChoiceCardGroupView`: its cards built, in `view.cards`, `view.kind`, and `view.render()`; a theme names the groups without their type (`patch_recipe::<RadioCardGroup>`), for groups of every `T`.
- In the design the card is 220 wide, padded 16, in `Background` with the large radius; the text first, the title in 14 and medium over the description 4 under it in 13 of `MutedForeground`, and the box 12 after it at the top. Not checked its edge is 1 of `Input`; checked, 2 of `Primary`. Under the pointer the edge of a card not checked is `Ring`; an invalid card is edged in `Destructive`; a disabled one is dimmed.
- To what reads the tree (`lurq_snapshot` of the MCP server) the card is a `checkbox` called by its title, described by its description, with `a11y_checked` saying whether it is checked; a group is a `group`.
- In a [field](#field) a card is invalid where the field is and disabled where the field is, and keeps its title as its name; a group takes the field's label as its name unless it has one.
- A renderer that draws another card is given a `ChoiceCardView`: `view.root()` is the behavior with the root's style, `view.text()`, `view.icon()` and `view.control()` the default parts; `view.checked` says whether a checkbox card is checked. The same renderer type serves the radio card, whose `view.selected` says whether it is selected; `view.kind` says which it is.

## Radio Card

A radio whose whole card is pressed, with a title, a description and an icon or a price: the plan of an account, the density of a list.

```rust
use lurq::ui::{RadioCard, RadioCardGroup};

RadioCardGroup::new(density, [
  (Density::Comfortable, RadioCard::new("Comfortable").description("Roomy rows, 36px controls.")),
  (Density::Compact, RadioCard::new("Compact").description("Dense rows, 28px controls.")),
  (Density::Touch, RadioCard::new("Spacious").description("For touch.").disabled(true)),
]);

// One card alone: the app says whether it is selected and is told when it is pressed.
RadioCard::new("Team")
  .price("$12")
  .selected(plan.get() == Plan::Team)
  .on_change(move |_| plan.set(Plan::Team));
```

| Builder | |
| --- | --- |
| `description(&str)`, `icon(name)`, `price(&str)`, `mark(element)`, `content(element)`, `disabled_reason(&str)`, `loading(bool)` | As a [checkbox card](#checkbox-card) has them |
| `selected(bool)` | Whether it is the one of several selected: the dot in its mark. The card does not keep it |
| `on_change(handler)` | Called with `true` on a press while the card is not selected. A press of the one selected calls nothing |
| `invalid(bool)`, `full_width(bool)`, `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a checkbox card has them |

| `RadioCardGroup<T>` | |
| --- | --- |
| `new(signal, [(T, RadioCard)])` | The card whose value the signal has is selected; a press of another sets the signal: a `Signal<T>`, or a `Signal<Option<T>>` of which `None` selects none |
| `controlled(Option<T>, [(T, RadioCard)], on_change)` | The app holds the choice, none while it is `None`, and is told the value (`&T`) of each other card pressed; what is drawn is what it says next |
| `on_change(handler)` | Called with `&T`, the value of a card pressed that was not the one selected, after the signal is set |
| `orientation(Orientation)`, `label(&str)`, `invalid(bool)`, `disabled(bool)`, `state(..)`, `style(..)`, `rules(..)`, `force(..)`, `id`, `key` | As a checkbox card group has them |

- A group is one stop of Tab, at the selected card or the first. The arrow keys move the focus to the next card and select it, Home and End the first and the last, and a disabled card is passed by: the focus group of a [radio group](#radio), `FocusGroup::both().activate(true).one_stop(true)`.
- Parts and states are a checkbox card's (`ChoiceCardPart`, `ChoiceCardState`); its mark is the radio's own `RadioPart::Mark` and `Dot`, from the theme's radio recipe.
- In the design it is a checkbox card with the radio's mark of 16 in place of the box: not selected, an edge of 1 of `Input`; selected, 2 of `Primary`, with the dot of 8 in the mark.
- To what reads the tree the card is a `radio` called by its title, described by its description, with `a11y_checked`; a group is a `radiogroup`.

## Fieldset

Fields that go together, under a legend that says what they are: the profile of an account, the address of an order. As a group box, a few options of a desktop dialog under a small legend.

```rust
use lurq::ui::{Button, Checkbox, Field, Fieldset, FieldsetVariant, Input};

Fieldset::new("Profile")
  .description("Shown to people you share a workspace with.")
  .field(Field::new("Display name", Input::new(name).full_width(true)).required(true))
  .field(Field::new("Bio", Input::new(bio).full_width(true)).optional(true))
  .child(Button::new("Save profile"))
  .disabled(locked.get());

// A group box: the legend over a box of what it holds.
Fieldset::new("Startup")
  .variant(FieldsetVariant::GroupBox)
  .child(Checkbox::new("Open at login").bind(login))
  .child(Checkbox::new("Restore windows").bind(restore));
```

| Builder | |
| --- | --- |
| `variant(..)` | `FieldsetVariant::Boxed` (the default: the edge around the legend and the fields) or `GroupBox` (a small legend over a box of what it holds), or a variant the theme's recipe names |
| `description(&str)` | More of it, 4 under the legend, in the muted color |
| `field(Field)` | Adds a field, made as wide as the fieldset (`Field::full_width`) |
| `child(element)`, `children(..)` | Adds anything else under what is there: a row of buttons, a group of cards, an [error summary](#error-summary) |
| `disabled(bool)` | Every control in it takes no input and Tab passes it by; the whole fieldset is dimmed |
| `full_width(bool)` | As wide as what holds it, in place of 400 (250 as a group box) |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [field](#field) has them |

- What it holds is one under another, 16 apart, and stretched to its width: a field keeps a width of its own unless it is made full width, which `field` does.
- Parts: `FieldsetPart::Root`, `Legend` (the title over the description), `Title`, `Description`, `Body` (what it holds; a group box's edge is around it). States: `FieldsetState::Disabled`, `FullWidth`.
- In the design the fieldset is 400 wide, padded 20, with an edge of 1 of `Border` inside it and the large radius. The legend's title is 15 and semibold in `Foreground`, its description 13 in `MutedForeground`.
- The group box of the design (`Group Box`) is the `GroupBox` variant: 250 wide, the legend's title 12 and semibold in `MutedForeground`, and 6 under it a box with an edge of 1 of `Border` and the medium radius, padded 12, what it holds 10 apart. The board gives it no description: one is 12, 2 under the title. It names its group and disables it as the boxed fieldset does.
- A disabled fieldset disables its root, and an element that is disabled takes everything in it out of input, whatever it is: a control the app built itself too. The fieldset is dimmed as a whole, so a control in it is not dimmed again by its own disabled state.
- To what reads the tree it is a `group` called by its legend and described by its description.

## Error Summary

What is wrong in a form, at its top, a line for each field with a link to it: what a person reads when a form they sent was not accepted.

```rust
use lurq::ui::{ErrorSummary, FieldError};

ErrorSummary::new([
  FieldError::new("Display name", "Display name is required").focuses(&name_field),
  FieldError::new("Email", "Email must include a domain").focuses(&email_field),
]);

// From a form's own errors, once a submit of it was attempted (the `form` feature).
ErrorSummary::form(&form, [("name", "Display name", &name_field), ("email", "Email", &email_field)]);
```

| Builder | |
| --- | --- |
| `new(errors)` | A line for each `FieldError`, in the order given. With none the summary draws nothing |
| `FieldError::new(label, error)` | The field's label and what is wrong with it, as its field says it. The line shows the error. `id` and `key` are the line's |
| `error(FieldError)`, `errors(..)` | Adds lines after those it has |
| `FieldError::focuses(&ElementRef)` | A press of the line, or Enter or Space on it, focuses the first control under the ref |
| `form(&FormHandle, [(name, label, &ElementRef)])` | The form's errors once a submit was attempted: a line for each listed field that has one, in their order, then one for each other field with an error, called by its name and focusing nothing. It reads them as signals: what renders it renders again when they change |
| `title(&str)` | The title in the app's own words, in place of "2 fields need attention" |
| `full_width(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a fieldset has them; `force` draws every line as under the pointer or focused |

- A line that focuses a field is a stop of Tab, underlined under the pointer, with the ring of the focus around it. One that focuses nothing is read and not pressed.
- Parts: `ErrorSummaryPart::Root`, `Icon`, `Text` (the title over the lines), `Title`, `Item` (a line), `Bullet`, `Error`. State: `ErrorSummaryState::FullWidth`.
- In the design the summary is 400 wide, padded 12 and 14, in `DestructiveMuted` with an edge of 1 of `Destructive` inside it and the medium radius. The alert icon of 16 in `Destructive` is 10 before the body; the title is 13 and semibold in `Destructive`, and the lines are 6 apart under it, a dot of 4 in `Destructive` 8 before each message in 13 of `Foreground`.
- To what reads the tree it is an `alert` called by its title; each line is a `link` called by the field's label and its error.
- A line focuses its field through `.focuses(&ElementRef)`, which a button has: a click of it or Enter or Space on it focuses the target. A ref on a column around a `Field` will do: the control is the first thing under it that takes the focus.

## Combobox

A select a person types in, whose list what is typed filters: a framework, a country, an assignee among many.

```rust
use lurq::{components::SelectOption, ui::Combobox};

Combobox::new(framework.clone(), [
  ("lurq".to_owned(), "lurq"),
  ("gpui".to_owned(), "GPUI"),
  ("slint".to_owned(), "Slint"),
])
.placeholder("Select framework");

// Options an app fetches as a person types: it keeps all it was given.
Combobox::new(country.clone(), fetched.get().iter().map(|name| (name.clone(), name.as_str())).collect::<Vec<_>>())
  .on_query(move |typed| fetched.set(search(typed)))
  .filter(|_, _| true)
  .loading(searching.get());

// A value that is no option's: what is typed becomes the value of a combobox of String.
Combobox::new(label.clone(), [SelectOption::new("bug".to_owned(), "bug")]).allow_custom(true);

// One that may hold none, with a cross that takes it away, and Enter that submits what is typed.
Combobox::optional(assignee.clone(), people)
  .clearable(true)
  .on_submit(|typed| search(typed));
```

| Builder | |
| --- | --- |
| `Combobox::new(value, options)` | Bound to a `Signal<T>`: the option with its value is chosen, and choosing another sets it. The options are a [select](#select)'s: `(value, label)` or `SelectOption`s with a detail, a hint, elements before and after the label, a group they start, or disabled |
| `optional(Signal<Option<T>>, options)`, `controlled(value, options, on_change)` | As a [select](#select)'s: a value that may be none; a value the app holds, whose answer the box shows |
| `on_change(Fn(&T))` | Called with the new value when a person chooses an option, keeps a value of their own (`allow_custom`) or clears it: not when the app sets the signal, and not for what is chosen already. Its handler names the type of the value, as a select's does |
| `on_submit(Fn(&str))` | Called on Enter with what the box holds, where Enter chooses nothing: the list is closed, or nothing in it is highlighted |
| `clearable(bool)` | A cross at the end of the box, while something is chosen or typed, that takes the value and what is typed away: `None` of an optional combobox, the empty text of a combobox of `String`. One of a value that cannot be none has no cross. `clear_label(&str)` is what tooling calls it, "Clear" |
| `read_only(bool)` | What is chosen is read and not changed: the list does not open, what is typed is not taken, and there is no cross |
| `placeholder(&str)` | What the box says while nothing is typed |
| `filter(Fn(&str, &str) -> bool)` | Which options are shown while something is typed, given the label and what is typed. By default the labels that contain it, without regard to case |
| `on_query(Fn(&str))` | Called with what the box will hold at each edit: for an app that fetches its options |
| `loading(bool)` | The list ends in a row with a spinner, "Loading results…" (`loading_text(&str)` says otherwise) |
| `empty_text(&str)` | What the list says while what is typed leaves no option: "No results" |
| `allow_custom(bool)` | What is typed may be no option's: typing highlights nothing, and Enter or leaving keeps it. A label of an option, in any case, chooses the option; anything else becomes the value of a combobox of `String` |
| `query(Signal<String>)` | Where the app keeps what is typed, to read or set it. Without one the combobox keeps it while it is in the tree, one for each signal of a value |
| `size(ComboboxSize)` | `Sm` 32, `Md` 36 (the default), `Lg` 40 high, with text of 13, 14, 15 |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | What is chosen is wrong, doubtful or right: the edge and the ring in the destructive, warning or success color. Of the three, the first that is set is shown |
| `disabled(bool)`, `full_width(bool)`, `label(&str)`, `name(..)` | As a [select](#select) has them. `name` is the name of the value in a form: the chosen option's label |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box; its input has the id with `-input` after it |

- The box is a select's with a text input in it, which keeps the focus the whole time. A press anywhere on the box focuses the input and opens the list; typing opens it too and filters it, with the first option that can be chosen highlighted. Down and Up move the highlight, which a row shows as it shows the pointer; Enter chooses; Escape closes the list and a second Escape clears what was typed; Tab chooses nothing, closes the list and goes on.
- Choosing puts the option's label in the box. While the box shows the chosen label the list opens whole; what is typed otherwise filters it. When the focus leaves, what was typed goes back to the chosen label, unless custom values are allowed. A group keeps its label over the first of its options that what is typed leaves.
- It is a control a [field](#field) holds (`FieldControl`): the field names it, judges it and disables it.
- Parts: `ComboboxPart::Trigger` (the box), `Value` (what is typed), `Placeholder`, `Chevron`, `Panel` (the panel of the list), `Item`, `Check`, `Detail`, `Hint`, `Group`, `Separator` (a select's, by the same names), `Status` (the row of no results or of loading), `Spinner`, and `Clear` and `ClearIcon` (the cross of `clearable`). States: `ComboboxState::Open`, `Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`; an item's own `Selected` and `Disabled` are resolved on its parts.
- In the design the box is a select's: 260 by 36, `Background` in an edge of 1 in `Input`, 6 round, its text 12 in, the placeholder in `MutedForeground`, two chevrons of 16 at its end; the ring's color under the pointer and a line of 2 inside it while the focus is in it or the list is open. The list is a select's: 4 under the box on `Popover`, 8 round, items of 32 on `Highlight` under the pointer or the keys, the chosen one with a check of 16 at its end. The status row is 32 high, 8 in, in `MutedForeground` at 13, its spinner 16 in `Foreground`.
- Its options and the row that says there are none or that more are loading follow the [density](../theme/#density) as a select's do; the box is a control and keeps its size.
- It is the engine's `Select` in its combobox mode (`Select::combobox`): one implementation of the list, its rows, groups, checks and adornments with the select's. To what reads the tree (`lurq_snapshot` of the MCP server) the box is a `combobox` with `expanded` while its list is open, the input in it a `textbox` called by the combobox's label, and the list a `listbox` of `option`s with `a11y_selected`, which says in `a11y_activedescendant` which option the keys are on.
- A renderer that draws another combobox has `view.select_style()`, `view.input()` (the text input, with what leaving it and Enter do), `view.clear()`, `view.chevron()`, `view.status()`, `view.query()`, `view.shown()` (the labels the list shows) and `view.select(style, trigger)`, which builds the combobox and calls `trigger` with a `SelectTriggerState` whose `input` is the input to place.

## Multi Select

Several values chosen from a list that what is typed filters, held as tags in the box: the labels of an issue, the people a review goes to.

```rust
use lurq::ui::MultiSelect;

MultiSelect::new(languages.clone(), [
  ("rust".to_owned(), "Rust"),
  ("wgsl".to_owned(), "WGSL"),
  ("typescript".to_owned(), "TypeScript"),
  ("swift".to_owned(), "Swift"),
])
.placeholder("Select languages")
.max_shown(3)
.clearable(true)
.on_change(|languages: &[String]| save(languages));
```

| Builder | |
| --- | --- |
| `MultiSelect::new(value, options)` | Bound to a `Signal<Vec<T>>`: the options with its values are chosen, shown as tags in the order they were chosen; choosing one adds its value, or takes it out. The options are a select's |
| `controlled(values, options, on_change)` | The app holds the values: a change is told to `on_change` with the whole new set, and the box shows what the app answers with. A multi select is the `multiple` of a select by what it is, so these are its own `new` and `controlled`, where a component that holds one value or any number has `multiple` and `controlled_multiple` |
| `on_change(Fn(&[T]))` | Called with the whole new set when a person chooses an option, takes one back or away, or clears them: not when the app sets the signal. Its handler names the type of the values, `on_change(\|languages: &[String]\| ..)` |
| `read_only(bool)` | What is chosen is read and not changed: the list does not open, what is typed is not taken, the tags have no cross and there is no clear control |
| `placeholder(&str)` | What the box says while nothing is chosen and nothing is typed |
| `max_shown(usize)` | How many tags the box shows: the rest are one tag that counts them, "+3" |
| `clearable(bool)` | A cross at the end of the box, while something is chosen, that takes every value away. `clear_label(&str)` is what tooling calls it, "Clear" |
| `filter(..)`, `on_query(..)`, `loading(bool)`, `loading_text(&str)`, `empty_text(&str)`, `query(..)` | As a [combobox](#combobox) has them |
| `size(MultiSelectSize)` | `Sm` 32, `Md` 36, `Lg` 40 high at least, with tags of 20, 24, 28 |
| `invalid`, `warning`, `valid`, `disabled`, `full_width`, `label`, `name`, `state`, `style`, `rules`, `force`, `id`, `key` | As a combobox has them. A disabled one keeps its tags and takes none away; `full_width` is in place of 280 |

- The box holds the chosen values as tags ([tag](#tag)s of `TagShape::Pill`), and a text input after them, which keeps the focus while the list is open. The tags go on to the next line where they do not fit, and the box grows with them.
- Typing opens the list and filters it; Down and Up move the highlight; Enter or a press chooses an option or takes it back, and the list stays open while one chooses; a choice clears what was typed, and the list shows every option again. A tag's cross takes its value away, and so does Backspace in the empty input, the last one's. Escape closes the list and a second Escape clears what was typed; Tab goes on, and what was typed is cleared when the focus leaves. A press of a tag or of the clear control does not open the list.
- The input is the one stop of Tab, as a [token input](#token-input)'s is: the tags are reached from it. With nothing typed, Left goes onto the last tag, drawn as its cross is when focused, and on to the ones before; Right goes back towards the input; Backspace or Delete takes away the tag the arrows are on, and they stay on the tag that takes its place. Typing leaves the tags.
- It is a control a [field](#field) holds.
- Parts: a combobox's, by the same names, and `MultiSelectPart::Check` (the box before a chosen option), `CheckIcon`, `Unchecked` (the box before one that is not), `Tags` (the lines of the tags and the input: its gap), `Tag` and `TagLabel` (what is laid over a tag's root and label), `Clear` and `ClearIcon`. Its states and sizes are a combobox's: `MultiSelectState` is `ComboboxState`, `MultiSelectSize` is `ComboboxSize`.
- In the design the box is a select's, 280 wide and at least 36 high, padded 6 at its start, 10 at its end and 6 over and under, a chevron down of 16 at its end. A tag is 24 high on `Secondary`, 4 round, padded 8 before its text of 13 in `SecondaryForeground` and 6 after its cross of 12 in `MutedForeground`; the tags are 6 apart, across and down, and what is typed keeps 48 on their line. The list is a select's, with a box of 16 before each option, 8 before its label, 4 round: `Background` in an edge of `Input`, and, chosen, `Primary` with a check of 12 in `PrimaryForeground`.
- Its options follow the [density](../theme/#density) as a select's do.
- It is the engine's multiple `Select` in its combobox mode. To what reads the tree it is a `combobox` with `expanded`, its list a `listbox` of `option`s with `a11y_selected`, and each tag's cross a `button` called "Remove" and the tag's text; the clear control is a `button` called "Clear", or its `clear_label`.
- A renderer that draws another multi select has `view.select_style()`, `view.input()`, `view.tags()`, `view.tag_rules()`, `view.clear()`, `view.chevron()`, `view.status()`, `view.chosen()`, `view.shown()`, `view.query()` and `view.select(style, trigger)`.

## Cascader

A select whose options are a tree shown as columns that open one after the other, as a menu of submenus does: a place by its continent, its country and its city, a category of a catalogue, a setting by its section.

```rust
use lurq::ui::{Cascader, TreeItem};

// city: Signal<Option<&str>>: one or none, of the cities alone.
Cascader::optional(city, [
  TreeItem::new("asia", "Asia").items([TreeItem::new("japan", "Japan").item(TreeItem::new("tokyo", "Tokyo"))]),
  TreeItem::new("europe", "Europe").items([
    TreeItem::new("ukraine", "Ukraine").items([TreeItem::new("kyiv", "Kyiv"), TreeItem::new("lviv", "Lviv")]),
    TreeItem::new("poland", "Poland").item(TreeItem::new("warsaw", "Warsaw")),
  ]),
])
.label("Place")
.placeholder("Select a city")
.clearable(true)
.mount(ctx);

// A continent or a country may be chosen too, and the columns open under the pointer.
Cascader::optional(region, items).leaves_only(false).open_on_hover(true).mount(ctx);

// Held by the app, which is told each choice.
Cascader::controlled(app.city(), items, |city| app.set_city(*city));
```

| Builder | |
| --- | --- |
| `new(value, items)`, `optional(value, items)` | The tree's items (`TreeItem<T>`, a [tree view](#tree-view)'s) and what is chosen of them, bound to a `Signal<T>` or a `Signal<Option<T>>`. A value is one item's: the trigger says the path down to it |
| `controlled(value, items, on_change)`, `controlled_optional(..)` | The same held by the app, which is told `on_change(&T)` or `on_change(&Option<T>)` and shows what it gives back |
| `on_change(handler)` | Told what is chosen after a choice a person makes, after the signal is set; not for the option chosen already, nor when the app sets the signal. The cascader is `Cascader<T, T>` or `Cascader<T, Option<T>>`, so a handler of another type does not compile |
| `leaves_only(bool)` | Whether only an option that holds nothing is chosen, as the board's cascader has it (it is unless told otherwise). Without it a press or Enter chooses any option, and a choice of one that holds others leaves the panel open on its column |
| `open_on_hover(bool)` | The pointer on an option that holds others opens its column, as a submenu opens, in place of a press. A press opens it either way |
| `on_load(handler)` | Told an item marked `lazy` that holds nothing yet as its column opens: where the app brings its children. The column has a spinner until they come |
| `open(Signal<bool>)` | Whether the panel is open, where the app keeps it. `on_open_change(bool)` hears each change a person makes, a choice too; `on_dismiss()` Escape and a press outside, before it |
| `path(bool)`, `separator_text(&str)` | The trigger says the whole path to what is chosen, `Europe / Ukraine / Kyiv` (it does unless told otherwise), or its label alone; and what stands between the steps, `" / "` |
| `placeholder(&str)`, `size(CascaderSize)`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `full_width`, `name` | As a [select](#select) has them. A read only one and a disabled one do not open |
| `clearable(bool)`, `clear_label(&str)` | A cross before the chevron, while something is chosen, that takes it away, of a cascader that may hold none; and what it is called, "Clear" |
| `empty_text(&str)`, `loading_label(&str)` | What the panel says where there are no items, "No options"; what the spinner of a column on its way is called, "Loading" |
| `locale(&Locale)` | The words it speaks in, the name of its cross, what an empty panel says and what a spinner is called, in place of the app's (`App::set_locale(Locale::for_tag("uk"))`, English where it set none), whose words are `Locale::input`. A word told by itself (`clear_label`, `empty_text`, `loading_label`) stays; the mark between the steps is no word |
| `label(&str)`, `panel_label(&str)` | What the cascader is called to what reads the tree, and its panel and its columns in place of that |
| `placement`, `offset`, `collision`, `inline`, `dismiss_on_escape`, `dismiss_on_outside_press` | As a [popover](#popover) has them: the panel is one |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the trigger's; the panel has it with `-panel`, the columns with `-columns` (each column that, `-column-` and its place), the cross with `-clear` |
| `mount(ctx)` | In place of converting it: the cascader keeps whether it is open, the trigger the focus goes back to and the columns open in it |

- It is built of what is there already: its trigger is a [select](#select)'s box (the same recipe, sizes, ring, judgments and fluid form in a [field](#field)), the one a [tree select](#tree-select) has, its panel a [popover](#popover) and what the panel holds a [column view](#column-view), with a column view's columns, options, chevrons and keys. What a `TreeItem` has, it has here: `icon`, `disabled`, `lazy`, `trailing`. A tree select is its sibling for a tree shown in one column, of which any number may be chosen.
- A press, Enter, Space, Down or Up on the trigger opens the panel with the columns of the path to what is chosen open, and the focus on it, or on the first option. A press of an option that holds others opens its column; a press of one that holds nothing chooses it and closes the panel, and the panel closes first and the handler is told after, so a handler that opens a dialog keeps its focus. Up and Down go through a column and open the column of the option they reach, Home and End go to its ends, Right goes into what an option holds and Left back to the option the column is of, a letter to the next option that starts with it; Enter on an option that holds nothing chooses it, on one that holds others goes into it. Moving chooses nothing.
- The focus goes back to the trigger on every way out of the panel: a choice, Escape, a press of the trigger. A press outside closes it and leaves the focus where the press put it. Opened again, the panel is on what is chosen, not on where it was left.
- The panel is as wide as the columns open in it, each 124, and as high as the longest of them, up to 280, past which a column scrolls; where the window is narrower than the columns they scroll sideways.
- A cascader that is only built keeps nothing: converted into an element it is the still form of itself, which opens only where the app holds `open`. Mounted (`mount(ctx)`, or by a [field](#field) that is mounted), it keeps whether it is open, its trigger and the path open in its columns.
- The board draws a cascader of one: there is no cascader of any number, and no search over the paths, which the board does not draw.
- To what reads the tree the trigger is a `combobox` with `a11y_haspopup` `listbox`, `a11y_expanded`, `a11y_value` (the path it says), `a11y_invalid` and `a11y_readonly` where they hold; the panel a `dialog`, each column a `listbox` called by the option it is of, and each option an `option` that says `a11y_selected` while it is on the path open, `a11y_expanded` where it holds others, `a11y_level`, `a11y_posinset` and `a11y_setsize`.
- Parts: `CascaderPart::Trigger`, `Value`, `Placeholder`, `Chevron`, `Clear`, `ClearIcon` (a select's and a tree select's), `Panel` (laid over the popover's panel), `Columns` (laid over the column view's box: its greatest height), `Column`, `Option`, `OptionIcon`, `OptionLabel` and `OptionChevron` (laid over the column view's column, item, icon, label and chevron), and `Empty`. States: a select's, `CascaderState::Open`, `Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`, `Fluid`, and `Selected`, resolved on the parts of an option on the path open.
- In the design the trigger is 260 by 36 on `Background` in an edge of 1 in `Input`, `radius-md`, its path 12 from its edge in the body's type and a chevron of 16 in `MutedForeground` 8 after it. The panel is 4 under it on `Popover` in an edge of 1 in `Border`, `radius-lg`, with the popover's shadow, and holds the columns with no room around them: each 124 wide, padded 4, with a line of 1 in `Border` at its end but the last's. An option is 30 high (22, 38 and 42 at the compact, comfortable and touch densities), 8 in, `radius-sm`, its label at 13 in `Foreground`, and where it holds others a chevron of 14 in `MutedForeground` 4 after it. The options on the path are on `Highlight` with their labels at 500, the last as the others; under the pointer and held down an option has the veils of those states, and the focus from the keyboard is a ring of 2 in `Ring` inside it.
- A theme's renderer is given `CascaderView`: `view.text` (what the trigger says), `view.trigger()` (the behavior with the trigger's style), `view.value()`, `view.clear()`, `view.chevron()`, `view.content()`, `view.panel()` and `view.render()`. It is set for `Cascader<T>` of any `T`.

## Tree Select

A select whose options are a tree: a file of a project, a category of a catalogue, the folders a search looks in.

```rust
use lurq::ui::{TreeItem, TreeSelect};

// file: Signal<Option<&str>>: one or none, of the files alone.
TreeSelect::optional(file, [
  TreeItem::folder("crates", "crates").items([
    TreeItem::folder("lurq", "lurq").items([
      TreeItem::file("lib.rs", "lib.rs"),
      TreeItem::file("theme.rs", "theme.rs"),
    ]),
  ]),
  TreeItem::file("Cargo.toml", "Cargo.toml"),
])
.label("File")
.placeholder("Select a file")
.leaves_only(true)
.clearable(true)
.mount(ctx);

// Any number: a press adds a row or takes it out, and the panel stays open.
TreeSelect::multiple(folders, items).max_shown(2).mount(ctx);

// Held by the app, which is told each choice.
TreeSelect::controlled(app.file(), items, |file| app.open(file));
```

| Builder | |
| --- | --- |
| `new(value, items)`, `optional(value, items)`, `multiple(values, items)` | The tree's items (`TreeItem<T>`, a [tree view](#tree-view)'s) and what is chosen of them, bound to a `Signal<T>`, a `Signal<Option<T>>` or a `Signal<Vec<T>>` |
| `controlled(value, items, on_change)`, `controlled_optional(..)`, `controlled_multiple(..)` | The same held by the app, which is told `on_change(&T)`, `on_change(&Option<T>)` or `on_change(&[T])` and shows what it gives back |
| `on_change(handler)` | Told what is chosen after a choice a person makes, after the signal is set; not for the row chosen already, nor when the app sets the signal. The select is `TreeSelect<T, T>`, `TreeSelect<T, Option<T>>` or `TreeSelect<T, [T]>`, so a handler of another type does not compile |
| `open(Signal<bool>)` | Whether the panel is open, where the app keeps it. `on_open_change(bool)` hears each change a person makes, a choice too; `on_dismiss()` Escape and a press outside, before it |
| `branches(Signal<Vec<T>>)`, `branches_controlled(open, on_branch_change)` | The branches of the tree that are open, where the app keeps them. `on_branch_change(&value, open)` hears each one a press or a key opens or closes: where a lazy item's children are brought |
| `leaves_only(bool)` | A press of a row that holds others opens or closes it in place of choosing it: only what holds nothing is chosen |
| `path(bool)`, `separator_text(&str)` | The trigger of a select of one says the whole path to what is chosen, `crates / lurq / lib.rs` (it does unless told otherwise), or its label alone; and what stands between the steps, `" / "` |
| `max_shown(n)` | How many labels the trigger of a select of any number says before it counts the rest, `lib.rs, theme.rs +2` |
| `placeholder(&str)`, `size(TreeSelectSize)`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `full_width`, `name` | As a [select](#select) has them. A read only one and a disabled one do not open; a full width one's panel is as wide as it is |
| `clearable(bool)`, `clear_label(&str)` | A cross before the chevron, while something is chosen, that takes it away, of a select that may hold none; and what it is called, "Clear" |
| `empty_text(&str)` | What the panel says where the tree has no items: "No options" |
| `label(&str)`, `panel_label(&str)` | What the select is called to what reads the tree, and its panel and its tree in place of that |
| `placement`, `offset`, `collision`, `inline`, `dismiss_on_escape`, `dismiss_on_outside_press` | As a [popover](#popover) has them: the panel is one |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the trigger's; the panel has it with `-panel`, the tree with `-tree`, the cross with `-clear` |
| `mount(ctx)` | In place of converting it: the select keeps whether it is open, the trigger the focus goes back to and which branches are open |

- It is built of what is there already: its trigger is a [select](#select)'s box (the same recipe, sizes, ring and judgments), its panel a [popover](#popover) and what the panel holds a [tree view](#tree-view), with a tree's rows, icons, chevrons and keys. What a `TreeItem` has, it has here: `disabled`, `lazy`, `trailing`.
- A press, Enter, Space, Down or Up on the trigger opens the panel, and the focus goes to the chosen row, or the first. Up and Down move through the rows, Right and Left open, close and go in and out, a letter goes to the next row that starts with it. Of one, a press of a row or Space chooses it, Enter does on a row that holds nothing (on one that holds others it opens or closes it), and the panel closes first and the handler is told after, so a handler that opens a dialog keeps its focus. Of any number, a press of a row or Space adds it or takes it out, a press with Shift chooses the rows from the last chosen to it, and the panel stays open.
- The focus goes back to the trigger on every way out of the panel: a choice, Escape, a press of the trigger. A press outside closes it and leaves the focus where the press put it. Tab in the panel leaves the tree, which is one stop.
- A select that is only built keeps nothing: converted into an element it is the still form of itself, which opens only where the app holds `open`. Mounted (`mount(ctx)`, or by a [field](#field) that is mounted), it keeps whether it is open, its trigger and which branches are open. As the panel opens, the branches the chosen rows are in open, so it opens on what is chosen; while it is open nothing opens or closes but by a person, so a row taken out of what is chosen stays under the focus.
- A select of any number has no checkboxes: the board draws none, and its rows are a tree's, in the highlight while they are chosen.
- A [cascader](#cascader) is its sibling for a tree shown as columns that open one after the other, of which one is chosen. Its trigger is the same.
- To what reads the tree the trigger is a `combobox` with `a11y_haspopup` `tree`, `a11y_expanded`, `a11y_value` (what it says), `a11y_invalid`, `a11y_readonly` and `a11y_multiselectable` where they hold; the panel a `dialog` and what it holds a `tree` of `treeitem`s, called by the select's label.
- Parts: `TreeSelectPart::Trigger`, `Value`, `Placeholder`, `Chevron`, `Clear`, `ClearIcon`, `Panel` (laid over the popover's panel), `Tree` (its greatest height, past which the tree scrolls), `Row`, `RowIcon` and `RowLabel` (laid over the tree view's row, icon and label) and `Empty`. States: a select's, `TreeSelectState::Open`, `Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`.
- In the design the trigger is 260 by 36 on `Background` in an edge of 1 in `Input`, `radius-md`, its text 12 from its edge in the body's type and a chevron of 16 in `MutedForeground` 8 after it. The panel is 4 under it, 260 wide on `Popover` in an edge of 1 in `Border`, `radius-lg`, with the popover's shadow and 8 of padding around the tree. The tree scrolls past 280.
- A theme's renderer is given `TreeSelectView`: `view.text` (what the trigger says), `view.trigger()` (the behavior with the trigger's style), `view.value()`, `view.clear()`, `view.chevron()`, `view.panel()` and `view.render()`. It is set for `TreeSelect<T>` of any `T`.

## Transfer

Two lists side by side, and the buttons between them that move the items checked in one to the other: the features a build has, the columns a table shows, the members of a group.

```rust
use lurq::ui::Transfer;

// enabled: Signal<Vec<&str>>: the values of the items on the right.
Transfer::new(enabled, [
  ("canvas", "canvas"),
  ("svg", "svg"),
  ("router", "router"),
  ("wgpu", "wgpu"),
  ("winit", "winit"),
])
.source_title("Available")
.target_title("Enabled")
.label("Features")
.mount(ctx);

// Held by the app, which is told each new value.
Transfer::controlled(app.features(), items, |features| app.set_features(features));
```

| Builder | |
| --- | --- |
| `new(value, items)` | Bound to a `Signal<Vec<T>>`: the items of its values are on the right, in the order they came there, and a move sets it. The items are a [listbox](#listbox)'s options: `(value, label)` or a `SelectOption` with `disabled`, `detail`, `hint`, `leading`, `trailing`. The left list has the rest, in the order of the items |
| `controlled(value, items, on_change)` | The same held by the app, which is told the whole value a move asks for and shows what it gives back |
| `on_change(handler)` | Told the whole new value, `&[T]`, after a move, after the signal is set; not when the app sets the signal. A second handler is called after the first |
| `checked(Signal<Vec<T>>)`, `checked_controlled(checked, on_checked_change)`, `on_checked_change(handler)` | What is checked in either list, where the app keeps it (without it a mounted transfer keeps it), and what hears each change a press, a key or a move makes |
| `source_title(&str)`, `target_title(&str)` | What the lists are called, in their headers and to what reads the tree: "Available" and "Selected" |
| `count_text(Fn(checked, total) -> String)` | What a header says of how many of its items are checked, of how many: `1/3` |
| `select_all_label(&str)`, `add_label(&str)`, `remove_label(&str)` | What the header's checkbox and the two buttons are called to what reads the tree: "Select all", "Add", "Remove" |
| `empty_text(&str)` | What a list with no items says: "No items" |
| `locale(&Locale)` | The words it speaks in, its lists' titles, the names of its checkboxes and buttons and what an empty list says, in place of the app's (`App::set_locale(Locale::for_tag("uk"))`, English where it set none), whose words are `Locale::input`. A word told by itself (`source_title`, `add_label` and the others) stays; the count is no word |
| `label(&str)`, `disabled(bool)`, `density(Density)` | What the transfer is called, to what reads the tree; nothing in it takes input; how high an item is, whatever the theme's density |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the root's; the lists' items have it with `-source` and `-target` (each item that and `-` and its place), their headers' checkboxes with `-source-all` and `-target-all`, the buttons with `-add` and `-remove` |
| `mount(ctx)` | The transfer keeps what is checked, measures its room before it is painted and finds its narrow form there, and a list of more than two hundred items builds only those near its view. Converting it into an element declares the same component; `build(theme)` is the still form, wide only |

- Each list is a [listbox](#listbox) of any number with `checkboxes(true)`: a press of an item checks it or unchecks it and leaves the rest, a press with Shift checks the range from the last one checked and keeps what else is, and the keys are a listbox's: one stop of Tab, Up and Down, Home and End, the page keys, a letter, Space to check, Shift with the keys to check a range, Ctrl+A to check every item that can be. A disabled item is faded, is not checked and does not move.
- The checkbox in a list's header checks every item of the list that can be checked, or none, and is half checked while some are; it is disabled while none can be. The count beside it says how many are checked, of how many items the list has.
- The primary button, with a chevron to the right, moves what is checked on the left to the end of the right, in the order of the items; the outline one, with a chevron to the left, moves what is checked on the right back. Each is disabled while its list has nothing checked. A move sets the value and tells `on_change`, then takes what moved out of what is checked. The focus stays on the button that was pressed.
- Its narrow form is chosen by its own room: mounted, it measures the room before the frame is painted, and where the two lists and the buttons do not fit side by side the lists are as wide as the room, one over the other, and the buttons are side by side between them with their chevrons turned down and up. The first frame is the settled one. Converted into an element (`.child(transfer)`) it is that same live component, as `mount(ctx)` makes it; `build(theme)` is its still form, side by side whatever its room, with nothing checked unless the app holds what is.
- A list scrolls past 264, eight items; one of more than two hundred items, in a mounted transfer, is a listbox mounted in a virtual list, which builds only the items near its view and which the keys reach every item of.
- To what reads the tree it is a `group` called by its label, each list a `listbox` that is `a11y_multiselectable`, called by its title, of `option`s that say `a11y_checked` and `a11y_selected`; the header's checkbox is a `checkbox` that says `mixed` while some are checked, and the buttons are `button`s called "Add" and "Remove".
- Parts: `TransferPart::Root` (the room between the lists and the buttons), `List`, `Header`, `Title`, `Count`, `Items` (laid over the listbox's box: its greatest height, past which the items scroll), `Item` and `ItemLabel` (laid over the listbox's option and label), `Empty` and `Actions`. States: `TransferState::Disabled` and `Stacked`.
- In the design a list is 190 wide on `Background` in an edge of 1 in `Border`, `radius-lg`, 12 from the buttons, the two lists in the middle of each other's height. Its header is 36 high on `Muted` with a line of 1 in `Border` under it, 12 in from its sides: a checkbox, the title in the body's type at 13 and 500 in `Foreground`, and the count in the mono type at 11 in `MutedForeground`, 8 apart. Its items are 4 in from its top and bottom, each 32 high (24, 40 and 44 at the compact, comfortable and touch densities), 12 in, a checkbox and the label at 13, 8 apart; under the pointer an item is on `Muted`, and the focus from the keyboard is a ring of 2 in `Ring` inside it. The buttons are 32 square, 8 apart. A list with no items says so at 13 in `MutedForeground`, 12 in.
- A theme's renderer is given `TransferView`: `view.source` and `view.target` (each a `TransferList` with its title, its count, how many are checked and `list.header()`, `list.select_all()`, `list.render(items)`), `view.button(to_target)`, `view.actions()`, `view.stacked` and `view.render()`. It is set for `Transfer<T>` of any `T`.

## Token Input

A box that holds a list as tokens, with what is being typed after them: tags, recipients. Or a text in which the app
finds its tokens: the variables of a template.

```rust
use lurq::ui::{TagVariant, Token, TokenInput};

// Tags, a list: Enter, a comma or a space makes a token of what is typed.
// tags: Signal<Vec<String>>
TokenInput::list(tags.clone())
  .separators([',', ' '])
  .unique(true)
  .placeholder("Add tag")
  .on_change(|tags| save(tags));

// A template, a text on rows: its variables are tokens with no cross.
// template: Signal<String>
TokenInput::new(template.clone())
  .rows(2, 6)
  .find_tokens(|text| {
    text
      .match_indices("{{name}}")
      .map(|(at, name)| Token::new(at..at + name.len()).variant(TagVariant::Info).removable(false))
      .collect()
  });
```

| Builder | |
| --- | --- |
| `list(Signal<Vec<String>>)` | A list: each item is a token. What is typed is the draft, which is no part of the list until it is made an item. What the app sets on the list is shown |
| `separators(chars)` | The characters that make an item of the draft, as Enter does: a comma by default. A pasted list is split at them, at line breaks and at tabs; an item cannot hold one |
| `query(Signal<String>)` | The draft, what is typed and is no item yet: the query read for suggestions, as a [combobox](#combobox)'s is, and what the app sets on it is what the input shows after its tokens |
| `unique(bool)` | An item the list already has is not added again; what was typed is dropped |
| `commit_on_blur(bool)` | The draft becomes an item when the focus leaves, in place of staying typed |
| `dress_token(impl Fn(&str, Token) -> Token)` | Dresses the token of each item of a list: a variant by what the item is, or no cross |
| `on_change(impl Fn(&[String]))` | Called with the list when a person changed it: an item made of the draft, pasted, or taken away |
| `on_query(impl Fn(&str))` | Called with the draft, the query, when a person changed it; not when the app sets `query` |
| `new(Signal<String>)` | A text, tokens and all, where the text around the tokens is the value too; what is typed is set on it, and what the app sets on it is shown |
| `find_tokens(impl Fn(&str) -> Vec<Token>)` | How the app finds its tokens in a text, called in every render. Tokens that overlap one before them, hold a line break or are not on characters are left out. Not asked over a list |
| `Token::new(range)` | A token over a byte range of the text: atomic and removable by default |
| `Token::variant(name)` | A `TagVariant` (`Neutral`, `Info`, `Success`, `Warning`, `Destructive`), `MENTION`, or a variant the theme's recipe names |
| `Token::removable(bool)` | The cross at its end, which a click takes it away by |
| `Token::atomic(bool)` | One thing to the caret (the default), or text drawn as a token that the caret goes into |
| `Token::separated(text, char)` | The items of a list typed with a separator: each item with a separator after it |
| `rows(min, max)` | Text on several rows, between the two counts, tokens wrapping with the words; one line without it |
| `placeholder(&str)` | What it says while it is empty |
| `size(TokenInputSize)` | `Sm` 32, `Md` 36 (the default), `Lg` 40 high, with tokens of 20, 24 and 28 |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | The edge in the destructive, warning or success color, as an input's |
| `read_only(bool)` | Focused, selected and copied, not changed; its tokens have no cross |
| `full_width(bool)` | As wide as what holds it, in place of 260 |
| `on_input(handler)` | Called before an edit, taking a token away by its cross too; `prevent_default()` refuses it |
| `controller(TextInputController)` | Kept by the app: reads the selection and puts text at it, a token chosen from a picker |
| `label(&str)`, `name(..)`, `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As an [input](#input) has them. `id` is the id of the control |

- Over a list (`TokenInput::list`) the tokens and what is typed are two things. The list holds the items and nothing
  else; the draft is the text after the tokens, and it is in no item until Enter or a separator makes one of it,
  trimmed. A token's cross takes its item out of the list; Backspace right after a token selects it and the next takes
  it out. What is typed with the caret among the tokens goes to the draft, which is always after them, and typing over
  a selected token replaces it with a draft. `on_change` hears the list and `on_query` the draft; `on_input` still
  hears every edit, with the text the input keeps the list as.
- The input keeps that text itself, for as long as it is in the tree, by the signal of the list: each item with a
  narrow space (U+2005) after it, which is the room between two tokens and where a row wraps, then the draft. It is
  what a `TokenInputView` has as `text`, what is copied from the input and what tooling reads as its value. A list
  whose signal is made anew in every render starts with no draft in every render: keep the signal.
- Over a text (`TokenInput::new`) it is the engine's `TextInput` with `TextInput::tokens`: what it holds is text, and
  a token is a range of that text the app's function finds in every render, so an edit that changes a token's text
  makes it what the function then says of it. Copying, pasting, undo, a form and tooling see the text: to what reads
  the tree it is a `textbox` whose value is the text, each token as its own words.
- A token is atomic: Left and Right go past it whole, a click selects all of it, Backspace after it selects it and the
  next deletes it. A click on its cross deletes it with the space after it, as one edit `on_input` hears and undo
  brings back. With `Token::separated`, deleting the space after the last token makes it text again, to be edited.
- The box is an input's (one line) or, with `rows`, a textarea's: the same sizes, judgments, read only and disabled,
  and a [field](#field) names and judges it as it does an input.
- Parts: `TokenInputPart::Root` (its width), `Control` (the box: edge, fill, height, padding, the type of the text),
  `Placeholder`, `Token` (fill, edge, radius, height, the room before the text and after the cross, and the gap
  between them), `TokenLabel` (the token's type and color), `TokenIcon` (the cross: its side and color). States:
  `TokenInputState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`, `Multiline`, and a token's own
  `Hovered` and `Selected`. A token's parts are resolved with its variant and its state as the case, so a recipe
  styles a variant's tokens with `.variant(..)` and their looks with `.state(TokenInputState::Hovered, ..)`.
- It is a text input in both forms, always bound to a signal: it has no `controlled` form. A `Token` is a range of
  the text, not an element: it has no `id` or `key`.
- In the design the box is 260 by 36, an input's, padded 6 at its sides. A token is 24 high, `Secondary`, `radius-sm`,
  8 before its text and 6 after its cross; its text is 13 in `SecondaryForeground` and its cross 12 in
  `MutedForeground`, 4 after the text. A colored variant is in the muted fill of its color with its text and cross in
  the color, as a tag is. Under the pointer a token is `SecondaryHover` (a colored one keeps its fill) and its cross
  `Foreground`; covered by the selection it is `Primary` with its text and cross in `PrimaryForeground`. On rows the
  box is padded 6 down too and its lines are 28 high, so a token has room around it.
- A renderer is given a `TokenInputView`: `view.tokens` (what was found), `view.control()` (the engine's input with
  the tokens drawn in the recipe's style), `view.token_style(recipe, cross)` (how the engine draws a token of a
  resolved case), `view.root(control)`.

## Mentions

Text in which a trigger opens a list of what can be mentioned, and what is chosen stands as a token: a comment, a
message, a prompt.

```rust
use lurq::ui::{Avatar, Mention, Mentions, MentionsController, icons};

// Kept by the app, with the value, from one render to the next.
let mentions = MentionsController::new();

Mentions::new(comment.clone(), &mentions)
  .trigger('@', vec![
    Mention::new("Ivan Zaida").avatar(Avatar::new("Ivan Zaida")),
    Mention::new("Ivy Chen").avatar(Avatar::new("Ivy Chen")),
  ])
  .trigger('#', |typed: &str| teams_starting_with(typed))
  .on_mention(|text| println!("mentioned {text}"))
  .placeholder("Write a comment");
```

| Builder | |
| --- | --- |
| `new(Signal<String>, &MentionsController)` | The text, and what the field keeps between renders: the app keeps both |
| `trigger(char, options)` | What a character at the start of a word offers: a `Vec<Mention>`, narrowed to the offers whose label or text has what is typed after it (without regard to case), or a function of what is typed. At most 8 are shown |
| `Mention::new(label)` | An offer: `text(&str)` is what it puts in the field (the trigger and the label by default), `avatar(Avatar)` or `icon(name)` before its label; `disabled(bool)` shows it dimmed and leaves it to no choice, `id` and `key` are its row's |
| `on_mention(handler)` | Called with the text of an offer as it is chosen |
| `find_tokens(..)` | Other tokens of the app's in the text, as a [token input](#token-input) finds them |
| `rows(min, max)` | 2 and 8 unless told |
| `full_width(bool)` | As wide as what holds it, in place of 280 |
| `input_rules(&recipe)` | Rules laid over the token input it is |
| `size(MentionsSize)` | An input's sizes (`MentionsSize` is `TextBoxSize`): the type of its text and mentions, 13, 14 or 15 |
| `placeholder`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `name`, `on_input`, `state`, `style`, `rules`, `force`, `id`, `key` | As a token input has them |

- Typing a trigger at the start of the text or after a space opens the list under it while it offers something. Up and
  Down move the offer that Enter (or Tab) chooses, a click chooses one, the pointer moves the active one, and Escape
  puts the list away while the caret stays in that word, so that Enter is a new line again. A disabled offer is
  passed by: Up and Down go over it, the pointer does not light it, and a click on it chooses nothing. The keys stay in the
  field the whole time: the list takes no focus, and a press on it does not blur the field.
- Choosing puts the offer's text and a space in place of the trigger and what was typed, as one edit through
  `on_input`. From then on that text is a mention wherever it stands as a word of its own: a token of the variant
  `MENTION`, atomic, with no cross. `MentionsController::mention(text)` makes a text a mention without choosing it
  (what a message loaded with mentions has), and `mentioned()` lists them.
- `MentionsController` holds the field's `TextInputController` (`input()`), the active offer, the trigger Escape put
  away and the mentions; a builder has no `Ctx`, and these must outlive a render, as the engine's controller does.
- It is a [token input](#token-input) on rows: to tooling a `textbox` whose value is the text with each mention as the
  offer's text, with `a11y_haspopup` `listbox` and `a11y_expanded`; the list is a `listbox` of `option`s, the active
  one `a11y_selected`.
- Parts: `MentionsPart::Root`, `Control` (laid over the token input's control: its padding, line and least height),
  `Panel` (the list that floats under the trigger), `Item`, `ItemAvatar`, `ItemIcon`, `ItemLabel`; the mentions
  themselves are the token input's parts in the variant `MENTION`. States: `MentionsState::Open`, and an offer's own
  `Highlighted` (the one Enter chooses) and `Disabled`.
- A renderer that draws another field has `view.control()`, `view.item(index, offer)` and `view.panel()`.
- In the design the box is 280 wide and at least 64 high, padded 12 across and 8 down, its text 14 with lines of 1.5.
  The list hangs 4 under the trigger: 220 wide, `Popover`, `radius-lg`, an edge of 1 in `Border`, the medium shadow,
  padded 4. An offer is 36 high, padded 8, `radius-sm`, an avatar of 24 (or an icon of 16 in `MutedForeground`) 8
  before its label in the body's type; the active one, and the one under the pointer, are on `Highlight`. A mention is
  in the text's type on `InfoMuted`, its text in `Info`, 20 high and padded 4.

## Rich Text Editor

Formatted text typed into a box, under a bar of the tools that format it: a comment, release notes, the body of an
article, with pictures among its words. It comes with the `markdown` feature.

```rust
use lurq::{components::RichText, ui::{RichTextEditor, RichTextTool}};

// notes: Signal<RichText>, kept by the app. The document reads from Markdown and writes it.
let notes = ctx.signal(RichText::from_markdown("## Release notes\n\nBackdrop blur **follows clips**.\n\n- Faster shaping"));
RichTextEditor::new(notes.clone())
  .placeholder("Write the release notes")
  .on_change(|document| println!("{}", document.to_markdown()))
  .mount(ctx);

// A comment box: a few tools, no footer.
RichTextEditor::new(comment.clone())
  .tools([RichTextTool::Bold, RichTextTool::Italic, RichTextTool::Bullet, RichTextTool::Link])
  .footer(false)
  .rows(3, 8)
  .mount(ctx);

// Held by the app: it is told each change, and the editor shows what the app answers with.
RichTextEditor::controlled(document.clone(), move |changed| store.set(changed.clone())).mount(ctx);

// Pictures. In `create`: the app's handle on the editor, and the system's dialog for a file.
let editor = RichTextController::new();
let window = ctx.window();
let choose = ctx.future_action(move |()| {
  FileDialog::new().title("Insert a picture").filter("Pictures", &["png", "jpg", "jpeg", "gif", "webp"]).pick_result(&window)
});
// In `render`: the image tool opens the dialog, and the file a person chose goes in at the caret.
if let Some(path) = chosen_path(&choose.state().get()) {
  editor.insert_picture(&path, "A picture");
}
let open = choose.clone();
RichTextEditor::new(notes.clone())
  .controller(&editor)
  .on_image(move || open.run(()))
  .mount(ctx);
```

`chosen_path` is the app's: the path of the first file of a fulfilled state, taken once (see
[Files a person gives](../styling-events/#files-a-person-gives)).

| Builder | |
| --- | --- |
| `new(Signal<RichText>)` | The document. Each change is written into the signal, and what the app writes there shows |
| `controlled(value, on_change)` | A document the app holds: each change is told to `on_change`, and a change the app does not take is taken back |
| `on_change(handler)` | Called with the `&RichText` after each change a person makes: typing, a tool, a key, undo. The signal is set first. Not when the app sets it |
| `placeholder(&str)` | What the box shows while it holds nothing |
| `rows(min, max)` | The rows of body text the box shows at least, and at most before its text scrolls inside it: 6 and 16 unless told (`RICH_TEXT_MIN_ROWS`, `RICH_TEXT_MAX_ROWS`) |
| `tools(tools)` | The `RichTextTool`s of the bar, in order: `Style`, `Bold`, `Italic`, `Underline`, `Strikethrough`, `Bullet`, `Ordered`, `Quote`, `Code`, `Link`, `Image`, `Undo`, `Redo`. A line stands between two groups and the history is at the bar's end. All but `Image` unless told (`RichTextTool::DEFAULT`) |
| `toolbar(bool)`, `footer(bool)` | Whether it has its bar, and the row under the box that says Markdown shortcuts are on and counts the words: both unless told |
| `markdown_shortcuts(bool)` | Whether Markdown typed becomes what it says: on unless told |
| `on_link_click(handler)` | Called with a link's target when it is clicked with Ctrl held (Cmd on a Mac): the app opens it |
| `on_image(handler)` | What the image tool does; with it the bar has the tool. The app asks for a picture, with the system's dialog for a file or a panel of its own, and puts it in with the controller's `insert_picture` |
| `on_paste_image(Fn(&ClipboardImage) -> Option<String>)` | What is done with a picture pasted as pixels, a screenshot: the app keeps the pixels (`image.to_png()` into a file, a store of its own) and answers with the source the document names them by, and the editor puts that picture at the caret. Without it a paste of pixels does nothing: a document names its pictures and holds no pixels |
| `picture(Fn(&InlineObject) -> Element)` | What is drawn for a picture of the document, in place of a [`Picture`](#picture) of its source: for pictures that are no files and no addresses (bytes the app holds, a resource, a key of its own store) |
| `controller(&RichTextController)` | The app's handle on the editor, kept from one render to the next: `toggle_mark`, `set_paragraphs`, `toggle_paragraphs`, `set_link`, `insert_text`, `insert_link`, `insert_picture(source, label)`, `insert_object(&InlineObject)` for a picture with a width, `select`, `undo`, `redo`, `value`, and `input()`, the engine's controller of its text |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | What the app judged of what it holds: its edge in the destructive, warning or success color |
| `read_only(bool)` | It is focused, selected and copied, and not changed: its tools take no input |
| `full_width(bool)` | As wide as what holds it, in place of 560 |
| `label(&str)` | What it is called, to what reads the tree |
| `name(..)` | The name of its document in a form (feature `form`): its value there is the document as Markdown, as it is at each change |
| `toolbar_label`, `style_label`, `bold_label`, `italic_label`, `underline_label`, `strikethrough_label`, `bullet_label`, `ordered_label`, `quote_label`, `code_label`, `link_label`, `image_label`, `undo_label`, `redo_label`, `more_label`, `apply_label`, `remove_link_label` | What each control is called |
| `paragraph_text`, `heading_text(Fn(level))`, `code_block_text`, `link_placeholder`, `shortcuts_text`, `count_text(Fn(words))` | The words it draws: the styles of its style control, the field of a link, and the footer's two texts |
| `mount(ctx)` | Puts it in the tree: it keeps the text being edited, what its bar shows and the panel of a link |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. Its parts have the id with `-content`, `-toolbar`, `-style`, `-bold` and each tool's name, `-count` and `-link-field` after it |

- Its value is a [`RichText`](../text-input-styled/#one-value-richtext): the text, the marks over it (`StyledSpan`s), what each paragraph is (`ParagraphKind`) and the pictures in it (`InlineObject`s). `RichText::from_markdown` and `to_markdown` read and write the Markdown an app stores.
- A picture is a character of the document, alone in a paragraph or among words: an [object](../text-input-styled/#objects) of the engine's input. It is as large as it is, or as wide as the document says (`InlineObject::width`), and never wider than its line; its row is as high as it needs, and it stands on the baseline of the words beside it. The caret goes past it as past one character, a press selects it, Backspace after it selects it and the next deletes it, a selection takes it whole, and undo brings it back as it was. While the editor has the focus, a selected picture is under a veil of the selection's color. It is no word to the count of the footer. Markdown writes it as an image, `![label](source)`, or as `<img src alt width>` where it has a width, and a copy and a paste carry it so. `insert_picture` is `false` in a line of a block of code, which holds no picture, and in an editor that takes no change.
- A picture selected alone has a handle on its lower corner at the end: dragged, it makes the picture wider or narrower, from 16 to the width of the text, its height following its shape, as one step of the undo told to `on_change` once, when it is let go. A read only or disabled editor has none. The width goes into the document (`InlineObject::width`) and into its Markdown as `<img width>`.
- What is typed after a picture goes after it: a pasted or inserted picture has the caret after it, an arrow from a selected picture puts the caret before or after it, and Enter with a picture selected starts a paragraph after the picture and leaves it. Anything else typed over a selected picture takes its place, as over any selection.
- A paste of a clipboard with no words in it is a paste of pictures: pixels (a screenshot, an image copied in another app) go to `on_paste_image`, and files that are pictures, copied in a file manager, are put in by their paths with no handler, each called by its file's name. A clipboard that holds words is pasted as text, whatever else it holds.
- A picture is drawn by a [`Picture`](#picture) of its source, an address where the source starts as one (`http://`, `https://`) and the path of a file otherwise, called by its label: it shows a skeleton while it loads and says so when it did not come. The pointer and the focus do not reach it, so it has no bar and opens no lightbox. `picture(..)` draws it another way.
- It is the engine's [styled text input](../text-input-styled/) with kinds of paragraphs: the caret, the selection, the clipboard and the undo history are a text input's own, and a selection runs over paragraphs of any kind. A copy keeps the formatting: the clipboard of the system is given the selection as Markdown where it has marks or a paragraph that is not body text (the plain text where it has neither), so another app or another lurq process pastes the Markdown, and an input of the same app the plain text while the clipboard still holds what was written. A paste of Markdown, or of a text that reads as it (a line with a block mark, or a mark Markdown reads), comes with its marks and kinds as one undo step; any other text, and anything pasted into a block of code, is plain text in the style at the caret.
- A tool works on the selection: a mark over the selected words, or over what is typed next where nothing is selected; a list, a quote or a style over every paragraph the selection touches, and a second press of a list or a quote makes them paragraphs again. The bar shows what the selection is: a mark's tool is on while all of the selection has it, the style control says what its paragraphs are. A press of a tool leaves the focus in the text.
- The style control makes a paragraph body text, a heading of a level from 1 to 3, or a line of a block of code. The link tool opens a panel at the selection with a field for the target: Enter or Apply links the selection, an empty target or Remove link takes the link away, Escape changes nothing, and the focus goes back to the text.
- The keys, with Ctrl (Cmd on a Mac): B, I and U for bold, italic and underline, Shift+X to strike, E for code, K for a link, Alt+0 to Alt+3 for a paragraph and the headings, Shift+7, Shift+8 and Shift+9 for a numbered list, a bulleted one and a quote; undo and redo are the text input's own, by lurq's keys (Ctrl+Z, Ctrl+Shift+Z and Ctrl+Y, Cmd+Shift+Z on a Mac, in any layout) and an Edit menu's, and the tooltips of Undo and Redo say those keys (`EditCommand::accelerator`). Enter in an empty item or quote ends the list, Enter at the end of a heading starts a paragraph, and Backspace at the start of anything but a paragraph makes it one.
- Markdown shortcuts: `# `, `## `, `### `, `- `, `* `, `1. `, `> ` and three backticks at the start of a paragraph make it what they say and are taken away; `**bold**`, `*italic*`, `_italic_`, `` `code` `` and `~~struck~~` become the mark as their last character is typed.
- Parts: `RichTextEditorPart::Root`, `Toolbar` (laid over a [toolbar](#toolbar)'s root), `Tool`, `ToolOn` and `ToolIcon` (over its buttons and toggles, and a toggle that is on), `Style`, `StyleChevron` and `StyleList` (over a [select](#select)'s trigger, chevron and panel), `Content` (the box: its padding, the type of body text, and its gap, the room between two paragraphs), `Placeholder`, `Heading1`, `Heading2`, `Heading3` (levels 4 to 6 are drawn as the third), `Bullet` and `BulletMark`, `Ordered` and `OrderedMark`, `Quote` and `QuoteMark`, `CodeBlock`, `Footer`, `FooterText`, `LinkPanel` (over a [popover](#popover)'s panel). A paragraph's part gives its type over the content's, its start padding is the room before its text, and a mark's part its width, its gap to the text and its color. States: `RichTextEditorState::Focused`, `Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`.
- In the design the box is 560 wide in `Background` with an edge of 1 in `Border` and the large radius. The bar is 40 high on `Muted` over a line of 1, padded 6, its tools 28 square with the small radius and icons of 15, 2 apart, one that is on filled with `Background` (the board's `accent` is the bar's own fill, and would not show); the style control is 28 high and as wide as the name it shows (padded 10 before it and 6 after its chevron of 12, 6 from the name), its text 12, and its list as wide as its longest style with the check beside it. The box is padded 18 across and 16 down, its text the body's 14 in rows of 18, its paragraphs 10 apart. A heading of the second level is 22 semibold in rows of 26 (the first 28 bold in rows of 34, the third 18 semibold in rows of 22). An item's text starts 21 in after a dot of 5 in `MutedForeground`; a numbered item's 24 in, 6 after its number; a quote's 14 in, in `MutedForeground`, after a bar of 3 in `Border`; a block of code is the mono family at 13 in rows of 21 on `Muted`, 12 in. A row is as high as its kind says, whatever the font's own line is. The footer is 28 high over a line of 1, padded 12, its texts the mono family at 11 in `MutedForeground`; the count is of the runs of what is not white space. An empty editor draws its placeholder in the kind of its first paragraph, after its mark.
- The board draws no picture in a document, only the tool: how one stands in its row, the veil of a selected one and its size are lurq's, and it has no handles, no caption and no bar of its own.
- In a room narrower than its bar the tools that do not fit go into the menu of a More button at the bar's end, as a toolbar's do; the style control stays.
- To what reads the tree it is a `group` called by its label, with a `toolbar` of named tools and a `textbox` that is `a11y_multiline`, in which each picture is an `img` called by its label.
- Built without `mount` (converted into an element) it is the still form of itself: the document as it is drawn, with nothing kept between two renders. A [field](#field) the app mounts (`Field::new("Notes", editor).mount(ctx)`) mounts it, calls it by its label, judges it and disables it.
- A renderer that draws another editor has `view.bar()` (the toolbar, to add to), `view.toolbar` (it built), `view.content()`, `view.footer()`, `view.link_panel()`, `view.root()`, `view.paragraph_styles()` and `view.format`, what the selection is.

## Editable

A text that is read where it stands and edited in place: a title, a name, a cell.

```rust
use lurq::ui::Editable;

// title: Signal<String>, set when an edit is kept.
Editable::new(title.clone())
  .label("Project name")
  .placeholder("Untitled")
  .on_change(|name| println!("renamed to {name}"))
  .mount(ctx);

// A list of names: no buttons, as wide as the row.
Editable::new(file.clone()).actions(false).full_width(true).mount(ctx);

// Held by the app, which starts the edit itself.
Editable::controlled(&name, move |typed| rename(typed)).editing(renaming.clone()).mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<String>)` | The text. An edit that is kept is written into the signal |
| `controlled(value, on_change)` | A text the app holds: an edit that is kept is told to `on_change`, and the text shown is what the app answers with |
| `on_change(handler)` | Called with the new `&str` when an edit that changed it is kept: by Enter, the button, or leaving the field. The signal is set first. Not for an edit that changed nothing |
| `on_cancel(handler)` | Called when an edit is dropped: by Escape or the button |
| `editing(Signal<bool>)` | Whether it is being edited, in the app's signal: the app starts an edit by setting it. Without it a mounted editable keeps this itself |
| `default_editing(bool)` | Whether one that keeps this itself is being edited as it is first shown: a name just made |
| `on_editing_change(handler)` | Called with `true` when a person starts an edit and with `false` when one ends, after `on_change` or `on_cancel`. Not when the app sets the signal |
| `placeholder(&str)` | What the reading form shows while the text is empty, and the field while nothing is typed |
| `actions(bool)` | Whether the editing form has the buttons that keep the edit and drop it: it has unless told |
| `submit_on_blur(bool)` | Whether leaving the field keeps what was typed: it does unless told, and then leaving drops it |
| `invalid(bool)` | What is typed is wrong: the field's edge says so |
| `read_only(bool)` | The text is read and cannot be edited: no pencil, and a press does nothing |
| `full_width(bool)` | As wide as what holds it: the text is cut where it is longer, and the field fills the room its buttons leave |
| `label(&str)` | What the text is, to what reads the tree |
| `confirm_label(&str)`, `cancel_label(&str)` | What its two buttons are called: "Save" and "Cancel" |
| `mount(ctx)` | Puts it in the tree: it keeps whether it is being edited and what is typed, and moves the focus as an edit starts and ends |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. The field has the id with `-field` after it, the buttons `-confirm` and `-cancel` |

- The reading form is one stop of Tab. A press of it, Enter, Space or F2 puts a field where the text was, with the text selected and the focus in it. Enter keeps what was typed and Escape drops it, as the two buttons do; leaving the field keeps it.
- When an edit ends by a key or a button the focus goes back to the reading form. When it ends because the focus went elsewhere, the focus stays where it went: Tab in the field keeps the edit and goes on to the control after the editable. The two buttons are pressed by the pointer and are no stops of Tab.
- Parts: `EditablePart::Root` (the reading form), `Label`, `Placeholder`, `Icon` (the pencil), `Editing` (the row of the field and its buttons), `Field` (laid over an [input](#input)'s root and control), `Confirm` and `Cancel` (over a [button](#button)'s root). States: `EditableState::Editing`, `Empty`, `Invalid`, `ReadOnly`, `Disabled`, `FullWidth`.
- In the design the reading form is a row 32 high: the text 16 semibold in `Foreground`, and 8 after it a pencil of 14 in `MutedForeground`. Edited, a field 160 wide and 32 high with text of 14, and 6 after it two buttons 32 square with icons of 16: the outline one keeps the edit, the ghost one drops it. A text longer than its room is cut with an ellipsis, and the pencil keeps its place.
- To what reads the tree the reading form is a button called by its label, with the text as its description; the field is a `textbox` of the same name.
- Built without `mount` it is the still form of itself: the reading form, or the editing one where the app's signal says so.
- A renderer that draws another editable has `view.reading()` and `view.editor()`.

## Alert

A message in the flow of a page about what is on it: a file that could not be saved, a disk that is nearly full, a shortcut to know.

```rust
use lurq::ui::{Alert, AlertVariant, Button, ButtonSize, ButtonVariant, Progress, icons};

Alert::new("Could not save")
  .description("The settings file is read-only.")
  .variant(AlertVariant::Destructive)
  .action(Button::new("Retry").variant(ButtonVariant::Outline).size(ButtonSize::Sm))
  .on_close(move || shown.set(false));

// More than text under the description, and an icon of its own.
Alert::new("Downloading lurq 0.61")
  .variant(AlertVariant::Info)
  .icon(icons::DOWNLOAD)
  .child(Progress::new(0.6).label("Downloading"));

// Several actions, under its words: the least first, the main one last.
Alert::new("Working offline")
  .description("Changes are kept on this computer until the server answers again.")
  .variant(AlertVariant::Warning)
  .stacked(true)
  .action(Button::new("Details").variant(ButtonVariant::Ghost).size(ButtonSize::Sm))
  .action(Button::new("Reconnect").variant(ButtonVariant::Outline).size(ButtonSize::Sm));
```

| Builder | |
| --- | --- |
| `Alert::new(&str)` | The title: what it is about. It is what the alert is called |
| `description(&str)` | More of it, under the title |
| `variant(impl Into<Name>)` | An `AlertVariant`: `Neutral` (the default), `Info`, `Success`, `Warning`, `Destructive`; a variant a theme adds has the neutral one's icon and role |
| `icon(impl Into<IconChoice>)` | An icon of the theme's set in place of the variant's (`icon(icons::DOWNLOAD)`), or none (`icon(None)`) |
| `action(element)`, `actions(..)` | Add what can be done about it, after those there: a button, a link. An alert has several, the least first and the main one last, at its end before the close control |
| `stacked(bool)` | Its actions under its words, at their start and 8 under them, wrapping where one line does not hold them, and not at its end: for long words, or more actions than its end has room for |
| `actions_end(bool)` | Actions under its words (stacked or compact) stand at their far end and not at their start. State `actions_end` |
| `align_top(bool)` | The icon, the words, the actions at its end and the close control stand at the top, the actions on its first line, however many lines the words take; it is centred on the actions unless told. State `align_top` |
| `note(&str)` | A quieter line after the actions, under the words: what a press of them does not say. Part `Note` (12, 1.5 high, `MutedForeground`), state `note` |
| `compact(bool)` | Laid out for a narrow column, as an inspector has it: the icon beside the title alone (part `Head`, 8 apart, with the close control at its end), and the description, the body, the actions and the note under them across the whole box, 6 under the head. Its actions are under its words. State `compact` |
| `Alert::untitled()` | An alert with no title: its description says it all. An empty title draws no title part, and the alert is called by its description |
| `child(element)`, `children(..)` | Elements under the description, for more than text |
| `on_close(handler)` | Called on a press of the close control, which an alert has once it is given `on_close` and has not until then. The app takes the alert away. A second handler is called after the first |
| `label(&str)` | What it is called to tooling, in place of its title |
| `role(&str)` | What it is to tooling, in place of the variant's |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the close control's states; `id` is the id of the box |

- It is read where it stands and is not an overlay. It is as wide as the room it is given, and its title and description wrap in the room the icon, the actions and the close control leave them; a stacked alert's words have all of its width but the icon's and the close control's.
- Its actions are 8 apart in the part `Action`, at its end and centred on their height, or in a stacked alert under the description and the body, as wide as the words. Given an id `x`, the row of them is `x-action`; each action keeps the id the app gave it.
- The close control is a stop of Tab with the focus ring; Enter, Space and a click press it.
- To what reads the tree (`lurq_snapshot` of the MCP server) a warning and a destructive alert are an `alert`, which a screen reader says at once, and the others a `status`, said when the reader is done; each is called by its title, or by its `label`. `role("status")` says a warning is no news, `role("alert")` that an informative one is urgent. `AlertVariant::role()` is the variant's.
- Parts: `AlertPart::Root` (the box), `Icon`, `Text` (the title, the description, the body, a stacked alert's actions and the note, one over the other), `Title`, `Description`, `Body`, `Action`, `Close`, `Note`, `Head` (a compact alert's icon and title). States: `AlertState::Action`, which centres what is in the box on the actions where they are at its end, `Stacked`, its actions under its words, `AlignTop`, `Note`, `Compact` and `ActionsEnd`. Variants by `AlertVariant`.
- Given an id `x`, its parts are `x-icon`, `x-text`, `x-title`, `x-description`, `x-body`, `x-action` (the row of the actions), `x-note`, `x-head` and `x-close`. An app whose own notice named the words under the title `x-body` and its actions `x-actions` reads `x-description` and `x-action`: the names of the parts, which a theme's rules style.
- In the design the box is in the `Card` color with an edge of 1 in `Border`, corners of `radius-lg`, padded 16. The icon is 16, 12 from the title (14, medium, `Foreground`) over the description (13, 1.5 high, `MutedForeground`), 4 apart. An informative, a successful and a warning alert are in `InfoMuted`, `SuccessMuted`, `WarningMuted`, with an edge and an icon of `Info`, `Success`, `Warning` and a description in `Foreground`. A destructive one keeps the `Card` color, with an edge, an icon and a title of `Destructive`. The icon is the information sign for a neutral and an informative alert, the check in a circle for a successful one, the warning sign for a warning and a destructive one. The close control is 16 in `MutedForeground`, in `Foreground` under the pointer.
- `AlertVariant` is shared with the banner and the callout: `icon()`, `accent()` (the color of its icon, edge, bar and link) and `tint()` (its fill) say the same for the three.
- The icon is the information sign for a neutral alert as for an informative one, as the board draws it.
- A renderer that draws another alert starts from `view.text()` (with a stacked or compact alert's actions under the words, and the note), `view.icon()`, `view.title()` (`None` for an untitled one), `view.actions()`, `view.note()`, `view.head()` (a compact alert's) and `view.close()` (the behavior), with `view.role`, `view.label`, `view.name()`, `view.close_button`, `view.stacked`, `view.align_top`, `view.compact` and `view.actions_end`. `view.variant` is the variant's name, and `view.actions` what the app gave it to do.

## Banner

A message across the full width of what holds it, one line where it fits, at the top of a window or a page: a new version, a connection lost, a trial that ends. The app shows it and takes it away.

```rust
use lurq::ui::{AlertVariant, Banner, BannerPart, Link, icons};

Banner::new("lurq 0.61 is available with faster text shaping.")
  .variant(AlertVariant::Info)
  .link(Link::new("Update now").on_click(|_| update()))
  .open(open.get())
  .on_close(move || open.set(false));

// Along the top edge of a window: in its middle, with no corners of its own.
Banner::new("No connection. Changes will sync later.")
  .variant(AlertVariant::Warning)
  .icon(icons::WIFI_OFF)
  .centered(true)
  .style(BannerPart::Panel, |s| s.radius(0.0));
```

| Builder | |
| --- | --- |
| `Banner::new(&str)` | What it says. It is what the banner is called |
| `variant(impl Into<Name>)` | As an [alert](#alert)'s. A neutral and an informative banner have the megaphone |
| `icon(impl Into<IconChoice>)` | An icon of its own, or none: `icon(None)` |
| `link(Link)` | A link after the text, in the variant's color and semibold |
| `action(element)` | Any element after the text in place of a link: a button. A banner has one action: a second, or a link, takes its place |
| `on_close(handler)` | Called on a press of the close control, which a banner has once it is given `on_close` and has not until then. A second handler is called after the first |
| `open(bool)` | Whether it is shown: the app's to say, which a press of the close control does not change. It is unless it says |
| `label(&str)` | What it is called to tooling, in place of its text |
| `centered(bool)` | Its icon, text and action in its middle rather than from its start |
| `sticky(bool)` | It sticks to the top of what scrolls the page it is on |
| `role(&str)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As an alert has them. `id` is the id of the bar, `key` the key of what stands in the page |

- It stays in the tree whether it is shown or not, so it fades in as `open(true)` shows it and out as `open(false)` takes it away (the `Panel` part's `enter` and `exit`: `ui::FADE_IN` over the theme's base duration, `ui::FADE_OUT` over its fast one). Taken away it is no height, and what is under it moves up.
- A text too long for one line goes on to the next in the room the rest leaves it; a centred one is as wide as it is, and no wider than that room.
- `sticky(true)` gives what stands in the page the engine's `Sticky::top(0.0)`: in a scroll container it stays at the top while the page scrolls under it.
- The link and the close control are stops of Tab, in that order. To what reads the tree it is an `alert` or a `status` as an alert is, called by its label or its text.
- Parts: `BannerPart::Root` (what stands in the page, sticky or not), `Panel` (the bar), `Icon`, `Label` (what it says), `Action`, `Link` (the style a link given by `link` takes over the theme's for a link), `Close`. States: `BannerState::Centered`, `Sticky`.
- In the design the bar is in the muted form of the variant's color (`Muted` for a neutral one), with corners of `radius-md`, padded 10 and 16, what is in it 12 apart and centred on a line: the icon of 16 in the variant's color, the text of 13 in `Foreground`, the link 13 and semibold in the variant's color, the close control 16 in `Foreground`.
- A renderer that draws another banner starts from `view.panel()`, or from `view.icon()`, `view.text()`, `view.action()` and `view.close()`.

## Callout

A note set apart inside prose or a form: a hint beside a setting, a caution in a page of documentation.

```rust
use lurq::ui::{AlertVariant, Callout, Link, icons};

Callout::new("Theme changes rerender only the components that read the theme.").variant(AlertVariant::Info);

// The note of a document: a bar along its start, the icon before its title.
Callout::new("Use backdrop blur for panels over busy content.")
  .title("Tip")
  .variant(AlertVariant::Success)
  .icon(icons::LIGHTBULB)
  .bar(true);

// In a form, with what the app puts under its text.
Callout::new("The key is shown once.")
  .title("Before you go on")
  .variant(AlertVariant::Warning)
  .child(Link::new("How keys are stored").external());
```

| Builder | |
| --- | --- |
| `Callout::new(&str)` | What it says. An empty text says nothing, for a callout of a title and elements |
| `title(&str)` | What it is, in a word, over its text. It is what the callout is called |
| `child(element)`, `children(..)` | Elements under the text |
| `variant(impl Into<Name>)`, `icon(impl Into<IconChoice>)` | As an [alert](#alert) has them |
| `bar(bool)` | A bar along its start in the variant's color, and the icon before a title in that color |
| `label(&str)` | What it is called to tooling, in place of its title |
| `role(&str)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As an alert has them |

- Its words are given to `new` and its title is the word over them, where an alert and a toast have a title and a `description` under it: a callout's one sentence is what it says, and "Tip" or "Note" is what it is.
- It is read, and is no stop of Tab. To what reads the tree it is a `note`, called by its label, or by its title where it has one.
- Without a bar, the icon stands at its start and the title, the text and the elements beside it. With a bar and a title, the icon and the title are a header over the text; with a bar and no title, the icon stands at the start as without one.
- Parts: `CalloutPart::Root` (the box), `Icon`, `Header` (the icon and the title, with a bar), `Text` (the title, what it says and the elements), `Title`, `Label` (what it says). States: `CalloutState::Bar`, `Title`.
- In the design the box is in the muted form of the variant's color (`Muted` for a neutral one), with corners of `radius-md`, padded 12 and 14; the icon of 16 in the variant's color is 10 from the text, 13 in `Foreground` and 1.5 high; a title is 13 and semibold. With a bar (the board's Markdown/Admonition) the bar is 3 wide in the variant's color and the corners on its side are square; the icon is 8 before a title of 14, semibold, in the variant's color, and the text of 14 is 4 under them.

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
| `separator_text(&str)` | The word between the steps of a sequence: "then" unless told |
| `label(&str)` | What the keys are called to what reads the tree, in place of their words |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [badge](#badge) has them. `id` is the id of the root |

- Keys are named as lurq names them in a `KeyboardEvent` (`Escape`, `ArrowUp`, `Backspace`, `PageUp`) or as a person writes them, in any case and with spaces or none: `Esc`, `Del`, `Page Up`. `Mod` is Ctrl, and ⌘ on a Mac; `Meta`, `Cmd`, `Super` and `Win` are ⌘ on a Mac. Arrows are drawn as ↑ ↓ ← →. A single letter is drawn in capitals; a key lurq does not name is drawn as it is written (`Up` stays `Up`).
- On a Mac the modifiers are glyphs and stand first, in the order `Accelerator::display` writes them: ⌃ ⌥ ⇧ ⌘. In words they keep the order they are given in.
- Keys are read and not operated: they take no press, no focus and no stop of Tab. To what reads the tree (`lurq_snapshot` of the MCP server) they are one `text`, called by their label or their keys in words: "Ctrl+K", "Cmd+Shift+P", "G then D".
- Parts: `KbdPart::Root` (the row of keys, its gap), `Key` (a key's box), `Label` (the text on a key, or of a plain step), `Separator` (what stands between steps). Variants: `KbdVariant`. Sizes: `KbdSize`. States: `KbdState::Sequence`, `Mac`.
- In the design a key is 20 high, padded 6, on `Muted` with an edge of 1 in `Border` and the radius `Sm`, its text 11 in the family of code in `MutedForeground`. The keys of a shortcut are 4 apart with nothing between them; large ones are 24 high, padded 7, at 12 and 6 apart, as the list of an app's shortcuts has them. Plain is the text of a menu's shortcut: 11 in the family of code in `MutedForeground`.
- A renderer that draws other keys has `view.steps` (each step's keys, with what is drawn on each and what it is called), `view.root()`, `view.key_cap(key)`, `view.label(text)`, `view.between()` and `view.written(step)` (a step as one text).
- `InlineCode::new(&str)` is a short text of code in a run of text: 13 in the family of code in `Foreground`, on `Muted` with the radius `Sm`, padded 5 across and 1 above and below. It is not cut, and is `code` to what reads the tree, called by its text. Parts: `InlineCodePart::Root`, `Label`.

## Code Block

A block of code: a snippet in a page of docs, a command to run, a file's lines. Its words are in the colors of what they are, under a bar with its name and a button that copies it.

```rust
use lurq::ui::CodeBlock;

CodeBlock::new("fn main() {\n  let app = App::new();\n}")
  .language("rust")
  .title("main.rs")
  .line_numbers(true)
  .highlight_lines([2])
  .on_copy(|code| println!("copied {} bytes", code.len()))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `CodeBlock::new(&str)` | The code. A newline at its end is not a line of its own |
| `language(&str)` | Whose tokenizer tells its words apart (see [Highlighting code](../resources-media/#highlighting-code): lurq's own for Rust, TOML, JSON, JavaScript, TypeScript, shells, Python, CSS, HTML, XML, YAML, SQL and Markdown, or the app's), what the bar shows where it has no title, and what tooling is told. A language with no tokenizer is drawn in one color |
| `tokenizer(impl Into<CodeTokenizer>)` | A tokenizer of the block's own in place of its language's: any `Fn(&str) -> Vec<CodeToken>`, a `Tokenizer`, or a `CodeLanguage` table. Made once and cloned into each render, what it gave is kept |
| `token_style(name, \|s\| ..)` | Styles the tokens of a name (`"keyword"`, `"string.escape"`) over the theme's code colors: color, weight, leaning. A part of that name in the block's recipe, so a theme's rules style it too |
| `code_scheme(CodeScheme)` | The block's own colors of code in place of the theme's: a block that looks apart, as a terminal's |
| `title(&str)`, `label(&str)` | What the bar shows in place of the language, a file's name, on one line ending in an ellipsis; its name to what reads the tree, its title or its language unless told |
| `line_numbers(bool)`, `first_line(usize)` | The numbers of the lines, in a column at their start that a selection does not take; the number of the first, 1 unless told |
| `highlight_lines(lines)` | Lines picked out by their numbers: a band in `Highlight` behind each, across the block, its number in the code's color |
| `wrap(bool)` | Wraps the lines at the block's width. Otherwise each is one line, and the code scrolls sideways under the [scroll area](#scroll-area)'s bar |
| `min_height(..)`, `max_height(..)` | The least and the greatest height, its bar with it: past the greatest, the code scrolls down and the bar stays |
| `copy_button(bool)`, `copy_label(&str)`, `copied_text(&str)`, `on_copy(Fn(&str))` | The button in the bar that copies the code (shown unless told otherwise), its name ("Copy code") and its name while it shows it copied ("Copied"); `on_copy` is told the code it put on the clipboard |
| `copy_text(&str)` | What the button copies in place of the code shown: a command shown short and copied whole. `on_copy` is told it |
| `copy_failed_text(&str)` | The button's name while it shows that the clipboard refused the copy ("Couldn't copy", the locale's `EditingWords::copy_failed`) |
| `selectable(bool)` | Whether a person selects the code: on unless told otherwise. Selectable, the code is a read-only input, a stop of Tab: the arrows move a caret through it, Shift with them selects, Ctrl+A takes it all, Ctrl+C copies, and the pointer selects across its lines. Off, it is text that what holds it makes selectable with its own (a Markdown document) |
| `framed(bool)` | Whether it is drawn in a frame of its own, as it is unless told otherwise: off, its bar and its code stand on what holds them (a card that frames them) |
| `header(bool)` | Whether it has its bar: where it has a title, a language or a copy button unless told. Told not to, the language still colors the code |
| `row_height(f32)` | How high each line is: 21 (`CODE_BLOCK_ROW_HEIGHT`) unless told, the code's own line of 17 and 4 between; the padding of 14 around the lines is kept to the glyphs of the first and the last |
| `mount(ctx)` | Mounted where it is declared, the copy button shows a check in `Success`, named `copied_text`, for two seconds (`CODE_BLOCK_COPIED_FOR`) after the clipboard took the code, or an alert in `Destructive`, named `copy_failed_text` (state `CopyFailed`), where it refused it, and a long block follows its scrolling. Converted into an element on its own it copies and does not show that it did; a long block of a greatest height is mounted as `mount` mounts it |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the copy button under the pointer or held; `id` is the block's, with `{id}-title`, `{id}-copy` and `{id}-code` |

- The bar is there where the block has a title, a language or a copy button; with none of them the code stands alone in the block.
- The code is one text in both forms, scrolling sideways and wrapped, and a selection runs across its lines and copies them with their line breaks. Its numbers stand in the input's gutter at the first row of each line as it is laid out (`components::LineGutter`), and a picked line's band covers every row it wrapped into; neither is selected or copied. The copy button copies all of it.
- A press of the copy button puts the code on the clipboard through the window it is in (`WindowHandle::copy_to_clipboard`): the OS clipboard with the `clipboard` feature, the clipboard the app gives in its place (`App::set_clipboard_writer`, so that a QA copy or a gallery never writes the person's), or what a test installs. It says it copied, or could not, by what that clipboard answers. Then `on_copy` is told.
- Its words are told apart by the tokenizer registered for its language (`lurq::code`, which Markdown's code blocks, `InlineCode` and a code field use too, without the `markdown` feature), or by its own. Each token has a dotted name (`keyword.control`, `string.escape`, `function.macro`), and is drawn in the style the block's recipe gives a part of that name, or the theme's code colors give it (`Theme::set_code_color`, `CodeScheme`), dropping the tail of a name neither has: `keyword.control.rust`, then `keyword.control`, then `keyword`, then the code's own color. The theme's colors unless told (`CodeScheme::lurq`) are the palette's roles: keywords in `Info`, as the board draws `fn` and `let`, and where the board is silent types, namespaces, tags, keys, `self` and the language's constants (`true`) in `Info`, strings in `Success`, numbers, constants and attributes in `Warning`, comments, operators and punctuation in `MutedForeground`; functions and variables in the code's own.
- A block of more than 400 lines (`CODE_BLOCK_WINDOWED_LINES`) lays out and paints the lines in view and a chunk of 64 lines on each side, and is as high as all of them: ten thousand lines draw what a screen of them draws. In view is in its own room where it has a greatest height, and in the window where it has none (a long fence of a Markdown document): it follows its room's scroll and the page's around it, and takes the lines in view before the frame is painted. It does so mounted, or as `.child(block)`; `build(theme)` draws a block with no greatest height whole, and one with a greatest height from its first line. Its tokens are kept by the tokenizer and the code, so a render that changes nothing of its code does not tokenize it again. In that form a person selects the lines drawn with the pointer, its scroll area is the stop of Tab, and select all (Ctrl+A) takes the code whole, on `Selection`, which a copy (Ctrl+C) then copies whole, all ten thousand lines; a press in the code lets it go. Its copy button copies all of it. A wrapped one counts each line's rows by its characters at its width, the figures of the family of code being as wide as each other, from its first frame.
- The code scrolls in a [scroll area](#scroll-area). Selectable, the code is the stop of Tab; otherwise the area is, called by the block's name, and its keys scroll it.
- To what reads the tree it is `code`, called by its label, its title or its language, with `a11y_language`, and so is its code (`a11y_readonly`); its copy button is a `button` called by `copy_label`, or `copied_text` while it shows it copied.
- Parts: `CodeBlockPart::Root`, `Header`, `Title`, `Copy`, `CopyIcon`, `Body`, `Code`, `LineNumber`, `Marked` (the band of a line picked out), and a part for each name of a token, by its name (`"keyword"`, `"string.escape"`): its foreground, weight and leaning are its tokens', over the theme's code colors. States: `CodeBlockState::Header`, `LineNumbers`, `Wrap`, `Copied`, `CopyFailed`, `Framed`.
- In the design the block is on `Background` in an edge of 1 in `Border` with the large radius. Its bar is 36 high on `Muted` with a line of 1 in `Border` under it, padded 14 at its start and 6 at its end; its title is 11 in the family of code in `MutedForeground`, and its copy button a ghost button of 28 with a copy glyph of 16 in `Foreground`, `Highlight` under the pointer. The code is 13 in the family of code in `Foreground`, padded 14, its lines 17 high and 4 apart. Where the board is silent: the numbers are in `MutedForeground` 16 before the code, a picked line's band is `Highlight`, and a long line scrolls sideways.
- A renderer that draws another block has `view.header()`, `view.copy_control()`, `view.body()`, `view.root()`, `view.spans(code)` (code in runs of its tokens' styles), `view.tokens` (the code's, by its tokenizer), `view.lines()`, `view.name()` and `view.heading()`, and `view.render()` the default.

## Blockquote

A quote set apart from the text around it by a line at its start: words someone said, a passage of another text, Markdown's `>`.

```rust
use lurq::{app::theme::TypographyStyle, components::Text, ui::Blockquote};

Blockquote::new("A design system is a set of decisions you no longer have to make.");

// Paragraphs, as Markdown's `>` holds them.
Blockquote::new("")
  .child(Text::new("First paragraph.").variant(TypographyStyle::Prose))
  .child(Text::new("Second paragraph.").variant(TypographyStyle::Prose));
```

| Builder | |
| --- | --- |
| `Blockquote::new(&str)` | What it says, wrapping at its width. An empty text says nothing, for a quote of elements alone |
| `child(..)`, `children(..)` | Elements under the text, 8 apart: paragraphs, a list, a block of code. What names no color of its own takes the quote's |
| `label(&str)` | What it is called to tooling: who said it, where it is from |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [badge](#badge) has them |

- It fills the width it is given. It is read and not operated: no stop of Tab. To what reads the tree it is a `blockquote`, called by its label where it has one.
- Parts: `BlockquotePart::Root` (the line, the padding), `Label` (the text), `Body` (what it holds under its text). States: `BlockquoteState::Body`.
- In the design it has a line of 2 in `Border` at its start, and is padded 16 after it and 4 above and below; its text is 14 in italics on lines of 1.5 in `MutedForeground`. The board draws no one who said it.
- A renderer that draws another quote has `view.root()`, `view.label()`, `view.body()` and `view.render()`.

## Bulleted List

A list of lines, each with a mark before it: a bullet, a number, an icon or a mark of the app's own. The features of a plan, the steps of a recipe, what a release changed; Markdown's `-` and `1.` are drawn with the same numbers.

```rust
use lurq::ui::{BulletedList, BulletedListItem, BulletedListMarker, icons};

BulletedList::new(["Semantic color roles", "Light and dark palettes"]);

// Numbers, from 4, written as the app says.
BulletedList::new(["Install", "Configure", "Run"])
  .marker(BulletedListMarker::Number)
  .start(4)
  .number_text(|number| format!("{number})"));

// An icon for every item: the name of one is a marker.
BulletedList::new(["Signed", "Notarized"]).marker(icons::CHECK);

// A list in an item, with a marker of its own.
BulletedList::new(["Tokens"]).item(
  BulletedListItem::new("Components")
    .list(BulletedList::new(["Button", "Switch"]).marker(BulletedListMarker::Number)),
);
```

| Builder | |
| --- | --- |
| `BulletedList::new(items)` | Its items: texts, or `BulletedListItem`s |
| `item(..)`, `items(..)` | Adds to them |
| `marker(impl Into<BulletedListMarker>)` | What stands before each item: `Bullet` (the default), `Number`, `Icon(name)`, or `None`, the lines alone. The name of an icon of the theme's set is a marker |
| `start(u64)` | The number of the first item of a numbered list, in place of 1 |
| `number_text(Fn(u64) -> String)` | How a numbered list writes a number, in place of `1.`: `a)`, a number in another script |
| `label(&str)` | What it is called to tooling |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [badge](#badge) has them. What is said of a list is said of the lists in its items |

| Item | |
| --- | --- |
| `BulletedListItem::new(&str)` | What it says, wrapping at the list's width. An empty text says nothing, for an item of elements alone |
| `child(..)`, `children(..)` | Elements under its line, 6 apart: a paragraph, a block of code |
| `list(BulletedList)` | A list in the item, under what it says and holds: set in to the item's text, its bullets rings, with the marker it was given |
| `icon(name)` | An icon as this item's mark, in place of the list's |
| `media(..)` | An element of the app's as this item's mark: a status dot, a picture |
| `id`, `key` | Of the item's row |

- It fills the width it is given. A mark stands in the middle of its item's first line, however many lines the item goes on to. It is read and not operated: no stop of Tab. To what reads the tree it is a `list` of `listitem`s, called by its label where it has one.
- An icon or a mark of one item's own makes an icon's room before every item of that list, so that their lines start together; an item of it with neither keeps its bullet.
- Parts: `BulletedListPart::Root` (the room between the items), `Item` (a mark and a body, and the room between them), `Mark` (the box a mark stands in, one line high: its width, and the type and color of a number), `Bullet`, `Label` (what an item says), `Body` (the line over what the item holds). States: `BulletedListState::Numbered`, `Icon`, `Nested` (it is in another list's item).
- In the design (`btfDc`) its items are 6 apart, each a dot of 5 in `MutedForeground`, 10 before its text in the body's 14 in `Foreground`. Where the board is silent it has the Markdown document's lists: a number at the end of a width of 18 in `MutedForeground`, 8 before the text, a ring for the bullet of a list in an item, an icon of 14. The numbers of a Markdown document's lists (`ThemeMarkdown::list_item_spacing`, `bullet_size`, `bullet_gap`, `list_marker_gap`, `ordered_list_marker_width`) start from the same source, so the two are one look until a theme changes one.
- A renderer that draws another list has `view.root()`, `view.item(index, item)`, `view.mark(index, icon, media)`, `view.label(text)`, `view.number(index)`, `view.line_height()` and `view.render()`.

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
| `label(&str)` | What it is called to what reads the tree, in place of its title |
| `description(&str)` | Why, or what to do, under the title |
| `icon(name)` | An icon of the theme's set over the title, in its well |
| `media(element)` | An element of the app's in place of the icon and its well: an illustration, a picture |
| `action(element)`, `actions(elements)` | Add something to do under the text, after what is there: an empty state has one or two, the main one last |
| `size(EmptyStateSize)` | `Sm` (the icon alone, 22, everything 10 apart), `Md` (a well of 40, in an edge), `Lg` (a round well of 64, a title of 18) |
| `bordered(bool)` | An edge around it. `Md` has one unless told; `Sm` and `Lg` have none |
| `width(Length)` | A width of its own in place of all of the room it is given |
| `height(Length)` | A height of its own, in which what it holds is centred: `Length::Fill` for all of the room a parent that stretches it gives |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [badge](#badge) has them. `id` is the id of the root |

- It is as wide as the room it is given and as high as what is in it, all of it centred; the title and the description wrap in the width there is. Its actions are the app's elements, as large as the place has them: a small button in a table.
- It is read; what it gives to do is operated as itself. To what reads the tree it is a `status`, called by its label or its title, with its description as `a11y_description`.
- Parts: `EmptyStatePart::Root`, `Mark` (the well), `Icon`, `Text` (the title over the description), `Title`, `Description`, `Actions`. Sizes: `EmptyStateSize`. States: `EmptyStateState::Bordered`.
- In the design the medium one is padded 32, with 16 between the well, the text and the actions, in an edge of 1 in `Border` with the radius `Xl`; the well is 40 on `Muted` with the radius `Lg`, its icon 20 in `Foreground`; the title is 14/500 in `Foreground`, the description 4 under it at 13 in `MutedForeground`; the actions are 8 apart. The small one is the empty table's: padded 32 and 16, everything 10 apart, the icon 22 in `MutedForeground` with no well. The large one is the phone's: padded 32 and 24, a round well of 64 with an icon of 28, a title of 18/600 and a description of 15, 6 apart.
- A renderer that draws another empty state has `view.root()`, `view.mark()` (the app's element or the icon in its well), `view.text()` and `view.actions()`.

## Result

What a page or a dialog says when something is over: payment received, build failed, nothing here. It is `ui::Outcome` in the code, so that `use lurq::ui::*` does not hide `std::result::Result`.

```rust
use lurq::ui::{Button, ButtonVariant, Outcome, OutcomeVariant};

Outcome::new("Build succeeded")
  .description("412 tests passed in 38 seconds.")
  .action(Button::new("View log").variant(ButtonVariant::Outline))
  .action(Button::new("Deploy"));

// On a page of its own, or in a dialog: no box.
Outcome::new("Nothing here")
  .variant(OutcomeVariant::NotFound)
  .description("This page was moved or never existed.")
  .bordered(false)
  .action(Button::new("Home"));
```

| Builder | |
| --- | --- |
| `Outcome::new(&str)` | Its title, what came of it in a line, which is what it is called |
| `variant(OutcomeVariant)` | What came of it: `Info`, `Success` (unless told), `Warning`, `Destructive` or `NotFound` (a 404), or a variant the theme's recipe names, marked with the information sign |
| `label(&str)` | What it is called to what reads the tree, in place of its title |
| `description(&str)` | More of it, under the title |
| `icon(impl Into<IconChoice>)` | Another icon of the theme's set in the mark, in the variant's colors; `None` for no mark |
| `media(element)` | An element of the app's in place of the mark |
| `child(element)`, `children(elements)` | Add what stands under the text and over the buttons, as wide as the outcome, 8 apart: a summary, a list |
| `action(element)`, `actions(elements)` | Add a button at the bottom, after those there: an outcome has several, the least first, the main one last |
| `bordered(bool)` | A box of its own with a fill and an edge, as the design draws it: true unless told |
| `width(Length)`, `height(Length)` | A width of its own in place of the design's 300; a height of its own, what it holds centred in it |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [badge](#badge) has them. `id` is the id of the root |

- Everything in it is centred; its title and description wrap in its width. With no box it has no padding either, and stands where what holds it puts it: a column that centres it on a page, the body of a [dialog](#dialog).
- It is read; its buttons are operated as themselves. To what reads the tree it is a `status`, called by its label or its title, with its description as `a11y_description`.
- Parts: `OutcomePart::Root`, `Mark`, `Icon`, `Text`, `Title`, `Description`, `Body` (what the app put under the text), `Actions`. Variants: `OutcomeVariant` (`OutcomeVariant::icon()` is the icon each is marked with). States: `OutcomeState::Bordered`.
- In the design it is 300 wide, padded 24 with 16 between the mark, the text and the buttons, on `Background` in an edge of 1 in `Border` with the radius `Lg`. The mark is a circle of 48 in the variant's muted color, its icon 24 in the variant's color: a check in `Success`, a cross in `Destructive`, a warning in `Warning`, an `i` in `Info`, and a crossed magnifier in `Foreground` on `Muted` for not found. The title is 16/600 in `Foreground`, the description 4 under it at 13 in `MutedForeground`; the buttons are 8 apart.
- A renderer that draws another outcome has `view.root()`, `view.mark()` (`None` with no icon and no media), `view.text()`, `view.body()` and `view.actions()`.

## Message Box

A dialog that asks one question or says one thing and waits for the answer: replace a file, save the changes, delete projects.

```rust
use lurq::ui::{MessageBox, MessageBoxRole, MessageBoxVariant, icons};

MessageBox::new(open.clone())
  .variant(MessageBoxVariant::Warning)
  .title("Replace existing file?")
  .description("“theme.json” already exists in this folder. Replacing it overwrites its contents.")
  .checkbox("Don't ask again", again.clone())
  .answer("Cancel", MessageBoxRole::Cancel, || {})
  .answer("Replace", MessageBoxRole::Default, move || replace_file());

// A question whose confirming answer cannot be undone: the focus goes to Cancel.
MessageBox::new(open.clone())
  .icon(icons::TRASH)
  .title("Delete 3 projects?")
  .description("This cannot be undone.")
  .answer("Cancel", MessageBoxRole::Cancel, || {})
  .answer("Delete", MessageBoxRole::Destructive, move || delete_projects());
```

| Builder | |
| --- | --- |
| `MessageBox::new(Signal<bool>)` | Shown while the signal holds `true`. The message box does not keep it: every answer sets it to `false` before what the answer does runs |
| `variant(MessageBoxVariant)` | `Info` (the default, `info`), `Question` (`help-circle`), `Warning` (`warning`), `Destructive` (`alert-circle`): the icon in the mark and their colors. A theme may add variants by name |
| `icon(impl Into<IconChoice>)` | An icon of the theme's set in place of the variant's: `trash` for a question that deletes. `None` for no icon, and no mark |
| `title(&str)`, `description(&str)` | What it asks or says, and more of it in the muted color. The title is what it is called, the description what describes it |
| `label(&str)` | What it is called to tooling, over its title |
| `detail(&str, Signal<bool>)` | A text behind "Details" under the description, open while the signal holds `true`; `detail_label(&str)` names the trigger |
| `checkbox(&str, Signal<bool>)` | A checkbox at the start of the footer: "Don't ask again" |
| `answer(&str, MessageBoxRole, handler)` | An answer. `Default`: filled, Enter gives it. `Cancel`: outlined, Escape and a press of the veil give it. `Destructive`: in the destructive fill. `Plain`: outlined, at the start of the footer |
| `dismiss_on_escape(bool)`, `dismiss_on_veil(bool)` | Whether Escape, and a press of the veil, give the cancel answer. Both on by default; with no cancel answer neither closes it |
| `width(impl Into<Length>)` | A width of its own over the board's 400 |
| `inline(bool)` | The panel where it is declared, without the veil: for a gallery, a screenshot, a test |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [dialog](#dialog) has them. `id` is the id of the panel |

- It is a [dialog](#dialog): the veil, the focus going in as it opens and back as it closes, and Tab going round its controls are the dialog's. As it opens the focus goes to the confirming answer (the default one, or else a destructive one) unless that answer is destructive, and then to the cancel one, the least destructive answer: Enter or Space does not confirm what cannot be undone. A [popconfirm](#popconfirm) asks by the same rule.
- Enter gives the default answer wherever the focus is, on the checkbox too, unless it is on another button, which Enter presses. Escape and a press of the veil give the cancel answer where there is one and do nothing where there is none.
- The plain answers stand at the start of the footer, after the checkbox, in their order; at its end the cancel answer stands first and the others after it, whatever order they were given in.
- Parts: `MessageBoxPart::Veil` and `Panel` (laid over the dialog's), `Content`, `Header` (the mark beside the text), `Mark`, `Icon`, `Text`, `Title`, `Description`, `DetailTrigger`, `DetailChevron`, `DetailLabel`, `DetailPanel`, `Detail`, `Footer`, `Option`, `Checkbox` and `CheckboxLabel` (laid over the checkbox's root and label), `Actions`, and for each role a part laid over the button's root and one over its label: `DefaultButton`, `DefaultLabel`, `CancelButton`, `CancelLabel`, `DestructiveButton`, `DestructiveLabel`, `PlainButton`, `PlainLabel`. Variants: `MessageBoxVariant`. States: `MessageBoxState::Destructive` (an answer is destructive: the icon in the destructive colors), `DetailOpen`.
- So a theme gives the two buttons of a message box fonts or fills of their own without touching a button anywhere else: `theme.patch_recipe::<MessageBox>(|rules| rules.base(MessageBoxPart::DefaultLabel, |s| s.font_family("Georgia")).base(MessageBoxPart::CancelLabel, |s| s.font_family(FontFamily::Mono)))`.
- In the design the panel is 400 wide, padded 20 with 16 between the header and the footer; the mark is 36 and round, in the variant's muted color (`InfoMuted`, `Secondary` for a question, `WarningMuted`, `DestructiveMuted`), its icon 18 in the variant's color; the title is 15/600, 14 after the mark, over the description of 13 in `MutedForeground`, 4 apart; the buttons are 32 high, padded 14, 8 apart; the checkbox's label is 13. "Details" is a chevron of 14 in the muted color 4 before its label of 13, and the detail opens 8 under it, in the mono type of 11 on the `Muted` fill, with its height over the theme's base duration.
- To what reads the tree it is an `alertdialog` called by its label or its title and described by its description.
- A renderer that draws another message box (an iOS alert: the text in the middle, the answers as cells side by side) has the pieces: `view.content()` (with Enter), `view.header()`, `view.mark()`, `view.title()`, `view.description()`, `view.find(role)`, `view.press(index)` (it closes the box and runs the answer), `view.button(index)` (the default renderer's button, which takes the focus first where it should), and `view.dialog(content)`, which puts what it built in the dialog with Escape, the veil and what tooling is told.

## Popconfirm

A question in a small panel hung from what was pressed, answered by one of two buttons: delete a file, leave a channel.

```rust
use lurq::ui::{Button, ButtonVariant, Popconfirm};

Popconfirm::new(open.clone(), anchor.clone())
  .trigger(Button::new("Delete file").variant(ButtonVariant::Outline))
  .title("Delete this file?")
  .description("It will be moved to the trash.")
  .destructive(true)
  .confirm("Delete", move || delete_file())
  .cancel("Keep", || {});
```

| Builder | |
| --- | --- |
| `Popconfirm::new(Signal<bool>, anchor)` | Shown while the signal holds `true`, hung from `anchor`. Keep the anchor from one render to the next, as for a [popover](#popover) |
| `trigger(element)` | A button that asks: it is the anchor, and a press of it turns the signal |
| `title(&str)`, `description(&str)` | The question, and more of it in the muted color |
| `icon(impl Into<IconChoice>)` | An icon of the theme's set before the title, in place of the warning sign, or `None` for no icon |
| `confirm(&str, handler)`, `cancel(&str, handler)` | The two buttons: their labels ("Yes" and "No" when not given) and what each runs after it closes the popconfirm. Escape and a press outside run the cancel one |
| `destructive(bool)` | The confirm button and the icon in the destructive color, and the focus on the cancel button as it opens |
| `placement(Placement)`, `offset(impl Into<SpacingValue>)`, `collision(CollisionStrategy)` | Where it is beside its anchor and how far from it, as a [popover](#popover) has them: under it at its start, 4 from it, by default |
| `width(..)` | How wide the panel is, over the recipe's 250 |
| `label(&str)` | What tooling calls it: its title by default |
| `on_dismiss(Fn())` | Runs after a person closed it by Escape or a press outside, after what the cancel button runs; not on a press of either button, nor when the app sets the signal. Each handler given runs, in order |
| `dismiss_on_escape(bool)`, `dismiss_on_outside_press(bool)` | Whether Escape, and a press outside it and its anchor, close it and cancel: both `true` by default |
| `inline(bool)` | Draws the panel where it is declared, open or not, without its trigger: for a gallery, a screenshot, a test. Its buttons still answer |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [popover](#popover) has them. `id` is the id of the panel |

- The title and the description wrap. Buttons too long to stand side by side go one under the other, the confirm button under the cancel one, both at the end. A button's label wider than the panel is cut with an ellipsis inside it.
- It is a [popover](#popover): kept in the window, gone to the other side where there is no room, the focus back on the anchor as it closes. As it opens the focus goes to the confirm button; where it is destructive, to the cancel button, the least destructive answer, so that Enter or Space does not remove what was not read. A message box puts the focus by the same rule.
- Escape and a press outside close it and run what the cancel button runs, then what `on_dismiss` was given. A button closes it before its handler runs.
- Parts: `PopconfirmPart::Panel` and `Body` (laid over the popover's panel and body), `Content`, `Header` (the row of the icon and the text), `Icon`, `Text`, `Title`, `Description`, `Footer` (the row of the buttons), `CancelButton`, `CancelLabel`, `ConfirmButton`, `ConfirmLabel`. States: `PopconfirmState::Destructive`.
- In the design the panel is 250 wide, padded 12 with 12 between the header and the buttons, in the popover's colors, edge, large radius and medium shadow; the icon is the warning sign of 16 in `Warning`, 8 before the title of 13/500 over the description of 12 in `MutedForeground`, 2 apart; the buttons are 28 high, padded 10, their labels 13, 8 apart at the end.
- To what reads the tree it is an `alertdialog` called by its label or its title and described by its description.
- A theme's renderer builds what the panel holds from `PopconfirmView` (`header()`, `cancel()`, `confirm()`, `footer()`, `content()` with Escape, `confirmer()`, `canceller()`, `dismisser()`); where the panel is shown, and when, stays the popover's.

## Progress Dialog

A dialog over the window while a piece of work runs: what is being done, how far it is, and a way to stop it.

```rust
use lurq::ui::{ProgressDialog, icons};

ProgressDialog::new(open.clone())
  .icon(icons::COPY)
  .title("Copying 128 items")
  .description("from Downloads to projects/lurq/assets")
  .value(copied.get())
  .max(128.0)
  .format(|done, all| format!("{done} of {all}"))
  .status("about 2 min left")
  .detail(&current_file.get(), details.clone())
  .button("Pause", move || pause())
  .on_cancel(move || stop());
```

| Builder | |
| --- | --- |
| `ProgressDialog::new(Signal<bool>)` | Shown while the signal holds `true`. It closes when the app sets it to `false`: Escape and a press of the veil do not close it |
| `icon(&str)`, `title(&str)`, `description(&str)` | What kind of work it is, what is being done (what it is called), and a line more of it in the muted color |
| `label(&str)` | What it is called to tooling, over its title |
| `value(f64)`, `max(f64)` | How far the work is, a fraction of one unless `max` says otherwise. Without a value its end is not known, and the bar runs a segment across it |
| `format(Fn(f32, f32) -> String)` | Says how much is done under the start of the bar: "42 of 128". A percentage when not given |
| `status(&str)` | What the app says of the rest, under the end of the bar |
| `detail(&str, Signal<bool>)` | A text behind "Details" at the start of the footer, open under the footer while the signal holds `true`; `detail_label(&str)` names the trigger |
| `button(&str, handler)` | Adds a button before the cancel one: "Pause". It does not close the dialog |
| `on_cancel(handler)`, `cancel_label(&str)` | A cancel button, which has the focus as the dialog opens and which Escape presses. It does not close the dialog: the app does once the work has stopped. A second handler runs after the first |
| `width(impl Into<Length>)`, `inline(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [message box](#message-box) has them |

- The title and the description wrap. The count is never cut; a status longer than what the count leaves is cut with an ellipsis on one line.
- It is a [dialog](#dialog) with its veil, its focus and its Tab, which nothing but the app closes.
- Parts: `ProgressDialogPart::Veil` and `Panel` (laid over the dialog's), `Content`, `Header`, `Icon`, `Text`, `Title`, `Description`, `Progress` (laid over the [progress](#progress)'s root), `Meta`, `Count`, `Status`, `Footer`, `DetailTrigger`, `DetailChevron`, `DetailLabel`, `DetailPanel`, `Detail`, `Actions`, `ActionButton`, `ActionLabel`, `CancelButton`, `CancelLabel`. States: `ProgressDialogState::Indeterminate`, `DetailOpen`.
- In the design the panel is 420 wide, padded 20 with 12 between its parts; the icon is 20, 12 before the title of 15/600 over the description of 12 in `MutedForeground`, 2 apart; the bar is the progress's of 6 across the panel; what is done and what is left are in the mono type of 11 in `MutedForeground`; the footer, 4 further down, has "Details" at its start and the outlined buttons of 32, padded 14, 8 apart, at its end.
- To what reads the tree it is a `dialog` called by its label or its title and described by its description, with the `progressbar` in it.

## Sheet

A panel that slides in from an edge of the window, over the page, for what goes with the page without leaving it: edit a profile, filter a list, share a document.

```rust
use lurq::{
  components::SheetDetent,
  ui::{Button, ButtonVariant, Input, Sheet, SheetSide, SheetSize},
};

// open: Signal<bool>, the app's.
let close = open.clone();
Sheet::new(open.clone())
  .title("Edit profile")
  .description("Changes save when you close the panel.")
  .close_button(true)
  .child(Input::new(name.clone()))
  .footer(Button::new("Close").variant(ButtonVariant::Outline).on_click(move |_| close.set(false)))
  .footer(Button::new("Save"));

// From the start, wider.
Sheet::new(filters_open.clone()).side(SheetSide::Start).size(SheetSize::Md).title("Filters").children(filters);

// At the bottom, mounted: a finger drags it between its heights and down to put it away.
Sheet::new(goal_open.clone())
  .side(SheetSide::Bottom)
  .title("Move goal")
  .detents([SheetDetent::Content, SheetDetent::Large])
  .footer(Button::new("Submit"))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<bool>)` | Shown while the signal holds `true`. The sheet does not keep it: Escape, a press on the veil and the close button set it to `false` |
| `side(SheetSide)` | The edge it comes from: `End` (the default) and `Start` as high as the window, `Top` and `Bottom` as wide as it |
| `size(SheetSize)` | Across from its edge: `Sm` 320, `Md` 400, `Lg` 540 wide at a side, `Sm` 240, `Md` 360, `Lg` 480 high at the top or the bottom, `Full` the window. Without one it is 320 wide at a side and as high as what it holds at the top or the bottom. Never larger than the window |
| `title(&str)`, `description(&str)` | What it is, and more of it under the title. The title is what the sheet is called |
| `label(&str)` | What the sheet is called to tooling where it has no title |
| `child(element)`, `children(..)` | The body, which scrolls between the header and the footer |
| `footer(element)` | Adds to the footer, the least action first and the main one last: a row at the panel's end under a line, or at the bottom a column of buttons as wide as the sheet with the main one on top |
| `close_button(bool)`, `close_label(&str)` | A ghost button with the close icon at the end of the header, and what it is called to tooling: "Close" when not given |
| `grabber(bool)` | The handle at the top. A sheet at the bottom has it unless told otherwise |
| `dismiss_on_veil(bool)`, `dismiss_on_escape(bool)` | Whether a press on the veil (and, mounted, a drag down), and Escape, close it. Both on by default |
| `on_dismiss(Fn())` | Runs after a person closed it: a press on the veil, the close button, Escape wherever the focus is (the engine's `Modal::on_dismiss`), and for a mounted sheet at the bottom a finger that drags it down and away or taps beside it (`SheetProps::on_dismiss`). A button of the app's that closes it, or the app setting the signal, does not run it. A second handler runs after the first |
| `detents([SheetDetent])`, `detent(Signal<usize>)`, `scroll_state(ScrollState)` | For a mounted sheet at the bottom: the heights it stands at, which it is at, and the body's scrolling, which hands a finger to the sheet once the body is at its top |
| `body_scrolls(bool)` | Whether the body scrolls. It does unless told; one that does not is laid out in the room the panel has for it, at a side the panel's height less the header and the footer, for what holds a scroll of its own. A drawer over the page with `body_scrolls(false)` gives its sheet this |
| `inline(bool)` | The panel where it is declared, without the veil and without taking the window: for a gallery, a screenshot, a test |
| `mount(ctx)` | At the bottom, the engine's draggable `Sheet` in this look; at another edge, the same as converting it |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the close button as under the pointer, pressed or focused; `id` is the panel's; `key` tells two sheets of one component apart |

- A title or a description longer than the panel wraps inside its padding on every side, centered where the sheet stands at the bottom and the top; neither is cut. A [drawer](#drawer) over the page is a sheet, and wraps the same.
- It is the engine's `Modal` with `target(Root)` and `takes_focus(true)`, as a [dialog](#dialog) is: declared anywhere, it takes no room there and covers the window under the window's own title bar while it is open. As it opens the focus goes to its first control (the close button where it has one), and as it closes back to what had it. Nothing under the veil takes the pointer, and Tab and Shift+Tab go round the sheet's own controls. A press on the panel goes no further.
- It slides in from its edge over the theme's slow duration in the entering curve as the veil fades in, and out to it over the base duration in the leaving curve: keyframes that move the panel by its own width or height (`ui::SHEET_IN_FROM_START` .. `ui::SHEET_OUT_TO_BOTTOM`). With motion reduced (`Theme::reduced_motion`, state `Modal` with `ReducedMotion`) it fades where it stands.
- At the start and the end the body takes the room the header and the footer leave, so the footer stands at the panel's bottom; at the top and the bottom the sheet is as high as what it holds while that fits, and the body gives way and scrolls when it does not.
- `mount(ctx)` puts a sheet at the bottom on the engine's [sheet](../modals/#sheets): a finger drags it between its `detents` (or the height its size gives it, or its content's) and throws it down to close it, it rises over the on-screen keyboard and keeps clear of a phone's edges, and it comes up with a spring. The engine lays out the panel and the backdrop; the recipe's `Panel` and `Veil` dress them (`SheetProps::dress`), and what the panel holds is the sheet's. A theme's renderer builds the sheet that is converted, not the mounted one, which is made of the view's parts.
- Parts: `SheetPart::Veil`, `Panel`, `Grabber` (the strip at the top of a bottom sheet), `Handle` (its bar), `Header` (the text and the close button), `Text` (the title over the description), `Title`, `Description`, `Body`, `Footer`, `Close` (a [button](#button) restyled), `Separator` (the [separator](#separator) over the footer, restyled). Variants: `SheetSide`. Sizes: `SheetSize`. States: `SheetState::Modal` (over the window, not inline), `ReducedMotion`.
- In the design a sheet at a side is in `Background` with a line of 1 in `Border` along the edge it shows the page and the large shadow. The header is padded 20 at the top and the sides, the title 16/600 in `Foreground` over the description in 13 `MutedForeground`, 4 apart, the close button's icon of 16 in `MutedForeground` at its end; the body is padded 20 with 16 between what it holds; the footer is padded 16 and 20, its buttons 8 apart at the end, under a line of `Border`. At the top the line is along its bottom. At the bottom it is the board's drawer: a line all round, corners of `radius-xl` (12) at its top, a handle of 40 by 4 in `Muted` 8 under its top edge, the text in the middle 20 under it, and its buttons as wide as it, 8 apart, 20 from the edges. The veil is `Scrim`.
- The board's phone and tablet sheets are the same parts restyled: the phone's bottom sheet a panel of 360 round 20 at its top with a handle of 36 in `Input`, its title 18 and its description 14, padded 16; the tablet's form sheet a panel of 520 by 360 round 16 with an edge all round. The gallery's page has them.
- To what reads the tree (`lurq_snapshot` of the MCP server) the panel is a `dialog` called by its title, or by its label.
- A renderer that draws another sheet has `view.panel()`, `view.grabber()`, `view.text()`, `view.close()`, `view.header()`, `view.body()`, `view.footer()`, `view.separator()`, `view.contents()` (all of them in order), `view.veil(panel)` and `view.modal(panel)`.

## Drawer

A panel at one side of a layout, beside the page rather than over it, that opens and closes: the navigation of an app, a pane of details or of filters.

```rust
use lurq::{
  components::Row,
  ui::{Drawer, SheetSide},
};

// open: Signal<bool>, the app's. The drawer pushes the page aside.
Row::new()
  .child(Drawer::new(open.clone()).label("Navigation").children(places))
  .child(page);

// On a narrow window it lies over the page with a veil.
Drawer::new(open.clone()).label("Navigation").overlay(narrow).children(places);
```

| Builder | |
| --- | --- |
| `new(Signal<bool>)` | Open while the signal holds `true`. The app's: over the page, Escape and a press on the veil set it to `false` |
| `side(SheetSide)` | The side of the layout it stands at: `Start` (the default) and `End` open with its width, `Top` and `Bottom` with the height of what it holds |
| `label(&str)` | What it is called to tooling |
| `child(element)`, `children(..)` | What it holds, which scrolls where it is too small |
| `width(impl Into<Length>)`, `height(impl Into<Length>)` | A width of its own at a side (300 without one), a height of its own at the top or the bottom |
| `overlay(bool)` | Over the page with a veil, as a [sheet](#sheet) at its edge, rather than beside it: for a window too narrow to push the page aside |
| `body_scrolls(bool)` | Whether what it holds scrolls where it is too small. It does unless told; what does not is laid out at the panel's height, beside the page and over it, for what holds a scroll of its own with something that stays beside it: a [sidebar](#sidebar) with its header and its footer |
| `dismiss_on_veil(bool)`, `dismiss_on_escape(bool)` | Over the page, whether a press on the veil, and Escape, close it. Both on by default |
| `on_dismiss(Fn())` | Over the page, runs after a person closed it by the veil or Escape, as a [sheet](#sheet)'s does. A second handler runs after the first |
| `inline(bool)` | Over the page, its panel where it is declared, without the veil and without taking the window: for a gallery, a screenshot, a test. A drawer beside the page is where it is declared already |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's |

- It is declared in the layout where it stands, at the start or the end of a row or the top or the bottom of a column. As it opens its room grows from nothing to the panel's over the theme's slow duration and pushes the page aside, and the panel, which keeps its size, comes out from the layout's edge and fades in; as it closes the room closes over it as it fades out, and what it held is gone, so that Tab does not reach it. With motion reduced it opens and closes at once.
- `overlay(true)` builds a [sheet](#sheet) at the drawer's edge, as large across as the drawer's panel, holding what the drawer holds laid out as in the drawer: it takes no room in the layout, slides in over a veil, and is a dialog to tooling.
- Parts: `DrawerPart::Root` (the room it takes, with the transition of its width or height), `Panel`, `Body`. Variants: `SheetSide`. States: `DrawerState::Open`.
- In the design (the board's navigation drawer) the panel is 300 wide in `Background` with a line of 1 in `Border` along the edge it shows the page; what it holds pads itself.
- To what reads the tree it is a `complementary` region called by its label.
- A renderer that draws another drawer has `view.body()`, `view.panel()` and `view.root(panel)`, and `view.is_open`.

## Hover Card

More about what the pointer rests on, a person, a link, a file, in a panel beside it. Unlike a tooltip it holds what a person reads and presses.

```rust
use lurq::{
  components::Placement,
  ui::{Avatar, Button, ButtonSize, HoverCard, Link, icons},
};

// In a component's render: a hover card is mounted on the element it is about, and returns that element.
let author = HoverCard::new()
  .media(Avatar::new("LQ"))
  .title("@lurq")
  .description("A Rust UI toolkit with GPU-backed rendering.")
  .fact("Joined March 2024", icons::CALENDAR_DAYS)
  .mount(ctx, Link::new("@lurq"));

// With controls in it: a dialog to tooling, and Tab goes into it from the link.
let reviewer = HoverCard::new()
  .title("Ivan Zaida")
  .child(Button::new("Follow").size(ButtonSize::Sm))
  .placement(Placement::Right)
  .arrow(true)
  .mount(ctx, Link::new("Ivan Zaida"));
```

| Builder | |
| --- | --- |
| `mount(ctx, element)` | Puts the card on the element it is about and returns that element |
| `title(&str)`, `description(&str)` | What it is about, and a line or two about it |
| `label(&str)` | What tooling calls it, in place of its title |
| `media(element)` | What stands at its start, beside the text: an avatar, the icon of a file |
| `fact(&str, icon)`, `facts(..)` | A fact in the row under the description: its text, and an icon of the theme's set before it or `None` |
| `child(element)`, `children(..)` | What it holds under its text: a link, a row of buttons |
| `placement(Placement)` | The side of the element it is on and the edge it lines up with: `Bottom`, centered, by default |
| `offset(..)`, `collision(CollisionStrategy)` | How far it stands from the element in place of the theme's 8, and what it does where there is no room on its side: `FlipThenClamp` by default |
| `arrow(bool)` | A triangle with the panel's edge from the panel to the element |
| `open_delay(..)`, `close_delay(..)` | A `MotionDuration` of the theme's or a `Duration`: how long the pointer rests before it opens (`Slower`, 400 ms) and how long it stays after the pointer left (`Slow`, 240 ms) |
| `force_open(Option<bool>)` | `Some(true)` shows it whatever the pointer and the focus do, `Some(false)` never |
| `disabled(bool)` | It never opens, and the element is as it would be without it |
| `pass_through(bool)` | The pointer passes through the card to what is under it, and the card goes as the pointer leaves the element, with no close delay: a card of words over a status bar's item. Off by default |
| `width(..)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [popover](#popover) has them. `id` is the id of what shows |

- It opens when the pointer has rested on the element for its open delay, and at once when the keyboard's focus comes to the element or to anything in it. It stays open while the pointer is on the element, on the card or crossing the room between them (the room is the card's root, which the pointer is on), and while the focus is in either; it closes when the pointer has left both for its close delay, when the focus leaves both, and on Escape, which it takes before anything under it. After Escape it stays closed until the pointer comes again or both the pointer and the focus have left.
- A card that the pointer passes through (`pass_through(true)`) is hit by nothing: what it covers is hovered and pressed through it, and it goes the moment the pointer leaves the element. The focus and Escape show and put it away as they do any card.
- Tab goes from the element into the card's controls and on to what follows the element (`Overlay::tab_after_anchor`): the focus is not moved into it as it opens, which is what `Overlay::takes_focus` would do. As it closes with the focus inside, the focus goes back to the element.
- One hover card of the pointer shows at a time. Where there is no room on its side it goes to the other, its arrow turned with it, and where the window pushes it along, its arrow still points at the middle of the element.
- To what reads the tree it is a `dialog` where it holds a control and a `tooltip` where it does not, called by its label or else its title; the element is described by the title and the description (`a11y_description`), and says it has a dialog where the card has controls.
- It is a mounted component, as the tooltip is. Converted into an element on its own, a `HoverCard` is its panel as it shows, anchored to nothing. As a tooltip, it has no `on_dismiss` and no `dismiss_on_escape`: the pointer and the focus open it, not the app, and Escape always closes it.
- Parts: `HoverCardPart::Root` (the panel and its arrow, and the room between them and the element), `Panel`, `Media`, `Text`, `Title`, `Description`, `Facts`, `Fact`, `FactIcon`, `Arrow`. States: `HoverCardState::Arrow`, `Controls`, and the side it is on by the names of `TooltipSide` (`"top"`, `"bottom"`, `"left"`, `"right"`).
- In the design the panel is 300 wide on `Popover` in an edge of 1 in `Border`, `Lg` round, with the `Md` shadow and 16 of padding, the media 12 before the text. The text is 6 apart: the title in 14 at medium weight, the description in 13 of `MutedForeground` in lines of 1.5, and the facts 12 apart, each an icon of 14 6 before its text in 12, both muted. It stands 8 from the element; the arrow is 18 along the panel and 9 out from it. It fades in over the base duration and out over the fast one.

## Toggletip

A tooltip opened by a press: a small button beside a label that opens a few words about it, for what must be reachable by a finger and told by a screen reader.

```rust
use lurq::ui::Toggletip;

let help = Toggletip::new("What is a workspace?")
  .text("A workspace holds the projects a team shares.")
  .link("Learn more", || open_docs())
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `Toggletip::new(&str)` | What its button is called to tooling: the question the panel answers |
| `mount(ctx)` | Puts its button where it goes, and its panel over the window while it is open |
| `text(&str)` | What the panel says |
| `link(&str, Fn())` | A link under the text, with an arrow after it, and what a press of it does |
| `child(element)`, `children(..)` | What the panel holds under its text: a button |
| `icon(name)` | The button's icon, `icons::INFO` unless this says otherwise |
| `placement(Placement)` | The side of the button the panel is on and the edge it lines up with: `Top`, centered, by default |
| `offset(..)`, `collision(CollisionStrategy)` | How far the panel stands from the button in place of the theme's 6, and what it does where there is no room on its side: `FlipThenClamp` by default |
| `arrow(bool)` | A triangle from the panel to the button: the tooltip's |
| `on_dismiss(Fn())` | Told after a person closed it: another press of its button, a press outside it, Escape. A second handler is called after the first |
| `dismiss_on_escape(bool)`, `dismiss_on_outside_press(bool)` | Whether Escape, and a press outside it and its button, close it: both on by default |
| `force_open(Option<bool>)` | `Some(true)` shows the panel whatever the button says, `Some(false)` never |
| `disabled(bool)` | The button takes nothing |
| `force(Option<Interaction>)` | Draws its button hovered, pressed or focused with nothing on it |
| `width(..)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `key` | As a [popover](#popover) has them |
| `id` | The id of the button; the root of the panel is `<id>-panel` |

- A press of the button opens the panel and another closes it; a press outside it and Escape close it too, and each of the three tells `on_dismiss` once it is closed. The focus stays on the button as it opens, and Tab goes from the button into the panel's link and on; as the panel closes with the focus in it, the focus goes back to the button.
- To what reads the tree the button is called by its label and says whether it is expanded (`a11y_expanded`), and the panel is a `status`.
- It is a mounted component: it keeps whether it is open. Converted into an element on its own, a `Toggletip` is its panel as it shows.
- Parts: `ToggletipPart::Root`, `Trigger` and `TriggerIcon` (what it gives the theme's button and its icon), `Panel`, `Label` (its text), `Link` and `LinkLabel` (what it gives the theme's link and its label), `Arrow`. States: `ToggletipState::Open`, `Arrow`, `Disabled`, and the side as a hover card has it.
- In the design the panel is 230 wide on `Primary`, `Md` round, with 12 of padding and 8 between what it holds. Its text is 13 in `PrimaryForeground` in lines of 1.4, and its link 13 at 600 with an arrow up and right of 14, underlined in `PrimaryForeground` under the pointer. The button is a ghost one of 20 that is a circle, its icon of 14 in `MutedForeground`, in `Foreground` under the pointer and while the panel is open. The panel stands 6 from the button, and its arrow is the tooltip's, 12 along and 6 out.

## Teaching Tip

A panel that points at one thing in the app and says what it is for, the first time a person comes to it or when it is new, without taking the focus from what they are doing. A tour shows several, one after another.

```rust
use lurq::ui::{Icon, TeachingTip, TeachingTipVariant, Tour, TourStep, icons};

// open: Signal<bool>, anchor: ElementRef, kept by the app.
let close = open.clone();
let dock = TeachingTip::new(open.clone(), anchor.clone())
  .media(Icon::new(icons::SPARKLES).size(24.0))
  .title("Dock panels anywhere")
  .description("Drag a panel by its header and drop it on any edge of the window.")
  .primary("Got it", move || close.set(false))
  .mount(ctx);

// step: Signal<Option<usize>>, which step shows; None while the tour is not on.
let tour = Tour::new(step.clone())
  .step(TourStep::new(theme_button, "Switch themes here").description("Light and dark palettes share the same roles."))
  .step(TourStep::new(panels, "Dock panels anywhere"))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `TeachingTip::new(open, anchor)` | Pointing at `anchor` while the `Signal<bool>` is `true` |
| `mount(ctx)` | Puts it in the app; the element it returns has no size and goes anywhere |
| `title(&str)`, `description(&str)` | What it points at, and what it is for |
| `label(&str)` | What tooling calls it, in place of its title |
| `media(element)` | A picture or an icon at its top, on a muted fill as wide as it |
| `child(element)`, `children(..)` | What it holds under the description |
| `step(n, of)` | That it is a step of several: "2 of 4" in its footer, in the tour's look |
| `primary(&str, Fn())`, `secondary(&str, Fn())` | The button that goes on, and the one before it, and what a press of each does |
| `tertiary(&str, Fn())` | A quiet button before them, "Skip", "Remind me later", and what a press of it does |
| `on_dismiss(Fn())` | Told after its close control, Escape or a press outside it closed it: the signal is `false` already. A second handler is called after the first |
| `dismiss_on_escape(bool)` | Whether Escape closes it, as its close control does: on by default |
| `dismiss_on_outside_press(bool)` | Whether a press outside it closes it: off by default, since a press elsewhere is the person going on with what they do beside it |
| `close_button(bool)` | A close control at the end of its title: on by default |
| `variant(TeachingTipVariant)` | `Plain` (the default), in the popover's colors, or `Primary` |
| `placement(Placement)` | The side of what it points at it is on and the edge it lines up with: `Bottom`, centered, by default |
| `offset(..)`, `collision(CollisionStrategy)` | How far it stands from what it points at in place of the theme's 4, and what it does where there is no room on its side: `FlipThenClamp` by default |
| `arrow(bool)` | A triangle with the panel's edge pointing at what it is about: on by default |
| `takes_focus(bool)` | The focus goes into it as it opens and back as it closes. Off by default |
| `traps_tab(bool)` | Tab stays in it while the focus is in it, going round from its last button to its first. Off by default; a tour turns it on |
| `force(Option<Interaction>)` | Draws its close control hovered, pressed or focused with nothing on it |
| `width(..)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `key` | As a [popover](#popover) has them |
| `id` | The id of what shows; its close control is `<id>-close`, its buttons `<id>-primary`, `<id>-secondary` and `<id>-tertiary` |

- It does not keep whether it is open: the app's signal does. Its close control and Escape close it: they set the signal to `false` and then tell `on_dismiss`, so that a handler that opens something else does not have it closed under it. A press elsewhere does not close it, so a person can go on with what they are doing beside it. Its own buttons do what their handlers say.
- It does not take the focus unless the app says so: Tab goes from what it points at into its buttons and on (`Overlay::tab_after_anchor`). With `takes_focus(true)` the focus goes into it as it opens and back to what had it as it closes.
- To what reads the tree it is a `dialog` called by its label or else its title. Where there is no room on its side it goes to the other, its arrow turned with it, and the arrow points at the middle of what it is about.
- `Tour::new(Signal<Option<usize>>)` shows the step the signal says, counted from 0, as a teaching tip with its count and no close control, with "Back" from the second step on and "Next", or "Done" on the last (`back_label`, `next_label`, `done_label`). Its buttons set the signal; Done and Escape set it to `None`, and then Done tells `on_finish` and Escape `on_dismiss` (`dismiss_on_escape(false)` keeps the tour on; `dismiss_on_outside_press(true)` leaves it on a press outside the tip, as Escape does). `skip_button(true)` gives every step but the last a quiet "Skip" before the others (`skip_label`), which leaves as Escape does; the design draws none, so it is off by default. `TourStep::new(anchor, title)` has `description`, `media`, `placement`, `id` and `key`; the tour has `variant`, `placement`, `offset`, `collision` and `arrow` for all its steps, and `state`, `style`, `rules`, `id` and `key` for the tip that shows, a step's `id` and `key` first.
- `Tour::controlled(Option<usize>, on_change)` is given the step as a value the app holds: a press of its buttons, its keys and Escape tell `on_change` the step it is asked to be at, and it stays where the app holds it. Bound, `on_change(Fn(Option<usize>))` is told the step after the signal is set, `None` as the tour ends or is left, before `on_finish` and `on_dismiss`; it is not told when the app sets the signal.
- A tour takes the focus (`takes_focus(false)` leaves it where it is, as a teaching tip does): into its card as it starts, on the button that goes on whatever stands before it, on the button that was pressed from step to step, on "Next" where "Back" is gone with the first step, and back to what had it before the tour as it ends. Tab is held in the card (`traps_tab(false)` lets it go on to what follows what the step points at): from its last button it goes round to its first, and Shift+Tab the other way; the page behind still answers the pointer. While the focus is in the card the Right arrow goes on a step and the Left one back, Home and End go to the first and the last step, and none of them ends the tour: Done and Escape do. The card of all its steps is one overlay that goes from one element to the next (see [Popups And Outside Presses](../modals/#popups-and-outside-presses)).
- Teaching tips and tours are mounted components: a tip keeps which side the window let it take, and what Escape tells. Converted into an element on its own, a `TeachingTip` is its panel as it shows, and a `Tour` the panel of the step its signal says, or of its first step while it says none.
- Parts: `TeachingTipPart::Root`, `Panel`, `Media`, `Content` (the text over the footer), `Text`, `Header` (the title and the close control), `Title`, `Close` (what it gives the theme's button), `Description`, `Footer`, `Step`, `Actions`, `PrimaryAction`, `SecondaryAction` and `TertiaryAction` (what it gives the theme's buttons), `Arrow`. Variants: `TeachingTipVariant::Plain`, `Primary`. States: `TeachingTipState::Arrow`, `Step` (it has a count), `Media`, and the side as a hover card has it.
- In the design a teaching tip is 300 wide on `Popover` in an edge of 1 in `Border`, `Lg` round, with the `Lg` shadow: media 96 high on `Muted`, then 14 of padding around the text and the footer 10 under it. The title is 14 at 600 with a ghost close control of 20 at its end, its icon of 16 in `MutedForeground`; the description 6 under it is 13 of `MutedForeground` in lines of 1.45; the buttons, 8 apart at the end of the footer, are 30 high and 12 from their sides, the primary one `Primary` and the other `Outline`. It stands 4 from what it points at; its arrow is 18 along the panel and 9 out from it, with the panel's edge. A step of a tour is the board's Tour: 280 wide with the `Md` shadow, padded 16, its title at 500 over the description 4 under it in lines of 1.5, its footer 12 under them with the count in mono 11 of `MutedForeground` at its start and buttons 28 high and 10 from their sides. In the `Primary` look the panel is `Primary` in its own color's edge, all it says in `PrimaryForeground`, the media on a white of 10%, and the buttons `Secondary` and `Ghost`. It fades in and out as the popover does.

## Notification

Something that happened which a person may act on now or later: who or what it is from, a title and a line, when, what can be done about it, whether it was read. A card on its own, and a row of a list of them.

```rust
use lurq::ui::{Avatar, Notification, NotificationVariant};

// The design's card: the bell, a title over a line, a button, and what dismisses it.
Notification::new("Review requested")
  .description("Ivy asked you to review feat/node-blur.")
  .button("Review", move || open_review())
  .on_close(move || dismiss(id));

// A row of a list: who it is from, when, and that it was not read.
Notification::new("Ivy Chen requested a review")
  .description("feat/node-blur")
  .media(Avatar::new("Ivy Chen"))
  .time("2m")
  .unread(true)
  .variant(NotificationVariant::Row)
  .on_click(move |_| open(id));
```

| Builder | |
| --- | --- |
| `Notification::new(&str)` | The title: what happened. It is what the notification is called |
| `description(&str)` | More of it, under the title |
| `media(element)` | Who or what it is from, before the text in place of the icon: an `Avatar`, an app's mark |
| `icon(impl Into<IconChoice>)` | An icon of the theme's set in place of the bell, or none (`icon(None)`) |
| `time(&str)` | When it happened, as the app words it: "2m" |
| `unread(bool)` | Whether it was not read: it has the dot, and a row's title is medium |
| `button(&str, Fn())` | Adds a button under the text: the first is the primary one, the others ghosts |
| `action(element)` | Adds something of the app's own to the actions: a link, a menu's button. It adds, since a notification has several |
| `child(element)`, `children(..)` | Elements under the description, for more than text |
| `variant(impl Into<Name>)` | A `NotificationVariant`: `Card` (the default), with an edge and a shadow, or `Row`, as a list draws each |
| `on_click(handler)` | A press of the whole notification: it opens what it is about. A second handler is called after the first |
| `on_close(Fn())` | Called on a press of what dismisses it, which it has once it is given `on_close`: a "Dismiss" button after a card's own, a close control at the end of a row. The app takes the notification out of what it lists |
| `close_label(&str)`, `unread_label(&str)` | What dismisses it says and is called ("Dismiss"), and what its dot is called to tooling ("Unread") |
| `label(&str)` | What it is called to tooling, in place of its title |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the notification's states where it is pressed, and else those of a row's close control; `id` is the id of the box, with `<id>-close` and `<id>-action-0`, `<id>-action-1` for what is in it |

- A notification stays: it is there until the app takes it out of what it lists. A [toast](#toast) leaves by itself and is shown over the window from anywhere; an [alert](#alert), a [banner](#banner) and a [callout](#callout) are part of the page they stand in and say something about it. It keeps nothing: whether it was read and whether it is listed are the app's, and it tells what a person did.
- Given `on_click` it is pressed as a whole: a button to what reads the tree, a stop of Tab with the ring inside it, pressed by Enter and Space. Its own buttons and what dismisses it are pressed apart from it, each a stop of Tab after it. Without `on_click` it is an `article` that takes no focus and answers no pointer.
- A title and a description longer than their room wrap; the time stays whole on one line; buttons that do not fit side by side go on to the next line. A card is 340 wide at most and as wide as a narrower room.
- A row follows the theme's density: 56 high at the default, 48 compact, 64 comfortable and 68 under a finger. A row that was read keeps the room of the dot, so what stands after it lines up down a list.
- Parts: `NotificationPart::Root`, `Unread` (the dot), `Media`, `Icon`, `Content` (the text over the body and the actions), `Text`, `Header` (the title, and on a card the time and the dot), `Title`, `Time`, `Description`, `Body`, `Actions`, `Action` (what it gives the theme's buttons), `Close` (a row's close control). Variants: `NotificationVariant::Card`, `Row`. States: `NotificationState::Unread`, `Pressable`.
- In the design the card is 340 wide on `Popover` in an edge of 1 in `Border`, `Lg` round, with the `Md` shadow, padded 16: the bell of 16 in `Foreground`, 12 from the title (14, medium) over the description (13, `MutedForeground`, lines of 1.4) 4 under it, and 10 under them the buttons, 28 high, 10 from their sides and 8 apart. The row is 12 from its top, its bottom and its start and 16 from its end, with 10 between the dot of 7 in `Info`, the avatar of 32, the text (the title 13 over the description of 12 in `MutedForeground`, 1 apart) and the time in mono 11 in `MutedForeground`; under the pointer it is in `Highlight`, as the board draws its first row. The board draws no time and no dot on a card: they stand after its title. It draws no pressed card and no focus: a pressed card has the veils of those states.
- A renderer that draws another notification starts from `view.media()`, `view.text()`, `view.content()`, `view.actions()`, `view.close()`, `view.unread_dot()`, `view.time()` and `view.root(children)` (the behavior), with `view.row`, `view.unread` and `view.close_button`.

## Notification Center

The panel that lists an app's notifications, opened from its bell: a header with what marks all read or clears, the notifications as rows, and a footer that leads to all of them.

```rust
use lurq::ui::{Avatar, Badge, Button, ButtonVariant, Notification, NotificationCenter, icons};

// open: Signal<bool> and bell: ElementRef, kept by the app; listed: Signal<Vec<(Id, bool)>>, its list.
let unread = listed.get().iter().filter(|(_, read)| !read).count();
NotificationCenter::new(open.clone(), bell.clone())
  .trigger(Badge::count(unread).label("unread notifications").on(
    Button::new("Notifications").icon(icons::BELL).icon_only(true).variant(ButtonVariant::Ghost),
  ))
  .items(listed.get().into_iter().map(|(id, read)| row(id).unread(!read)))
  .on_mark_all_read(move || mark.update(|all| all.iter_mut().for_each(|(_, read)| *read = true)))
  .on_clear(move || clear.set(Vec::new()))
  .on_view_all(move || go_to_inbox());
```

| Builder | |
| --- | --- |
| `NotificationCenter::new(open, anchor)` | Shown beside `anchor`, the bell, while the `Signal<bool>` is `true` |
| `trigger(element)` | What opens and closes it: a `Button`, or a `Badge` with its count or its dot on one. It is put before the panel and is its anchor; the first thing in it that takes the focus is what is pressed, and says whether the panel is open |
| `title(&str)` | What the panel is called, at its top and to tooling: "Notifications" by default |
| `item(Notification)`, `items(..)` | What it lists. Each is drawn as a row, whatever its own variant |
| `on_mark_all_read(Fn())` | Draws "Mark all read" in the header and is called on a press of it. The control takes no press while nothing is unread |
| `on_clear(Fn())` | Draws "Clear all" in the header and is called on a press of it. The control takes no press while the list is empty |
| `on_view_all(Fn())` | Draws the footer, "View all", and is called on a press of it, after the panel closed |
| `mark_all_read_label(&str)`, `clear_label(&str)`, `view_all_label(&str)` | What each control says |
| `empty_text(&str)`, `empty(element)` | What it says under an inbox while it lists nothing ("No notifications"), or the app's own empty state in their place |
| `on_dismiss(Fn())` | Told after a press outside it or Escape closed it; not when the app sets the signal, and not on "View all" |
| `dismiss_on_escape(bool)`, `dismiss_on_outside_press(bool)` | Whether each closes it: both by default |
| `placement(Placement)`, `offset(..)`, `collision(CollisionStrategy)` | Under the bell with its end at the bell's end (`BottomEnd`), the theme's smallest spacing from it, and to the other side then into the window where there is no room |
| `width(..)`, `max_height(..)` | How wide the panel is (360), and how high the list is at most before it scrolls (360) |
| `inline(bool)` | Draws the panel where it is declared, open or not, with no trigger: a panel of a page, a gallery, a test |
| `label(&str)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [popover](#popover) has them. `force` draws its first pressed row's states; its controls are `<id>-mark-all-read`, `<id>-clear` and `<id>-view-all` |

- The list and what is read are the app's. The panel tells what a person did (its three handlers, and each notification's own `on_click` and `on_close`) and is given the list as it is then; it keeps nothing but what a popover keeps, and is built anew with each render.
- As it opens the focus goes into it, to the first thing in it that takes the focus, and back to the bell as it closes. Tab stops in the rows once and the arrow keys move between those that are pressed; a row's close control is a stop of Tab after it. "View all" closes the panel first and then tells, so what the handler opens keeps the focus it takes.
- The list is as high as its rows up to its greatest height and scrolls past it, under the header and over the footer, and gives way where the window has less room. A title longer than the header's room is cut with an ellipsis; the controls keep their room.
- To what reads the tree it is a `dialog` called by its label or its title, holding a `list` of the same name; its bell says `a11y_haspopup` and whether it is expanded.
- Parts: `NotificationCenterPart::Panel`, `Header`, `Title`, `Actions`, `Action` and `ActionLabel` (what they give the theme's [link](#link)), `List`, `Item` (what holds a row that has another under it: the line between the two), `Empty`, `EmptyIcon`, `EmptyText`, `Footer`. States: `NotificationCenterState::Footer`, `Empty`. The rows are styled by [`Notification`](#notification)'s recipe.
- In the design the panel is 360 wide on `Popover` in an edge of 1 in `Border`, `Xl` round, with the `Lg` shadow. The header is 48 high and 16 from its sides over a line in `Border`: the title at 15, semibold, and "Mark all read" at 13, medium, in `Info`. A row is 56 with a line in `Border` along its bottom, and the footer 40 with "View all" at 13, medium, in its middle. The board draws three rows and no end of the list, no empty panel and nothing under the pointer: the list scrolls past 360, an empty panel has an inbox of 24 over its words at 13, both `MutedForeground`, 32 over and under them, the header's controls are underlined under the pointer as a link is, and the footer is in `Highlight`. It fades in and out as the popover does.

## AI Label

The mark on what a model made or filled in, and on what asks one: a summary that was generated, a field that was suggested.

```rust
use lurq::ui::{AiLabel, AiLabelSize};

AiLabel::new();
// Said otherwise, and called by more than it draws.
AiLabel::new().text("Draft").label("Drafted by the assistant");
AiLabel::new().size(AiLabelSize::Sm).icon(None);
```

| Builder | |
| --- | --- |
| `new()` | Sparkles and "AI" |
| `text(&str)` | The word it draws in place of "AI": the app's word for it, or the word in another language |
| `label(&str)` | What it is called to tooling, where that is more than the word it draws: "Generated by AI" |
| `icon(IconChoice)` | An icon of the theme's set in place of the sparkles; `None` for no icon |
| `size(AiLabelSize)` | `Sm` 18, `Md` 22 (the default, the board's), `Lg` 26 |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- It is read and not operated: it takes no press, no focus and no stop of Tab. To what reads the tree it is a `note` called by its label, or by its word.
- What explains it stands on it as on any element: `Tooltip::new("…").mount(ctx, AiLabel::new())`, or a [toggletip](#toggletip) beside it.
- A word longer than the room it has ends in an ellipsis, and the icon keeps its place before it.
- Parts: `AiLabelPart::Root`, `Icon`, `Label`. State: `AiLabelState::Icon` (it has one). Sizes: `AiLabelSize`.
- In the design it is 22 high with the corners of `RadiusSize::Sm`, on `InfoMuted` in an edge of 1 in `Info`, padded 8, with sparkles of 12 4 before "AI" in 11 and semibold, both in `Info`. The board draws that one size: small is 18, padded 6, at 10 with an icon of 10, and large 26, padded 10, at 12 with an icon of 14, the steps of a [badge](#badge).

## Back To Top

A round button that shows once a page is scrolled down, and takes it back to its top.

```rust
use lurq::{
  components::{Column, Stack},
  layout::{StackAlignment, layout_kind::ScrollState},
  ui::{BackToTop, ScrollArea},
};

// scroll: ScrollState, kept by the page and given to what scrolls it too.
Stack::new()
  .stack_align(StackAlignment::BottomEnd)
  .child(ScrollArea::new().scroll_state(scroll.clone()).children(article))
  .child(Column::new().padding(16.0).overflow_visible().child(BackToTop::new(&scroll).mount(ctx)));
```

| Builder | |
| --- | --- |
| `new(&ScrollState)` | The state of what it scrolls |
| `threshold(f32)` | How far down the content is scrolled before the button shows: 300 (`ui::BACK_TO_TOP_THRESHOLD`) unless told; at 0, as soon as it leaves its top |
| `label(&str)` | What the button is called to tooling, in place of "Back to top" |
| `icon(name)` | Its icon, in place of the arrow that points up |
| `on_click(..)` | Hears the press, and Enter and Space, before the content is scrolled; a handler that prevents the event's default keeps the content where it is. A second handler is called after the first |
| `force_shown(Option<bool>)` | Shown wherever the content is scrolled to, or never |
| `disabled(bool)` | It takes no input |
| `mount(ctx)` | The button where it goes: it shows and goes with the scroll. Converting it (`.into()`) builds its still form, always shown, whose press takes the content to its top at once |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them |

- It is a mounted component: after each layout it reads how far the content is scrolled, and shows once that is past the threshold, fading in, and goes, fading out, as the content comes back above it. A page opened scrolled shows it at once.
- A press takes the content to its top with `ctx.scroll_to_top_of(&scroll)` (see [layout](../layout/#scroll)): over the frames that follow, as a wheel's scroll goes, and at once while motion is reduced or the theme's smooth scroll is off. A notch of the wheel that comes meanwhile adds to where the content is going, as with any wheel's scroll.
- Where it floats is the app's to say, as a [fab](#fab)'s is: it is an element, put in a corner of a `Stack` over the scroll, in something that gives its shadow room or does not clip (`.overflow_visible()`). While it is gone it takes no room.
- It is a [button](#button) to tooling, called "Back to top" or by its label, and a stop of Tab while it shows. Once the content is at its top the button is gone and the keyboard's focus with it: the next Tab goes on from where it stood.
- Parts: `BackToTopPart::Root` (what holds the button, whoever draws it: its `enter` and `exit` are how it comes and goes), `Button` and `Icon`, the last two laid over the theme's recipe for the root and the icon of an outlined, round button that is an icon alone. State: `BackToTopState::Disabled`.
- In the design it is a circle of 40 on `Background` in an edge of 1 in `Border`, with the medium shadow and an arrow of 16 in `Foreground`. Under the pointer, pressed and focused it is the outlined button it is made of.
- A renderer that draws another has `view.button()`, the button with its press attached, to build or to put in what it likes. When it shows, and how it comes and goes, stay the component's: what a renderer returns is put in the root, which fades, so a renderer of any root fades out as the default one does.

## Command Palette

A search over the window that finds a command or a place by typing: every command of an editor, the pages of an app, the files of a project.

```rust
use lurq::ui::{CommandItem, CommandPalette, icons};

CommandPalette::new(open.clone())
  .placeholder("Type a command or search")
  .group("Suggestions", [
    CommandItem::new("Toggle dark mode").icon(icons::MOON).shortcut("Ctrl+D").keywords(["theme"]).on_select(toggle),
    CommandItem::new("Open DevTools").icon(icons::TERMINAL).shortcut("F12").on_select(devtools),
  ])
  .group("Files", files.iter().map(|file| CommandItem::new(&file.name).icon(icons::FILE).detail(&file.folder)))
  .label("Commands")
  .key_hints(true);

// Commands an app fetches as a person types: it keeps every one it was given.
CommandPalette::new(open.clone())
  .query(query.clone())
  .on_query(move |typed| fetched.set(search(typed)))
  .filter(|_, _| true)
  .loading(searching.get())
  .items(fetched.get().iter().map(|hit| CommandItem::new(&hit.title)));
```

| Builder | |
| --- | --- |
| `CommandPalette::new(open)` | Shown while the app's `Signal<bool>` holds `true`. Running a command, Escape on an empty input and a press on the veil set it to `false` |
| `placeholder(&str)` | What the input says while nothing is typed: "Type a command or search" |
| `item(CommandItem)`, `items(..)` | Commands with no group |
| `group(&str, items)` | Commands under a label: "Suggestions". A group with nothing shown for what is typed is not shown |
| `filter(Fn(&CommandItem, &str) -> bool)` | Which commands are shown while something is typed. By default (`matches_every_word`) those whose label, detail or keywords hold every word typed, without regard to case, in the order the app gave them |
| `on_query(Fn(&str))` | Called with what the input will hold at each edit: for an app that fetches its commands. Each handler is called, in order |
| `query(Signal<String>)` | Where the app keeps what is typed. Without one the palette keeps it while it is open, and it is empty when it opens again |
| `loading(bool)`, `loading_text(&str)` | The list ends with a spinner and what `loading_text` says: "Loading…" |
| `empty_text(&str)` | What the list says when nothing is found: "No results" |
| `key_hints(bool)` | A row at the bottom, under a line, of the keys that work in it: ↑ ↓ to move, ↵ to run, Esc to close |
| `label(&str)` | Its name to what reads the tree: what its input says, unless told |
| `on_dismiss(Fn())` | A person closed it other than by a command: Escape on an empty input (the engine's `Modal::on_dismiss`), a press on the veil. Each handler is called, in order |
| `dismiss_on_escape(bool)`, `dismiss_on_veil(bool)` | Whether Escape on an empty input and a press on the veil close it: both by default. Off, Escape still clears what is typed |
| `inline(bool)` | The panel drawn where it is declared, open or not, without the veil: for a gallery or a test |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's |
| `CommandItem::new(label)` | A command: what it does, or where it goes |
| `icon(name)`, `detail(&str)`, `shortcut(&str)` | An icon of 16 before the label; a second text after it, muted; the keys that do the same at the end, as a [menu](#menu) writes them (the app binds them) |
| `keywords([..])` | Words that find it and are not shown |
| `disabled(bool)`, `on_select(Fn())`, `id`, `key` | Dimmed, and passed by; called when it is run, after the palette closed; the row's id and key |

- It stands near the top of the window over the veil, as the engine's `Modal`: the focus goes to its input as it opens and back to what had it as it closes, and Tab stays inside it, as a [dialog](#dialog)'s does.
- The focus stays in the input. Down and Up move the highlight through the rows that are shown, across groups and round at the ends, past a disabled one, and scroll the list to it; the pointer moves it too, and a row shows it as a menu item shows the keyboard's focus. Enter runs the highlighted command and closes the palette; a click on a row does the same. Escape clears what was typed, and closes the palette when nothing is. Typing puts the highlight on the first row it shows.
- To what reads the tree it is a `dialog` holding a `combobox` (the input, `expanded`, with the highlighted command's label as its `a11y_activedescendant`) and a `listbox` of `option`s with `a11y_selected`.
- Parts: `CommandPalettePart::Veil`, `Panel`, `Search` (the row at the top), `SearchIcon`, `Input` (laid over the theme's input), `Key` (laid over the [Kbd](#kbd) key of the search row's Escape and of the footer), `Results` (what scrolls: its most height), `List` (the room around the rows), `GroupLabel`, `Row` (laid over a menu item's root), `Detail`, `Empty`, `Status`, `Footer`, `FooterHint`. The rows are a [menu](#menu)'s items: a theme's rules for `MenuItem` style them too. States: `CommandPaletteState::Loading`, `Empty`.
- In the design the panel is 420 wide on `Popover` in an edge of 1 in `Border`, `radius-xl`, the large shadow, 72 from the top of the window over `Scrim`. The search row is 44 high, 14 in, with 10 between the magnifier of 16 in `MutedForeground`, the input in the body's type (no edge or fill of its own) and the key, and a line of `Border` under it. The list is padded 4 and scrolls past 320; a group's label is 12/500 in `MutedForeground`, padded 6 and 8; a row is a menu item, 32 high on `Highlight` while it is highlighted. The keys are the theme's [Kbd](#kbd), in the platform's names (`Enter` is ↩ on a Mac), and a row's shortcut is a Kbd's plain text, as a menu writes it, with the menu item's `Shortcut` part laid over its label.
- Its rows, a menu's items, follow the [density](../theme/#density) as a menu's do: 24, 32, 40 and 44 high, and the status row with them. The search row and the footer keep their heights.
- A renderer that draws another palette has `view.panel()`, `view.search()`, `view.input()`, `view.key(label)`, `view.results()`, `view.group_label(label)`, `view.row(index)`, `view.empty_state()`, `view.status()`, `view.footer_row()` (the key hints), `view.veil(panel)` and `view.modal(panel)`, with `view.shown()`, `view.command(index)` and `view.highlighted`.

## Context Menu

A [menu](#menu) of what can be done with an element, opened where the element is pressed with the secondary button or held with a finger: a file in a list, a node of a canvas, selected text.

```rust
use lurq::ui::{ContextMenu, MenuItem, icons};

ContextMenu::new(open.clone(), file_row)
  .item(MenuItem::new("Open").icon(icons::FOLDER_OPEN).on_select(open_file))
  .item(MenuItem::new("Rename").icon(icons::PENCIL).shortcut("F2").on_select(rename))
  .separator()
  .item(MenuItem::new("Delete").icon(icons::TRASH).destructive(true).on_select(delete));

// The board's edit menu: the commands of selected text in one row.
ContextMenu::new(editing.clone(), editor)
  .bar(true)
  .items(["Cut", "Copy", "Paste", "Select All"].map(MenuItem::new));
```

| Builder | |
| --- | --- |
| `ContextMenu::new(open, target)` | The app's `Signal<bool>`, and the element the menu is for, which lays out as it would without it: what asks for the menu sets the signal, and choosing an item, a press outside, Escape and Tab clear it |
| `item(MenuItem)`, `items(..)`, `separator()`, `heading(&str)` | As a [menu](#menu) has them: icons, shortcuts, checks, radios, submenus, a destructive item |
| `label(&str)` | The menu's name to what reads the tree: what it is the menu of |
| `inline(bool)` | The panel drawn in place of its element, open or not, and not over the window: for a gallery or a test |
| `on_dismiss(Fn())`, `dismiss_on_escape(bool)`, `dismiss_on_outside_press(bool)` | As a [menu](#menu) has them |
| `offset(impl Into<SpacingValue>)`, `collision(CollisionStrategy)` | How far its corner is from the point it was asked for, down and toward the end (under its element, from the keyboard), and what it does where the window has no room there (opens up or back, by default) |
| `bar(bool)` | The commands in one row, as the board's edit menu over selected text has them. Left and Right move between them; a separator is a line down the row, and a heading is left out |
| `disabled(bool)` | The element has no menu now |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's |

- It opens at the pointer on a press of the secondary button on the element (and on Control and a press on macOS), at the finger on a long press, and under the element from its start on the Menu key or Shift+F10 while the focus is in the element. A press asks for the menu of the innermost element that has one; asked for again on its element, it moves to the new point.
- It stays inside the window: where there is no room down or toward the end of the point, it opens up or back from it. One context menu is open at a time: asking for another closes the first.
- It is a [menu](#menu): the focus goes to its first item as it opens and back to what had it as it closes, Up and Down move between the items, a letter goes to the item it starts, Enter chooses, a submenu opens beside its item. To what reads the tree it is a `menu`, and the element says it has one (`a11y_haspopup`).
- Parts: `ContextMenuPart::Panel` (laid over the menu's panel), `Bar` (the panel of a bar), `BarItem` and `BarLabel` (laid over a menu item's root and label in a bar); the menu's and its items' parts are a menu's. State: `ContextMenuState::Bar`.
- In the design the menu is a menu at least 220 wide, its corner at the pointer. The bar is a row on `Popover` padded 3, `radius-lg`, in an edge of `Border` with the large shadow; its items are 28 high, padded 10, their labels 13.
- Its items follow the [density](../theme/#density), as a menu's do. The bar's items keep their 28: they are the commands of a toolbar.
- The engine places it: `Popup::at(Some((x, y)))` (`Overlay::at`) stands a popup at a point of the window and keeps it inside, which an app may use for a popup of its own.

## Sidebar

The way round an app, down one side of its window: its groups of places, the one the app is at, a header with the app or the workspace and a footer with who is signed in.

```rust
use lurq::ui::{Avatar, Sidebar, SidebarGroup, SidebarHeading, SidebarHeadingVariant, SidebarItem, SidebarTrigger, icons};

let heading = |collapsed| SidebarHeading::new("lurq").description("Workspace").icon(icons::BOX).collapsed(collapsed);
let account = SidebarHeading::new("Ivan Zaida")
  .variant(SidebarHeadingVariant::Account)
  .description("ivan@lurq.dev")
  .media(Avatar::new("Ivan Zaida"));

Sidebar::new()
  .label("Workspace")
  .header(heading(false))
  .header_collapsed(heading(true))
  .group(
    SidebarGroup::new("Project")
      .item(SidebarItem::new("Overview").icon(icons::HOME).href("/"))
      .item(SidebarItem::new("Inbox").icon(icons::INBOX).count(12).selected(ctx.route_path() == "/inbox"))
      .item(
        SidebarItem::new("Components")
          .icon(icons::FOLDER)
          .child(SidebarItem::new("Buttons").icon(icons::FILE))
          .child(SidebarItem::new("Inputs").icon(icons::FILE)),
      ),
  )
  .footer(account)
  .collapsed(collapsed.get())
  .open(open.clone())
  .mount(ctx);

// Wherever the app puts it: collapses the sidebar, or opens it over the page on a narrow window.
SidebarTrigger::new(collapsed.clone()).open(open.clone());
```

| Builder | |
| --- | --- |
| `label(&str)` | What the navigation region is called to tooling. "Sidebar" unless told |
| `header(element)`, `footer(element)` | What stays at its top and its bottom while its items scroll: a `SidebarHeading`, a search. The footer has a line over it. It has one of each: a second call takes the place of the first, where a card's or a dialog's `footer` adds to a row of buttons |
| `header_collapsed(element)`, `footer_collapsed(element)` | What the rail shows in their place: the mark alone, an avatar. Nothing unless told |
| `group(SidebarGroup)`, `groups(..)`, `item(SidebarItem)`, `items(..)` | Groups of items; `item` and `items` add to a group of no label |
| `collapsed(bool)` | The rail of the items' icons, 52 wide: the value of the signal `SidebarTrigger` turns, `collapsed.get()`. Its width moves between the two over the theme's slow duration |
| `open(Signal<bool>)` | On a narrow window, whether it is open over the page. `SidebarTrigger` sets it; choosing an item, Escape and a press on the veil clear it |
| `narrow(bool)` | Whether it lies over the page whatever the window's width: it does below the `md` breakpoint unless told |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the root |
| `mount(ctx)` | Puts it where it goes, keeping which lists are open and the rail's menus |
| `build(&theme)` | The sidebar as it is at rest in that theme, with no tooltips on the rail and nothing kept: for a gallery or a test. Converted into an element on its own (`.into()`) it is the same in the theme of what is being rendered |

| `SidebarGroup` | |
| --- | --- |
| `new(&str)` | A group under a label, written in capitals; an empty one has none |
| `count(usize)` | How many it has, at the end of its label |
| `action(element)` | An action at the end of its label: a small ghost button. The arrow keys of the items pass it by |
| `collapsible(bool)`, `default_open(bool)` | Its label opens and closes it; `default_open` is how it starts, and the sidebar keeps it from then on |
| `item(SidebarItem)`, `items(..)`, `key` | Its items; `key` tells two groups of one label apart |

| `SidebarItem` | |
| --- | --- |
| `new(&str)`, `icon(name)` | Its name, and the icon before it: what the rail shows of it (the first letter of its name where it has none) |
| `count(usize)`, `badge(element)`, `shortcut(&str)` | At its end: a count in the mono family, a `Badge`, the keys that go to it as `Kbd::shortcut` writes them |
| `selected(bool)` | It is where the app is: the app's, following its route |
| `disabled(bool)` | It is dimmed and takes nothing |
| `on_select(handler)`, `href(path)` | What choosing it does: the handler, and with the `router` feature the path the router goes to. A press, Enter and Space choose |
| `on_open_aside(handler)` | Called when a person asks for it beside what is open, in place of going to it: a press of the middle button, or a press with Ctrl held (Cmd on macOS). An app with tabs opens it in a new one. Such a press does not choose the item |
| `child(SidebarItem)`, `children(..)`, `default_open(bool)` | Nested items, which open and close with their height under it; `default_open` is how they start, open where one of them is selected unless told |
| `force(..)`, `id`, `key` | As a button's. `key` tells two items of one name apart |

- The items are a tree to the keyboard (`FocusGroup::vertical().one_stop(true).typeahead(true).nested(true)`, each item at its `focus_level`, one with nested items `focus_expanded`): Tab stops at it once, on the selected item; Up and Down move through the items shown, nested ones included, Home and End go to the first and the last, and a letter to the item whose name starts with it. Right opens a closed item and on an open one goes to its first nested item; Left closes an open one and on a nested item goes to the item it is in. A group that opens and closes is an item its items are in. On the rail the items are a list, and Right opens an item's menu.
- Collapsed, it is the rail: each item is its icon, says its name (and its keys) in a `Tooltip` at its right, and an item with nested items opens them in a `Menu` there, the selected one marked. An item whose nested item is selected is the selected one on the rail.
- A closed item that holds the selected item is marked as the selected one is, and a closed group that holds it has its label and its chevron in `Foreground`: what is selected shows where it is hidden. Tab stops on what marks it, and it is what is scrolled into view. To what reads the tree such an item has `a11y_current` `location`.
- On a window narrower than the `md` breakpoint it lies over the page in a `Drawer` with `overlay(true)` and `scroll(false)`, as wide as it is beside the page, its header and footer staying while its items scroll; whatever `collapsed` says, it is whole there. Choosing an item closes it first, and then calls the item's handlers and goes to its path, so a handler that reads the signal sees it closed.
- Which item is selected is the app's: with a router, `.selected(ctx.route_path() == path)` on each, and `.href(path)` to go there. The gallery's shell does this for its pages. Where the selected item moves (the app selects another: a router, a search, a link of the page), it is scrolled into view, the lists it is in opened; the item selected at the start is scrolled to as well.
- `SidebarTrigger::new(collapsed).open(open)` is a ghost button of the `panel-left` icon, called by which way it goes: "Collapse sidebar", "Expand sidebar", or on a narrow window "Open sidebar", "Close sidebar", each of which `collapse_label`, `expand_label`, `open_label` and `close_label` say otherwise. `narrow(bool)` tells it as the sidebar is told; `force(..)`, `id` and `key` are a button's.
- `SidebarHeading::new(title)` is the row of the header and the footer: `description`, `icon(name)` (a mark on the primary fill) or `media(element)` (an avatar), `trailing_icon(name)`, `variant(name)` (`SidebarHeadingVariant::Workspace`, the default, or `Account`, or a theme's), `collapsed(bool)` (the mark alone, for the rail), `on_click(|event| ..)` and `anchor(ElementRef)` (a button, from which a switcher's menu can hang), and `state`, `style`, `rules`, `force`, `id`, `key` as a button has them.
- Parts: `SidebarPart::Root`, `Header`, `Body` (what scrolls), `Footer` (its room at the sidebar's sides), `FooterContent` (the line and what is under it), `Group`, `GroupLabel`, `GroupText`, `GroupCount`, `GroupChevron`, `GroupAction`, `Item`, `ItemIcon`, `ItemLabel`, `ItemCount`, `ItemBadge`, `ItemShortcut`, `ItemChevron`, `ItemIndent` (one for each level an item is nested), `Nested`, `NestedContent`, `Heading`, `HeadingMark`, `HeadingText`, `HeadingTitle`, `HeadingDescription`, `HeadingTrailing`. States: `SidebarState::Collapsed`, `Overlay`, `Selected`, `Disabled`, `Open`, `Pressable` (a group's label that is pressed, a heading with a handler). Variants of a heading: `SidebarHeadingVariant::Workspace`, `Account`.
- In the design the sidebar is 260 wide on `Muted` with a line of 1 in `Border` along its end, padded 8, 2 between rows. The 8 at its sides is its header's, body's and footer's: the body reaches the sidebar's edge, and at its end it has the lane of the theme's scroll bar (its width and its inset on either side, 12 with the default bar) where that is more than 8, so that the bar, which is drawn over what scrolls, lies beside the rows and not over them. The rail shows no bar: it scrolls with the wheel and the keys. An item is 32 high (it follows the density: 24, 40 and 44 at compact, comfortable and touch), padded 8 across, 8 between an icon of 16 in `MutedForeground` and its name in 13; the selected one is on `Background` with an edge of `Border`, its name in the medium weight and its icon in `Foreground`. Under the pointer an item has the `StateHover` veil, held down `StatePressed`, and the ring of the focus is inside it. A group's label is 11/600 in capitals, 0.6 apart, in `MutedForeground`, 10 over it and 4 under it. A nested item is 18 further in, after the chevron of 14 of the item it is in. The heading of the header is 44 high with a mark of 28 on `Primary`, its title 13/600 over a description of 11 in `MutedForeground`; the footer's is under a line of 1 and 48 high, its name in the medium weight. The rail is 52 wide, its items 36 by 32, 4 apart.
- It is a mounted component: the tooltips of the rail are mounted, and it keeps which nested lists and groups are open and the anchor and the open signal of each menu of the rail.
- To what reads the tree it is a `navigation` called by its label, its items `link` (with `href`) or `button`, the selected one with `a11y_current` `page`, an item with nested items with `a11y_expanded`, and a nested list a `group`.
- A theme's renderer (`SidebarRenderer`) is given a `SidebarView` with the groups built, the header and the footer as the sidebar is now, and `header()`, `body()`, `footer()` and `root(children)` to lay them out; `render()` is the default.

## Menubar

The menus of an app in a row, File, Edit, View, each opened from its title, as the title bar of a window holds them on Windows and Linux.

```rust
use lurq::ui::{Menu, MenuEntry, MenuItem, Menubar, MenubarMenu, icons};

// open: Signal<Option<usize>>, the menu open now; export: ElementRef, exporting: Signal<bool> for a submenu.
Menubar::new(open)
  .label("Application")
  .menu(MenubarMenu::new("File", [
    MenuItem::new("New").icon(icons::FILE).shortcut("Ctrl+N").on_select(move || new_file()).into(),
    MenuItem::new("Open…").shortcut("Ctrl+O").on_select(move || open_file()).into(),
    MenuEntry::Separator,
    MenuItem::new("Export as").submenu(Menu::new(exporting, export).items(["PNG", "SVG", "PDF"].map(MenuItem::new))).into(),
    MenuEntry::Separator,
    MenuItem::new("Quit").shortcut("Ctrl+Q").on_select(move || quit()).into(),
  ]))
  .menu(MenubarMenu::new("Edit", [
    MenuItem::new("Undo").icon(icons::UNDO).shortcut("Ctrl+Z").on_select(move || undo()),
    MenuItem::new("Redo").shortcut("Ctrl+Y").disabled(true),
  ]))
  .menu(MenubarMenu::new("View", [MenuItem::new("Show grid").checked(grid.get()).on_select(move || toggle_grid())]))
  .menu(MenubarMenu::new("Help", [MenuItem::new("About")]).disabled(!online.get()));
```

| `Menubar` | |
| --- | --- |
| `new(open)` | Which menu is open, by its index among the bar's: the app's `Signal<Option<usize>>`. A press of an item sets it; choosing in a menu, a press outside, Escape and Tab set it to `None`; the app may set it too |
| `menu(MenubarMenu)`, `menus(..)` | Menus, after the bar's others |
| `label(&str)` | The bar's name to what reads the tree |
| `disabled(bool)` | No menu of the bar can be opened |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the first item that is not disabled, the one Tab stops on. `id` is the bar's; its items are `{id}-0`, `{id}-1` and on, and each menu's panel `{id}-0-menu` |

| `MenubarMenu` | |
| --- | --- |
| `new(title, entries)` | A menu called `title` holding [menu](#menu) items: `MenuItem`s, with `MenuEntry::Separator` and `MenuEntry::Heading` between them (`.into()` each where the list mixes them). Its panel is called by its title to what reads the tree |
| `disabled(bool)` | The menu cannot be opened: its item is dimmed, and the pointer, the arrows and Alt pass it by |
| `id`, `key` | The id and key of its item on the bar, in place of `{id}-{index}`; its panel is `{id}-menu` |

- Its menus are [menus](#menu), with all their items have: icons, shortcuts, checks, radio items, submenus, disabled and destructive items, separators and headings. One is open at a time.
- The pointer: a press of a title opens its menu, and a press of it again closes it. While a menu is open the pointer coming onto another title opens that one, with no press. Choosing an item, a press outside and Escape close the menu.
- The keyboard: the bar is one stop of Tab (a [focus group](../focus-navigation/#focus-groups) of one stop). Left and Right move between the titles and go round at the ends, a letter goes to the title that starts with it, and a disabled title is passed by. Down, Enter and Space open the focused title's menu with the focus on its first item, Up on its last that can be chosen. In an open menu Left and Right close it and open the menu beside it, at its first item: Left in a submenu closes the submenu first, Right on an item with a submenu opens it, and Right on one without goes to the next title from a submenu too. Escape closes the menu and gives the focus back to its title.
- Alt: pressed and let go alone, Alt puts the focus on the first title and underlines the first letter of each title; Alt alone again gives the focus back to what had it, closing a menu that was open. The letters show while Alt is held down too. Alt with a title's first letter opens its menu from wherever the focus is, and from a menu of the bar opens the other; several titles with one first letter are gone round. On a Mac, where Option with a letter types a character and the app's menus are the system's, Alt does neither.
- Under the pointer, focused from the keyboard, or with its menu open, a title is filled with `Highlight`, the board's `accent`. In the design the bar is 36 tall in `Background`, padded 4, with the medium radius and an edge of 1 in `Border`, its titles 2 apart; a title is 28 tall, padded 10 at its sides, with the small radius, its text 13/500 in `Foreground`. A disabled title is dimmed. A menu opens under its title from its start, 4 below it.
- The engine's part is `FocusGroup::menu_row(true)`, which the bar's row has: a row of elements each of which opens a menu hung from it when it is pressed. While one of those menus has the focus, the Left and Right that nothing in the menu takes close it, with the menus opened from it, and press the element beside it; Alt with a letter and Alt alone act on the first such row of the window. A bar of your own, of buttons and [menus](#menu), gets the same keys from it.
- The bar keeps, for the signal it is given, a signal for each menu kept as one with the app's, the anchor each menu hangs from, and what Alt shows, while it is in the tree.
- Parts: `MenubarPart::Bar`, `Item` (a menu's title on the bar: what is pressed, and what is lit), `Label` (its text). States: `MenubarState::Open` (the item's menu is open) and `Disabled` (the bar, or the item, cannot be opened); an item is resolved in the states of the bar and its own.
- To what reads the tree the bar is a `menubar`, each title a `menuitem` called by its text with `a11y_haspopup` `menu` and `a11y_expanded`.
- A renderer for `Menubar` gets a `MenubarView`: `view.bar()` is the row with its style, its focus group and its handlers; `view.item(index)` an item's behavior with its style, `view.label(index)` its text, and `view.menu(index)` its menu, hung from the item, to put after it; `view.item_part(index, part)` is the style of a part of one item. `view.render()` builds the bar.

On a Mac an app's menus belong in the system's menu bar, which `App::set_menu_bar` fills (see [native menus](../window-lifecycle-menus/#building-a-native-menu)): there `app.menu_bar_support()` is `MenuBarSupport::Native`, and the app shows no `Menubar`. The native model routes a command by its id (`MenuAction::new(id, label)`, `app.on_menu_activate`), where a `MenuItem` calls a handler and has no id; and it has no check or radio item. So one list feeds both from the app's own description of its commands, built into each:

```rust
use lurq::app::{Accelerator, Menu as NativeMenu, MenuAction, MenuBarSupport};
use lurq::ui::{MenuItem, Menubar};

struct Command { id: &'static str, label: &'static str, key: Option<&'static str> }
const FILE: [Command; 2] = [
  Command { id: "new", label: "New", key: Some("n") },
  Command { id: "open", label: "Open…", key: Some("o") },
];

if app.menu_bar_support() == MenuBarSupport::Native {
  // menu_bar: the app's MenuBar, its application menu as it likes
  menu_bar.menus = vec![NativeMenu {
    title: "File".into(),
    items: FILE.iter().map(|c| {
      let action = MenuAction::new(c.id, c.label);
      match c.key { Some(key) => action.accelerator(Accelerator::command(key)), None => action }.into()
    }).collect(),
  }];
  app.set_menu_bar(menu_bar);
  app.on_menu_activate(move |id| run(id));
} else {
  // In the render, the same commands as a drawn bar.
  let file = FILE.map(|c| {
    let item = MenuItem::new(c.label).on_select(move || run(c.id));
    match c.key { Some(key) => item.shortcut(&format!("Ctrl+{}", key.to_uppercase())), None => item }
  });
  Menubar::new(open).menu("File", file);
}
```

## Toolbar

The tools of what is in front of a person, in a row: the marks of a text, the actions on a table, the commands of a document.

```rust
use lurq::ui::{Button, Toggle, Toolbar, ToolbarGroup, ToolbarItem, ToolbarVariant, icons};

Toolbar::new()
  .label("Formatting")
  .item(ToolbarItem::from(Toggle::new("Bold").icon(icons::BOLD).icon_only(true).on(bold.get())).shortcut("Ctrl+B"))
  .item(Toggle::new("Italic").icon(icons::ITALIC).icon_only(true).on(italic.get()))
  .item(Toolbar::separator())
  .item(Button::new("Link").icon(icons::LINK).icon_only(true).on_click(link))
  .item(Toolbar::spacer())
  .item(ToolbarItem::from(Button::new("Search").icon(icons::SEARCH).icon_only(true)).stays(true))
  .overflow(true)
  .mount(ctx);

// A command bar's groups: a label under each, and the group goes into the menu whole.
Toolbar::new()
  .variant(ToolbarVariant::Plain)
  .item(ToolbarGroup::new().label("Clipboard").item(Button::new("Paste").icon(icons::PASTE)))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `Toolbar::new()` | An empty bar on a surface, its controls of the small size |
| `item(impl Into<ToolbarItem>)`, `items(..)` | A `Button`, a `Toggle`, a `ToggleGroup`, a `Select`, a `ToolbarGroup`, `Toolbar::separator()`, `Toolbar::spacer()`, an `Element`, or a `ToolbarItem` made of any of them. A `ToolbarGroup` takes its controls with `item` and `items` too |
| `mount(ctx)` | Puts the bar in the tree of `ctx`'s component: it keeps the menu of what does not fit and the control the focus left. `build(theme)` and `From<Toolbar> for Element` draw it once with nothing kept: no tooltips, no menu |
| `variant(ToolbarVariant)` | `Surface` (the default): the background's fill and an edge. `Plain`: neither. `Floating`: the popover's fill, an edge and the large shadow |
| `size(ToolbarSize)` | The size of its buttons and toggles (a `ButtonSize`): `Sm` by default, as the board draws the bar, so it is 40 high |
| `overflow(bool)` | Where the bar is narrower than its controls, those that do not fit go into a menu behind a More button at its end, the last first. Without it they are cut off |
| `label(&str)` | What the bar is called, to what reads the tree |
| `more_label(&str)` | What the More button is called, in its tooltip and to what reads the tree: "More" |
| `more_icon(name)` | The icon of the More button: `more-horizontal` by default, a command bar's chevron |
| `secondary_command(ToolbarCommand)`, `secondary_commands(..)` | A command that is never in the bar, only in the menu of More, after what did not fit; with one, More is always there |
| `disabled(bool)` | Its buttons, toggles and groups of toggles are disabled, and so are the items of its menu and More; an element given as it is is the app's to disable. State: `ToolbarState::Disabled` |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the More button; a control is forced with its own. The More button has the id with `-more` after it, its menu `-menu` |

| `ToolbarItem` | |
| --- | --- |
| `shortcut(&str)` | Keys shown in the tooltip of an icon alone and beside its item in the menu |
| `stays(bool)` | The control never goes into the menu |
| `as_given(bool)` | The button or toggle keeps its own variant, size and style over the bar's: the outline Export of a table's bar |
| `ToolbarItem::element(el)`, `in_menu(ToolbarCommand)` | Any element: it stays in the bar unless it has a command to stand for it in the menu |
| `ToolbarItem::stack(items)` | Controls one over another in one place of the bar, 2 apart: a command bar's Cut over Copy. It goes into the menu whole |

A `ToolbarCommand` is an item of the menu: `ToolbarCommand::new(label)` with `icon`, `checked` (a tick, as a menu item's), `disabled`, `on_select` (each one adds, called in order), `id` and `key`.

- The bar gives its buttons its size and makes them ghost ones, and its toggles its size, unless an item is `as_given`. A button or toggle that is its icon alone is called by its label and has a tooltip (below it) of its label and its shortcut.
- It is one stop of Tab: Tab enters it at the control the focus last left (the first, the first time), and leaves it at once. Left and Right move between its controls, the toggles of a group in it among them; Home and End go to the first and the last; a disabled one is passed by. A field or a select in it keeps its own keys while it is focused.
- With `overflow`, a control that does not fit drops out of the bar whole, the last first, and the More button (`more-horizontal`, after a line) appears at the end. Enter or Space on it opens its menu with the focus on its first item, and Escape gives the focus back to More; the menu has an item for each control that went, in the order of the bar: its label, its icon, its shortcut, a check for a toggle (each toggle of a group, after a line), and its press. A labelled group goes whole, under its label. A select or a field stays unless it has `in_menu`. The bar learns what went from the layout (the engine's `ShrinkLimit::Drop` and `drop_marker`), and its menu follows it a pass later.
- A labelled group has its line at its end, and no line of the bar stands beside it; the last group of the bar, or the last before its spacer, has none. No line stands between the spacer and More. A [command bar](#command-bar) is a toolbar of labelled groups under tabs.
- Parts: `ToolbarPart::Root`, `Item` (laid over each button's and toggle's root), `Slot` (a control with the line before it, 2 apart), `Separator` (laid over a separator's root), `Spacer`, `Group`, `GroupItems`, `GroupLabel`, `Stack`, `More`, `MoreButton` (laid over the More button's root, after `Item`). States: `ToolbarState::Disabled`, `Overflowing`. Variants: `ToolbarVariant::Plain`, `Surface`, `Floating`.
- In the design the bar is padded 4, its controls 2 apart; a button is a ghost one of 32 with an icon of 16; a toggle that is on has the fill of what is picked out; a line of 20 in the border's color stands between two groups with 4 on either side. On a surface it has the background's fill, an edge of 1 in the border's color and the middle radius. A group of a command bar is padded 10 with a line at its end and its label of 10 in the muted foreground under it. A focused control has its ring inside it, where the bar does not cut it off.
- To what reads the tree it is a `toolbar` called by its label, with `a11y_orientation`; the More button says `a11y_haspopup` and `a11y_expanded`.
- A renderer that draws another bar gets a `ToolbarView`: `view.slot(index)` is a control as the bar lays it out (dropping when it does not fit), `view.more()` the More button with its menu, `view.menu()` the menu alone, `view.dropped` which slots went.

**In a narrow room.** With `overflow(true)` every control has a form in the menu at the end of the bar: a button,
a toggle (checked), a group of toggles, a labelled group (under its label), a stack, and a select or a field that
was given `in_menu`. What is kept in the bar is a control that `stays`, and a select, a field or any element with
no `in_menu`. What is kept never runs past the bar: a bar narrower than what stays in it (and a bar without
`overflow` narrower than its controls) scrolls sideways to it, by the wheel, a drag, and the keys, which bring the
focused control into view. A bar that is only built has no menu, and scrolls to all of its controls. The board draws the bar and its overflow menu (Toolbar, Overflow Menu, Table Toolbar) and no narrower form: scrolling to what stays is lurq's.

## Action Bar

The toolbar of a selection: over the list whose rows are selected, it says how many are, offers what can be done to them, and clears them.

```rust
use lurq::{components::Stack, ui::{ActionBar, Button, icons}};

Stack::new()
  .child(list)
  .child(
    ActionBar::new(selected.get().len())
      .action(Button::new("Move").icon(icons::FOLDER_IMPORT).on_click(move_them))
      .action(Button::new("Duplicate").icon(icons::COPY).on_click(duplicate))
      .destructive_action(Button::new("Delete").icon(icons::TRASH).on_click(delete))
      .on_clear(move || selected.set(Vec::new()))
      .escape_clears(true)
      .return_focus(list_ref.clone())
      .mount(ctx),
  );
```

| Builder | |
| --- | --- |
| `ActionBar::new(count)` | The bar of a selection of `count`: shown while it is more than none |
| `count_text(&str)` | What it says, in place of "3 selected" |
| `label(&str)` | What it is called to what reads the tree, in place of what it says |
| `action(Button)`, `destructive_action(Button)` | Adds an action on what is selected: a round ghost button; one that destroys in the destructive color. `actions(..)` and `destructive_actions(..)` add several |
| `on_clear(handler)` | What clearing does: the round button with a cross at the end (none without one), and Escape where it clears. Each one adds, called in order |
| `escape_clears(bool)` | Escape clears the selection while the bar is shown, wherever the focus is, unless what has it took the key |
| `return_focus(ElementRef)` | Where the focus goes as the bar leaves with the focus in it, cleared by its button or by Escape, or emptied by an action: the list it acts on (its first control that takes the focus). Without it the focus leaves with the bar |
| `mount(ctx)` | Puts the bar in the tree of `ctx`'s component, which keeps whether the focus is in it: `return_focus` needs it. `build(theme)` and `From<ActionBar> for Element` draw it with nothing kept |
| `side(ActionBarSide)` | The edge of the room it is given that it stands at: `Bottom` (the default) or `Top` |
| `clear_label(&str)` | What the clear button is called: "Clear selection" |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the clear button; an action is forced with its own. The clear button has the id with `-clear` after it |

- It is as large as the room it is given and takes no press itself: put it over what it acts on, in a `Stack`. The bar stands 16 from the bottom (or the top) of that room, in the middle; it comes in with a fade and a move of 16 from that edge, and leaves the same way as the count goes to none.
- It is a toolbar to what reads the tree, called by its label or what it says, and one stop of Tab whose Left and Right move between its buttons.
- What it says is on one line. Where the room is narrower than the bar, it gives way first and is cut with an ellipsis: the actions and the clear button stay whole, at any text size.
- Parts: `ActionBarPart::Root`, `Bar`, `Count`, `Separator` (laid over a separator's root), `Action`, `ActionIcon`, `ActionLabel`, `Destructive` (laid over a button's root, icon and label), `Clear`, `ClearIcon`. State: `ActionBarState::Empty`. Variants: `ActionBarSide::Bottom`, `Top`.
- In the design the bar is 44 high, of the popover's color, round at its ends, with an edge of 1 in the border's color and the large shadow, padded 16 before its count and 6 after its clear button, its parts 4 apart. The count is 13/600; a line of 20 with 6 on either side sets it apart; an action is 32 high, padded 10, its icon 14 and its label 13, 6 apart; the clear button is a round 32 with a cross of 15. A phone's selection bar is the same parts restyled: across the top, 56 high, in the fill of what is picked out.

**In a narrow room.** A mounted bar wider than its room draws each action that has an icon as its icon alone,
called by its label in a tooltip (`ActionBarView::compact`); an action with no icon keeps its label, and the
count and the clear button stay. Narrower than its icons too, the actions go, the last first, into the menu of a
More button before the clear button (`more_label`, "More actions"), and the text ends in an ellipsis only once
every action is in the menu. This is the order of the board's bar of a phone (Mobile/Selection Bar: the count, the
actions as icons, More). A bar that is only built (converted into an element, or `build`) cannot measure: it keeps
its labels in any room and cuts its text; `mount(ctx)` is the one that measures.

## Floating Panel

A tool window inside the app's window, over its content: layers, a color picker, the properties of what is chosen. A person moves it by its title bar, resizes it by its edges and its corner, collapses it to its title bar and closes it.

```rust
use lurq::{components::Stack, ui::{FloatingPanel, FloatingPanels, MenuItem}};

// One panel over the canvas. place: Signal<(f32, f32)>, kept with the window.
Stack::new()
  .child(canvas)
  .child(
    FloatingPanel::new("Layers")
      .position(place.clone())
      .on_dismiss(move || layers.set(false))
      .menu(more.clone(), || [MenuItem::new("Lock all")])
      .child(layer_rows)
      .mount(ctx),
  );

// Several: the one pressed comes to the front and is the active one; a press on the canvas leaves none active.
FloatingPanels::new()
  .panel(FloatingPanel::new("Layers").default_position((16.0, 16.0)).child(layer_rows))
  .panel(FloatingPanel::new("Color").default_position((280.0, 16.0)).child(color))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `FloatingPanel::new(title)`, `child(..)`, `children(..)` | A panel called by its title, and what it holds under its title bar |
| `position(Signal<(f32, f32)>)`, `position_controlled((x, y), on_position_change)`, `default_position((x, y))` | Where it stands in its room, from the top start: the app's signal, which a drag and the keys set; the app's value, which it is told a move of; or where its own place starts (16, 16 unless told) |
| `size(Signal<Option<(f32, f32)>>)`, `size_controlled(..)`, `default_size((w, h))`, `min_size((w, h))` | Its size the same three ways: `None` is the board's 240 wide and the height of what it holds; a size of its own makes what it holds scroll. No smaller than 160 by 80 unless told |
| `on_position_change(handler)`, `on_size_change(handler)` | Called with its new place, its new size, when a person moves or resizes it. Not when the app sets the signal. Each call adds |
| `open(Signal<bool>)`, `default_open(bool)`, `on_open_change(handler)` | Whether it shows what it holds or its title bar alone, which its collapse button turns; told when the button turns it |
| `on_dismiss(handler)`, `close_button(bool)`, `dismiss_on_escape(bool)` | Called when its close button or Escape in it closes it: the app takes it away. It has both where it has `on_dismiss`, unless told |
| `on_active_change(handler)` | Called with whether it is the active panel when a person makes it so or not (see below). Not when the app sets the active panel of a `FloatingPanels`. Each call adds |
| `media(element)` | Between the grip and the title: an icon, a swatch |
| `button(icon, label, handler)`, `menu(Signal<bool>, Fn() -> items)` | A button of its title bar, before its own; the menu its "more" button opens, of [`MenuItem`](#menu)s |
| `collapsible(bool)`, `resizable(bool)` | Its collapse button; its edges and its corner. Both unless told |
| `label(&str)` | What it is called to tooling: its title unless told |
| `move_label`, `resize_label`, `collapse_label`, `more_label`, `close_label` | What its grip ("Move"), its corner's grip ("Resize") and its buttons are called to tooling. The collapse button keeps its name and says whether it is open |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. Its grip is `{id}-grip`, its corner's `{id}-resize`, its buttons `{id}-collapse` and `{id}-close`. In a layer its key (its title unless told) tells it from the others |

- It is mounted (`mount(ctx)`): it keeps where a drag began, and its place, its size and whether it is open where the app gives no signal. Mounted, it is a layer as large as what holds it (a `Stack` over the content), which lets a press through where the panel is not, with the panel in it at its place. Converted into an element it is drawn where it stands, and a drag does not move it.
- A drag of its title bar moves it, the pointer held; a drag of its start edge, its end edge, its bottom row or its corner resizes it. A drag keeps it inside its layer, and no smaller than its least size.
- From the keyboard: its grip, at the start of its title bar, is a stop of Tab, and the arrow keys move the panel by 10, by 1 with Shift. The grip in its corner resizes it with the arrows the same way. Its buttons are pressed with Enter and Space, and Escape anywhere in it closes it. The platforms move a window from the keyboard by its own menu (Alt+Space on Windows) or not at all; a panel inside a window has a grip that takes the focus, as a splitter's divider does.
- `FloatingPanels` holds the panels over one content, the one pressed last in front: a press anywhere in a panel, or the focus coming into it, brings it there.
- The active panel is `Active`, its title in `Foreground`; the others' are in `MutedForeground`, as a tool window's title is while the document has the focus. A press in a panel, or the focus coming into it (Tab), makes it active; a press outside it (on the content the panels float over, on another panel) or the focus going to something outside it makes it not, and the one in front stays in front. A press in its menu, which opens outside it, leaves it active. A panel starts not active; one converted into an element is drawn active, as the board draws it. In a `FloatingPanels` one panel at most is active, which the app may hold: `active(Signal<Option<Arc<str>>>)` is the key of the active panel (its `key`, or its title where it has none), `None` where none is, set by a press or the focus and shown when the app sets it; `on_active_change(Fn(Option<&str>))` is told the key, or `None`, after it changes by what a person does. Each panel's `on_active_change` is told too.
- In the design the panel is 240 wide on `Popover`, an edge of 1 in `Border` inside it, the large radius and the large shadow. Its title bar is 30 high on `Muted` with a line of 1 in `Border` along its bottom, 6 in at its start and 4 at its end, 4 between what it holds: a grip of 14 in `MutedForeground`, the title at 12 and semibold, and buttons of 22 with the small radius, an icon of 14 in `Foreground` in each, on `Highlight` under the pointer. What it holds is padded 4, its rows 2 apart; the row along its bottom is 22 high with a grip of 10 in `MutedForeground` 3 in from the corner.
- Parts: `FloatingPanelPart::Root`, `Header`, `Grip`, `Media`, `Title`, `Actions`, `Button`, `Body`, `Footer`, `ResizeGrip`, `Edge`. States: `FloatingPanelState::Active`, `Collapsed`, `Sized`, `Resizable`, `Media`.
- To what reads the tree it is a `dialog` called by its label; its grip and its corner's grip are buttons called "Move Layers" and "Resize Layers", and its collapse button says `a11y_expanded`.
- A renderer is given a `FloatingPanelView`: `view.root()` the panel with its place and behavior, `view.header()`, `view.grip()`, `view.title()`, `view.actions()`, `view.body()`, `view.footer()`, `view.resize_grip()` and `view.edges()`.
- A dock layout is a batch of its own: it would take this panel's title bar as its tab, `FloatingPanels`' order as the order of its floating windows, and the board's Dock Drop Indicator (five squares of 32 on `Popover`, the one under the pointer in `Info`) as what a drag of a panel shows over the room it would dock into.

## Panel Header

The header of a panel that stands on its own in a layout: a side bar's Explorer, an inspector's Properties.

```rust
use lurq::ui::{MenuItem, PanelHeader, icons};

PanelHeader::new("Explorer")
  .button(icons::REFRESH, "Refresh", || refresh())
  .menu(more.clone(), || [MenuItem::new("Collapse folders")])
  .on_close(move || explorer.set(false));
```

| Builder | |
| --- | --- |
| `PanelHeader::new(title)` | A header called by its title, written in capitals |
| `media(element)` | What stands before the title |
| `button(icon, label, handler)`, `menu(Signal<bool>, Fn() -> items)` | A button at its end, after those it has; the menu its "more" button opens |
| `on_close(handler)` | Shows its close button, and is called on a press of it. Each call adds |
| `more_label(&str)`, `close_label(&str)`, `level(u32)` | What its buttons are called to tooling ("More", "Close"); the level of the heading it is (2) |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them |

- In the design it is 30 high on `Muted` with a line of 1 in `Border` along its bottom, 10 in at its start and 4 at its end; its title is 11 and semibold in capitals, 0.6 apart, in `MutedForeground`; its buttons are a floating panel's.
- Parts: `PanelHeaderPart::Root`, `Media`, `Title`, `Actions`, `Button`. State: `PanelHeaderState::Media`. To what reads the tree its title is a heading of its level.

## Dock Layout

Panels an app arranges in groups, splits, floating windows and strips, which a person rearranges: the layout of an editor, its explorer at the start, its documents in the middle and its terminal along the bottom.

```rust
use lurq::{
  components::Text,
  core::Signal,
  ui::{DockLayout, DockNode, DockPanel, DockPlace, DockSide, DockState, icons},
};

// The first layout, which the app stores once a person changes it.
let mut first = DockState::new(
  DockNode::row([
    DockNode::group(["explorer"]),
    DockNode::column([
      DockNode::documents(DockNode::group(["blur.rs", "theme.rs"])),
      DockNode::group(["terminal", "problems", "output"]),
    ])
    .shares([0.7, 0.3]),
  ])
  .shares([0.25, 0.75]),
);
first.show("outline", DockPlace::Hidden(DockSide::Right));

// layout: Signal<DockState>, made once in the component's create.
DockLayout::new(layout.clone(), [
  DockPanel::new("explorer", "Explorer").icon(icons::FOLDER).child(tree),
  DockPanel::new("blur.rs", "blur.rs").icon(icons::FILE_CODE).child(editor),
  DockPanel::new("terminal", "Terminal").icon(icons::TERMINAL).child(terminal),
  DockPanel::new("problems", "Problems").count(3).child(problems),
  DockPanel::new("output", "Output").child(output),
  DockPanel::new("outline", "Outline").child(outline),
])
.on_change(move |state| storage.set("layout", state.to_text()))
.on_detach(move |panel| open_in_a_window(panel))
.mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<DockState>, panels)` | The panels, arranged as the signal says; what a person changes is set in it |
| `controlled(DockState, panels, Fn(&DockState))` | The app's arrangement, and what it is told of a change, which the layout shows once the app gives it back |
| `panel(DockPanel)`, `panels(..)` | Panels after those it has |
| `on_change(Fn(&DockState))` | Told the new arrangement after a person changes it (after the signal is set); not when the app sets it. Each call adds |
| `on_detach(Fn(&str))` | Told the id of a panel let go outside the window: the app may open it in a window of its own. The layout changes nothing by itself |
| `empty(element)` | What the area of documents shows while no document is in it |
| `label(&str)` | What it is called to tooling: "Dock layout" |
| `maximize_label`, `restore_label`, `close_label`, `more_label`, `float_label`, `hide_label`, `put_back_label`, `move_left_label`, `move_right_label`, `move_top_label`, `move_bottom_label`, `strip_label` | The words of its buttons and menus: "Maximize", "Restore", "Close", "More", "Float", "Hide", "Put back", "Move to the left" (and the right, the top, the bottom), "Hidden panels" |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the shown tab of each bar of capitals. Its parts are found by ids after `id`: `{id}-tab-{panel}`, `{id}-group-{panel}` (a group by its first panel) and `{id}-body-{panel}`, `{id}-strip-{panel}`, `{id}-flyout-{panel}`, `{id}-maximize-{panel}`, `{id}-close-{panel}`, and `{id}-split-{path}` with its dividers `{id}-split-{path}-divider-{n}` |
| `mount(ctx)` | Mounts it: what each panel holds is kept while it moves, and a drag, a strip and the menus work. Converted into an element it is drawn as its state says, and nothing in it is kept or dragged |

| `DockPanel` | |
| --- | --- |
| `DockPanel::new(id, title)` | A panel the state names `id`, called `title` |
| `icon(name)`, `child(element)`, `children(..)` | Its icon (on its strip button, before its document tab's title); what it holds |
| `close_button(bool)`, `floatable(bool)`, `hideable(bool)`, `movable(bool)` | Whether it closes, floats, hides and moves at all: each unless told |
| `min_size((w, h))` | Its least size, which the splits that hold it keep: 120 by 80 |
| `count(u32)`, `dirty(bool)`, `label(&str)` | A count after its title in a tab of capitals; the dot of a document with changes not saved; what it is called to tooling (its title) |
| `on_close(Fn(&DockCloseEvent))` | Called when its close control or its menu's Close is pressed, before it is taken out of the state; `event.prevent_default()` keeps it. Each call adds |

**The arrangement.** A `DockState` is plain data (`Clone`, `PartialEq`, `Debug`): `root`, a tree of `DockNode::Split` (an orientation, its children, each child's share), `DockNode::Group` (its panels as tabs, the one shown, the panels hidden from it) and one `DockNode::Documents` (the area of documents, empty or holding a node); `floating`, the groups that float with their place and size; and `maximized`, the panel whose group fills the layout. `DockNode::row`, `column`, `group`, `documents`, `shares` and `shown` write a first layout. A state is changed by what a person does or by the app's own calls, which return whether anything changed: `show(panel, place)` and `move_panel` (a panel the state does not name is not shown until the app shows it), `move_group`, `close`, `select`, `put_back`, `maximize`, `restore`, `set_shares`, `set_float`, `raise_float`. A place is a `DockPlace`: `Tab { of }` (a tab of the group that holds `of`), `Beside { of, side }` (a new group on that side of it), `Edge(side)` (a new group along that edge of the layout, a quarter of it), `Documents`, `Float { position, size }` or `Hidden(side)`.

- A state is always made valid, never refused (`make_valid(known)`, `valid(known)`, which every change ends with): a panel the app no longer gives is left out, a panel named twice keeps its first place, an empty group goes, a split of one child folds and a split in a split of its orientation is flattened, shares are brought to at least 0.05 and to a sum of 1, a second area of documents is replaced by what it holds, and a `maximized` that is no docked panel is none.
- It is stored as text: `state.to_text()` (and `Display`) writes it in a readable form, `row(0.25 group("explorer") 0.75 column(…))`, and `DockState::parse(&text)` (and `FromStr`) reads it back, or `None` for text that is not the form; with the `persistent_storage` feature it is a value of the app's storage (`ctx.set_persistent_value("layout", state)`), stored as that text. A state read back is made valid by the layout as any other.
- A group in the area of documents has the [documents' tabs](#document-tabs), in the board's compact bar (32 high, titles of 12, crosses of 12 on every tab). A group of one tool panel has the panel's [header](#panel-header): its title in capitals, its menu and its close. A group of several has tabs in capitals with a maximize (or restore) and a close at its end. The rule is the board's: the editor's documents, the explorer's header, the terminal's three tabs.
- **What a panel holds is kept while it moves**: to another group, floating and back, hidden and shown, maximized and restored, and while a split is dragged. What it holds is mounted in the layout by the panel's id, wherever it is shown, and kept offstage while it is not (a tab not shown, a panel hidden): what is typed in it, its caret, where it is scrolled, what is open in it, and the focus in it, which follows it. What it holds is built when the app builds the layout, not when the layout renders for a drag.
- **A drag**: a tab, a header or a floating group's title bar dragged shows the indicator over the group under the pointer (its middle and its four sides) and a square at the middle of each edge of the layout; the zone under the pointer is lit and where the panel would land is laid over the layout. Let go on a zone, it lands there: the middle makes it a tab of the group, a side a group beside it, an edge a group along the layout's edge. Let go over nothing, it floats there, if it may float; outside the window, `on_detach` is told and nothing moves. Escape gives the drag up and nothing moves. A whole group is dragged by its bar's empty room. A press that moves less than 4 is a press.
- **From the keyboard and from a menu**: the menu of a tab (the secondary button, the Menu key or Shift+F10 on it) or of a header's More has what a drag does, Move to the left, the right, the top and the bottom (along that edge of the layout), Float, Hide (into the strip of the nearest edge), Maximize or Restore, and Close. F6 and Shift+F6 go from the group with the focus to the next and back, focusing its body. In a bar of capitals the arrows go between the tabs and Enter or Space shows one.
- **A strip** at an edge has a button for each panel hidden there. A press shows the panel over the layout beside the strip, with its header's Put back (which puts it back in its group) and its close; a press outside it or Escape hides it again.
- Floating groups are [floating panels](#floating-panel) over the layout; their place and size are written to the state when a move or a resize ends.
- An area of documents with nothing in it shows what `empty(..)` gives.
- The layout fills what holds it, which is to give it a size (a window's root, a column that grows). A panel's body is the room its group leaves under its bar; what the panel holds pads and scrolls itself (a `ScrollVertical` that fills it). `DockPanel::id()` and `title()` read a panel back. Two layouts in one component have their own `key` or `id`.
- In the design the layout is on `Background` in an edge of 1 in `Border` with the large radius. A bar of capitals is 30 high on `Muted` with a line of 1 in `Border` along its bottom, 4 in at its end; its tabs are 12 in at either side and 6 between what they hold, their titles 11 and semibold in capitals 0.6 apart in `MutedForeground`, the one shown in `Foreground` with a line of 2 in `Foreground` under it; a count is mono 11; its buttons are 22 with the small radius and an icon of 14 in `Foreground`, on `Highlight` under the pointer. A strip is 30 across on `Muted` with a line of 1 in `Border` on its inner side, 6 in along it and 4 between its buttons of 22. The indicator's zones are squares of 32 on `Popover` in an edge of 1 in `Input` with the small radius and the large shadow, the one under the pointer filled and edged in `Info`; where the panel lands is laid over in `InfoMuted` at 0.7 with an edge of 1 in `Info`.
- Parts: `DockLayoutPart::Root`, `Group`, `Bar`, `Tab`, `Label`, `Count`, `Actions`, `Button`, `Body`, `Empty`, `Strip`, `StripButton`, `Flyout`, `Preview`, `Zone`. States: `DockLayoutState::Selected` (a tab shown, a strip button whose panel is shown), `Hot` (a zone under the pointer), `Vertical`, `Left`, `Right`, `Top`, `Bottom` (the edge a strip stands along), `Maximized`.
- To what reads the tree the layout is a `group` called by its label, each group a `group` called by the panel it shows, a bar of capitals a `tablist` of `tab`s that say `a11y_selected`, a strip a `toolbar` called "Hidden panels" of buttons called by their panels' titles that say `a11y_expanded`, and a panel shown from a strip a `dialog`; the splits are [splitters](#splitter).
- A renderer is given a `DockLayoutView`: the element the default renderer built, and `view.part(..)`.

## Splitter

Panes that share a room by the dividers between them, which a person drags: the side bar, the editor and the terminal of a window.

```rust
use lurq::ui::{Orientation, Splitter, SplitterPane};

// layout: Signal<Vec<f32>>, saved with the window.
Splitter::new()
  .sizes(layout.clone())
  .pane(SplitterPane::new(files).length(240.0).min(160.0).max(480.0).collapsible(true).label("Files"))
  .pane(SplitterPane::new(
    Splitter::new()
      .orientation(Orientation::Vertical)
      .pane(SplitterPane::new(editor).min(120.0))
      .pane(SplitterPane::new(terminal).length(160.0).min(80.0).collapsible(true))
      .mount(ctx),
  ))
  .pane(SplitterPane::new(outline).share(0.2).min(100.0))
  .on_resize_end(|sizes| save(sizes))
  .mount(ctx);

// The board's Resizable: a grip on each divider, in a box of its own.
Splitter::new().grip(true).framed(true).pane(sidebar).pane(content);
```

| Builder | |
| --- | --- |
| `Splitter::new()`, `orientation(Orientation)` | Panes side by side (`Horizontal`, the default), or one over the other (`Vertical`). `SplitterOrientation` is a name of `ui::Orientation` |
| `pane(SplitterPane)`, `panes(..)` | A pane after the others, with a divider before it |
| `SplitterPane::new(content)` | A pane holding `content`, laid out in its whole width from its top: a height of `Dimension::Pct(100.0)` fills it |
| `SplitterPane::length(px)`, `share(0..1)` | How large it is to begin with: pixels it keeps as the splitter grows and shrinks, or a share it keeps. A pane with neither takes the rest; where all have one, the last does |
| `SplitterPane::min(px)`, `max(px)` | The least and the most it is |
| `SplitterPane::collapsible(bool)` | It closes when a divider is dragged under half its least size, and opens by a drag back, a double press on the divider (a finger's double tap) or Enter. The pane that takes the rest does not close |
| `SplitterPane::label(&str)`, `id(..)`, `key(..)` | What it is called: the divider after it is called so to tooling. The id of the pane's element, and what tells it apart where panes come and go |
| `SplitterPane::disabled(bool)` | It keeps its size: no divider beside it moves it, by the pointer or the keys, and it does not close |
| `sizes(Signal<Vec<f32>>)` | The sizes of the panes, the app's: pixels along the splitter as a divider last left them, 0 for a closed pane, written as a divider moves and once it is first laid out |
| `on_resize(handler)` | Called with the sizes each time a divider moves: while it is dragged, and for each key |
| `on_resize_end(handler)` | Called with the sizes once a divider has moved: let go after a drag, or moved by a key, a double press or Enter |
| `step(px)`, `large_step(px)` | What an arrow key moves a divider by, and with Shift: 10 and 50 |
| `grip(bool)` | A grip in the middle of each divider |
| `framed(bool)` | A box of its own: a fill, an edge and round corners, as the board draws its specimens |
| `label(&str)` | What a divider is called where the pane before it has no label |
| `mount(ctx)` | Mounted where it is declared: it keeps its sizes where the app gives it no signal, and shows the sizes the app sets in the signal from outside |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the dividers; `id` is the root's, and the dividers are `{id}-divider-{n}` |

- A drag holds the pointer: the divider follows it wherever it goes, in the window and out of it, the cursor is the resizing one over the divider and for the whole drag, and no text is selected by it. A double press puts the divider back where it began, or opens the closed pane beside it. A closed pane is inert (`.inert`, see Focus And Keyboard Navigation): what is in it stays mounted and takes neither the focus nor the pointer.
- A divider is a stop of Tab. The arrow keys of its orientation (Left and Right across, Up and Down one over the other) move it by a step, with Shift by a larger one; Home and End take the pane before it to its least and most size, and Enter closes and opens the collapsible pane beside it.
- The panes say what gives way when the window grows or shrinks: one of pixels keeps them, one of a share keeps its share, and the pane given no size takes the rest. No open pane is made smaller than its least size while there is room: where the rest would be, the panes nearest it give way down to theirs.
- A splitter measures its room before the frame is painted ([`ElementRef::observe_rect_before_paint`](../layout/#a-form-that-follows-its-own-room)): its first frame shows the panes at the sizes that fit the room, a pane whose share is under its least size at its least size, each divider at the edge of its pane, and nothing moves in the frames after. A splitter in a pane settles in that frame too.
- Converted into an element, a splitter keeps its sizes while what holds it does not render again, unless it is given `sizes`; mounted, it keeps them, and an app that sets `sizes` from outside (to restore a layout) sees the panes take them. Pixels set from a saved layout are the pane's pixels, and a pane of a share takes the share its pixels were of the room they were saved in.
- A drag renders nothing again while it moves: the panes are resized by sizes their element refs are given (`ElementRefMut::set_size`), and what is in them is only laid out again at the next pass. Nor does letting it go: tooling reads where it stands from the divider at once (`ElementRefMut::describe`).
- Parts: `SplitterPart::Root`, `Pane`, `Handle` (the room around a divider's line that takes the pointer and the focus), `Line`, `Bar` (the wider line under the pointer, dragged and focused), `Grip`, `GripIcon`. States: `SplitterState::Horizontal` or `Vertical`, `Framed`, `Disabled`.
- In the design a divider is a line of 1 in the `Border` color; under the pointer and focused from the keyboard it is 3 wide in `Ring`, and dragged in `Primary`. The room that takes the pointer is 12 across the line, over the edges of the panes beside it. The grip is 12 by 20 (20 by 12 across a vertical splitter), round by the small radius, in `Border`, with a grip icon of 10 in `Foreground`. Framed, the splitter has the `Background` fill, an edge of 1 in `Border` and the large radius.
- To what reads the tree a divider is a `separator` that can be focused, with `a11y_orientation` (that of its line), `a11y_valuenow` (the pixels of the pane before it), `a11y_valuemin`, `a11y_valuemax` (where it has a most) and the name of the pane before it, or the splitter's `label`. `a11y_valuenow` follows the divider as it is dragged and moved by the keys.
- Splitters nest, one orientation in the other: a pane holds a splitter of its own.
- A renderer that draws another splitter is given the view: `view.part(..)`, `view.thickness()` (the room between two panes), `view.sizes()`, and `view.render()` the default.

**A room that changes.** A splitter follows its room: panes that a narrow room took room from have it back
when the room grows, as a splitter first laid out at that width has them, until a person drags a handle. A dock
layout follows through its splitters.

## Property Grid

An inspector: the names of what is selected and the editors of their values, in categories that open and close. The position and the size of a shape on a canvas, its fill, its opacity.

```rust
use lurq::{components::SelectOption, ui::{Property, PropertyCategory, PropertyGrid}};

PropertyGrid::new()
  .title("Properties")
  .category(
    PropertyCategory::new("Transform")
      .property(Property::xy("Position", x.clone(), y.clone()))
      .property(Property::pair("Size", ("W", w.clone()), ("H", h.clone())))
      .property(Property::number_with("Rotation", rotation.clone(), "°").stepper(true).default(0.0)),
  )
  .category(
    PropertyCategory::new("Appearance")
      .property(Property::color("Fill", fill.clone()))
      .property(Property::slider("Opacity", opacity.clone(), 0.0, 100.0, "%"))
      .property(Property::number("Radius", radius.clone()).default(8.0))
      .property(Property::choice("Blend", blend.clone(), [SelectOption::new(0, "Normal"), SelectOption::new(1, "Multiply")]))
      .property(Property::toggle("Visible", visible.clone())),
  )
  .on_change(|id| save(id))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `PropertyGrid::new()` | A grid with no category yet |
| `category(PropertyCategory)`, `categories(..)` | Adds categories after those it has |
| `title(&str)`, `trailing(Fn() -> Element)` | The header over the categories, its title in capitals, and its buttons at its end |
| `name_width(f32)` | The width of the names, 84 unless told: where the mounted grid's divider starts |
| `read_only(bool)`, `disabled(bool)` | Every value read and not changed; or nothing takes input, dimmed |
| `full_width(bool)` | As wide as what holds it, in place of 300: an inspector's column |
| `height(impl Into<Length>)` | Its rows scroll in it: a long inspector's. Mounted, it builds only the rows near its view |
| `label(&str)` | What it is called to tooling: its title, else "Properties" |
| `on_change(Fn(&str))` | Called with the id of a property a person changed (its `id`, else its name), after its value is set; not when the app sets a value |
| `mount(ctx)` | The grid mounted where it is declared: it keeps which categories are open and where the divider is, Escape gives an editor's focus back to its row, and a text is told as the focus leaves it |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. With an `id`, a category's row is `<id>-<category>`, a property's `<id>-<property>`, its editor's holder `<id>-<property>-editor`, its reset `<id>-<property>-reset`, the divider `<id>-divider` |

| `Property<V>` | |
| --- | --- |
| `Property::text(name, Signal<String>)` | A text in an input |
| `Property::number(name, Signal<f64>)`, `number_with(name, value, unit)` | A number in a number input, stepped with Up and Down; with a unit written after it (`°`, `%`). `stepper(true)` gives it the board's stepper: one control of chevrons at the end of its box, its upper half a step up and its lower half a step down |
| `Property::slider(name, Signal<f32>, min, max, unit)` | A slider with the number typed beside it |
| `Property::toggle(name, Signal<bool>)` | A checkbox |
| `Property::choice(name, Signal<T>, options)` | One of the options in a select |
| `Property::color(name, Signal<Color>)` | Its swatch, a press of which opens the [color picker](#color-picker) in a popover, and its hex typed in a box beside it; a hex is committed with Enter or as the focus leaves it, and one that is no color is put back. `eyedropper(true)` gives the box an eyedropper at its end, which opens the picker too |
| `Property::xy(name, x, y)`, `pair(name, (letter, a), (letter, b))` | Two numbers side by side, each with its letter in its box: X and Y in the axes' colors, or W and H |
| `Property::custom(name, Fn() -> Element)` | Any element as the editor: it has no default and tells the grid nothing |
| `default(V)` | What the reset puts back; while the value is another, the name is marked and the reset is shown. Of the property's type: a default of another type does not compile |
| `description(&str)`, `read_only(bool)`, `disabled(bool)`, `id`, `key` | Said of its row to tooling; its value read and not changed; dimmed; what `on_change` is told and its row's id; its row's key |
| `PropertyCategory::new(title)` | A category, open. `property`, `properties` add to it; `default_open(bool)` whether it is open as it is first shown; `disabled`, `id`, `key` (what the mounted grid keeps it open by: its title unless told) |

- Every value is the app's signal, edited at once by its editor: `ui`'s own controls, the [input](#input), the [number input](#number-input), the [select](#select), the [checkbox](#checkbox) and the [slider](#slider), each with the grid's `Editor` part laid over its box, and for a color a [swatch](#color-swatch) and an input. The color row and the stepper are the grid's own: the board draws them only in its inspector (Property Grid's Fill and Rotation rows); its Color Field and Number Input are drawn otherwise, a swatch inside the box with an eyedropper and a minus and a plus at the ends. A choice, a box, a color, a slider and a number are told to `on_change` as each change is made; a text as it is committed, with Enter, or (mounted) as the focus leaves it after an edit.
- To the keys it is a tree ([a nested group](../focus-navigation/#a-nested-list)): Tab enters at one row, Up and Down go between the rows, Right opens a closed category and Left closes an open one or goes from a property to its category, typing a letter goes to the row it starts. Tab or Enter on a property's row go into its editor, where its own keys work (Up steps a number), and Escape gives the focus back to the row (mounted). A press of a name gives its editor the focus; a press of a category opens or closes it.
- A category opens and closes as an [accordion](#accordion)'s item does, over the theme's base duration and its standard easing, at once where motion is reduced: its rows slide open under its row and what is below moves with them, its rows fade in and out, and its chevron turns. Pressed again on the way, it turns round from where it is; the keys and the focus work all through it, and a closed category's rows are reached neither by Tab nor by the arrows. In a grid that builds every row, the rows are in a panel whose height runs, as the accordion's; in a grid with a `height`, whose rows are items of a virtual list, each row's height runs from nothing to its own in its turn, the rows coming into view built as they come, and the rows of a closing category stay, inert, until they have closed. That motion of rows in a virtual list is the one a mounted [tree view](#tree-view)'s rows have: the two share it.
- The divider between the names and the editors is dragged, or moved by 8 with Left and Right while it has the focus (40 with Shift); it is a stop of Tab after the rows, a `separator` to tooling with its value. It moves nothing where the grid is not mounted.
- To tooling it is a `treegrid`: a category is a `row` at level 1 with `a11y_expanded`, a property a `row` at level 2 called by its name, with its value in `a11y_value` ("X 120, Y 64") and its description; its editor is called by the name too ("Position X"), so the name is heard with the value from the row and from the editor alike. A group of labelled controls would say the value only in the editor, and the arrows of a tree need rows.
- A grid given a `height` scrolls, its bar in a lane of its own beside the rows (`ScrollBarPlacement::Reserved`), so no editor, reset or mark is ever under it; one with no height keeps its full width. Mounted, it builds only the rows near its view, as a [tree view](#tree-view) does: the engine's `FocusRows` hold every row shown, so Up, Down, Home, End, PageUp, PageDown, Left, Right and typeahead reach any of them, and the row the focus goes to is built and brought into view. A row's editor keeps what is half typed while the row is built; scrolled far away, the row is let go. A grid with no height has nothing to scroll and builds every row; so does one a theme's renderer draws.
- Parts: `PropertyGridPart::Root`, `Header`, `Title`, `Category`, `Chevron`, `Panel` (what opens and closes under a category's row), `Body` (its rows, which fade), `CategoryTitle`, `Row`, `Label` (a property's name), `Modified` (its mark), `Value`, `Editor` (laid over each control's box), `Axis`, `AxisX`, `AxisY`, `Reset`, `Stepper` (a number's chevrons, a color's eyedropper), `Swatch` (a color's), `Divider`. States: `PropertyGridState::Disabled`, `FullWidth`, and on a category's or a property's parts `Open`, `Modified`, `ReadOnly`, `Unavailable`.
- In the design the grid is 300 wide on `Background` with an edge of 1 in `Border` and the large radius. Its header is 30 high on `Muted`, its title in 11 semibold capitals spaced 0.6 in `MutedForeground`. A category's row is 28 high on `Muted` with a line under it, its chevron of 14 in `MutedForeground` 6 before its title in 12 semibold. A property's row is 32 high with a line under it, padded 12 at its start and 8 at its end, its name 84 wide in 12 in `MutedForeground` and its editors 8 after it, 6 apart. An editor is 24 high on `Background` with an edge of 1 in `Input`, the small radius, padded 6 across, its text in 12 mono; the letter of a value of two is 11 semibold mono, X in `Destructive`, Y in `Success`, others in `MutedForeground`. A number's stepper is the chevrons of 12 in `MutedForeground`, 6 from the end of its box. A color's swatch is 24, the small radius with an edge of 1 in `Border`, 6 before its box. A changed value's name is 12 semibold in `Foreground` with a dot of 5 in `Info` after it, and the reset of 13 in `MutedForeground` ends its row. The board draws no row under the pointer or focused: a row under the pointer is on `Muted`, a focused one has the focus ring inside it. Rows follow the theme's density; editors keep their 24.
- A renderer that draws another grid has `view.header()`, `view.category_row(index, entry)`, `view.property_row(category, row, entry)`, `view.divider()` and `view.categories`, each category's and property's view with its recipe in its state.

## Document Tabs

The open documents of an editor, in a bar across its top: one of them shown, each closed by its control, moved by a drag, and those that do not fit in a menu at the end.

```rust
use lurq::ui::{DocumentTab, DocumentTabs, MenuItem, icons};

// shown: Signal<PathBuf>; files: the app's list of what is open.
DocumentTabs::new(shown.clone(), files.iter().map(|file| {
  DocumentTab::new(file.path.clone(), &file.name)
    .icon(icons::FILE_CODE)
    .dirty(file.dirty)
    .pinned(file.pinned)
    .preview(file.preview)
}))
.label("Open files")
.on_close(|path| close(path))
.on_reorder(|from, to| files.update(|files| { let file = files.remove(from); files.insert(to, file); }))
.on_keep(|path| keep(path))
.on_add(|| new_file())
.context_menu(|path| vec![MenuItem::new("Close Others"), MenuItem::new("Copy Path")])
.mount(ctx);
```

| Builder | |
| --- | --- |
| `DocumentTabs::new(Signal<T>, tabs)`, `optional(Signal<Option<T>>, tabs)` | The tabs, of which the one whose value the signal has is shown, or none where it is `None`: an editor with nothing open. A press of another tab sets the signal |
| `controlled(value, tabs, on_change)`, `controlled_optional(Option<T>, tabs, on_change)` | The app holds which is shown, and `on_change` is told what it is to be; the bar shows nothing itself |
| `tab(impl Into<DocumentTab<T>>)`, `tabs(..)` | Tabs after those it has. `(value, "name")` is a tab |
| `on_change(handler)` | Told the new value as a person shows another tab, after the signal is set: `&T` of a `DocumentTabs<T>`, `&Option<T>` of a `DocumentTabs<T, Option<T>>`. Not when the app sets the signal, nor for a press of the tab shown |
| `on_close(Fn(&T))` | Told the tab a person closes, after its own handlers: its close control, a press of it with the middle button, Ctrl+W (or Cmd+W) while it is focused, or while it is shown with `shortcuts` on, but a pinned tab, which no key closes. The app takes it out, and says which is shown next where it was the one shown |
| `on_reorder(Fn(usize, usize))` | Told that a person moved the tab at the first index to the second, both counted in the tabs as they are before and after: `tabs.insert(to, tabs.remove(from))`. By a drag along the bar, and by Ctrl+Shift+PageUp and PageDown on a focused tab. A pinned tab moves among the pinned ones, another among the others. The bar moves nothing itself |
| `on_drag_away(Fn(&T, &DragEvent))` | Told while a person drags a tab away from the bar (farther from it than it is high, or outside the window), of mounted tabs: each move, and the release, whose `drop_result` is `Some`. While it is away the bar marks no place and moves nothing; where it goes is the app's, as a [dock layout](#dock-layout) docks it. Each call adds |
| `on_add(Fn())` | A press of the new tab button after the tabs, which is there once this is given |
| `on_keep(Fn(&T))` | A double press of a preview, or a finger's double tap: the app keeps the document open as a tab that is no preview |
| `context_menu(Fn(&T) -> Vec<MenuItem>)` | The commands of a tab's [context menu](#context-menu), opened by a press of the secondary button, a long press, the Menu key or Shift+F10 on a mounted bar |
| `label(&str)` | What the tabs are called to what reads the tree: a `tablist` of that name |
| `close_label(&str)`, `add_label(&str)`, `more_label(&str)` | What the close control is called before the tab's name ("Close blur.rs"), the new tab button and the More button: "Close", "New tab" and "Show the hidden tabs" unless told |
| `mount(ctx)` | Puts it where it goes: it keeps which tabs did not fit and the menu of them, a tab being dragged, and each tab's context menu. Built on its own (`build`, `.into()`) every tab is in the bar whether it fits or not, with no drag and no context menu |
| `DocumentTab::new(value, name)` | A tab. `icon(name)` an icon before its name, `dirty(bool)` a document not saved, `pinned(bool)` a square of the pin at the start with no close control, `preview(bool)` its name in italics, `on_close(Fn())` its own handler, called before the bar's, `disabled`, `id` (its close control is `{id}-close`, its item in the menu `{id}-item`) and `key` (what the focus follows it by when it moves: its name and how many before it have the same unless told) |
| `shortcuts(bool)` | Whether the editor's keys work from anywhere in the window, the focus in the document or elsewhere: on unless told otherwise |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the tab Tab stops on; the bar's buttons have ids after its own, `{id}-add` and `{id}-more` |

- The tabs are one stop of Tab, at the one shown. Left and Right move the focus between them, and Space and Enter show the one focused: a document is not opened by an arrow. The new tab button and the More button are stops of their own.
- A tab under the pointer, or holding the focus, shows its cross whole; another's is at half, and a document not saved shows a dot in its place, which the cross takes the place of under the pointer. The close control is no stop of Tab: Ctrl+W closes the tab focused, and the focus goes to the tab after it, or before it where it was the last.
- The editor's keys are heard wherever the focus is, as [window tabs](#window-tabs) hear theirs: Ctrl+Tab and Ctrl+Shift+Tab show the next tab and the one before, round at the ends, as Ctrl+PageDown and Ctrl+PageUp do on Windows and Linux; Ctrl+W (Cmd+W on macOS) closes the tab shown, but a pinned one; Ctrl+Shift+PageDown and PageUp move it among its kind. With the focus on a tab, they act on the focused tab's own keys first (Ctrl+W closes that one) and the focus follows the tab shown. `shortcuts(false)` turns them all off. Where the window's title bar has window tabs with their keys on too, a press is the documents' and the window's pass it by; an app that wants the window's to have the keys turns these off.
- A drag of a tab along the bar marks where it goes with a line of 2 in `Primary` at the edge of the tab it goes before (or after the last), and the dragged tab is dimmed; let go, `on_reorder` is told. A press that moves less than 4 is a press. Ctrl+Shift+PageUp and PageDown move the focused tab one place, and the focus stays on it.
- Mounted, the tabs that do not fit leave the bar from its end, the last first, but the one shown and the pinned ones. The More button at the end says how many it holds and opens a menu of them; a tab chosen there is shown, and is in the bar from then on with the focus on it. A tab pressed keeps the focus as it becomes the one shown, and a tab closed leaves it on the tab that takes its place.
- A name longer than its room ends in an ellipsis: a tab is 240 wide at most, and its close control keeps its room.
- The bar is a row of the density's height: 38 at the default, the steps of a row from it (`ui::ROW_STEPS`); the pinned tab and the new tab button stay square.
- To what reads the tree the bar is a `tablist` (its `label`), each tab a `tab` called by its name with `a11y_selected`, a pinned one too; the close control is called "Close" and the tab's name; the More button says `a11y_haspopup` `menu` and whether it is `a11y_expanded`.
- Parts: `DocumentTabsPart::Root`, `Bar` (the box under the tabs, on `Bar`, with the line the tab shown covers), `List`, `Tab`, `Icon`, `Label`, `Close`, `CloseIcon`, `Dirty`, `Add`, `AddIcon`, `More`, `MoreLabel`, `MoreIcon`. States of a tab: `DocumentTabsState::Selected`, `Dirty`, `Pinned`, `Preview`, `Disabled`, `Dragged`, `DropBefore`, `DropAfter`; of the bar, `Overflowing`.
- In the design the bar is 38 high in `Muted` with a line of 1 in `Border` along its bottom. A tab is padded 12 at its start and 8 at its end, its icon of 14, its name of 13 and its close control of 20 (the small radius, a cross of 13) 8 apart, a line of 1 in `Border` at its end; its icon and name are in `MutedForeground`. The tab shown is `Background` with a line of 2 in `Primary` along its top, over the bar's line, its icon, name and cross in `Foreground`. The close control under the pointer is `SecondaryHover`. The dot of a document not saved is 8 in `Foreground`. A pinned tab is a square of 38 with its pin. The new tab button is a square of 38 with a plus of 16 in `Foreground`; the More button is padded 10 with a line at its start, its count of 12 and a chevron of 14 in `MutedForeground`, 4 apart.
- A renderer that draws other tabs is given the view: `view.tabs` (each tab's name, icon and states), `view.dropped`, `view.part(..)`, `view.tab_part(index, ..)`, `view.tab(index)`, `view.list()`, `view.add()`, `view.more()` and `view.render()` the default.

## Status Bar

The bar along the bottom of a window, which says what state the app is in: the branch, the problems, what is being done at its start; where one is in a document at its end.

```rust
use lurq::ui::{StatusBar, StatusBarVariant, StatusItem, StatusItemVariant, icons};

StatusBar::new()
  .label("Editor status")
  .leading([
    StatusItem::new("feat/design-system").icon(icons::BRANCH).variant(StatusItemVariant::Primary).priority(2),
    StatusItem::new("3").icon(icons::WARNING).tooltip("3 warnings").on_select(|| show_problems()),
    StatusItem::new("Indexing").progress(0.55),
  ])
  .trailing(["Ln 42, Col 7", "UTF-8", "Rust"])
  .mount(ctx);

// Debugging: the whole bar in a tone.
StatusBar::new().variant(StatusBarVariant::Warning).leading(["Paused on a breakpoint"]);
```

| Builder | |
| --- | --- |
| `StatusBar::new()` | An empty bar in the neutral tone |
| `leading(items)`, `trailing(items)` | Items at its start and at its end, after those it has there. A `&str` is an item of that text |
| `variant(StatusBarVariant)` | The tone of the whole bar: `Neutral` (the default), `Primary`, `Info`, `Success`, `Warning`, `Destructive`. Each item is in it unless it has its own |
| `drops_first(StatusRegion)` | The end that gives way first where the bar is too narrow: `Trailing` (the default) or `Leading` |
| `label(&str)` | What the bar is called to what reads the tree |
| `mount(ctx)` | Puts it where it goes with the tooltips of its items, which keep their timers. Built on its own it has none |
| `StatusItem::new(text)` | An item: its text, which is its name to what reads the tree. `icon(name)` an icon before it, `icon_only(bool)` the icon alone, `variant(StatusItemVariant)` its tone, `tooltip(&str)`, `progress(0..1)` a rail after its text, `priority(i32)` how long it stays in a bar too narrow (the lower leave first, 0 unless told), `disabled`, `id`, `key` |
| `StatusItem::on_select(Fn())` | Called when it is pressed, by a click, Space or Enter: an item with a handler is a button, one without says what it says. Each one adds |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the first pressed item |

- The items that are pressed are one stop of Tab, at the first of them; Left and Right move between them and pass the others by.
- Where the bar is too narrow its items leave it whole, those of the lowest priority first; of one priority, the end `drops_first` says before the other, each from its end. An item that left is no stop of Tab. A text is 240 wide at most, and ends in an ellipsis.
- A tone is the muted fill of its color, the item's text and icon in the color: `DestructiveMuted` and `Destructive`. `Primary` is the primary fill and its foreground. A pressed item under the pointer is under the hover veil (`StateHover`), held down under `StatePressed`, whatever its fill.
- The bar is 26 high whatever the density: it is a bar, as the toolbar and the menubar are, not a row.
- To what reads the tree the bar is a `status` (its `label`), each item called by its text, an icon alone too; an item with progress says `a11y_valuenow` where it is pressed.
- Parts: `StatusBarPart::Root`, `Item`, `Icon`, `Label`, `Track`, `Fill`. States of an item: `StatusBarState::Pressable`, `Progress`, `Disabled`; variants: `StatusBarVariant` (`StatusItemVariant` is the same).
- In the design the bar is 26 high in `Muted` under a line of 1 in `Border`. An item is padded 8, its icon of 13 and its text of 12 in `Foreground` 5 apart. The item with progress has its text in `MutedForeground` and, 8 after it, a rail 72 by 4 of the `Input` color, round, its part done in `Foreground`. The branch is in the `Primary` fill.
- A renderer that draws another bar is given the view: `view.entries` (each item's text, its end, whether it is pressed and its order), `view.part(..)`, `view.item(index)` (an item, built, as it leaves the bar) and `view.render()` the default.

## Window Chrome

The title bar of a window the app draws itself, as Windows, macOS, GNOME and KDE draw theirs: the app's icon, the title, room for the app's own controls (a menubar, tabs, a search) and the window's controls.

```rust
use lurq::ui::{MenuItem, Menubar, MenubarMenu, WindowChrome, WindowControls, WindowPlatform};

// In the app's root component: open is a Signal<Option<usize>> of the menubar.
WindowChrome::new()
  .title("Untitled — Notes")
  .leading(Menubar::new(open).menu(MenubarMenu::new("File", [MenuItem::new("New"), MenuItem::new("Open…")])))
  .controls(WindowControls::new().close_label("Close Notes").on_close(|| save_draft()))
  .content(document)
  .mount(ctx);

// The bar of another platform, as a picture: a gallery, a test.
WindowChrome::new().platform(WindowPlatform::Kde).title("Notes");
```

| Builder | |
| --- | --- |
| `WindowChrome::new()` | The chrome of the platform the app runs on (`WindowPlatform::current()`), with the window's icon and its three controls |
| `platform(WindowPlatform)` | `Windows`, `Macos`, `Gnome` or `Kde`: the bar's look and order, in place of the platform's own |
| `title(&str)` | The window's title: in the bar (not a bar with tabs) and given to the window, where the system shows it. On one line, ending with an ellipsis where it has no room |
| `icon(impl Into<WindowChromeIcon>)`, `label(&str)` | The app's icon where the platform shows one (Windows, KDE): by default the window's own, the image the app set as the window's icon, or the box where it has none; a name of the theme's icons; an image of the app's (`ImageData`, with the `raster` feature); or `None`. The bar's name to what reads the tree, its title unless told |
| `leading(element)`, `trailing(element)` | The app's own at the start of the bar (after the icon, after the lights on macOS) and at its end (before the controls). A press there is theirs: it does not drag the window |
| `tabs(element)` | [Window tabs](#window-tabs) in the bar: the bar of the board's chrome with tabs, higher, with the app's icon and title before the tabs (where it has them) and no line under it, the tabs on its bottom edge so that the selected one runs into the content |
| `controls(WindowControls)` | The window's controls: `minimizable(bool)`, `maximizable(bool)`, their names `minimize_label`, `maximize_label`, `restore_label` and `close_label`, and `on_close(Fn())`, called before the window is asked to close |
| `content(element)`, `overlay(element)`, `overlays(..)` | What the window shows under the bar, and layers over the whole of it, as the engine's chrome has them |
| `mode(WindowChromeMode)`, `resize_handles(ResizeHandlePolicy)`, `border(ChromeBorderPolicy)` | The engine's chrome: when the app draws it (Windows and macOS by default), the zones along the edges that resize the window, the line around it |
| `mount(ctx)` | The window: the engine's [`WindowChrome`](../components/) around the content, with this bar as its title bar |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `state` gives the still form `WindowChromeState::Inactive` and `Maximized`; `force` draws the close control under the pointer or held, and on macOS the lights' glyphs. `id` is the bar's: its icon is `{id}-icon`, its title `{id}-title`, its controls `{id}-minimize`, `{id}-maximize` and `{id}-close` |

- Mounted, it is the window's frame. The window loses the system's title bar, and the bar drags the window from wherever nothing in it takes the press, a double press maximizes and restores it, and a right press shows the system's menu of the window (Restore, Move, Size, Minimize, Maximize, Close) where the system has one, on Windows (`WindowHandle::show_system_menu`). The edges resize it. The three controls are the window's to the system: each is marked with the engine's `.window_control(..)` ([components](../components/#the-windows-controls-in-a-title-bar-of-your-own)), in whichever platform's look the bar is drawn, so on Windows 11 the pointer resting on the maximize control opens the system's snap layouts over it. The controls are still under the pointer, held and pressed as any element is, and it is they that act on the window. A bar drawn as a picture marks none.
- The bar's icon is the window's: the image the app gave the window (`WindowHandle::set_icon`, or `with_icon` as it opens), drawn at the icon's size, 16, and drawn again as the app changes it; the board's box where the window has none. A bar drawn as a picture shows the icon of the window it is drawn in, as the gallery's pages show the gallery's. `icon(..)` puts a glyph of the theme's in its place (`WindowChromeIcon::Glyph`, from a name), an image of the app's (`WindowChromeIcon::Image`, from an `ImageData` the app keeps, so that it is uploaded once), or none (`None`). An image wants the `raster` feature (on with `image`); without it the window's icon is not drawn and the box is. Its id is `{id}-icon`.
- The controls act on the window: minimize, maximize and restore, and close asks the window to close (`WindowHandle::request_close`), which an app that must confirm first hears with `ctx.window().on_close_requested(..)`. A press on a control is the control's, never a drag.
- The bar is drawn as the window is: a window that is not the one in front has its title, its icon and its glyphs in `MutedForeground` and grey lights on macOS (`WindowChromeState::Inactive`), and a maximized one's middle control restores it, named `restore_label` (`Maximized`). A mounted chrome reads both from the window and is drawn again as they change; the still form takes them from `state(..)`.
- An app's menus go in the bar's leading room: `leading(Menubar::new(open)..)`. Its titles open their menus over the window; a press on them is theirs, and the bar around them still drags the window. A menubar draws a bar of its own, on `Background` in an edge; on the title bar's fill it is laid over with no fill, no edge and no padding: `.style(MenubarPart::Bar, |s| s.background(Color::new(0, 0, 0, 0)).border(Border::inside(0.0, PaletteColor::Border)).padding_x(0.0))`. On macOS an app's menus belong in the system's menu bar (`App::set_menu_bar`), and the leading room holds what else the app puts there.
- Converted into an element on its own it is the still form of itself: the window, its bar over its content, as a picture, with controls that act on no window (`on_close` is still told). A gallery shows the four platforms side by side so.
- One component, four renderers: `view.windows()`, `view.macos()`, `view.gnome()` and `view.kde()`, which `view.render()` chooses by the platform. A theme's renderer may draw every platform as one of them, or another bar from `view.bar()`, `view.icon()`, `view.title()`, `view.balance()`, `view.leading()`, `view.trailing()`, `view.tabs()`, `view.drag_room()`, `view.control(WindowControl)` and `view.controls()`.
- In the design the bar is on `Bar` (white in the light palette, `Muted`'s color in the dark) with a line of 1 in `Border` under it, its title 13 in `Foreground`. With tabs the line is under the tabs (`WindowChromePart::Line`), and the selected tab stands over it, open to the content:
  - Windows: 36 high, 12 in at the start; the box icon 16 and the title 13/500, 8 apart, at the start; three controls 46 by 36 at the end, a minus 16, a square 12 (two squares 13 while maximized) and a cross 16 in `Foreground`; minimize and maximize on `SecondaryHover` under the pointer, close on `Destructive` with its cross in `DestructiveForeground`.
  - macOS: 36 high, 12 in; the lights 12 across, 8 apart, at the start, close, minimize and zoom, in the system's red `#FF5F57`, yellow `#FEBC2E` and green `#28C840` with a rim of `#0000001F`, grey (`Input`) in a window that is not in front, showing a cross, a minus and a plus of 8 in `#00000099` under the pointer on any of them; the title 13/600 in the middle of the bar.
  - GNOME: 44 high, 10 in; the title 13/600 in the middle; round controls of 24, 8 apart, on `SecondaryHover`, a minus 14, a square 10 and a cross 14; close on `Destructive`.
  - KDE: 32 high, 8 in at the start and 6 at the end; the box icon 16 at the start, the title 13/400 in the middle; round controls of 22, 6 apart, a chevron down 16, a chevron up 16 and a cross 14; close on `Destructive`.
  - A bar with tabs is 40 high (44 on GNOME, 38 on KDE), and has the icon and the title before its tabs, no line under it. The board draws no title there; Ivan wanted the app named in its window (2026-11-17). On Windows the controls keep the top 36 of it; elsewhere they are in its middle.
  - The bar's height is its recipe's, with tabs or not: the content of a mounted chrome starts under the height the recipe gives `WindowChromePart::Bar` in the bar's states and on its platform, so an app's rule makes a bar with tabs 36 high: `.rules(&Recipe::new().when(Selector::new().state(WindowChromeState::Tabs), WindowChromePart::Bar, |s| s.height(36.0)))`, or the same in its theme. A height that is no number of pixels is the board's.
  - The window is `Background` in an edge of 1 in `Border`, its corners 8 on Windows, 12 on macOS and GNOME, 6 on KDE (the still form; a window's own corners and shadow are the system's).
  - Where the board is silent: a pressed control is as it is under the pointer, close `DestructiveHover`; GNOME's round controls have a veil over their fill under the pointer (`StateHover`) and a stronger one held (`StatePressed`), so they grow stronger in the light appearance and the dark alike, KDE's are `SecondaryHover` under the pointer and that with the `StatePressed` veil held; GNOME's maximized control shows two squares, KDE's keeps its chevron; a window not in front is muted on every platform.
- Parts: `WindowChromePart::Root` (the window, in the still form), `Bar`, `Icon`, `Title`, `Balance` (the room on the other side of a centred title), `Line` (the line under a bar with tabs), `Controls`, `Minimize`, `Maximize`, `Close`, `MinimizeGlyph`, `MaximizeGlyph`, `CloseGlyph`. The platform is the recipe's variant (`windows`, `macos`, `gnome`, `kde`); states `WindowChromeState::Inactive`, `Maximized`, `Tabs`.
- To what reads the tree the bar is a `group` called by its label or its title, and each control a `button` called by its label.
- The engine's own title bar (`components::ChromeTitleBar` with `components::WindowControls`) is drawn in the theme's roles too: `Muted` under a line of `Border`, controls in `Foreground` on `SecondaryHover`, close on `Destructive`.

## Window Tabs

The tabs of a desktop app's title bar, as a browser or an editor has them: one is selected, the tab of what the window shows, and a person closes them, asks for a new one, and drags one along the bar or out of it.

```rust
use lurq::ui::{WindowTab, WindowTabs, icons};

// open: Signal<u32>, which file the window shows; files: the app's list of open files.
WindowTabs::new(open, files.iter().map(|file| {
  WindowTab::new(file.id, &file.name).icon(icons::FILE_CODE).dirty(file.dirty)
}))
.label("Open files")
.on_close(|id| close(*id))
.on_add(|| open_new())
.on_reorder(|from, to| move_file(from, to))
.on_detach(|id| open_in_window(*id))
.mount(ctx);

// The app holds which is selected, and is told the tab a person selects.
WindowTabs::controlled(1, [(1, "blur.rs"), (2, "theme.rs")], |id| show(*id));
```

| Builder | |
| --- | --- |
| `WindowTabs::new(Signal<T>, tabs)`, `controlled(value, tabs, on_change)` | The tabs, and which is selected: the one with the signal's value, or the app's value. `T` is the app's own type; `WindowTabs<T = ()>` |
| `optional(Signal<Option<T>>, tabs)`, `controlled_optional(value, tabs, on_change)` | The same where the window may show none of its tabs (a start page): with `None` no tab is selected. A `WindowTabs<Option<T>>`, whose handlers are told `&Option<T>` |
| `tab(tab)`, `tabs(tabs)` | A tab, or tabs, after the others |
| `WindowTab::new(value, &str)`, `(value, &str)` | A tab of `value` with its label. The label takes the room the icon and the close slot leave and ends in an ellipsis where it does not fit |
| `WindowTab::icon(name)`, `dirty(bool)` | An icon before the label; a dot for changes that are not saved: in the close slot with the x in its place under the pointer, or after the label (`dirty_mark`) |
| `WindowTab::closable(bool)` | Whether a person may close it: they may unless told otherwise. One that may not (a home tab) shows no close button, and nothing closes it: no middle press, no Delete, no Ctrl+W |
| `WindowTab::close_button(bool)`, `on_close(Fn())` | Whether its close button shows (unless told otherwise; a closable tab without one is still closed by the middle button, Delete and Ctrl+W), and what hears it closed, before the strip's `on_close` |
| `WindowTab::icon_only(bool)` | Its icon alone, as a home tab: 28 wide, the icon in its middle, no label and no close slot. The label is still its name to tooling |
| `WindowTab::width(f32)`, `max_width(f32)` | A width of its own, which neither grows nor gives way; or the most it is wide, under the recipe's most (a cap of the app's on a long label, cut there with an ellipsis) |
| `WindowTab::fit_content(max)` | As wide as what it shows, its icon, label, marks and close slot, up to `max`, where its label ends in an ellipsis: the app measures nothing. Never narrower than the recipe's least width. In the list mode it keeps that width and leaves the strip whole; in the scroll mode it gives way to its least width, and the strip is reckoned with it at `max`. A `width` of its own wins, and a `max_width` caps it too |
| `WindowTab::trailing(element)` | An element of the app's after the label, in the tab's `Marks` part, 6 after the label and 4 from the next: a count, a mark that something waits with its tooltip. It keeps its width, and the label gives way first |
| `WindowTab::describe(name, value)` | Says `value` of the tab under `name` to tooling (the DevTools, the MCP tree), before what the strip says of it, so a reader that takes the first value of a name takes the app's: its route, its state. As [`Styled::describe`](#one-instance-one-place-one-subtree) of a component |
| `WindowTab::disabled(bool)`, `id(..)`, `key(..)` | It cannot be selected, closed or dragged, and the keys pass it by. Its id, which its close button has with `-close` after it, and its parts after it: `{id}-title`, `{id}-marks` |
| `on_change(Fn(&T))` | Told the value of the tab a person selects (a press, a drag, the arrows, the window's keys), after the signal is set: not a value the app sets, nor a press of the selected tab |
| `on_close(Fn(&T))` | Told the tab a person closes: its close button, a press of the middle button on it (any closable tab, whether its close button shows or not), Delete on it, or Ctrl+W (Cmd+W on macOS) on the selected one. The app removes it, and selects another where it was the selected one |
| `on_add(Fn())` | Told a press of the add button, which is shown once this is given |
| `on_reorder(Fn(usize, usize))` | Told a tab moved, from its place to the place it is to have: `tabs.insert(to, tabs.remove(from))`. Dragged along the strip, or Ctrl+Shift+PageDown and PageUp. The app reorders its list |
| `on_detach(Fn(&T))` | Told a tab dragged out of the strip and let go away from it. The app takes it out, to open it in a window of its own |
| `overflow(WindowTabsOverflow)` | What the tabs do in less room than they need at their most: `Scroll`, the default, shares it down to their least width and then scrolls; `List` keeps their width, and the tabs that do not fit leave the strip, the last first and never the selected one, into the menu of the More button at the end of the tabs that stay. Mounted only: converted into an element on its own the strip cuts them at its end |
| `menu(WindowTabsMenu)` | What the More button of the list mode opens: `Hidden`, the default, a menu of the tabs that left the strip; `All`, the list of every open tab (below) |
| `list_shortcut(bool)` | Ctrl+Shift+A (Cmd+Shift+A on macOS) opens and closes the More button's menu from anywhere in the window, apart from the window's other `shortcuts`: off unless told. The menu of the hidden tabs opens only while a tab is hidden, the list of every tab at any time. The More button's tooltip says the keys |
| `dirty_mark(WindowTabsDirtyMark)` | Where the dot of unsaved changes stands: `InCloseSlot`, the default, swapped for the x under the pointer; or `AfterLabel`, 6 after the label, the close button in its own slot drawn as a clean tab's, both shown at once and nothing swapped |
| `tab_stop(bool)` | Whether Tab stops in the strip; it does unless told otherwise. `false` is a strip the pointer and the app's own shortcuts reach: Tab passes its tabs, its add button and its More button by, and a press of one leaves the focus where it was. A tab the app focuses still takes Left, Right and Delete, and the window's keys work as before |
| `shortcuts(bool)` | Whether the window's keys work, from anywhere in the window: on unless told otherwise |
| `label(&str)`, `add_label(&str)`, `close_label(&str)`, `more_label(&str)` | What the list is called to tooling; the add button's name ("New tab"); the word before a tab's label that names its close button ("Close", which makes "Close blur.rs"); what the More button and its menu are called ("Show the hidden tabs") |
| `mount(ctx)` | Mounted where it is declared: the dragged tab follows the pointer, the selected tab is kept in view where the strip scrolls, and the focus goes to the selected tab when the keys close the one it was on |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the tab Tab stops on; `id` is the strip's, with `{id}-list` for the list, `{id}-add` for the add button and `{id}-more` for the More button; a tab in its menu is the tab's id with `-item` after it |

- The strip is as wide as its tabs and its add button, never wider, nor wider than what holds it, a row or a column. Given less room (a title bar narrower than its tabs), the tabs share it alike down to 72 each, the icon and the close slot, and past that the list scrolls sideways between the strip's start and the add button, which stays at the end; mounted, the selected tab is kept in view. A wheel turned up and down over the strip scrolls it sideways, as a browser's tab strip does.
- A press on a tab selects it when it is let go. A press and a move of more than 4 pixels is a drag: it selects the tab, and, mounted, the tab follows the pointer along the strip, drawn over the others, which make room for it where it would be let go. Let go, `on_reorder(from, to)` is told where it went; let go more than the strip's height above or below the strip, or out of the window past any of its edges (`DragEvent::outside_window`), `on_detach` is told instead; Escape puts the tab back, and its release tells nothing. Converted into an element on its own, the strip still tells where a dragged tab was let go, but the tab does not follow the pointer on the way.
- A press on a tab, its close button or the add button goes no further than them (`stop_propagation` on `on_mouse_down`), so a title bar that drags the window from a press is dragged from the strip's empty room and the bar around it, and not from a tab.
- With `overflow(WindowTabsOverflow::List)` the tabs keep their width, and where the strip has too little room for them all, those that do not fit leave it whole with the line before them, the last first, never the selected one (the engine's `ShrinkLimit::Drop` and `drop_marker`, learned before the frame is painted). The More button follows the tabs that stay, laid out only while a tab has left: how many left, 12, and a chevron of 12. Its menu lists them in their order with their icons; choosing one selects it, which brings it back into the strip in place of the last that fits, and gives it the focus. A drag moves a tab among those in the strip (`on_reorder` is told the places among them all), and the window's keys go round every tab, those in the menu too, each selected coming back into the strip.
- The tabs are one stop of Tab ([focus groups](../focus-navigation/#focus-groups)), at the selected tab: Left and Right select the next tab and the one before in the strip, and Delete closes the focused one where it is closable, whether its close button shows or not. The More button and the add button are stops after them, Enter and Space open the More button's menu with the focus on its first item, and Escape gives it back. A close button and a press of the middle button are for the pointer; the keyboard has Delete. `tab_stop(false)` takes all of them out of the order of Tab.
- The window's keys are heard wherever the focus is: Ctrl+Tab and Ctrl+Shift+Tab select the next tab and the one before, round at the ends, as Ctrl+PageDown and Ctrl+PageUp do on Windows and Linux; Ctrl+W (Cmd+W on macOS) closes the selected tab; Ctrl+Shift+PageDown and PageUp move it on and back. `shortcuts(false)` turns them all off. A key the strip takes is not passed on (`prevent_default`), and a key something took before it is not the strip's: in a window whose page has [document tabs](#document-tabs) with their keys on, a press switches the documents and not the window's tabs, as in an editor. An app that wants the window's tabs to have the keys turns the documents' off (`DocumentTabs::shortcuts(false)`).
- The gap of the `List` part is the room between two tabs (and between a separator and the tabs beside it) and before the More button; the gap of the `Root` part the room before the add button. The strip counts both as it fits its tabs: in the list mode a tab leaves with its gap and the separator before it, and comes back as soon as that room is there again. A separator stands between two tabs that are not the selected one whether they are in the strip or not, and in the list mode the separator before the add button keeps its room, drawn only after a tab that is not the selected one. A separator the recipe makes 0 wide is not laid out. The default recipe has no gap; Orchester's has 2 and no separators (`.base(WindowTabsPart::List, |s| s.gap(2.0))`, the same for `Root`, and `Separator` 0 wide).
- The More button points its chevron up while its menu is open, and the strip is then in the state `MenuOpen`, which the recipe draws as pressed. Its tooltip says how many tabs left ("8 hidden tabs"), with the keys of the `list_shortcut` where it has one.
- With `menu(WindowTabsMenu::All)` the More button opens the list of every open tab, in the strip's order, under it: a panel 320 wide with a search field at its head once it holds more than eight tabs ("Search 12 open tabs"), and a row of each tab whose label holds what is typed, in any case, or "No open tab matches “…”". A row has the tab's icon and label, an eye struck through where the tab is not in the strip, a check on the selected tab, and a close button whose x shows while the row is under the pointer or focused. The focus goes into the list as it opens, to the search field or else to the selected tab's row, and back to the More button as it closes. Down goes from the search field to the first row and Up from the first row back to it; Up, Down, Home and End move through the rows; Enter and Space choose a row, as a press does, and Enter in the search field chooses the first that matches: the tab is selected and comes back into the strip, and the list closes. The middle button and Ctrl+W (Cmd+W on macOS) on a row close its tab, and the keys go on to the next row: while the list is open Ctrl+W is the row's, never the selected tab's. Escape empties the search field and then closes the list; a press outside closes it. Rows have the ids `{id}-item` after their tab's id, their close buttons `{id}-item-close`, their marks `{id}-item-hidden` and `{id}-item-current`; the panel is `{strip}-panel` and the field `{strip}-search`. To what reads the tree the panel is a `dialog` and the rows a `listbox` of `option`s called "Open tabs", each `a11y_selected` where it is the selected tab, and described "Not in the tab strip" where it is not in the strip.
- Parts: `WindowTabsPart::Root` (the strip), `List`, `Tab`, `Icon`, `Title` (the label with what stands right after it), `Label`, `Marks` (the app's elements after the label), `Close` (the slot at a tab's end, its close button), `CloseIcon`, `Dot`, `Separator`, `Add`, `AddIcon`, `More` (the button of the tabs that left the strip), `MoreLabel`, `MoreIcon`, and of the list of every tab `Panel`, `Search`, `Items`, `Item` (a row, in the states of its tab), `ItemIcon`, `ItemLabel`, `ItemHidden`, `ItemCurrent`, `ItemClose`, `ItemCloseIcon` and `Empty`. States: `WindowTabsState::Selected`, `Dirty`, `Dragged`, `Disabled` and `IconOnly` of a tab, and `Disabled`, `Overflowing` (some tabs are in the More button's menu) and `MenuOpen` (its menu is open) of the strip. With the dot after the label, the dot and the close slot are drawn in the recipe of a clean tab, so that the pointer swaps nothing; the tab and its label are still `Dirty`.
- In the design the strip is 32 high with its tabs abutting. A tab is 128 by 32, round by 8 at its top corners and square at its bottom, padded 10 at its start and 6 at its end, its parts 8 apart: an icon of 14, the label 12 in the regular weight, and a close slot of 18 round by the small radius with an x of 12 in `MutedForeground`. The selected tab is `Background`, the color of the content it joins in a title bar, edged at its top and sides by a line of 1 in `Border` (on the light palette's white bar its fill alone does not tell it apart), its icon and label `Foreground`; another has no fill, its icon and label `MutedForeground` and its x at 0.6, and under the pointer `SecondaryHover` with its x whole. The dot is 7 in `Foreground`. A separator is 1 by 16 in `Border`, between two tabs that are not the selected one and before the add button. The add button is 30 by 28, round by the small radius, with a plus of 14 in `Foreground`. Where the board is silent: the close slot under the pointer is `Muted` and the add button `SecondaryHover`, the ring of the focus is inside a tab, and a disabled tab is dimmed. A tab of its icon alone is 28 wide with no padding. The dot after a label and the app's marks are 6 from the label, the marks 4 apart. The More button is 28 high, padded 8 across, its count 12 and its chevron 12 4 apart in `MutedForeground`, `Foreground` on `SecondaryHover` under the pointer, its ring inside it. A tab with a cap or a width of its own grows to its most as the others do, by as much as it can grow, so that every tab is at its most when the strip has room for all of them so.
- In a title bar the strip sits on the bar's bottom edge, and the bar has no line under it, so the selected tab runs into the content.
- To what reads the tree the list is a `tablist` with the strip's `label`, each tab a `tab` with `a11y_selected`, after what the app said of it with `describe`, and its close button is named by `close_label` and the tab's label. In the list mode each tab says where it is among them all (`a11y_posinset`, `a11y_setsize`), and the More button is named by `more_label`, described by how many it holds ("3 hidden tabs", in the locale's `navigation.hidden_tabs`), and says `a11y_haspopup` and `a11y_expanded`.
- A renderer that draws other window tabs is given the view: `view.part(..)`, `view.tab_part(index, ..)`, `view.tab(index)`, `view.close(index)`, `view.separator()`, `view.add()`, `view.more()`, `view.list()`, `view.hidden` and `view.hidden_count()`, and `view.render()` the default.

## Command Bar

The tabs of a window's commands over the commands of the selected one, in groups with a label under each: the ribbon of an editor or a file manager. It is a `Tabs` over a `Toolbar`: the groups, the overflow, the focus group and the menu are the toolbar's.

```rust
use lurq::ui::{CommandBar, CommandBarCommand, CommandBarTab, Toggle, ToolbarCommand, ToolbarGroup, ToolbarItem, icons};

let clipboard = ToolbarGroup::new()
  .label("Clipboard")
  .item(CommandBarCommand::new("Paste").icon(icons::PASTE).large(true).on_select(paste))
  .item(ToolbarItem::stack([
    CommandBarCommand::new("Cut").icon(icons::CUT).on_select(cut),
    CommandBarCommand::new("Copy").icon(icons::COPY).on_select(copy),
  ]));
let format = ToolbarGroup::new()
  .label("Format")
  .item(ToolbarItem::from(Toggle::new("Bold").icon(icons::BOLD).icon_only(true).on(bold.get())).shortcut("Ctrl+B"));
let insert = ToolbarGroup::new()
  .label("Insert")
  .item(CommandBarCommand::new("Table").icon(icons::TABLE).large(true).on(table.get()).on_change(set_table));

CommandBar::new(tab.clone(), [
  CommandBarTab::new("home", "Home")
    .item(clipboard)
    .item(format)
    .item(insert)
    .secondary_command(ToolbarCommand::new("Customize").on_select(customize)),
  CommandBarTab::new("view", "View").item(view),
])
.mount(ctx);
```

| Builder | |
| --- | --- |
| `CommandBar::new(Signal<T>, tabs)`, `controlled(value, tabs, on_change)` | A bar whose selected tab is the signal's value, or the app's value, told the value of each other tab a person selects. `CommandBar<T = ()>` is generic over the tabs' values |
| `on_change(handler)` | Called with the value of a tab a person selects, after the signal is set. Each one adds |
| `CommandBarTab::new(value, label)` | A tab, with `item(impl Into<ToolbarItem>)` and `items(..)`: what its bar holds while it is selected, as a toolbar has it (labelled `ToolbarGroup`s, `ToolbarItem::stack`, commands, toggles, a select), `disabled`, `id` and `key` |
| `CommandBarTab::secondary_command(ToolbarCommand)`, `secondary_commands(..)` | A command that is never in the bar, only in the menu behind the chevron, after what did not fit; with one the chevron is always there |
| `label(&str)` | What the bar is called to what reads the tree: "Commands". The row of its commands is called by the selected tab |
| `more_label(&str)` | What the chevron at the end is called, in its tooltip and to what reads the tree: "More commands" |
| `disabled(bool)` | Its tabs and its commands take no input, the items of its menu too. State: `CommandBarState::Disabled` |
| `mount(ctx)` | Puts the bar in the tree of `ctx`'s component: its tabs and its toolbar keep what they keep. `build(theme)` and `From<CommandBar> for Element` draw it once with nothing kept: what does not fit is cut off |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the chevron; a command is forced with its own. The tabs have the id with `-tabs` after it, the row of commands `-bar`, the chevron `-bar-more` and its menu `-bar-menu` |

| `CommandBarCommand` | |
| --- | --- |
| `new(&str)`, `icon(name)` | A command: its icon and its label, 24 high |
| `large(bool)` | Its icon of 20 over its label of 11, 56 by 52: what a person reaches for first in a group. State: `CommandBarCommandState::Large` |
| `on(bool)`, `on_change(Fn(bool))` | It is a toggle, picked out while on, and a press tells `on_change` the other value. State: `On` |
| `on_select(handler)` | What running it does: a press, Enter or Space, or its item in the menu. Each one adds |
| `disabled(bool)`, `state`, `style`, `rules`, `force`, `id`, `key` | As a button has them; its item in the menu has the id with `-item` after it |

- What a command bar is that a toolbar is not, as the board draws it: tabs over the commands, each tab a set of its own; groups with a label under each and a line at the end of each, where a toolbar has lines between groups; large commands, the icon over the label, beside stacks of two small ones; and the overflow behind a chevron at the end, a group whole under its label. All of that but the tabs is the toolbar's: `ToolbarItem::stack`, a labelled `ToolbarGroup`, `Toolbar::more_icon` and `Toolbar::secondary_command`.
- Tab stops in the tabs once and in the commands once. Left and Right select the next tab and move between the commands; a disabled one is passed by. What does not fit goes into the menu the last first, a labelled group whole under its label and a toggle checked; a stack goes whole.
- Parts: `CommandBarPart::Root`, `Tabs`, `Tab`, `TabLabel` (laid over the tabs' list, tab and label), `Bar` (laid over the toolbar's root), `More` (laid over the toolbar's More button); `CommandBarCommandPart::Root`, `Body`, `Icon`, `Label`. A command's states: `CommandBarCommandState::Large`, `On`, `Disabled`.
- In the design the bar is a frame of the background's color with an edge of 1 and the large radius. The tabs are a row 32 high on `Bar` over a line of 1, padded 8, 2 apart; a tab is padded 12, its label 13 in the muted foreground, the selected one 600 in the foreground and underlined with 2. The commands are padded 8 and 4; a group is padded 10, its commands 4 apart, its label of 10 in the muted foreground 4 under them, and a line of 1 at its end but the last's. A small command is 24 high, padded 8, its icon of 14 and its label of 12 6 apart; a large one 56 by 52, its icon of 20 over its label of 11; a toggle of its icon alone is 28. At the far end the chevron is 28 by 52, level with the large commands. Under the pointer and while on a command has the fill of what is picked out.
- To what reads the tree it is a `group` called by its label, which holds a `tablist` and a `toolbar` called by the selected tab; a toggled command says `a11y_pressed`.
- A renderer that draws another bar gets a `CommandBarView`: its `tabs` and its `bar`, built.

**In a narrow room.** The bar's groups go behind its chevron, the last first, as a toolbar's controls do; its
tabs, mounted with it, scroll between their buttons when they have no room; and a bar narrower than what stays
in it scrolls sideways.

## Find Bar

The search of what a view shows, hung from the top of the view over its content: the field, the count, the next and the previous match, the options of the search, and a row that replaces.

```rust
use lurq::{components::Stack, ui::FindBar};

Stack::new()
  .child(editor)
  .child(
    FindBar::new(query.clone())
      .open(find_open.clone())
      .matches(current, total)
      .match_case(case.clone())
      .whole_word(whole.clone())
      .regex(regex.clone())
      .replace(with.clone())
      .replace_open(replacing.clone())
      .on_query(move |text| search(text))
      .on_next(next)
      .on_previous(previous)
      .on_replace(move |text| replace_current(text))
      .on_replace_all(move |text| replace_every(text))
      .return_focus(editor_ref.clone())
      .mount(ctx),
  );

// Ctrl+F is the app's to bind: a key handler over the view opens the bar.
view.on_key_down(move |event: KeyboardEvent| {
  if event.ctrl && event.key.eq_ignore_ascii_case("f") {
    event.prevent_default();
    find_open.set(true);
  }
});
```

| Builder | |
| --- | --- |
| `FindBar::new(Signal<String>)` | The bar of what is typed in the signal: the app finds it and gives the bar the count |
| `on_query(handler)` | Called after each edit of the field, with what it holds |
| `on_next(handler)`, `on_previous(handler)` | Going to the next match and the one before: Enter and Shift+Enter in the field, F3 and Shift+F3 in the bar, the arrow buttons |
| `matches(current, total)` | The current match, from 1 (0 where none is), and how many there are: "3 of 12", "No results" for none. Without it the bar says nothing of the count |
| `count_text(Fn(usize, usize) -> String)` | What it says of the count in place of "3 of 12" |
| `match_case(Signal<bool>)`, `whole_word(..)`, `regex(..)` | An option of the search in the field, "Aa", "ab" and ".*", shown where it is given. Each has `_controlled(on, on_change)` and `on_<option>_change(handler)`, as a second value of a component has them |
| `replace(Signal<String>)` | What the matches are replaced with: the replace row under the field, with its buttons |
| `replace_open(Signal<bool>)` | Whether the replace row is shown: the chevron before the field shows it and hides it. Without it the row is always there |
| `on_replace(handler)`, `on_replace_all(handler)` | Told the replace text: its buttons, and Enter in the replace field for the current one |
| `open(Signal<bool>)` | Whether the bar is shown: Escape and the close button set it to false. Without it the bar is always there |
| `on_dismiss(handler)`, `on_open_change(handler)` | Called when a person closes it, after the signal is set |
| `return_focus(ElementRef)` | Where the focus goes as the bar closes with the focus in it: the view it searched |
| `mount(ctx)` | Puts the bar in the tree of `ctx`'s component, which puts the focus in its field as it opens (as its `open` signal turns true, or as it is first rendered where it has none: a bar open from the start leaves the focus where it is) and where `return_focus` says as it closes. `build(theme)` and `From<FindBar> for Element` draw it with nothing kept |
| `disabled(bool)` | Its controls take no input. State: `FindBarState::Disabled` |
| `label`, `placeholder`, `replace_placeholder` | What the bar and its field are called, "Find", and what the fields show: "Find", "Replace" |
| `previous_label`, `next_label`, `close_label`, `match_case_label`, `whole_word_label`, `regex_label`, `replace_label`, `replace_all_label`, `toggle_replace_label` | What its controls are called to what reads the tree |
| `match_case_text`, `whole_word_text`, `regex_text` | What its options show: "Aa", "ab" and ".*" |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws its buttons. Its field has the id with `-field` after it; its buttons `-previous`, `-next`, `-close`, `-toggle-replace`, `-replace`, `-replace-all`; its options `-match-case`, `-whole-word`, `-regex`; the replace field `-replace-field` |

- It is as large as the room it is given and takes no press itself: put it over the view it searches, in a `Stack`. The bar hangs 8 from the top and the end of that room, 440 wide or as wide as the room, comes down into place as it opens and goes back up as it closes.
- The keys act where the focus is in the bar: Enter, Shift+Enter, F3, Shift+F3, Escape. The app binds what opens it (Ctrl+F) and F3 elsewhere.
- Parts: `FindBarPart::Root`, `Bar`, `FindRow`, `ReplaceRow`, `Field` (laid over an input's box), `Options`, `Option`, `OptionOn`, `Count`, `Button`, `ButtonIcon` (laid over a button's root and icon), `Pad`. States: `FindBarState::Closed`, `NoMatches`, `Expandable`, `Replacing`, `Disabled`.
- In the design the bar is of the popover's color with an edge of 1, the large radius and the large shadow, padded 6, its rows 6 apart and their controls 4 apart. The field is an input 28 high with its text of 13, padded 8 before it and 3 after the options, which are 22 square with a glyph of 11 in the muted foreground; one that is on has the fill of what is picked out, an edge in an input's color and its glyph in the foreground. The count is 11 in the muted foreground, padded 6, in the destructive color where nothing matches. The buttons are ghost ones 24 square with icons of 14. The replace row is indented 28 under a bar that has the chevron and ends 52 before the end.
- To what reads the tree it is a `search` called by its label; each option says `a11y_pressed`, the chevron `a11y_expanded`, and the count is `a11y_live` polite.
- The bar counts and the app finds: what it finds it marks on its text with a [Highlight](#highlight) that keeps the text's weight, every match and the current one stronger, which brings the current one into view as it changes. `find_matches(text, query, FindOptions)` finds the ranges of a query under the bar's three options, for an app that searches a plain text.
- A renderer that draws another bar gets a `FindBarView`: `field()`, `options()`, `count()`, `find_row()`, `replace_row()`, `bar()`, and `button(..)` for a button of its own.

**In a narrow room.** A mounted bar measures its field: where the field has no room for the options in it,
they go behind one button (`options_label`, "Search options") whose menu checks the ones that are on; the count
and the arrows stay. Under the least a field is wide, the count and the buttons go on to a line under it. The board draws the bar at 440 with the options in its field and no narrower form: the one button is lurq's.

## Text Editing

What every field of the design system draws while it is typed in. It is the engine's text input, so nothing is
asked for but the two marks an app gives a field: the ranges its spelling checker found, and what would complete
what is typed.

```rust
use lurq::ui::{ContextMenu, Input, MenuItem, Textarea};

// The app's own checker says which bytes of the text are wrong.
Input::new(text.clone()).squiggles(misspelled(&text.get()));
Textarea::new(note.clone()).squiggles(misspelled(&note.get()));

// What would complete what is typed, for what the field holds now. Tab takes it.
Input::new(command.clone()).completion("n --release");

// The commands of an edit in one row over the field.
ContextMenu::new(open.clone(), Input::new(text.clone()))
  .bar(true)
  .items(["Cut", "Copy", "Paste", "Select All"].map(|name| MenuItem::new(name)));
```

| Mark | What draws it | |
| --- | --- | --- |
| Selection | The engine, in `PaletteColor::Selection` | Behind the band of the selected text (the font's ascent and descent), while the field has the focus. `TextInput::selection_color` for one field |
| Caret | The engine, in the color of the text | A line where the next character goes, as wide and as high as [the theme's caret](../theme/#caret) says: the board's 1.5 by 18 in type of 14. It blinks unless the theme's `CaretMode` is `Persistent`. `TextInput::caret_color` for one field |
| Composition | The engine, in the color of the text | A line under what an input method is composing, until it is committed, as thick as the theme's caret says: the board's 1.5 |
| Spell check | `Input::squiggles(ranges)`, `Textarea::squiggles(ranges)` | A wavy line in `Destructive` under each range, bytes of the text, on every row the range is on. lurq has no checker: the app finds the ranges, and again after an edit. The part `Squiggle` (`InputPart`, `TextareaPart`) gives them another color |
| Ghost completion | `Input::completion(&str)` | What would complete what is typed, drawn after it in `MutedForeground` at 0.7 while the field has the focus, holds text, and has the caret at its end with nothing selected. Tab puts it in the field as an edit (through `on_input`, one step of the undo) and the focus stays; anywhere else Tab goes on. The part `InputPart::Completion` gives it another color and opacity |
| Edit menu | [`ContextMenu::bar(true)`](#context-menu) | The commands of selected text in one row over a field |

- On the engine's input the two marks are `TextInput::squiggles(InputSquiggles::new(ranges).color(..))` and `TextInput::ghost(InputGhost::new(text).color(..).opacity(..))`. A masked input draws no squiggle, and an input with tokens or styled spans no ghost.
- The key that takes a completion is not drawn by the input: put a [`Kbd`](#kbd) beside the field where it should be said.
- The board fills a selection with `info-muted`; the engine's is the theme's `Selection` role, which the board has as a variable of its own.
## Tray Popover

The small panel a press of an app's icon in the system tray or the menu bar opens: who the app is and how it is doing, a few switches, a few commands.

```rust
use lurq::ui::{TrayAction, TrayPopover, TrayStatusVariant, TrayToggle, icons};

// sync, paused: Signal<bool>, kept by the app.
TrayPopover::new("lurq")
  .icon(icons::BOX)
  .status("Synced just now")
  .item(TrayToggle::new("Sync in background").bind(sync.clone()))
  .item(TrayToggle::new("Pause notifications").bind(paused.clone()))
  .separator()
  .item(TrayAction::new("Open lurq").on_select(move || show_window()))
  .item(TrayAction::new("Preferences…").on_select(move || preferences()))
  .item(TrayAction::new("Quit").on_select(move || quit()));

// Over the app's own window, beside an element of its own: open: Signal<bool>, anchor: ElementRef.
TrayPopover::new("lurq")
  .status("Offline")
  .status_variant(TrayStatusVariant::Warning)
  .item(TrayAction::new("Retry now").on_select(move || retry()))
  .anchored(open.clone(), anchor.clone());
```

| Builder | |
| --- | --- |
| `TrayPopover::new(&str)` | What the app is called: the title, and what the panel is called |
| `icon(impl Into<Name>)`, `media(element)` | The app's mark before its title: an icon of 16 on a tile of 28 in the primary color, or the app's own picture in place of the tile |
| `status(&str)`, `status_variant(impl Into<Name>)` | How the app is doing, under its title after a dot, and the tone that colors the dot: a `TrayStatusVariant`, `Neutral`, `Info`, `Success` (the default), `Warning`, `Destructive` |
| `item(impl Into<TrayItem>)`, `items(..)` | A row: a `TrayAction` or a `TrayToggle` |
| `separator()` | A line between two groups of rows. The line under the header is there by itself |
| `child(element)`, `children(..)` | Something of the app's own among the rows: a progress, an account |
| `anchored(open, anchor)` | Shows the panel over the window beside `anchor` while the `Signal<bool>` is `true`, in place of where it is declared |
| `placement(Placement)`, `offset(..)`, `collision(CollisionStrategy)` | Where an anchored one stands: under its anchor with its end at the anchor's end (`BottomEnd`; `TopEnd` over a taskbar), 6 from it, to the other side then into the window where there is no room |
| `on_dismiss(Fn())`, `dismiss_on_escape(bool)`, `dismiss_on_outside_press(bool)` | What is told after Escape or a press outside closed an anchored one, and whether each closes it: both by default. A command is no dismissal |
| `width(..)`, `label(&str)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [popover](#popover) has them. `force` draws its first command's states |

| Row | |
| --- | --- |
| `TrayAction::new(&str)` | A row that runs a command: `on_select(Fn())`, `disabled(bool)`, `id`, `key` |
| `TrayToggle::new(&str)` | A row with a switch at its end: `bind(Signal<bool>)`, or `on(bool)` with `on_change(Fn(bool))` told what a press asks for; `disabled(bool)`, `id` (the switch's), `key` |

- Declared plainly it is the panel in the flow, 280 wide at most. **lurq has no tray icon**: the icon and its press are the platform's, or a crate's for it (`tray-icon`). An app that has one opens a window with no frame on the press, `ctx.window_opener().open_with(WindowOptions::new("lurq", 280, 272).undecorated(), ..)`, mounts a root that renders the panel, and closes the window (`WindowHandle::close`) from its commands. What lurq does not have yet for that window is a place on the screen, a level over other windows and a way out of the taskbar: until then the window opens where the system puts it.
- `anchored` shows it as a [popover](#popover) is shown, for a status item of the app's own title bar or status bar: Escape and a press outside close it, the focus goes into it as it opens and back as it closes, and a command closes it first and then runs, so what the command opens keeps the focus it takes.
- A press of a toggle's row turns its switch as a press of the switch does. Bound, the signal is set and then `on_change` is told; not bound, `on_change` is told the value a press asks for and the row stays as the app holds it. Nothing is told when the app sets the signal.
- Every command and every switch is a stop of Tab, and the arrow keys move through them, past the lines. A command is pressed by Enter and Space and has the ring inside it; a row that is `disabled` is faded, takes no press and is passed by.
- A title, a status and a row's words longer than their room are cut with an ellipsis; a switch keeps its room. Rows follow the theme's density: a command is 28 at the default, 20 compact, 36 comfortable and 40 under a finger, and a toggle's row 2 more.
- To what reads the tree it is a `group` (a `dialog` where it is anchored) called by its label or its title; its status is a `status`, a command a `menuitem`, and a toggle its switch.
- Parts: `TrayPopoverPart::Panel`, `Header`, `Media` (the tile), `Icon`, `Text`, `Title`, `Status`, `StatusDot`, `Description` (the status's words), `Separator` and `Line`, `Toggle` and `ToggleLabel`, `Item` (a command), `Body` (what the app puts among the rows). Variants by `TrayStatusVariant`. States: `TrayPopoverState::Disabled`, resolved on a row's parts.
- In the design the panel is 280 wide on `Popover` in an edge of 1 in `Border`, 12 round, with the `Lg` shadow, padded 10, what it holds 8 apart. The header is 4 from the sides: the mark of 16 in `PrimaryForeground` on a tile of 28 in `Primary`, `Md` round, 10 before the title (13, semibold) over the status 1 under it, a dot of 6 in `Success` 5 before words of 11 in `MutedForeground`. A line of 1 in `Border` has 4 over and under it. A toggle's row is 30 and a command 28, both 4 from their sides with words of 13; the command under the pointer is in `Highlight`, `Sm` round. The board draws the tray the popover hangs from, which is the platform's, and no focused or disabled row.

## Path Field

The path of a folder or a file in an input, with a button beside it that browses for one: where a project is kept, the file a report is written to.

```rust
use lurq::{files::FileDialog, ui::{PathField, PathKind}};

// path: Signal<String>. The app answers the button: it opens what it likes and sets the signal.
PathField::new(path.clone()).on_browse(move || choose_a_folder(&path));

// Told what its path is of and mounted, the button opens the system's dialog and sets what is chosen.
PathField::new(folder.clone())
  .picks(PathKind::Folder)
  .dialog(FileDialog::new().title("Project folder"))
  .on_pick(|path| open_project(path))
  .mount(ctx);

PathField::new(report.clone())
  .picks(PathKind::SaveFile)
  .browse_label("Save as…")
  .dialog(FileDialog::new().filter("Markdown", &["md"]).file_name("report.md"))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `PathField::new(Signal<String>)` | A field bound to the path: it shows what the signal holds, and what is typed or chosen is set on it |
| `on_browse(Fn())` | Called on a press of the button, before the system's dialog opens where the field opens one. A second one is called after the first |
| `picks(PathKind)` | What the path is of: `PathKind::Folder`, `File` (one that is there, to open) or `SaveFile`. Mounted, the button then opens the system's dialog for it and sets the path chosen; of what is dropped on the field, a folder or a file of that kind is taken. Without it the button only tells `on_browse`, and a drop is taken whatever it is |
| `dialog(FileDialog)` | The dialog as the button opens it: its title, the kinds of file it shows, the name a file to save starts with. It opens where the field's path is (in the folder itself for a folder, in the file's folder for a file) unless it is given a folder of its own |
| `on_pick(Fn(&str))` | Called with the path a person chose in the dialog or dropped on the field, once the signal is set. Not for what is typed (`on_input`), nor for what the app sets |
| `mount(ctx)` | Puts it where it goes, keeping the dialog it opens: a second press while the dialog is open opens no second one. `build(theme)` and `From<PathField> for Element` are its still form, whose button tells `on_browse` and opens nothing |
| `browse_label(&str)` | What the button says, and is called to what reads the tree: "Browse…" unless told otherwise |
| `icon(..)` | The glyph at the start of the box, by its name in the theme's set: a folder, or a file for a field that picks one, unless told otherwise. `icon(None)` takes it away |
| `placeholder(&str)`, `size(..)`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `full_width`, `label(&str)`, `name(..)`, `on_input(..)`, `on_submit(..)` | As an [input](#input) has them. `PathFieldSize` is the input's sizes, the button's too, and the small one unless told: the board's. A read-only field's button takes nothing; `full_width` is in place of 440 |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the control, what is typed in; the button has it with `-browse` after it |

- It is an [input](#input) and an outlined [button](#button) side by side: Tab stops in the input, then on the button; Enter in the input is told to `on_submit`, Enter and Space on the button press it.
- The dialog is the system's with the feature `file_dialog`, through `lurq::files::FileDialog` (`pick_folder`, `pick`, `pick_save`): see [Files a person gives](../styling-events/#files-a-person-gives) for what each platform has. A browser has no dialog for a folder or for where to save, and no path of a file: there the button tells `on_browse` and nothing is chosen. Under an MCP server with a [`FileDialogBroker`](../mcp-file-dialogs/) the broker answers in the dialog's place.
- A folder or a file dragged from the system and dropped on the field is its path, as one chosen in the dialog is: set on the signal, then told to `on_pick`. A field that is disabled or read only takes none.
- In a [field](#field) it is called by the field's label and judged as the field is: `Field::new("Output folder", PathField::new(path))`, mounted by a field that is mounted.
- Parts: `PathFieldPart::Root` (the two side by side), `Field` (laid over the input's root), `Control` (over the input's box: the type of the path), `Icon` (over the input's icon), `Button` (over the button's root). States: `PathFieldState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`.
- In the design (`Path Field`) it is 440 wide, the input and the button 8 apart and 32 high. The path is in the mono type at 12 in `Foreground`, 10 from the edge of the box, after a folder of 16 in `MutedForeground` and 8; the button is outlined, 12 in from its sides. A medium field is 36 with a path of 13, a large one 40 with a path of 14. A path longer than the box scrolls in it as in any input, and the button keeps its room.
- To what reads the tree the input is a `textbox` called by its label, and the button a `button` called by what it says.
- A renderer that draws another path field (`PathFieldRenderer`) is given `view.input` (the input's own view), `view.field()` (the input in its place, which takes a drop), `view.button()`, `view.root()` and `view.browse()` (what a press of the button does).

## Settings Card

A card of a settings page: one setting with its icon, its title, its description and the control that sets it, which may open to more settings; or the rows of several settings in one card.

```rust
use lurq::ui::{Button, ButtonVariant, ListItem, Select, SelectSize, SettingsCard, Switch, icons};

// One setting in a card of its own. With rows it opens to them; mounted, it keeps whether it is open.
SettingsCard::new(
  ListItem::new("App theme")
    .icon(icons::PALETTE)
    .description("Choose how lurq looks.")
    .trailing(Select::new(theme, themes).size(SelectSize::Sm)),
)
.item(ListItem::new("High contrast").trailing(Switch::new("High contrast").label_hidden(true).bind(contrast)))
.item(ListItem::new("Accent color").value("Blue").chevron(true).href("/settings/accent"))
.mount(ctx);

// Several settings in one card, a row each.
SettingsCard::group([
  ListItem::new("Open at login")
    .description("Start lurq when you sign in to this computer.")
    .trailing(Switch::new("Open at login").label_hidden(true).bind(login)),
  ListItem::new("Default project folder")
    .description("Where new projects are created.")
    .trailing(Button::new("Choose…").variant(ButtonVariant::Outline).size("sm")),
])
.label("Startup");
```

| Builder | |
| --- | --- |
| `SettingsCard::new(ListItem)` | The card of one setting: the row is its own, with its icon, its title, its description and the control that sets it at its end |
| `SettingsCard::group(rows)` | A card of several settings, a row each, with a line between each two |
| `item(ListItem)`, `items(rows)` | Rows, after those it holds: the settings of a group, or what the card of one setting opens to |
| `open(bool)` | Whether the rows under the card's own row are shown, where the app holds it: the card opens nothing itself, and tells `on_open_change` |
| `bind(Signal<bool>)` | Open while the signal is true, which a press of the card's row turns; the handlers of `on_open_change` are told after it is set |
| `default_open(bool)` | Whether a card that keeps it itself starts open |
| `on_open_change(Fn(bool))` | Called on a press of the card's own row with what the card is to be: open if it was closed. Not for what the app sets. A second one is called after the first |
| `mount(ctx)` | Puts it where it goes, keeping whether it is open where the app says neither `open` nor `bind`. Built on its own (`build`, `.into()`) with neither, it is its still form, as it starts |
| `label(&str)` | What the card is called to what reads the tree, in place of the title of its own row. It draws nothing: a heading over a group of settings is the page's own text |
| `density(Density)`, `width(..)` | How high its rows are, whatever the theme is at; a width of its own in place of all the room it is given |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `disabled` takes everything in the card out of input and dims it; `force` draws the card's own row where it opens, and the pressed rows of a group. A row has the id the app gave it |

- A row is a [list item](#list): its `icon`, its `description`, the control at its end (`trailing`), a `value` and a `chevron` for one that leads to a page of its own, which `on_click` or `href` makes pressed. The card puts each in the list item's `Setting` state, which is the design's Settings Row. Nothing else is a settings row: `ListItem::new("Open at login").description("..").trailing(Switch::new(..))` in a `SettingsCard`.
- A card with a row of its own and rows under it opens. Its own row is pressed as a whole, by the pointer and by Enter and Space, after what the app made of its press, and shows a chevron at its end: down while the card is closed, up while it is open. A press of the control in that row is the control's and not the card's, and the control is a stop of Tab after the row. The rows open and close with their height over the theme's base duration and fade with it, as a [collapsible](#collapsible)'s panel does, and at once where motion is reduced; while the card is closed they are not there.
- In a card the rows that are pressed and the controls in rows are one group to Up and Down, Home and End, as in a list.
- Parts: `SettingsCardPart::Root` (the card), `Header` (laid over the root of the card's own row: the room over and under it), `Panel` (what opens and closes), `Body` (what the panel holds, under a line). States: `SettingsCardState::Open`, `Disabled`, `Header` (it has a row of its own). A row's parts are the list item's.
- In the design (`Settings Card`, `Settings Row` and the cards of the Preferences window) the card is on `Card` with an edge of 1 in `Border` and the large radius, as wide as its room. A setting is 14 in from its sides, 12 between what it holds: an icon of 20 in `Foreground`, the title 13 at 500 over the description, 12 in `MutedForeground`, 2 under it, the control, and a chevron of 16 in `MutedForeground` 12 after it. The card's own row is 14 from the top and the bottom of the card, 61 high with a description; a row of a group is 12 from them, 57 high, with a line of 1 in `Border` between each two, along the bottom of the row over it. The board draws no setting of one line and no open card: a setting of one line is 44 at the least, and the rows of an open card are under a line. A longer title is cut with an ellipsis, a longer description wraps, and the control keeps its room. In a room that leaves the title and the description less than 80 beside the control (a card in a window as narrow as a phone), the control and the chevron go under the text, at the row's start, and the description is not broken in its words; a wide room never reaches it.
- A setting follows the [density](../theme/#density) by half of a row's step over and under it: a row of a group with a description is 49, 65 and 69 at compact, comfortable and touch, and one of a single line 36, 52 and 56.
- To what reads the tree (`lurq_snapshot` of the MCP server) the card is a `group` called by its label or the title of its own row, its rows a `list`; the row of a card that opens is a `button` with `a11y_expanded`.
- A renderer that draws another card (`SettingsCardRenderer`) starts from `view.root()` and puts in it `view.header()` (the card's own row with its behavior), `view.panel()` (what opens and closes, with the rows while the card is open) or `view.rows()` (the rows of a group).

## Column View

A hierarchy as columns side by side: choosing an item of one shows what it holds in the next, as a file browser's columns do.

```rust
use lurq::ui::{ColumnView, TreeItem};

// path: Signal<Vec<&str>>, the values from the top down to the item chosen last.
ColumnView::new(path, [
  TreeItem::folder("crates", "crates").items([
    TreeItem::folder("lurq", "lurq").items([
      TreeItem::folder("src", "src"),
      TreeItem::file("Cargo.toml", "Cargo.toml"),
    ]),
  ]),
  TreeItem::folder("docs", "docs"),
])
.label("Files")
.on_activate(|file| open_file(file))
.mount(ctx);

// The path held by the app, which is told each new one.
ColumnView::controlled(app.path(), items, |path| app.go(path));
```

| Builder | |
| --- | --- |
| `new(path, items)` | What is at the top, each a `TreeItem<T>` with what it holds (a [tree view](#tree-view)'s items), and the path chosen, bound to a `Signal<Vec<T>>`: the values from an item at the top down to the item chosen last |
| `controlled(path, items, on_change)` | The path held by the app, which is told `on_change(&[T])` and shows what it gives back |
| `on_change(handler)` | Told the whole new path after a press or a key changes it, after the signal is set; not for the path chosen already, nor when the app sets it |
| `on_activate(handler)` | Told of the item a double press or Enter activates: open the file |
| `press_activates(bool)`, `activate_branches(bool)`, `open_on_hover(bool)` | For a menu of columns, a [cascader](#cascader)'s: a single press activates the item it chooses; an item that holds others is not activated, and Enter goes into it as Right does (`activate_branches(false)`); the pointer on an item that holds others chooses it, opening its column |
| `label(&str)`, `loading_label(&str)` | What the view is called to what reads the tree, and the spinner of a column whose items are on their way, "Loading" |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the root's, and each column has it with `-column-` and its place; `force` draws the item Tab stops on so |
| `mount(ctx)` | In place of converting it: the view keeps where its columns are scrolled to, starts at the end of its path and brings the last column into sight as the path grows |

- The first column is what is at the top; each one after it is what the item chosen in the one before holds. A press of an item ends the path at it. An item on the path is in the highlight and the one the path ends at on the info fill; a value of the path that is no item ends the path before it.
- A `TreeItem` marked `lazy` with no items yet has a chevron and, chosen, a column with a spinner: the app hears the path in `on_change` and gives the item its children. `icon` and `open_icon` (while the item is on the path), `trailing`, `trailing_on_hover` and `disabled` are a tree's.
- The view is one stop of Tab, at the item the path ends at, or the first. The choice follows the focus. Up and Down go through the items of a column, Home and End to its first and its last, without going round and without leaving the column; Right goes into what the item holds, to its first item, and Left back to the item the column is of, where the path then ends. A letter goes to the next item that starts with it, in the column and then in those after it. Enter activates. A disabled item is passed by.
- The columns share the view's width, none narrower than the `Column` part's least width (160), and scroll sideways where they do not fit. A column scrolls down where its items are more than the view is high. The keys keep the item they go to in sight.
- The keys are the engine's: the view is a [focus group](../focus-navigation/#focus-groups) that is `one_stop`, `activate`s, with `typeahead` and without `wrap`, and each item takes the arrows to the item they go to by its key (`KeyboardEvent::focus_key`), which may be in a column the choice has only just shown.
- To what reads the tree it is a `group` called by its label, each column a `listbox` called by the item it is of, each item an `option` with `a11y_selected`, `a11y_level` (its column), `a11y_posinset`, `a11y_setsize` and, where it holds others, `a11y_expanded`.
- Parts: `ColumnViewPart::Root`, `Column`, `Item`, `Icon`, `Label`, `Trailing`, `Reveal` (around what is at the end of an item marked to show it on hover), `Chevron`, `Status` and `Spinner` (of a column whose items are on their way). States: `ColumnViewState::Selected` (on the path), `Current` (the end of the path), `Last` (of a column: the last has no line at its end), `Disabled`.
- In the design the view is 520 by 176 on `Background` in an edge of 1 in `Border`, `radius-lg`. A column is padded 4 with a line of 1 in `Border` at its end, its items 1 apart. An item is 26 high with `radius-sm`, 6 at each side, and holds an icon of 14 and a chevron of 12 in `MutedForeground` and its label in the body's type at 13, 6 apart. On the path it is on `Highlight`; at its end on `Info`, its icon, label and chevron in white. Under the pointer and held down it has the `StateHover` and `StatePressed` veils, the focus is a ring of 2 in `Ring` inside it, and a disabled item is half opaque. An item follows the [density](../theme/#density): 18, 26, 34 and 38 high.
- A theme's renderer is given `ColumnViewView`: `view.columns` (each a `ColumnViewColumn` with its `items`, each a `ColumnViewItem` with its label, icon, whether it opens, is selected, is current, is disabled), `view.root()`, `column.render()`, and on an item `item.button()`, `item.icon()`, `item.label()`, `item.chevron()` and `item.render()`.

## Sortable List

A list whose rows a person puts in another order, by a drag of a row or of its handle and by the keys: the columns of a table, the layers of a scene, the steps of a pipeline.

```rust
use lurq::ui::{SortableItem, SortableList, move_rows};

// sections: Signal<Vec<&str>>, the rows in the app's order.
let order = sections.clone();
SortableList::new(sections.get().into_iter().map(|name| (name, name)))
  .label("Sections")
  .on_reorder(move |from, to| {
    order.update(|rows| {
      let row = rows.remove(from);
      rows.insert(to, row);
    })
  })
  .mount(ctx);

// A selection, whose rows are dragged together.
SortableList::new(rows)
  .selected(selected)
  .on_reorder(|from, to| ..)
  .on_reorder_many(move |rows, to| sections.update(|order| move_rows(order, rows, to)))
  .mount(ctx);

// Rows that hold what is pressed itself are dragged by their grip alone.
SortableList::new(settings.map(|name| SortableItem::new(name, name).trailing(switch(name)))).handle_only(true);
```

| Builder | |
| --- | --- |
| `new(items)` | The rows in the app's order: `(value, title)`, or `SortableItem::new(value, title)` with `description`, `icon`, `trailing`, `disabled`, `id` and `key` |
| `on_reorder(handler)` | Told `(from, to)` when a person moves one row: where it was and where it goes, as it is taken out and put back, `rows.insert(to, rows.remove(from))`, as a [strip of tabs](#window-tabs) tells it. The app reorders its rows |
| `on_reorder_many(handler)` | Told `(&rows, to)` when several selected rows are moved together: where they were, in order, and where the first of them goes in the list without them. `ui::move_rows(&mut list, rows, to)` does it to a `Vec` |
| `selected(Signal<Vec<T>>)`, `selected_controlled(values, on_selected_change)` | The rows that are selected, bound or held by the app. A list without it has no selection. `on_selected_change(&[T])` hears each change a person makes |
| `handle(bool)`, `handle_only(bool)` | Whether each row has its grip (it has unless told otherwise), and whether a row is dragged by its grip alone and not by the rest of it |
| `label(&str)` | What the list is called to what reads the tree |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the row Tab stops on so |
| `mount(ctx)` | In place of converting it: the list keeps a drag from one render to the next, and draws it |

- The list does not keep the order: the app's rows do, and it shows them in the order it is given. A row is a [list item](#list) in the list's dress, so it has a list item's two lines, its veils, its ring and its heights at each [density](../theme/#density).
- A press that moves past a few pixels is a drag. The row follows the pointer over the list, turned a little (`ui::SORTABLE_LIST_TILT`) under the theme's large shadow and narrower than the list by what the `Lifted` part says (40, taken off about the pointer, so what was under it stays under it); what stood in its place is a ghost; and a line with a dot at its start stands in the flow of the list between the rows it would go between, which move apart to make room for it: the room between two rows is a `Slot`, which opens round the line over the theme's base duration in its standard easing, at once where motion is reduced, and closes again when the line goes. The list is as much higher while the line shows. Where letting go would move nothing there is no line. Let go, the app is told where; let go where it was, nothing. Escape puts it back, and the release that follows tells nothing. A drag that moved presses and selects nothing.
- The list is one stop of Tab. Up and Down move the focus through the rows. Alt with Up or Down moves the focused row one place, Alt with Home or End to the first or the last, and the focus goes with it. A disabled row is passed by, is not moved, and other rows are put before and after it.
- With a selection a press selects a row, with Ctrl (Cmd) one in or out and with Shift the rows from the last selected, Space adds the focused row or takes it out, Shift with the arrows selects on the way, and Ctrl+A selects them all. A drag of a selected row, or Alt with a key on it, moves every selected row together: each leaves a ghost, the row under the pointer counts them in a badge on its corner, they land together in their order, and the app is told by `on_reorder_many`. One row moved, selected or not, is told by `on_reorder`.
- Converted into an element on its own the list keeps nothing: a drag still tells where the row was let go, with nothing drawn on the way.
- To what reads the tree it is a `list` of `listitem`s, or with a selection a `listbox` (`a11y_multiselectable`) of `option`s with `a11y_selected`; each row says `a11y_posinset` and `a11y_setsize`.
- Parts: `SortableListPart::Root` (the room between the rows), `Item` (laid over the list item's root), `Handle`, `Icon`, `Title`, `Description`, `Ghost`, `Slot` (the room between two rows, before the first and after the last, where the line shows), `Line`, `Cap` (the dot at the line's start), `Count` and `Lifted` (what holds the row under the pointer). States of a row: `SortableListState::Selected`, `Dragged` (the row drawn under the pointer), `Disabled`.
- In the design the rows are 4 apart, each 36 high on `Card` in an edge of 1 in `Border`, `radius-md`, with a grip of 14 in `MutedForeground` 6 from its start, its text 8 after it at 13 and 10 from its end. The ghost is on `Muted` in an edge of 1 in `Input` at 0.6; the line is 2 in `Info` with a dot of 6, 4 from the row over it and 4 from the row under it, so those two are 10 apart; the row under the pointer is 260 over a list of 300; the count is a disc of 20 in `Info` with its number in white at 11 and 600, 4 past that row's end and 8 over it. A selected row is on `Highlight`.
- A theme's renderer is given `SortableListView`: `view.rows` (each a `SortableRow` with its title, whether it is selected, disabled, a ghost, dragged), `view.list()`, `view.slot(index, count)`, `view.line_at()`, `view.line()`, `view.lifted(row)`, `view.dragged()` and `view.dragged_count()`; on a row `row.handle()` and `row.render()`.

## Selection Marquee

The rectangle a person drags over a surface to select what it touches: the tiles of a grid, the files of a folder, the nodes of a canvas.

```rust
use lurq::ui::SelectionMarquee;

// selected, before: Signal<Vec<usize>>
SelectionMarquee::new(tiles)
  // What was selected when the drag began: what Shift adds to, and what Escape puts back.
  .on_start(move |_| before.set(selected.get_untracked()))
  .on_change(move |marquee| {
    let touched = (0..count).filter(|tile| {
      let (x, y, width, height) = place(*tile);
      marquee.touches(x, y, width, height)
    });
    selected.set(marquee.select(&before.get_untracked(), touched));
  })
  .on_cancel(move || selected.set(before.get_untracked()))
  .mount(ctx);

// In a scroll container the marquee wraps what scrolls, and is given the container's state.
ScrollVertical::new(SelectionMarquee::new(tiles).scroll(&state).mount(ctx)).with_scroll_state(state.clone());
```

| Builder | |
| --- | --- |
| `new(child)` | The app's surface: what the rectangle is dragged over |
| `on_start(handler)` | Told the rectangle once, as a drag becomes a marquee: where the app keeps what was selected before it |
| `on_change(handler)` | Told the rectangle (`&Marquee`) each time it changes, the first too: as the pointer moves, and as the surface scrolls under it |
| `on_change_end(handler)` | Told the last rectangle when the button comes up; not for a marquee Escape ended |
| `on_cancel(handler)` | Told when Escape ends a marquee: where the app puts back what was selected before it |
| `scroll(&ScrollState)` | The state of the scroll container the surface is in: the rectangle keeps its start on the content while the container scrolls under the drag |
| `rect(x, y, width, height)` | Draws the rectangle at a place with nothing dragging it: for a gallery, a test |
| `label(&str)` | What the surface is called to what reads the tree |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the surface's, and the rectangle has it with `-marquee` |
| `mount(ctx)` | In place of converting it: the marquee keeps a drag from one render to the next, and draws the rectangle |

- The marquee draws nothing and takes nothing until a press on its surface moves past a few pixels: a press that does not is the press of what is under it. From then the rectangle runs from where the button went down to the pointer, kept on the surface, and the release presses nothing.
- A `Marquee` is `x`, `y`, `width` and `height` in the surface's own coordinates (from the top left corner of what the marquee wraps, whatever it is scrolled to) and `mode`, what the keys held at the press ask for: `MarqueeMode::Replace` with none, `Add` with Shift, `Toggle` with Ctrl (Cmd). `touches(x, y, width, height)` says whether it shares some room with a box of the surface, `contains(..)` whether the box is wholly in it, `touches_element(&element_ref)` the same of an element of the surface by where it is laid out, and `select(&before, touched)` what is selected with it: the touched alone, `before` and the touched, or `before` with each of the touched put in or taken out.
- What is selected is the app's: the marquee tells the rectangle and keeps no selection. A press of the surface's own room that is no drag is the app's too, which selects none there.
- Escape ends a marquee under way and tells `on_cancel`; the release that follows tells nothing.
- Something on the surface that is dragged itself (it has `on_drag_*` handlers) keeps its drag: a press on it starts no marquee.
- In a scroll container the marquee is what scrolls. A drag held at the container's edge scrolls it, faster the further the pointer is past the edge (the engine's [`drag_autoscroll`](../styling-events/#a-drag-that-scrolls-its-container)), and with the container's state the rectangle keeps its start on the content and grows over what comes into sight.
- Converted into an element on its own the marquee keeps nothing: a drag still tells the app its rectangle, and none is drawn.
- Parts: `SelectionMarqueePart::Root` (the surface, which the recipe gives nothing) and `Marquee`. States: `SelectionMarqueeState::Active` (a rectangle is drawn) and `Disabled`.
- In the design the rectangle is on `InfoMuted` in an edge of 1 in `Info`, the whole of it at 0.55.
- A theme's renderer is given `SelectionMarqueeView`: `view.rect` (the rectangle to draw, if any), `view.root()` (the surface with the drag), `view.marquee()` and `view.render()`.

## Highlight

A text with marks on ranges of it: the matches of a search, one of them the current one.

```rust
use lurq::ui::{FindOptions, Highlight, find_matches};

let options = FindOptions { match_case: case.get(), whole_word: whole.get(), regex: regex.get() };
let found = find_matches(&document, &query.get(), options).unwrap_or_default();
let current = (!found.is_empty()).then(|| current.get().min(found.len() - 1));
// A find in a text keeps the text's weight: it must not wrap again as the query is typed.
Highlight::new(&document).marks(found).current(current).keep_weight(true).mount(ctx);

// A mark in a sentence, its words at the board's 500.
Highlight::new("Results for backdrop blur in docs").mark(12..25);
```

| Builder | |
| --- | --- |
| `Highlight::new(&str)` | The text, with no marks yet |
| `mark(Range<usize>)`, `marks(..)` | A mark on a range of the text, in bytes, after the ones it has |
| `current(Option<usize>)` | Which mark is the current one, by its index: drawn stronger |
| `keep_weight(bool)` | The marks' words keep the text's weight, whatever the recipe gives `Mark` and `Current`: the find in a text. Without it they are the board's 500 |
| `mount(ctx)` | Puts it in the tree of `ctx`'s component, which brings the current mark into view each time it changes (`Ctx::scroll_range_into_view`), and not as the text is first shown. `build(theme)` and `From<Highlight> for Element` draw it with nothing kept |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them; the id is the text's |

| `find_matches(text, query, FindOptions) -> Result<Vec<Range<usize>>, FindError>` | |
| --- | --- |
| `FindOptions::match_case` | The case of the query's letters counts; without it they match whatever their case, of any script |
| `FindOptions::whole_word` | A match has no letter, digit or `_` right before or after it |
| `FindOptions::regex` | The query is a regular expression, with lurq's `regex` feature: `FindError::InvalidPattern` for one that is not, `FindError::RegexUnavailable` without the feature. A match of no length is passed over |

- The marks are the text's highlights (`Text::highlights`, see [Text Styling](../dsl/#text-styling)): they follow their words as the text wraps, and a mark over the end of a line goes on at the start of the next. A mark's words are of the weight of the `Mark` part's style (`Current`'s for the current one), 500 by default: the text is drawn in runs, each mark's words in its weight, and the fill is behind them as they are laid out. Heavier words are wider, so the text wraps otherwise than without the marks: a find in a text, where the marks change with each letter typed, keeps the text's weight with `keep_weight(true)`, and the text is then shaped and laid out as without them. Nothing searches for the app unless it calls `find_matches`: it keeps the query, the matches and the current one.
- Parts: `HighlightPart::Root` (the text's type and color), `Mark` (its fill, its radius, its padding, its words' weight), `Current` (in place of `Mark`, with its edge).
- In the design (`Highlight`) the text is 14 in the foreground and a mark is on the muted warning fill, of a radius of 2, reaching 3 past its words and 1 above and below. The board draws no current mark: it is the mark with an edge of 2 in the warning color, which reads in both appearances where a fill of the warning color would not under the text's own color. A mark's words are 500, as the board's; a face without a 500 draws its nearest, as the system's sans on Windows does with 400.
- A renderer that draws another highlight gets a `HighlightView`: `fill(range, part)` is a mark as a `TextHighlight` in a part's style, `fills()` every one.

## Tree View

Things inside things, one level under the other: the files of a project in their folders, the layers of a scene, settings in their groups.

```rust
use lurq::ui::{TreeItem, TreeView};

// open: Signal<Vec<&str>>, selected: Signal<Option<&str>>
TreeView::new(selected, [
  TreeItem::folder("crates", "crates").items([
    TreeItem::folder("lurq", "lurq").items([
      TreeItem::file("lib.rs", "lib.rs"),
      TreeItem::file("theme.rs", "theme.rs"),
    ]),
  ]),
  TreeItem::file("Cargo.toml", "Cargo.toml"),
])
.label("Files")
.open(open)
.on_activate(|file| open_file(file));

// Any number selected, with Ctrl and Shift.
TreeView::multiple(many, project).open(open);

// The selection held by the app, which is told each new one.
TreeView::controlled(app.selected(), project, |row| app.select(row));

// Children the app brings when the row is first opened.
TreeItem::folder("registry", "registry").lazy(true);

// What stands at a folder's end while its row is pointed at or focused.
TreeItem::folder("src", "src").trailing(new_file_button).trailing_on_hover(true);

// Ten thousand rows: in the engine's virtual list, which builds those in view.
TreeView::new(selected, items)
  .open(open)
  .style(TreeViewPart::Root, |s| s.height(480.0))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `new(selected, items)` | The rows at the top, each a `TreeItem<T>` with what it holds, of which one is selected at most, bound to a `Signal<Option<T>>`. `T` is what rows are told apart by: a path, an id |
| `multiple(selected, items)` | The same of which any number are selected, with Ctrl and Shift as a list of files has them, bound to a `Signal<Vec<T>>` |
| `controlled(selected, items, on_change)`, `controlled_multiple(selected, items, on_change)` | The selection held by the app: an `Option<T>` of one row, told `on_change(&row)`, or the rows of any number, told `on_change(&[rows])`; the tree shows what the app gives back. Two built at the one place in the app's code, in a loop, are told apart by their `id` or `key` |
| `TreeItem::new(value, label)`, `folder(value, label)`, `file(value, label)` | A row; a folder has the icons of a folder and of an open one, a file the icon of a file |
| `TreeItem::icon(icon)`, `open_icon(icon)` | The icon before the label, and another while the row is open: a name, or `None` for none in place of a folder's or a file's |
| `TreeItem::trailing(element)` | What stands at the row's end: a count, a badge |
| `TreeItem::trailing_on_hover(bool)` | What stands at the row's end shows only while the row is pointed at or the focus is in it, and always at the touch density: an action |
| `TreeItem::items(items)`, `item(item)` | What the row holds |
| `TreeItem::lazy(bool)` | It has children the app brings when it is first opened: it opens as a folder does, with a spinner in place of its chevron while it is open with none |
| `TreeItem::disabled(bool)`, `id(id)`, `key(key)` | A row that takes nothing; the id of the row's element; what the row is told apart by among the rows in place of its place, which keeps its node and its focus in a virtual list as rows are put in before it |
| `open(Signal<Vec<T>>)` | The rows that are open. Opening a row adds it, closing one takes it out |
| `open_ids(ids, toggle)` | The same held by the app: `toggle(&value, open)` is told of each row a press or a key asks to open or close |
| `guides(bool)` | A line down each level, under the chevron of the row it is in |
| `label(&str)` | What the tree is called, to what reads the tree. It draws nothing |
| `on_change(handler)` | Told the row a click or Space selects (`&T`), or, of a tree of any number, the whole new selection (`&[T]`) after a click, Space, Shift with a key or Ctrl+A changed it; not for what was selected already, nor when the app sets it. The tree is `TreeView<T, OneSelected>` or `TreeView<T, ManySelected>`, so a handler of another type does not compile |
| `on_activate(handler)` | Told of the row a double click or Enter activates, or in the touch form (`ui::touch_form()`) a tap, never held back for a second: open the file. A row that opens opens or closes as well |
| `on_open_change(handler)` | Told of each row a press or a key opens or closes, `(&value, open)`: where a lazy row's children are brought |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the root; `force` draws the row Tab stops on so, the first selected, else the first that is not disabled |
| `mount(ctx)` | In place of converting it: the tree in the engine's virtual list, for thousands of rows. Give it a height |

- What is open and what is selected are the app's, as an [accordion](#accordion)'s open items are: a signal the tree reads and writes, or the app's own state and a handler. The rows are one flat list of what is shown: what a closed row holds is not built.
- A row opens and closes as an accordion's item does, and as a [property grid](#property-grid)'s category: over the theme's base duration and its standard easing, at once where motion is reduced. What it holds slides open under it and what is below moves with it, its rows fade in and out, and its chevron turns. Pressed again on the way (Left on an opening row, Right on a closing one, its chevron), it turns round from where it is. The keys and the focus work all through it: Up, Down and typeahead pass by the rows of a closing row, which are reached neither by Tab nor by the arrows. In a tree that builds every row, what a row holds is in its panel, whose height runs; in a mounted tree each row's height runs from nothing to its own in its turn, the rows coming into view built as they come and those of a closing row kept, inert, until they have closed, so that a row of thousands opens and closes building only the rows in view, the first of them in turn and the rest with the last. A tree has no drag of its rows, so nothing opens under a drag at rest.
- A click of a row selects it, as the button comes up over the row it went down on, so a press dragged off the row selects nothing; a press of its chevron opens or closes it (and does not select it), a double click activates it. With `multiple`, a press with Ctrl (Cmd) adds the row or takes it out, one with Shift selects the rows from the last selected to it. The last of the selection is where a range starts.
- The tree is one stop of Tab, at the selected row or the first. Up and Down move through the rows that are shown, Home and End to the first and the last, without going round. Right opens a closed row and goes into an open one; Left closes an open row and goes out of a closed one or a leaf to the row it is in; `*` opens every row at the focused one's level. A letter goes to the next row that starts with it. Space selects the focused row (with `multiple` it adds it or takes it out; with Shift it selects to it), Enter activates it. With `multiple`, Shift with Up, Down, Home or End moves and selects the rows on the way, and Ctrl+A selects every row shown. A disabled row is passed by.
- The keys are the engine's: the tree is a [focus group](../focus-navigation/#focus-groups) that is `nested`, `one_stop`, with `typeahead` and without `wrap`, and each row says its level and whether it is open with `focus_level` and `focus_expanded`. A list of items in items (a sidebar's) gets the same keys the same way.
- `mount(ctx)` puts the rows in the engine's virtual list, which builds the rows near its view: a tree of ten thousand rows costs about one screen of them. The keys still reach every row: the tree gives its rows to its focus group ([`FocusRows`](../focus-navigation/#focus-groups)), so Up, Down, Home, End, PageUp and PageDown (a view of rows), Right, Left, `*` and typeahead go to any row, the list builds and shows the row they go to, and the focus goes to it once it is built. Shift with them and Ctrl+A select over every row. The list fills the tree, which is given a height; its rows are drawn by `TreeRow::render`, so a theme's rules reach them and its renderer of the whole tree does not.
- What an item marks with `trailing_on_hover` is in a `Reveal` part, hidden at rest and shown `within` the row: each row is an interaction group (`interaction_group`), lit while the pointer is on it or the focus is in it. At the touch density, where nothing points, it always shows.
- To what reads the tree it is a `tree` (`a11y_multiselectable` with `multiple`) of `treeitem`s, each with `a11y_level`, `a11y_posinset` and `a11y_setsize` among the rows of its level in its row, `a11y_selected`, `a11y_expanded` where it opens and `a11y_busy` while its children are on their way.
- Parts: `TreeViewPart::Root` (the rows, one under the other), `Row` (what is pressed, selected and focused), `Indent` (one step of a row's indent, for each level it is in), `Guide` (the line in a step), `Content` (what a row holds after its indent), `Chevron` (also the room of one in a row that does not open, and the spinner's), `Icon`, `Label`, `Trailing`, `Reveal` (around what is at the end of a row marked to show it on hover), `Panel` (what opens and closes under a row that opens, in a tree that builds every row) and `Body` (its rows, which fade). States of a row: `TreeViewState::Selected`, `Open`, `Disabled`; a row's `Open` gives its panel its height.
- In the design a row is 28 high, 2 under the one before, with `radius-sm` and 8 at each side; each level moves it in by 16. It holds a chevron of 14 in `MutedForeground` (down while it is open, turned to the right while it is closed), an icon of 16 in `MutedForeground` and its label in the body's type at 13, 6 apart. The selected row is on `Highlight`, its label at 500 and its icon in `Foreground`. Under the pointer and held down a row has the `StateHover` and `StatePressed` veils, the focus from the keyboard is a ring of 2 in `Ring` inside it, and a disabled row is half opaque. A guide is a line of 1 in `Border` under the middle of the chevron of the row it is in, as high as a row and the room under it. What is at a row's end is 12 in `MutedForeground`.
- A row follows the [density](../theme/#density): 20, 28 (the design's), 36 and 40 high at compact, default, comfortable and touch, its guide with it.
- A theme's renderer is given `TreeViewView`: `view.rows` (each a `TreeRow` with its label, level, position, whether it opens, is open, is loading, is selected, is disabled, its icon and what is at its end), `view.root()` (the column with the keys and its role) and, on each row, `row.button()` (the behavior with the row's style), `row.indent()`, `row.chevron()`, `row.icon()`, `row.label()` and `row.render()`. It is set for `TreeView<T>` of any `T`: `theme.set_renderer::<TreeView<String>>(..)` draws trees of every id.

## Navbar

The way round a site or an app along the top of its window: the brand at its start, its links, and a search, actions and who is signed in after them.

```rust
use lurq::ui::{Avatar, NavItem, NavLink, NavPanel, Navbar, NavbarBrand, SearchField, TextBoxSize, icons};

Navbar::new()
  .label("Main")
  .brand(NavbarBrand::new("lurq").icon(icons::BOX).on_select(move || go_home()))
  .item(
    NavItem::new("Docs").current(at_docs).panel(
      NavPanel::new()
        .columns(2)
        .link(NavLink::new("Getting started").description("Install lurq and open your first window.").href("/docs"))
        .link(NavLink::new("Theming").description("Roles, palettes and dark mode.").href("/docs/theming")),
    ),
  )
  .item(NavItem::new("Components").href("/components"))
  .item(NavItem::new("Blog").href("/blog"))
  .trailing(SearchField::new(query.clone()).placeholder("Search").size(TextBoxSize::Sm))
  .trailing(account) // an Avatar that opens a Menu
  .sticky(true)
  .mount(ctx)
```

| `Navbar` | |
| --- | --- |
| `brand(element)` | What stands at its start: a `NavbarBrand` (an `icon(name)` or the app's `media(element)`, a logo, beside a name, pressed where it has `on_select`; with `state`, `style`, `rules`, `force`, `id` and `key`), or an element of the app's |
| `item(NavItem)`, `items(..)` | A link after the others; one with a `panel` opens a panel of links, as in a [navigation menu](#navigation-menu) |
| `trailing(element)` | What stands after the links, after what stands there: a [search field](#search-field), a [button](#button), an [avatar](#avatar) with its [menu](#menu) |
| `sticky(bool)` | It stays at the top of the page that scrolls it |
| `open(signal)`, `on_open_change(handler)` | Which item's panel is open, by its index: the app's `Signal<Option<usize>>`; and what hears it change. Without a signal the bar keeps its own |
| `menu_open(signal)`, `on_menu_open_change(handler)` | On a narrow window, whether the links are open over the page; and what hears the menu button turn it |
| `narrow(bool)` | Whether its links are behind the menu button, whatever the window's width: they are below the `md` breakpoint unless told |
| `label(&str)` | What the navigation region is called to tooling: "Main" unless told |
| `menu_label(&str)` | What the menu button of a narrow window is called to tooling: "Menu" unless told. Whether the links are open it says with `a11y_expanded` |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the bar's; its links are `{id}-links` (the items `{id}-links-0` and on, a panel `{id}-links-0-panel`), the menu button `{id}-menu`, the links over the page `{id}-drawer` |

- It is mounted (`mount(ctx)`): its links are a mounted navigation menu, and on a narrow window a mounted [sidebar](#sidebar). Converted into an element on its own it is drawn with what the signals it is given say, for a gallery or a test.
- The links take the room the bar has to spare and no more: where they do not fit they are cut at its end, and never run under what stands after them, which keeps its width. A bar that has more than its window's room for its links is one for `narrow(true)`.
- Its links are a [navigation menu](#navigation-menu) in its links variant: each a stop of Tab, the page shown (`NavItem::current`) in the foreground and the others in the muted foreground, brought forward under the pointer. One with a panel opens it under the bar by a press, under the pointer after a moment, and with Enter, Space and Down, with all a navigation menu does.
- On a window narrower than the `md` breakpoint (or where `narrow(true)` says) the links go behind a ghost button of the `menu` icon at the bar's start, called "Menu" (`menu_label`), into a `Sidebar` over the page, which slides in from the start over a veil: each link an item, the page shown selected, an item with a panel an item that holds its links, open, in from it, with the items after it beside it. Choosing one closes it, as Escape and a press on the veil do. The brand and what stands after the links stay in the bar.
- In the design the bar is 56 tall in `Background`, with a line of 1 in `Border` along its bottom, padded 16 at its sides with 24 between what it holds. The brand is a mark of 20 in `Foreground` 8 before its name in 16/600; the links are 20 apart in 13/500; what stands after them is 24 apart. On a narrow window it is padded 8 with 8 between what it holds.
- Parts: `NavbarPart::Root`, `Brand`, `BrandMark`, `BrandName`, `Links` (the room of the links, which takes what the bar has to spare), `Trailing`. States: `NavbarState::Narrow`, `Sticky`, `Interactive` (the brand is pressed). How its links look is the navigation menu's recipe, in its `Links` variant: a theme restyles them there, into the board's tablet tab bar, say (a bar on `Muted` with the full radius, its items 36 tall and round, the page shown on `Background` with a shadow).
- To what reads the tree the bar is a `navigation` called by its label, its links `link`s (the page shown with `a11y_current` `page`) or `button`s that open a panel, and the menu button a `button` with `a11y_expanded` and `a11y_controls` naming the links over the page.
- A renderer for `Navbar` gets a `NavbarView`: `view.root()` is the bar with its style and its role; `brand`, `links`, `trailing`, `menu_button` and `drawer` are what it holds, built, to put in it. `view.render()` builds the bar.

## Navigation Menu

The way round a site along a row of links, some of which open a panel of links under them: Docs, Components, Blog.

```rust
use lurq::ui::{Card, NavItem, NavLink, NavPanel, NavigationMenu, icons};

NavigationMenu::new()
  .label("Site")
  .item(
    NavItem::new("Docs").panel(
      NavPanel::new()
        .columns(2)
        .featured(Card::new().title("lurq 0.62").description("A navbar, a navigation menu and a sidebar."))
        .link(NavLink::new("Getting started").description("Install lurq and open your first window.").icon(icons::PLAY_CIRCLE).href("/docs"))
        .link(NavLink::new("Theming").description("Roles, palettes and dark mode.").href("/docs/theming")),
    ),
  )
  .item(NavItem::new("Blog").href("/blog"))
  .open(open.clone())
  .mount(ctx)
```

| `NavigationMenu` | |
| --- | --- |
| `item(NavItem)`, `items(..)` | An item after the others |
| `open(signal)` | Which item's panel is open, by its index: the app's `Signal<Option<usize>>`, which a press, the pointer and the keys set, and Escape, a press outside, a choice and the pointer going clear set to `None`. Without one the menu keeps its own |
| `on_open_change(handler)` | What hears a panel open, another take its place, or the open one close |
| `variant(impl Into<Name>)` | A `NavigationMenuVariant`, or a variant a theme's recipe names. `Menu`, the board's: each item a box lit under the pointer and while its panel is open. `Links`: the text alone, as a navbar has it |
| `open_delay(..)`, `close_delay(..)` | How long the pointer rests on an item before its panel opens (the theme's base duration), and how long a panel stays after the pointer has left it and its item (the slow one) |
| `disabled(bool)`, `label(&str)` | No item can be pressed; what the navigation region is called to tooling, "Main" unless told |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the row's; its items are `{id}-0` and on unless they have ids of their own, each panel `{id}-0-panel`, its links `{id}-0-0` and on |

| `NavItem`, `NavPanel`, `NavLink` | |
| --- | --- |
| `NavItem::new(label)` | A link: `href(path)` (the router's), `on_select(handler)`, `current(bool)`, `icon(name)`, `disabled(bool)`, `id`, `key`. With `panel(NavPanel)` it opens a panel in place of going somewhere, and has a chevron |
| `NavPanel::new()` | `link(NavLink)`, `links(..)`; `columns(n)`, the first filled first (one unless told); `featured(element)`, a column of its own before the links: a [card](#card) |
| `NavLink::new(title)` | `description(&str)` under its title, `icon(name)` at its start, `href(path)`, `on_select(handler)`, `current(bool)`, `disabled(bool)`, `id`, `key` |

- It is mounted (`mount(ctx)`): it keeps the timers of its delays, where the pointer is and how the open panel was opened. Converted into an element on its own it opens its panels by a press and the keys, from the signal `open` gives.
- The pointer: a press of an item with a panel opens the panel, and a press again closes it; resting on the item opens it after the open delay (a press that follows keeps it). One panel is open at a time, and with one open the pointer coming onto another item opens that one at once. The panel stays while the pointer is on the item, on the panel or crossing the 6 between them, and closes when it has left them for the close delay; a press outside it closes it and goes on to what it is on. Choosing a link closes the panel first, and then runs it and goes to its path: a handler that opens a dialog keeps its focus.
- The keyboard: each item is a stop of Tab, as links on a page are, and Left and Right move between the items as well. Enter, Space and Down open an item's panel with the focus on its first link (Down on an item whose panel the pointer opened takes the focus into it); Up and Down go through the panel's links, column after column; Left and Right close it and go to the item beside it, opening that one's panel where it has one and only focusing a link, which is not followed; Escape closes it with the focus back on its item. Tab goes from an item into its open panel and from its last link on to what follows the item, closing the panel; the focus leaving the item and the panel closes it. Alt does nothing to it.
- Under the pointer, focused from the keyboard, or with its panel open, an item is filled with `Highlight`, the board's `accent`; the page shown, or an item whose panel holds it, is on `Muted`. In the design an item is 36 tall, padded 12 with the medium radius, its text 13/500 in `Foreground`, and an item with a panel has a chevron of 14 in `MutedForeground` 6 after it, up while its panel is open; the items are 4 apart. The panel opens 6 under its item, from its start, in `Popover` with an edge of 1 in `Border`, the large radius and the medium shadow, padded 8 with 8 between its columns, each 208 wide; it fades in over the theme's base duration and out over the fast one. A link is padded 12 with the medium radius, its title 14/500 over its description 13 in `MutedForeground`, 4 apart and 1.4 high, an icon of 16 in `MutedForeground` 12 before them; it is lit with `Highlight` under the pointer and focused. In the `Links` variant an item is its text alone, 32 tall, 20 apart, in `MutedForeground`, in `Foreground` under the pointer, while its panel is open and where it is the page shown, with the focus ring around it; its panel opens 18 under it, 6 past the edge of a navbar.
- The engine's part is `FocusGroup::panel_row(true)`, which the row has: a [row of menus](#menubar) whose elements are links as well as buttons. Left and Right in an open panel that nothing in it takes close it and focus the element beside it, asking it to open where it says it opens and closes (`focus_expanded(open, toggle)`), and press nothing; Alt is a menubar's, not its.
- Parts: `NavigationMenuPart::Bar`, `Item`, `ItemIcon`, `Label`, `Chevron`, `Viewport` (the room between an item and its panel, which keeps the panel under the pointer), `Panel`, `Column`, `Featured`, `Link`, `LinkIcon`, `LinkText`, `LinkTitle`, `LinkDescription`. States: `NavigationMenuState::Open`, `Current`, `Disabled`; variants: `NavigationMenuVariant::Menu`, `Links`.
- To what reads the tree the row is a `navigation` called by its label (in a navbar, a `list` in the navbar's), an item a `link` (with `a11y_current` `page` for the page shown) or a `button` with `a11y_expanded` and `a11y_controls` naming its panel, the panel a `group` called by its item, and its links `link`s with their description.
- A renderer for `NavigationMenu` gets a `NavigationMenuView`: `view.bar()` is the row with its style, its role and its focus group; `view.item(index)` an item's behavior with its style, `view.item_icon(index)`, `view.label(index)` and `view.chevron(index)` what it holds, and `view.panel(index)` its panel hung from it, to put after it (`view.panel_content(index)` is the panel alone); `view.item_part(index, part)` is the style of a part of one item. `view.render()` builds the menu.

**In a narrow room.** A menu is as wide as its items and no wider than its room: the items it has no room for
go, the last first, behind a More item at its end (`more_label`), which opens a menu of them, a link as an item
(the page shown checked) and an item with a panel as its name over its panel's links. More is an item of the row
to the keys, is drawn as the item of the page shown while that page is behind it, and is gone when every item is
back. A theme's renderer builds the row of `NavigationMenuView::slot` and `more`. Its open panel takes the room a narrow window has: its columns go one under another where two have no room side by side, and none is wider than the window lets it be. The board draws the menu with room for all of it (Navigation Menu, a panel of two columns of 208) and no narrow form: More and the panel of one column are lurq's.

## Table

Rows of the same things in columns: crates and their status, files and their sizes, people and their roles. The columns of the header and of every row are the same widths whatever is in a cell.

```rust
use lurq::ui::{Badge, Table, TableColumn, TableSort};

// sort: Signal<Vec<TableSort>>, selected: Signal<Vec<String>>
let columns = vec![
  TableColumn::new("name", "Name").text(|row: &Crate| row.name.clone()).mono(true).sortable(true),
  TableColumn::new("status", "Status").width(120.0).cell(|row: &Crate| Badge::new(&row.status).into()),
  TableColumn::new("size", "Size").width(70.0).numeric().text(|row: &Crate| format!("{} KB", row.size)).sort_key(|row: &Crate| row.size),
  TableColumn::actions("actions", |row: &Crate| open_button(row)),
];
// The app gives the rows in the order it wants.
let mut rows = rows;
TableSort::sort_rows(&sort.get(), &columns, &mut rows);
Table::new(columns, rows)
  .row_key(|row: &Crate| row.name.clone())
  .label("Crates")
  .sort(sort)
  .selected(selected)
  .height(320.0)
  .on_row_activate(|name| open(name));

// Thousands of rows: mounted, it builds those in view.
Table::new_shared(columns, rows.clone()).row_key(|row: &Crate| row.id).height(480.0).mount(ctx);
```

| Builder | |
| --- | --- |
| `new(columns, rows)`, `new_shared(columns, Arc<Vec<R>>)` | The columns and the rows, in the order the app wants them. Shared rows given again at each render are not copied |
| `TableColumn::new(id, title)` | A column that takes a share of the room the others leave |
| `TableColumn::text(\|row\| String)`, `cell(\|row\| Element)` | A row's cell: a text in the cell's type, or what the app builds (a badge, an avatar and a name) |
| `TableColumn::width(px)`, `share(f32)`, `fit()`, `min_width(px)`, `max_width(px)` | Its width: in pixels, a share of what is left, or as wide as the widest of its cells shown, between a least and a most |
| `TableColumn::align(ColumnAlign)`, `numeric()`, `mono(bool)`, `muted(bool)` | Where its content sits; a column of numbers (at its end, mono 12 in the muted color); its text in the mono type or the muted color |
| `TableColumn::wrap(bool)` | Its text wraps onto as many lines as it needs, and its row grows to hold them, never lower than a row |
| `TableColumn::sortable(bool)`, `sort_by(\|a, b\|)`, `sort_key(\|row\|)` | A press of its header sorts the rows by it; what `TableSort::sort` and `TableSort::sort_rows` compare rows by |
| `TableColumn::pin(ColumnPin)` | Where the column stays while the rows scroll sideways: `Start` (the first columns, after the checkboxes) or `End` (the last columns, a row's actions); `None` unless told |
| `TableColumn::resizable(bool)` | Whether a person makes the column wider or narrower: as the table says unless told |
| `TableColumn::editor(CellEditor)` | How a [data grid](#data-grid) edits its cells; a table shows them |
| `TableColumn::controls(bool)`, `TableColumn::actions(id, cell)` | Its cells hold what is pressed, and a press of it is not the row's; a column of a row's buttons at its end |
| `row_key(\|row\| K)` | What tells a row apart, which the selection and the handlers name it by. Without it, a row's place |
| `sort(Signal<Vec<TableSort>>)`, `sort_controlled(sort, on_change)` | What the rows are sorted by, the first column first, the app's signal or its value: a press of a sortable header sorts by its column alone, through ascending, descending and none; with Shift it adds the column after the others, turns it, then takes it out |
| `on_sort_change(\|sort: &[TableSort]\|)` | Told the new sort at each press of a sortable header, bound or not; a second handler is called after the first |
| `selected(Signal<Vec<K>>)`, `selected_controlled(keys, on_change)` | The rows selected, the app's signal or its value, with a column of checkboxes |
| `on_selection_change(\|keys: &[K]\|)` | Told the whole new selection after a press or a key that changes it, bound or not; not for one that leaves it as it is, nor when the app sets the signal |
| `checkboxes(bool)` | Whether a table whose rows are selected has the column of checkboxes; without it a press of a row selects it |
| `reveal(key, request)` | A mounted table brings the row of `key` into view, once for each new `request`: for a row the app selects from outside the table. It scrolls the least that shows the row and makes it the row Tab stops on; asking again with the same number does nothing |
| `on_row_click(handler)`, `on_row_activate(handler)` | Told of a press of a row, and of a double press or Enter; in the touch form (`ui::touch_form()`) a tap selects and activates the row at once, never held back for a second |
| `density(Density)` | Rows of 36 compact, 44 at the default, 52 comfortable and touch |
| `height(..)`, `max_height(..)` | The table's height: the header stays and the rows scroll under it |
| `empty(element)`, `empty_text(&str)` | What stands under the header where there are no rows: by default an empty state that says "No rows", or the app's words |
| `select_all_label(&str)`, `select_row_label(&str)` | The names of the header's checkbox and of a row's: "Select all" and "Select row" unless told |
| `count_text(\|count: TableCount\| String)` | What the footer's count says, given how many rows it shows (`count.shown`), how many there are (`count.total`) and how many are selected (`count.selected`): "24 rows", "3 of 24 rows selected", filtered "12 of 340 rows" unless told |
| `filters(Signal<TableFilters>)`, `filters_controlled(filters, \|filters\| ..)`, `on_filters_change(\|filters: &TableFilters\| ..)` | What the rows are filtered by: a filter for each column filtered, which the table applies to the rows it is given (see the [data grid](#data-grid)) |
| `loading(bool)`, `loading_rows(n)` | Rows of skeletons under the header while the rows are on their way |
| `pagination(element)`, `count(bool)`, `total(n)` | A footer that counts the rows and what is selected, with the pagination at its end |
| `resizable(bool)`, `on_column_resize(\|id, width\|)`, `on_column_resize_end(\|id, width\|)`, `resize_label(\|title\| String)` | Columns made wider or narrower by a person (off unless told): told while a column moves and when it is done; what a resizer is called, "Resize Name" unless told |
| `mount(ctx)` | The table mounted: it builds the rows and the columns in view and a few around them, and keeps its columns' measured widths and the widths a person made them |
| `label(&str)`, `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. A row's id is the table's with `-row-` and its place; a header's `-header-` and its column's id; the header's checkbox `-select-all` |

- What the rows are, their order, what is selected and what they are sorted by are the app's. A press of a sortable header asks for the rows sorted by it alone, ascending, then descending, then not at all; a press with Shift adds its column to the sort after the others, ascending, or turns it in its place to descending, then takes it out (`TableSort::after_press`). Where the rows are sorted by more than one column, each sorted header shows its place in the sort after its arrow (`TablePart::SortOrder`, mono 11 in `MutedForeground`). The app gives the rows in that order: `TableSort::sort_rows(&sort, &columns, &mut rows)` sorts them by the first column, the rows equal in it by the next, each by its key or its text, and `TableSort::sort` by one. A column's menu sorts by its column alone, and its "clear" takes the column out of the sort.
- With a selection a table has a column of checkboxes. A press of a row selects it alone, with Ctrl (Cmd) adds it or takes it out, with Shift selects the rows from the last selected one to it; a press of its checkbox adds or takes it out, with Shift the range. The header's checkbox selects every row the table shows, or none where all are, and is mixed while some are. A press in a column of `controls` is the control's alone.
- The header stays while the rows scroll in the table's own height, and a table wider than its box scrolls sideways, its header with it, the scroll bars beside the rows and not over the header or the pinned columns: the bar down runs under the header at the table's end edge, over the edge of a column pinned at the end and never over a column that scrolls, the bar across runs between the columns pinned at the start and at the end and has a strip of its own under the rows while they are wider than the box, so no row is under it (a table whose columns fit has none). The first columns pinned at the start stay there after the checkboxes, each after the ones before it, and the last ones pinned at the end stay at the end; a pinned column after the first is best given a width of its own, since where it stays is reckoned from the widths of the pinned ones before it. Converted into an element a table builds every row; mounted (`mount(ctx)`) it builds the rows in view and a few around them, and of those the cells of the columns in view, two past each side, the pinned ones and the focused one, a run of columns not built standing as one room across their tracks (`in_tracks`): a million rows of two hundred columns cost about what a screen of them does, and so does each render. Before it knows how wide it is it builds the columns a wide window shows, not all of them. The keys of the rows are reckoned for the rows built and those a selection or a range names; where a selected key is looked up among the rows (the header's box, a range by Shift), a mounted table makes an index of the keys once for the rows it is given and keeps it while they are the same rows, so give them shared (`new_shared`) where there are many: given anew at each render, the rows are new to it each time. Grouped rows are laid out once for the rows, the columns grouped by and the groups open. Where a column wraps (`TableColumn::wrap`), the rows not built are taken to be a row high, those built are measured as they are laid out, before they are painted, and a row above the view that is higher or lower than it was taken to be moves the scroll by as much: what is in view stays still as heights become known, and the bar's thumb dragged anywhere lands among the rows it says. A table with no height builds every row either way.
- A resizable column has a resizer at the end of its header, in the room after the column: dragged, it makes the column wider or narrower between its least (48 at the least) and its most, and Ctrl with Left or Right on the column's header does it by 10 (50 with Shift). The resizer is a `separator` to what reads the tree, with the column's width as `a11y_valuenow`; its line of 2 in `Ring` shows under the pointer and while it is dragged. A mounted table keeps the widths a person made; a header's title ends in an ellipsis where its column is too narrow for it.
- The columns are one list of tracks for the header and every row (`ColumnTracks`), measured from the rows that are built: a column sized by its content is as wide as the widest of its cells shown so far, and does not narrow as rows scroll by.
- The rows are one stop of Tab, at the focused, the first selected or the first row. Up and Down move through them, Home and End to the first and the last, and in a mounted table PageUp and PageDown by what is shown. Space selects the focused row (adds it or takes it out), Enter activates it, Ctrl+A selects them all. Tab from a row goes into what it holds that takes the focus, its buttons, and then out of the table. The sortable headers and the header's checkbox are a stop of their own before the rows, Left and Right between them. A row the keys reach is scrolled into view clear of the header.
- To what reads the tree it is a `table`, or a `grid` (`a11y_multiselectable`) where its rows are selected, with `a11y_rowcount` (the header's among them) and `a11y_colcount`, `a11y_busy` while loading; its header and rows are `row`s with `a11y_rowindex` (the header's 1), `a11y_selected` where rows are selected; a header is a `columnheader` with `a11y_sort` (`ascending`, `descending`, `none`) where it sorts; a cell is a `cell`.
- Parts: `TablePart::Root` (the box), `Scroll` (what scrolls both ways), `Header`, `HeaderCell`, `SortIcon`, `Resizer` and `ResizeLine` (a column's edge), `MenuTrigger` (what opens a grid's column menu), `DropLine` (where a column dragged by its header goes), `GroupRow`, `Chevron`, `GroupLabel` and `GroupCount` (a group's row and what it shows), `Summary`, `SummaryLabel` and `SummaryValue` (a summary row and what its cells say), `RowNumber` (a row's number and the numbers' header), `Row`, `Cell`, `Select` (the checkbox column's cell), `Editor` (what holds a grid's editor), `Empty`, `Footer`, `Count`. States: `TableState::Selectable`, `Selected`, `Last`, `Sortable`, `Sorted`, `Muted`, `Mono`, `Numeric`, `Pinned`, `Footer`, `Disabled`, and a data grid's range of cells on its cells: `InRange` (the range's fill, on every cell of it but the focused one, the rows of the groups open reckoned as they are shown), `RangeTop`, `RangeBottom`, `RangeStart`, `RangeEnd` (its edge on that side); and on a grid's headers `Open` (its column's menu is open), `DropBefore` and `DropAfter` (a column dragged by its header goes before it, or after the last).
- In the design the table is a box with `radius-lg`, an inner edge of 1 in `Border` and the `Background` fill. The header is 40 high on `Muted` with a line of 1 in `Border` under it, its titles 13/500 in `MutedForeground`; a row is 44 high with the same line under it but the last, its text 13 in `Foreground`. Both are padded 16 at each side with 16 between columns, 12 where the rows have checkboxes, whose column is 16 wide. A sorted column's title is in `Foreground` with an arrow of 14, 4 after it; a sortable one has a muted mark of `chevrons-vertical`, and its title is in `Foreground` under the pointer. A selected row is on `Highlight`. A number is mono 12 in `MutedForeground` at its column's end. The footer is 44 high, padded 16 and 8, its count 13 in `MutedForeground`. Under the pointer and held down a row has the `StateHover` and `StatePressed` veils, the focus from the keyboard is a ring of 2 in `Ring` inside it, and a disabled table is half opaque.
- A theme's renderer is given `TableView`: `view.headers` (each a `TableHeader` with `cell()`, `sort_icon()`, `render()`), `view.rows` (each a `TableRow` with its place, whether it is selected, its `cells` and `button()`, `checkbox()`, `render()`), `view.root()`, `view.header()`, `view.select_all()`, `view.body()` (the rows and the room of those not built), `view.skeletons()`, `view.empty()`, `view.footer()` and `view.tracks`, the list every row of the table lays its cells out in, with `view.columns_built` (which columns a mounted table built) and `in_tracks(built, cells)` to lay a row's cells out with the room of those not built. A header and a cell say their `column`. It is set for `Table<R, K>` of any `R` and `K`, a data grid's too.

**In a narrow room.** A cell's text wider than its column ends in an ellipsis unless the column wraps; the
whole text is what tooling reads of the cell. The lines of a loading table are shares of their cells, so they
end inside a narrow column. A footer too narrow for its count and its pagination puts the pagination under the
count. The shares are the board's (Table/Loading: lines of 150, 190, 120 and 170 in a cell of 176, and 70 in one of 90).

## Charts

What every chart of the system stands on, behind the `canvas` feature: each is drawn on a [canvas](../canvas/), its words (the header, the axis, the legend, the tooltip) are elements in the theme's type, and it is drawn again only when its data, its size, its theme or the point under the pointer changes (`Canvas::paint`), sharp at any scale.

- **Data** is the app's: `ChartSeries::new(name, values)`, a value for each category, or `ChartSeries::points(name, points)`, an x and a y each; numbers are `f64`. A place in it is a `ChartPoint { series, index }`.
- **Colors** come from the theme: a series takes `Chart1` to `Chart5` in turn (`series_color(index)`), unless `ChartSeries::color(..)` gives it a role or a color. The grid, the line the values stand on, the cursor and every text are on roles.
- **The frame** is shared: the card (the card's fill, an edge of 1, the extra large radius, the small shadow), the title over the description, the plot, the axis under it and the legend. `ChartSize::Md` is the board's Bar Chart (padded 24, 20 between rows, a title of 16 over a description of 13, a plot of 160); `Sm` its other charts (20, 16, 15 over 12, a plot of 140).
- **The words** every chart has, as its builder says them: `title`, `description`, `label` (the name to tooling, the title's unless told), `size`, `height` (the plot's), `legend(bool)` (where it has more than one series unless told), `hidden(Signal<Vec<String>>)` with `hidden_controlled` and `on_hidden_change` (the series a press of the legend's item hides, by name), `value_text(|value| ..)` (every number it writes: a tick, a tooltip's value, what tooling reads), `grid(bool)` and `value_axis(bool)` (a line at each tick across the plot, and the ticks' numbers in a room of their own at its start), `empty_text`, `loading`, `on_select(|point| ..)`, and the six of every themed builder. `build(theme)` and `From` give the still chart, drawn and read; `mount(ctx)` the one that follows the pointer and the keys.
- **Scales**: `LinearScale::nice(values, range, count)` maps values onto a length, from a round number at or under the least (zero where none is under it) to one at or over the most, its ticks a step of 1, 2 or 5 times a power of ten apart; `BandScale` lays categories on bands of equal width with a gap between them.
- **The pointer and the keys**: the pointer finds the point it is on, or the nearest one near enough (`nearest`, `nearest_sorted` for a line of thousands of points, by halves), and the tooltip shows it. The plot is one stop of Tab: Left and Right go along a series, Up and Down to the series before and after, Home and End to its ends; the focused point shows what a hovered one shows, and a press, Enter or Space tells `on_select`.
- **To tooling** a chart is a `figure` with its name and its description; its plot is a canvas whose items are its data (a bar's place, its category and its value as written), which a reader goes through, and whose `a11y_value` says the point under the pointer or the focus.
- **Motion**: the plot moves in as it is first shown and to a change of its data (a series hidden or shown again included), for the theme's slow duration (`MotionDuration::Slow`, eased `Standard`), and is drawn again at each frame only while it moves; at rest it is drawn only when it changes. Nothing moves where motion is reduced, nor while the chart is resized, and `motion(false)` keeps a chart still. A chart's plot draws itself on its way (`ChartPlot::paint_moving`, given a `ChartMotion`: what it was, and how far along it is): unless it does, it is revealed from its start as it enters (a line drawn on) and fades to a change. Tooling reads the data where it goes.
- **States**: no data (`empty_text`, "No data" unless told), loading (the skeleton as high as the plot), values under zero, values of very different size (the ticks stay round), a long category name (cut with an ellipsis under its band).
- A plot whose values run across (`ChartPlot::across`, a bar chart lying across) has its grid lines down the plot, its ticks' numbers under it and its categories' names at its start.
- A chart's author writes its plot (`impl ChartPlot`: what it draws, its ticks, its axis, where its points are, what a point's tooltip says, its items, a signature of its data) and lays its own parts over `chart_recipe()`; `ChartProps` and `chart_props!()` give its builder the words above, and `ChartFrame` builds and mounts it. See the module's docs in `lurq::ui`.
- **Its shape**, where a plot is not as wide as the frame with the legend under it: `ChartFrame::plot_width(w)` and `plot_justify(..)` (a pie's or a radar's square, a grid of cells, at the frame's start, middle or end), `legend_beside(values, size)` (the legend beside the plot, one item under another with each value at its end, `ChartPart::Body` the room between them, 20 in `Md` and 18 in `Sm`; the plot's tooltip stands over the legend), `own_legend(element)` (a chart's own legend in place of its series', left out where the app turns the legend off and while loading). A plot whose height follows its width (a heatmap's grid) says it with `ChartPlot::height_for(width)`: the frame lays it out at that height and again when the width it is drawn at asks another. Words in the plot's box are `ChartLabel`s (`ChartPlot::labels`), in the axis's type unless given a part of the chart's own; a tooltip's key in another color than its series' is `ChartPlot::tip_color`. A point with no rows in its tooltip says its title alone to tooling.

### Chart Legend

The board's Chart Legend: a key in each series' color and its name, in a row that wraps.

```rust
use lurq::{app::theme::PaletteColor, ui::{ChartLegend, ChartLegendItem, ChartLegendShape}};

ChartLegend::new([
  ChartLegendItem::new("Layout", PaletteColor::Chart1),
  ChartLegendItem::new("Text", PaletteColor::Chart2),
])
.shape(ChartLegendShape::Dot)
```

- Square keys of 10 with corners of 2 and the names 12 in `Foreground`, 16 between items, 6 between a key and its name; `ChartLegendShape::Dot` has the dots of 8 and the muted names a chart's own legend has. Given `hidden(signal)` (or `hidden_controlled`, `on_hidden_change`), an item is pressed to hide its series and faded while it is hidden, the arrows go between the items, and each is a `checkbox` to tooling.
- `ChartLegendItem::value(text)`: what an item says after its name, mono 11 in `MutedForeground` (a slice's share); tooling reads it with the name. `orientation(Orientation::Vertical)`: the items one under another, each as wide as the legend, its name in `CardForeground` taking the room and its value at the end, as the board's pie has them; `size(ChartSize)` gives them 8 apart and names of 13 in `Md`, 7 and 12 in `Sm`.
- Parts: `ChartLegendPart::Root`, `Item`, `Key`, `Label`, `Value`. States: `ChartLegendState::Dot`, `Pressable`, `Hidden`, `Vertical`.

### Chart Tooltip

The board's Chart Tooltip: what a point is over a row for each series, its dot, its name and its value. A chart shows it at the point under the pointer or the focus; it stands on its own too.

```rust
use lurq::{app::theme::PaletteColor, ui::{ChartTooltip, ChartTooltipRow}};

ChartTooltip::new("Build 0.59")
  .row(ChartTooltipRow::new(PaletteColor::Chart1, "wgpu", "6.9 ms"))
  .row(ChartTooltipRow::new(PaletteColor::Chart2, "DirectX 12", "9.4 ms"))
```

- A panel at least 140 wide on `Popover` with an edge of 1, the medium radius and shadow, padded 8 and 10 with 6 between rows; the title 12/600 in `PopoverForeground`; a row a dot of 7, the name 11 in `MutedForeground` (cut with an ellipsis where it is long, the value keeping its room), the value mono 11. A chart's tooltip stands over its point, under it where the plot has no room above, and inside the plot at its ends.
- Parts: `ChartTooltipPart::Panel`, `Title`, `Row`, `Key`, `Label`, `Value`.

## Bar Chart

A value for each category, as a bar from zero: the frame time of each release, the budget of a frame by phase.

```rust
use lurq::ui::{BarChart, ChartSeries};

BarChart::new(
  ["0.55", "0.56", "0.57", "0.58", "0.59", "0.60"],
  [ChartSeries::new("Frame time", [11.8, 13.2, 9.6, 8.4, 7.0, 5.2])],
)
.title("Frame time")
.description("Median per build, last six releases")
.current(5)
.value_text(|ms| format!("{ms} ms"))
.mount(ctx)
```

| Builder | |
| --- | --- |
| `BarChart::new(categories, series)` | The names under the bars, and the series, a value for each category |
| `stacked(bool)` | The series one on another in each category, the first at the bottom: side by side unless told |
| `current(usize)` | The category that is current (the latest release): its bars in their colors, the others in `Input` |
| `orientation(Orientation)` | Bars standing up from the bottom (`Vertical`, the board's), or lying across from the start (`Horizontal`): the names of the categories at the start, each at its bar's height and at most 160 wide, the ticks' numbers under the plot. Long names read better across |
| `title`, `description`, `label`, `size`, `height`, `legend`, `hidden`, `hidden_controlled`, `on_hidden_change`, `value_text`, `grid`, `value_axis`, `empty_text`, `loading`, `motion`, `on_select`, `state`, `style`, `rules`, `force`, `id`, `key` | As every [chart](#charts) has them |
| `mount(ctx)` | The tooltip of the bar under the pointer or the focus, the arrows between categories and series, `on_select` told a press. Converted into an element it is the still chart |

- The board's Bar Chart (`Sk5h8`): the chart's frame at `Md`, bars 12 apart sharing the plot's width, their tops rounded 4, on the line of the plot's bottom in `Border`; the names under them mono 11 in `MutedForeground`, each under its bar. Several series stand side by side in a category, 4 apart, and the legend under the plot names them in dots. Stacked (`jJ85X`), they stand one on another with a pixel of the card between segments and the top of each stack rounded, 14 between stacks. A value under zero goes down from zero, its bottom rounded, and zero is a line across the plot. The band behind the category under the pointer or the focus is in `Muted`.
- Its motion: the bars grow from the line of zero as the chart is first shown (from its start, lying across), and move from where they were to their new place when the data changes; the bars of a series shown again grow, and those of one hidden shrink away.
- To tooling each bar is an item of the plot: `bar-<series>-<category>`, named by its category, valued as `value_text` writes it.
- Parts: the chart's (`ChartPart::Root`, `Header`, `Title`, `Description`, `Plot`, `Grid`, `Baseline`, `Cursor`, `Axis`, `AxisLabel`, `ValueLabel`, `Empty`) and `BarChartPart::Bar` (the bars that give way to the current one), `Band`. States: `ChartState::Header`, `Empty`, `Loading`, `BarChartState::Stacked`, `Horizontal`, `Vertical`; sizes `ChartSize::Sm`, `Md`.
- Where the board draws nothing: the tooltip of a bar is the board's Chart Tooltip over it; the band behind the category under the pointer; the ring of the focus around the plot; values under zero; the grid and the ticks' numbers, off unless told.

## Line Chart

A value for each category joined by a line, of one series or several: the frame time of each build on two backends.

```rust
use lurq::ui::{ChartSeries, LineChart, LineCurve};

LineChart::new(
  ["0.54", "0.55", "0.56", "0.57", "0.58", "0.59", "0.60"],
  [
    ChartSeries::new("wgpu", [10.4, 9.6, 9.9, 8.2, 8.6, 6.9, 6.3]),
    ChartSeries::new("DirectX 12", [11.6, 11.2, 10.3, 10.6, 9.4, 9.4, 8.6]),
  ],
)
.title("Frame time")
.description("Milliseconds per frame, last 7 builds")
.value_text(|ms| format!("{ms} ms"))
.mount(ctx)
```

| Builder | |
| --- | --- |
| `LineChart::new(categories, series)` | The names of the places along the plot, the first at its start and the last at its end, and the series, a value for each; a value that is not a finite number leaves a gap in its line |
| `curve(LineCurve)` | `Straight`, as the board draws it, or `Smooth`: a monotone curve through every value that never goes past the values beside it |
| `axis_labels(texts)` | The texts under the plot, spread from its start to its end, in place of the categories' names; of more than eight categories eight are written |
| `title`, `description`, `label`, `size`, `height`, `legend`, `hidden`, `hidden_controlled`, `on_hidden_change`, `value_text`, `grid`, `value_axis`, `empty_text`, `loading`, `motion`, `on_select`, `state`, `style`, `rules`, `force`, `id`, `key` | As every [chart](#charts) has them; `size` is `Sm` and `grid` on unless told, as the board draws it |
| `mount(ctx)` | The cursor, the marks and the tooltip of the place under the pointer or the focus, the arrows, `on_select` told a press. Converted into an element it is the still chart |

- The board's Line Chart (`ufn5w`): the chart's frame at `Sm`, a plot of 140 with its lines across at the ticks, lines 2 wide with round joins and ends in the series' colors, the first drawn over the others, the names under the plot from its start to its end, and the legend in dots. At the place under the pointer: a line of 1 in `Input` across the plot (the frame's `ChartPart::Cursor`), a mark of 10 on each line in its color ringed 2 in `Card`, and the Chart Tooltip with every series' value there, 10 before the cursor and 4 from the plot's top (after it where there is no room before it).
- The pointer finds its place at once whatever the number of values (they are evenly apart), and the series whose value there is nearest it. A line of ten thousand values is one path, with no mark of its own for each.
- Left and Right go along the categories, Up and Down between the series, Home and End to the ends; the focused place shows what a hovered one shows.
- To tooling each value is an item of the plot: `point-<series>-<index>`, named by its category (and its series where there are several), valued as `value_text` writes it.
- Parts: the chart's, and `LineChartPart::Line` (its opacity), `Area`, `Point` (the color of a mark's ring). States: `LineChartState::Smooth`, `Stacked`.
- Where the board draws nothing: a smooth curve, gaps, more series, values under zero (zero is a line across the plot), one value (a dot in the plot's middle).
- Its motion: a line is drawn on from its start as the chart is first shown. When the data changes each value moves from where it was drawn to its new place, the line and its curve with it; a value added at the end comes out of the last one before, and one removed goes into the last one left, so the others only slide to their new place; a series shown again rises from the line of zero, and one hidden sinks to it. A gap the data has now is one at once.

## Area Chart

A line chart with the fill under each line: memory over a session, or a whole and its parts stacked.

```rust
use lurq::ui::{AreaChart, ChartSeries};

AreaChart::new(
  ["0", "5", "10", "15", "20", "25", "30", "35"],
  [ChartSeries::new("Resident", [380.0, 520.0, 470.0, 720.0, 670.0, 920.0, 870.0, 1060.0])],
)
.title("Memory")
.description("Megabytes resident during a session")
.axis_labels(["0 min", "10", "20", "30", "40"])
.mount(ctx)
```

| Builder | |
| --- | --- |
| `AreaChart::new(categories, series)`, `curve`, `axis_labels` | As a [line chart](#line-chart)'s |
| `stacked(bool)` | The areas one on another, the first at the bottom, each from the line of the one under it: overlaid unless told. A stacked series' missing value counts as none |
| everything else | As every chart has it, with the line chart's cursor, tooltip and keys |

- The board's Area Chart (`yTmhJ`): a line chart whose lines have the fill under them, down to zero, in the series' colors at 0.16 (`LineChartPart::Area`), the line over it. Its parts and states are the line chart's.
- Where the board draws nothing: stacked areas, several series, the tooltip and the keys (the line chart's).
- Its motion is the line chart's, the fill moving with its line: in a stack each band's bottom moves with the series under it, a series shown again rises from the one under it, and one hidden sinks onto it.

## Scatter Chart

Points by an x and a y on two value axes: how the frame time of a scene grows with its nodes, and what stands apart from the trend.

```rust
use lurq::ui::{ChartSeries, ScatterChart};

ScatterChart::new([
  ChartSeries::points("Scenes", [(550.0, 3.3), (1600.0, 4.3), (3200.0, 8.4), (5000.0, 11.0), (7300.0, 15.5)]),
  ChartSeries::points("Outlier", [(1850.0, 7.6), (6150.0, 14.2)]),
])
.title("Nodes vs frame time")
.description("One dot per scene")
.trend(true)
.x_text(|nodes| format!("{}k", nodes / 1000.0))
.mount(ctx)
```

| Builder | |
| --- | --- |
| `ScatterChart::new(series)` | Series of points (`ChartSeries::points`), each an x and a y, in any order |
| `trend(bool)` | The straight line that fits the first series shown best (by least squares), from its least x to its most, over the points |
| `x_text(Fn(f64) -> String)` | How a number across is written, under the plot and as a tooltip's title: `value_text`'s unless told |
| everything else | As every chart has it; `size` is `Sm`, `grid` on and `legend` off unless told, as the board draws it |

- The board's Scatter Chart (`s9y62`): the chart's frame at `Sm`, a plot of 140 with its lines across at the ticks of the value up, the numbers of the value across under it from its start to its end (both scales start and end on round ticks), dots of 8 in the series' colors at 0.85, and the trend 1 wide in `MutedForeground` at 0.6 over them. The point under the pointer is a dot of 10 ringed 2 in `Card`.
- The pointer finds the point nearest it in two dimensions, within 24. Left and Right go along a series in the order of its x, Up and Down between the series.
- To tooling each point is an item of the plot: `point-<series>-<index>`, named by its series and its x, valued as `value_text` writes its y.
- Parts: the chart's, and `ScatterChartPart::Point` (its opacity), `Ring`, `Trend`. States: `ScatterChartState::Trend`.
- Where the board draws nothing: the tooltip (the board's Chart Tooltip over the point, its x as its title), the ring of the point under the pointer, a legend (off unless told), values under zero, several series.
- Its motion: the points grow in at their place as the chart is first shown. When the data changes each point moves from where it was drawn to its new place (a point is the one of its series at its place by x), and the trend with them; a point that is new grows at its place and one no longer there shrinks away at its old one, as do the points of a series shown or hidden; a trend that comes or goes fades.

## Pie Chart

The parts of a whole as slices of a disc, or of a ring: the crates by size, the phases of a frame. The legend beside it names each slice and writes its share.

```rust
use lurq::ui::{ChartSeries, PieChart};

PieChart::new(
  ["render", "layout", "text", "other"],
  ChartSeries::new("Size", [4.2, 2.8, 1.9, 1.1]),
)
.title("Crates by size")
.description("Share of compiled code")
.value_text(|mb| format!("{mb} MB"))
.mount(ctx)
```

| Builder | |
| --- | --- |
| `PieChart::new(names, series)` | The slices' names, and one series with a value for each, from the top and clockwise |
| `donut(bool)` | A ring with a hole of 0.66 of its radius, the board's Donut Chart, `Md` unless told a size; a pie, `Sm`, unless told |
| `total_text(&str)` | The words a donut writes in its hole under the total of the slices shown (written with `value_text`); nothing in the hole unless told |
| `colors(..)` | Each slice's color, a role or a color, in place of `Chart1` to `Chart5` in turn |
| `legend_value(PieLegendValue)` | What each item of the legend writes after the name: `Share` (unless told), `Value`, `None` |
| `share_text(\|share\| ..)` | How a share from 0 to 1 is written in the legend and to tooling: a whole percent, `<1%` for a sliver, unless told |
| `title`, `description`, `label`, `size`, `height`, `legend`, `hidden`, `hidden_controlled`, `on_hidden_change`, `value_text`, `empty_text`, `loading`, `on_select`, `state`, `style`, `rules`, `force`, `id`, `key` | As every [chart](#charts) has them; `height` is the pie's side, `hidden` the slices hidden by name, `legend` on unless told |
| `mount(ctx)` | The slice under the pointer or the focus picked out with its tooltip, the arrows round the slices, `on_select` told a press (`ChartPoint { series: 0, index }`). Converted into an element it is the still chart |

- The board's Pie Chart (`dDHPj`): the chart's frame at `Sm`, a pie of 96 from the top and clockwise, a line of 1.5 of `Card` on each side of the line between two slices; the legend beside it, 18 away, its items 7 apart, each a dot of 8, the name 12 in `CardForeground` and the share mono 11 in `MutedForeground` at its end. The Donut Chart (`GJ5Mg`): `Md`, a ring of 96 with a hole of 0.66 and no lines between slices, the legend 20 away with names of 13 and items 8 apart.
- A slice hidden through the legend leaves the whole to the others, whose shares are written again. A slice too narrow at the rim to keep its color between two lines joins its neighbour without one, and is still in the legend (`<1%`), the tooltip, the keys and to tooling; one slice is the whole disc; a value under zero or not a number is no slice; all zero is no data (`empty_text`).
- Motion: the slices sweep in from the top, clockwise, as the pie first shows; on a change each slice's ends move from where they were to where they are, so a slice hidden through the legend closes and the others take its room, and one shown again opens. Off where motion is reduced or the chart is told `motion(false)`, ended by a resize, nothing drawn at rest.
- To tooling each slice is an item of the plot at its middle: `slice-<index>`, named by its slice, valued as `value_text` writes it with its share.
- Parts: the chart's and `PieChartPart::Slice` (a slice that gives way to the one under the pointer or the focus: its opacity, 0.45), `Edge` (the line between slices, `Card`), `Total` (15/600 in `CardForeground`), `TotalLabel` (mono 10 in `MutedForeground`). States: the chart's and `PieChartState::Donut`.
- Where the board draws nothing: the slice under the pointer or the focus picked out by the others giving way to it, with the board's Chart Tooltip at its middle titled by the series; the total in a donut's hole; the ring of the focus around the plot.

## Radar Chart

A value for each of several axes round a center, a polygon for each series: the profile of a renderer, the strengths of two backends side by side.

```rust
use lurq::ui::{ChartSeries, RadarChart};

RadarChart::new(
  ["Draw", "Text", "Layout", "Memory", "Startup", "Input"],
  [
    ChartSeries::new("wgpu", [92.0, 70.0, 64.0, 58.0, 75.0, 66.0]),
    ChartSeries::new("DX12", [68.0, 76.0, 70.0, 80.0, 62.0, 86.0]),
  ],
)
.title("Renderer profile")
.description("Relative score per area")
.mount(ctx)
```

| Builder | |
| --- | --- |
| `RadarChart::new(axes, series)` | The axes' names, the first at the top and the others clockwise, and the series, a value for each axis |
| `value_axis(bool)` | The rings' numbers beside the first axis, each under its ring; off unless told |
| `title`, `description`, `label`, `size`, `height`, `legend`, `hidden`, `hidden_controlled`, `on_hidden_change`, `value_text`, `empty_text`, `loading`, `on_select`, `state`, `style`, `rules`, `force`, `id`, `key` | As every [chart](#charts) has them; `height` is the square's side |
| `mount(ctx)` | The series and the axis under the pointer or the focus picked out with the tooltip of every series' value on that axis; Left and Right round the axes, Up and Down between the series, `on_select` told a press (`ChartPoint { series, index: axis }`). Converted into an element it is the still chart |

- The board's Radar Chart (`cokgK`): the chart's frame at `Sm`, the plot a square of 150 (180 in `Md`) in the middle of the frame; the outer ring 15 inside the square, the rings at the scale's round ticks (from zero, or the least value under it, to a round number at or over the most, about three) and the spokes, 1 in `Border`, polygons through the axes (circles where there are fewer than three axes); each series' area and its edge of 1.5 in its color at 0.2; the legend under the plot in dots where there is more than one series.
- The axes' names, mono 11 in `MutedForeground`, stand 6 outside the outer ring, away from the center: over the top axis, before a left one, after a right one.
- The pointer finds the axis nearest its direction and on it the series whose value is nearest; that series' area is at 0.35 with its edge in its full color and drawn over the others, its point a dot of 3.5 ringed in `Card`, the axis's spoke in `Input`.
- A missing value (not a number) stands at the center, and has no point to tooling or in the tooltip.
- Motion: the polygons grow from the center as the chart first shows; on a change each corner moves from where it was to its new value (on the scale it was on, to the one it is on), a series shown again grows from the center and one hidden shrinks to it. Off where motion is reduced or the chart is told `motion(false)`, ended by a resize.
- To tooling each value is an item of the plot: `point-<series>-<axis>`, named by its axis and its series, valued as `value_text` writes it.
- Parts: the chart's and `RadarChartPart::Ring` (the rings and the spokes), `Area` (a series' opacity), `ActiveArea` (the opacity of the series under the pointer or the focus).
- Where the board draws nothing: the axes' names (the board has none, and the square is in the middle of the frame so that they have room on both sides); the picked-out series and axis with the board's Chart Tooltip; the rings' numbers, off unless told; the ring of the focus around the plot.

## Heatmap

A grid of cells, a row for each series and a column for each category, each colored by its value's step on a scale: commits per day, temperature by hour.

```rust
use lurq::ui::{ChartSeries, Heatmap};

Heatmap::new(
  weeks, // the columns' names
  ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map(|day| ChartSeries::new(day, commits_on(day))),
)
.title("Commits")
.description("Per day, last 18 weeks")
.mount(ctx)
```

| Builder | |
| --- | --- |
| `Heatmap::new(columns, rows)` | The columns' names, and the rows, each a series with a value for each column |
| `steps(usize)` | How many steps the scale has, two at least: five unless told, the board's |
| `range(least, most)` | The values the scale runs between: zero (or the least value, where one is under zero) to the most unless told |
| `cell_size(f32)` | A cell's side: 14 unless told; smaller where the grid is wider than the frame |
| `column_axis(bool)` | The columns' names under the grid, as many as have room; off unless told, as the board's |
| `less_text`, `more_text` | The scale's words at its ends: "Less" and "More" unless told |
| `missing_text` | What the tooltip and tooling say of a cell with no value: "No value" unless told |
| `title`, `description`, `label`, `size`, `height`, `legend`, `value_text`, `empty_text`, `loading`, `on_select`, `state`, `style`, `rules`, `force`, `id`, `key` | As every [chart](#charts) has them; `height` makes the cells as tall as the rows can be in it, `legend` is the scale |
| `mount(ctx)` | The cell under the pointer or the focus ringed with its tooltip; the arrows from cell to cell, Left and Right along a row and Up and Down along a column; `on_select` told a press (`ChartPoint { series: row, index: column }`). Converted into an element it is the still chart |

- The board's Heatmap (`rSu67`): the chart's frame at `Sm`; the rows' names mono 10 in `MutedForeground` at the plot's start, every other one where more than four rows are as close as the board's; cells of 14 with corners of 3, 4 apart; the scale under the grid at the frame's end, its words mono 11 in `MutedForeground` 6 from its steps of 12 with corners of 3 (`FGewe`).
- The scale's steps are measured from the board: a value at or under the scale's least is in `Muted`; the others are `Chart1` at a quarter, a half, three quarters and the whole of its opacity (with five steps), drawn over the card. A value's step is as high as its share of the range, rounded up, so that any value over the least shows. A cell and the scale's step of its value are drawn in one color, so a cell is the scale's color at its value.
- A cell with no value (not a number) is an edge of 1 in `Border`; the tooltip and tooling say `missing_text`. A grid wider than the frame has smaller cells, its gaps as much smaller, and the plot is as high as the cells are drawn at its width, laid out again as the width changes (the rows' names fewer where they would crowd).
- Motion: the cells fade in as the heatmap first shows; on a change each cell goes from its old step's color to its new one, a cell that comes fading in and one that goes fading out. Off where motion is reduced or the chart is told `motion(false)`, ended by a resize.
- To tooling each cell is an item of the plot: `cell-<row>-<column>`, named by its row and its column, valued as `value_text` writes it. The scale is an `img` named from its two words.
- Parts: the chart's and `HeatmapPart::Cell` (the lowest step, `Muted`), `Fill` (the highest, `Chart1`), `Missing`, `Ring` (round the cell under the pointer or the focus, 1.5 in `Foreground`), `Scale`, `Step`, `ScaleLabel`.
- Where the board draws nothing: the tooltip of a cell, its key in the cell's color; the ring of the cell under the pointer or the focus; the columns' names, off unless told; a cell with no value. The board names Mon, Wed and Fri of its seven days; the heatmap names every other row from the first, so Sun as well.

## Data Grid

A table whose cells are worked in: the focus is on a cell, the arrows move it from cell to cell, and a cell is edited in place with the system's own controls. A list of files whose names are edited, a sheet of orders whose states are chosen.

```rust
use lurq::ui::{CellEditor, CellValue, DataGrid, TableColumn};

// files: Signal<Vec<File>>, the app's rows
let columns = vec![
  TableColumn::new("name", "Name").text(|row: &File| row.name.clone()).editor(CellEditor::text(|row: &File| row.name.clone())),
  TableColumn::new("kind", "Kind").width(120.0).text(|row: &File| KINDS[row.kind].into()).editor(CellEditor::choice(KINDS, |row: &File| row.kind)),
  TableColumn::new("size", "Size").width(90.0).numeric().text(|row: &File| format!("{} KB", row.size)).editor(CellEditor::number(|row: &File| row.size)),
  TableColumn::new("shared", "Shared").width(80.0).editor(CellEditor::checkbox(|row: &File| row.shared)),
];
let store = files.clone();
DataGrid::new(columns, files.get())
  .row_key(|row: &File| row.name.clone())
  .label("Files")
  .selected(selected)
  .height(320.0)
  .on_edit(move |edit| {
    let mut files = store.get();
    let Some(file) = files.iter_mut().find(|file| &file.name == edit.row) else { return false };
    match (edit.column, &edit.value) {
      ("name", CellValue::Text(name)) if name.is_empty() => return false, // refused: the editor stays open
      ("name", CellValue::Text(name)) => file.name = name.clone(),
      ("kind", CellValue::Choice(kind)) => file.kind = *kind,
      ("size", CellValue::Number(size)) => file.size = *size,
      ("shared", CellValue::Checked(shared)) => file.shared = *shared,
      _ => return false,
    }
    store.set(files);
    true
  })
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `DataGrid::new(columns, rows)`, `new_shared(columns, Arc<Vec<R>>)`, `row_key(\|row\| K)` | A table's columns and rows, and what tells a row apart |
| `TableColumn::editor(CellEditor)` | How the column's cells are edited: `CellEditor::text(\|row\| String)` (an `Input`), `number(\|row\| f64)` (a `NumberInput`), `choice(choices, \|row\| usize)` (a `Select` of the choices), `checkbox(\|row\| bool)` (a box ticked in the cell, which a column that builds no cell of its own shows). A column with none is read |
| `on_edit(\|edit: &CellEdit<K>\| bool)` | Told each edit committed: `edit.row` (the row's key), `edit.column` (the column's id), `edit.value` (a `CellValue`: `Text`, `Number`, `Choice`, `Checked`, or `Empty` for a number or a choice cleared). It returns whether the app takes it; every handler is told, and one refusing it refuses it. A paste and Delete come here a cell at a time |
| `column_layout(Signal<ColumnLayout>)`, `column_layout_controlled(layout, \|layout\| ..)`, `on_column_layout_change(\|layout: &ColumnLayout\| ..)` | How the columns are arranged, the grid's own unless the app holds it: `layout.columns`, a `ColumnPlace` for each in the order shown (`id`, `hidden`, `width` where a person made it, `fit` where it is as wide as its content, `pin`). The grid tells the whole of it at each change, so an app that stores it gives back what a person left; a column it does not name is as the app made it, after the others |
| `column_menu(bool)` | A menu on each header: on unless told |
| `row_numbers(bool)` | A column of the rows' numbers at the start: off unless told |
| `TableColumn::filter_text(bool)`, `filter_set(bool)`, `filter_values(\|row\| String)`, `filter_number(\|row\| f64)`, `filter_date(\|row\| Option<Date>)`, `filter_bool(\|row\| bool)` | How a column is filtered: by its text (contains, starts with, equals), by the values ticked among those it has (its text, or what `filter_values` gives a column whose cell is an element), by a number or a day (a range, or a comparison), by a yes or no |
| `filters(Signal<TableFilters>)`, `filters_controlled(filters, \|filters\| ..)`, `on_filters_change(\|filters: &TableFilters\| ..)` | The filters, the grid's own unless the app holds it: a person sets them from the column's menu and the filter row |
| `filter_row(bool)`, `filter_labels(FilterLabels)` | A row of filters under the header: off unless told; and every word of filtering |
| `search(Signal<String>)`, `search_controlled(query, \|query\| ..)`, `on_search_change(\|query: &str\| ..)`, `search_options(FindOptions)`, `on_matches_change(\|matches: SearchMatches\| ..)` | What is searched for in the cells, every match marked; how the query matches; told how many matches there are and which is current |
| `DataGrid::on_demand(columns, total, \|request: GridRequest\| async { .. })`, `on_query_change(\|query: &GridQuery\| ..)`, `source_labels(SourceLabels)` | A grid over rows the app gives on demand: see below |
| `tree(\|row\| Option<K>)`, `expanded(Signal<Vec<K>>)`, `expanded_controlled(open, \|open\| ..)`, `on_expanded_change(\|key, open\| ..)` | The rows as a tree, each under the row whose key its parent is; the rows open, the grid's own unless the app holds them (every row closed at first) |
| `detail(\|row\| Element)`, `details(Signal<Vec<K>>)`, `details_controlled(open, \|open\| ..)`, `on_detail_change(\|key, open\| ..)` | What a row shows under it while it is open; the rows whose detail is open, the grid's own unless the app holds them |
| `toolbar(bool)`, `toolbar_labels(GridBarLabels)` | The bar of the grid's tools over it, off unless told: see below; and its words (`GridBarLabels { bar, search, filter, columns, export, density, comfortable, default, compact }`) |
| `export(bool)`, `on_export(\|export: &GridExport\| ..)`, `export_labels(ExportLabels)` | Export in each column's menu: the rows shown or selected written out as CSV or tab-separated text, on unless told; told the text (`GridExport { format, scope, text, rows }`), which goes on the clipboard where no one is told; and its words. `to_csv(&columns, &rows)`, `to_tsv` and `to_delimited(&columns, &rows, ExportFormat)` write any rows in a grid's columns, the grid or not |
| `find_bar(bool)`, `find_bar_with(\|bar: FindBar\| bar)` | Ctrl+F opens a find bar over the grid: off unless told; and what the bar is made of besides its query and its count (its words) |
| `column_menu_labels(ColumnMenuLabels)`, `menu_trigger_label(\|title\| String)`, `group_label(\|title, value, count\| String)`, `summary_labels(SummaryLabels)`, `row_numbers_label(&str)` | The words the grid writes, each said otherwise: the column menu's items (`ColumnMenuLabels { sort_ascending, sort_descending, clear_sort, pin_start, pin_end, unpin, size_to_fit, hide, columns, show_all }`, English by `..ColumnMenuLabels::default()`), the name of a menu's trigger ("Name column menu"), of a group's row ("Status: Passing, 3 rows"), the summary's words (`SummaryLabels { sum, average, count, min, max, row }`) and the numbers' mark ("#") |
| `TableColumn::summary(SummaryKind)`, `TableColumn::summary_text(\|rows: &[&R]\| String)` | What a column says in the summary rows: `SummaryKind::Sum`, `Average`, `Count`, `Min`, `Max`, which the grid computes from the values the column's editor edits or the numbers its text starts with ("12 KB"), a count of any column; or the app's text of the rows summed up. A grid with a column that says one has a summary row |
| `group_by(["status", ..])`, `open(Signal<Vec<GroupKey>>)`, `open_controlled(open, \|open\| ..)`, `on_open_change(\|key: &GroupKey, open\| ..)` | The rows grouped by the values of these columns (a column grouped by may be hidden), and the groups open, the grid's own unless the app holds them (every group closed at first). A `GroupKey` is the values from the first column grouped by to the group's own: `GroupKey::new(["Passing"])`, `GroupKey::new(["Passing", "Ada"])` |
| `cell_selection(Signal<Option<CellRange<K>>>)`, `cell_selection_controlled(range, \|range\| ..)`, `on_cell_selection_change(\|range: Option<&CellRange<K>>\| ..)` | The range of cells selected, the grid's own unless the app holds it: `range.anchor` and `range.extent` its corners (a `CellKey`: `row`, the row's key, and `column`, the column's id), `range.rows` and `range.columns` the keys and the ids it spans in the order shown. `CellRange::new(anchor, extent)` makes one to give the grid |
| `read_only(bool)` | No cell is edited: the cells take the focus and are read |
| `resizable(bool)` | Columns made wider or narrower by a person: on unless told |
| `mount(ctx)` | The grid that works: its cells take the focus and are edited, and it builds the rows and the columns in view. Converted into an element it is the still form of itself, read and selected as a table |
| `sort`, `sort_controlled`, `on_sort_change`, `selected`, `selected_controlled`, `on_selection_change`, `checkboxes`, `on_row_click`, `on_row_activate`, `on_column_resize`, `on_column_resize_end`, `density`, `height`, `max_height`, `empty`, `empty_text`, `loading`, `loading_rows`, `pagination`, `count`, `total`, `count_text`, `select_all_label`, `select_row_label`, `resize_label`, `label`, `disabled`, `state`, `style`, `rules`, `force`, `id`, `key` | As a [table](#table) has them. A cell's id is the grid's with `-cell-`, its row and its column (`files-cell-2-0`); the editor's is the grid's with `-editor` |

- The grid is one stop of Tab after its header (whose sortable and resizable headers are a stop of their own), at the cell that last had the focus or the first cell of the first selected row. Left and Right move along the row, Up and Down to the same column of the row before and after, Home and End to the row's first and last cell, Ctrl+Home and Ctrl+End to the first cell of the first row and the last of the last, PageUp and PageDown by what the grid's height shows; a row or a column that is not built is scrolled to, built and focused. Tab goes on to the next cell of the row, Shift+Tab to the one before, and out of the grid from the row's last and first. Space selects the row (with Shift from the last selected one), Enter on a cell that is not edited activates its row, as a double press of it does. A press of a cell focuses it and does not select its row: the rows are selected with their checkboxes and Space.
- Filters: a column says how it is filtered (`TableColumn::filter_text` and the others), and its menu has "Filter…", which opens a panel under its header (a choice of contains, starts with or equals and a box for a text; the values the column has, each ticked, with "Select all" and "Select none", at most 500 of them; boxes of at least and at most for a number, of from and to for a day; a choice of all, yes or no), "Clear filter" and "Clear all filters". A change applies as it is made. With `filter_row(true)` a row under the header, which stays with it, has a filter in the cell of each column that filters: a box for a text (contains), for a number or a day (`>10`, `<=5`, `5..20`, `=7` or one value; `>` and `<` are taken as at least and at most), a choice of a value or of yes and no. A filtered column's header has the funnel after its title (`TablePart::FilterIcon`, `icons::FILTER`). The grid shows the rows every filter lets through, in their order: grouped, the groups are of those, and a summary is of them; the count reads "12 of 340 rows". `TableFilters` is the app's value (`with`, `without`, `get`, `iter`, `valid(ids)`), its text `filter("name" contains "lurq") filter("size" at_least:10) filter("status" in "Passing" "Failed") filter("updated" from:2026-01-01 to:2026-02-01) filter("shared" is:true)` (`to_text`, `parse`, `Display`, `FromStr`) and, with `persistent_storage`, a value of the storage. A filter is a `ColumnFilter`: `Text { how: TextMatch, text }`, `Number { at_least, at_most }`, `Date { from, to }`, `Set(values)`, `Bool(value)`. Parts: `TablePart::FilterRow` (44 high on `Background`, a line under it), `FilterLabel` (the panel's words over its boxes, 12 in `MutedForeground`).
- Rows on demand, as a server's table gives them: `DataGrid::on_demand(columns, total, fetch)` is told how many rows there are at first and asks `fetch` for the ranges of the rows built, a hundred at a time, four at once, through the engine's futures; `fetch` answers now or later with a `GridPage { rows, total }` (the rows of the range, fewer at the end, and how many there are as the query has them) or an `Err(message)`. A row whose range has not come is a skeleton; a range that failed says so in each of its rows, with a Retry (`SourceLabels`). The rows are `SourceRow<R>`, each its row where it has come (`get()`), told apart by their place. The sort, the filters and the search are the app's: each `GridRequest` carries them as one `GridQuery { sort, filters, search }`, another of them asks for every range again, and `on_query_change` is told it as a person changes one. `GridQuery` has a text form, `sort("size" descending) filter("status" in "Failed") search("canvas")` (`to_text`, `parse`, `Display`, `FromStr`), and is a value of the storage. Over a source the cells are not edited and the rows are not grouped or summed, and the grid does not search: the app does. At most 200 ranges are kept; those far from the view go first.
- Search: `search(query)` marks every match of the query in the text of the cells of the rows shown (a cell whose content is an element is not searched), as a find marks it (`Highlight`'s marks, the text's weight kept), the current one stronger, with its edge. F3 and Shift+F3 in the grid go to the next match and the one before, round from the last to the first, across rows not built yet: each is brought into view, down and across. With `find_bar(true)` Ctrl+F (Cmd+F) opens `FindBar` over the grid: its query is the grid's search (`on_search_change` hears it), its toggles the case and whole words, its count `2 of 8`, its arrows the next and the one before. `on_matches_change` is told `SearchMatches { count, current }` each time that changes. The matches are found once for the rows, the lines shown and the query, over every row shown: a search of a million rows of many columns takes as long as reading them. With rows the app gives on demand the app searches.
- A range of cells: a drag from a cell selects the cells from it to the one under the pointer (the rows and the columns scroll where the drag goes past the view), Shift with the arrows, Home, End, PageUp and PageDown (with Ctrl, Home and End to the corners) moves the range's other corner from the focused cell, Shift with a press takes it to the cell pressed, and Ctrl+A selects every cell. The focus stays on the cell the range started from; a move of the focus, a press without Shift and Escape end the range. It is drawn as one area: every cell of it but the focused one in `Selection`, an edge of 2 in `Ring` round it, the focused cell on its row's fill; each of its cells is `a11y_selected` to what reads the tree. It is apart from the rows' selection. It names its cells by the rows' keys and the columns' ids, so the same cells stay in it when the app sorts the rows.
- A column's menu: a button of 24 at the end of each header, its mark in `MutedForeground`, shown while the pointer is on the header or the focus is in it and while the menu is open; Alt+Down on the header (or the menu key, or Shift+F10) opens it too. The menu is a `ui::Menu` under the button: Sort ascending, Sort descending and Clear sort for a sortable column, Pin to start, Pin to end and Unpin, Size to fit, Hide column (not the last one shown), and Columns, a menu of every column checked where it is shown, with Show all columns. A choice of the sort sets the grid's sort as a press of the header does; the others change the arrangement of the columns.
- A header dragged to another place moves its column there: a line of 2 in `Ring` at the start of the header it goes before, or the end of the last. Ctrl+Shift+PageUp and PageDown on a header move its column a place back or on, as a browser moves a tab. A column pinned at the start stays among those at the start, one at the end among those at the end. A resize is kept in the arrangement too, as the column's `width`.
- Group rows: a row for each value of the first column grouped by, in the order its first row is, with a chevron (right closed, down open), the value 13/500 in `Foreground` and how many rows it holds mono 12 in `MutedForeground`, on `Muted`, as high as a row; in it, a group for each value of the next column, indented 20 a level, then its rows. What it says stays at the start while the rows scroll sideways. A press of it, Enter or Space on it opens or closes it; the rows are a nested list, so Right opens a group and Left closes it, and Left on a row in it goes to the group. The rows of a closed group are not built, and the window of rows in view counts the groups' rows with the rows: ten thousand rows in three groups build a screen of them. To what reads the tree a group's row is a `row` with `a11y_level` and `a11y_expanded`, named by its column, its value and its count. A group's row is the grid's with `-group-` and its line (`files-group-0`).
- Row numbers: a column at the start, before the checkboxes, that stays while the columns scroll sideways, with each row's number among the rows shown, from 1, mono 12 in `MutedForeground` at its end (the selected row's in `Foreground`), as wide as the widest number. Where the rows are selected, a number is pressed to select its row alone, with Ctrl to add it or take it out, with Shift the rows from the last selected one, as a press of a table's row does; the column's header, a `#`, selects every row, or none where all are. To what reads the tree a number is a `rowheader`. A number's id is its row's with `-number`, the header's the grid's with `-numbers`.
- Storing what a person did: the arrangement of the columns, the sort, the filters (`TableFilters`, above) and the groups open are the app's values, each stored as text, as a dock's state is. `layout.to_text()` (and `Display`) writes the arrangement, `column("size" width:150) column("name" pin:start) column("status" hidden)`, and `ColumnLayout::parse(&text)` (and `FromStr`) reads it back to an equal value, or `None` for text that is not the form; `layout.valid(ids)` brings it to the columns the app has now (one it no longer has out, a new one at its place after the one before it), which the grid also does as it shows it. A sort is `"status" ascending "size" descending`, its columns in turn (`TableSort::list_to_text`, `TableSort::parse_list`; one column alone `TableSort::to_text`, `TableSort::parse`), a group `group("Passing" "Ada")` (`GroupKey::to_text`, `GroupKey::parse`), the groups open their list (`GroupKey::list_to_text`, `GroupKey::parse_list`); `TableTextError` is what `FromStr` returns for text that is not the form. With the `persistent_storage` feature each is a value of the app's storage, stored as that text: `ctx.set_persistent_value("columns", layout.get())`, `ctx.set_persistent_value("sort", sort.get())` (a `Vec<TableSort>`, none as no text), `ctx.set_persistent_value("open", open.get())` (a `Vec<GroupKey>`), and `ctx.persistent_value::<ColumnLayout>("columns")` reads one back.
- Summary rows: where a column says a summary, the grid has a row at its bottom that stays there while the rows scroll, over what scrolls under it, with what each column says of every row, and each group open ends in a row with what it says of the group's rows. A summary row is as high as a row, on `Muted` with a line of 1 over it; a cell says its kind 12 in `MutedForeground` ("Sum") and its value 13/500 in `Foreground`, at the column's side; a cell of a column that says none is empty. The bars run above the grid's summary. A summary row is not a stop: the keys pass it by. A mounted grid computes what its summaries say when what they are computed from changes (the rows the app gives, the columns' summaries, the grouping), every open group's at once, and keeps it: a scroll, the pointer and the focus compute nothing. Its id is the grid's with `-summary`, a group's `-summary-` and its line.
- Ctrl+C copies the range, or the focused cell, as tab-separated text, a line for each row: the value a cell's editor edits (a number as it is, a choice by its name, a box `true` or `false`), or its column's text. Ctrl+V pastes tab-separated text from the range's first cell, or the focused one: each cell that is edited and would change is told to `on_edit`, which may refuse it, a number read as one, a choice by its name, a box by `true`, `yes`, `1` or `x`; one value pasted into a range fills it. Delete clears what is edited of the range: a text to nothing, a box unticked, a number and a choice told `CellValue::Empty`. Ctrl+X copies and clears. All of it reaches the rows not built too: Ctrl+A and Ctrl+C on ten thousand rows copy ten thousand lines. Through the engine's clipboard ([copy events](../styling-events/#copy)), which a test installs its own of.
- A cell is edited with its column's editor: Enter or F2 on it, a double press of it, or a letter or a digit typed on it, which the edit starts from. Enter commits and goes to the cell under it, Tab commits and goes on to the next cell, Shift+Tab to the one before, Escape puts the cell back as it was and gives it the focus again. A choice commits as it is chosen; a box turns at a press or Space, with no editor. An edit the app refuses leaves the editor open with its value, marked `invalid`, and the focus in it. The focus moving to another cell commits what was typed, and closes the editor however the app answers.
- Tree rows: with `tree(|row| parent)` each row comes after the row whose key its parent is, the rows under one in the order the app gives them (its sort's), a row whose parent is not among the rows at the top. The first column's cell is indented 18 a level and starts with the chevron (`TablePart::TreeToggle`, 14, `MutedForeground`, `Foreground` under the pointer; right closed, down open), 6 before its content, or with the chevron's room where no row is under it, as the board's Tree table draws it. A press of the chevron opens or closes the row and does nothing else; Right on the first cell opens a closed row, Left closes an open one, and Left on a closed row, or one with nothing under it, goes to the row it is under. A filter keeps the rows a row it keeps is under. The rows of a closed row are not lines: they are not built, and ten thousand rows in a few open folders build a screen of them. To what reads the tree a row has `a11y_level` from 1 and, where rows are under it, `a11y_expanded`; the body is a nested list, as a grouped grid's. A grid grouped by columns is no tree.
- A row's detail: with `detail(|row| element)` an open row has a line under it with the element, on `Muted` with 12 down and 16 across and a row's line under it (`TablePart::Detail`), as wide as the grid's view and staying in it while the columns scroll sideways, as high as what it holds: its line is measured as it is laid out, as a row that wraps is, and the window of lines counts it. Where the rows are no tree, the first cell's chevron opens and closes it, and Right and Left on that cell do. What is in it is reached with Tab; the arrows go from row to row past it. Its id is the grid's with `-detail-` and the row's place (`files-detail-3`).
- The bar over the grid: with `toolbar(true)` the grid has the board's Table Toolbar over it, 8 above it, as wide as it, built from a `Toolbar` (its surface: 52 high, padded 12 across, 8 between, the large radius): a `SearchField` of 220 (`Sm`) whose query is the grid's search, the app's (`search`, `search_controlled`, told through `on_search_change`) or the grid's own where the app holds none; a `Tag` (`Info`, 26 high, 12) for each filter set, "Status: Passing", whose × takes it off; Filter (an outline button of 26, its plus 12), a menu of the columns that filter, each opening its filter's panel under its header; then at the end the density, a `ToggleGroup` of three cells of 30 (`icons::ROWS_2`, `ROWS_3`, `ROWS_4`) showing the grid's density until a person chooses one, which the grid then takes; Columns (an outline button of 30 with its chevron), a menu of every column checked where it is shown; and Export (an outline button of 30 with `icons::DOWNLOAD`), the menu of the column menus' export. What does not fit goes into the bar's menu, the search field last. Its ids are the grid's with `-bar`, `-bar-search`, `-bar-filter`, `-bar-filter-` and a column's id for a tag, `-bar-density`, `-bar-columns` and `-bar-export`.
- Export: a column's menu ends in Export, a menu of Shown rows as CSV, Shown rows as tab-separated, Selected as CSV and Selected as tab-separated (the last two only where rows are selected or cells are in a range). The text's first line is the titles of the columns shown, in their order; then a line for each row, every row the grid shows (filtered and sorted as it shows them, those not built too), or the rows selected in the order shown, or, with no row selected, the range's rows in its columns. A cell says its column's text as it is shown, or the value its editor edits where the column has no text (a box `true` or `false`), or nothing for a cell the app builds. CSV is RFC 4180's: a field with a comma, a quote, a line break or a space at an end is quoted, its quotes doubled, and the lines end with CR LF; tab-separated text has a space for a tab or a line break in a field, and LF. The app is told it (`on_export`, a `GridExport`: `format`, `scope`, `text`, `rows`) to save it as a file, `format.extension()` and `format.media_type()` saying of what kind; with no one told the text goes on the clipboard. It is written at once from every row: a million rows of two hundred columns are that much text. Over rows the app gives on demand, the rows that have not come are empty lines: the app has them, and writes them with `to_csv(&columns, &rows)`.
- The grid edits nothing of its own: the rows are the app's, which `on_edit` changes and gives again, and the grid shows them at the next render.
- To what reads the tree it is a `grid`, its rows `row`s with `a11y_rowindex` and `a11y_selected` where rows are selected, its cells `gridcell`s with `a11y_colindex` (the checkboxes' column counted), `a11y_rowindex`, `a11y_readonly` and `a11y_selected` where they are in the range, its headers `columnheader`s.
- In the design (`pencil/lurq-design-system.pen`, Data Grid) it is the table with the column of checkboxes: 12 between columns, the checkboxes 16, the name in the mono type and sorted with its arrow, a status as a badge, a size of 12 in `MutedForeground` at its end, the selected row on `Highlight`, a footer of 44 with the count and the pager. Where the board draws nothing: a cell focused from the keyboard has the ring of 2 in `Ring` inside it, as a row of a table; the range of cells is the text selection's `Selection` edged in `Ring`; a column's menu is the system's menu from a ghost button of 24 at the header's end, as the board's toolbars have them; an editor is the system's control of the small size (32), as wide as its cell; a column's resizer is the table's.
- Its parts and states are the table's (`TablePart`, `TableState`), and so is its renderer: a theme restyles a grid as it restyles a table, and a `TableView` of a grid says `view.grid`, each `TableCell` its `grid` (a `GridCell`: its row, whether it is editable, whether it is in the range, its editor while it is edited, whether Tab enters at it, its `a11y_colindex`).
- The board's Property Grid (two columns of a name and a control, in categories that open and close) is not a data grid: it would take its tracks and its editors' controls from it, its categories from a tree view's open rows, and edit every value in place at once rather than one cell at a time.

## Calendar

The days of a month in weeks, which a person chooses a date from, or a range of dates: a due date, a day to book, the week of a report.

```rust
use lurq::{
  core::{Date, DateRange, Signal, YearMonth},
  ui::{Calendar, DateMemory},
};

// due: Signal<Option<Date>>; in a component's render, mounted so that it keeps the month it shows.
Calendar::new(due.clone()).min(Date::today()).mount(ctx);

// The month shown in a signal of the app's: it is not the value.
Calendar::new(due.clone()).month(shown.clone()).mount(ctx);

// stay: Signal<Option<DateRange>>: a range with two presses, two months side by side, weekends not to be chosen.
Calendar::new(stay.clone())
  .months(2)
  .disabled_days(|date| date.weekday().days_from_monday() >= 5)
  .on_change(|range: DateRange| println!("{range}"))
  .mount(ctx);

// The app holds the date and answers a choice.
Calendar::controlled(self.due.get(), move |date: Date| due.set(Some(date)));

// What it keeps, kept by the app in a field of its component, made once: for a calendar built without ctx.
Calendar::new(due.clone()).memory(&self.memory);
```

| Builder | |
| --- | --- |
| `new(Signal<Option<Date>>)`, `new(Signal<Option<DateRange>>)` | What is chosen, which a choice sets: a date (a `Calendar`), or a range chosen with two presses (a `Calendar<DateRange>`) |
| `controlled(Option<V>, Fn(V))` | The app's value, and what it is told of a choice, which the calendar does not set: the app answers with the value it renders it with next |
| `month(Signal<YearMonth>)` | The month shown, the first where there are several: not the value. The heading's buttons and the keys set it |
| `memory(&DateMemory)` | What the calendar keeps between renders, the app's: the month it shows where the app gives none, a range's first press and where it would end |
| `mount(ctx)` | The calendar with a memory of its own |
| `months(n)` | How many months side by side, each with its heading |
| `min(Date)`, `max(Date)` | The least and the greatest date that can be chosen. The months beyond are not shown |
| `disabled_days(Fn(Date) -> bool)` | Which days cannot be chosen besides those out of the range |
| `locale(impl DateText)`, `first_day(Weekday)` | Its text (a `DateLocale`, the app's own `DateText`, or an `Arc` of it) in place of the app's (`app.set_date_locale`, English where it set none), and the first day of the week in place of the text's |
| `week_numbers(bool)` | The ISO 8601 week number before each week |
| `outside_days(bool)` | The days of the months before and after in the first and the last week; on, as the design draws them, and none with several months |
| `today(Date)` | The day marked as today (the state `Current`), in place of the system's |
| `other_end(Option<Date>)` | Of a calendar of one date: the other end of a range its date is one end of, the date of the range's other picker, marked (the state `OtherEnd`, on `Highlight` with its radius, `range_other_end` to tooling) while a person chooses this end |
| `disabled(bool)` | It takes no input |
| `footer(element)` | An element of the app's under the months, across the panel, in the part `Footer`: 8 under a line of 1 in `Border`. Today and Clear, a line of what the days mean. It is not in the focus group of the days |
| `label(&str)` | What the calendar is called to tooling: its panel is then a `group`. Each month is a `grid` called by its month whatever it says |
| `previous_label(&str)`, `next_label(&str)` | What the buttons of the heading are called: "Previous month" and "Next month" in English |
| `on_change(Fn(Date))`, `on_change(Fn(DateRange))` | Told of another date chosen, or another range ended, with it; bound to a signal, the value is set already. Not for what is chosen already, nor for the app setting the value. A second handler is called after the first |
| `on_choose(Fn(Date))`, `on_choose(Fn(DateRange))` | Told of every choice, of what is chosen already too: what closes a picker of the app's that holds the calendar. A second handler is called after the first |
| `heading(CalendarHeading)` | How the headings are laid out: `Centered`, the month between the buttons (the default, the design's), or `MonthThenNav`, the month at the start and both buttons after it at the end, 4 apart (of several months, at the end of the last month's heading; state `CalendarState::NavAtEnd`) |
| `keys(CalendarKeys)` | Where the keys are read: `Days`, the default, the days taking the focus; or `Panel`, the panel reading them wherever the app keeps the focus (see below) |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the day Tab would enter at. Each day has the id with `-day-` and the date after it: `due-day-2026-11-12`; a weekday's head `-weekday-` and its English short name (`due-weekday-mon`); the buttons `-nav-previous` and `-nav-next`; its other parts `-` and the part's name (see [ids of parts](#ids-of-parts)) |

- The calendar does not keep what is chosen: the app's signal does. It keeps the month it shows, starting at the month of what is chosen, or of today, and follows the value to its month when the app sets it to a date that is not shown; mounted, or with a `DateMemory`, it keeps it between renders. Converted into an element with neither it keeps nothing: its buttons show no other month unless the app gives it the month, and a range shows only once it is chosen.
- Its text (the title of a month, the heads of the weekdays, what a day and the buttons are called) is one [`DateText`](../dates/#how-an-app-writes-its-dates): its own `.locale(..)`, else the app's (`app.set_locale(..)`, whose `Locale::dates` it is, or `app.set_date_locale(..)` after it), else English. The app's is read where the calendar is built in a render, mounted or not, and a change of it builds the calendar again. An app that names its months or its weekdays in words of its own changes them in the locale's `dates` (`months`, `weekdays_short`, `weekdays`, `labels`, ...) and sets it:

  ```rust
  let mut locale = Locale::for_tag("uk");
  locale.dates.weekdays_short = ["пн", "вт", "ср", "чт", "пт", "сб", "нд"].map(Arc::from);
  locale.dates.months[10] = Arc::from("Листопад");
  app.set_locale(locale);
  ```
- A range: the first press starts it, the pointer and the keys show where it would end, and the second ends it, before the first or after it. Its ends are chosen days, and the days between are one band.
- The days are one stop of Tab, at the chosen day, today or the first of the month; the buttons of the heading are stops of their own before them. Left and Right go a day, Up and Down a week, Home and End to the ends of the week, PageUp and PageDown a month and with Shift a year: into the month before or after, which is then shown, and kept between the least and the greatest date. A day that cannot be chosen takes the focus too and says so: the keys pass no day by. Enter and Space choose. The keys are the days' own, through the engine's `KeyboardEvent::focus_key`.
- Read at the panel (`keys(CalendarKeys::Panel)`), for a picker in a [popover that keeps the focus](#popover) in the app's field or panel, the days take no focus and are no group of it, and a press of one leaves the focus where it was. One day is the one the keys are on (state `CalendarState::Cursor`, ringed as a focused day is, `cursor` on it and the date on the panel to tooling): the chosen day, else today, else the first of the month to begin with, and again after a choice and where its month is no longer shown. While the calendar is shown the panel reads the keys wherever the focus is: Left and Right move that day a day, Up and Down a week, PageUp and PageDown a month and with Shift a year, showing its month; Enter chooses it and tells `on_choose`, so that the picker closes. A key a handler before took (`prevent_default`), a key with Ctrl, Alt or Cmd, an Enter where the focus is on what takes Enter itself (a button of the footer), and Home and End, which are the field's, are left alone. The day is kept where the month is: mounted, or in a `DateMemory`.
- To what reads the tree each month is a `grid` called by its month (`November 2026`), of `row`s of `gridcell`s called by their day (`Thursday, Nov 12, 2026`) that say `a11y_selected`, `a11y_current` (`date`, for today) and `a11y_disabled`; the names of the days are `columnheader`s.
- Parts: `CalendarPart::Root` (the panel), `Months` (the row of headings and the row of days, its gap the room between months), `Header`, `Title`, `Nav` and `NavIcon` (rules over the outline buttons and their glyphs), `Grid`, `Weekdays`, `Weekday`, `Week`, `WeekNumber`, `Day`, `Footer`. States of the calendar: `CalendarState::Disabled`, `Bare` (in a picker's panel: no fill or edge of its own); `NavAtEnd` (the buttons at the end of the heading); of a day: `Selected`, `Current` (today), `Outside`, `Unavailable`, `RangeStart`, `RangeEnd`, `InRange`, `OtherEnd`, `Cursor` (the day the keys are on, read at the panel).
- In the design the panel is 248 wide, on `Popover` with an edge of 1 in `Border`, `radius-lg` and 12 of padding; the heading is 32 high, the month in the body's type at 500 between two outline buttons of 28 (`radius-md`, edge in `Input`, chevrons of 16 in `MutedForeground`), 8 over the days. A day is 32 square with `radius-md`, its number at 13; the names of the days are at 12 in `MutedForeground`; the rows are 8 apart. Today and a day under the pointer are on `Highlight` (the board's `accent`); the chosen day is on `Primary`, its number in `PrimaryForeground` at 500. A day of the month beside, and one that cannot be chosen, is in `MutedForeground` at half opacity. The days between a range's ends are on `Highlight` with no radius.
- A theme's renderer is given `CalendarView`: `view.months`, `view.chosen`, `view.draft`, `view.hover`, `view.shown_range()`, `view.day_states(date, outside)`, `view.day_recipe(states)`, `view.entry()`, and the pieces `nav(by)`, `header(index)`, `headers()`, `weekdays()`, `day(date, month)`, `week(days, month)`, `grid(index)`, `grids()`, `panel()` and `render()`.

## Date Picker

A field that holds a date, typed in it or chosen from a calendar that opens under it: a due date, a date of birth, the day of a booking.

```rust
use lurq::{
  core::{Date, Signal},
  ui::{DateMemory, DatePicker, Field},
};

// due: Signal<Option<Date>>
DatePicker::new(due.clone()).placeholder("Pick a date").clearable(true).mount(ctx);

// In a field, which names it and judges it; the field mounted mounts the picker.
Field::new("Due date", DatePicker::new(due.clone()).min(Date::today()))
  .error("The due date has passed.")
  .mount(ctx);

// Written and read the app's way.
DatePicker::new(due.clone()).format(|date| date.to_string()).parse(|text| text.parse().ok());

// The app holds the date and answers what is typed or chosen.
DatePicker::controlled(self.due.get(), move |date| due.set(date));
```

| Builder | |
| --- | --- |
| `new(Signal<Option<Date>>)` | The date the field holds |
| `controlled(Option<Date>, Fn(Option<Date>))` | The app's date, and what it is told of an edit or a choice, which the picker does not set: the field shows what the app holds until it answers |
| `memory(&DateMemory)`, `mount(ctx)` | What it keeps between renders (whether the calendar is open, the field it opens under, the month shown): the app's, or its own where it is mounted |
| `open(Signal<bool>)`, `month(Signal<YearMonth>)` | Whether the calendar is open and the month it shows, where the app holds them |
| `placement(Placement)`, `offset(..)`, `collision(CollisionStrategy)` | Where the calendar opens beside the field (`BottomStart`), how far from it, and where it goes with no room, as a [popover](#popover)'s |
| `inline(bool)` | The panel drawn in place under the field, open or not, taking no focus: for a gallery, a test, a page that holds a calendar |
| `placeholder(&str)` | What the field says while it is empty |
| `format(Fn(Date) -> String)`, `parse(Fn(&str) -> Option<Date>)` | How the field writes the date and reads what is typed, in place of the locale's |
| `locale(impl DateText)` | How dates are written and read, and the calendar's names, first day and the controls' names, in place of the app's (`app.set_date_locale`, English where it set none) |
| `min`, `max`, `disabled_days`, `first_day`, `week_numbers`, `today` | As the calendar's: what cannot be chosen cannot be typed either |
| `other_end(Option<Date>)` | The other end of a range the date is one end of, which the calendar marks: a start's picker is given the end's date, and the end's the start's |
| `clearable(bool)` | A button at the end of the field that empties it while it holds a date |
| `size(DatePickerSize)`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `full_width`, `name` | As an [input](#input)'s; `DatePickerSize` is `TextBoxSize`. A read-only picker opens no calendar |
| `trigger_label(&str)`, `clear_label(&str)`, `panel_label(&str)` | What the button, the cross and the panel are called to tooling: "Choose date", "Clear", "Choose date" in English |
| `previous_label(&str)`, `next_label(&str)` | What the calendar's buttons are called |
| `on_change(Fn(Option<Date>))` | Told of another date, by typing, a choice or the button that empties it; a choice closes the calendar first. Not for the date it holds, nor for the app setting it |
| `on_input(Fn(&TextInputEvent))` | Told of an edit of the text before it is made, which it may refuse: what is refused is not read |
| `on_submit(Fn(Option<Date>))` | Told of Enter, once the text is put right, with the date it holds |
| `on_open_change(Fn(bool))`, `on_dismiss(Fn())` | Told of the calendar opening and closing, and of Escape or a press outside closing it |
| `state`, `style`, `rules`, `force`, `id`, `key` | As an input has them. `id` is the field's; the button has it with `-trigger`, the cross `-clear`, the panel `-panel`, the calendar `-calendar` |

- Its text, the field's form of a date, how what is typed is read and the names of its button, cross and panel, is its calendar's `DateText`: its own `.locale(..)`, else the app's `app.set_date_locale(..)`, else English. `trigger_label`, `clear_label` and `panel_label` are above it.
- What is typed is read as it is typed, in the locale's short form (`Nov 12, 2026`), in its numbers (`11/12/2026`, `12.11.2026`) or as ISO 8601, a month by its name or the start of it in the locale's words or in English: a date that can be chosen is the value at once, and the field keeps what is half typed while it has the focus. The year has four digits, so `Nov 12, 20` is no date yet. When the focus leaves the field, or on Enter, the text is written again as the picker writes the value; an emptied field is no date, and anything else takes back the date it had.
- The calendar opens in a [popover](#popover) under the field, from the button at its start or from Down in the field, with the focus on the chosen day, today or the first of the month. A choice closes it and the focus goes back to the field, as Escape and a press outside do.
- To what reads the tree the field is a `combobox` that says `a11y_haspopup` `dialog` and `a11y_expanded`, as the date picker of the ARIA patterns is; the panel is a `dialog` called "Choose date". The button that opens it and the cross that empties it are pressed by the pointer and take no focus.
- In a [field](#field) the picker is called by the field's label, judged as the field is, and disabled where it is. A field the app mounts (`Field::mount(ctx)`) mounts the picker; a field converted into an element keeps what the picker keeps only where the app gives it a `DateMemory`.
- Parts: `DatePickerPart::Control` (rules over the input's box), `Trigger` and `TriggerIcon` (the button at the start and its glyph), `Clear` and `ClearIcon`, `Panel` (rules over the popover's root), and a range picker's `Presets`, `Preset` and `Separator` (rules over the line beside them). The field is an `Input` in the theme's dress, the panel a `Popover`, the days a `Calendar` in the state `Bare`. States: `DatePickerState::Open`, `Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`, and the input's size.
- In the design the field is the input, 260 by 36, with a calendar's glyph of 16 in `MutedForeground` 12 from its start and the date 8 after it in the body's type. The glyph is in a button of 24 with `radius-sm`, on `Highlight` under the pointer. The calendar opens 4 under the field, in the popover's panel with no padding of its own, as wide as the calendar.
- A theme's renderer is given `DatePickerView` (`PickerView<Date>`): `view.text`, `view.value`, `view.open`, and the pieces `control()`, `trigger()`, `clear()`, `field()`, `calendar()`, `panel()` and `render()`.

## Date Range Picker

A field that holds a range of dates, typed or chosen from a calendar under it, with ranges offered beside it: the dates of a stay, the period of a report.

```rust
use lurq::{
  core::{Date, DateRange, Signal},
  ui::DateRangePicker,
};

let today = Date::today();
// period: Signal<Option<DateRange>>
DateRangePicker::new(period.clone())
  .placeholder("Pick dates")
  .months(2)
  .preset("Last 7 days", DateRange::new(today.add_days(-6), today))
  .preset("Last 30 days", DateRange::new(today.add_days(-29), today))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<Option<DateRange>>)`, `controlled(Option<DateRange>, Fn(Option<DateRange>))` | The range the field holds: bound to a signal, or the app's |
| `months(n)` | How many months the calendar shows side by side: one as the design draws it, or two |
| `preset(label, DateRange)`, `presets([(label, DateRange)])` | Ranges offered beside the calendar, after those offered already; a press chooses one and closes the calendar. The one chosen is on the secondary fill |
| everything else | As a [date picker](#date-picker)'s, over a range: `format(Fn(DateRange) -> String)`, `parse(Fn(&str) -> Option<DateRange>)`, `on_change(Fn(Option<DateRange>))`, `on_submit(Fn(Option<DateRange>))`; the button and the panel are called "Choose dates" |

- The range is in one field, as the design draws it: `Nov 9 – Nov 15, 2026`, the year once where both ends are in it. What is typed is read as two dates in any form the locale reads, with a dash or `to` between them; the year of the first may be left out where it is the second's.
- In the calendar the first press starts the range, the pointer and the keys show where it would end, and the second press ends it and closes the calendar. Escape or a press outside drops a range half chosen. Neither end may be a day that cannot be chosen.
- The field's button has the glyph of a calendar's range; the panel is a `dialog` called "Choose dates".
- A theme's renderer is given `DateRangePickerView` (`PickerView<DateRange>`), with `presets()` besides the date picker's pieces.

## Event Calendar

A calendar that shows events in its days: the days of a week side by side, the weeks of a month, or one day, each day with what is on it. The meetings of a team, the releases of a project, what is due.

```rust
use lurq::{
  core::{Date, Signal},
  ui::{CalendarEvent, CalendarEventVariant, EventCalendar, EventCalendarPeriod},
};

let chosen = Signal::new(None::<Date>);
let events = vec![
  CalendarEvent::new("standup", monday, "Standup"),
  CalendarEvent::new("release", tuesday, "Release 0.60").variant(CalendarEventVariant::Success),
  CalendarEvent::new("freeze", friday, "Deploy freeze").variant(CalendarEventVariant::Destructive),
];

// The design's week: five days, no header. Mounted, it keeps the week it shows.
EventCalendar::new(chosen.clone(), events.clone())
  .weekends(false)
  .header(false)
  .on_select(|event: &&str| open(event))
  .mount(ctx);

// A month, of which a day shows three events at most and "+2 more".
EventCalendar::new(chosen.clone(), events)
  .period(EventCalendarPeriod::Month)
  .on_more(|date| show_day(date))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `EventCalendar::new(Signal<Option<Date>>, events)` | Bound to the chosen day, which a press of a day sets, with its events |
| `EventCalendar::controlled(value, events, on_change)` | It shows `value` chosen and tells of a press, which it does not set |
| `event(..)`, `events(..)` | Adds to its events |
| `period(EventCalendarPeriod)` | What it shows at once: `Week` (the default, the design's), `Month`, or one `Day` |
| `shown(Signal<Date>)` | The date whose week, month or day is shown, the app's: the buttons and the keys that leave the period set it. Without it a mounted calendar keeps it, starting at the chosen day, or today |
| `weekends(bool)` | Whether Saturdays and Sundays are shown: they are unless told. Without them a week is the five days the design draws, and the keys pass a weekend by |
| `header(bool)` | The row at its top: the buttons that show the period before and after, what the period is, and the button that shows today. It has it unless told |
| `max_shown(usize)` | How many events of a day are shown at most: 4 in a week and 3 in a month unless told, all in a day. With more, one fewer are shown and "+N more" in the place of the last |
| `on_change(Fn(Date))` | A person chose another day: a press or Enter on it, or a press of its "+N more". Not for the day chosen already, nor when the app sets the value |
| `on_select(Fn(&T))` | An event was pressed, with what the event is to the app (the first argument of `CalendarEvent::new`), after the event's own handlers. A press of an event does not choose its day |
| `on_more(Fn(Date))` | The "+N more" of a day was pressed, after the day is chosen: an app shows the day alone, or a list of what is on it |
| `first_day(Weekday)`, `locale(..)`, `today(Date)` | The day weeks start on, the words of its dates and the day it marks as today, in place of the app's locale and the system's day |
| `previous_label(&str)`, `next_label(&str)`, `today_label(&str)`, `more_text(Fn(usize) -> String)` | The names of its buttons, in place of the locale's, and what stands for the events not shown, in place of "+N more" |
| `disabled(bool)` | It takes no input |
| `label(&str)` | What its box is called to tooling, which is then a `group` |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the day Tab would enter at. Each day has the calendar's id with `-day-` and its date after it (`team-day-2026-11-12`), the buttons with `-nav-previous`, `-nav-next` and `-today`, a day's "+N more" with `-more-` and the date; its other parts `-` and the part's name, by the date where there is one of each day (see [ids of parts](#ids-of-parts)) |
| `mount(ctx)` | The calendar where it is declared: it keeps the period it shows, and follows the chosen day to its period when the app sets it to a day that is not shown. Converted (`.into()`) it keeps nothing, and its buttons show no other period unless the app gives it `shown` |

| Event | |
| --- | --- |
| `CalendarEvent::new(value, Date, &str)` | What it is to the app (its id, the record it is of), the day it is on, and its title |
| `CalendarEvent::on(Date, &str)` | An event that is no value to the app: its own `on_select` hears its press |
| `variant(impl Into<Name>)` | Its kind: `CalendarEventVariant::Neutral`, `Info` (the default), `Success`, `Warning`, `Destructive` |
| `on_select(Fn())` | This event was pressed, before the calendar's `on_select` |
| `disabled(bool)`, `id`, `key` | It takes no input; the id and the key of its element |

- Its days are a [calendar](#calendar)'s: the weeks of a month are the same grid (`YearMonth::weeks`), filled with the days of the months beside it, and the keys are the same, from one function. The days are one stop of Tab, at the chosen day where it is shown, else today, else the first day shown. Left and Right go a day, Up and Down a week, Home and End to the ends of the week, PageUp and PageDown a month and, with Shift, a year, into a period that is then shown. Enter and Space choose the focused day.
- Tab goes on from the focused day into its events, each a stop, and then out of the days: an event is pressed with Enter or Space, and its press is its own. The arrows on an event move between the days.
- A press anywhere on a day chooses it. A day of the month beside, in a month, is pressed to choose it and show its month, and is no stop of the keys.
- An event's title is on one line in a week and a month, cut with an ellipsis where its day is narrower; in the day view, where the room is the day's, it wraps onto as many lines as it needs.
- **In a narrow room.** Mounted, the calendar measures the room it is laid out in before the frame is painted, and takes the form that room holds from its first frame (`EventCalendarForm`, `view.form`); the room is its own, so a narrow panel of a wide window is a narrow room too:
  - A week whose days would be narrower than `EventCalendar::WEEK_DAY_LEAST` (110) keeps them at that width, and they scroll sideways inside the calendar, under its header, which stands still (`Scrolling`).
  - A week in a room under `EventCalendar::DAY_VIEW_BELOW` (330, three such days) shows one day of it (`OneDay`): the chosen day where it is in the week, else the day the week was shown by. Its title and its buttons are the day view's, and go a day; the keys are the same, and a press or Enter chooses as ever. With room again it shows the week of that day.
  - A month whose days would be narrower than `EventCalendar::MONTH_DAY_LEAST` (80) shows each day's number and under it a dot of 6 for each of its events, in the ink of its kind (`Dots`); a press of a day chooses it and tells `on_more`, where an app shows what is on it.
  - `EventCalendarForm::for_room(period, days, width)` is the form of a width. Converted into an element on its own the calendar has no room of its own to read, and keeps the full form.
- An event is on one day. The events of a day are in the order the app gave them.
- The names of the days and the months, the title of the header and the names of the buttons are the app's date locale's (`App::set_date_locale`, see [dates](../dates/)): a day of a week says `weekday_medium` and its number ("Thu 12"), the header a month's `month_title`, a week's `format_range` of its first and last day shown, or a day's `day_label`; the buttons say `previous_week_label`, `next_week_label`, `previous_day_label`, `next_day_label`, `today_label` and a month's `previous_month_label` and `next_month_label`.
- To tooling the days are a `grid` called by the period shown, of `row`s of `gridcell`s called by their dates, which say whether they are chosen (`a11y_selected`) and today (`a11y_current`); a month's names of the days are `columnheader`s; an event and a "+N more" are `button`s.
- Parts: `EventCalendarPart::Root`, `Header`, `Title`, `Nav` and `NavIcon` (rules over the buttons' root and icon), `Grid`, `Weekdays`, `Weekday`, `Week`, `Day` (what is pressed and focused), `Label` (the row at the top of a day), `Events`, `Event` (a pill, in its variant's fill and ink), `More`, `Dots` and `Dot` (a month's dots, in its variant's ink). States: `EventCalendarState::Month`, `Week`, `Day` (the period), `Header`, `Disabled`, `Compact` (a month of dots), `Scrolling` (a week that scrolls); a day's `Selected`, `Current`, `Outside`, `Last` (the last of its row, and a week the last of the calendar: no line after it); an event's `Unavailable`.
- In the design (`eh5zV`) it is a box on `Background` in an edge of 1 in `Border` with the large radius, 150 high, its days side by side with a line of 1 after each but the last. A day's label is a row of 32 with a line under it, padded 10, its text 12 at medium weight in `MutedForeground`; today's is on `Highlight` (the board's accent) in `Foreground` at 600. The events are 6 from the day's sides and 4 apart, each a pill with the small radius padded 4 and 6, its title 11 at medium weight in the color of its kind on that kind's muted fill. The board draws a week of five days, and nothing else: the header (44 high, a calendar's buttons, the title at medium weight, an outline "Today"), the month (days at least 115 high under a row of the days' names, a day of the month beside at half its opacity), the chosen day (its label on `Primary`), "+N more" (an event's box in `MutedForeground`), the focus (a line of 2 in the ring's color inside a day or an event) and the event of no kind (`Muted`, `Foreground`) are lurq's.
- A renderer that draws another calendar has `view.root()`, `view.header()`, `view.nav(by)`, `view.today_button()`, `view.title_text()`, `view.grid()`, `view.weekdays()`, `view.weeks()`, `view.week(days, last)`, `view.day(date, last)`, `view.day_text(date)`, `view.day_states(date, last)`, `view.events(date)`, `view.event(event)`, `view.more(date, count)`, `view.entry()`, `view.bounds()`, `view.within(date)`, `view.recipe_in(variant, states)` and `view.render()`.

## Color Picker

A color chosen in an area of saturation and brightness, on a rail of hue and one of alpha, from swatches, or typed as hex, RGB or HSL: the color of a label, of a brush, of a theme's accent.

```rust
use lurq::{node::color::Color, ui::{ColorFormat, ColorPicker}};

// The board's: the area, the rail of hue, swatches and the hex.
ColorPicker::new(accent.clone())
  .swatches(["#3B82F6", "#2DD4BF", "#F59E0B", "#A78BFA", "#F472B6", "#FAFAFA"].map(Color::from_hex))
  .on_change_end(|color| save(color));

// With a rail of alpha, and the fields of RGB a button switches to hex and HSL.
ColorPicker::new(overlay.clone())
  .alpha(true)
  .format(ColorFormat::Rgb)
  .formats([ColorFormat::Hex, ColorFormat::Rgb, ColorFormat::Hsl]);

// The color held by the app, told each change.
ColorPicker::controlled(series, move |color| set_series(color));
```

| Builder | |
| --- | --- |
| `ColorPicker::new(Signal<Color>)` | Bound to the app's color, the engine's `Color`: what it shows, and what a person's change sets |
| `ColorPicker::controlled(Color, on_change)` | The app holds the color: each change is told, and the picker shows what the app answers with. Two built at the one place in the app's code, in a loop, are told apart by their `id` or `key` |
| `alpha(bool)` | A rail of alpha over a checkerboard, and the fields say the alpha. Without it the alpha of the color is kept as it is |
| `format(ColorFormat)` | The fields: `Hex` (the board's, one field), `Rgb` (red, green and blue from 0 to 255) or `Hsl` (hue in degrees, saturation and lightness in percents) |
| `formats(..)` | The fields a button before them switches between, in order. One or none: no button |
| `eyedropper(bool)`, `eyedropper_label(&str)` | The button at the end of the fields that picks a color from the screen ([`EyeDropper`](../resources-media/#a-color-from-the-screen)): shown where the platform can pick, on by default; `false` hides it anyway. What it is called to what reads the tree: "Pick a color from the screen" |
| `swatch(color)`, `swatches(..)` | Circles of 20 in a row under the rails; a press of one makes the color it, and the one of the color is selected |
| `on_change(Fn(Color))` | Called with each change a person makes: as a thumb moves, as a key steps, as a swatch is pressed, as a field is committed. Not for what is so already, nor when the app sets the color. Each call adds |
| `on_change_end(Fn(Color))` | Called once a change is over (a drag let go, a key's step, a swatch's press, a field committed), where the color is another than it was as it began. Each call adds |
| `bare(bool)` | Without its panel (fill, edge, corners, shadow): what a popover that holds it draws |
| `label(&str)` | What the picker is called, to what reads the tree: a `group` |
| `saturation_label`, `brightness_label`, `hue_label`, `alpha_label`, `format_label` | The words the sliders and the switch say to tooling: "Saturation", "Brightness", "Hue", "Alpha", "Format" |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's; the area is `{id}-area`, the rails `{id}-hue` and `{id}-alpha`, the hex `{id}-hex`, the swatches `{id}-swatch-0` and on, the switch `{id}-format`, a channel `{id}-r` and the like |

- The area is a slider of two axes: a press or a drag sets saturation across and brightness up, and a drag past its edge keeps the color at the edge. With the focus in it the arrows step by 1%, ten with Shift; PageUp and PageDown step the brightness by 10%; Home and End take the saturation to none and to the whole. The rails are sliders: of hue from 0° to 360° (the true hues, red through yellow, green, cyan, blue and magenta back to red, so that the thumb is at the hue), and of alpha from 0% to 100%; a press or a drag, the arrows by a degree or 1%, ten with Shift, PageUp and PageDown a tenth, Home and End to the ends. A press of the area or a rail gives it the focus, and each is a stop of Tab.
- The picker keeps, while it is in the tree, the hue and saturation its color last had, which a grey and black do not say: moving through the greys does not lose the hue. It keeps the fields a person switched to, and where a drag began.
- The field of the hex reads what is typed on Enter and as the focus leaves, in any form a color is written in (`#2563EB`, `2563eb`, `#25e`, `rgb(37, 99, 235)`, `hsl(221 83% 53%)`), and writes the color in capitals, with its alpha where it is not opaque. What is no color takes back the hex. A field of a channel is a bare [number input](#number-input).
- To what reads the tree the area is a `slider` called "Saturation and brightness" whose `a11y_valuetext` is "Saturation 84%, brightness 92%"; the rails are `slider`s called "Hue" and "Alpha", with `a11y_valuenow`, `a11y_valuemin`, `a11y_valuemax` and `a11y_valuetext` ("Hue 221°", "Alpha 50%").
- Parts: `ColorPickerPart::Root` (the panel), `Area`, `AreaThumb`, `Hue`, `Alpha`, `Thumb` (a rail's), `Checker` and `CheckerCell` (the checkerboard under the rail of alpha), `Swatches`, `Swatch` (laid over a [color swatch](#color-swatch)'s root), `Fields`, `Input` (the hex), `Format` (the switch), `FieldLabel` (the letter over a channel), `Eyedropper` and `EyedropperIcon` (the button that picks from the screen, a ghost button of 36 with an icon of 16 in `MutedForeground`). States: `ColorPickerState::Alpha`, `Bare`, `Disabled`.
- The eyedropper starts a pick from the screen; the picker stays as it is while the person picks, and the color picked is a change of theirs: `on_change` and `on_change_end` are told it once, and nothing where they gave up (Escape) or picked the color it holds. The board draws no eyedropper in the picker: it stands at the end of the fields, where the field's stands at the end of its box.
- In the design the panel is 232 wide in `Popover`, with an edge of 1 in `Border`, the large radius and the medium shadow, 12 of padding and 12 between what it holds. The area is 208 by 120 with the medium radius: the hue, white over it from its start and black over that from its bottom; its circle is 12, a ring of 2 in white. The rail of hue is 10 high and round at its ends. The swatches are circles of 20, 8 apart; the hex is an input 36 high in the mono type. The board draws no thumb on the rail, no rail of alpha, no fields but the hex and no switch: a rail's thumb is a circle of 14 with a ring of 2 in white and the small shadow; the rail of alpha is the rail of hue over the checkerboard of a swatch (squares of 5 in `Background` and `Muted`); a channel's letter is 11 in `MutedForeground` over its number; the switch is a ghost button 36 high in the mono type at 12. The focus of the area and of a rail is the theme's ring around it.
- The area and the rails are drawn with the engine's linear gradients: a node has one, so the area is three boxes laid one over another (the hue, white to clear across, clear to black down) and the rail of alpha the checkerboard under the color from clear to whole.
- A renderer that draws another picker has `view.area()`, `view.hue()`, `view.alpha_rail()`, `view.swatches()`, `view.format_button()`, `view.hex_field()` and `view.fields()`, each with its behavior, and `view.hsv` (the color with the hue a grey keeps).

## Color Field

A color typed in a box, or chosen in a [color picker](#color-picker) that opens under it: the color of a label, of a brand, of a chart's series.

```rust
use lurq::{node::color::Color, ui::{ColorField, Field}};

Field::new("Brand color", ColorField::new(brand.clone()));

// A color or none: emptied, the field holds `None`.
ColorField::optional(tint.clone()).placeholder("No tint").alpha(true);

// The color held by the app.
ColorField::controlled(series, move |color: Color| set_series(color));
```

| Builder | |
| --- | --- |
| `ColorField::new(Signal<Color>)` | Bound to the app's color: what the box and its swatch show, and what is typed or chosen sets |
| `ColorField::optional(Signal<Option<Color>>)` | Bound to a color or none: `None` shows the placeholder, and emptying the field makes it none. Its `on_change` is told an `Option<Color>` |
| `ColorField::controlled(value, on_change)` | The app holds the value (a `Color` or an `Option<Color>`): each change is told, and the field shows what the app answers with. Two built at the one place in the app's code, in a loop, are told apart by their `id` or `key` |
| `on_change(Fn(V))` | Called with the value each time a person changes it: typed and committed, chosen in the picker, emptied. Not for what is so already, nor when the app sets it. Each call adds. The field is `ColorField<Color>` or `ColorField<Option<Color>>`, so a handler of another type does not compile |
| `alpha(bool)`, `format(..)`, `formats(..)`, `swatch(..)`, `swatches(..)` | What the picker that opens has: see [color picker](#color-picker) |
| `placement(Placement)` | Where the picker opens beside the field: under it at its start (the default), flipped where there is no room |
| `size(ColorFieldSize)` | An input's: `Sm` 32, `Md` 36, `Lg` 40 high, with a swatch of 20, 24 and 32 |
| `placeholder(&str)`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `full_width`, `label`, `name(..)` | As an [input](#input) has them. Read only, nothing typed is taken and the picker does not open. `full_width` is in place of 180 |
| `eyedropper(bool)` | The eyedropper at the box's end, which picks a color from the screen ([`EyeDropper`](../resources-media/#a-color-from-the-screen)): shown where the platform can pick, on by default; `false` hides it anyway |
| `trigger_label(&str)`, `swatch_label(&str)`, `panel_label(&str)` | What the eyedropper, the swatch that opens the picker and the popover are called to what reads the tree: "Pick a color from the screen", "Choose color" and "Color" |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the box's; the swatch is `{id}-swatch`, the eyedropper `{id}-trigger`, the popover `{id}-panel`, the picker `{id}-picker` |

- What is typed is read on Enter and as the focus leaves, in any form a color is written in, as the picker's hex is: a color is the value and is written again as its hex in capitals, an emptied optional field is none, and anything else takes back the hex. The box keeps what is half typed while it has the focus.
- The picker opens in a [popover](#popover) under the field from the swatch at its start or Down in the box, with the focus in its area; a change in it sets the field's value at once. Escape and a press outside close it, and the focus goes back to the box; Tab from its last field goes on past the field. The field keeps, while it is in the tree, whether its picker is open.
- The eyedropper picks a color from the screen, where the platform can: the color picked is the field's value, as a person's change (`on_change`), and the picker, open or not, stays as it is while the person picks. Where the platform cannot pick, there is no eyedropper; the picker in the field's popover has none of its own.
- To what reads the tree the box is a `combobox` called by its label, which says it opens a `dialog` and whether it is open; the swatch and the eyedropper take no focus.
- Parts: `ColorFieldPart::Root` (its width), `Control` (the box typed in), `Placeholder`, `Swatch` (laid over a [color swatch](#color-swatch)'s root), `Trigger` (the eyedropper's button), `TriggerIcon`, `Panel` (over the popover's). States: `ColorFieldState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`, `Open`.
- In the design the field is 180 by 36, an input's box (the `Background` fill in an edge of 1 in `Input`, 6 round), the swatch of its color (24) 6 from its start, its hex 8 after it in the mono type at 13 in `Foreground`, and an eyedropper of 16 in `MutedForeground` 6 from its end. The board draws it at rest only: under the pointer, focused, judged, read only and disabled it is an input; the eyedropper is in a button of 24 with the small radius, on `Highlight` under the pointer; while the picker is open the edge is in `Ring`. The picker opens 4 under it in a popover whose panel is the picker's, 232 wide.
- A renderer that draws another field has `view.control()`, `view.swatch()`, `view.trigger()`, `view.field()` and `view.panel()`.

## Color Swatch

One color, shown as a small square or circle: a color an app has, a choice among colors, what a color field holds.

```rust
use lurq::{node::color::Color, ui::{ColorSwatch, ColorSwatchShape, ColorSwatchSize}};

ColorSwatch::new(Color::from_hex("#2563EB")).label("Brand blue");
// Not opaque: over a checkerboard.
ColorSwatch::new(Color::from_hex("#2563EB80")).shape(ColorSwatchShape::Round);
// Pressed: a button.
ColorSwatch::new(amber).size(ColorSwatchSize::Lg).on_click(move |_| choose(amber));
```

| Builder | |
| --- | --- |
| `ColorSwatch::new(impl Into<Color>)` | A swatch of the color |
| `size(ColorSwatchSize)` | `Sm` 20, `Md` 24 (the board's), `Lg` 32 square |
| `shape(ColorSwatchShape)` | `Square` (the small radius, the board's) or `Round` |
| `selected(bool)` | A ring of 2 in `Foreground` around it: the one chosen |
| `on_click(handler)` | A click, and Enter and Space while it has the focus: the swatch is then a button, a stop of Tab. Each call adds |
| `label(&str)` | What it is called, to what reads the tree: its color's hex in capitals where it has none |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them |

- A color that is not opaque stands over a checkerboard of squares of 4, `Background` and `Muted`.
- To what reads the tree a swatch is an `img` called by its label or its hex; one with `on_click` is a `button`, with `a11y_pressed` where it is selected.
- Parts: `ColorSwatchPart::Root`, `Checker` (the light squares), `CheckerCell` (the dark squares and their side). States: `ColorSwatchState::Selected`, `Round`, `Pressable`, `Disabled`. Sizes: `ColorSwatchSize`.
- In the design a swatch is 24 square with the small radius and an edge of 1 in `Border` inside it. The board draws none focused or disabled: the focus is the theme's ring, and a disabled one is dimmed as every control is.

## Drop Zone

Where files are dropped from the system, or chosen in its file dialog: images for a gallery, a document to import. It says what a drag over it will do, takes what it accepts, and tells the app what it refused.

```rust
use lurq::ui::DropZone;

DropZone::new()
  .accept(["image/*", "pdf"])
  .max_size(5 * 1024 * 1024)
  .description("PNG, JPG or PDF, up to 5 MB")
  .on_files(|files| upload(files))
  .on_reject(|refused| say_why(refused))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `DropZone::new()` | A zone that takes any number of files of any kind and size |
| `accept(impl IntoIterator<Item = &str>)` | The files it takes: extensions (`"png"`, `".png"`) and kinds (`"image/*"`, `"video/*"`, `"audio/*"`, `"text/*"`, `"archive/*"`), in any case. A second call adds. Nothing accepted takes every file |
| `multiple(bool)` | More than one file at once: on unless told. Off, the first is taken and the rest refused as too many |
| `max_size(u64)`, `max_files(usize)` | The greatest size of a file in bytes (a file whose size is not known is taken), and the most files of one drop or choice |
| `icon(impl Into<IconChoice>)` | The icon over its words in place of the upload icon, or `None` |
| `title(&str)`, `description(&str)`, `no_description()` | What it says, in place of "Drop files here" and "or click to browse" |
| `drop_text(&str)`, `reject_text(&str)` | What it says while files it takes, or files it takes none of, are dragged over it, in place of "Drop to add" and "These files can't be added" |
| `label(&str)` | What it is called to tooling, and its dialog's title; its title where it has none |
| `invalid(bool)`, `disabled(bool)` | Its edge in the destructive color; or it takes no drop and no press, dimmed |
| `on_files(Fn(&[DroppedFile]))` | Called with the files it took from a drop or from its dialog, in their order |
| `on_reject(Fn(&[RejectedFile]))` | Called with the files it refused and why: `RejectReason::Kind`, `TooLarge`, `TooMany`, `Folder` |
| `mount(ctx)` | The zone mounted where it is declared: it shows a drag over it, and a press, Enter or Space opens the system's dialog |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `state(DropZoneState::DragOver, true)` draws a drag with none |

- It is a stop of Tab, a `button` to tooling called by its label. A press, Enter or Space opens the system's dialog for files to open ([`lurq::files::FileDialog`](../styling-events/#files-a-person-gives), with the feature `file_dialog`), filtered by the extensions it accepts; a second press while the dialog is open opens no second one. What is chosen is sorted as a drop is.
- A drag over it is taken where it takes at least one of its files; one it takes none of is refused (`FileDragEvent::refuse`) and drawn so. The drop gives `on_files` what it takes and `on_reject` the rest, each only where there is something to tell. A folder is refused.
- Showing a drag and opening the dialog are what the mounted zone does: converted into an element on its own it is its still form, which takes a drop, shows no drag and opens nothing.
- Parts: `DropZonePart::Root` (the zone), `Icon`, `Text` (the column of the title and the description), `Title`, `Description`. States: `DropZoneState::DragOver`, `Rejected`, `Invalid`, `Disabled`, `Icon`, `Description`.
- In the design the zone is 320 by 140 on `Muted`, with the large radius and an edge of 1.5 in `Input` inside it; in its middle the upload icon of 20 in `MutedForeground`, 8 over "Drop files here" in 14 medium in `Foreground` and "or click to browse" in a caption's 12 in `MutedForeground`. The board draws it at rest only: a drag it takes is drawn as the board's Drop Target, on `InfoMuted` with an edge of 2 in `Info` and its icon and title in `Info`; a drag it refuses the same in `DestructiveMuted` and `Destructive`. Under the pointer its edge is `Ring`, focused from the keyboard it has the focus ring, invalid its edge is `Destructive`. A longer title wraps and the zone grows; it is padded 24 across and 16 down for that, which the board's zone, of three short lines, does not show.
- In a [field](#field) it is called by the field's label, invalid where the field is, and disabled where it is.
- A renderer that draws another zone starts from `view.zone()`, the element files are dropped on and that is pressed, and puts `view.icon()` and `view.text()` in it, or what it likes.

## File Upload

The files a person gives, by the system's file dialog or a drop, in a list with how far each is sent. The app sends them and says how far: lurq sends nothing.

```rust
use lurq::{files::DroppedFile, ui::{FileUpload, UploadFile}};

// files: Signal<Vec<UploadFile>>
FileUpload::new(files.clone())
  .accept(["image/*", "json"])
  .on_files(|given| send(given))
  .on_retry(|file| send_again(file))
  .mount(ctx);

// As the app sends them, it moves each file on.
files.update(|files| files[0] = files[0].clone().uploading(0.62));
```

| Builder | |
| --- | --- |
| `FileUpload::new(Signal<Vec<UploadFile>>)` | Bound to the app's list: what is given is added as waiting and a removed file is taken out, then the handlers are told |
| `FileUpload::controlled(files, on_change)` | The list the app holds: a person's change is told to `on_change` as the whole new list, and the upload shows what the app answers with |
| `FileUpload::at(files)` | A list shown and not operated: nothing is given, removed or sent again |
| `accept`, `multiple`, `max_size`, `max_files` | What it takes, as a [drop zone](#drop-zone)'s; `max_files` counts the files the list holds |
| `zone(bool)` | A drop zone in place of the button; `title` and `description` are the zone's words |
| `choose_label(&str)` | What the button says, in place of "Choose files" |
| `remove_label(&str)`, `retry_label(&str)` | What a file's controls are called before its name, in place of "Remove" and "Retry" |
| `size_text(Fn(u64) -> String)` | How a file's size is said, in place of "4 KB" and "1.2 MB" (`lurq::files::format_size`) |
| `label(&str)`, `invalid(bool)`, `disabled(bool)` | What it is called to tooling and its dialog's title; its button's or zone's edge destructive; nothing given or removed, its list dimmed |
| `on_change(Fn(&[UploadFile]))` | Called with the whole new list a person made; bound, after the signal is set |
| `on_files(Fn(&[DroppedFile]))`, `on_reject(Fn(&[RejectedFile]))` | Called with the files given, which the app sends, and with those refused and why |
| `on_remove(Fn(&UploadFile))`, `on_retry(Fn(&UploadFile))` | Called with a file a person removed (one being sent is to be stopped), and with a failed file to send again (the app sets it uploading) |
| `mount(ctx)` | Mounted where it is declared: its button or zone opens the system's dialog, the zone shows a drag, and the focus of a removed file's control goes on |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. With an `id`, the button or zone is `<id>-trigger`, a file `<id>-<file's id>`, its controls `<id>-<file's id>-remove` and `-retry` |

| `UploadFile` | |
| --- | --- |
| `UploadFile::new(DroppedFile)` | A file waiting to be sent. Its `id`, what the app finds it by, is its path, else its name |
| `uploading(f32)`, `done()`, `failed(&str)`, `waiting()` | Where it is: being sent, a fraction from 0 to 1 of the way; sent; not sent, and why; waiting again |
| `id(..)`, `key(..)`, `disabled(bool)` | Its id in place of its path; the key of its element; whether a person may not remove it or send it again |

- Files dropped anywhere on it are given as a drop zone's are, and a drag it takes nothing of is refused. Tab reaches the button, then each file's controls in order; Enter or Space presses them. Mounted, a removed file's control gives the focus to the next file's, else the one before, else the button.
- A file says its size while it waits and once it is sent, and how far it is while it is sent, with a bar under its line. A failed file says why under its line and has a retry control before its remove control. A file that was sent has the check and no control: the app takes it out where it likes.
- A name longer than its line ends in an ellipsis; its size keeps its room.
- Parts: `FileUploadPart::Root`, `Trigger` (the button, whose rules are laid over a button's root), `List`, `Item`, `Icon` (the icon of the file's kind: an image, a text, an archive, a file), `Text`, `Label` (the name), `Meta` (the size or how far), `Progress`, `Error`, `Status` (the check), `Remove`, `Retry`. States: `FileUploadState::Zone`, `Invalid`, `Disabled`, and on a file's parts `Waiting`, `Uploading`, `Done`, `Failed`, `Unavailable` (a disabled file).
- In the design the upload is 300 wide: the outline button of 36 with the upload icon, and 8 under it the files 8 apart. A file is a row with an edge of 1 in `Border` inside it, the medium radius, padded 12 across and 10 down, its parts 10 apart: the icon of its kind, 16 in `MutedForeground`; its name in 13 medium in `Foreground`, and 8 after it its size or how far in 11 mono in `MutedForeground`; while it is sent a progress of 4, 6 under the line; at the end the check in `Success` or the remove control of 16 in `MutedForeground`. The board draws no failed file: its edge and its icon are `Destructive`, its error in a caption's 12 in `Destructive`. With a zone it is 320 wide, the zone's.
- In a [field](#field) it is called by the field's label, invalid where the field is, and disabled where it is.
- A renderer that draws another upload has `view.trigger()`, `view.item(index)`, `view.list()`, and `view.files`; `view.item_part(index, part)` is a part in its file's state.

## Date Field

A date typed in by its segments, the day, the month and the year, each stepped by the arrows: a date of birth, a date in a form where a calendar would be in the way.

```rust
use lurq::{
  core::{Date, Signal},
  ui::{DateField, DateMemory, Field},
};

// born: Signal<Date>; mounted, so that it keeps what is half typed.
DateField::new(born.clone()).label("Date of birth").mount(ctx);

// A date that may be none, in a field; the field mounted mounts the date field.
Field::new("Due date", DateField::optional(due.clone()).min(Date::today())).mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<Date>)`, `optional(Signal<Option<Date>>)` | The date the field holds, which a date typed whole sets; `optional` holds none while a segment is empty |
| `controlled(Date, Fn(Date))`, `controlled_optional(Option<Date>, Fn(Option<Date>))` | The app's date, and what it is told of a date typed, which the field does not set |
| `memory(&DateMemory)`, `mount(ctx)` | What it keeps of what is half typed and which segment has the focus: the app's, or its own where it is mounted |
| `min(Date)`, `max(Date)` | The least and the greatest date that is the field's: one typed out of them is no value |
| `today(Date)` | The day an empty year starts at under the arrows |
| `locale(impl DateText)` | The order of the segments, what stands between them, their names and what an empty one shows |
| `icon(impl Into<IconChoice>)` | The glyph at the end: the calendar's, or `None` |
| `size(DateFieldSize)`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `full_width`, `label`, `name` | As an [input](#input)'s; `DateFieldSize` is `TextBoxSize`. The form holds the date in ISO 8601 |
| `on_change(Fn(V))` | Told of another date: once every segment is filled, or, for `optional`, when one is emptied. Not for the app setting it |
| `state`, `style`, `rules`, `force`, `id`, `key` | As an input has them. Each segment has the id with its name after it: `due-day`, `due-month`, `due-year` |

- Each segment is a spin button and a stop of Tab, as a browser's own date input and the date fields of macOS are: Tab and Shift+Tab go from one to the next, and so do Left and Right. Digits fill the focused segment and go on to the next once it is full, or once no other digit could follow (`4` is April). Up and Down step it, round from the last to the first, and an empty year starts at this year; Home and End go to its ends; Backspace takes the last digit typed, or empties it.
- The value is a date only when every segment is filled with a day that exists and is in the range. Until then a field of `new` keeps the date it had, one of `optional` holds none, and what is typed stays. A date the app sets shows in place of what was typed.
- To what reads the tree the field is a `group` called by its label, of `spinbutton`s called by the segment's name (`Month`) that say `a11y_valuenow`, `a11y_valuemin`, `a11y_valuemax` and `a11y_valuetext` (`November`).
- Its text is its `.locale(..)`, else the app's `app.set_date_locale(..)`, else English, which writes the month first (`11/12/2026`); `en-GB` writes the day first, as the board draws it.
- Parts: `DateFieldPart::Root` (the box), `Segments` (the row of segments, its gap), `Segment`, `Separator`, `Icon`. States: `DateFieldState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`; of a segment `Focused` and `Placeholder`.
- In the design the box is 180 by 36 on `Background`, an edge of 1 in `Input`, `radius-md` and 10 of padding; its segments, 2 apart, are in the mono type at 13 in `Foreground`, 19 high with 3 of padding across and a radius of 3, the separators in `MutedForeground`. The segment with the focus is the box's selected text, on `Selection` in `Foreground` as a text input's selection is; the board fills it with `info` under white, for which the palette has no pair of roles. A calendar's glyph of 16 in `MutedForeground` is at the end. An empty segment, which the board does not draw, shows `dd`, `mm`, `yyyy` in `MutedForeground`.
- A theme's renderer is given `DateFieldView` (`SegmentFieldView`): `view.segments` (each a `SegmentShown`), and the pieces `segment(index)`, `separator(index)`, `segments_row()`, `icon()` and `render()`.

## Time Field

A time typed in by its segments, as a date field is a date: the time of a meeting, an alarm, the opening hours of a shop.

```rust
use lurq::{
  core::{Signal, Time},
  ui::TimeField,
};

// starts: Signal<Time>; the minutes step by a quarter of an hour.
TimeField::new(starts.clone()).label("Starts at").step(15).mount(ctx);

// With seconds, a time that may be none.
TimeField::optional(lap.clone()).seconds(true).mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<Time>)`, `optional(..)`, `controlled(..)`, `controlled_optional(..)` | As a date field's, of a `Time` |
| `step(minutes)` | What Up and Down step the minutes by: 1, or 15 for a quarter of an hour. Any minute may be typed |
| `seconds(bool)` | A segment for the seconds. Without it the value's seconds are kept as they are |
| `hour_cycle(HourCycle)` | From 0 to 23, or from 1 to 12 with a last segment for AM and PM, in place of the locale's |
| everything else | As a [date field](#date-field)'s: `min(Time)`, `max(Time)`, `locale`, `memory`, `icon` (the timer's), the input's sizes and states |

- Its segments are a date field's. On a clock of twelve hours the last is the half of the day, which `A`, `P` and the arrows set. Up and Down go round the clock: 23 is before 0, and a step of 15 goes from 45 to 0.
- In the design the box is 120 by 36, the hours and the minutes with a colon between them, a timer's glyph at the end; it is wider for its seconds and its half of the day. Its parts and states are the date field's (`DateFieldPart`, `DateFieldState`).

## Time Picker

A field that holds a time, typed in it or chosen from columns of hours and minutes that open under it: the time of a meeting, a reminder, a delivery slot.

```rust
use lurq::{
  core::{Signal, Time},
  ui::TimePicker,
};

// at: Signal<Time>; the minutes offered by the quarter of an hour.
TimePicker::new(at.clone()).step(15).mount(ctx);

// Half hours of an afternoon, the panel drawn in place.
TimePicker::new(slot.clone())
  .step(30)
  .min(Time::new(13, 0, 0).unwrap())
  .max(Time::new(16, 0, 0).unwrap())
  .inline(true)
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<Time>)`, `optional(..)`, `controlled(..)`, `controlled_optional(..)` | The time the field holds: typed, or chosen from the panel |
| `step(minutes)` | The minutes the panel offers apart: 15 as the board draws them |
| `hour_cycle(HourCycle)` | From 0 to 23, or from 1 to 12 with a column of AM and PM |
| `min(Time)`, `max(Time)` | What can be typed or chosen: the hours and minutes out of it are not offered |
| `memory(&DateMemory)`, `mount(ctx)`, `open(Signal<bool>)` | Whether the panel is open and the field it opens under |
| `placement`, `offset`, `collision`, `inline` | As a [date picker](#date-picker)'s panel |
| `trigger_label(&str)`, `panel_label(&str)` | What the button and the panel are called to tooling: the locale's `Choose time` |
| `placeholder`, `locale`, `size(TimePickerSize)`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `full_width`, `label`, `name` | As a date picker's |
| `on_change(Fn(V))`, `on_input(Fn(&TextInputEvent))`, `on_submit(Fn(V))`, `on_open_change(Fn(bool))`, `on_dismiss(Fn())` | As a date picker's; a minute closes the panel before `on_change` is called |

- What is typed is read as it is typed (`9:30`, `14:05`, `2:05 pm`) and is the value as soon as it is a time in the range; the focus leaving and Enter write it as the locale writes times.
- The panel opens from the button at the start of the field and from Down in it, the focus on the hour held, each column scrolled to its item. A press of an hour sets the hour and leaves the panel open; a press of a minute sets the minute and closes it, the focus back in the field.
- To what reads the tree the field is a `combobox` that opens a `dialog` of `listbox`es called by the segment's names, of `option`s.
- Parts: `TimePickerPart::Control` (rules over the input's box), `Trigger`, `TriggerIcon`, `Panel` (rules over the popover's panel), `Column`, `Item`. States: `TimePickerState::Open`, the field's, and of an item `Selected`.
- In the design the field is an input 160 by 36 with a timer's glyph of 16 at its start and the time 8 after it in the mono type at 14; the panel opens 4 under it, 160 wide, with 4 of padding and 4 between its two columns, items 74 by 30 and 2 apart with `radius-sm`, their numbers in the mono type at 13, the held one on `Highlight` at 500. Four items of a column are seen; it scrolls.

## Month Picker

The twelve months of a year, of which a person chooses one, and the years before and after under the buttons of its heading: the month of a report, the expiry of a card.

```rust
use lurq::{
  core::{Signal, YearMonth},
  ui::{MonthPicker, YearPicker},
};

// month: Signal<YearMonth>; mounted, so that it keeps the year it shows.
MonthPicker::new(month.clone()).max(YearMonth::current()).mount(ctx);

// year: Signal<i32>
YearPicker::new(year.clone()).min(1900).mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<YearMonth>)`, `optional(..)`, `controlled(..)`, `controlled_optional(..)` | The month the picker holds, which a press sets |
| `min(YearMonth)`, `max(YearMonth)` | What can be chosen: the months out of it are muted, and the years beyond are not shown |
| `today(YearMonth)` | The month marked as this month |
| `locale(impl DateText)` | The months' short names (`Nov`, `Лис`) and the buttons' names |
| `previous_label(&str)`, `next_label(&str)` | What the buttons are called: the locale's `Previous year`, `Next year` |
| `memory(&DateMemory)`, `mount(ctx)` | The year shown, between renders |
| `label`, `invalid`, `disabled` | Its name to tooling, which makes the panel a `group`; a field's judgment; no input |
| `on_change(Fn(V))` | Told of another month chosen; not for the one chosen, nor for the app setting it |
| `state`, `style`, `rules`, `force`, `id`, `key` | Each month has the id with `-cell-` and the month after it: `due-cell-2026-11` (a year picker's year `due-cell-2026`); the buttons `-nav-previous` and `-nav-next` |

- `ui::YearPicker` is one of years: `YearPicker::new(Signal<i32>)` and the same builder over `i32`, twelve years from the start of a decade (`2020 – 2031`), the buttons going ten years, called `Previous years` and `Next years`.
- To the keyboard the cells are one stop of Tab, at the chosen one, this one or the first that can be chosen: Left and Right go one, Up and Down four (a row), Home and End to the ends of the row, PageUp and PageDown a page (a year of months, ten years), into the page before or after, which is then shown. Enter and Space choose. To what reads the tree the cells are a `grid` called by its heading, of `gridcell`s that say `a11y_selected`, `a11y_current` and `a11y_disabled`.
- Parts: `MonthPickerPart::Root` (the panel), `Header`, `Title`, `Nav` and `NavIcon` (rules over the bare buttons), `Grid`, `Row`, `Cell`. States: `MonthPickerState::Disabled`; of a cell `Selected`, `Current`, `Unavailable`.
- In the design the panel is 252 wide on `Popover` with an edge of 1 in `Border`, `radius-lg`, the popover's shadow and 12 of padding; the heading, 28 high, has the year in the body's type at 500 between two bare chevrons of 16 in `Foreground`, 8 over the grid. The months are in three rows of four, 8 apart, cells 34 high and 4 apart with `radius-md`, their names at 13; the chosen one is on `Primary`, its name in `PrimaryForeground` at 500. This month, which the board does not draw, is on `Highlight` as a calendar's today is; one out of the range is muted at half opacity.
- A theme's renderer is given `MonthPickerView` (`PeriodsView`, a `YearPickerView` too): `view.title`, `view.cells` (each a `CellShown`), and the pieces `nav(by)`, `header()`, `cell(index)`, `grid()` and `render()`.
## List

Rows one under the other: settings, people, files, notifications. A row has a title, a description under it, what stands before it and what stands at its end.

```rust
use lurq::ui::{Avatar, List, ListItem, Switch, icons};

List::new()
  .item(
    ListItem::new("Ivan Zaida")
      .description("Pushed 3 commits to feat/node-blur")
      .leading(Avatar::new("Ivan Zaida"))
      .meta("2m"),
  )
  .item(ListItem::new("Dependabot").description("Bumped wgpu to 26.0").selected(true));

// Settings: a value and a chevron where a row leads somewhere, a switch where it holds one.
List::new()
  .separators(true)
  .inset(40.0)
  .item(ListItem::new("Wi-Fi").icon(icons::WIFI).value("Home").chevron(true).on_click(|_| open_wifi()))
  .item(ListItem::new("Dark mode").icon(icons::MOON).trailing(Switch::new("Dark mode").label_hidden(true)));

// A contained list: the rows in a box, under a header.
List::new().contained(true).title("Recent").item(ListItem::new("theme.md").chevron(true).href("/docs/theme"));
```

| Builder | |
| --- | --- |
| `ListItem::new(title)` | A row that says `title`, with nothing before or after it |
| `description(&str)` | A second line under the title, in the muted color |
| `tag(element)`, `info(&str)` | The app's elements after the title on its line, 6 after it and 6 apart (a [tag](#tag) that says *Default*, a badge; part `ListItemPart::Tags`), and the mark of an [info label](#info-label) after them, its words in a tooltip above it wherever the row is rendered (part `Info`). The title gives way before them, and a press of the mark is not the row's |
| `detail(element)` | An element of the app's under the description, in the text's column, 2 under it (part `Detail`): a link that says more, a path on a line of its own. Its presses are its own |
| `below(element)` | What the row says under itself: a problem, a question, a code to copy. It stands across the row under the line of its title and its end, 12 in from the sides (14 in a setting) and 8 above the bottom (12 in a setting), one under another 8 apart (part `Below`); a list's line is under it, and a press of it is not the row's. The row's id is on the line that is pressed, its key on what holds both |
| `leading(element)`, `icon(name)` | What stands before the text: an avatar, a picture; or an icon of the theme's set |
| `checkbox(bool)`, `radio(bool)` | A checkbox or a radio before the text, checked or not: the row is that control, which its press changes through `on_click` |
| `value(&str)`, `meta(&str)` | A value at the end in the muted color ("Home"), or a note in the mono type ("2m") |
| `trailing(element)`, `shortcut(keys)`, `chevron(bool)` | What the app puts at the end (a switch, a badge), the keys that do what the row does, and a chevron for a row that leads somewhere |
| `on_click(handler)`, `href(path)` | What a press does, given the press (`Fn(MouseEvent)`), or where it goes in the app's router: the row is pressed. A second handler is called after the first |
| `on_open_aside(handler)`, `tab_stop(bool)` | Called, given the press, for the middle button or a press with Ctrl held (Cmd on macOS) in place of `on_click` and the path; and `false`, a pressed row Tab passes by and a press does not focus. See [pointer-only pressables](#pointer-only-pressables) |
| `selected(bool)`, `disabled(bool)`, `density(Density)` | The selected row is highlighted; a disabled one is dimmed and takes nothing; how high it is, whatever the theme is at |
| `List::new()`, `item(row)`, `items(rows)` | The rows, one under the other |
| `separators(bool)`, `inset(f32)` | A line between each two rows, and how far from the start it starts |
| `contained(bool)`, `title(&str)`, `action(element)` | The rows in a box, touching, under a header with a title (in capitals) and an action: the one action, a second takes its place |
| `label(&str)`, `density(Density)`, `force(..)` | What the list is called to what reads the tree; the density of every row; every row drawn as under the pointer or focused |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them, on the row and on the list |

- A row given `on_click` or `href` is pressed: by the pointer, and by Enter and Space while it is focused, and Tab stops at it. A row that is not pressed is not a stop of Tab and answers no pointer: a control in it (a switch at its end) is pressed on its own. In a row that is pressed, what the app put at its end with `trailing` keeps its own presses, which are not the row's, and is a stop of Tab after the row. A title longer than its room is cut with an ellipsis, and a description goes on to the next line. In a list Up and Down, Home and End move between the rows that are pressed and the controls in rows, and a letter to the next whose title starts with it.
- A row with `href` goes to its path in the router around it (`router::current_navigator`), after its `on_click`; outside a router it goes nowhere.
- What is selected is the app's: `selected(true)` on the row. A list of which one or many are chosen by a person is a [listbox](#listbox).
- To what reads the tree a list is a `list` called by its label or its title. A row is a `listitem`; a pressed one a `button`, a `link` with a path, a `checkbox` or `radio` with one before its text (with `a11y_checked`), and a selected one says `a11y_selected`.
- Parts of a row: `ListItemPart::Root` (what is pressed, selected and focused), `Leading`, `Icon`, `Content`, `Title`, `Description`, `Tags`, `Info`, `Detail`, `Below`, `Trailing`, `Value`, `Meta`, `Chevron`. Given an id `x`, the title is `x-title` and the description `x-description`. States: `ListItemState::Selected`, `Disabled`, `Pressable`, `Description` (it has one), `Icon` (what is before its text is an icon), `Contained`, `Setting` (it is a row of a [settings card](#settings-card), which draws it as the design's Settings Row). Parts of a list: `ListPart::Root`, `Header`, `Title`, `Action`, `Items`, `Separator` (what holds a line; its padding at the start is the inset). States: `ListState::Contained`, `Separator`.
- In the design a row is 36 high (28, 44 and 56 at the compact, comfortable and touch densities), 12 in, with the medium radius and 12 between what it holds; with a description it is 16 higher, 52 at the default, its title at 500. The title is the body's type at 14, the description 12 in `MutedForeground` 2 under it, a value 13 and a chevron of 14 in `MutedForeground`, a note the mono type at 11, an icon 16 in `MutedForeground`. The selected row is on `Highlight`. A pressed row under the pointer and held down has the `StateHover` and `StatePressed` veils, the focus from the keyboard is a ring of 2 in `Ring` inside it, and a disabled row is half opaque. Under a finger a row is the board's mobile row: 56 high, 16 in, its title at 16, its value at 15, its chevron 18, its icon of 18 on a tile of 32 on `Muted` with the large radius.
- Rows are 2 apart; with lines between them they touch, a line of 1 in `Border` from the inset to the end along the bottom of the row over it, inside it: the lines add no height, and a contained list of three rows is 32 + 3 × 36 = 140, as the board's is. A disabled row says `a11y_disabled` to tooling. A contained list is a box on `Background` with an edge of 1 in `Border` and the large radius; its header is 32 high on `Muted`, 12 in, with a line under it, its title the mono type at 11 in `MutedForeground`, 1 apart; its rows are square, their titles 13 and 8 from what is at their end.
- A theme's renderer is given `ListItemView` (`view.root(children)`, `view.leading()`, `view.content()`, `view.trailing()`, `view.below()`, `view.render()`; `view.tags`, `view.info`, `view.details`) and `ListView` (`view.root()`, `view.header()`, `view.rows()`, `view.separator()`).

## Listbox

A list in the page of which one option is chosen, or any number: a renderer, a time zone, the columns a table shows.

```rust
use lurq::{components::SelectOption, ui::Listbox};

// renderer: Signal<&str>
Listbox::new(renderer, [
  SelectOption::new("wgpu", "wgpu").group("Renderers"),
  SelectOption::new("dx12", "DirectX 12"),
  SelectOption::new("metal", "Metal"),
  SelectOption::new("gl", "OpenGL (legacy)").disabled(true),
])
.label("Renderer");

// None chosen until an option is: a Signal<Option<&str>>, and on_change is told the Option.
Listbox::optional(theme, [("light", "Light"), ("dark", "Dark")]).on_change(|theme: &Option<&str>| save(*theme));

// Any number: a Signal<Vec<&str>>, and on_change is told the whole set.
Listbox::multiple(columns, [("name", "Name"), ("size", "Size"), ("kind", "Kind")]).label("Columns");

// What is chosen held by the app: it is told, and answers.
Listbox::controlled(renderer, options, move |chosen: &&str| ask_to_switch(*chosen));

// A long list scrolls in a box with a most height.
Listbox::new(zone, zones).style(ListboxPart::Root, |s| s.max_height(240.0));

// Thousands of options: in a virtual list, in a box with a height.
Listbox::new(item, items).style(ListboxPart::Root, |s| s.height(320.0)).mount(ctx)
```

| Builder | |
| --- | --- |
| `new(value, options)` | Bound to a `Signal<T>`: the option of its value is chosen, and choosing another sets it. Options as a [select](#select) takes them: `(value, label)`, a `Choice`, or a `SelectOption` with `group`, `detail`, `hint`, `leading`, `trailing`, `disabled` |
| `optional(value, options)` | Bound to a `Signal<Option<T>>`: as `new`, with none chosen while it is `None` |
| `multiple(values, options)` | Bound to a `Signal<Vec<T>>`: the options of its values are chosen, and choosing one adds it or takes it out |
| `controlled(value, options, on_change)`, `controlled_optional(..)`, `controlled_multiple(..)` | The same three, held by the app: a `T`, an `Option<T>` or the values, chosen as it says; `on_change` is told what a press or a key would choose, and the listbox shows what the app answers with |
| `follow_focus(bool)` | Of one, whether the arrows choose as they move. They do unless told otherwise; without it Space, Enter and a press choose |
| `checkboxes(bool)` | Of many, a checkbox before each option's label, ticked while it is chosen, in place of the check at its end and the highlight: a press checks one or unchecks it and keeps the rest, a press with Shift and Shift with the keys check a range and keep what else is checked. A [transfer](#transfer)'s lists are these |
| `on_change(handler)` | Told the new choice after each press or key that changes it, after the signal is set, and not for what is already chosen nor when the app sets the signal: `&T` of a listbox made with `new` or `controlled`, `&Option<T>` of an optional one, `&[T]`, the whole set, of a multiple one. A handler of another type does not compile. A second handler is called after the first |
| `label(&str)` | What it is called, to what reads the tree. A [field](#field) gives it its label |
| `invalid(bool)`, `disabled(bool)`, `full_width(bool)`, `density(Density)` | Its edge in `Destructive`; no option chosen; as wide as what holds it, in place of 220; how high an option is |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box; each option's is that id, `-` and its place from 0 |
| `mount(ctx)` | In place of building it: the options in a virtual list, which builds only those near the view, for thousands. Give `Root` a height |

- It is one stop of Tab, at the chosen option or the first. Up and Down move through the options, Home and End to the first and the last, PageUp and PageDown by what the box shows, without going round, and a letter to the next option that starts with it; a disabled option is passed by.
- Of one, the choice follows the focus, as in a radio group: a key that moves the focus chooses what it moves to, a letter too. With `follow_focus(false)` the focus and the choice are apart: the focus moves alone and Space, Enter or a press choose.
- Of many, a press chooses one alone, with Ctrl (Cmd) it adds or takes out one, with Shift it chooses from the last chosen to it. Space adds or takes out the focused option; Shift with the arrows, Home, End and the page keys chooses from where the choice started to where the focus goes; Ctrl+A chooses every option that can be. The last of the `Vec` is where a range starts.
- The keys are the engine's: the options are a [focus group](../focus-navigation/#focus-groups) that is `one_stop`, with `typeahead`, `pages` and without `wrap`; one of one `activate`s, one of many `shift_extends`. The range and the toggle are `ui::selection`'s, which the [tree view](#tree-view) shares.
- Mounted with `mount(ctx)`, the options are in the engine's virtual list and the group steps over every option, built or not (`FocusRows`): the keys, the page keys and typeahead go to any option, the list builds it and the focus goes to it, and the choice follows as it does where every option is built; Shift with the keys and Ctrl+A choose over every option. The list opens at the chosen option. Each option is drawn with `ListboxOption::render`, its group's label over it; a theme's renderer of the whole listbox is not used there, its rules are.
- To what reads the tree it is a `listbox` (`a11y_multiselectable` of many) called by its label, with `option`s that say `a11y_selected`, `a11y_posinset` and `a11y_setsize`; a group's label is a `group`.
- Parts: `ListboxPart::Root` (the box; its most height makes the options scroll), `Content` (the options with the keys), `Group`, `Option`, `Leading`, `Label`, `Detail`, `Hint`, `Trailing`, `Check`. States: `ListboxState::Chosen`, `OptionDisabled`, `Detail` (of an option), `Checkboxes`, `Disabled`, `Invalid`, `Multiple`, `FullWidth`.
- In the design the box is 220 on `Background` with an edge of 1 in `Border` and the large radius, its options 1 apart, 4 in. An option is 32 high (24, 40 and 44 at the compact, comfortable and touch densities), and 16 higher with a detail under its label (48; 40, 56 and 60), 8 in, with the small radius, its label in the body's type at 13. The chosen one is on `Highlight`, its label at 500, with a check of 14 at its end; the one under the pointer is on `Muted`; one held down has the `StatePressed` veil; the focus from the keyboard is a ring of 2 in `Ring` inside it; a disabled one says its label in `MutedForeground`, faded, is drawn as nothing points at it or presses it whatever the listbox is forced to, and says `a11y_disabled` to tooling. A group's label is 11 at 600 in `MutedForeground`, 6 over it, 4 under it, 8 in.
- A theme's renderer is given `ListboxView`: `view.options` (each a `ListboxOption` with its label, group, detail, whether it is chosen and disabled, and `option.button()`, `option.group_label()`, `option.text()`, `option.check()`, `option.checkbox()`, `option.render()`), `view.content()` (the column with the keys) and `view.root(content)`. It is set for `Listbox<T>` of any `T`, and draws every form.

## Data List

Terms with their values: what a thing is, item by item. The details of a build, of an order, of a file.

```rust
use lurq::ui::{Badge, DataList, Orientation};

DataList::new()
  .text("Version", "0.60.0")
  .text("Renderer", "wgpu")
  .text("License", "MIT")
  .item("Status", Badge::new("Stable").variant("success"));

// Each term over its value, for a narrow column.
DataList::new().orientation(Orientation::Vertical).text("Ship to", "Ivan Zaida, 12 Khreshchatyk St, Kyiv");
```

| Builder | |
| --- | --- |
| `text(term, value)` | An item whose value is a text, in the value's type |
| `item(term, element)` | An item whose value is what the app puts there: a badge, a link, tags |
| `texts(items)`, `items(items)` | Several of them at once, from `(term, value)` pairs |
| `orientation(Orientation)` | `Horizontal` (the default), each term beside its value, or `Vertical`, each over it. `DataListOrientation` is `Orientation` |
| `align(Alignment)` | How a term and its value stand to each other across: `Center` (the default), the term centred on its value's height; `Start`, the term at the top of its value, where values wrap onto several lines or are elements of several lines |
| `label(&str)` | What the list is called, to what reads the tree |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- Side by side, the terms are one column as wide as the widest of them and the values the column of the rest of the width (a grid of two columns, `Auto` and `Fr(1.0)`); a line runs under each item but the last, the term's and the value's meeting. One over the other, each item is a column.
- To what reads the tree each term is a `term` and each value its `definition`.
- Parts: `DataListPart::Root`, `Item` (an item one over the other), `Term`, `Value`. States: `DataListState::Horizontal` and `Vertical` (one of them is on), `Separator` (of an item's parts: a line under it).
- In the design each term and its value are 10 from the lines over and under them and 12 apart, both in the body's type at 13, the term in `MutedForeground` and the value in `Foreground`; the line is 1 in `Border`. One over the other the term is 12, 2 over its value.
- A theme's renderer is given `DataListView`: `view.items` (each a term with its text or its element), `view.orientation`, `view.align`, `view.id` and `view.key`, `view.term(item, lined)`, `view.value(item, lined)` and `view.render()`. A root it builds with no id of its own is given the list's: the tests and the tooling that find the list by its id find it whatever the theme.

## Timeline

What happened, in its order: the releases of a project, the history of a deploy, the tracking of an order, the activity of a document, a log.

```rust
use lurq::ui::{Avatar, Button, Card, Orientation, Timeline, TimelineAlign, TimelineItem, TimelineSize, TimelineVariant};

Timeline::new([
  TimelineItem::new("Released 0.60").description("Blur and frosted glass land in the renderer.").time("Nov 10"),
  TimelineItem::new("Merged node-blur").description("Blurs follow transforms and rounded clips.").time("Nov 8"),
  TimelineItem::new("Opened pull request").time("Nov 3").upcoming(true),
])
.label("History");

// A deploy: an icon for each mark, in the item's tone; the current item running.
Timeline::new([
  TimelineItem::new("Built").icon("box").variant(TimelineVariant::Success),
  TimelineItem::new("Tests failed").icon("close").variant(TimelineVariant::Destructive),
  TimelineItem::new("Retrying").pending(true).current(true),
  TimelineItem::new("Deploy").upcoming(true),
]);

// Activity: an avatar for the mark, a comment under the item, an action at its end.
TimelineItem::new("Olena Koval commented")
  .media(Avatar::new("Olena Koval").size("xs"))
  .time("2 hours ago")
  .child(Card::new().child(comment))
  .action(Button::new("Reply"));

// An order's steps across, each told by its value when it is pressed.
Timeline::new([TimelineItem::new("Ordered").value(1u32), TimelineItem::new("Shipped").value(2)])
  .orientation(Orientation::Horizontal)
  .on_select(|step: &u32| open_step(*step));

// A log: small, the times in a column, the history loaded a page at a time.
Timeline::new(entries)
  .size(TimelineSize::Sm)
  .align(TimelineAlign::TimeColumn)
  .on_load_more(move || fetch_next_page())
  .loading(fetching);
```

| Builder | |
| --- | --- |
| `Timeline::new(items)` | The items, the first at the top or the start; `item(item)` and `items(items)` add more. `Timeline<T>` holds items of `TimelineItem<T>`, `()` unless they have a `value` |
| `orientation(Orientation)` | `Vertical`, the board's: one under another, the line down. `Horizontal`: side by side, the line across, each item's text under its mark |
| `size(impl Into<Name>)` | A `TimelineSize`: `Md`, the board's, or `Sm`, a log |
| `align(TimelineAlign)` | Where the text stands: `Start`, after the line (the board's); `Alternate`, on either side of it item by item; `TimeColumn`, the time in a column of its own before the line, over it across |
| `on_select(Fn(&T))` | Told the value of the item pressed, after the item's own handlers: every item that is not disabled is then pressed. A second handler is called after the first |
| `on_load_more(Fn())`, `loading(bool)` | An item at the end that shows more, which the line runs on to: its button calls the handler, and while `loading` its mark is a spinner and it takes no press. The app stops giving `on_load_more` once it has loaded everything |
| `more_label(&str)`, `loading_text(&str)` | What that button says, "Show more" and "Loading" when not given |
| `label(&str)` | What the timeline is called, to what reads the tree |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the first item that is pressed |
| `mount(ctx)` | In place of building it: a component that reads its own room, so that alternate sides fold to one where it is given less than `Timeline::ALTERNATE_LEAST` (480) across. It takes the width of what holds it. Its value is to be `Clone` |
| `TimelineItem::new(&str)` | What happened: its title |
| `value(T)` | What the timeline's `on_select` is told when the item is pressed: the id of the event, of the commit, of the step |
| `description(&str)`, `time(&str)` | The text under the title, and when it happened as the app writes it ("Nov 10", "2 hours ago"): under the description, beside the title in the small size, or apart where the timeline's `align` puts it |
| `icon(name)`, `media(element)` | In place of the dot: an icon in a well, or the app's mark, an avatar |
| `variant(impl Into<Name>)` | A `TimelineVariant` (`Neutral`, `Info`, `Success`, `Warning`, `Destructive`), or a variant a theme adds: the color of the mark. One that failed is `Destructive` |
| `current(bool)`, `upcoming(bool)`, `pending(bool)` | The one that is now, its mark ringed; one to come, its dot hollow; one running, a spinner for its mark |
| `child(..)`, `children(..)` | Content under the time: a card, a comment, a list of files |
| `action(element)` | The one action, at the end of the item beside what is pressed; a second takes the place of the first |
| `on_select(Fn())` | The item is pressed: a press, Enter or Space calls it, before the timeline's handler. A second handler is called after the first |
| `disabled(bool)`, `id`, `key` | An item that is pressed takes nothing, faded and passed by the keys |

- Each item is its rail beside its content. The rail is the mark over a line that runs to the next item's mark: the line is as long as the item, so a title or a description that wraps, or content under the time, makes it longer. The last item has no line and no room under its content, unless the timeline shows more, whose item is then the last.
- The titles and the descriptions wrap in the room the rail, the other side and the action leave them; the action keeps its room at the end of the item, beside what is pressed and never in it.
- A timeline is read, not operated, but where an item is pressed (its own `on_select`, or the timeline's) the pressed items are one stop of Tab, a [focus group](../focus-navigation/#focus-groups) that is `one_stop`, with `typeahead` and without `wrap`: Up and Down move between them (Left and Right across), Home and End go to the first and the last, a letter to the next title it starts, and a disabled item is passed by. An item's action is a stop of its own.
- Across, the items share the width, each at least 160 (120 small), and wrap their text in it; a timeline wider than its room scrolls sideways in a [scroll area](#scroll-area) that fills its width (`ScrollArea::fill_width`), and the keys bring the focused item into view. Across, the text is under the marks, and the time column is a line over them; alternate sides are drawn down only.
- Alternate sides need room. Mounted with `mount(ctx)`, a timeline measures the room it is given and folds them to one side, as `Start` draws it, where that is less than `Timeline::ALTERNATE_LEAST`, 480: in a narrow panel of a wide window too. It measures with [`ElementRef::observe_rect_before_paint`](../layout/#a-form-that-follows-its-own-room), so the frame that lays it out shows the form that fits, from its first frame and as a splitter's divider drags its pane across that width. Built as an element of its own (converted with `into()`, or built by another component's renderer) it has no room of its own to read, and folds them in a window narrower than the theme's `md` breakpoint, 768 by default.
- A long history is loaded a page at a time, as a feed is: `on_load_more` and `loading`, with the app appending what comes. It is not in a virtual list: a timeline stands in the page and scrolls with it, where a virtual list needs a box of its own, and its items hold the app's elements (a card, a comment). Every item it holds is built.
- The small size is a log: its items are rows, as high as the theme's [density](../theme/#density) makes a row of 28 at the least (20, 36 and 40 at compact, comfortable and touch); the board's size is not a row and keeps its height.
- To what reads the tree it is a `list` called by its label, of `listitem`s called by their titles and described by their descriptions; the current one says `a11y_current`, a pending one `a11y_busy`, and inside an item that is pressed is a `button`. The item that shows more is a `listitem` with its button, busy while it loads.
- Parts: `TimelinePart::Root` (the list), `Item`, `Rail`, `Dot`, `Mark` (the well of an icon, what holds the media, the spinner), `Icon`, `Separator` (the line), `Content` (the title, the description, the time and the body), `Header` (the title, and the time beside it in the small size), `Title`, `Description`, `Time`, `Body`, `Action` (at the end of the item), `Opposite` (the side of the line the time stands on where it is apart), `More` (the button that shows more). States of the timeline: `TimelineState::Vertical` and `Horizontal` (one is on), `Start`, `Alternate` and `TimeColumn` (one is on, as the timeline is laid out), `Mark` (an item has an icon or media); of an item: `Before` (its text is before the line), `Current`, `Upcoming`, `Pending`, `Last`, `Icon`, `Media`, `Pressable`, `Disabled`; of the item that shows more: `More`, `Loading`. Sizes: `TimelineSize::Sm`, `Md`.
- In the design an item's rail is 12 and its content 12 from it. The rail is 4 down to a dot of 10 in `Primary` and, 4 under it, a line of 1 in `Border`. The title is the body's type at 14 at 500 in `Foreground`, the description 13 in `MutedForeground` with lines 1.4 high, the time the mono type at 11 in `MutedForeground`, 2 apart, with 20 under them. The board's last item has a hollow dot, `Background` with an edge of 1.5 in `Input` inside it.
- The board draws nothing else of it, and these follow its parts: a tone colors the dot (`Primary`, `Info`, `Success`, `Warning`, `Destructive`), and the edge of a hollow one; the current dot has a ring of 2 in its color, 2 out from it; an icon is 14 in a well of 24 on `Muted`, in the tone's color; media stands in a mark of 24, and every rail of a timeline with a mark is 24 wide, its line stopping 4 over the next mark; a pending item's mark is a spinner of 12; the mark of a current item is ringed as its dot would be, and the well of an upcoming one is hollow. A body is 8 under the time. An item that is pressed has the `StateHover` and `StatePressed` veils and a ring of 2 in `Ring` inside it, with the medium radius, and is half opaque where it is disabled.
- The small size: a dot of 8 on a rail of 8, 10 from the content, the title 13 with the time at the end of its row, the description 12, 4 under the content; a mark is 20, its icon 12. The time column is 80 (64 small), its time at its end, 2 down to stand by the title. Alternate, the two sides share the room, the text before the line at its end. Across, the rail is a row 12 high (10 small, 24 with marks), its mark in its middle and its line 4 short of the next mark, the content 8 under it (6 small) and 12 from the next item. The item that shows more is a small ghost button, its hollow dot in the middle of it.
- A theme's renderer is given `TimelineView`: `view.items` (each a `TimelineItemView` with its title, description, time, whether it is current, pending, the last and before the line, and `item.mark()`, `item.rail()`, `item.content()`, `item.opposite()`, `item.action()`, `item.root(children)`, `item.render()`), `view.more` (a `TimelineMore` with `more.mark()`, `more.button()`, `more.render()`), `view.horizontal`, `view.grouped` and `view.root(items)`.

## Bar List

Named values as bars of one scale: the slowest tests of a run, the largest folders, the pages visited most.

```rust
use lurq::ui::{BarList, BarListItem};

BarList::new([("text_pipeline", 9.2), ("canvas_mesh", 7.0), ("layout_flex", 4.7), ("theme_roles", 2.5)])
  .title("Slowest tests")
  .description("Seconds");

// Rows that are pressed, told which by name: one stop of Tab, the arrows between them.
BarList::new([BarListItem::new("target/", 27.4), BarListItem::new("node_modules/", 3.1)])
  .title("Largest folders")
  .value_text(|gigabytes| format!("{gigabytes:.1} GB"))
  .on_select(|folder| open(folder));
```

| Builder | |
| --- | --- |
| `BarList::new(items)` | The rows in the app's order: `BarListItem::new(name, value)` or `(name, value)`; `item(..)` and `items(..)` add more |
| `title(&str)`, `description(&str)` | Over the rows: what they are, and the line under it ("Seconds") |
| `label(&str)` | What the list is called to tooling, where it has no title or another name is wanted |
| `max(impl Into<f64>)` | The value a bar fills its track at. Not given, the largest value rounded up to 1, 2, 2.5 or 5 of a power of ten |
| `color(impl Into<BackgroundColor>)` | The color of the bars, a role of the palette or a color as a chart's series takes it: `Chart1` (`series_color(0)`) unless given; `BarListItem::color` for one row's |
| `value_text(Fn(f64) -> String)` | Writes a row's value as the app does. Not given, with as many decimals as the most precise value has, up to 2 |
| `empty_text(&str)` | What a list with no rows says, "No data" when not given |
| `framed(bool)` | The board's card, as it is unless told otherwise; off, the rows stand on what holds them |
| `on_select(Fn(&str))` | Told the key of the row pressed (its name, unless it has a `key`), after the row's own handlers: every row that is not disabled is then pressed |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the first row that is pressed |
| `BarListItem::on_select(Fn())`, `disabled(bool)`, `key(..)`, `id(..)`, `color(..)` | A row's own handler, called before the list's; a row that takes nothing; what the list's `on_select` is told in its place |

- The bars share one scale, from 0 to `max`: 9.2 of a scale to 10 fills 92% of its track. A value under 0 draws no bar, and one over the scale fills its track; the value is written as it is either way.
- A name longer than its column of 92 is cut with an ellipsis; the track gives way, the value keeps its room.
- A row is as high as the theme's [density](../theme/#density) makes a row of 26 (18, 34 and 38 at compact, comfortable and touch); its bar keeps its 14.
- It is read and not operated, but where a row is pressed the pressed rows are one stop of Tab, a [focus group](../focus-navigation/#focus-groups) that is `one_stop`, with `typeahead`: Up and Down move between them, Home and End to the ends, a letter to the next name it starts, and a disabled row is passed by.
- To what reads the tree it is a `figure` called by its label or its title and described by its description, holding a `list` whose `listitem`s are called by their names and say their value as `a11y_valuetext`; inside a row that is pressed is a `button`.
- Parts: `BarListPart::Root`, `Header`, `Title`, `Description`, `Rows`, `Row`, `Name`, `Track`, `Bar`, `Value`, `Empty`. States: `BarListState::Framed` (of the list), and of a row `Pressable`, `Disabled`.
- In the design the list is a card (`Card` with an edge of 1 in `Border`, the extra large radius and the small shadow), 20 in, its header and its rows 16 apart. The title is 15 at 600 in `CardForeground` over the description, 12 in `MutedForeground`, 3 under it. A row is its name, the mono type at 11 in `CardForeground` in a column of 92, a track of 26 with a bar of 14 in `Chart1` and the small radius, and its value, the mono type at 11 in `MutedForeground`, 10 apart. The board draws nothing else of it: a row that is pressed has the `StateHover` and `StatePressed` veils and a ring of 2 in `Ring` inside it, and is half opaque where it is disabled.
- A theme's renderer is given `BarListView`: `view.rows` (each a `BarListRow` with its name, value, value text, the share of its track and its color, and `row.name()`, `row.track()`, `row.value()`, `row.root(children)`, `row.render()`), `view.max`, `view.header()`, `view.rows(rows)` and `view.root(content)`.

## Sparkline

A series as a line with no axes, small enough to stand beside a number: the frames of the last minute, the builds of the week. It is drawn on the canvas, behind the `canvas` feature.

```rust
use lurq::{app::theme::PaletteColor, ui::Sparkline};

Sparkline::new([118.0, 124.0, 121.0, 133.0, 129.0, 141.0, 138.0, 144.0])
  .color(PaletteColor::Success)
  .label("Frames per second, the last minute");
```

| Builder | |
| --- | --- |
| `Sparkline::new(values)` | The values, `impl Into<f64>`, the first at the start |
| `min(..)`, `max(..)` | The values at the bottom and at the top: the least and the greatest of the values unless given |
| `color(impl Into<BackgroundColor>)` | The color of the line, a role of the palette or a color as a chart's series takes it: `Chart1` (`series_color(0)`) unless given, or a tone where the series is good or bad (the board's frames are `Success`) |
| `width(..)`, `height(..)` | Its size, `impl Into<Length>`: the board's 80 by 28 unless given |
| `label(&str)` | What it shows, to tooling |
| `summary_text(Fn(&[f64]) -> String)` | What its values are, to tooling. Not given, "8 values from 118 to 144, the last 144" |
| `empty_text(&str)` | What a sparkline of no values says to tooling, "No data" when not given |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- Its values run from the first at the start to the last at the end, evenly apart, the least at the bottom and the greatest at the top, as a line of straight steps with round joins and ends, kept half its width in from every edge.
- Values on both sides of 0 have a line at 0 under them; values all the same make a line across the middle; one value is a dot in the middle; none draw nothing. A value that is not a number is left out.
- It is painted with [`Canvas::paint`](../canvas/#a-drawing-that-follows-from-a-key): sharp at every scale, and painted again when its values, its size, its scale or the theme change.
- To what reads the tree it is an `img` called by its label and described by its summary, and its canvas holds a `point` [item](../canvas/#describing-what-you-drew) for each value with the value as its text, which a reader can go through.
- Parts: `SparklinePart::Root` (the canvas: its width and height), `Line`, `Baseline`, `Point`, each drawn in its part's background and as wide as its part's width.
- In the design the line is 80 by 28 of straight steps, 1.75 wide with round joins and ends. The board draws nothing else of it: the line at 0 is 1 in `Border`, and a single value a dot of 4 in `Chart1`.
- A theme's renderer is given `SparklineView`: its values, their range, its label, its summary, and `view.render()`.

## Stat Card

One number with what it is and how it moved: frames per second, the time a build takes, the runs that failed.

```rust
use lurq::{
  app::theme::PaletteColor,
  ui::{Sparkline, StatCard, StatCardSize, StatCardVariant},
};

StatCard::new("Frames per second", "144")
  .delta("+12.5%")
  .variant(StatCardVariant::Success)
  .description("vs 0.59");

// With a sparkline beside its number, in the small size the board draws it in.
StatCard::new("Frames per second", "144")
  .size(StatCardSize::Sm)
  .media(Sparkline::new(frames).color(PaletteColor::Success));
```

| Builder | |
| --- | --- |
| `StatCard::new(label, value)` | What the number is, and the number as the app writes it |
| `delta(&str)` | How it moved, as the app writes it ("+12.5%"): a [badge](#badge) in the variant's tone |
| `variant(impl Into<Name>)` | A `StatCardVariant` (`Neutral`, `Info`, `Success`, `Warning`, `Destructive`), or a variant a theme adds: whether the change is for the better or the worse |
| `description(&str)` | What the change is measured against: "vs 0.59" |
| `media(impl Into<Element>)` | What stands at the end of the number's row: a [sparkline](#sparkline) of its last values |
| `size(impl Into<Name>)` | A `StatCardSize`: `Md`, the board's stat card, or `Sm`, the card the board draws around its sparkline |
| `on_click(impl IntoMouseEventHandler)` | The card is pressed and a stop of Tab; a second handler is called after the first |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them |

- Its number, its change and what it is measured against are the app's words. A label or a value longer than the card wraps.
- To what reads the tree it is a `group` called by its label and described by its value, its change and what it is measured against ("144, +12.5% vs 0.59"); a card that is pressed is a `button`.
- Parts: `StatCardPart::Root`, `Label`, `Body` (the value and the media), `Value`, `Media`, `Footer` (the change and the description), `Description`. States: `StatCardState::Pressable`, `Media`. Sizes: `StatCardSize::Sm`, `Md`.
- In the design the card is `Card` with an edge of 1 in `Border`, the extra large radius and the small shadow, 200 on the board and as wide as what holds it here, 20 in, its label, its value and its change 8 apart. The label is 13 in `MutedForeground`, the value 28 at 600 in `CardForeground`, 0.5 closer, the change a badge in its tone 8 from what it is measured against, the mono type at 11 in `MutedForeground`. The small size is the card around the board's sparkline: 16 in, the label 12, the value 24, 0.4 closer, the media at the end of the value's row, at its foot. A card that is pressed has the `Ring` edge and the medium shadow under the pointer, and a ring of 2 in `Ring` around it from the keyboard.
- A theme's renderer is given `StatCardView`: its label, value, change, description and tone, `view.label()`, `view.body()`, `view.footer()`, `view.root(content)` and `view.render()`.

## Media

A video player, an audio player and the parts they are built of. Given a source (`source(&Source)`, with the
feature `media`), a player plays it with the system's decoder and output and its controls move it; without one,
every value it shows is the app's, bound to its signals or held by it, and what a person does is told back. A
player, a queue, a seek bar and a volume control keep a drag, a measure of their room or a menu between renders:
each is a component, mounted with `.mount(ctx)` or declared where an element goes (`.child(player)`).

## Video Player

A picture the app gives, the controls over it, and a layer of the app's own over the picture: an ordinary player, or only the picture with the app's own controls, as in a vertical feed. Nothing is decoded here: the app draws its frame as an element and holds every value of the player; the player tells it what a person asks for.

```rust
use lurq::ui::{VideoPlayer, VideoPlayerSize};

// Bound to the app's signals: a press sets the signal, and then the handlers are told.
VideoPlayer::new()
  .media(frame)
  .duration(250.0)
  .buffered(140.0)
  .playing(playing.clone())
  .position(position.clone())
  .volume(volume.clone())
  .muted(muted.clone())
  .preview(move |at| thumbnails.frame_at(at))
  .on_position_change_end(move |to| decoder.seek(to))
  .mount(ctx);

// Held by the app: a press changes nothing until the app answers by setting the prop.
VideoPlayer::new()
  .media(frame)
  .duration(250.0)
  .playing_controlled(is_playing, move |play| decoder.set_playing(play))
  .position_controlled(at, move |to| decoder.seek(to));

// The picture alone, with the app's own controls over it.
VideoPlayer::new().media(frame).controls(false).child(my_controls).mount(ctx);
```

| Builder | |
| --- | --- |
| `new()` | A player of nothing yet: a black picture, paused at the start, of no known length |
| `media(element)` | What is drawn as the picture, filling the player |
| `poster(element)` | Drawn over the picture before the first play (not playing, at the start, not waiting, ended or failed), with the length in a chip at its bottom end and no bar. With a [source](#playing-a-video) it also stays until the video's first frame is there, whether or not it was told to play already. Nothing of the picture is drawn under it: a poster that does not cover the box shows the box beside it |
| `child(element)`, `children(..)` | The app's own layer over the whole picture, under the middle and the bar, with the controls and without them |
| `child_with(Fn(&VideoPlayback) -> element)` | A layer of the app's built from what the player is doing each time it is rendered: `position`, `duration`, `buffered`, `playing`, `waiting`, `ended`, `failed`, `opening`, `poster_shown`, `volume`, `muted`, `rate`, `progress()`, and `seek(seconds)`, `set_playing(bool)`, `set_muted(bool)` to move it. How an app's own controls show and set the time of a player with a source: see [your own controls](#your-own-controls-over-a-video) |
| `background(color)` | What is behind the picture, where the picture does not cover the box: beside a video of another shape, and before its first frame. Black unless told; a clear color shows what the app has under the player. The same as `style(VideoPlayerPart::Root, |s| s.background(..))` |
| `controls(bool)` | The bar and the middle of the picture; on by default. Without them the player is its picture and the app's layer |
| `playing(Signal<bool>)`, `playing_controlled(bool, handler)`, `on_playing_change(handler)` | Whether it plays. Play, the disc in the middle, a press of the picture, Space and K tell what it is to be |
| `position(Signal<f32>)`, `position_controlled(f64, handler)`, `on_position_change(handler)`, `on_position_change_end(handler)` | Where it is, in seconds. `on_position_change` is told as the seek bar's handle moves, `on_position_change_end` where it is let go; a key that seeks tells both |
| `duration(f64)`, `buffered(f64)`, `chapters(&[f64])` | How long it is (until said, the time reads `-:-- / -:--`), how far it is loaded, and where its chapters start. With a [source](#playing-a-video) the player knows how far unless told (`MediaState::buffered`): all of a file, of bytes and of a sound loaded whole; of an address the app's loader reads as it plays (Windows), the end of the ranges it holds in a row from where it plays, by the share of its bytes, which grows as ranges come and is no further than the position after a seek to a part not fetched; and nothing is drawn as loaded where it is not known (before it opened, and an address the system's own player reads: macOS, iOS, a browser). It was the whole length from the moment it opened |
| `preview(Fn(f64) -> Element)` | The frame for a time, which the mounted seek bar shows in a frame of 128 by 72 over the time under the pointer |
| `volume(Signal<f32>)`, `volume_controlled(f64, handler)`, `on_volume_change(handler)`, `on_volume_change_end(handler)` | How loud, from 0 to 1 |
| `muted(Signal<bool>)`, `muted_controlled(bool, handler)`, `on_muted_change(handler)` | Whether the sound is off; the volume is kept |
| `rate(Signal<f32>)`, `rate_controlled(f64, handler)`, `on_rate_change(handler)`, `rates(&[f32])` | How fast it plays, and the speeds offered (`MEDIA_RATES` unless told) |
| `captions(Signal<bool>)`, `captions_controlled(bool, handler)`, `on_captions_change(handler)` | Whether the captions show |
| `caption_text(&str)`, `captions_track(&str)` | The line to show now, and the name of the track the settings say ("English") |
| `captions_button(bool)`, `settings_button(bool)` | Whether the captions' button (with their row in the menus and the key C) and the settings' button (with its menu) are drawn: they are unless told. The captions' button is drawn only where the player has captions, whatever this says: a track (`caption_cues`, `captions_track`), a line (`caption_text`), captions the app holds (`captions`, `on_captions_change`) or that are on. The settings' button only where its menu has something in it: qualities, a speed (not live), captions. Without the settings the large form keeps its speed's button; the compact form has none, and its "more" menu stays |
| `looped(bool)` | Its [source](#playing-a-video) goes on from its start at its end, as a clip in a feed does: in what it has open, with nothing opened or read again. `on_ended` is told each time round, `playing` stays true and the player is not drawn as ended. Off unless told; a player without a source does nothing with it |
| `qualities(&[&str])`, `quality(Signal<Option<usize>>)`, `quality_controlled(..)`, `on_quality_change(handler)` | The qualities offered after "Auto", and the one it plays in: `None` for the one the player picks |
| `fullscreen(Signal<bool>)`, `fullscreen_controlled(..)`, `on_fullscreen_change(handler)` | Whether it fills the screen; its button then exits. The app makes it so |
| `picture_in_picture(Signal<bool>)`, `picture_in_picture_controlled(..)`, `on_picture_in_picture_change(handler)` | Whether it plays in a small window of its own |
| `waiting(bool)` | It waits for what it plays: a spinner in the middle, and the bar stays |
| `ended(bool)` | It played to the end: the veil, replay in the middle and in the bar, the seek bar full. Replay tells `playing` true: the app plays again from the start |
| `live(bool)` | No end: the LIVE badge for the time, the seek bar full (or the window that can be gone back in, given as the duration), no speed |
| `error(&str)`, `error_description(&str)`, `on_retry(handler)` | That it cannot play: the veil, the title and the line in the middle, and no bar. The button ("Try again") is drawn once `on_retry` is given |
| `on_load(Fn(&MediaInfo))`, `on_error(Fn(&MediaError))`, `on_started(Fn())`, `on_ended(Fn())`, `on_waiting_change(Fn(bool))` | What its [source](#playing-a-video) tells: it opened (its length, its picture's size, whether it has sound), it cannot play (a kind, which the locale says under the error's title), it really started to play (the first frame after it was told to is shown), it played to its end, it began or stopped waiting. A player without a source is told none of them |
| `size(VideoPlayerSize)` | `Lg`, `Md` (compact) or `Sm` (minimal) in place of the form its width gives it |
| `width(Length)`, `height(Length)` | Its size, in place of its form's (640 by 360 for the large one). A width with no height gives the height its aspect makes |
| `caption_cues(&MediaCaptions)` | A track of captions: while they are on, the player shows what the track says at its position. `caption_text` given too is shown in its place |
| `fullscreen_fills(VideoFullscreen)` | What a mounted player in fullscreen fills: its window as it is (`Window`, the default); its window taken to the whole screen (`Screen`); nothing, where the app lays it out itself (`App`) |
| `picture_in_picture_window(bool)` | Whether a mounted player in picture in picture opens its small window itself (the default). Without it the value is told and drawn, and the window is the app's |
| `in_picture_text(&str)`, `back_label(&str)` | What stands where the player was while it is in its small window, and the button that brings it back |
| `touch(bool)` | The touch form: larger controls over a scrim, shown and hidden by a tap; two taps on a side third seek ten seconds, and more add up. On its side while in fullscreen. Unless told, the touch form under a finger, `Pointer::Coarse` ([the touch form](#the-touch-form)), whatever its width and the density |
| `title(&str)`, `description(&str)` | What plays and a line under it, at the top of the touch form on its side |
| `on_back(impl Fn())` | The first button of the touch form's top row: drawn upright where a handler is given, always on its side, where it leaves fullscreen if there is none |
| `max_width(Length)` | The most it is wide: narrower where what holds it is, its aspect kept |
| `aspect(f32)` | The shape of the picture, its width over its height: `16.0 / 9.0` unless told |
| `force_menu(Option<VideoPlayerMenu>)`, `force_controls_shown(Option<bool>)` | A menu drawn open, or the bar shown or faded, with nothing pointing at it: for a gallery or a test |
| `label`, `play_label`, `pause_label`, `replay_label`, `mute_label`, `unmute_label`, `volume_label`, `seek_label`, `captions_label`, `speed_label`, `settings_label`, `quality_label`, `picture_in_picture_label`, `fullscreen_label`, `exit_fullscreen_label`, `more_label`, `retry_label`, `speed_heading_text`, `auto_text`, `on_text`, `off_text`, `live_text` | Every word it says and every name it gives a control |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the buttons of the bar as under the pointer or focused. The controls have ids after the player's: `{id}-play`, `{id}-seek`, `{id}-mute`, `{id}-time`, `{id}-captions`, `{id}-speed`, `{id}-settings`, `{id}-pip`, `{id}-fullscreen`, `{id}-more`, `{id}-center`, `{id}-retry`, `{id}-controls`, `{id}-caption`, `{id}-overlay`, and the menus' panels `{id}-settings-menu`, `{id}-speed-menu`, `{id}-more-menu` |
| `mount(ctx)` | Keeps its width, the timer that fades the bar, its menus and their pages, the seek bar's preview and its tooltips. `.child(player)` is the same component, declared: what renders the tree mounts it |
| `build(theme)` | A still picture for a test or a screenshot: the bar shown, the large form unless told or given a width in pixels, its menus not opened, no tooltips and no preview |

- A press on a control tells the app what the value is to be and changes nothing by itself: bound, the signal is set first; held, the app answers with the next render. No handler is called when the app sets a value, nor for a choice of what is already chosen.
- **Forms.** The large form has the seek bar over a row of play, the volume (its slider folds into its button under 480), the time, then captions (where it has them), the speed ("1×"), settings, picture in picture and fullscreen. The compact form, under 400 wide, has play, the time, mute, a "more" menu of captions (where it has them), speed, quality and picture in picture, and fullscreen. The minimal form, under 280, is one row: play, the seek bar and the elapsed time. Mounted, the form follows the width it is laid out at from the first frame (`ElementRef::observe_rect_before_paint`); `size` says another. `VideoPlayerSize::for_width` gives the form of a width, and `VIDEO_PLAYER_COMPACT_BELOW`, `VIDEO_PLAYER_MINIMAL_BELOW` and `VIDEO_PLAYER_VOLUME_FOLDS_BELOW` are the widths.
- **Layers**, bottom to top: the picture (and the poster), the veil, the app's layer, the middle (the 64 overlay play button while paused, 48 in the compact form; a spinner while it waits; replay when it ended; the error), the captions' line, the poster's length, and the bar on its gradient. A press of the picture plays and pauses at once; a second press within `VIDEO_PLAYER_DOUBLE_PRESS` (500 ms) asks for fullscreen too. The app's layer and the middle let a press through where they hold nothing.
- **Hiding.** While it plays the bar fades out `VIDEO_PLAYER_HIDE_AFTER` (3 s) after the pointer rests, over the slow duration, and the pointer is hidden over the player (`CursorIcon::None`); both come back on a move of the pointer, on pause, while the keyboard's focus is in the player and while a menu is open, over the base duration. Paused, waiting, ended and failed keep them. The captions' line moves down to 12 over the bottom while the bar is hidden.
- **Keys**, while the focus is in the player: Space and K play and pause, Left and Right seek 5 s (`VIDEO_PLAYER_SEEK_STEP`), J and L 10 s, Home and End go to the start and the end, Up and Down move the volume by 0.05, M mutes, C turns the captions (where it has them), F asks for fullscreen. Space and the arrows are the controls' own while the keyboard's focus is on one of them. A press of the picture gives the player the focus.
- **Menus.** Settings holds Quality (when `qualities` are given), Speed (not live) and Captions (where it has them), each with its value at its end; choosing a row shows its choices in the menu's place under a row that goes back (the menu's pages), and choosing one closes the menu. The speed button opens `ui::MediaSpeedMenu`; the compact form's "more" holds captions, speed and quality (each with its page) and picture in picture. They hang over the seek bar, their end on the bar's end, and stay the theme's.
- **Seeking with a preview.** Mounted, the seek bar shows the time under the pointer and, where `preview` is given, the frame for it over the time (`Media/Seek Preview`).
- Everything over the picture is white on black at the board's alphas, the same in a light and a dark theme: the picture is not themed, so themed controls would lose their contrast on it. The literals are in the recipe. The menus, the tooltips and the live badge are the theme's.
- Every control has a tooltip with its key (mounted) and a name to what reads the tree; the toggles say `a11y_pressed`, the menu buttons `a11y_haspopup` and `a11y_expanded`. The player is a `group` called by its label.
- Parts: `VideoPlayerPart::Root`, `Media` (the picture's layer; its cursor is the pointer's while it is pressed to play), `Veil`, `Overlay`, `Center`, `Spinner`, `Error`, `ErrorIcon`, `Text`, `Title`, `Description`, `Retry`, `Captions`, `Caption`, `Chip`, `ChipValue`, `Controls` (its background is the color its gradient ends in), `Row`, `Control`, `ControlIcon`, `Speed`, `Time`, `Elapsed`, `Menus` (its gap is how far a menu stands over the row), `SettingsMenu`, `MoreMenu`, `MenuValue`. States: `VideoPlayerState::Playing`, `Waiting`, `Ended`, `Error`, `Live`, `Muted`, `Captions`, `Fullscreen`, `Poster`, `Bare`, `Hidden`, `Narrow`, and `On`, resolved for a button of the bar that is on, under the pointer too. Sizes: `VideoPlayerSize::Sm`, `Md`, `Lg`.
- In the design (`Video Player` `yKk3P`, `Compact` `cak7v`, `Minimal` `OUkUD`) the large player is 640 by 360 with the large radius. Its bar is 88 high, padded 24 at the top, 12 at the sides and 8 at the bottom, 4 between the seek bar and the row, on a gradient from nothing to `#000000B3`; the row is 36 high, 4 between buttons of 36 with white glyphs of 20, the middle radius, `#FFFFFF1F` under the pointer, `#FFFFFF33` while on, and a white ring of 2 from the keyboard. The time is mono 12 in the large bar and 11 in the compact one. The compact bar is 72 high, padded 20 8 6, a row of 28 with 2 between buttons of 28 and glyphs of 16; the minimal one is a row 48 high padded 16 8 4 4, 6 apart. The settings are 240 wide, the compact menu 200. The captions' line is 12 over the bar and 24 from the sides, 16/500 white with a line of 1.4 in a box of `#000000B3` padded 2 8. The poster's length is mono 11 in a box of `#000000B3` padded 2 6, 12 from the bottom and the end. The veil is `#00000099`. The error is a white glyph of 24, a title 14/500, a line 13 in `#FFFFFF99`, and a white button 32 high.
- A renderer that draws another player starts from the view: `view.root()` (the box with its behavior), `media()`, `veil()`, `overlay()`, `center()`, `captions()`, `chip()`, `bar()`, or `view.bar` for the controls made (`VideoPlayerBar`: the seek bar, play, the volume, the time, the buttons at the end and the menus).

## Audio Player

Sound without a picture, in four forms: a full card, one row of 48 for a list or a message, a voice message, and a
mini player docked to the bottom of a window. Every value is the app's; a press of a control tells the app what it
is to be and changes nothing by itself.

```rust
use lurq::ui::{AudioPlayer, AudioPlayerVariant, AudioRepeat};

// The card, bound to the app's signals: a press of play sets `playing`, a drag on the bar sets `position`.
AudioPlayer::new(position.clone(), 250.0)
  .title("Night Drive")
  .description("Aurora Lane · Low Tide")
  .media(artwork)
  .playing(playing.clone())
  .volume(volume.clone())
  .muted(muted.clone())
  .rate(rate.clone())
  .shuffle(shuffle.clone())
  .repeat(repeat.clone())
  .on_previous(|| previous())
  .on_next(|| next())
  .mount(ctx);

// A voice message: the app holds where it is and seeks where the waveform is let go.
AudioPlayer::controlled(17.0, 42.0, |_| {})
  .variant(AudioPlayerVariant::Voice)
  .peaks(&peaks)
  .unheard(!heard)
  .playing_controlled(false, move |play| player.set_playing(play))
  .on_change_end(move |to| player.seek(to))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<f32>, duration)` | Bound to the position, in seconds, of something `duration` seconds long. A drag on the bar sets the signal |
| `controlled(position, duration, on_change)` | The position held by the app, which is told where a person moves it |
| `variant(AudioPlayerVariant)` | `Full` (the card, the default), `Compact`, `Voice`, `Mini` |
| `on_change(Fn(f64))`, `on_change_end(Fn(f64))` | Where a person moves the position, as a drag moves it and where it is let go, a key's step is made or the mini player's line is pressed |
| `buffered(f64)` | How far it is loaded, in seconds. With a source the player knows it unless told, as a [video player](#video-player) does: all of a sound loaded whole or kept, the ranges fetched of one read as it plays, nothing where it is not known |
| `playing(Signal<bool>)`, `playing_controlled(bool, handler)`, `on_playing_change(Fn(bool))` | Whether it plays. A press of play, or Space with the focus on the bar or the volume, tells what it is to be |
| `volume(Signal<f32>)`, `volume_controlled(f64, handler)`, `on_volume_change(Fn(f64))` | The volume, from 0 to 1: the card and the mini player |
| `muted(Signal<bool>)`, `muted_controlled(bool, handler)`, `on_muted_change(Fn(bool))` | Whether it is muted; muting keeps the volume |
| `rate(Signal<f32>)`, `rate_controlled(f64, handler)`, `on_rate_change(Fn(f64))` | The speed: the card's speed button opens `MediaSpeedMenu`; a voice message's badge steps 1×, 1.5×, 2× (`VOICE_RATES`) while it plays |
| `shuffle(Signal<bool>)`, `shuffle_controlled(bool, handler)`, `on_shuffle_change(Fn(bool))` | Shuffle, in the card |
| `repeat(Signal<AudioRepeat>)`, `repeat_controlled(AudioRepeat, handler)`, `on_repeat_change(Fn(AudioRepeat))` | `Off`, `All`, `One`; a press steps it (`AudioRepeat::next`), and `One` has the `repeat-one` glyph |
| `source(&Source)` (feature `media`) | What it plays: a file, bytes, a resource of the app, an address. The player opens it as a sound and plays it; where it is, how long it is and whether it plays, loads or failed are then the sound's, and its controls move it. An address is loaded when it is first played, once, and kept (`media::sound_cache()`). See `media-playback.md` |
| `on_load(Fn(&MediaInfo))`, `on_error(Fn(&MediaError))`, `on_started(Fn())`, `on_ended(Fn())`, `on_waiting_change(Fn(bool))` | What its source tells, as a [video player's](#playing-a-video): it opened (its length; a sound has no picture's size), it cannot play (a kind, which the locale says in its line), it played to its end (each end of a track that repeats; where a queue goes on to the next), it began or stopped waiting. With a source `on_playing_change` hears the sound stop and start by itself too. A player without a source is told none of them |
| `loading(bool)` | Lines of skeleton for the texts, "-:--" for the times, an empty bar, a spinner in play, previous and next dimmed |
| `error(Option<&str>)`, `on_retry(Fn())`, `retry_text(&str)` | What went wrong: a line in the destructive color in place of the times (in place of the bar in a row, where play turns to a retry button; in place of the description in the mini player). "Try again" at its end in the card |
| `unheard(bool)` | A voice message not heard yet: a dot of 8 in the info color, and the whole waveform in the color of what is not played |
| `title(&str)`, `description(&str)`, `media(element)` | What plays, who it is by, and the artwork (64 in the card, 36 in the mini player); the `music` glyph on `Muted` where there is none |
| `peaks(&[f32])` | The loudness of each bar of a waveform, from 0 to 1: a voice message draws it, the compact row in place of the bar |
| `on_previous`, `on_next`, `on_more`, `on_queue`, `on_expand` | Commands; each call adds a handler. `more_anchor(ElementRef)` is the ref of the card's more button, for a menu of the app |
| `label(&str)` and `play_label`, `pause_label`, `previous_label`, `next_label`, `shuffle_label`, `repeat_label`, `more_label`, `speed_label`, `queue_label`, `expand_label`, `seek_label` | What the player and its controls are called |
| `mount(ctx)` | The tooltips of the buttons, the speed menu, the mini player's form from its own width, and the time over the mini player's line under the pointer. `.child(player)` is the same component, declared: what renders the tree mounts it |
| `build(theme)` | A still picture for a test or a screenshot: no tooltips, no speed menu, a mini player as wide as it is given |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the play button; `id` is the root's, and each control has it with `-play`, `-seek`, `-previous`, `-next`, `-shuffle`, `-repeat`, `-more`, `-speed`, `-volume`, `-queue`, `-expand`, `-retry`, `-rate`, `-line` after it |

- Built of the media parts: `PlayButton`, `SeekBar`, `Waveform`, `MediaTime`, `MediaVolume`, `MediaSpeedMenu`. Shuffle and repeat are `Toggle`s; previous, next, more, queue and expand ghost `Button`s; "Try again" a `Link`.
- Keys: Tab reaches each control; Space on play presses it, and with the focus on the bar or the volume plays and pauses; the arrows move the bar and the volume.
- The mini player measures itself before its first frame is painted: under 480 wide (`AUDIO_MINI_NARROW`) it keeps the track, play and next.
- Parts: `AudioPlayerPart::Root`, `Bar`, `Edge`, `Header`, `Media`, `MediaIcon`, `Text`, `Title`, `Description`, `Skeleton`, `More`, `Progress`, `Times`, `Elapsed`, `Total`, `Time`, `Transport`, `Shuffle`, `Repeat`, `Footer`, `Speed`, `Error`, `ErrorIcon`, `ErrorText`, `Retry`, `Rate`, `Dot`, `Track`, `Fill`, `Tools`. States: `AudioPlayerState::Playing`, `Loading`, `Error`, `Disabled`, `Media`, `Unheard`, `Shuffle`, `Narrow`, `Scrubbing`.
- In the design the card is 360 wide on `Card` edged in `Border`, `radius-xl`, padded 20 with 16 between rows; the artwork 64 with `radius-md`; the title 14/600 and who it is by 13 in `MutedForeground`; the times mono 11 6 under the bar; the transport 8 apart (shuffle 32, previous 36, play 48, next 36, repeat 32, icons 16); shuffle and repeat in `MutedForeground` off and on `Highlight` on. The compact row is 320 by 48, `radius-lg`, padded 8 and 12, 10 apart. The voice message is padded 8 12 8 8, `radius-xl`. The mini player is on `Background` with a line of `Border` on top, under a progress line of 2 (4 under the pointer) in `Primary` over `Secondary`, and a row of 54 padded 12.
- To what reads the tree the player is a `group` called by its label; the error line is an `alert`; the mini player's line a `progressbar`; repeat says its state in `a11y_value`.
- A renderer that draws another player starts from `AudioPlayerView`: the controls are built (`view.controls`), and `media()`, `text()`, `time()`, `error_line()` and `line()` are the default pieces.

## Audio Queue

The tracks a player plays one after another, the one it is on marked. The arrow keys move between the rows.

```rust
use lurq::ui::{AudioQueue, AudioQueueItem};

// Bound to the app's current track, which a press of a row sets.
AudioQueue::new(
  current.clone(),
  [
    AudioQueueItem::new("Night Drive").description("Aurora Lane").duration(250.0).media(artwork),
    AudioQueueItem::new("Paper Boats").description("Mira Sol").duration(232.0),
    AudioQueueItem::new("Glass Garden").description("The Fernweh").duration(301.0).disabled(true),
  ],
)
.playing(true)
.mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<Option<usize>>, items)` | The tracks, on the one whose index the signal holds, which a press of a row sets |
| `controlled(Option<usize>, items, on_change)` | The current track held by the app, which is told the index of a row pressed |
| `on_change(Fn(usize))` | The index of a row pressed, where it is another than the current one and can be played. A second handler is called after the first |
| `playing(bool)` | Whether the current track plays: the `equalizer` glyph stands for its number |
| `item(AudioQueueItem)`, `items(..)` | Adds tracks at the end |
| `label(&str)`, `more_label(&str)`, `playing_label(&str)` | What the queue ("Queue"), a row's more button ("More") and the equalizer ("Now playing") are called |
| `mount(ctx)` | Mounts it at once, with the tooltips of the rows' more buttons. `.child(queue)` is a component that has them too |
| `build(theme)` | A still picture for a test or a screenshot: the buttons have no tooltips |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the row Tab would stop on |

| `AudioQueueItem` | |
| --- | --- |
| `new(&str)` | A track by its title |
| `description(&str)`, `media(element)`, `duration(f64)` | Who it is by, the artwork (cut to 32; the `music` glyph on `Muted` where there is none), and its length in seconds |
| `disabled(bool)` | It cannot be played: half the opacity, a press does nothing, the arrows pass it by |
| `on_select(Fn())` | Called on a press of the row, after the queue's `on_change` |
| `on_more(Fn())`, `more_anchor(ElementRef)` | The row's more button, there once a handler is given, and the ref a menu of the app hangs from |
| `id`, `key` | The id of the row; its more button has it with `-more` after it |

- Tab enters the queue at the current row (or the first that can be played) and stops in it once; the arrows move between the rows; Enter and Space play one. The more buttons are stops of Tab of their own, after their row, and the arrows do not go to them.
- Parts: `AudioQueuePart::Root`, `Item`, `Lead`, `Number`, `LeadIcon`, `Media`, `MediaIcon`, `Text`, `Title`, `Description`, `Duration`, `More`. States: `AudioQueueState::Current`, `Playing`, `Unavailable`, `Media` (on a row's parts) and `Disabled`.
- In the design a row is 48 high (following the density), padded 8 with 12 between its parts, `radius-md`: the number 20 wide in mono 12 in `MutedForeground`, the artwork 32 with `radius-sm`, the title 13/500 over who it is by 12, the length in mono 11, a more button of 28 with an icon of 14. The current row is on `Highlight` with its title 600; while it plays the `equalizer` 16 stands for its number. Under the pointer the row takes the hover veil, `play` 14 stands for the number and the more button shows; the focus is a ring of 2 in `Ring` inside the row. An unavailable row is at half the opacity.
- To what reads the tree the queue is a `listbox` called by its label, each row an `option` called by its title and who it is by, `a11y_selected` on the current one.

## Seek Bar

Where a player is in what it plays, how much of it is loaded, and the handle that moves it; over the pointer, the time it is on.

```rust
use lurq::ui::{SeekBar, SeekBarSize};

// Bound to the app's position, in seconds, of a recording 250 seconds long.
SeekBar::new(position.clone(), 250.0)
  .buffered(loaded)
  .on_change_end(move |to| player.seek(to))
  .mount(ctx);

// Over a video, cut into chapters, with a frame of the moment under the pointer.
SeekBar::new(position.clone(), duration)
  .on_media(true)
  .chapters(&[0.0, 312.0, 840.0])
  .preview(move |time| thumbnail_at(time))
  .mount(ctx);

// The app holds the position, and is told where a person moves it.
SeekBar::controlled(at, 250.0, move |to| seek_to(to));
```

| Builder | |
| --- | --- |
| `new(Signal<f32>, duration)` | Bound to the position, in seconds, of something `duration` seconds long (0 where it is not known). A drag, a press on the track and the keys set the signal |
| `controlled(position, duration, on_change)` | The app holds the position; the bar shows it and tells `on_change` where a person moves it, and shows what the app answers |
| `buffered(seconds)` | How far it is loaded: drawn between what is played and the rest |
| `chapters(&[f64])` | Where the chapters start, in seconds: the track is cut there, 2 apart. The first starts at 0, given or not |
| `step(seconds)` | What an arrow key moves it: 5 seconds unless told. A drag is not snapped to it |
| `on_media(bool)` | It stands over a picture: white, the same in a light and a dark theme |
| `size(SeekBarSize)` | `Md` (16 high) or `Lg`, a phone's: 28 high with a thumb of 28, 16 over a picture |
| `preview(Fn(f64) -> Element)` | A picture of the moment under the pointer, given its time: shown in a frame of 128 by 72 over the time. Shown by a mounted bar |
| `on_change(Fn(f64))` | The position, in seconds, each time a person moves it. Bound, it is called after the signal is set. Each call adds |
| `on_change_end(Fn(f64))` | Where a drag is let go, a press is over or a key's step is made, where it moved. Each call adds |
| `on_hover(Fn(Option<f64>))` | The time under the pointer as it moves over a mounted bar, and `None` when it leaves. Each call adds |
| `empty(bool)` | Nothing is loaded: the track alone, with nothing played or loaded and no thumb, and it takes no input (no stop of Tab). A player draws it while it loads and after an error. It is not dimmed |
| `live(bool)` | A live stream with nothing to go back in: the bar full, the thumb at its end, and a press, a drag and the keys tell nothing (no stop of Tab). The whole may be of no length |
| `label(&str)` | What it is called, to what reads the tree: "Seek" unless told |
| `mount(ctx)` | Puts it where it goes keeping the time under the pointer and the width it is laid out at: it shows the chip with the time, and the preview over it. `.child(bar)` is the same component, declared: what renders the tree mounts it |
| `build(theme)` | A still picture for a test or a screenshot: with no chip, and it reads nothing of a pointer that does not press it |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of what is dragged and focused. A mounted bar forced `Hovered` shows its position in the chip |

- It is the engine's slider over a track the bar draws itself: the engine's slider takes the drag, the press on the track and the focus, and paints the thumb; the track, what is played, what is loaded and the chapters' gaps are the bar's. The track's ends are half the thumb wide, so what is played ends under the thumb's middle.
- The keys: Right and Up a step later, Left and Down a step earlier, Page Up and Page Down a tenth of the whole in whole steps, Home and End to the ends. A disabled bar is not a stop of Tab and takes nothing.
- Parts: `SeekBarPart::Root` (the bar), `Track`, `Chapter` (the room between two is its padding at the side they share), `Played`, `Buffered`, `Rest`, `Control` (the engine's slider), `Thumb`, `Hover` (what stands over the pointer), `Preview` (the frame), `Tip` (the chip with the time). States: `SeekBarState::Disabled`, `OnMedia`, `Chapters`, `Buffered`, `Preview`, `Empty`, `Live`. Sizes: `SeekBarSize::Md`, `Lg`.
- In the design (Media/Seek Bar) it is 16 high with a track of 4 whose ends are round by 2: played in `Primary`, loaded in `MutedForeground` at half its opacity, the rest in `Secondary`, and the Slider's thumb, 16, in `Background` with an edge of 1.5 in `Primary`. The ring of the focus is the thumb's. Disabled it is at half its opacity and draws nothing loaded. Over a picture (Media/Seek Bar/On Media) played and a solid thumb of 12 are `#FFFFFF`, loaded `#FFFFFF59`, the rest `#FFFFFF33`, and the ring of the focus is white: the picture is not themed. The time under the pointer (Media/Seek Preview) is in a tooltip's chip, the mono type at 12 on `Primary`, 4 above the bar and centred on the pointer; the preview's frame is 6 above the chip, with a radius md and an edge of 2 in white, and the two stand 6 above the bar.
- To what reads the tree it is a `slider` called by its label, with `a11y_value` reading "1:24 of 4:10".
- A renderer that draws another bar starts from `view.dressed(view.control())`, the behavior with the thumb's style; `view.track()` is the default track and `view.hover_layer()` what stands over the pointer.

## Media Time

The time a player reads out: how far it is, and how long the whole is.

```rust
use lurq::ui::{MediaTime, MediaTimeSize};

MediaTime::new(position, Some(duration));

// Over a video, turning between elapsed and remaining on a press.
MediaTime::new(position, Some(duration))
  .on_media(true)
  .remaining(left.get())
  .on_click(move |_| left.set(!left.get()));

// A compact player's; and a stream that is live.
MediaTime::new(position, Some(duration)).size(MediaTimeSize::Sm);
MediaTime::new(0.0, None).live(true);
```

| Builder | |
| --- | --- |
| `new(position, Option<f64>)` | The time at `position` seconds of a whole of that many seconds; `None` while the length is not known: it reads `-:-- / -:--` |
| `remaining(bool)` | It reads what is left, with a minus before it, alone |
| `on_media(bool)` | It stands over a picture: white, the "/" and the whole `#FFFFFF99`, the same in a light and a dark theme |
| `live(bool)`, `live_text(&str)` | A live stream: a badge, solid in `Destructive`, reads "LIVE" (or the text given) in place of the time |
| `size(MediaTimeSize)` | `Sm` (11, a compact player's and an audio player's), `Md` (12), `Lg` (13, a phone's player held across) |
| `on_click(handler)` | A press: a player turns the reading between elapsed and remaining. Given it, the time is a button, a stop of Tab with the ring of the focus, which Enter and Space press. Each call adds |
| `label(&str)` | What it is called, to what reads the tree: what it reads unless told ("1:24 of 4:10", "2:46 left") |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` shows only on a time that is pressed |

- From an hour up both readings have hours (`1:02:45 / 2:10:00`). `media_time_text(seconds, hours)` is a reading alone as a string, `media_rate_text(rate)` a speed as a menu names it, and `MEDIA_RATES` the speeds a player offers.
- Parts: `MediaTimePart::Root` (the row, or the button), `Elapsed`, `Divider`, `Total`, `Live` (what the recipe lays over the badge's root). States: `MediaTimeState::OnMedia`, `Remaining`, `Unknown`, `Live`, `Pressable`. Sizes: `MediaTimeSize::Sm`, `Md`, `Lg`.
- In the design (Media/Time) it is the mono type at 12, 4 between the readings, the elapsed time in `Foreground` and the "/" and the whole in `MutedForeground`; not loaded the elapsed `-:--` is muted too. The mono type keeps it from jumping as it counts.

## Waveform

The loudness of a recording as a row of bars, the played part of it marked: a voice message's seek bar.

```rust
use lurq::ui::Waveform;

// Shown and not operated, a third of it played.
Waveform::at(0.33, &peaks);

// Bound to how far it is played, from 0 to 1, which a press, a drag and the keys set.
Waveform::new(progress.clone(), &peaks).on_change_end(move |to| player.seek(to * length));

// The app holds it, and is told where a person presses or drags to.
Waveform::controlled(at, &peaks, move |to| seek_to(to));

// Not heard yet: every bar in the color of what is not played.
Waveform::at(0.0, &peaks).unheard(true);
```

| Builder | |
| --- | --- |
| `at(progress, &[f32])` | Shown and not operated: `progress` of it played, from 0 to 1; no stop of Tab, and a press does nothing |
| `new(Signal<f32>, &[f32])` | Bound to how far it is played, from 0 to 1, which a press, a drag and the keys set |
| `controlled(progress, &[f32], on_change)` | The app holds how far it is; the waveform shows it and tells `on_change` where a person presses or drags to |
| `unheard(bool)` | Nothing of it was heard yet: every bar in the color of what is not played |
| `step(f64)` | What an arrow key moves it, from 0 to 1: a twentieth unless told |
| `on_change(Fn(f64))` | Where a person moves it, from 0 to 1, as a press or a drag moves it. Bound, it is called after the signal is set. Each call adds |
| `on_change_end(Fn(f64))` | Where it ends: the drag let go, the press or a key's step made, where it moved. Each call adds |
| `label(&str)` | What it is called, to what reads the tree: "Seek" unless told |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of what is pressed where it is operated; `force` shows only on one that is operated |

- `peaks` are the heights of its bars, each from 0 to 1, one for each bar: about 36 for a voice message. How many are played is `progress` of them, rounded.
- Operated it is the engine's slider over the bars, as large as they are and drawing nothing but the ring of the focus: the arrow keys move it a step, Page Up and Page Down a tenth, Home and End to the ends. A disabled one is not a stop of Tab and takes nothing.
- Parts: `WaveformPart::Root`, `Bars` (the row), `Bar` (one not played), `Played`, `Control` (the engine's slider). States: `WaveformState::Disabled`, `Unheard`, `Operated`.
- In the design (Media/Waveform) the bars are 3 wide and 2 apart, fully round, at most 24 high and no lower than they are wide; the played ones in `Primary`, the rest in `Input`. Operated, the ring of the focus is around the bars at a radius sm; disabled, it is at half its opacity.
- To what reads the tree an operated waveform is a `slider` called by its label, with `a11y_value` the percent played.

## Play Button

The round control that starts and pauses what a player plays, and plays it again once it has ended.

```rust
use lurq::ui::{PlayButton, PlayButtonSize, PlayButtonVariant};

// playing: Signal<bool>. A press tells what it is to be.
PlayButton::new()
  .playing(playing.get())
  .on_change(move |next| playing.set(next));

// In the middle of a paused video, over the picture.
PlayButton::new()
  .variant(PlayButtonVariant::Overlay)
  .size(PlayButtonSize::Xl)
  .replay(ended.get())
  .on_change(move |next| play(next));
```

| Builder | |
| --- | --- |
| `playing(bool)` | Whether what it plays is playing: the glyph is `pause`. The button does not keep it: the app says so |
| `on_change(handler)` | Called on a press with what it is to be: playing if it was paused or had ended. Not while it is loading or disabled. A second handler is called after the first |
| `replay(bool)` | What it played has ended: the glyph is `rotate-left`, and a press tells `true` |
| `loading(bool)` | What it would play is still being fetched: the `spinner` glyph turns in the glyph's place, the disc is at 75%, and a press does nothing |
| `size(PlayButtonSize)` | `Sm` 32 with a glyph of 14, `Md` 40 with 16 (the default), `Lg` 48 with 20, `Xl` 64 with 28 |
| `variant(PlayButtonVariant)` | `Primary` on a surface of the theme; `Overlay` over a picture |
| `play_label(&str)`, `pause_label(&str)`, `replay_label(&str)` | What it is called while a press would play, pause or play again: "Play", "Pause", "Replay" |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the disc |

- A click, Enter and Space press it; Tab reaches it, and the ring of the focus is drawn from the keyboard.
- Parts: `PlayButtonPart::Root` (the disc, what is pressed), `Icon`. States: `PlayButtonState::Playing`, `Replay`, `Loading`, `Disabled`. Variants `PlayButtonVariant::Primary`, `Overlay`; sizes `PlayButtonSize::Sm`, `Md`, `Lg`, `Xl`.
- In the design the disc is round, in `Primary` with the glyph in `PrimaryForeground`, `PrimaryHover` under the pointer; held down it is `Primary` under the pressed veil, as the primary button is. Loading it is at 75%, disabled at half.
- Over a picture the disc is `#00000099` (black at 60%) with a white glyph and a white ring of the focus, the same in a light and a dark theme: the picture is not themed, and a themed control would lose its contrast on it. The board draws no overlay under the pointer: it is `#000000B3`, the end of the board's bar gradient. The board uses the overlay at `Xl` in a player and `Lg` in a compact one.
- To what reads the tree it is a `button` called by the label of what a press would do; loading it says `a11y_busy`.
- A renderer that draws another play button starts from `view.root()`, the behavior with the disc's style, and `view.icon()`, the glyph of the state it is in (`view.icon_name()`).

## Volume

A button that mutes and a slider of how loud, in a player's bar. Muting keeps the volume, so that the sound comes back as loud as it was.

```rust
use lurq::ui::{MediaVolume, MediaVolumeSize};

// volume: Signal<f32> from 0 to 1, muted: Signal<bool>.
MediaVolume::new(volume.clone()).muted(muted.clone());

// The app holds both, and is told what a person makes them.
MediaVolume::controlled(player.volume(), move |volume| player.set_volume(volume))
  .muted_controlled(player.muted(), move |muted| player.set_muted(muted));

// In a video's bar, over the picture; in a narrow one, the button alone until the pointer or the focus is in it.
MediaVolume::new(volume.clone()).muted(muted.clone()).on_media(true).size(MediaVolumeSize::Lg).length(64.0);
MediaVolume::new(volume.clone()).muted(muted.clone()).on_media(true).collapsed(true).mount(ctx);
```

| Builder | |
| --- | --- |
| `new(Signal<f32>)` | Bound to the app's volume, from 0 to 1, which a drag, a press on the slider and the keys set |
| `controlled(volume, on_change)` | The app holds the volume and is told what a person makes it |
| `muted(Signal<bool>)`, `muted_controlled(bool, handler)` | Muted while true, which a press of the button turns. Muted, the slider stands at nothing and the volume is kept |
| `on_change(handler)` | Called with the volume each time a person moves it. Bound, after the signal is set |
| `on_change_end(handler)` | Called with the volume when a drag is let go or a key's step is made |
| `on_muted_change(handler)` | Called with whether it is to be muted: on a press of the button, and with `false` when the slider is moved up from nothing while muted |
| `collapsed(bool)` | The button alone, the slider beside it while the pointer is over the control, the focus is in it or it is being dragged. What it keeps for that is `mount(ctx)`'s; a still picture (`build(theme)`) is the button alone |
| `on_media(bool)` | Over a picture: white, the same in a light and a dark theme |
| `size(MediaVolumeSize)` | `Md`: a button of 32 with a glyph of 16 (the default); `Lg`: 36 with 20, as a video's bar has it |
| `length(f32)` | How long the slider is, in place of the recipe's 72 |
| `label(&str)` | What the slider is called: "Volume" |
| `mute_label(&str)`, `unmute_label(&str)` | What the button is called while a press would mute or bring the sound back: "Mute", "Unmute" |
| `mount(ctx)` | Puts it in the tree of `ctx`'s component, keeping whether the pointer is over it, the focus is in it and its slider is being dragged. `.child(volume)` is the same component, declared: what renders the tree mounts it |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` and `force` are the button's |

- The glyph is `volume` from a half up, `volume-low` under it, and `volume-off` while muted or at nothing. Muted, the slider stands at nothing and the app's volume is not touched; moving the slider up from there sets the volume and tells `on_muted_change(false)` once in the gesture.
- Tab reaches the button, then the slider: Space and Enter press the button, the arrows move the slider a twentieth, Page Up and Page Down a tenth, Home and End to nothing and to the whole. A disabled control is passed by.
- Parts: `MediaVolumePart::Root` (the button and the slider, side by side), `Button` and `Icon` (laid over a [button](#button)'s `Root` and `Icon`), `Slider`, `Track`, `Fill`, `Thumb` (laid over a [slider](#slider)'s parts). States: `MediaVolumeState::Muted`, `Collapsed`, `OnMedia`, `Disabled`. Sizes `MediaVolumeSize::Md`, `Lg`.
- In the design the button is a ghost one of 32 with a glyph of 16 and the slider is 72 long, 4 apart. Over a picture the glyph is white, the button `#FFFFFF1F` under the pointer and `#FFFFFF33` held down with a white ring of the focus; the slider's range and thumb are white, the thumb 12 with no edge, and its rail is `#FFFFFF33`.
- To what reads the tree the button is a `button` called by what a press would do, and the slider a `slider` called by `label`.
- A renderer that draws another volume control starts from `view.button()` and `view.slider()`, the button and the slider dressed by the recipe with their behavior; `view.revealed` says whether the slider is shown.

## Speed Menu

The menu a player's speed is chosen from: the speeds under the heading "Playback speed", the one in use checked at the end of its row.

```rust
use lurq::{
  components::Placement,
  ui::{MediaSpeedMenu, media_rate_text},
};

// open: Signal<bool>, set by a press of the speed button; speed_button: ElementRef; speed: Signal<f32>.
MediaSpeedMenu::new(open.clone(), speed_button.clone(), speed.get())
  .on_change(move |rate| speed.set(rate as f32));

// From a bar at the bottom of a picture, opening up from the button's end, with the app's own speeds.
MediaSpeedMenu::new(open.clone(), speed_button.clone(), speed.get())
  .rates(&[0.25, 0.5, 1.0, 1.5, 2.0, 3.0])
  .placement(Placement::TopEnd);
```

| Builder | |
| --- | --- |
| `new(open, anchor, rate)` | Open while the signal is true, hung from the anchor, with `rate` checked |
| `on_change(handler)` | Called with the speed chosen, after the menu has closed, where it is another than the one in use. A second handler is called after the first |
| `on_dismiss(handler)`, `on_open_change(handler)` | As a [menu](#menu) has them: a person closed it other than by choosing; a person closed it, a choice too, after `on_dismiss` |
| `rates(&[f32])` | The speeds offered, in place of `MEDIA_RATES` (0.5×, 0.75×, Normal, 1.25×, 1.5×, 2×). Each is named by `media_rate_text` |
| `heading_text(&str)` | The words over the speeds: "Playback speed" |
| `label(&str)` | The menu's name to tooling: its heading unless told |
| `placement(Placement)`, `offset(..)`, `collision(..)`, `inline(bool)` | As a [menu](#menu) has them |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's; `force` draws the speed in use |

- It is a [menu](#menu): choosing a speed, Escape, a press outside and Tab close it, Up and Down move between the speeds, Enter chooses. Choosing the speed in use closes it and tells `on_change` nothing.
- Parts: `MediaSpeedMenuPart::Panel` and `Label` (laid over a menu's `Panel` and `Label`), `Item`, `ItemLabel`, `Check` (laid over a menu item's `Root`, `Label` and `Check`). State: `MediaSpeedMenuState::Inline`.
- In the design it is the menu's panel 168 wide (where a menu is at least 230), with the group label and a menu item for each speed; the check is at the end of the row of the speed in use. Over a picture it stays themed: it is the system's own surface, and opaque.
- A renderer that draws another speed menu starts from `view.menu()`, the menu with its speeds, or `view.item(rate)` for one speed.

## Playing a video

`VideoPlayer::source(&Source)` (feature `media`) makes a mounted player play what the source holds: a file, bytes, a resource of the app, an address.

```rust
VideoPlayer::new()
  .source(&Source::file("clips/lake.mp4"))
  .width(640.0)
  .max_width(Length::Fill)
  .mount(ctx)
```

An address is read as it plays, and the source's headers are on each of its requests: a host that serves only its own pages is given its `Referer`, one that serves only who signed in its `Authorization`.

```rust
VideoPlayer::new()
  .source(&Source::url("https://videos.example.com/lake.mp4").header("Referer", "https://example.com/"))
  .mount(ctx)
```

On Windows the app's loader reads every address as it plays (the feature `http`; a build without it has the system read it, with no header): seeking asks the server for the part it goes to, the player waits where it has nothing left to show while the part it needs is still on the network (not while it reads ahead), and a server that refuses on the way is the error it shows (`MediaError::Server(403)`). The loader's rule gives headers to every address of a host (`app.loader().headers(..)`). On macOS and iOS the system's player is given them. A browser plays no video yet, and its `<audio>` cannot send them (see [Loading](../resources-media/#loading)).

The player opens the source with the app's loader, decodes it with the system's decoder and draws its frames as its picture. Where it is, how long it is, and whether it plays, waits, ended or failed are the media's, in place of what the app would say. A press on a control tells the app's handlers and does what it says to the media: play, pause, the seek bar, the speed, the keys. An app that holds `playing` or `rate` (a signal, or a value it sets) moves the media by changing them. With no source the player is drawn from the values it is given, as before.

What happens to the media is told to the app, as a browser's `<video>` tells it:

```rust
VideoPlayer::new()
  .source(&Source::file("clips/lake.mp4"))
  .on_load(|info| println!("{:?} long, {:?}, sound: {}", info.duration, info.size, info.sound))
  .on_error(|why| tracing::warn!("{why:?}: {}", why.detail().unwrap_or_default()))
  .on_started(move || poster.set(false))
  .on_ended(move || next.set(true))
  .on_playing_change(move |plays| playing.set(plays))
  .on_waiting_change(move |waits| buffering.set(waits))
  .mount(ctx)
```

- Each is told on the UI thread, once each time it happens, in the order the media lives them: it opened, it began or stopped waiting, it began or stopped playing, it started, it ended, it failed. A handler may set signals and move the media; what it sets is drawn in the same frame.
- `on_started` is told when the video really plays: it was told to play, it does not wait for what it plays, and the first frame since is on the screen (for a sound, its sound is given to the output). `on_load` comes before it, with nothing moving yet, and so does `on_playing_change(true)`, which a press tells at once: between them and `on_started` the player shows the frame it stood on, or nothing. An app that covers the player with a poster of its own takes it away here, and measures from a swipe to here how long a video took to move. It is told again after a pause, after it waited and after its end; not for a round of a `looped` one, which did not stop, nor for a seek while it plays.
- `on_ended` is told again only after it played again. `on_error` is told a kind (`core::MediaError`), whose words are the locale's (`MediaWords::error`), once until the person tries again, which opens the source anew. Waiting is reading what is not there yet once it opened (an address on Apple's systems); opening is not waiting.
- `on_playing_change` hears a press, as without a source, and the media stopping or starting by itself: at its end, when it fails, a sound that starts over. What a press told or the app set is not told again when the media does it. A signal bound to `playing` follows the media at its end, and that is not the app pausing it.
- A player without a source tells none of these: whether it waits, ended or failed is what the app gives it.
- **How far it is loaded** is the media's too: `VideoPlayback::buffered` in a layer of the app's (`child_with`), and the lighter part of the player's own seek bar. It is how far the video is fetched from where it plays, and `None` where that is not known: a line of the app's draws nothing as loaded then (`SeekBar::buffered(playback.buffered.unwrap_or(0.0))`). A looped video near its end is fetched to its end; gone on from its start, it is fetched as far as what is held from there, which for a long one may be its first megabyte alone.

A clip that plays round and round, as in a feed, is looped:

```rust
VideoPlayer::new()
  .source(&Source::url("https://videos.example.com/clip.mp4"))
  .looped(true)
  .on_ended(move || rounds.update(|count| *count += 1))
  .mount(ctx)
```

At its end the media goes to its start in what it has open: nothing is opened again, an address read through the app's loader is asked for nothing it holds (its first megabyte is always held, and the sixteen blocks read last), and it does not stop. `on_ended` is told each time round; `playing` stays true, and `on_playing_change` is told nothing. Setting `playing` again in `on_ended` did this with a stop at the seam. `Media::set_looped(true)` is the same for an app's own player, and `AudioPlayer::looped(true)` for a sound.

### Your own controls over a video

A player with `controls(false)` is its picture and the app's layer. With a source, where it is and how long it is are the media's, and no signal of the app's follows them: a layer that shows the time is built from the player itself, with `child_with`. It is given a `VideoPlayback` each time the player is rendered, which is as often as the player's own bar would be drawn again, and its `seek`, `set_playing` and `set_muted` do what the player's own controls do: the media is moved, and the app's handlers are told.

```rust
use lurq::ui::{MediaTime, PlayButton, PlayButtonVariant, SeekBar, VideoPlayer};

VideoPlayer::new()
  .source(&Source::url("https://videos.example.com/clip.mp4"))
  .poster(still)
  .looped(true)
  .controls(false)
  .child_with(move |playback| {
    let (to, play) = (playback.clone(), playback.clone());
    Stack::new()
      .child(
        PlayButton::new()
          .variant(PlayButtonVariant::Overlay)
          .playing(playback.playing)
          .on_change(move |plays| play.set_playing(plays)),
      )
      .child(MediaTime::new(playback.position, playback.duration).on_media(true))
      .child(
        SeekBar::controlled(playback.position, playback.duration.unwrap_or(0.0), move |at| to.seek(at))
          .on_media(true),
      )
  })
  .mount(ctx)
```

- The layer is built in the player's own render: it reads no signal for the time, and nothing but the player is rendered again as the video plays. What it captures is moved into it (elements built once outside are cloned in it).
- `playback.opening` is true until the video's first frame is there, and `playback.waiting` while it reads what is not there yet: a spinner of the app's own shows on either.
- A poster stays over the box until the first frame is there, also where the player plays at once, so a feed shows the still of its next video and not a black box. Without a poster the box shows its `background`, black unless told.

### A video opened ahead

A feed wants the video after the one shown to start at once. A player that is given its source and not told to play is that: it opens the source, shows its first frame (under its poster, where it has one) and stays paused at its start, and it plays at once when `playing` is set.

- From an address it then reads what opening asks for and nothing ahead of it. On Windows that is one request, for the first 256 KiB, which is what Media Foundation reads of a file at a time, and the 16 KiB about an index kept at the file's end where it has one there. A neighbour opened ahead does not take the connection from the video that plays.
- Told to play, it reads ahead of what plays from then on (the two blocks after the one read, see [Loading](../resources-media/#loading)), and what follows its start is asked for at once.
- `Media::open` with no `play` is the same for an app's own player; `Media::has_frame()` says when its first frame is there, and `Media::is_open()` when it opened.

An app that draws its own player asks for a media itself:

```rust
let media = Media::open(app.loader(), &Source::file("clips/lake.mp4"));
media.play();
let picture = media.frame().map(Video::new); // drawn again by itself as frames come
let state = media.state(); // duration, position, buffered, size, playing, waiting, ended, error, rate
media.seek(42.0);
media.set_rate(1.5);
media.on_change(|| { /* it changed: called on the media's own thread, with nothing said */ });
// Its events, on the UI thread, once each time: as a player's.
media.on_load(|info| println!("{:?} by {:?}", info.duration, info.size));
media.on_error(|why| tracing::warn!("{why}"));
media.on_started(|| println!("it plays"));
media.on_ended(|| println!("the end"));
media.on_playing_change(|plays| println!("playing: {plays}"));
media.on_waiting_change(|waits| println!("waiting: {waits}"));
```

See [Media events](../resources-media/#media-events) for how they reach the UI thread.

## Playing a sound

`AudioPlayer::source(&Source)` (feature `media`) makes a player play what the source holds: a file, bytes, a resource of the app, an address.

```rust
AudioPlayer::new(position, 0.0)
  .source(&Source::url("https://example.com/night-drive.mp3"))
  .title("Night Drive")
  .volume(volume)
```

The player opens the source as a sound and plays it through the system's output. Where it is, how long it is, and whether it plays, loads or failed are the sound's, in place of what the app would say: the length given to `new` is drawn only until it opened. A press on a control tells the app's handlers and does what it says to the sound: play, pause, the seek bar, the volume, the mute, the speed. An app that holds `playing`, `volume`, `muted` or `rate` moves the sound by changing them. With `AudioRepeat::One` the track starts over at its end. "Try again" loads the source anew.

Its events are a video player's (`on_load`, `on_error`, `on_ended`, `on_playing_change`, `on_waiting_change`): an app that plays a queue goes on to the next track in `on_ended`. A `ui::AudioQueue` is the list the app shows and plays nothing itself; the player that plays its current item tells each end.

A sound at an address is loaded whole through the app's loader (its headers, its timeout, what it allows; the feature `http`), and kept: played again, by this player or another, it starts at once and asks the network for nothing. While it loads the player is drawn loading. One larger than the loader reads whole (`max_bytes`) is read as it plays, with its headers as a video's are, and not kept. What is kept is what the server sent, up to 64 MiB of sounds; over that, what was played longest ago is let go.

```rust
sound_cache().budget(256 * 1024 * 1024); // keep more
sound_cache().forget(&source);           // this one has changed: load it again
sound_cache().clear();
```

That is kept in memory, for the run of the app. An app that gives a folder has what it loads from addresses kept there as files, for its next run too:

```rust
// The folder is the app's own cache folder: lurq picks none.
let app = App::new().enable_media_disk_cache(cache_folder);
app.loader().media_disk_cache().budget(2 * 1024 * 1024 * 1024); // 512 MiB unless told
```

A sound then plays from its file and is not kept in memory as well, and a player of a sound an earlier run kept opens it as it is drawn. `sound_cache()` goes on answering whether a sound is kept and how much is, and `forget` and `clear` delete the files too; its `budget` is of the memory alone. Over the folder's budget the file used longest ago is deleted. What is kept is not asked of the server again, however old it is: an app that knows a sound changed forgets it. See [Loading](../resources-media/#loading).

An app that draws its own player asks for the sound itself:

```rust
let song = Media::sound(app.loader(), &Source::file("songs/night-drive.mp3"));
song.set_volume(0.8);
song.play();
song.set_muted(true);
let state = song.state(); // duration, position, buffered, sound, volume, muted, playing, waiting, ended, error, rate
```

A video has its sound too: `Media::open` and `VideoPlayer::source` play it, and the picture goes by what is heard.

## Picture

An image in its box: it covers the box unless told otherwise, shows a skeleton while it loads and its name when it
cannot be shown, and carries a badge, a bar and a layer of the app's over it.

```rust
use lurq::ui::{Picture, PictureFit, PicturePlaceholder};

// Pixels the app has, a file, bytes, a resource of the app, an address.
Picture::from_file("photos/lake.jpg")
  .label("The lake at dusk")
  .size_px(640.0, 360.0)
  .lightbox(true);

// As wide as its room, 3 by 2 before its pixels are there, with a small version blurred while they come.
Picture::from_url("https://example.com/lake.jpg")
  .label("The lake at dusk")
  .aspect_ratio(3.0 / 2.0)
  .placeholder(PicturePlaceholder::blur("photos/lake-small.jpg"))
  .on_error(|why| tracing::warn!("{why:?}: {}", why.detail().unwrap_or_default()));

// A figure: the picture with a caption under it.
Picture::from_resource("images/lake.jpg")
  .width(320.0)
  .caption("Figure 1. The lake at dusk.");
```

| Builder | |
| --- | --- |
| `new(ImageData)`, `from_file(path)`, `from_bytes(bytes)`, `from_resource(path)`, `from_url(url)`, `from_source(source)`, `empty()` | Pixels the app has; a file or bytes, decoded off the UI thread; a resource of the app or an address, read by the app's `Loader`; any `resources::Source`; or no source: a box that says it has no picture. `ImageData` converts into a picture. An address outside a browser needs the feature `http`, decoding the feature `image`: without it the box says the picture did not load, and why |
| `cache_key(u64)`, `downscale_to(width, height)` | The key the decoded pixels are kept under, for bytes the app tells apart itself; and the most pixels that are kept of a large picture |
| `label(&str)`, `decorative(bool)` | What the picture is called: what it shows of itself where there are no pixels, and its name to tooling. A decorative one says nothing to tooling |
| `width(Length)`, `height(Length)`, `size_px(width, height)`, `aspect_ratio(f32)` | As a browser's `img`: with neither side it is as large as its pixels, with one the other follows their ratio, with both it is a box. A ratio gives the side that is not said, and with neither makes the box as wide as its room; it holds the box of a picture whose pixels are not there yet. The width is the most the box takes: in a narrower room it is as wide as the room, and a box of pixels keeps its shape |
| `fit(PictureFit)`, `anchor(StackAlignment)` | `Cover` (the default), `Contain`, `Fill`, `None` (its own size); and which side is kept where the pixels are cut or do not fill the box |
| `shape(PictureShape)`, `bare(bool)` | `Rounded` or `Circle`; bare is the pixels alone, with no fill, no corners and no line |
| `size(PictureSize)` | The form: `Lg` from 240 wide, `Md` under it, `Sm` at 96 and under, `Xs` at 40 and under. Not said, it follows the width the box gets: the width in pixels it was told, or less where its room is narrower, or the width of its room |
| `placeholder(PicturePlaceholder)` | What the box shows while the pixels are on their way: `Skeleton` (the sheen), `blur(source)` or `blur_pixels(data)` (a small version of the picture, blurred), `None` (the box alone) |
| `loading(bool)`, `progress(Option<f64>)`, `progress_text(Fn(PictureLoad) -> String)` | The box as while its pixels are on their way, for a picture the app is still fetching; how far the load has come, of 1, which draws a rail in place of the sheen; and the words under the rail. A picture that loads from an address shows its own progress where the server says how long it is |
| `on_load(Fn())`, `on_error(Fn(&PictureError))`, `on_retry(Fn())` | Told once the pixels are there; once the picture did not come, with why; and on a press of "Try again", before the picture asks again. Why is a kind: `NotFound`, `Denied`, `TimedOut`, `TooLarge`, `Server(status)`, `Network`, `NotAPicture` (bytes that are no picture the decoder reads), `Unavailable` (a build without `image`, an address without `http`) and `Other`; the locale says it (`MediaWords::picture_error`), and what the system or the decoder said is kept for a log (`PictureError::detail`) |
| `error_description(&str)`, `retry_label(&str)`, `download_label(&str)`, `more_label(&str)`, `open_label(&str)` | Its words: "The picture didn't load.", "Try again", "Download", "More", "Open" |
| `title(&str)`, `description(&str)`, `badge(element)` | What the bar along the bottom says, and what stands at the top, at the start. The bar shows under the pointer and with the focus |
| `on_download(Fn())`, `menu_item(MenuItem)`, `menu_items(..)`, `action(element)`, `actions(..)` | The bar's Download button, the menu behind More, and buttons of the app's |
| `bar(bool)`, `force_bar_shown(Option<bool>)` | Whether the bar is drawn, and whether it shows whatever the pointer does |
| `child(element)`, `children(..)` | The app's own layer over the whole picture |
| `caption(&str)` | A caption under the picture, which makes it a figure |
| `on_click(handler)` | A press of the picture as a whole |
| `selectable(bool)`, `selected(bool)`, `on_change(Fn(bool))` | A picture that can be chosen, whether it is, and what a press would make it. The picture does not keep it |
| `lightbox(bool)`, `open(Signal<bool>)`, `on_open_change(Fn(bool))` | A press, and the last button of a large picture's bar, open the picture alone in a [lightbox](#lightbox) over the window, with its title and description; whether it is open, to read and to set; and what tells of it. Without a signal the picture keeps it. A picture that is chosen by a press is opened by the button |
| `variant(PictureVariant)` | `Desktop`, or `Phone` (the board's `Mobile/Picture`, `T9drb`): a large picture runs edge to edge with no corners and no line, its badge 16 in, its bar always there with no buttons and a title of 15, and a long press brings its actions, Download and the menu's items, in an [action sheet](#action-sheet). A picture is the phone's under a finger, `Pointer::Coarse` ([the touch form](#the-touch-form)), whatever the window's width and the density, unless told |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box |

- A picture is a component of its own wherever it is put: `Row::new().child(Picture::from_url(..))`. It is mounted
  by the component whose render returns it, keeps its load, its menu and its lightbox there, and is told apart from
  its like by its place or its `key`. In a tree no component renders (`Tree::set_root(element)`) it shows nothing.
  `mount(ctx)` mounts it at once, for a place that needs the element.
- A picture made of the same plain things as the last time is not rendered again with what holds it: a list of five
  hundred costs what its changed pictures cost. A handler, a child, a badge or a menu makes it render with its
  parent.
- While a box that does not wait for its pixels loads, what stands in for them goes out over them as they come;
  a box the pixels size drops it at once.
- A press, Enter and Space press a picture that takes a press; Tab reaches it, and the ring of the focus is drawn
  from the keyboard. The buttons of the bar are pressed on their own, and the picture is not.
- Parts: `PicturePart::Figure`, `Root`, `Image`, `Edge`, `StandIn`, `Sheen`, `Veil`, `Overlay`, `Center`, `Mark`,
  `Label`, `ErrorIcon`, `ErrorLabel`, `ErrorDescription`, `Retry`, `Progress`, `Track`, `Fill`, `ProgressLabel`,
  `ProgressLine`, `Count`, `Top`, `Select`, `SelectIcon`, `Controls`, `Text`, `Title`, `Description`, `Actions`,
  `Control`, `ControlIcon`, `MoreMenu`, `Caption`. States: `PictureState::Bare`, `Circle`, `Pressable`,
  `Selectable`, `Selected`, `Disabled`, `Shown` (the bar is held shown), `On` (a button of the bar whose menu is
  open). Sizes
  `PictureSize::Xs`, `Sm`, `Md`, `Lg`.
- In the design the box is `Muted` with a line of `Border` inside it, round by `RadiusSize::Lg` (`Md` for a small
  one); the bar is white on a gradient to black at 70% in both themes, since the picture is not themed. A chosen
  picture has a disc of `Primary` with a check at its top end and a line of `Primary` about it.
- To what reads the tree it is an `img` called by its label, a `button` where it is pressed, a `checkbox` where it
  is chosen, and a `figure` about the two where it has a caption.
- A renderer that draws another picture has `view.frame()` (the box with everything in it), and the pieces it is
  made of: `view.image()`, `view.empty()`, `view.overlay()`, `view.top()`, `view.controls()`, `view.edge()`.

## Lightbox

Pictures one at a time, whole, over the window: to look at closely, to bring closer, to turn through.

```rust
use lurq::ui::{Lightbox, Picture};

// open: Signal<bool>. Open while it says so; Escape, a press beside the picture and Close set it to false.
Lightbox::new(open.clone(), photos.iter().map(|photo| Picture::from_file(&photo.path).title(&photo.name)))
  .index(shown.clone())
  .wrap(true)
  .on_download(move |which| save(which));

// One picture needs none of this: Picture::lightbox(true). A grid of them: Gallery.
```

| Builder | |
| --- | --- |
| `new(open, pictures)`, `picture(Picture)`, `pictures(..)` | Open while the signal says so, with the pictures it shows. Each is shown whole (`Contain`), with its `title` and `description` over its lower edge; what else a picture has over it (a bar, a badge, a mark) is not shown |
| `index(Signal<usize>)`, `index_controlled(index, on_change)`, `on_index_change(Fn(usize))` | Which picture is shown, counted from 0. Without either the lightbox keeps it and starts at the first |
| `zoom(Signal<f32>)`, `on_zoom_change(Fn(f64))`, `zoom_max(f32)` | How close the picture is, 1 where it fits; and the closest it is brought, 4 unless set. Turning to another picture fits it again |
| `variant(LightboxVariant)`, `on_share(Fn(usize))`, `menu_item(MenuItem)`, `menu_items(..)`, `share_label`, `more_label` | `Desktop`, or `Phone`: the phone's viewer, which a lightbox is under a finger, `Pointer::Coarse` ([the touch form](#the-touch-form)), over a window of any width and in a page, whatever the density, unless told. In a narrow window of a desktop it is the desktop's, laid out in the room it has. Open, it keeps the form it opened in until it closes (in a page, the one it was first shown in); the app setting the pointer changes it at once. On a phone, the Share button with the picture shown, and the menu behind More, which holds Download first where it is given |
| `preload(usize)` | How many pictures each way of the one shown are brought on once its own pixels came, read by the app's loader and decoded, so that an arrow shows the next at once: 1 unless told, 0 for none, round the ends where it wraps |
| `wrap(bool)` | Whether the first picture comes after the last |
| `filmstrip(bool)`, `counter(bool)`, `counter_text(Fn(usize, usize) -> String)` | The strip of small pictures and the counter: both show with more than one picture unless told. The counter says "3 / 12" |
| `on_download(Fn(usize))` | Shows the Download button, and is called with the picture shown |
| `action(element)`, `actions(..)` | Buttons of the app's in the top row, before Download |
| `inline(bool)`, `width(Length)`, `height(Length)` | Stands in the page where it is declared, open or not, in place of over the window: for a gallery, a screenshot, a test, a viewer that is part of a page. Its size there; the width is the most it takes |
| `on_dismiss(Fn())`, `dismiss_on_escape(bool)`, `dismiss_on_veil(bool)`, `close_button(bool)` | Told when a person closes it; and whether Escape, a press beside the picture and a Close button do |
| `label`, `close_label`, `previous_label`, `next_label`, `zoom_in_label`, `zoom_out_label`, `fit_label`, `download_label` | Its words to tooling: "Pictures", "Close", "Previous picture", "Next picture", "Zoom in", "Zoom out", "Fit to the window", "Download" |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the panel; its parts are `<id>-close`, `-next`, `-previous`, `-zoom-in`, `-zoom-out`, `-level`, `-download`, `-stage` |

- It is a component wherever it is put, as a [picture](#picture) is: it keeps which picture is shown, how close, and
  where the view is.
- The wheel brings the picture closer about the pointer, turned away from the hand, and further toward it, and a drag
  moves it; two fingers on a touch screen pinch and pan. On a touchpad two fingers move the picture and their pinch
  brings it closer (on Windows, as Ctrl and the wheel do); a Mac trackpad's smart zoom is a double tap. A double
  click goes to twice the size it fits and back.
- Keys, while the focus is in it: the left and right arrows turn to the picture before and after, and move a
  picture that is closer than it fits, as the up and down arrows do; `+` and `-` bring it closer and further, `0`
  fits it; Escape closes. Over the window it takes the focus.
- The level says how close the picture is as a share of the size it fits at (100% fitted), and a press of it fits
  the picture again. In a panel too narrow for the whole top row the level is not there.
- The strip shows as many small pictures as it has room for, about the one shown.
- Parts: `LightboxPart::Veil` (what covers the window), `Root` (the panel), `Top`, `Counter`, `Total`, `Control`,
  `ControlIcon`, `Level`, `Stage`, `Step`, `StepIcon`, `Side`, `Caption`, `Title`, `Description`, `Filmstrip`,
  `Thumb`. States: `LightboxState::Inline`, `Current` (the small picture of the one shown).
- In the design it is white on near black (`#09090BF2`) in both themes, since a picture is not themed: a top row of
  52 with the counter in the mono face and buttons of 36, round buttons of 36 on black at 40% over the sides of
  the stage, a caption on a gradient to black at 70%, and a strip of 52 with pictures of 44 by 32, the one shown
  whole and ringed in white, the others at 60%.
- To what reads the tree its buttons are `button`s called by their labels.
- A renderer that draws another lightbox has `view.top()`, `view.stage()`, `view.caption()`, `view.filmstrip()`,
  `view.panel()` (the three with the keys) and `view.dismisser()`.

## Gallery

Pictures in rows, each in a room of one shape, with one [lightbox](#lightbox) for them all; a gallery that chooses
keeps which of them are chosen.

```rust
use lurq::ui::{Gallery, Picture};

// A press of a picture opens it in a lightbox that holds them all.
Gallery::new(photos.iter().map(|photo| Picture::from_file(&photo.path).label(&photo.name)))
  .columns(4)
  .max_shown(8);

// chosen: Signal<Vec<PhotoId>>. A press chooses, Enter opens.
Gallery::keyed(photos.iter().map(|photo| (photo.id, Picture::from_file(&photo.path))))
  .selectable(true)
  .selected(chosen.clone());
```

| Builder | |
| --- | --- |
| `new(pictures)`, `picture(Picture)`, `pictures(..)` | The pictures, told apart by their place |
| `keyed(items)`, `item(key, Picture)` | The pictures, each with a key of the app's (`GalleryKey`: `Clone + PartialEq + Send + Sync`): what is chosen is a list of those keys, which stays right when the pictures are put in another order |
| `columns(usize)`, `min_picture_width(f32)`, `gap(f32)`, `aspect_ratio(f32)` | The most pictures in a row (4); the least width of a picture (92, the board's thumbnail), which takes columns away as the room narrows, down to one, and which 0 turns off; the room between them across and down (8); and the width of a picture's room to its height (92 to 68). The room is measured before paint: the first frame has the rows of the settled one. A row the pictures do not fill keeps its empty rooms: its pictures are as wide as the others |
| `width(Length)` | How wide the gallery is: all the room there is unless told, and never wider than its room. The pictures share it |
| `size(PictureSize)` | The form of its pictures: `Sm` unless told |
| `max_shown(usize)`, `more_text(Fn(usize) -> String)` | The most pictures it shows: with more, the last one shown lies under a veil that says how many more there are, itself among them ("+5"). A press of it opens that picture in the lightbox, which holds every picture |
| `selectable(bool)`, `selected(Signal<Vec<K>>)`, `selected_controlled(keys, on_change)`, `on_selection_change(Fn(&[K]))` | Pictures that can be chosen, and the keys of the ones that are. Without a signal the gallery keeps them |
| `lightbox(bool)`, `open(Signal<Option<usize>>)`, `on_open_change(Fn(Option<usize>))` | Whether a picture opens in the lightbox (it does unless told), and which one is open, `None` while it is closed |
| `on_select(Fn(usize))` | A press of a picture, with its place, in a gallery that neither chooses nor opens |
| `label(&str)`, `disabled(bool)` | What it is called to tooling; and a gallery that takes nothing, its pictures at half the opacity |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the gallery; its pictures are `<id>-0`, `<id>-1`, … |

- It is a component wherever it is put, as a [picture](#picture) is: it keeps which picture is open, and what is
  chosen where the app gives no signal.
- Each picture is the app's, without what lies over it (its bar, its badge, its own press and its own lightbox):
  the gallery gives it its press, its mark and its place in the lightbox. Its title and description are what the
  lightbox says under it.
- In a gallery that chooses, a press chooses a picture and takes it out again, and Enter opens the one with the
  focus; the picture that stands for the rest opens and is not chosen.
- Tab reaches the gallery once, and the arrows move the focus among its pictures, across and down.
- Parts: `GalleryPart::Root`, `Row`, `Cell`. States: `GalleryState::Selectable`, `Disabled`. What a picture looks
  like in it is the picture's recipe: `.rules(..)` on the pictures the app gives.
- To what reads the tree it is a `list` of pictures, or a `listbox` that takes several where it chooses.
- A renderer that draws another gallery has `view.render()` and the parts' styles; the rows are of `flex(1)` rooms
  with the ratio, so a picture is as wide as its share.

## Carousel

Slides one after another in a room that shows one, or a few, at a time: turned by the round buttons beside them,
the dots under them, the arrow keys, a swipe or a drag.

```rust
use std::time::Duration;
use lurq::ui::{Carousel, Picture};

// Pictures as wide as the room, the first after the last.
Carousel::new(photos.iter().map(|photo| Picture::from_file(&photo.path).aspect_ratio(16.0 / 9.0)))
  .wrap(true)
  .label("Photos");

// Three cards in view, turning by itself every five seconds.
Carousel::new(cards)
  .per_view(3)
  .autoplay(Duration::from_secs(5))
  .index(shown.clone());
```

| Builder | |
| --- | --- |
| `new(slides)`, `slide(element)`, `slides(..)` | The slides: any elements, each laid out as wide as its room |
| `index(Signal<usize>)`, `index_controlled(index, on_change)`, `on_index_change(Fn(usize))` | Which slide it is at, counted from 0: the first one shown. Without either the carousel keeps it and starts at the first |
| `per_view(usize)`, `gap(f32)` | How many slides the room shows at once, each as wide as its share (1); and the room between two (12). A turn moves by one, and the last few slides are one place |
| `wrap(bool)` | Whether the first slide comes after the last, and the last before the first |
| `arrows(bool)`, `dots(bool)` | The round buttons beside the slides and the dots under them: both show unless told. With one place there are no dots |
| `autoplay(Duration)` | Turns to the next slide by itself, from the last to the first. It waits while the pointer is over it and while the focus is in it from the keyboard, and stops for good once a person turns it |
| `width(Length)` | How wide it is, its buttons with it: all the room there is unless told, and never wider than its room. The slides share what the buttons leave |
| `label(&str)`, `previous_label(&str)`, `next_label(&str)`, `slide_label(Fn(usize, usize) -> String)` | Its words to tooling: "Carousel", "Previous slide", "Next slide", "Slide 2 of 4" |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the carousel; its parts are `<id>-previous`, `-next`, `-viewport`, `-slide-<n>`, `-dot-<n>` |

- It is a component wherever it is put, as a [picture](#picture) is: it keeps which slide it is at.
- A press of a round button turns by one, a press of a dot goes to its place; a turn takes the slides there over a
  few frames, at once with motion reduced. A finger swipes a page at a time, as many slides as are shown. A drag
  of the mouse carries the slides, and let go a fifth of a slide away it goes on to the next. A trackpad scrolls
  them sideways and they come to rest on a slide.
- Keys: Tab reaches a round button, and the left and right arrow keys on it turn to the slide before and after;
  Tab reaches the dots once, the arrow keys move among them and Enter or Space goes to the one with the focus. A
  button that has nothing more to turn to takes nothing, and the focus it had goes to the other one. The slides
  that are not shown take neither the focus nor the pointer.
- Parts: `CarouselPart::Root`, `Track`, `Viewport`, `Slide`, `Arrow` (the round buttons, which are design-system
  [buttons](#button), `Outline`, round), `Dots`, `Dot`, `DotMark`. States: `CarouselState::Current` (the dot of the
  place it is at), `Disabled`.
- In the design the buttons are outline buttons of 36 with arrows of 16, 12 from the slides; 12 under the slides
  the dots are 6 across, 6 apart, in `Input`, the current one in `Primary`. A dot is pressed in a room 3 larger on
  every side.
- To what reads the tree it is a `region` described as a carousel, each slide a `group` described as a slide and
  called by its label, the dots a `tablist` of `tab`s.
- A renderer that draws another carousel has `view.previous()`, `view.next()`, `view.viewport()` (the slides, with
  what turns them) and `view.dots()`.

## QR Code

A text an app gives, drawn as a code a phone's camera reads: an address to open, a code to pair a device, a ticket.

```rust
use lurq::ui::{QrCode, QrLevel};

QrCode::new("https://lurq.dev/pair?code=7F3K-92QD").label("Pair your phone");
QrCode::new("TICKET-2026-11-16-0042").level(QrLevel::Quartile).length(220.0).quiet_zone(4);
QrCode::new(address).media(logo);
```

| Builder | |
| --- | --- |
| `QrCode::new(text)` | The code of the text's UTF-8 bytes, in the smallest version (1 to 40) that holds them |
| `level(QrLevel)` | How much can be lost and the code still read: `Low` (7%), `Medium` (15%, the default), `Quartile` (25%), `High` (30%) |
| `length(length)` | The side of the card, its padding included: 160 unless the app or a recipe says another |
| `quiet_zone(modules)` | Modules of light around the code inside the card's padding: none unless the app says |
| `media(element)` | A logo over the middle, in a well of the light fill; the code is made at `High` |
| `label(&str)` | What tooling calls it: "QR code" unless the app says what it is for |
| `locale(&Locale)` | The words it speaks in, what tooling calls it, in place of the app's (`App::set_locale(Locale::for_tag("uk"))`, English where it set none), whose words are `Locale::display`. A `label` told by itself stays |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- It is dark on light in the light look and in the dark one alike, as the design draws it: a phone reads a code dark on light, and not every phone reads it the other way. Its fills are the roles of its recipe read in the theme's light palette (`Theme::palette_color_for(Appearance::Light, role)`): the card `Background`, the modules `Foreground`. Its edge is the window's `Border`.
- The modules are as large as fit in whole logical pixels, and the code stands in the middle of the room inside the card's padding. The card is drawn with `.snap_to_pixels(true)`, so every module's edge is on a whole device pixel: at 100% and 200% every module is as wide as the next, at 150% a module of 5 is 7 or 8 device pixels, and two modules side by side meet with no seam. A run of dark modules, and the same run in the rows under it, is one box.
- The encoder is lurq's own, byte mode at every version and level, with the mask that scores best: no dependency, in every build and in the browser. A higher level is taken where it fits in the same version. A text too long for the level asked for is drawn at the highest level that holds it; one no level holds (over 2953 bytes) draws the card empty. `QrMatrix::encode(text, level)` gives the modules themselves, to print or to draw another way.
- The card's padding of 10 is light, and is the quiet zone a phone needs where the modules are small; a code of large modules, or one that is printed, takes `quiet_zone(2)` to `quiet_zone(4)`.
- A logo's well is a fifth of the code's side, an odd number of modules across so that it sits on the middle module, or the logo's size with the well's padding where the logo is larger: a logo larger than a fifth of the side may leave a code no phone reads.
- To tooling it is an image (`img`) called by its label, whose description is the text, also where no code holds the text.
- Parts: `QrCodePart::Root` (the card), `Module` (a dark module), `Media` (the logo's well). State: `QrCodeState::Media`.
- In the design the card has an edge of 1 inside it in `Border`, `radius-lg` corners and 10 of padding, white, with modules of near black. The board draws eleven modules of 7, a picture of a code: a code of a real text has 21 or more, so the card is 160 by default. The board draws no logo, no loading and no expired code.

## Watermark

A text, or a picture, repeated across what it covers, turned and faint, over the content: who a shared screen is shown to, a draft, a confidential report.

```rust
use lurq::ui::Watermark;

Watermark::new("lurq internal").child(card);
Watermark::new("ivan@lurq.dev · 16 Nov 2026").child(page);
Watermark::new("Draft").angle(-35.0).gap(16.0, 24.0).child(page);
Watermark::new("lurq").media(logo).child(report);
```

| Builder | |
| --- | --- |
| `Watermark::new(text)` | The marks of a text |
| `child(element)`, `children(elements)` | What it covers; the watermark is as large as that |
| `media(element)` | A picture repeated in place of the text, turned and faint as the text would be |
| `gap(x, y)` | The room between one mark's box and the next, across and down, before they are turned: 27 and 29 |
| `angle(degrees)` | Clockwise where more than nothing: -20, rising to the right |
| `offset(x, y)` | Where the first mark's box is from the top left: -20 and 18 |
| `stagger(x)` | How much further along every other row is: 40 |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- The marks take nothing from the content: their layer is hit by no pointer, so the content is pressed, selected and scrolled through them, and it says `a11y_hidden`, so tooling reads the content and not the marks (see [the MCP page](../mcp/)). The layer's id is the watermark's with `-layer` after it.
- The mark is in the tree once; the engine paints it again across the layer ([`.repeat(Repeat)`](../layout/#repeat)), only where it can be seen. Over a page of any length in a scroll a watermark costs the marks in view, scrolling builds nothing again, and what is laid out again is the mark when its text or look changes, and the layer when the size of what it covers does.
- A mark's glyphs are drawn at its angle (`TextTransformMode::Rasterized`), so it is sharp at every scale. Its color is `Foreground` at 0.12: dark on the light look, light on the dark one.
- Parts: `WatermarkPart::Root` (what holds the content and the layer), `Layer`, `Mark` (its type, color and faintness). State: `WatermarkState::Media`. The board's numbers are `WATERMARK_GAP`, `WATERMARK_ANGLE`, `WATERMARK_OFFSET` and `WATERMARK_STAGGER`.
- In the design the marks are "lurq internal" in the sans of 12, medium, in `foreground` at 0.12, turned 20 degrees counter-clockwise, 96 apart along a row and 44 from row to row, every other row 40 further along, over a card of 220 by 120 with an edge and `radius-lg` corners. The card is the content it covers, not the watermark's: a watermark draws no box of its own.

## Link Preview

What a link says about itself, shown before it is followed: the site with its icon, the title, a line of the description and the picture, as one link. A card, a row for lists and messages, and a link of 24 that stands in a sentence.

```rust
use lurq::ui::{LinkPreview, LinkPreviewVariant};

// What the page says, fetched.
LinkPreview::new("https://lurq.dev/docs/layout");
// The app's own title over the page's; the rest is fetched.
LinkPreview::new(url).title("Layout, the guide");
// Everything given: nothing is fetched.
LinkPreview::new(url).title("Layout in lurq").description("Rows, columns and gaps.").site("lurq.dev").no_image().no_icon();
// A row, and a link in a sentence.
LinkPreview::new(url).variant(LinkPreviewVariant::Compact);
LinkPreview::new(url).variant(LinkPreviewVariant::Inline);
// In a message being written: a close drops it.
LinkPreview::new(url).on_remove(move || draft.drop_preview());
```

| Builder | |
| --- | --- |
| `LinkPreview::new(url)` | The address. What the page says about itself is fetched and shown, the skeleton while it is read |
| `title(&str)`, `description(&str)`, `site(&str)` | Shown in place of the page's, one by one; what is not given is still fetched. The site is the domain of the address (no `www.`) where neither names one |
| `image(..)`, `icon(..)` | The picture and the site's icon in place of the page's: an address, a `resources::Source`, or a `ui::Picture` (`LinkImage`) |
| `no_image()`, `no_icon()`, `no_site()` | None of it, whatever the page has: the text alone, the link mark, the title alone |
| `video(seconds)` | A video, its length in seconds where it is known: the play mark and the length over its picture. A page that gives `og:video` or a kind of `video.*` is one by itself |
| `variant(LinkPreviewVariant)` | `Card` (360, the default), `Compact` (a row of 432), `Inline` (24 high) |
| `fetcher(impl LinkFetcher)` | What fetches the page for this preview, in place of the app's (`resources::set_link_fetcher`) and of lurq's own: any `Fn(&str) -> LinkFetch`. It is asked off the UI thread and may block; what it holds (`LinkFetcher::held`) is shown in the first frame. `shared_fetcher(Arc<dyn LinkFetcher>)` shares one |
| `on_open(Fn(&str))` | Told the address on a click, Enter or Space, in place of opening it in the browser |
| `on_remove(Fn())` | Gives it a close button, a stop of Tab of its own after the link: the app takes the preview out of what it shows |
| `return_focus(&ElementRef)` | Where the focus goes once the close is pressed: the draft's input. Without one it goes on to what follows the preview in the order of Tab, or to what comes before it where nothing follows ([`FocusHandle::focus_onward`](../ctx/#moving-the-focus-from-a-handler)), never to nothing |
| `width(length)` | A width of its own in place of the form's; never wider than its room |
| `loading(bool)` | The skeleton, whatever is known: an app that reads the page itself and has nothing to give yet |
| `label(&str)`, `no_preview_text(&str)`, `remove_label(&str)`, `locale(&Locale)` | What tooling calls it in place of its title; "No preview for this page." and "Remove preview", which are `Locale::display`'s unless told |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the link's; the close button is `{id}-remove` |

- **It works from the address alone.** As it renders it asks its fetcher for the page's `resources::LinkMeta` and is drawn again when the fetch comes further. When every field the form shows is given (the title, the description and the site, the picture and the icon given or said to be none; an inline one shows no description and no picture) the fetcher is not asked. lurq's own fetcher reads the start of the page through the app's loader and keeps what it found in memory by the address ([link previews](../resources-media/#link-previews)); it needs the feature `http`. The gallery gives its previews a fetcher of its own and fetches nothing unless told.
- **A page that tells nothing** (no title, no description, no picture), one that could not be read, and every page in a build without `http` or in a browser, is its address (without the scheme), the link mark, and a line under it: "No preview for this page.", or what went wrong in the locale's words (`DisplayWords::link_error`: "The page was not found.", "The site answered 503."). What the app gave stands over it. A page read that names no icon has its site's `/favicon.ico` tried, through the same loader and kept by it as any load; a picture or an icon that does not load is left out, and the link mark stands for the icon.
- **It is one link.** A click, Enter and Space open the address with [`open_url`](../ctx/#opening-an-address), unless `on_open` is given; a disabled one is no stop of Tab and opens nothing. To tooling it is a `link` named by its title (or its label) and described by its site, its description and, for a video, "Video, 4:10". Under the pointer it lifts (a card to the medium shadow) on the hover's veil and its title is underlined; from the keyboard it has the ring.
- **Its text has a rule for its room**: in a card the title and the description each end in an ellipsis after two lines, in a row after one; an inline one's title ends in an ellipsis and its site keeps its room. Each form is as wide as the board draws it and never wider than its room; the skeleton's lines are no wider than it.
- **In a sentence**, an inline one is an object of a rich text ([elements in a line](../styling-events/#elements-in-a-line)): `Text::rich([.., RichTextSpan::object(body), ..]).object(LinkPreview::new(url).variant(LinkPreviewVariant::Inline))`. It is as wide as it is, wraps with the words, and the line keeps its height. A selection over it copies `[title](url)` (`LinkPreviewView::copy_text`), its address alone where the page told nothing.
- It is a component of its own, mounted where it is converted into an element (`mount(ctx)` mounts it at once): it keeps the fetch, whether the pointer is on it, and a picture that did not load. `build(theme)` is the still picture of what it was given, with nothing fetched, for a test.
- Parts: `LinkPreviewPart::Root` (the link), `Media` (the box of the picture), `Play`, `PlayIcon`, `Length`, `Text` (the column of the site, the title and the description), `Site`, `Icon`, `Domain`, `Title`, `Description`, `Skeleton`, `Close`, `CloseIcon`. States: `LinkPreviewState::Loading`, `Media`, `Video`, `Empty`, `Removable`, `Disabled`.
- In the design a card is 360 in `Card` with an edge of `Border`, `radius-xl` corners and the small shadow; its picture is 360 by 188 (`LINK_PREVIEW_PICTURE_RATIO`) with a line under it, its text padded 12, 16, 14 and 16 with 6 between: the site in 12 of `MutedForeground` after its icon of 16 (`radius-sm`), the title 14/500 at 1.4, the description 13 of `MutedForeground` at 1.5. A video has the play mark of 48 (a glyph of 20, white on black at 0.6) in the middle of its picture and its length in mono 11 on a dark tag 8 from the lower end; the close of a card in a composer is a disc of 28 on its picture, 8 from the top end. The compact row is 432, padded 12 with 12 between, the text (4 at its start, 4 between) beside a picture of 72 (`radius-md`); the inline link is 24 high with `radius-md` corners, padded 5 and 8 with 6 between, its title 13/500 in `Foreground` and its site 12 of `MutedForeground`. While the page is read the skeleton's lines stand for the text: 88 by 10, the width by 14, the width by 12 and 168 by 12 in a card, 88, 232 and 168 in a row, 10 apart; a square of 14 and a line of 96 in a sentence.

## Action Sheet

On a phone, the actions on something, risen from the bottom of the screen over a scrim, and Cancel under them.

```rust
use lurq::ui::{ActionSheet, MenuItem};

// open: Signal<bool>. An option does what it does and closes the sheet.
ActionSheet::new(open.clone(), [
  MenuItem::new("Save to library").on_select(save),
  MenuItem::new("Duplicate").on_select(duplicate),
  MenuItem::new("Delete photo").destructive(true).on_select(delete),
])
.title("Photo options");
```

| Builder | |
| --- | --- |
| `new(open, items)`, `item(MenuItem)` | Open while the signal says so, with its options: what an item says, whether it destroys something or is disabled, and what choosing it does. An icon, a shortcut and a submenu of an item are not shown |
| `title(&str)` | What the sheet is about, over its options |
| `cancel_label(&str)` | What Cancel says: "Cancel" |
| `on_dismiss(Fn())` | Told when it is closed with no option: Cancel, a press on the scrim, Escape |
| `inline(bool)` | The sheet where it is declared, open or not, without the scrim: for a gallery, a screenshot, a test |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. Its options are `<id>-option-<n>`, Cancel `<id>-cancel` |

- It rises from the bottom as the scrim fades in, and goes down as it fades out. It takes the focus as it opens and
  gives it back as it closes; the arrow keys move among the options, and Enter and Space choose.
- Parts: `ActionSheetPart::Scrim`, `Root`, `Options`, `Title`, `Option`, `Label`, `Cancel`, `CancelLabel`. States:
  `ActionSheetState::Destructive`, `Disabled`.
- In the design the options are a card in `Card` with a line of `Border` and the large corners, 8 in from the
  screen's sides and bottom: the title 13 in `MutedForeground`, 12 over and under; options of 56 with a line over
  each and their words 17 in the middle, in `Foreground` or `Destructive`; Cancel 8 under them, a card of its own,
  17 semibold. The scrim is `Scrim`.
- To what reads the tree it is a `dialog` called by its title, its options and Cancel `button`s.

## Tab And The Design System's Controls

A plain `lurq::components::Button` is a stop of Tab only inside a form or with a `tab_index` of its own. A control of the design system, a button, a toggle, a link, the track of a switch, the box of a checkbox, an input, is a stop wherever it is, in the order of the tree; a disabled one is passed by.

What is one choice or one bar is one stop, and the arrow keys move within it ([focus groups](../focus-navigation/#focus-groups)): a radio group and a segmented control, where the choice follows the focus, and a toggle group, where Space turns the toggle the focus is on. In a button group every button is its own stop, as each is its own action, and the arrows move between them as well.

## A Component That Holds Another

A group does not draw its buttons: it holds them, and says what they are in its place. Its recipe has a part for that (`ButtonGroupPart::Item`), and `ResolvedRecipe::rules_for(part, inner)` turns what the recipe has for it, at rest and in each interaction, into rules for a part of the inner component, which the group lays over the theme's recipe for that component (`Button::rules`):

```rust
let recipe = theme.resolve::<ButtonGroup>(&case, None);
let item = recipe.rules_for(ButtonGroupPart::Item, ButtonPart::Root);
let button = button.rules(&item); // over the theme's recipe for a button, under the button's own style
```

So a theme restyles a button in a group by patching the group's recipe, and a button elsewhere is as it was.

## A Component Of Your Own

A component of an app is themed the same way when it implements `Themed`: a name the theme keeps its recipe under, the type of its renderer, and its recipe when the theme says nothing about it.

```rust
use std::sync::Arc;
use lurq::{node::Element, ui::{Case, Recipe, Themed, current_theme}};

struct Tile;

impl Themed for Tile {
  const NAME: &'static str = "tile";
  type Renderer = Arc<dyn Fn(&str) -> Element + Send + Sync>;

  fn recipe() -> Recipe {
    Recipe::new().base("root", |s| s.padding_x(16.0).padding_y(12.0))
  }
}

fn tile(title: &str) -> Element {
  let theme = current_theme();
  let recipe = theme.resolve::<Tile>(&Case::new(), None);
  let root = recipe.part("root");
  lurq::components::Column::new().part(&root).child(root.text(title)).into()
}
```

`theme.patch_recipe::<Tile>(...)`, `set_recipe`, a bare theme and `in_theme` then reach it as they reach lurq's own.

A component says what a recipe styles of it: `Themed::parts()` and `Themed::states()`, each a `Declared`, the names and the Rust that writes them. lurq's own give their `*Part` and `*State` enums (`names!` gives each a `declared()`: `fn parts() -> Declared { SwitchPart::declared() }`); one that says nothing has the parts and the states its recipe names. A tool reads them: the gallery's playground lists them in its Styling tab. A builder of a component of one's own that has `rules` and `state` implements `Styled` with them, and is styled by what styles lurq's.
