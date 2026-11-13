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
- `ctx.in_theme(&theme, |ctx| ...)` is the theme of everything the closure builds and mounts, and returns it as an `Element`. Components mounted in it keep the theme when they render again on their own.
- The theme reaches the whole of the subtree. A component built in it takes its recipe and its renderers, the sizes, type, opacity and motion of its parts, its density and its icons from that theme; and the roles left on the nodes, a color of the palette, a radius, a border width, a shadow, a spacing role, a typography style, the scroll bar and the caret, are laid out and painted with that theme's values. A subtree in a dark theme inside a light window is dark.
- `.theme(&theme)` on any element does the second half alone: the element and what is in it are laid out and painted with the values of that theme. What was built outside `in_theme` keeps the recipes it was built with, so this is for plain elements, and `in_theme` is for components.
- A change of the subtree's theme while the app runs lays the window out again, as a change of the window's theme does, and is there at once: a transition in the subtree runs between the colors of its theme, and not from the window's to its.
- What is declared in the subtree and drawn over the window, the content of a `Popup`, a `Modal`, the menu of a `Select`, is laid out and painted with the subtree's theme too.
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
- A split button is a filled group of two: the action, and a button that opens a [menu](#menu) hung from the group (`Menu::new(anchor, open).placement(Placement::BottomEnd)`, the group in an element that carries the anchor's ref).

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

A choice is a value and a label, `(value, "Label")`, or a `Choice` with an icon, the icon alone, or disabled. `size(size)` and `disabled(bool)` are for the whole group. Tab stops in the group once; Left and Right move between its toggles, and Space or Enter turns the one the focus is on. Parts: `ToggleGroupPart::Root`, `Item` (laid over a toggle's root) and `Separator`.

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

- The control is one choice: Tab stops in it once, at the chosen segment; Left and Right move the focus to the next segment and choose it, Home and End the first and the last, and a click chooses one.
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

## Switch

A setting that is on or off and takes effect at once: dark mode, sync in the background.

```rust
use lurq::ui::Switch;

Switch::new("Dark mode")
  .on(dark.get())
  .on_change(move |on| dark.set(on));

// The track alone, in a row of settings that has the text already.
Switch::new("Sync in background").label_hidden(true).on(sync.get());
```

| Builder | |
| --- | --- |
| `on(bool)` | Whether it is on. The switch does not keep it: the app says so |
| `on_change(handler)` | Called on a press with what the switch is to be: on if it was off |
| `label_hidden(bool)` | The track alone. The label is still what the switch is called |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the track |

- A press of the track or of the label asks for the other state. Tab reaches the track, Space and Enter press it, and the focus ring is the track's.
- Parts: `SwitchPart::Root` (the track and the label, side by side), `Track` (what is pressed), `Thumb`, `Label`. States: `SwitchState::On`, `Disabled`.
- In the design the track is 36 by 20 and round, in the `Input` color and in `Primary` while it is on; the thumb is 16, in `Background`, 2 from the edge; the label is 8 beside the track in the body's type. A disabled switch is dimmed with its label.
- The thumb slides: the default renderer moves it by an offset, how far is `SwitchView::travel()` (the track's width less its padding and the thumb), and the recipe gives the thumb a transition of `Transition::offset_x()` over the theme's fast duration. A theme with reduced motion moves it at once.
- To what reads the tree (`lurq_snapshot` of the MCP server) the track is a `switch` called by its label, with `a11y_checked` saying whether it is on.
- A renderer that draws another switch starts from `view.track()`, the behavior with the track's style, and puts in it what it likes; `view.thumb()` is the default thumb.

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
| `on_input(handler)` | Called before an edit with the `TextInputEvent`; `prevent_default()` refuses it |
| `name(..)` | The name of its value in a form (feature `form`) |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the control, the box that is typed in |

- It is the engine's `TextInput`, single line, dressed by the recipe: the control *is* the box, so that the ring of its focus is drawn on it. A text input is focus-visible however it was focused, so a click shows the ring too. Tab reaches it wherever it is (`tab_index(0)`).
- Parts: `InputPart::Root` (the place it takes: its width, and the room between the icon and the text), `Control` (the box: edge, fill, height, the type of the value), `Placeholder` (laid over the control's type), `Icon`. States: `InputState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`, `WithIcon`. Sizes: `InputSize::Sm`, `Md`, `Lg` (`InputSize` is `TextBoxSize`, which a textarea has too).
- In the design the box is 260 by 36, `Background` in an edge of 1 in the `Input` color, `radius-md`, the text 12 from the edge in the body's type, the placeholder in `MutedForeground`. The edge is the only signal of its state: `Ring` under the pointer, a line of 2 in `Ring` inside the box in the focus (an outline, so that the edge under the pointer does not take it away), `Destructive`, `Warning`, `Success` for a judgment (which keeps its color under the pointer and in the focus). Disabled is half opaque on `Muted`; read only is on `Muted` in the `Border` color. The icon is 16 in `MutedForeground` (in `Success` while it is valid), 8 before the text.
- In a [field](#field) it is called by the field's label unless it has a `label` of its own, it is judged as the field is unless the app judged the input itself, and it is disabled where the field is.
- A text input does not take the foreground of its node, so the renderer names the color of the value and of the placeholder from the parts' foregrounds (`InputView::value_style()`, `placeholder_style()`).
- To what reads the tree the control is a `textbox`, with `a11y_invalid`, `a11y_readonly` where it is read only, and the name its label or its field's gives it (`InputView::label`).
- A renderer that draws another input starts from `view.control()`, the bound text input in the control's style; `view.icon()` is the icon laid over its start, and `view.text_start()` how far the text starts from the edge.
- What an input and a textarea have in common is one recipe's worth of rules on the box (its edge and fill, the focus inside it, the placeholder's color, the judgments, read only, disabled, the type of the sizes): a theme that changes one of them for both patches each recipe, by the same part and state names.

## Field

A control with what it is called above it, and under it what it is for or what is wrong with what it holds. The field finishes its control: the label names it, a press of the label focuses it, and an input in a field with an error is invalid without being told.

```rust
use lurq::ui::{Field, Input, Switch, Textarea};

Field::new("Display name", Input::new(name.clone()).full_width(true))
  .description("Shown in the title bar.")
  .required(true);

// The error makes the input invalid: its edge is destructive.
Field::new("Email", Input::new(email.clone()).full_width(true))
  .error("Enter a full email address.");

Field::new("What happened", Textarea::new(story.clone())).warning(true);

// Any element is a control; the label beside it, where a form lines its labels up.
Field::new("Notifications", Switch::new("Pause notifications")).horizontal(true);
```

| Builder | |
| --- | --- |
| `new(&str, impl FieldControl)` | Its label and its control: an `Input`, a `Textarea`, another component, any `Element` (an engine node goes in with `.into()`) |
| `description(&str)` | What the control is for, under it |
| `error(&str)` | What is wrong, in place of the description; the field is invalid |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | The description in the destructive, warning or success color, and the control judged so |
| `disabled(bool)` | Its control takes no input, and a press of its label focuses nothing |
| `required(bool)` | A `*` after the label |
| `optional(bool)`, `optional_text(&str)` | "Optional" after the label, or the app's own words. A required field shows none |
| `horizontal(bool)` | The label in a column of 110 beside the control and its description |
| `full_width(bool)` | As wide as what holds it, in place of 260 |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the field's root |

- A field's control is a `FieldControl`: what the field tells it (`InField`: the label, the judgment, whether it is disabled, the description) it takes as far as it has the prop, and only where the app did not say it of the control itself: an input with a label of its own keeps it, and one the app judged keeps that judgment whatever the field says. A control of an app's own implements `FieldControl` and passes what it is told to the control in it. Any `Element` takes none of it.

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
- An input that is to fill the field says so (`Input::full_width`).
- Parts: `FieldPart::Root`, `Label` (the label and its mark), `Required`, `Optional`, `Body` (the control and the description, where the label is beside them), `Description`. States: `FieldState::Invalid`, `Warning`, `Valid`, `Horizontal`, `FullWidth`, `Disabled` (no rule of the design's: a theme dims what it likes).
- In the design the field is 260 wide; the label is 6 above the control in a field label's type (13, medium) and `Foreground`, the description 6 under it in a caption's (12) and `MutedForeground`. The `*` is 4 after the label, semibold, in `Destructive`; "Optional" is 11 in `MutedForeground`. Beside the control the label is 110 wide and as high as a control, 12 from it, and the field is 360 wide.

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
| `on_change(handler)` | Called on every edit with the text it holds once the edit applies. A read-only one makes no edit |
| `label(&str)` | What it is called, to what reads the tree. It draws nothing: a [field](#field) gives it its own label |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box |

- It is the engine's multiline `TextInput`: as high as its text between its rows, its own scroll bar past them, Enter a new line. Tab reaches it wherever it is, as every control of the design system; a disabled one is passed by and takes no click. A read-only one is a stop of Tab and refuses every edit.
- Parts: `TextareaPart::Root` (the box, which is the text input), `Text` (the type and color of what it holds), `Placeholder` (laid over the text's type). States: `TextareaState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`. Sizes: `TextareaSize::Sm`, `Md`, `Lg` (`TextareaSize` is `TextBoxSize`, an input's sizes).
- In the design the box is as wide as its room, in `Background` with an edge of 1 in `Input`, the medium radius, padded 12 across and 8 down; its text is in the body's type with a line of 1.5, in `Foreground`, and the placeholder in `MutedForeground`. Under the pointer the edge is `Ring`; focused, a ring of 2 is drawn inside the box, over the edge. Invalid, warning and valid, the edge and the ring are `Destructive`, `Warning` and `Success`. Read only, it is on `Muted` in the `Border` color, as a read-only input. Disabled, it is at the theme's disabled opacity on `Muted`. Small, its text is 13; large, 15; the padding stays.
- In a [field](#field) it is called by the field's label unless it has a `label` of its own, it is judged as the field is unless the app judged it itself, it is disabled where the field is, and a press of the label focuses it.
- A text input draws in its own text style and does not take the foreground of what it is in, so the `Text` and `Placeholder` parts' foregrounds are resolved to colors of the theme when the textarea is built (`TextareaView::text_style`, `placeholder_style`).
- To what reads the tree it is a `textbox`, `a11y_multiline`, with `a11y_invalid`, `a11y_readonly` where it is read only, and the name its label or its field's gives it.
- A renderer that draws another textarea starts from `view.input()`, the behavior in the box's style, and puts what it likes around it: a count of characters, a grip.

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
- `CheckboxGroup` is checkboxes over one list: `CheckboxGroup::new(signal, choices)` with a `Signal<Vec<T>>` checks the boxes whose values the signal has, and a press adds a value or takes it out; `CheckboxGroup::controlled(checked, choices, press)` leaves the list to the app. `horizontal(bool)`, `invalid(bool)` and `disabled(bool)` are for the whole group, and `Choice::disabled` takes one box out. Each box stays its own stop of Tab. Its parts are `CheckboxGroupPart::Root` (12 apart, or 24 side by side in the state `CheckboxGroupState::Horizontal`) and `Item`, laid over each checkbox's root.
- `lurq::components::Checkbox` is the engine's node, which keeps its value in a `Signal` and draws its mark from an image; `lurq::ui::Checkbox` is the design system's and does not use it.

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
| `horizontal(bool)` | The radios side by side, going on to the next line where there is no room |
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
  once made is not taken back. A group is one stop of Tab, at the chosen radio or the first; the arrow keys
  move the focus to the next radio and choose it, Home and End the first and the last, and a disabled one is
  passed by. A radio on its own is a stop of Tab that Space and Enter press. The focus ring is the mark's.
- Parts: `RadioPart::Root` (the mark and the label, side by side), `Mark` (what is pressed), `Dot`, `Label`.
  States: `RadioState::Chosen`, `Invalid`, `Disabled`. A group's parts: `RadioGroupPart::Root` (the radios
  one under the other, 12 apart) and `Item`, laid over the theme's recipe for each radio's root; its state
  `RadioGroupState::Horizontal` (side by side, 24 apart).
- In the design the mark is 16 and round, in `Background` with an edge of 1 inside it in `Input`; chosen, the
  edge is `Primary` and a dot of 8 in `Primary` is in its middle. The edge of an invalid one is
  `Destructive`. The label is 8 beside the mark in the body's type, and a group's radios are 12 apart. A
  disabled radio is dimmed with its label.
- A choice's icon is not drawn: a radio is its mark and its label.
- To what reads the tree (`lurq_snapshot` of the MCP server) the mark is a `radio` called by its label, with
  `a11y_checked` saying whether it is chosen, and a group is a `radiogroup`.
- A renderer that draws another radio starts from `view.mark()`, the behavior with the mark's style, and puts
  in it what it likes; `view.dot()` is the default dot.

## Slider

A value picked along a track, between a least and a greatest: volume, the size of a brush, a price no higher than. A range slider picks two on one track: a price between, the hours a shop is open.

```rust
use lurq::ui::{RangeSlider, Slider, SliderOrientation};

// volume: Signal<i32>
Slider::new(volume.clone()).label("Volume");

// A fraction, in steps of a twentieth.
Slider::new_f32(opacity.clone()).step(0.05).label("Opacity");

// From 10 to 50 in steps of 5.
Slider::new(size.clone()).min(10).max(50).step(5);

// Up, the least value at the bottom; 100 high.
Slider::new(level.clone()).orientation(SliderOrientation::Vertical).length(100.0);

// price: Signal<(i32, i32)>, the low value first.
RangeSlider::new(price.clone()).step(5).label("Price");
```

| Builder | |
| --- | --- |
| `Slider::new(Signal<i32>)`, `Slider::new_f32(Signal<f32>)` | The value it is bound to: whole numbers, from 0 to 100 in steps of 1, or fractions, from 0 to 1 in steps of a hundredth |
| `RangeSlider::new(Signal<(i32, i32)>)`, `RangeSlider::new_f32(Signal<(f32, f32)>)` | The two values a range slider is bound to, the low one first, in the same ranges and steps |
| `min(v)`, `max(v)`, `step(v)` | The least value, the greatest and how far apart the values are, from the least. Any number: `min(10)`, `step(0.25)` |
| `orientation(SliderOrientation)` | `Horizontal`, or `Vertical`: 16 wide and 140 high, the least value at the bottom |
| `length(f32)` | How long the slider is along its track: its width across, its height up |
| `label(&str)` | What the slider is called, to what reads the tree. The slider shows no text: the field it is in does |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the slider's node |

- It is the engine's [`Slider`](../styling-events/#slider-styling) dressed by the recipe: a drag of the thumb, a press on the track and the keys set the signal, snapped to the step and kept in the range. Tab reaches it wherever it is (`tab_index(0)`), and the focus ring is the thumb's. A vertical slider is dragged along its height, and the up and down arrows move it.
- The keys: the arrows a step (right and up toward the greatest value), Page Up and Page Down a tenth of the range in whole steps and at least one, Home and End to the least and the greatest. The [engine's slider keys](../styling-events/#slider-keys) are the same for a vertical slider and for each thumb of a range slider.
- A range slider is the engine's slider with two thumbs. A press on the track moves the nearer thumb, a drag the thumb it began on, and neither passes the other, by the pointer or the keys. Each thumb is a stop of Tab, the low one first; the keys move the one focused, and the ring is around it. In a form it is `FormRangeSliderInput` (see [Forms](../forms/)).
- Parts: `SliderPart::Root` (the slider's node: its size, its cursor, its opacity), `Track`, `Range` (the track from its start to the thumb, or between a range slider's thumbs), `Thumb` (both of a range slider's). States: `SliderState::Disabled`, `Vertical`, `Range`. A range slider is drawn by the slider's recipe and renderer in the state `Range`, so a theme that restyles the slider restyles it too, and `rules.state(SliderState::Range, ..)` styles it alone.
- In the design the slider is 240 by 16; the track is 4 high and round, in `Secondary`; the range is `Primary`; the thumb is 16, round, in `Background` with an edge of 1.5 in `Primary` inside it. The vertical one is 16 by 140, its track 4 wide and as high as it. A disabled slider is dimmed. `style(SliderPart::Root, |s| s.width(Length::Fill))` makes it as wide as what holds it.
- The engine paints the track, the range and the thumb, not nodes: of their style it takes the width and the height in pixels, the fill, the corners, the border and the outline (`focus_ring()` too), at rest, under the pointer (`hovered`), and for the thumb focused from the keyboard (`focus_visible`). An opacity, a shadow or a transition of one of those three parts is not painted. On a vertical slider a part's width is its thickness.
- To what reads the tree (`lurq_snapshot` of the MCP server) it is a `slider` called by its label; a range slider's value is `low,high`.
- A renderer that draws another slider starts from `view.slider()`, the engine's slider bound to the value (or the two) with its orientation, the root's style and the length, and `view.dressed(slider)` gives it the recipe's track, range and thumbs. `view.value()` reads the value, `view.values()` a range slider's two, and `view.is_range()` tells which it is.

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
```

| Builder | |
| --- | --- |
| `new(Signal<T>, options)` | What it is bound to, and its options: `(value, label)` pairs or `SelectOption`s (`detail`, a second line; `hint`, a note at its end; `leading` and `trailing`, an element before and after the label, which the closed select shows for the chosen option, and `leading_with` and `trailing_with` for one built for the state of its row, as an [icon](#icon) in the color of the label is; `group(label)`, the group it starts; `disabled`). The option whose value the signal has is chosen; choosing another sets the signal |
| `placeholder(&str)` | What the trigger says while no option is chosen |
| `size(SelectSize)` | `Sm` 32, `Md` 36 (the default), `Lg` 40 high, with text of 13, 14, 15: an input's sizes |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | What the app judged of what is chosen: the edge and the ring in the destructive, warning or success color. One is shown, in that order, as an input shows them |
| `full_width(bool)` | As wide as what holds it, in place of 260 |
| `label(&str)` | What it is called, to what reads the tree. The field it is in shows its label |
| `name(..)` | The name of its value in a form, the chosen option's label (feature `form`) |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the select's node. `state(SelectState::Open, true)` draws the trigger as it is while open, for a gallery |

- It is the engine's [`Select`](../styling-events/#select) dressed by the recipe, not a second one: a press, or Enter, Space and the arrows from the keyboard, open the list under it (over it where there is no room); the arrows, Home, End and the first letters move through it, skipping disabled options and the groups' labels; Enter or a press chooses, and Escape closes. Tab reaches it wherever it is (`tab_index(0)`); a disabled one does not open and Tab passes it by.
- It stands beside an [input](#input): the same box, sizes, edge, ring and judgments. The ring is a line of 2 inside the box in `Ring`, shown from the keyboard and while the list is open.
- Parts: `SelectPart::Trigger` (the select's node and box: its size, edge, fill, padding, ring, opacity, cursor, the type of what it says, and in its gap the room before the chevron), `Value` and `Placeholder` (laid over the trigger's type and color), `Chevron` (its side and color), `Menu` (the panel: fill, edge, corners, shadow, padding; its gap is the room under the trigger, its height the most it is before it scrolls), `Item` (an option: height, padding, corners, type, color; `hovered` is the item under the pointer or the keys; its gap is the room before the check), `Check` (the chosen item's mark), `Detail`, `Hint`, `Group` (a group's label), `Separator` (the line between groups: its fill, its height, and in its padding the room around it). States: `SelectState::Open`, `Invalid`, `Warning`, `Valid`, `Disabled`, `FullWidth`, and of an item `Chosen` and `ItemDisabled`. Sizes: `SelectSize::Sm`, `Md`, `Lg`.
- In the design the trigger is 260 by 36, `Background` in an edge of 1 in the `Input` color, `radius-md`, the text 12 from the edge in the body's type, the placeholder and a chevron of 16 in `MutedForeground`, 8 apart; `Ring` under the pointer; `Destructive`, `Warning` or `Success` for a judgment, under the pointer, in the focus and open too; disabled is half opaque on `Muted`. The list is 4 under it and as wide, on `Popover` in an edge of `Border`, `radius-lg`, with the theme's middle shadow and 4 of padding. An item is 32 high, its text 8 in, `radius-sm`, in the body's type, on `Highlight` under the pointer or the keys; the chosen one has a check of 16 in `Foreground` at its end, 8 after the label; a disabled one is at 0.45. A hint is 11 in the mono type in `MutedForeground`. A group's label is 11, semibold, in `MutedForeground`, in line with the items' text; a line of 1 in `Border` is between groups, 4 from each.
- The engine paints the trigger, the panel and the rows itself, from its `SelectStyle`, not from nodes the recipe styles: of the trigger it takes the fill, the edge, the corners, the shadow, the ring and the padding, at rest, `hovered`, `focus_visible` and in the state `Open`; of an item, the fill, edge, corners, padding, height (at least), type, color and opacity, at rest, `hovered` and in the states `Chosen` and `ItemDisabled`; of a group's label, its padding, fill, type and color; of the separator, its fill, height and padding. A transition is not painted.
- To what reads the tree (`lurq_snapshot` of the MCP server) it is a `combobox`, called by its label where it has one.
- A renderer that draws another select starts from `view.select_style()`, the engine's `SelectStyle` the recipe makes, changes what it likes (another chevron, a menu as wide as its widest option), and builds it with `view.select(style)`, the engine's select bound to the value with the trigger's size, opacity and cursor.

## Popover

A panel beside the button that opens it, over the window, holding anything: the size of a layer, a few settings, a form. A press outside it or Escape closes it.

```rust
use lurq::{
  components::Placement,
  ui::{Button, ButtonVariant, Field, Input, Popover},
};

// anchor: ElementRef, open: Signal<bool>, width: Signal<String>
Popover::new(anchor.clone(), open.clone())
  .trigger(Button::new("Dimensions").variant(ButtonVariant::Outline))
  .title("Dimensions")
  .description("Set the size of the layer.")
  .placement(Placement::BottomStart)
  .child(Field::new("Width", Input::new(width).full_width(true)).full_width(true));
```

| Builder | |
| --- | --- |
| `new(anchor, open)` | The element it is shown beside, and the app's `Signal<bool>` of whether it is shown |
| `trigger(..)` | The button that opens and closes it: put before it, given the anchor, and pressed to turn the signal |
| `title(&str)`, `description(&str)` | The header: what it is about, and under it what it is for. Without either there is no header |
| `child(..)`, `children(..)` | What it holds, one under another, as wide as the panel |
| `placement(Placement)` | The side of the anchor it is on and the edge it lines up with. `BottomStart` by default |
| `width(..)` | How wide the panel is, over the recipe's 292 |
| `offset(..)` | How far it is from the anchor, along the side it is on: `SpacingSize::Xs` (4) by default |
| `collision(CollisionStrategy)` | What it does where there is no room on its side: `FlipThenClamp` by default |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the panel |

- It is the engine's `Popup` with the panel dressed by the recipe: it is placed against its anchor in every frame, goes to the other side where there is no room and is kept in the window, and closes on Escape and on a press outside it, which sets the signal to `false`. Declare it always and drive it with the signal: lurq only knows the anchor of a popup it can see.
- With a trigger, the trigger's own node is the anchor: a press of it, or Enter and Space on it, opens and closes the popover, and while another popup or select is open one press of it closes that one and opens this one. It tells what reads the tree `a11y_haspopup` and `a11y_expanded`. Without a trigger the app opens it, and the anchor is any element of its own (`ref_element`); a press of the anchor is not a press outside it.
- As it opens the focus goes to the first thing in it that takes the focus (`Popup::takes_focus`), and as it closes with the focus inside, back to what had it when it opened: the trigger. Its stops of Tab come right after the trigger's: Tab from its last one goes on to what follows the trigger. It does not keep Tab in it: it is not modal.
- The anchor is kept from one render to the next (a field of the component, `ctx.element_ref()`): one made anew in each render is not the one the focus goes back to.
- It fades in over the theme's `Base` duration in the `Enter` curve, and out over `Fast` in the `Exit` curve (`PartStyle::enter`, `PartStyle::exit` on the root, `FADE_IN` and `FADE_OUT`). A fade is not movement, so reduced motion leaves it.
- A theme given to a subtree reaches a popover declared in it.
- Parts: `PopoverPart::Root` (the panel), `Header` (the title over the description), `Title`, `Description`, `Body` (what it holds). It has no states of its own.
- In the design the panel is 292 wide in `Popover` and `PopoverForeground`, with an edge of 1 in `Border`, the `Lg` radius and the `Md` shadow, 16 of padding and 14 between the header and the body. The title is in the body's type at medium weight, the description in 13 and `MutedForeground`, 4 under it; what the body holds is 14 apart.
- To what reads the tree the panel is a `dialog` called by its title.
- A renderer builds the panel alone: where it is shown, when, its trigger and its focus stay the popover's. It starts from `view.panel()`, the panel with its style and nothing in it, and has `view.header()` and `view.children`.

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

Menu::new(anchor, open)
  .item(MenuItem::new("Undo").icon(icons::UNDO).shortcut("Ctrl+Z").on_select(move || undo()))
  .item(MenuItem::new("Redo").shortcut("Ctrl+Y").disabled(true))
  .separator()
  .item(MenuItem::new("Show grid").checked(grid.get()).on_select(move || toggle_grid()))
  .separator()
  .label("Zoom")
  .item(MenuItem::new("Fit to window").radio(zoom.get() == Zoom::Fit).on_select(move || fit()))
  .item(MenuItem::new("100%").radio(zoom.get() == Zoom::Actual).on_select(move || actual()))
  .separator()
  // export: ElementRef, exporting: Signal<bool>: the submenu hangs from its item.
  .item(MenuItem::new("Export as").submenu(Menu::new(export, exporting).items(["PNG", "SVG", "PDF"].map(MenuItem::new))))
  .item(MenuItem::new("Delete").icon(icons::TRASH).destructive(true).on_select(move || delete()));
```

| `Menu` | |
| --- | --- |
| `new(anchor, open)` | What it hangs from (an `ElementRef`) and whether it is open: the app's `Signal<bool>`, or a `bool` that holds it open or closed |
| `item(MenuItem)`, `items(..)` | Commands, after what the menu holds already |
| `separator()` | A line that ends one group of items and starts the next |
| `label(&str)` | A text over the items after it: what they are about |
| `placement(Placement)` | Where the panel opens beside its anchor, 4 from it. `BottomStart` by default, `RightStart` for a submenu |
| `inline(bool)` | The panel drawn where the menu is in the tree, open or not, and not over the window: for a gallery, a test, a list of commands in a page |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the panel |

| `MenuItem` | |
| --- | --- |
| `icon(name)` | An icon before the label, by its name in the theme's set |
| `shortcut(&str)` | The keys that do the same, written at the end: "Ctrl+Z". The item only shows them; the app binds the keys |
| `checked(bool)` | Whether what it turns on is on: a check at its end |
| `radio(bool)` | One of several of which one is chosen: a dot at the end of the chosen one |
| `submenu(Menu)` | A menu the item opens beside it, with a chevron at its end. The submenu's anchor is given to the item |
| `destructive(bool)` | It removes or cannot be undone: drawn in the destructive color, icon and all |
| `on_select(handler)` | Called when it is chosen, before its menu closes |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the item |

- The menu does not keep whether it is open: the app does, in the signal it gives it, and the anchor's own handler opens it. Choosing an item calls the item's handler and sets the signal to `false`; a press outside the panel and Escape do too (the engine's `Popup`). Declare the menu always, open or not, beside its anchor: lurq then knows the anchor is its trigger.
- The keyboard: a menu opened from the keyboard (Enter or Space on its button) has its first item focused and lit; opened by the pointer it has the focus there too, unlit. Up and Down move between its items and go round at the ends, Home and End go to the first and the last, and a disabled item is passed by. Typing a letter goes to the item whose label starts with it, letters typed quickly to the one a word starts, the same letter again to the next. Enter or Space chooses the focused item; the arrows only move the focus. Escape, or choosing, closes it and gives the focus back to its button. Tab stops in an open menu once (the panel is a [focus group](../focus-navigation/#focus-groups) of one stop).
- The pointer and the keyboard are on one item: the pointer coming onto an item moves the focus there, and an arrow key moves the light off the item under a pointer that holds still.
- Under the pointer or focused from the keyboard an item is highlighted: filled with `Highlight`, the board's `accent`. An item whose submenu is open stays highlighted.
- A submenu opens beside its item, its first item level with it, under the pointer, on a press of the item, or on Right, Enter or Space while the item is focused, with the focus on its first item; the pointer on another item of the menu closes it, and so do Left and Escape while the focus is in it, giving the focus back to its item. Choosing in it closes it and every menu it hangs from. A press in the menu while a submenu is open closes the submenu and still reaches what it pressed.
- Parts of the menu: `MenuPart::Panel`, `Separator` (the room around the line), `SeparatorLine`, `Label`; state `MenuState::Submenu`. Parts of an item: `MenuItemPart::Root` (what is pressed), `Icon`, `Label`, `Shortcut`, `Check` (the check or the dot), `Chevron`. States of an item: `MenuItemState::Checked` (checked, or the chosen radio), `Radio`, `Destructive`, `Disabled`, `Expanded` (its submenu is open).
- A menu is as wide as its widest item, and at least as wide as the design says; never as wide as the window. In the design the panel is at least 230 wide (a submenu 160), in `Popover` with `PopoverForeground`, padded 4, with the large radius, an edge of 1 in `Border` and the medium shadow. An item is 32 tall, padded 8, with the small radius and 8 between what it holds: an icon of 16 in `MutedForeground`, the label in the body's type, and at the end the shortcut in the mono type at 11 in `MutedForeground`, the check or the dot of 16, and the chevron of 16 in `MutedForeground`. A separator is a line of 1 in `Border` with 4 above and below; a group's label is 11/600 in `MutedForeground`, 8 in from the side. A destructive item is in `Destructive`, its icon too; a disabled one is dimmed.
- To what reads the tree the panel is a `menu`; an item a `menuitem`, a `menuitemcheckbox` or a `menuitemradio` (these two with `a11y_checked`) called by its label; an item with a submenu has `a11y_haspopup` `menu` and `a11y_expanded`; a separator is a `separator`.
- A renderer for `Menu` gets a `MenuView`: `view.panel()` is the panel with its style and its focus group, `view.item(item)`, `view.separator()` and `view.label(text)` build what it holds, and `view.render()` hangs it from the anchor. A renderer for `MenuItem` starts from `view.root()`, the behavior with the root's style, and puts in it `view.icon()`, `view.label()`, `view.shortcut()`, `view.check()`, `view.chevron()` or what it likes; `view.submenu()` is the submenu to put after it. Choosing it still closes its menu.

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
| `size(DialogSize)`, `width(f32)` | `Sm` 360, `Md` 480, `Lg` 640, `Full` the window; 420 without one. Never wider than the window |
| `scrollable(bool)` | The body scrolls between a header and a footer that stay, each set off by a line |
| `close_button(bool)` | A ghost button with the close icon at the end of the header |
| `dismiss_on_veil(bool)`, `dismiss_on_escape(bool)` | Whether a press on the veil, and Escape, close it. Both on by default |
| `inline(bool)` | The panel where it is declared, without the veil and without taking the window: for a gallery, a screenshot, a test |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's; `key` tells two dialogs of one component apart |

- It is the engine's `Modal` with `target(Root)` and `takes_focus(true)`: declared anywhere, it takes no room there and covers the whole window while it is open. As it opens the focus goes to its first control (the close button where it has one), and as it closes back to what had it, the button that opened it. Nothing under the veil takes the pointer, and Tab and Shift+Tab go round the dialog's own controls.
- A press on the panel goes no further: only a press on the veil itself closes the dialog.
- The panel is never larger than the window less the veil's margin of 16. A dialog is as high as its content while that fits, and its body gives way and scrolls when it does not, in either form; `style(DialogPart::Panel, |s| s.height(300.0))` gives it a height of its own.
- Parts: `DialogPart::Veil`, `Panel`, `Header` (the heading and the close button), `Heading` (the title over the description), `Title`, `Description`, `Body`, `Footer`, `Close` (laid over the theme's button), `Separator` (the lines of a scrollable dialog). States: `DialogState::Scrollable`. Sizes: `DialogSize::Sm`, `Md`, `Lg`, `Full`.
- In the design the panel is 420 wide in `Popover` with an edge of `Border`, corners of `radius-xl` and the `lg` shadow, padded 24 with 20 between its heading, its body and its footer; the veil is `Scrim`. The title is 16/600, the description 8 under it in 14 of `MutedForeground`, and the footer's buttons stand at its end 8 apart. The scrollable one has a header of 56 (its title 20 from the edge, the close button 12 from it), higher for a description under the title, and a footer of 60, the body padded 20 and 16 between them: the body is as high as what is in it, and scrolls when the window has no room for all of the dialog. The close button is 28 with an icon of 16.
- To what reads the tree the panel is a `dialog` called by its title.
- A renderer that draws another dialog builds its panel from `view.panel()`, the parts from `view.header()`, `view.body()`, `view.footer()`, `view.close()` and `view.separator()`, and lays it over the window with `view.modal(panel)`; `view.closer()` closes it.

## Tooltip

A short text beside an element while the pointer rests on it or the keyboard's focus is on it: what an icon button does, and the keys that do it too.

```rust
use lurq::ui::{Button, ButtonVariant, Tooltip, TooltipSide};

// In a component's render: a tooltip is mounted on the element it describes, and returns that element.
let copy = Button::new("Copy").variant(ButtonVariant::Ghost).icon("copy").icon_only(true);
let copy = Tooltip::new("Copy").shortcut("Ctrl+C").mount(ctx, copy);

let zoom = Button::new("Zoom in").icon("zoom-in").icon_only(true);
let zoom = Tooltip::new("Zoom in").side(TooltipSide::Bottom).arrow(true).mount(ctx, zoom);

// A disabled button shows its tooltip too: it can say why it is disabled.
let export = Button::new("Export").disabled(true);
let export = Tooltip::new("Nothing to export yet").mount(ctx, export);
```

| Builder | |
| --- | --- |
| `mount(ctx, element)` | Puts the tooltip on the element it describes and returns that element |
| `shortcut(&str)` | The keys of a shortcut, after the text in a chip of their own |
| `side(TooltipSide)` | `Top` (the default), `Bottom`, `Left` or `Right` of the element, centered on it |
| `arrow(bool)` | A triangle from the box to the element |
| `delay(Duration)` | How long the pointer rests before it shows: `TOOLTIP_DELAY`, 500 ms, unless this says otherwise |
| `open(Option<bool>)` | `Some(true)` shows it whatever the pointer and the focus do, `Some(false)` never: for a gallery, a test, a tour |
| `disabled(bool)` | It never shows, and the element is as it would be without it |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of what shows |

- It shows after the pointer has rested on the element for its delay, and at once within `TOOLTIP_SKIP_DELAY` (300 ms) of the pointer leaving another: along a toolbar each button's shows at once, and the one before goes, so one tooltip of the pointer shows at a time.
- Focus from the keyboard shows it at once: the focus of the element or of anything in it (a row with a button in it), read by `ElementRef::focus_visible_within()`.
- The pointer can be moved onto the tooltip to read it: it goes 100 ms after the pointer has left both the element and the tooltip. It also goes when the focus leaves, when the element is pressed, and on Escape; after a press or Escape it stays away until the pointer comes again, or until both the pointer and the focus have left.
- Escape is the tooltip's before anything under it: the tooltip is the top layer that takes Escape, so the key closes the tooltip and not the dialog or the popup it is shown in.
- A disabled element shows its tooltip: a disabled element is not hovered, but it hears the pointer come and go, and the tooltip is on it as on any other.
- It is kept in the window: where there is no room on its side it is on the other one, its arrow turned with it, and where the window pushes it along, its arrow still points at the middle of the element. It takes no focus.
- To what reads the tree (`lurq_snapshot` of the MCP server) the element is described by the text (`a11y_description`), and called by it (`a11y_name`) when nothing else names it, as an icon-only button is named by its label already. What shows is a `tooltip`. An element with an element ref of its own keeps it, and the tooltip is anchored to it.
- It is a mounted component, not a builder that goes where an element goes: it keeps timers for its delay and its grace, an anchor that stays the same from one render to the next, and whether the pointer has shown it, and a builder has no `Ctx` to keep them in. Converted into an element on its own, a `Tooltip` is its box as it shows, anchored to nothing: for a gallery, a test or a host of an app's own.
- Parts: `TooltipPart::Root` (the box and its arrow, and the room between them and the element), `Content` (the box), `Label`, `Shortcut`, `Arrow`. The side is the recipe's variant (`TooltipSide`), and the states are `TooltipState::Arrow`, `Shortcut` and `Multiline` (the text wraps, known once it is laid out).
- In the design the box is in `Primary` with a radius of `Md`, its text in `PrimaryForeground` in the caption's 12, 10 from its sides and 6 from its top and bottom, and no wider than 200; a text that wraps has 8 above and below it, in lines of 1.4. A shortcut is 8 after the text in a chip 16 high, 4 from its sides, with a radius of 3, in a white of 15%, its keys in the mono family in 10. The arrow is 12 along the box and 6 out from it, in the box's color. The tooltip stands 6 from the element: the root's padding on the side's axis.
- The arrow is a square in the box's color turned by 45°, its middle on the edge of the `Arrow` part that meets the box; the part clips it to the half that points away (`TooltipView::arrow()`), moved along the box by `TooltipView::aim`.
- A renderer builds what shows, not when: it is given a `TooltipView` with the text, the shortcut, the side the tooltip is on, whether it has an arrow and its aim; `view.content()` is the default box and `view.arrow()` the default arrow.

## Toast

A short message that something happened, in a corner of the window over everything, that leaves by itself: settings saved, a file moved to the trash, a connection that failed.

```rust
use lurq::ui::{Button, ButtonSize, ButtonVariant, Toast, ToastVariant, Toasts};

// In the root component's create, once, kept in a field of it.
let toasts = Toasts::new();
// In its render, the host last, so that its layer is over every other.
column.child(self.toasts.host(ctx));

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
| `variant(ToastVariant)` | `Neutral` (the default, no icon), `Success`, `Info`, `Warning`, `Error`: the icon and its color |
| `icon(name)`, `icon_hidden(bool)` | An icon of the theme's set in place of the variant's, or none |
| `action(Button \| Link)` | What can be done about it, before the close control |
| `closable(bool)`, `on_close(handler)` | Whether it has its close control (it has by default), and what a press of it calls |
| `timer(bool)` | A rail along the bottom edge that says how long it has left |
| `duration(Duration)`, `persistent(bool)` | How long a `Toasts` shows it (`TOAST_DURATION`, 5 s), or that it stays until it is closed |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the panel; `force` draws the close control |

| `Toasts` | |
| --- | --- |
| `Toasts::new()` | No toasts yet. It needs no `Ctx`: make it once and clone it into whatever shows a toast |
| `show(toast) -> ToastId`, `dismiss(id)`, `clear()` | Shows one in front of the others; takes one away; takes all away |
| `ids()`, `len()`, `is_empty()` | The toasts shown and waiting, the oldest first |
| `host(ctx)`, `host_at(ctx, ToastPosition)` | What draws them: at the bottom right, or at `TopLeft`, `Top`, `TopRight`, `BottomLeft`, `Bottom` |

- A toast is a panel like any other component and can be put anywhere. Sent to a `Toasts`, it is drawn by the host in a layer over the window, clear of its safe area and under the title bar a `WindowChrome` draws: the layer takes a press only on the toasts, and Tab reaches their controls from the page as it reaches any other.
- It leaves after its duration, on a press of its close control or of its action, when a finger throws it to a side or toward the window's edge, and by `dismiss`. While the pointer is on the toasts their time stands still, timers too, and runs on from there when it leaves.
- Several are stacked: the newest in front, an edge of each of two others behind it, on the side away from the window's edge. With the pointer on them they are laid out one over another, 10 apart, the newest nearest the edge: those behind move out from behind the front one, fading in, and move back and fade when the pointer has gone, so nothing jumps and no shadow is left to vanish. At most `TOASTS_SHOWN` (3) are shown, and their time runs, those behind the front one too; the others wait in order.
- A toast comes in from the window's edge, fading in over the theme's slow duration, and leaves toward it over the base duration. One that comes while another is shown comes in over it: the one that was in front stays where it is, and fades under the new one when that is in. With reduced motion it comes and goes at once.
- Parts: `ToastPart::Root` (the panel), `Content` (what is in it above the timer, side by side), `Icon`, `Text` (the title over the description), `Title`, `Description`, `Action`, `Close`, `Timer` (the rail), `Remaining` (what is left of it); and the host's `Viewport` (how far from the window's edges), `List` (the toasts one over another), `BehindNear`, `BehindFar` (the edges behind the front one). State: `ToastState::Action`. Variants: `ToastVariant`.
- In the design the panel is 360 wide, in `Popover` with an edge of `Border`, large corners and the large shadow; 16 inside it are the icon of 16 in the variant's color (`Success`, `Info`, `Warning`, `Destructive`), 12 from the title (14, medium) over the description (13, `MutedForeground`), each going on to the next line where the panel has no room for it, and the close control of 16 in `MutedForeground`. With an action it is 14 high inside, centered on the action, and the title is 2 over the description. The timer is a rail of 2 in `Secondary` with what is left in `Primary`. Toasts are 16 from the window's edges; the edges behind are 24 and 48 narrower than the panel, three quarters and half opaque.
- To what reads the tree the panel is a `status` called by its title, and its close control a `button` called "Close". Tab reaches the action and the close control; Space and Enter press them.
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

// The app holds which is selected, and is told of each other one selected.
Tabs::controlled(filter, [(0, "Overview"), (1, "Activity")], move |index| set_filter(index))
  .variant(TabsVariant::Pill)
  .orientation(TabsOrientation::Vertical);
```

| Builder | |
| --- | --- |
| `Tabs::new(Signal<T>, tabs)` | The selected tab is the one with the signal's value; selecting another sets it. A tab is a `(value, label)` or a `Tab` |
| `Tabs::controlled(value, tabs, select)` | The app holds which is selected: the tab of `value`, and `select` is told the value of each other tab selected |
| `variant(TabsVariant)` | `Default`: a muted rail with the selected tab lifted out of it. `Line`: a row over a line, the selected tab underlined. `Pill`: round tabs, the selected one in the primary fill |
| `orientation(TabsOrientation)` | `Horizontal` (the default): the list along the top, the panel under it. `Vertical`: the list down the start side, the panel beside it |
| `manual(bool)` | The arrow keys move the focus alone, and Enter or Space selects: for panels slow to build. Otherwise the selection follows the focus |
| `label(&str)` | What the list is called to what reads the tree |
| `overflow(TabsOverflow)` | The overflowing form: the list scrolls between two scroll buttons, and More at its end opens a menu of every tab. `TabsOverflow::new(ctx.signal(false))` is made once, in the app's `create` |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `disabled` makes every tab disabled. `id` is the root's; in the overflowing form the scroll buttons are `<id>-back` and `<id>-on` and the More button `<id>-more` |
| `Tab::new(value, label)` | A tab. `icon(name)` before the label, `count(n)` after it, `disabled(bool)`, `separated(bool)` a divider before it, `panel(Fn() -> Element)` what it shows while selected, `id` (its item in the More menu is `<id>-item`) |

- Exactly one tab is selected, and the app holds which. A press of a tab selects it; a press of the selected one or of a disabled one asks for nothing.
- The list is one stop of Tab, at the selected tab. The arrow keys of the orientation (Left and Right, or Up and Down) select the tab after and the one before, Home and End the first and the last, and a disabled tab is passed by; with `manual(true)` they move the focus and Space or Enter selects. Tab from the list goes to the panel, and on to what is in it.
- The panel of a tab is built only while the tab is selected, keyed by the tab, so one tab's panel never takes another's nodes. What the runtime keeps of a panel's nodes (a scroll offset, a caret, an open select) goes when another tab is selected; what the app keeps in its own signals stays. A panel that is to come back as it was keeps its state in the app: its values in signals made in the app's `create`, its scroll in a `ScrollState` it gives the scroll area (`with_scroll_state`).
- The panel is a stop of Tab itself, so a panel with nothing focusable in it is still reached and read; its ring is the focus ring.
- Parts: `TabsPart::Root` (the list and the panel), `List`, `Tab` (what is pressed and focused), `Label`, `Icon`, `Count`, `Divider`, `Panel`, and in the overflowing form `Bar` (the row of the list, its buttons and More), `ScrollButton`, `ScrollIcon`, `More`, `MoreLabel`, `MoreIcon`. A tab's parts are styled in its states: `TabsState::Selected`, `Disabled`; the whole has `Horizontal` or `Vertical`, and `Overflow`. Variants: `TabsVariant::Default`, `Line`, `Pill`.
- In the design the default list is a rail of `Muted`, `radius-lg`, padded 3, its tabs 2 apart; a tab is 28 high, padded 12, `radius-md`, its label 13/500 in `MutedForeground` and a count of 12 in `MutedForeground` 6 after it; the selected tab is `Background` with the small shadow and its label in `Foreground`. The line list is 44 high over a line of 1 in `Border`, as wide as what holds it, its tabs 28 apart; a label is 14 in `MutedForeground` and a count 13 at 0.7, 8 apart; the selected tab is underlined with 2 in `Foreground`, its label 500 in `Foreground` and its count whole. A divider is 1 by 16 in `Border`. Down the side the line list is at least 180 wide with a line of 1 in `Border` at its start, a tab 32 high, padded 12, its label 13, and the selected tab marked with 2 in `Foreground` at its start. The pills are 30 high, padded 14, round, 4 apart, the label 13/500; the selected one is `Primary` with its label in `PrimaryForeground`. The panel is 16 under the list, or 24 beside it.
- The overflowing form is a row 40 high (over the line in the line variant): the list, its tabs 20 apart with labels of 13, between two scroll buttons 28 wide with chevrons of 14 in `Foreground`, and More at the end, set apart by a line of 1 in `Border`, padded 8 and 4, its label 13 and a chevron of 14 in `MutedForeground`. A scroll button moves the list most of the way it shows; it is for the pointer and no stop of Tab, since the arrow keys scroll the list to the tab they select. More opens a menu of every tab (`ui::Menu`), the selected one marked as a radio and a disabled one not to be chosen; choosing one selects it and scrolls the list the least that shows it.
- A tab under the pointer has its label in `Foreground`; a disabled tab is dimmed and does not light up. The focus ring is the tab's; in the overflowing form it is drawn inside the tab, where the list that scrolls does not cut it off.
- To what reads the tree (`lurq_snapshot` of the MCP server) the list is a `tablist` called by `label`, with `a11y_orientation`; a tab is a `tab` called by its label, with `a11y_selected` (which `lurq_inspect` reports as `state.selected`); the panel a `tabpanel` called by its tab. More has `a11y_haspopup` `menu` and `a11y_expanded`.
- A renderer that draws other tabs has `view.tabs` (each with its label, icon, count, and whether it is selected, disabled, set apart), `view.part(part)`, `view.tab_part(index, part)`, `view.tab(index)` (a tab with its behavior and its name to tooling), `view.list()` (the tabs as one focus group), `view.divider()`, `view.panel()` and `view.bar()` (the overflowing form).
- `lurq::components::Tabs` is another thing: a set of places of an app, each a router with a history of its own kept while it is not shown, with a bar to move between them across the bottom or the top. Tabs of the design system are a choice within a page.

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
  one header to the next and round, Home and End to the first and the last. A disabled header is passed by, and so is
  what an open item holds: a button in it is a stop of Tab between its header and the next, and the arrows are not the
  accordion's while the focus is on it (`FocusGroup::none()` around the content).
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

## Breadcrumb

The way to where one is: a step for each place on it, each but the last a link back there, the last the page one is on.

```rust
use lurq::ui::{Breadcrumb, BreadcrumbSeparator, BreadcrumbStep, icons};

Breadcrumb::new()
  .step(BreadcrumbStep::new("Settings").value("/settings"))
  .step(BreadcrumbStep::new("Appearance").value("/settings/appearance"))
  .step("Theme")
  .on_select(|route| go_to(route));

// More steps than four: the first, an ellipsis that opens a menu of the hidden ones, and the last three.
// anchor: ElementRef, open: Signal<bool>, kept by the app as for any menu.
Breadcrumb::new()
  .step(BreadcrumbStep::new("Home").icon(icons::HOME).value("/"))
  .steps(["Projects", "lurq", "crates", "ui", "breadcrumb.rs"])
  .max_items(4)
  .menu(anchor.clone(), open.clone())
  .separator(BreadcrumbSeparator::slash());
```

| Builder | |
| --- | --- |
| `step(impl Into<BreadcrumbStep>)`, `steps(iter)` | Steps, in order, after those it has. The last is the page one is on: text, not a link. A `&str` is a step of that label |
| `on_select(Fn(&str))` | Called with the value of the step followed, after the step's own handlers |
| `max_items(usize)` | The most steps shown. With more, the first, an ellipsis and the last `max_items - 1`; never fewer than two. All of them without it |
| `menu(ElementRef, Signal<bool>)` | What the menu of the hidden steps hangs from, and whether it is open: the app's, as for a [menu](#menu). A press of the ellipsis turns it; choosing a step, a press outside and Escape close it |
| `separator(BreadcrumbSeparator)` | What stands between two steps: `BreadcrumbSeparator::chevron()` (the default), `slash()`, `Icon(name)` or `Text(..)` |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws every link and the ellipsis in the state. The ellipsis's id is the root's with `-ellipsis` after it |
| `BreadcrumbStep::new(label)` | A step. `value(&str)` is what `on_select` is told (its label until then), `icon(name)` an icon before its text, `on_select(Fn())` called when it is followed, `id` the id of its link, of the page, or of its item in the menu while it is hidden |

- A link is followed by a click, and by Enter or Space while it is focused. The links and the ellipsis are stops of Tab in their order; the page is not. Choosing a hidden step in the menu follows it as a press of its link would.
- Parts: `BreadcrumbPart::Root` (the row), `Link` (a step that is a link: what is pressed, with its icon and text), `Page` (the last step), `Icon` (a step's icon), `Separator` and `SeparatorIcon`, `Ellipsis` (what is pressed to open the menu) and `EllipsisIcon`. States: `BreadcrumbState::Collapsed` (some steps are hidden), `Disabled`.
- In the design the steps are 8 apart in the body's 14; the links are in `MutedForeground` and the page in `Foreground` at 500; a chevron of 14 in `MutedForeground` stands between each two. The ellipsis is a square of 24 filled with `Highlight` (the board's `$accent`), with the small radius and an icon of 16 in `Foreground`. Under the pointer a link is in `Foreground`; the ring of the focus is around the link. A step's icon is 14, 4 before its text. A disabled breadcrumb is dimmed, follows nothing and is no stop of Tab.
- A theme gives the separator a renderer of its own with `set_part_renderer::<Breadcrumb, BreadcrumbPartRenderer>(BreadcrumbPart::Separator, ..)`, built once for each place between two steps; or restyles it by the recipe.
- To what reads the tree (`lurq_snapshot` of the MCP server) the root is a `navigation` called "Breadcrumb", each link a `link` called by its label, the page has `a11y_current` `page`, and the ellipsis says `a11y_haspopup` `menu` and whether it is `a11y_expanded`.
- A renderer that draws another breadcrumb has `view.shown()` (the steps to draw, by index), `view.step(index)` (a link with its behavior, or the page), `view.separator()`, `view.ellipsis()` (the button with its menu, while some steps are hidden), `view.root()` and `view.follow(index)` (what following a step does).

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

// The app holds the page: go is told the page of each press.
Pagination::controlled(current, 12, move |to| go(to));
```

| Builder | |
| --- | --- |
| `new(Signal<usize>, pages)` | `pages` pages, of which the current one is the signal's, counted from 1 |
| `for_items(Signal<usize>, total, page_size)` | The pages of `total` items, `page_size` to a page |
| `controlled(current, pages, go)` | The app holds the page: `current`, counted from 1, and `go` told the page of each press |
| `items(total, page_size)` | The pages of so many items, in place of a number of pages; the compact form then says "1–20 of 284" |
| `siblings(usize)` | How many pages are drawn on each side of the current one: 1 unless it is said |
| `compact(bool)` | Previous and next with where the list is, and no pages |
| `page_size(label, Select)` | A select of how many to a page, after its label, at the start; the pagination is then as wide as what holds it |
| `summary(&str)` | What is said of where the list is, in place of "1–20 of 284" or "Page 3 of 12": in another language |
| `label(&str)` | What the whole is called to what reads the tree: "Pagination" unless it is said |
| `on_change(handler)` | Called with the page of each press |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. Its buttons have ids after the pagination's: `{id}-previous`, `{id}-next`, `{id}-page-3` |

- Which pages are drawn is the pagination's: the first and the last always, the current one with its siblings on each side, and an ellipsis where pages are left out. While every page fits in `2 × siblings + 5` every page is drawn; once they do not, that many things are drawn wherever the current page is, so that the row does not change its length as a person goes through it. Near an end the pages run on to that end and take the room of the ellipsis that is not needed there: on page 1 of 20, `1 2 3 4 5 … 20`; on page 10, `1 … 9 10 11 … 20`; on page 20, `1 … 16 17 18 19 20`. An ellipsis always stands for two pages or more. `Pagination::pages_shown(current, pages, siblings)` says what is drawn, as `PageItem::Page(n)` and `PageItem::Ellipsis`.
- A current page past the ends is drawn as the end it is past; the signal is not changed. With no pages, previous and next are drawn, disabled, and nothing between them.
- Previous is disabled on the first page and next on the last. A press of a page goes to it; a press of the current one does nothing.
- Each page, previous and next is a button and a stop of Tab of its own; Left and Right move along the row (`FocusGroup::horizontal()`), from the last on to the first, and Home and End go to its ends. Enter and Space press the button the focus is on. A disabled button is passed by, and one that is disabled while the focus is on it, as next is by Enter on the page before the last, passes the focus to the button before it: the keyboard stays in the pagination.
- The page-size select is the app's `ui::Select`, bound to its own signal; the pagination gives its trigger the board's size over the select's recipe (`PaginationPart::PageSize`, laid over `SelectPart::Trigger`), and calls it by the label beside it.
- To what reads the tree (`lurq_snapshot` of the MCP server) the whole is a `navigation` called "Pagination" (or its `label`), each page a button called "Page 3", and the current one has `a11y_current` "page". Previous and next are called "Previous page" and "Next page".
- Parts: `PaginationPart::Root`, `Nav` (the row of buttons the arrow keys move along), `Item` (laid over each button's root), `Current` (laid over the current page's, after `Item`), `Ellipsis`, `EllipsisIcon`, `Summary`, `PageSizeLabel`, `PageSize` (laid over the select's trigger). States: `PaginationState::Compact`, `PageSize`, `Disabled`.
- In the design the pages are ghost buttons of 36, 4 apart, the current one an outline button; previous and next are ghost buttons of 36 with a chevron of 16. The ellipsis is a box of 36 with `more-horizontal` of 16 in `MutedForeground`. A page button is at least 36 wide and 6 from its sides, so that a number of four digits or more still fits. The compact form has outline buttons of 30 for previous and next, 12 after where the list is, in the body's type at 13 in `Foreground`; its page-size select is 72 by 30 with its text at 13, 12 after its label in `MutedForeground`. A disabled button is dimmed as a button is.
- A renderer is given a `PaginationView`: where the list is (`current`, `pages`, `shown`, `summary`), and `view.previous()`, `view.next()`, `view.page(n)`, `view.ellipsis(i)`, `view.summary()`, `view.nav()` (the row of buttons) and `view.page_size()` (its label and the select) built in the recipe's style with their behavior.

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
| `variant(BadgeVariant)` | `Default` (the primary color), `Secondary`, `Outline` (an edge and no fill), and the statuses `Success`, `Warning`, `Info`, `Destructive` (the status's muted fill and its strong text). A count in `Destructive` is solid |
| `size(BadgeSize)` | `Sm` (18 high, at 11), `Md` (22, at 12, the default), `Lg` (26, at 13) |
| `shape(BadgeShape)` | `Pill` (the default) or `Square`, with the theme's small radius |
| `icon(name)` | An icon of the theme's set before the text |
| `dot(bool)` | A dot in the variant's color before the text; the badge then has an edge and no fill, as the design's "Online" |
| `dot_only(bool)` | A dot alone in the variant's color. Its text is not drawn and is still what it is called |
| `max(usize)` | The most a count shows: past it the maximum and a plus. 99 unless told |
| `show_zero(bool)` | Whether a count of 0 is drawn. It is nothing unless told |
| `label(&str)` | On a count, what it counts, which tooling is told with the number |
| `on(element)` | Puts the badge on the top end corner of `element` |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the badge's own, not the element's it is on |

- A badge is read and not operated: it takes no press, no focus and no stop of Tab, and has nothing to force.
- On an element (`on`), the badge and the element are one element as large as the element; the middle of the badge is on the point 3 in from the element's top end corner, whatever the badge's width, so a dot of 12 is at 27, -3 on a box of 36 (the design's Indicator), a count of one digit reaches 6 past the corner each way, and a wider one as far to either side of the point. It has a ring of 2 in `Background` that parts it from what it is on. It reaches past the element, so what holds them gives it room or does not clip (`overflow_visible()`). A count that is not drawn leaves the element alone.
- Parts: `BadgePart::Root` (the badge: its fill, edge and height; a dot alone is its root), `Label`, `Icon`, `Dot` (the dot before the text), `Anchor` (what holds the element and the badge on it) and `Corner` (a square at the element's top end corner whose centre the badge's round end is centred on: its width is all that is read of it). States: `BadgeState::Dot`, `DotOnly`, `Count`, `OnCorner`, `Square`. The variant and the size are the recipe's.
- In the design a badge is 22 high and round, padded 8, its text 12/500 (the caption's type at medium), an icon of 12 and 4 before the text; small is 18, padded 6, at 11 with an icon of 10; large is 26, padded 10, at 13 with an icon of 14. With a dot it is in `Background` with a 1 px edge of `Border` and its text in `Foreground`, the dot 7, 6 before the text, the badge padded 8 before and 10 after. A count is at 600 and at least as wide as it is high. A dot alone is 8 (6 small, 10 large), and 12 on a corner (10 small, 14 large).
- To what reads the tree (`lurq_snapshot` of the MCP server) a badge is a `status` called by its text; a count is called by its number with what it counts ("120 unread messages"), the whole number however it is shown.
- A renderer that draws another badge has `view.root()` (the root with its style and its name to tooling), `view.badge()` (the default badge), `view.text` (what is drawn: "99+"), `view.name()`, `view.shown` (false for a count of 0 not drawn), and `view.place(element, badge)` with `view.corner_offset()` to put it on `view.on`.

## Tag

A label on a thing, a keyword or a category, that the app may let a person take away.

```rust
use lurq::ui::{Tag, TagSize, TagVariant, icons};

Tag::new("design");
Tag::new("in review").variant(TagVariant::Info).icon(icons::CIRCLE);

// With the button that takes it away: the app holds the list.
Tag::new("rust").on_remove(move || topics.update(|all| all.retain(|t| t != "rust")));

// Round, with no line: the token of a box that holds several values.
Tag::new("React").pill(true).on_remove(move || remove("React"));
```

| Builder | |
| --- | --- |
| `variant(TagVariant)` | `Neutral` (the default), `Info`, `Success`, `Warning`, `Destructive`, or a variant the theme's recipe names |
| `size(TagSize)` | `Sm` (20 high), `Md` (24, the default), `Lg` (28) |
| `icon(name)` | An icon before the text, by its name in the theme's set |
| `on_remove(handler)` | Gives the tag its remove button, and is called on a press of it |
| `pill(bool)` | Round at both ends, with no line around it |
| `disabled(bool)` | Dimmed; its remove button is not pressed and Tab passes it by |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the root's; `force` draws the remove button |

- A tag does not keep whether it is there: the app holds its list, and `on_remove` tells it which one a person took away. A tag with no handler is read only: it has no button and is no stop of Tab.
- The remove button is a button of its own, called "Remove" and the tag's text ("Remove rust") to what reads the tree. Tab reaches it, and a press, Enter, Space, Backspace and Delete press it; Backspace and Delete only while the focus is on it, with no modifier held.
- Tags an app lays in a row with a focus group (`Row::new().focus_group(FocusGroup::horizontal())`) keep the focus when one is taken away: it goes to the next tag's button, or to the last, so Backspace takes them one after another. Outside a group the focus leaves with the button.
- Parts: `TagPart::Root`, `Icon`, `Label`, `Remove` (the button), `RemoveIcon` (the cross: its side). States: `TagState::Removable` (it has the button), `Pill`, `Disabled`.
- In the design a tag is 24 high, padded 8, with the corners of `RadiusSize::Sm`, in `Secondary` with a line of `Border` inside it, its text 12 and medium in `SecondaryForeground`. A removable one is padded 4 at its end, where a button of 16 (corners of 3) holds a cross of 11 in `MutedForeground`, 4 after the text; under the pointer the button is filled `StateHover` and its cross is `Foreground`. `Info`, `Success`, `Warning` and `Destructive` are in the muted fill of their color, line and fill alike, with the text in the color. A pill is round, has no line, is padded 10 and 6, and its cross is 12.
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
| `ChipGroup::controlled(selected, choices, press)` | The app holds what is chosen, and `press` is told each value pressed |
| `disabled(bool)`, `style(part, ..)`, `id`, `key` | `ChipGroupPart::Item` lays rules over each chip's root |

- Tab reaches a chip, Space and Enter turn it, and the focus ring is the chip's. A group is one stop of Tab: the arrow keys move between its chips and Space or Enter turns the one the focus is on. A choice is a [`Choice`](#toggle-group), as a toggle group's: its icon is the chip's.
- To what reads the tree a chip is a `button` called by its text, with `a11y_pressed` saying whether it is chosen; `lurq_inspect` reports it as `state.pressed`.
- Parts: `ChipPart::Root` (what is pressed), `Icon`, `Label`; `ChipGroupPart::Root`, `Item`. States: `ChipState::Selected`, `Leading` (it has an icon before its text: the check or its own), `Disabled`.
- In the design a chip is 28 high and round, padded 12, its text 13 and medium. One not chosen is in `Background` with a line of `Input` and its text in `Foreground`, `Highlight` under the pointer; a chosen one is in `Primary`, line and fill, its text in `PrimaryForeground`, with a check of 14 before it, 6 from it and padded 8 at its start, and in `PrimaryHover` under the pointer. A disabled chip is dimmed. The chips of a group are 6 apart and go on to the next line where the row has no room.
- A renderer that draws another chip starts from `view.root()`, the behavior with the root's style; `view.leading()` is the icon the default renderer puts before the text.

## Avatar

Who a person is, at a glance: the initials of the name on a fill, a picture in their place, whether the person is there.

```rust
use lurq::{components::Image, ui::{Avatar, AvatarPresence, AvatarShape, AvatarSize, icons}};

Avatar::new("Ivan Zaida");
Avatar::new("Ivan Zaida").image(Image::from_file("ivan.png")).presence(AvatarPresence::Online);
Avatar::new("Ivy Chen").size(AvatarSize::Xl);

// What is not a person: an icon in place of the initials, in a square.
Avatar::new("Build bot").icon(icons::TERMINAL).shape(AvatarShape::Square);
```

| Builder | |
| --- | --- |
| `new(&str)` | Who it is. The initials are the first letters of the first word and the last: "Ivan Zaida" is "IZ", "Ivan" is "I". A name with no letters shows the icon of a person |
| `image(Image)` | A picture in place of the initials, which fills the face, cropped to it (`cover`) and clipped to its shape: the engine's image element as the app builds it, from pixels (`Image::new`) or from a file or bytes it decodes (`Image::from_file`, `Image::from_bytes`, feature `image`), with a cache key and a size to decode to where it wants them. Feature `raster` |
| `icon(name)` | An icon of the theme's set in place of the initials, for what is not a person: a bot, a team |
| `size(AvatarSize)` | `Xs` 20, `Sm` 24, `Md` 32 (the default), `Lg` 40, `Xl` 56, or a size the theme's recipe names |
| `shape(AvatarShape)` | `Circle` (the default), or `Square` with the corners of a control |
| `presence(AvatarPresence)` | The dot at the bottom end of the face: `Online`, `Away`, `Busy`, `Offline`; `None` for none |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the root's |

- While a picture decodes and when it fails (a broken file, a format the decoder does not read), the avatar shows its initials or its icon: they are the image's placeholder and its error view. A picture with clear pixels shows the face's fill through them.
- An avatar is read, not operated: it takes no press and is no stop of Tab. To what reads the tree (`lurq_snapshot` of the MCP server) it is an `img` called by its name, with its presence after it: "Ivan Zaida, away".
- Parts: `AvatarPart::Root` (the face and the dot, as large as the face), `Face` (the shape: side, corners, fill, the ring of a group; it clips what it holds), `Initials`, `Icon` (its width is the icon's side), `Image`, `Presence` (the dot). States: `AvatarState::Square`, and the avatar's `AvatarPresence` under its own name (`.state(AvatarPresence::Away, AvatarPart::Presence, ..)`). Sizes: `AvatarSize`.
- In the design an avatar is a circle of 32 in `Muted`, its initials 12 and medium in `Foreground`; at 20, 24, 40 and 56 they are 9, 10, 14 and 18. A square one has the corners of `RadiusSize::Md`. The icon is half the side, in `MutedForeground`. The dot is 10 on a face of 32 (6, 8, 12 and 14 on the others), at the face's bottom end corner, round, ringed 2 outside it in `Background`: `Success` online, `Warning` away, `Destructive` busy, `MutedForeground` offline.
- The ring of the dot stands outside the avatar's box, as the rings of a group do: what holds an avatar at its edge clips them, as a container clips what is in it, unless it leaves 2 of room or is `overflow_visible()`.
- A person with a name and a line under it (the board's Persona) is an avatar beside two texts, built where it is used: `Row::new().spacing(10.0).child(Avatar::new(name).presence(..)).child(Column::new().spacing(1.0).child(name).child(line))`, the name 14 and medium, the line 12 in `MutedForeground`.
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
AvatarGroup::new(people.map(Avatar::new)).max(3).label("Reviewers");

// On a card, the ring is the card's color.
AvatarGroup::new(people.map(Avatar::new))
  .size(AvatarSize::Sm)
  .style(AvatarGroupPart::Item, |s| s.border(Border::outside(2.0, PaletteColor::Card)));
```

| Builder | |
| --- | --- |
| `new(avatars)` | The avatars, in order: each lies over the end of the one before it |
| `max(usize)` | The most avatars it shows; after them a last one says how many it leaves out, "+3" |
| `size(AvatarSize)` | The size of every avatar in it |
| `label(&str)` | What it is called to what reads the tree. Without one, the names it shows and how many more |
| `style(part, ..)`, `id`, `key` | As a [button](#button) has them |

- The group gives its avatars its size and what its recipe says of an item; each keeps everything else it was given: its picture, its icon, its presence.
- To what reads the tree it is a `group` called by its label; the count is an `img` called "3 more".
- Parts: `AvatarGroupPart::Root` (the row: how far each avatar lies over the one before, as a gap below nothing), `Item` (laid over each avatar's `Face`: its ring), `More` (laid over the count's `Face`, after `Item`). Sizes: `AvatarSize`.
- In the design avatars of 32 lie 8 over one another, each ringed 2 outside its face in `Background`, the color of the surface behind them; the count is in `Secondary`, its text as the initials. Smaller and larger avatars lie a quarter of their side over each other.
- A renderer that draws another group has `view.avatars` (ready, with the group's size and rules), `view.hidden` and `view.more()`, the count as an avatar.

## Card

A surface that holds one thing: a group of settings, a summary of a project, a form of a few fields.

```rust
use lurq::ui::{Button, ButtonVariant, Card, CardSize, CardVariant};

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
```

| Builder | |
| --- | --- |
| `title(&str)` | What the card is about, 16/600. It is what the card is called, and wraps in the width the card has |
| `description(&str)` | More of it, under the title, 14 in the muted color |
| `action(element)` | At the end of the header, beside the title: a button, a menu, a badge |
| `child(element)`, `children(..)` | The content, under the header |
| `footer(element)` | Added to a row at the end of the card, under a line: its least action first and its main one last |
| `variant(..)` | `CardVariant::Outline` (the default), `Elevated`, `Filled`, or a variant the theme's recipe names |
| `size(..)` | `CardSize::Md` (the default) or `Sm`, or a size the theme's recipe names |
| `width(..)` | A width of its own. Without one the card is as wide as the room it is given |
| `on_click(handler)` | Makes the card pressed as a whole, from the pointer and the keyboard |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the surface |

- Every part is optional: a card with children alone is a padded surface. The header is there where the card has a title, a description or an action; the footer where it has something in it, set off by a line only where there is a body over it.
- A card is as wide as the room it is given, as a block is. Cards side by side in a row are each given a share of it: a `Column::new().flex(1.0)` around each, or a grid. The title and the description wrap in the room the action leaves them.
- A card given `on_click` is pressed as a whole: one stop of Tab, with the focus ring around it, pressed by a click, Enter and Space. The controls in its action and its footer are stops of Tab of their own, after the card, and a press of one of them is that control's and not the card's; a control in its content is pressed with the card, so a pressed card keeps its controls in the action and the footer. A card without `on_click` is read: it is no stop of Tab, and each control in it is a stop of its own.
- Parts: `CardPart::Root` (the surface; what is pressed), `Body` (the header over the content, padded), `Header` (the heading and the action), `Heading` (the title over the description), `Title`, `Description`, `Action`, `Content`, `Footer`. States: `CardState::Pressable`, `Disabled`, `Footed` (it has a footer under a body; the line over the footer is the body's `underline`).
- In the design a card is in `Card` with `CardForeground` ink and corners of `radius-xl`. The outlined card has an edge of 1 in `Border` and the small shadow, the elevated one the medium shadow and no edge, the filled one `Muted` and no edge. Its body is padded 24 with 24 between the header and the content; the title is 16/600, the description 6 under it in 14 of `MutedForeground`; what is in the content is 16 apart; the footer is padded 16 and 24 with its buttons 8 apart at its end. A small card is padded 16, with a title of 14 and a description of 13. A pressed card takes an edge of `Ring` and the medium shadow under the pointer, and the small shadow again while it is held down; a disabled one is dimmed.
- The settings card and the stat card of the design are cards with their parts styled: `style(CardPart::Body, |s| s.padding_x(14.0).padding_y(14.0))`, `style(CardPart::Root, |s| s.radius(RadiusSize::Lg).shadow(BoxShadowValue::none()))`, and what they hold as the content.
- To what reads the tree (`lurq_snapshot` of the MCP server) a card is a `group` called by its title, and a pressed card a `button`.
- A renderer that draws another card (`CardRenderer`) starts from `view.read_root()` or `view.pressed_root()`, the surface with its behavior, and puts in it what it likes: `view.body()`, `view.header()`, `view.heading()`, `view.action()`, `view.content()` and `view.footer()` are the default ones.

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
| `Separator::horizontal()`, `vertical()`, `new(orientation)` | A line across a column, or down a row |
| `label(&str)` | A word in the middle, with the line on either side of it |
| `length(f32)` | How long it is: its width across a column, its height down a row |
| `decorative(bool)` | Whether tooling passes it by. On by default |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- A separator across a column is as wide as the room it is given. One down a row is as high as the row where the row stretches its children (`align_items(Alignment::Stretch)`); in a row that does not, it is given a `length`. An inset is the root's padding: `style(SeparatorPart::Root, |s| s.padding_x(16.0))`.
- Parts: `SeparatorPart::Root` (the line, or the lines and the label), `Line`, `Label`. Its orientation is the variant of its recipe: `SeparatorOrientation::Horizontal`, `Vertical`.
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
```

| Builder | |
| --- | --- |
| `kind(SpinnerKind)` | `Ring` (the default), `Dots` or `Bars` |
| `size(name)` | `SpinnerSize::Sm` (16, the default), `Md` (24), `Lg` (32); the dots and the bars grow by a half and by twice |
| `variant(name)` | `SpinnerVariant::Current` (the color of the text around it, dots and bars too), `Muted`, `Primary` |
| `label(&str)` | What is being done, drawn 8 beside it, and what it is called |
| `label_hidden(bool)` | The label is its name and is not drawn |
| `decorative(bool)` | No status of its own, where what it is in says what is under way: a button that loads |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- To what reads the tree a spinner is a `status`, called by its label, or "Loading" when it has none.
- The ring is the glyph a [button](#button) turns while it loads, and `Button::loading` draws a `Spinner`: a rule a theme lays over the spinner reaches the button's. It turns with `ui::SPIN` once in `SPIN_PERIOD`. The dots fade one after another with `ui::SPINNER_DOTS`, the bars shrink about their middle one after another with `ui::SPINNER_BARS`; all of them play on the animation clock, so every spinner on screen moves as one. A spinner moves whether or not motion is reduced: one that stood still would say nothing.
- A spinner is drawn in the color of the text around it, so a spinner in a button is the color of its label: the ring as text is, and the dots and the bars by the fill `BackgroundColor::CurrentForeground`. Where nothing around it names a color, that is `Foreground`. A variant gives it a color of its own.
- Parts: `SpinnerPart::Root` (the spinner and its label), `Indicator` (the ring as a square of its side, or the row of the dots or of the bars), `Dot`, `Bar`, `Label`. States: `SpinnerState::Dots`, `Bars`.
- In the design the ring is 16, 24 or 32; the dots are 8, 6 apart, in a row 16 high, at 1, 0.6 and 0.3 of their color at rest; the bars are 3 wide and round, 3 apart, 8, 18, 12, 20 and 10 high in a row of 20; the label is 13 in `MutedForeground`.
- `LoadingOverlay::new(loading, content)` lays a veil over `content` while `loading` is true: the background's color at 85 hundredths, with a spinner in its middle (`.spinner(Spinner)` gives one of the app's). Meanwhile no press reaches the content and Tab passes it by. The veil fades in with `ui::FADE_IN` and out with `ui::FADE_OUT` over the theme's base duration. The overlay is as large as its content; its root clips the veil, so a rule that rounds `LoadingOverlayPart::Root` rounds the veil with a rounded content. Parts: `Root`, `Content`, `Veil`, and `Scrim`, whose background and opacity are the veil's fill. State: `LoadingOverlayState::Loading`.
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
| `animation(SkeletonAnimation)` | `Pulse` (the default), `Shimmer` or `None` |
| `Skeleton::status(label, content)` | A `status` called `label` around a group of skeletons |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- A skeleton is nothing to tooling that reads the tree: nothing of it is described, so it is left out as any box with no text, id or handler is. What is loading is said once, by `Skeleton::status` around a group of them.
- A skeleton pulses with `ui::SKELETON_PULSE`, from whole to half and back in `SKELETON_PULSE_PERIOD`; one that shimmers has a sheen as wide as itself cross it in `SKELETON_SHIMMER_PERIOD`, from wholly before its start to wholly past its end, whatever its width: in a row three times the skeleton's width centred on it, the room before the sheen grows from nothing to two thirds of the row (`ui::SKELETON_SHIMMER`, a width in percent). Both play on the animation clock, so every skeleton on screen pulses as one, and the sheen crosses all of them at once. Where motion is reduced a skeleton stands still.
- Parts: `SkeletonPart::Root` (the box of a shape, or the column of a paragraph's lines), `Bone` (a shape), `Sheen` (the fill of its middle; it is as wide as the bone). The shapes are the variants of `SkeletonShape`: `Line`, `Circle`, `Block`, `Paragraph`.
- In the design every shape is in `Muted`; a line is 12 high and `radius-sm`, a circle `radius-full`, a block 200 by 112 and `radius-lg`; a paragraph's lines are 10 apart, its last two 92 and 60 hundredths of its width. The sheen runs from the shape's fill to `SecondaryHover` and back.
- The board's media row and card are built from these and are no components of their own: the gallery's Skeleton page has both, with their code.

## Progress

How far a piece of work is: a file uploading, items being copied. A bar along a rail, the thin bar across the top of a window, or a ring with the value in its middle.

```rust
use lurq::ui::{Progress, ProgressSize, ProgressVariant, RingProgress};

// A fraction of one.
Progress::new(0.62).label("Uploading theme.json").show_value(true).size(ProgressSize::Md);

// 42 of 128 items, said as the app says it.
Progress::new(42).max(128).show_value(true).format(|done, all| format!("{done} of {all}"));

// Work whose end is not known.
Progress::indeterminate().label("Connecting");

// The thin bar across the top of a page, first in the page's column.
Progress::new(loaded.get()).top_bar(true);

// The same value around a circle, with it in the middle.
RingProgress::new(0.7).variant(ProgressVariant::Success);
```

| Builder | |
| --- | --- |
| `Progress::new(value)` | How far the work is: a fraction of one, or of `max`. Kept between nothing and `max` |
| `Progress::indeterminate()` | Work whose end is not known: a segment crosses the rail again and again |
| `value(n)`, `max(m)` | How far the work is, and the whole of it (1 when not given). A `max` of nothing or less: nothing is done |
| `variant(ProgressVariant)` | The color of what is filled: `Default` (primary), `Success`, `Warning`, `Destructive` |
| `size(ProgressSize)` | The rail's height: `Sm` 4, `Md` 6, `Lg` 8 (the default). A ring of 40, 56 or 72 |
| `label(&str)` | What the work is, over the start of the rail, and what the progress is called |
| `show_value(bool)` | The value over the end of the rail, as a percentage or as `format` says it. None while it is not known. A ring shows it unless told not to |
| `format(Fn(f32, f32) -> String)` | Says the value, given the value and `max`: "42 of 128" |
| `top_bar(bool)` | The thin bar across the top of a window or a surface: 3 high, no rail, as wide as what holds it |
| `width(impl Into<Length>)` | How wide the progress is: 240 when not given |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the root |

- A progress is read, not operated: it takes no focus and no press. What is filled is a share of the rail, and moves to a new value over the theme's slow duration; while motion is reduced it is there at once.
- Work that is not known runs a segment of 90 of 240 across the rail, growing out of its start and leaving at its end, once in `ui::PROGRESS_PERIOD` (1.6 s), in phase with every other such bar. While motion is reduced the segment stands still 70 of 240 from the start of the rail, where no value would be, as the board draws it.
- Parts: `ProgressPart::Root` (the header over the rail), `Header` (the label and the value), `Label`, `Value`, `Track` (the rail), `Fill` (what is filled; the moving segment). States: `ProgressState::Indeterminate`, `TopBar`.
- In the design the rail is 240 by 8, round, in `Secondary`, filled in `Primary`, or `Success`, `Warning`, `Destructive`. Under a label and in a dialog it is 6, in a row of a list 4. The label is 8 over the rail in the label's type (13, medium) and `Foreground`; the value is at the end in the mono type of 11, `MutedForeground`. The top bar is 3 high with no rail, its fill square where it starts and round 2 where it ends.
- A ring (`RingProgress`) is 72 across, the ring a tenth of that wide, in `Secondary`, with what is done over it in the variant's color from the top and clockwise, and the value in the middle in the label's type of 13 at a weight of 600 (12 and 11 in the rings of 56 and 40). Nothing is painted in its middle, so it sits on any surface. It takes `value`, `max`, `variant`, `size`, `label` (its name to tooling: the board draws no text beside it), `show_value`, `format`, `state`, `style`, `rules`, `id` and `key`. It always knows the value: what does not is a spinner's.
- The ring is two round boxes one over the other: the track, whose inside border is the ring, and the fill, whose inside border, as wide and in the arc's color, is one dash as long as the arc with a gap of the rest of the circle. A border's dashes start at the top of a circle and run clockwise, and their period is fitted to the outline, so a dash and a gap that make the circle are one dash of it. The arc moves round to a new value, and fades in from nothing, over the theme's slow duration (`Transition::border_dash()`, `border_color()`); while motion is reduced it is there at once. Parts: `RingProgressPart::Root`, `Track` (its border is the ring: its width and the color of what is not done), `Fill` (its border's color is the arc's), `Middle` (holds the value, paints nothing), `Value`.
- To what reads the tree (`lurq_snapshot` of the MCP server) a progress and a ring are a `progressbar` called by their label, with `a11y_valuenow`, `a11y_valuemin` (0), `a11y_valuemax` and `a11y_valuetext` (the value as it is said); one that does not know how far the work is has no value.
- A renderer that draws another progress has `view.fraction()`, `view.value()`, `view.max()`, `view.value_text()`, `view.root()` (the root with its name to tooling), `view.header()` and `view.track()` (the rail with what is filled, or with the moving segment). A meter is the same parts restyled: `Progress::new(96).max(128).label("Disk space").show_value(true).variant(ProgressVariant::Warning).format(..)` with a radius of 2 on the `Track` and the `Fill`; a meter of cells is a renderer that draws a row of them from `view.fraction()`. A ring's renderer has `view.fraction()`, `view.value_text()`, `view.side()`, `view.thickness()`, `view.root()`, `view.track()`, `view.arc()` (the fill with its dash) and `view.middle()`. A gauge is such a renderer: half a ring, its arc's box turned a quarter back so that the dash starts at 9 o'clock, in a box that clips the lower half.

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
