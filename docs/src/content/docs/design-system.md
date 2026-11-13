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
| `at(breakpoint, part, style)` | In a window at a [breakpoint](../theme/#breakpoints) or wider |
| `state(state, part, style)` | While the component is in a state: disabled, checked, loading |
| `hovered(part, style)`, `pressed(...)`, `focus_visible(...)` | Under the pointer, held down, focused from the keyboard |
| `when(selector, part, style)` | In the case a `Selector` describes: several of the above together |

- The rules that hold are laid one over another, the more specific over the less: always, then a variant, then a size, then a density, then a breakpoint (a wider one over a narrower), then states (more of them over fewer). Rules of one rank apply in the order they were written.
- A style a later rule sets replaces the earlier one's field by field; what it leaves alone shows through.
- Variants, sizes, states and parts are names. A component gives its own as enums, which are accepted where a name is, and an app adds any others: `.variant("brand", ...)`, `.state("pinned", ...)`.
- Rules for the pointer and the keyboard become [state styles](../styling-events/) of the node. A hover renders nothing again, and changes only what is painted: no size, padding or text.
- `Recipe::extend(&other)` adds another recipe's rules after this one's.

### Breakpoints

A rule `at(Breakpoint::Md, ...)` holds at that breakpoint and at every wider one; a rule with no breakpoint is for every width, the narrowest included. This is the order [`Responsive`](../theme/#responsive-values) values resolve in.

```rust
use lurq::{app::theme::Breakpoint, ui::{Button, ButtonPart}};

// A finger's button on a narrow window, the design's from the medium breakpoint up.
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

The design's recipe for `Button` has no rule for a density: a button has its sizes. `Selector::density(..)` joins a density with a variant, a size or a state in one rule.

## Part Styles

`PartStyle` is the style of one part: a patch in which every field is optional. Every component uses this one type.

| Builder | Sets |
| --- | --- |
| `background(color)`, `foreground(color)` | The fill, and the color of the text and icons in the part and under it |
| `border(border)`, `radius(radius)` | A border on every side; the corners |
| `underline(border)` | A line along the bottom of the part only: what a link has under the pointer |
| `shadow(shadow)`, `veil(color)` | Shadows; a color laid over the fill and under the content (`StateHover`, `StatePressed`) |
| `outline(outline)`, `focus_ring()` | An outline; the theme's focus ring as the outline |
| `opacity(value)` | A number, or a level of the theme's (`OpacityLevel::Disabled`) |
| `padding_x(spacing)`, `padding_y(spacing)`, `gap(spacing)` | Padding, and the room between the part's children |
| `width(length)`, `height(length)`, `min_width(length)`, `square(length)` | A `Length`: pixels, a step of the control, density or icon scale, `Length::Fill` |
| `typography(role)` | Every field of the part's text at once |
| `font_family(family)`, `font_size(px)`, `weight(weight)`, `font_style(style)`, `line_height(ratio)`, `letter_spacing(px)` | One field of the text, over the role's |
| `transition(PartTransition::new(Transition::background_color(), MotionDuration::Fast))` | A transition, with a duration and an easing of the theme's |
| `cursor(icon)` | The cursor over the part |

Colors, radii, spacings and shadows are roles of the theme or concrete values, as everywhere in lurq.

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
```

- The order, later over earlier: the theme's recipe, the rules of the place (`.rules(...)`), the instance's own (`.style(...)`).
- `ctx.in_theme(&theme, |ctx| ...)` is the theme of everything the closure builds and mounts. Components mounted in it keep the theme when they render again on their own.
- A theme for a subtree changes what a component takes from the theme as it is built: its recipe and its renderers, the sizes, type, opacity and motion its parts resolve, its density and its icons. The roles its nodes are painted with, a color of the palette, a radius, a shadow, a spacing role, are still read from the window's theme: a subtree in a dark theme inside a light window is not dark yet.
- `ui::current_theme()` is the theme of what is being built. Outside a render there is none, and it is a neutral one that follows nothing: build components where they are rendered.

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
```

| Builder | |
| --- | --- |
| `Icon::new(name)` | A constant of `ui::icons`, or a name the theme added |
| `size(size)` | An [`IconSize`](../theme/#icon-size) (`Md`, 16, by default) or a number of logical pixels. The icon is a square of that side |
| `color(color)` | A color in place of the foreground of what it is in |
| `spin(bool)` | It turns once in `ui::SPIN_PERIOD`, a second |
| `id(id)`, `key(key)` | Its id and key |

- An icon names no color: it is drawn in the `.foreground(..)` of what it is in, as text is, so the icon of a button is the color of its label.
- `ui::icons` has a constant for each of the 152 icons of lurq's font (`icons::CHEVRON_DOWN` is `"chevron-down"`), `icons::ALL` lists them with their characters, and `icons::FAMILY` is the family of the font. Every app has the font loaded, hinted: at 16 and 20 pixels its strokes land on whole pixels of the display.
- The theme says what a name is drawn with ([`ThemeIcons`](../theme/#icons)). A theme gives a name another glyph, or adds names of its own, from a font the app loads; lurq's components name the icons they draw, so they change with it.
- A name the theme has no glyph for takes its room and draws nothing.
- Spinning icons are in phase: spinners that appear at different moments turn together. `ui::SPIN` is the keyframes of the turn, registered in every window; keyframe identifiers from `0xFF00` up are lurq's own.

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
| `icon_only(bool)` | The icon alone: a square as wide as it is high, a circle with `ButtonShape::Pill`. The label is not drawn |
| `loading(bool)` | A spinner turns in the place of the icon, or before the label, and a click, Enter and Space do nothing until it is over |
| `description(text)` | A second line under the label that says more of what the button does. The button is as high as the two lines, with an icon of 20 beside them |
| `leading(element)`, `trailing(element)` | Any element before and after the label, where an icon by name is not enough |
| `disabled(bool)` | It takes no input: no press, hover or focus, and Tab passes it by. It is dimmed to the theme's `OpacityLevel::Disabled` |
| `full_width(bool)` | As wide as what it is in |
| `on_click(handler)` | A click, and Enter and Space while it is focused |
| `state(name, on)` | A state the theme's recipe names |
| `style(part, \|s\| ...)`, `rules(&recipe)` | One instance over the theme; the rules of the place it is in |
| `force(Some(Interaction::Hovered))` | Drawn hovered, pressed or focused with nothing pointing at it: for a gallery, a screenshot, a test |
| `id(id)`, `key(key)`, `submit()` | Its id and key; with the `form` feature, the button that submits the form it is in |

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

States a recipe styles: `ButtonState::Disabled`, `FullWidth`, `Loading`, `IconOnly`, `Pill` and `Compound` (it has a description), and hovered, pressed and focus-visible.

- In the design a button under the pointer has its variant's hover fill (`PrimaryHover`, `SecondaryHover`, `DestructiveHover`, and `Highlight` for an outline or a ghost button), a button held down has the `StatePressed` veil over that, and one focused from the keyboard has the theme's focus ring.
- A loading button shows three quarters of itself. It stays where the focus is and is not pressed: a form it submits is not submitted again. Its label is the app's to change.
- A theme's renderer for `ButtonPart::Label`, `ButtonPart::Icon` or `ButtonPart::Spinner` builds that part in place of the default one: dots that pulse where the design turns a ring.
- A button that is an icon alone is still called by its label: tooling that reads the tree (DevTools, the MCP tools) names it so.
- The design's close button, menu button and compound button are this button: `Button::new("Close").variant(ButtonVariant::Ghost).size(ButtonSize::Xs).icon(icons::CLOSE).icon_only(true)`, `Button::new("Export").variant(ButtonVariant::Outline).trailing_icon(icons::CHEVRON_DOWN)`, and a button with a `description`.

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
| `on_change(handler)` | Called on a press with what the toggle is to be: on if it was off |
| `icon(name)`, `icon_only(bool)` | An icon before the label; the icon alone, a square |
| `size(size)` | A `ButtonSize`: a toggle is as high as a button of that size |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them |

Parts: `TogglePart::Root`, `Label`, `Icon`. States: `ToggleState::On`, `Disabled`, `IconOnly`. In the design a toggle has no fill until the pointer is on it or it is on, and then the `Highlight` fill; it is padded 10.

## Button Group

Buttons that belong together, joined into one shape.

```rust
use lurq::ui::{Button, ButtonGroup, ButtonGroupVariant, icons};

ButtonGroup::new()
  .child(Button::new("Years"))
  .child(Button::new("Months"))
  .child(Button::new("Days"));

// An action and the choice of how to do it: a split button.
ButtonGroup::new()
  .variant(ButtonGroupVariant::Primary)
  .child(Button::new("Merge").on_click(|_| merge()))
  .child(Button::new("How to merge").icon(icons::CHEVRON_DOWN).icon_only(true).on_click(|_| choose()));
```

| Builder | |
| --- | --- |
| `child(button)`, `children(buttons)` | The buttons, in order |
| `variant(variant)` | `Outline` (the default): one line around the group and a line between its buttons, which have no fill. `Primary`, `Secondary`, `Destructive`: buttons in that fill, a pixel apart |
| `size(size)` | The `ButtonSize` of every button in it |
| `style(part, ..)`, `id`, `key` | One instance over the theme; its id and key |

- The group gives its buttons their variant and their size and squares their corners: the corners of the group are the round ones. A button keeps the rest of what it was given: its icon, what a press does, whether it is disabled.
- Parts: `ButtonGroupPart::Root`, the group; `Item`, what the group says of each button, laid over the theme's recipe for a button's root (padded 14 in the design); `Separator`, the line between two.
- The second button of a split button opens what the app shows: a menu of the design system is not built yet.

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
| `ToggleGroup::controlled(on, choices, press)` | The app holds the state: the toggles of `on` are on, and `press` is told the value of each one pressed |

A choice is a value and a label, `(value, "Label")`, or a `Choice` with an icon, the icon alone, or disabled. `size(size)` and `disabled(bool)` are for the whole group. Parts: `ToggleGroupPart::Root`, `Item` (laid over a toggle's root) and `Separator`.

## Segmented Control

One of a few choices that are all in view, side by side on a track.

```rust
use lurq::ui::SegmentedControl;

SegmentedControl::new(period.clone(), [(Period::Day, "Day"), (Period::Week, "Week"), (Period::Month, "Month")]);

// The app holds what is chosen.
SegmentedControl::controlled(theme.preferred_appearance(), [(None, "System"), (Some(Light), "Light")], |look| {
  theme.set_preferred_appearance(look)
});
```

| Builder | |
| --- | --- |
| `SegmentedControl::new(signal, choices)` | The chosen segment is the one with the signal's value; a press of another sets it |
| `SegmentedControl::controlled(value, choices, choose)` | The app holds the value and is told of a press |
| `size(size)` | A `ButtonSize`: the track is as high as a button of that size |
| `full_width(bool)` | As wide as what it is in, its segments sharing the width |
| `disabled(bool)`, `style(part, ..)`, `force(..)`, `id`, `key` | As a button has them |

- Each segment is a button: Tab reaches it, and a click, Enter or Space chooses it.
- Parts: `SegmentedControlPart::Root`, the track (`Muted`, padded 3, its segments 2 apart); `Segment`; `Label`; `Icon`. States: `Selected`, of a segment (a `Background` surface with a small shadow); `Disabled`; `FullWidth`.

## Link

Text that goes somewhere when it is pressed. It has no box, so it sits in a line of text or beside one.

```rust
use lurq::ui::Link;

Link::new("Read the docs").external().on_click(move |_| navigator.push("/docs"));
```

| Builder | |
| --- | --- |
| `on_click(handler)` | A click, and Enter and Space while it is focused. Where the link goes is what its press does |
| `icon(name)`, `external()` | An icon after the text; `external()` is the arrow of a link that leaves the app |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `force(..)`, `id`, `key` | As a button has them |

Parts: `LinkPart::Root`, `Label` (14/500), `Icon` (14, four after the text). Under the pointer the root has an `underline`.

## Fab

A floating action button: the one thing a view is for, raised over it.

```rust
use lurq::ui::{Fab, icons};

Fab::new("Compose", icons::COMPOSE).on_click(|_| compose());
Fab::new("Compose", icons::COMPOSE).extended(true); // with its label
```

- It is a [button](#button) that is an icon alone, round, 48 across, with an icon of 20 and the theme's large shadow, and takes what a button takes: `variant`, `on_click`, `disabled`, `loading`.
- Parts: `FabPart::Root`, `Icon`, `Label`, each laid over the theme's recipe for the same part of a button. State: `FabState::Extended`.
- Where it floats is the app's to say: it is an element, put in a corner of a `Stack`. A container clips what is in it, so give its shadow room, or let the container show what is outside it (`.overflow_visible()`).

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
