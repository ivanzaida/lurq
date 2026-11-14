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
- In a [toolbar](#toolbar) a button is a ghost one of the bar's size (unless its item is `as_given`), and an icon alone has a tooltip of its label. Where the bar is too narrow for it, it goes into the bar's menu as an item of its label, its icon, its id with `-item` after it, disabled where the button is disabled or loading; choosing the item calls the button's `on_click` handlers with the click Enter would make (`from_keyboard`, at 0, 0).

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

In a [toolbar](#toolbar) a toggle takes the bar's size, and an icon alone has a tooltip of its label. Where the bar is too narrow for it, it goes into the bar's menu as a checked item, on while the toggle is: its label, its icon and its id with `-item` after it. Choosing the item calls `on_change` with the other state, as a press would.

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

In a [toolbar](#toolbar) the group takes the bar's size and is no stop of its own: its toggles are among the bar's controls, and the bar's Left and Right move through them and on. Where the bar is too narrow for it, the whole group goes into the bar's menu, after a line, as a checked item for each toggle, on while it is; choosing one presses that toggle.

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
- What a menu holds, in order, is a `MenuEntry`: `Item(MenuItem)`, `Separator`, or `Label(text)`, as `MenuView::entries` has them. A `MenuItem` converts into one with `.into()`, for a list that mixes items with separators and labels, as a [menubar](#menubar)'s `menu(title, entries)` takes.
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
| `reveal_labels(show, hide)` | What the eye is called to tooling, in place of "Show password" and "Hide password" |
| `strength(Option<PasswordStrength>)` | The meter under the input: `Weak`, `Fair`, `Good`, `Strong` fill 1 to 4 of its bars |
| `strength_message(&str)` | What the meter says, in place of "Weak password" to "Strong password" |
| `new_password(bool)` | The password is being chosen, not entered: the platform may offer a strong one |
| `input_rules(&recipe)` | Rules over the theme's recipe for the input it is made of |
| `placeholder`, `icon`, `size`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `full_width`, `on_input`, `name` | As an [input](#input) has them |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the control |

- It is an [input](#input) in every other way: its box, its sizes, its judgments, a field's label. The reveal is the app's, as a switch's state is: a `bool` and a handler, or a `Signal<bool>` bound with `bind_reveal`. The eye is drawn where something hears it.
- Tab reaches the input and then the eye, which Space and Enter press. To tooling the eye is a `button` called "Show password" or "Hide password", with `a11y_pressed`.
- Masked, the password is not copied or cut, the platform's keyboard keeps it out of its suggestions, and tooling (`lurq_snapshot`, `lurq_set_value` of the MCP server) reads only its dots and `masked=true`. Shown, it is what the screen shows, to tooling too. Shown or not it holds a password to the platform (`ContentKind::Password`, or `NewPassword`), which may offer to fill it in.
- Parts: `PasswordInputPart::Root` (the input, the meter and its message, one under another), `Mask` (the room between the dots), `Toggle` (the eye), `Strength` (the meter), `Bar` (a bar the strength does not reach), `Filled` (one it does), `Message`. States: `PasswordInputState::Revealed`, `Disabled`, `FullWidth`, and the strength by its name (`weak`, `fair`, `good`, `strong`). The box, its text and its placeholder are the input's parts, styled by the theme's recipe for an input or `input_rules`.
- In the design the dots are 1 apart; the eye is 16 in `MutedForeground`, 12 from the end of the box and 8 after the text, in `Foreground` under the pointer; while the password is shown it is an eye with a line through it (`icons::EYE_OFF`), in `Foreground`. The meter is 8 under the box: four bars 4 high and 4 apart, round, in `Input` where the strength does not reach and in its color where it does, `Destructive` for weak, `Warning` for fair, `Success` for good and strong; the message is 8 under it in a caption's type in the same color.
- A renderer that draws another password input has `view.input` (the input's view: `render_with_end` is crate-private, so it lays the eye over `view.control()` itself), `view.control()` (the masked text input with room at its end), `view.toggle()`, `view.strength_meter()` and `view.message()`.

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
| `clear_label(&str)` | What the clear button is called to tooling, in place of "Clear search" |
| `input_rules(&recipe)` | Rules over the theme's recipe for the input it is made of |
| `placeholder`, `size`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `full_width`, `on_input`, `name` | As an [input](#input) has them |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the control |

- While it holds something a clear button is at its end, which empties it and takes no focus, so the caret stays in it; Escape does the same while the focus is in it, and keeps the focus there. Escape in an empty field does what it does in any input. A read-only or disabled field has no clear button, and Escape does not empty it.
- The clear button is no stop of Tab: from the keyboard Escape does what it does. The hint shows while the field is empty; the field does not bind the key, which the app does and focuses the field on (`ctx.focus`).
- To tooling the field is a `searchbox` and its button a `button` called "Clear search".
- Parts: `SearchFieldPart::Root` and `Control` (what the field says of the input's root and box: its width, its padding), `End` (what is at the end of the box), `Clear`, `Shortcut` (the hint). States: `SearchFieldState::Filled`, `FullWidth`. The box, its text, its placeholder and its icon are the input's parts.
- In the design the field is 280 by 36, padded 10 at its start and 8 at its end; the search icon is 16 in `MutedForeground`, 8 before the text. The clear button is a circled cross of 16 in `MutedForeground`, `Foreground` under the pointer. The hint is 20 high, padded 6, 4 round on `Muted` in an edge of 1 in `Border`, in the mono type of 11 in `MutedForeground`.
- The field renders again as it fills and empties, to show the clear button or the hint: it reads its signal as it is built.
- A renderer that draws another search field has `view.input`, `view.control()` (the text input with its keys and room at its end), `view.clear()` and `view.shortcut()`.

## Input Group

An input joined with what stands before and after it into one box: a scheme, a domain, a unit, an icon, the button that sends it.

```rust
use lurq::ui::{Button, ButtonVariant, Input, InputAddon, InputGroup, Select, icons};

InputGroup::new(Input::new(site.clone())).before("https://").after(".com");

InputGroup::new(Input::new(email.clone()).placeholder("Email"))
  .before(InputAddon::icon(icons::MAIL))
  .after(Button::new("Invite").variant(ButtonVariant::Ghost).on_click(invite));

InputGroup::new(Input::new(site.clone())).before(Select::new(scheme.clone(), [("https", "https://"), ("http", "http://")]));
```

| Builder | |
| --- | --- |
| `InputGroup::new(input)` | A group around an [input](#input), which keeps its own value, handlers and id |
| `before(addon)`, `after(addon)` | What stands before and after the input, in order: a text (`&str`), `InputAddon::icon(name)`, a `Button`, a `Select` |
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
NumberInput::new(quantity.clone()).min(1.0).max(99.0).label("Quantity");

// A price with two decimals, its thousands set apart, the buttons stacked at the end.
NumberInput::new(price.clone())
  .min(0.0)
  .step(0.5)
  .precision(2)
  .format(NumberFormat::new().prefix("$").grouping(true))
  .layout(NumberInputLayout::Stacked);

// A number that may be left out: an emptied field is None, not 0.
NumberInput::optional(limit.clone()).placeholder("No limit");

// The app's own value, told back.
NumberInput::controlled(seats.get()).on_change(move |value| seats.set(value));
```

| Builder | |
| --- | --- |
| `NumberInput::new(Signal<f64>)` | Bound to a number there always is: an emptied field takes back the number it had when it is left |
| `NumberInput::optional(Signal<Option<f64>>)` | Bound to a number that may be none: an emptied field sets `None` |
| `NumberInput::controlled(Option<f64>)` | Shows what the app gives it and tells each change to `on_change` |
| `min(f64)`, `max(f64)` | The ends of the range, where it has them. With no least value a minus can be typed and Home moves the caret; with no greatest End does |
| `step(f64)` | How far apart the values a step takes it to are, from the least (from 0 with none): 1 when not given |
| `precision(usize)` | How many decimals the value keeps and shows: as many as the step has when not given. With none no decimal separator is taken |
| `format(NumberFormat)` | How the number is written out of the focus: `prefix`, `suffix`, `grouping`, `separators(decimal, group)`, or `NumberFormat::custom(format, parse)`, the app's own |
| `layout(NumberInputLayout)` | `Split` (the board's: a minus at the start, a plus at the end, the number in the middle), `Stacked` (a chevron up over one down at the end, the number at the start), `Bare` (no buttons) |
| `size(NumberInputSize)` | `Sm` 32, `Md` 36, `Lg` 40 high, the type 13, 14, 15 |
| `placeholder(&str)` | What an empty input says |
| `invalid`, `warning`, `valid`, `read_only` | As an [input](#input) has them. A read-only one is focused and selected, and its keys and buttons change nothing |
| `on_change(Fn(Option<f64>))` | Called with the value each time it changes |
| `name(..)` | The name of its text in a form's data. A form's number is bound with `NumberInput::new(form.number("seats"))` |
| `label`, `full_width`, `disabled`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As an [input](#input) has them. `id` is the id of the box that is typed in |

- What is typed is kept as text while it is typed, and is the value as soon as it is a number. A minus (first, where the range has room below 0), digits and one decimal separator (where the precision keeps decimals) are taken; anything else is not. A number pasted as it is written (`$2,000`) is read and typed as the number. On Enter and when the focus leaves, the text is put right: the number rounded to the precision, kept in the range and written in the format; what is not a number goes back to the value.
- In the focus the number is typed plain, with the decimal separator and nothing around it; out of it, it is written in the format. A locale's decimal separator is the format's: `NumberFormat::new().separators(',', '.')`.
- Up and Down take a step, Page Up and Page Down ten, Home and End go to the least and the greatest value where the range has them. A step from a value between two steps goes to the next one; from an empty field it goes to 0, or to the end of the range nearer to it. A wheel over the input scrolls what it is in and leaves the number as it is.
- The buttons take a step when pressed, and while held another after 400 ms and then every 50 ms. They are not stops of Tab and take no focus: a press leaves the focus on the number being typed. The one toward an end of the range the value is at is disabled and dimmed (states `AtMin`, `AtMax`).
- Parts: `NumberInputPart::Root` (its width), `Control` (the box that is typed in, which holds the ring of the focus), `Placeholder`, `Buttons` (what holds the buttons over the box: its inset and corners), `Decrease`, `Increase`, `Divider` (the line between a button and the number), `Icon`. Variants: `NumberInputLayout`. States: `NumberInputState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `FullWidth`, `AtMin`, `AtMax`.
- In the design the box is 132 by 36, an input's box (the `Background` fill in an edge of 1 in `Input`, 6 round), its number in the middle in the mono type at 14 in `Foreground`. The minus and the plus are 36 wide at the ends, each with a line of 1 in `Input` between it and the number, their glyphs 16; under the pointer a button is in `Highlight`. The ring's color under the pointer, a line of 2 inside the box in the focus, and the judgments' colors are the input's.
- The board's mobile quantity stepper is the number input restyled: rules make the `Control` 44 high, round, in `Secondary` with no edge, the number in the sans type at 16, semibold; the `Buttons` with no inset and round; `Decrease` and `Increase` 44 wide; the `Divider` 0 wide; the `Icon` 20. The gallery's page has them.
- To what reads the tree (`lurq_snapshot` of the MCP server) the box is a `spinbutton` called by its label, with `a11y_valuenow`, `a11y_valuetext` (as it is written), `a11y_valuemin` and `a11y_valuemax`; the buttons are `button`s called "Increase" and "Decrease", with `a11y_disabled` at an end of the range.
- A renderer that draws another number input has `view.control()` (the box with all of its typing, keys and putting right), `view.button(part, icon, steps)`, `view.divider()`, `view.buttons()` (laid over the box as the layout has them) and `view.buttons_reach()` (how far they reach in from each end).

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
| `on_complete(handler)` | Called with the code when an edit fills its last cell |
| `size(InputOtpSize)` | `Sm`, `Md`, `Lg`: cells of 32 by 36, 36 by 40, 40 by 44, with type of 14, 16, 18 |
| `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `name` | As an [input](#input) has them; `name` is the whole code's in its form |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the input under the cells |

- To a person it is one field. Tab puts the focus on the first empty cell. Typing fills the cell that has the focus and goes on to the next; typing on a filled cell replaces its character. Backspace empties the cell that has the focus, or, on an empty one, goes back and empties the one before; Delete empties it and stays. Left and Right move, no further than the first empty cell; Home and End go to the first cell and to the first empty one. A click puts the focus on the cell clicked, or on the first empty one where that is before it.
- A code has no gaps: emptying a cell before the last filled one moves the cells after it up.
- A paste of a whole code, or more, fills every cell from the first; a shorter one fills the cells from the one that has the focus. What the code does not take of it (spaces, dashes) is dropped, so `123-456` is `123456`.
- To a form, to tooling and to the platform it is one text input: under the cells is the engine's `TextInput`, bound to the code, which draws nothing of its own. It is the one stop of Tab, the one `textbox` to what reads the tree (`lurq_snapshot` of the MCP server), called by its label, with the code as its value, and the one value of its form under its `name`. It says it holds a `ContentKind::OneTimeCode`, so a phone offers the code from a message that just came, and asks for the digits keyboard where the code is of digits. A masked code is a masked input.
- The cell the focus is on, and whether the input has the focus, are kept by the code's signal for as long as its input is in the tree, since a builder keeps nothing between renders: two codes bound to one signal share them.
- Parts: `InputOtpPart::Root` (the row of cells and separators, and the room between them), `Cell`, `Value` (the character, or its dot), `Caret` (in the empty cell that has the focus), `Separator` (the dash between groups). States: `InputOtpState::Invalid`, `Warning`, `Valid`, `ReadOnly`, `Disabled`, `Masked`, and on a cell of its own `Active` (the focus is on it) and `Filled`.
- In the design a cell is 36 wide and 40 high, 6 from the next, in the background's fill with an edge of 1 in the `Input` color, `radius-md` round, its character in the mono type at 16 in `Foreground`. The cell the focus is on has a line of 2 inside it in the `Ring` color, and while it is empty a caret 1.5 wide and 18 high in `Foreground`, which blinks unless the theme reduces motion. The dash between groups is the `minus` glyph at 16 in `MutedForeground`, 6 from the cells on either side. A judgment colors every edge and the ring; a disabled code is half opaque on the muted fill, a read-only one on the muted fill in the `Border` color.
- The mobile skin of the board (Mobile/OTP Field) is a rule of the theme: `Root` 8 apart, `Cell` 46 by 56 and `radius-lg`, `Value` in the sans type at 22 and semibold, `Caret` 2 by 24.
- A renderer that draws other cells starts from `view.input()`, the behavior, and lays its own cells over it: `view.characters`, `view.shown(index)` (what a cell shows, its dot where masked), `view.active` (the cell the focus is on, while the code has it), `view.separators`, and `view.cell_part(index, part)` for the style of a part of one cell. `view.cell(index)`, `view.separator()` and `view.cells()` are the default ones. A cell built by `view.cell(index)` is one a click is known to be on.

## Checkbox Card

A checkbox whose whole card is pressed, with a title, a description and an icon or a price: the add-ons of a plan, the reports a team is sent.

```rust
use lurq::ui::{CheckboxCard, CheckboxCardGroup, icons};

CheckboxCard::new("Analytics")
  .description("Share anonymous usage data.")
  .checked(analytics.get())
  .on_change(move |checked| analytics.set(checked));

// Cards over one list: those whose values the signal has are checked.
CheckboxCardGroup::new(add_ons, [
  ("backups", CheckboxCard::new("Backups").icon(icons::SHIELD).price("$4")),
  ("support", CheckboxCard::new("Priority support").icon(icons::MAIL).price("$20")),
])
.horizontal(true);
```

| Builder | |
| --- | --- |
| `description(&str)` | More of it, under the title in the muted color; what the card is described by to tooling |
| `icon(name)`, `price(&str)` | An icon of 20 before the text; what the choice costs, at the end of the title |
| `checked(bool)` | Whether it is checked. The card does not keep it: the app says so |
| `on_change(handler)` | Called on a press with what the card is to be: checked if it was not |
| `invalid(bool)`, `full_width(bool)` | The edge in the destructive color; as wide as what holds it in place of 220 |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the card |

| `CheckboxCardGroup` | |
| --- | --- |
| `new(Signal<Vec<T>>, [(T, CheckboxCard)])` | The cards whose values the signal has are checked; a press adds a value or takes it out |
| `controlled(checked, [(T, CheckboxCard)], press)` | The app holds what is checked and is told each value pressed |
| `horizontal(bool)` | Side by side, 12 apart and as high as the highest, going on to the next line where there is no room |
| `label(&str)`, `invalid(bool)`, `disabled(bool)`, `style(..)`, `id`, `key` | What the group is called to tooling; every card invalid or disabled |

- The card is what is pressed: a click anywhere on it, or Space or Enter while it has the focus. It is the one stop of Tab and the ring of the focus is around it; the box in it is drawn and takes no focus of its own. In a group each card is its own stop.
- Parts: `ChoiceCardPart::Root` (what is pressed), `Icon`, `Text`, `Heading` (the title and the price), `Title`, `Price`, `Description`. States: `ChoiceCardState::Chosen`, `Invalid`, `Disabled`, `FullWidth`. The box is the checkbox's own `CheckboxPart::Box` and `Indicator`, resolved from the theme's checkbox recipe: a theme that restyles a checkbox restyles the card's box. A group's parts are `ChoiceCardGroupPart::Root` and `Item` (laid over each card's root), its state `ChoiceCardGroupState::Horizontal`.
- In the design the card is 220 wide, padded 16, in `Background` with the large radius; the text first, the title in 14 and medium over the description 4 under it in 13 of `MutedForeground`, and the box 12 after it at the top. Not checked its edge is 1 of `Input`; checked, 2 of `Primary`. Under the pointer the edge of a card not checked is `Ring`; an invalid card is edged in `Destructive`; a disabled one is dimmed.
- To what reads the tree (`lurq_snapshot` of the MCP server) the card is a `checkbox` called by its title, described by its description, with `a11y_checked` saying whether it is checked; a group is a `group`.
- In a [field](#field) a card is invalid where the field is and disabled where the field is, and keeps its title as its name; a group takes the field's label as its name unless it has one.
- A renderer that draws another card is given a `ChoiceCardView`: `view.root()` is the behavior with the root's style, `view.text()`, `view.icon()` and `view.control()` the default parts. The same renderer type serves the radio card; `view.kind` says which it is.

## Radio Card

A radio whose whole card is pressed, with a title, a description and an icon or a price: the plan of an account, the density of a list.

```rust
use lurq::ui::{RadioCard, RadioCardGroup};

RadioCardGroup::new(density, [
  (Density::Comfortable, RadioCard::new("Comfortable").description("Roomy rows, 36px controls.")),
  (Density::Compact, RadioCard::new("Compact").description("Dense rows, 28px controls.")),
  (Density::Touch, RadioCard::new("Spacious").description("For touch.").disabled(true)),
]);

// One card alone: the app says whether it is chosen and is told when it is pressed.
RadioCard::new("Team").price("$12").chosen(plan.get() == Plan::Team).on_choose(move || plan.set(Plan::Team));
```

| Builder | |
| --- | --- |
| `description(&str)`, `icon(name)`, `price(&str)` | As a [checkbox card](#checkbox-card) has them |
| `chosen(bool)` | Whether it is the choice made: the dot in its mark. The card does not keep it |
| `on_choose(handler)` | Called on a press while the card is not chosen. A press of the one chosen does nothing |
| `invalid(bool)`, `full_width(bool)`, `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a checkbox card has them |

| `RadioCardGroup` | |
| --- | --- |
| `new(Signal<T>, [(T, RadioCard)])` | The card whose value the signal has is chosen; a press of another sets the signal |
| `controlled(Option<T>, [(T, RadioCard)], choose)` | The app holds the choice, none while it is `None`, and is told each value pressed |
| `horizontal(bool)`, `label(&str)`, `invalid(bool)`, `disabled(bool)`, `style(..)`, `id`, `key` | As a checkbox card group has them |

- A group is one stop of Tab, at the chosen card or the first. The arrow keys move the focus to the next card and choose it, Home and End the first and the last, and a disabled card is passed by: the focus group of a [radio group](#radio), `FocusGroup::both().activate(true).one_stop(true)`.
- Parts and states are a checkbox card's (`ChoiceCardPart`, `ChoiceCardState`); its mark is the radio's own `RadioPart::Mark` and `Dot`, from the theme's radio recipe.
- In the design it is a checkbox card with the radio's mark of 16 in place of the box: not chosen, an edge of 1 of `Input`; chosen, 2 of `Primary`, with the dot of 8 in the mark.
- To what reads the tree the card is a `radio` called by its title, described by its description, with `a11y_checked`; a group is a `radiogroup`.

## Fieldset

Fields that go together, under a legend that says what they are: the profile of an account, the address of an order.

```rust
use lurq::ui::{Button, Field, Fieldset, Input};

Fieldset::new("Profile")
  .description("Shown to people you share a workspace with.")
  .field(Field::new("Display name", Input::new(name).full_width(true)).required(true))
  .field(Field::new("Bio", Input::new(bio).full_width(true)).optional(true))
  .child(Button::new("Save profile"))
  .disabled(locked.get());
```

| Builder | |
| --- | --- |
| `description(&str)` | More of it, 4 under the legend, in the muted color |
| `field(Field)` | Adds a field, made as wide as the fieldset (`Field::full_width`) |
| `child(element)`, `children(..)` | Adds anything else under what is there: a row of buttons, a group of cards, an [error summary](#error-summary) |
| `disabled(bool)` | Every control in it takes no input and Tab passes it by; the whole fieldset is dimmed |
| `full_width(bool)` | As wide as what holds it, in place of 400 |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [field](#field) has them |

- What it holds is one under another, 16 apart, and stretched to its width: a field keeps a width of its own unless it is made full width, which `field` does.
- Parts: `FieldsetPart::Root`, `Legend` (the title over the description), `Title`, `Description`. States: `FieldsetState::Disabled`, `FullWidth`.
- In the design the fieldset is 400 wide, padded 20, with an edge of 1 of `Border` inside it and the large radius. The legend's title is 15 and semibold in `Foreground`, its description 13 in `MutedForeground`.
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
| `FieldError::new(label, message)` | The field's label and what is wrong with it. The line shows the message |
| `FieldError::focuses(&ElementRef)` | A press of the line, or Enter or Space on it, focuses the first control under the ref |
| `form(&FormHandle, [(name, label, &ElementRef)])` | The form's errors once a submit was attempted: a line for each listed field that has one, in their order, then one for each other field with an error, called by its name and focusing nothing. It reads them as signals: what renders it renders again when they change |
| `title(&str)` | The title in the app's own words, in place of "2 fields need attention" |
| `full_width(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a fieldset has them; `force` draws every line as under the pointer or focused |

- A line that focuses a field is a stop of Tab, underlined under the pointer, with the ring of the focus around it. One that focuses nothing is read and not pressed.
- Parts: `ErrorSummaryPart::Root`, `Icon`, `Body` (the title over the lines), `Title`, `Item` (a line), `Bullet`, `Message`. State: `ErrorSummaryState::FullWidth`.
- In the design the summary is 400 wide, padded 12 and 14, in `DestructiveMuted` with an edge of 1 of `Destructive` inside it and the medium radius. The alert icon of 16 in `Destructive` is 10 before the body; the title is 13 and semibold in `Destructive`, and the lines are 6 apart under it, a dot of 4 in `Destructive` 8 before each message in 13 of `Foreground`.
- To what reads the tree it is an `alert` called by its title; each line is a `link` called by the field's label and its message.
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
  .on_input(move |typed| fetched.set(search(typed)))
  .filter(|_, _| true)
  .loading(searching.get());

// A value that is no option's: what is typed becomes the value of a combobox of String.
Combobox::new(label.clone(), [SelectOption::new("bug".to_owned(), "bug")]).allow_custom(true);
```

| Builder | |
| --- | --- |
| `Combobox::new(value, options)` | Bound to a `Signal<T>`: the option with its value is chosen, and choosing another sets it. The options are a [select](#select)'s: `(value, label)` or `SelectOption`s with a detail, a hint, elements before and after the label, a group they start, or disabled |
| `placeholder(&str)` | What the box says while nothing is typed |
| `filter(Fn(&str, &str) -> bool)` | Which options are shown while something is typed, given the label and what is typed. By default the labels that contain it, without regard to case |
| `on_input(Fn(&str))` | Called with what the box will hold at each edit: for an app that fetches its options |
| `loading(bool)` | The list ends in a row with a spinner, "Loading results…" (`loading_text(&str)` says otherwise) |
| `empty(&str)` | What the list says while what is typed leaves no option: "No results" |
| `allow_custom(bool)` | What is typed may be no option's: typing highlights nothing, and Enter or leaving keeps it. A label of an option, in any case, chooses the option; anything else becomes the value of a combobox of `String` |
| `query(Signal<String>)` | Where the app keeps what is typed, to read or set it. Without one the combobox keeps it while it is in the tree, one for each signal of a value |
| `size(ComboboxSize)` | `Sm` 32, `Md` 36 (the default), `Lg` 40 high, with text of 13, 14, 15 |
| `invalid(bool)`, `warning(bool)`, `valid(bool)` | What is chosen is wrong, doubtful or right: the edge and the ring in the destructive, warning or success color. Of the three, the first that is set is shown |
| `disabled(bool)`, `full_width(bool)`, `label(&str)`, `name(..)` | As a [select](#select) has them. `name` is the name of the value in a form: the chosen option's label |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box; its input has the id with `-input` after it |

- The box is a select's with a text input in it, which keeps the focus the whole time. A press anywhere on the box focuses the input and opens the list; typing opens it too and filters it, with the first option that can be chosen highlighted. Down and Up move the highlight, which a row shows as it shows the pointer; Enter chooses; Escape closes the list and a second Escape clears what was typed; Tab chooses nothing, closes the list and goes on.
- Choosing puts the option's label in the box. While the box shows the chosen label the list opens whole; what is typed otherwise filters it. When the focus leaves, what was typed goes back to the chosen label, unless custom values are allowed. A group keeps its label over the first of its options that what is typed leaves.
- It is a control a [field](#field) holds (`FieldControl`): the field names it, judges it and disables it.
- Parts: `ComboboxPart::Trigger` (the box), `Value` (what is typed), `Placeholder`, `Chevron`, `Menu` (the panel of the list), `Item`, `Check`, `Detail`, `Hint`, `Group`, `Separator` (a select's, by the same names), `Status` (the row of no results or of loading) and `Spinner`. States: `ComboboxState::Open`, `Invalid`, `Warning`, `Valid`, `Disabled`, `FullWidth`, and of an item `Chosen` and `ItemDisabled`.
- In the design the box is a select's: 260 by 36, `Background` in an edge of 1 in `Input`, 6 round, its text 12 in, the placeholder in `MutedForeground`, two chevrons of 16 at its end; the ring's color under the pointer and a line of 2 inside it while the focus is in it or the list is open. The list is a select's: 4 under the box on `Popover`, 8 round, items of 32 on `Highlight` under the pointer or the keys, the chosen one with a check of 16 at its end. The status row is 32 high, 8 in, in `MutedForeground` at 13, its spinner 16 in `Foreground`.
- It is the engine's `Select` in its combobox mode (`Select::combobox`): one implementation of the list, its rows, groups, checks and adornments with the select's. To what reads the tree (`lurq_snapshot` of the MCP server) the box is a `combobox` with `expanded` while its list is open, the input in it a `textbox` called by the combobox's label, and the list a `listbox` of `option`s with `a11y_selected`, which says in `a11y_activedescendant` which option the keys are on.
- A renderer that draws another combobox has `view.select_style()`, `view.input()` (the text input, with what leaving it does), `view.chevron()`, `view.status()`, `view.query()`, `view.shown()` (the labels the list shows) and `view.select(style, trigger)`, which builds the combobox and calls `trigger` with a `SelectTriggerState` whose `input` is the input to place.

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
.clearable(true);
```

| Builder | |
| --- | --- |
| `MultiSelect::new(value, options)` | Bound to a `Signal<Vec<T>>`: the options with its values are chosen, shown as tags in the order they were chosen; choosing one adds its value, or takes it out. The options are a select's |
| `placeholder(&str)` | What the box says while nothing is chosen and nothing is typed |
| `max_shown(usize)` | How many tags the box shows: the rest are one tag that counts them, "+3" |
| `clearable(bool)` | A cross at the end of the box, while something is chosen, that takes every value away |
| `filter(..)`, `on_input(..)`, `loading(bool)`, `loading_text(&str)`, `empty(&str)`, `query(..)` | As a [combobox](#combobox) has them |
| `size(ComboboxSize)` | `Sm` 32, `Md` 36, `Lg` 40 high at least, with tags of 20, 24, 28 |
| `invalid`, `warning`, `valid`, `disabled`, `full_width`, `label`, `name`, `state`, `style`, `rules`, `force`, `id`, `key` | As a combobox has them. A disabled one keeps its tags and takes none away; `full_width` is in place of 280 |

- The box holds the chosen values as tags ([tag](#tag)s with `pill`), and a text input after them, which keeps the focus while the list is open. The tags go on to the next line where they do not fit, and the box grows with them.
- Typing opens the list and filters it; Down and Up move the highlight; Enter or a press chooses an option or takes it back, and the list stays open while one chooses, with what was typed kept, to choose another of it. A tag's cross takes its value away, and so does Backspace in the empty input, the last one's. Escape closes the list and a second Escape clears what was typed; Tab goes on, and what was typed is cleared when the focus leaves. A press of a tag or of the clear control does not open the list. Tab reaches the cross of each tag before the input.
- It is a control a [field](#field) holds.
- Parts: a combobox's, by the same names, and `MultiSelectPart::Check` (the box before a chosen option), `CheckIcon`, `Unchecked` (the box before one that is not), `Tags` (the lines of the tags and the input: its gap), `Tag` and `TagLabel` (what is laid over a tag's root and label), `Clear` and `ClearIcon`. Its states and sizes are a combobox's (`ComboboxState`, `ComboboxSize`).
- In the design the box is a select's, 280 wide and at least 36 high, padded 6 at its start, 10 at its end and 6 over and under, a chevron down of 16 at its end. A tag is 24 high on `Secondary`, 4 round, padded 8 before its text of 13 in `SecondaryForeground` and 6 after its cross of 12 in `MutedForeground`; the tags are 6 apart, across and down, and what is typed keeps 48 on their line. The list is a select's, with a box of 16 before each option, 8 before its label, 4 round: `Background` in an edge of `Input`, and, chosen, `Primary` with a check of 12 in `PrimaryForeground`.
- It is the engine's multiple `Select` in its combobox mode. To what reads the tree it is a `combobox` with `expanded`, its list a `listbox` of `option`s with `a11y_selected`, and each tag's cross a `button` called "Remove" and the tag's text; the clear control is a `button` called "Clear".
- A renderer that draws another multi select has `view.select_style()`, `view.input()`, `view.tags()`, `view.tag_rules()`, `view.clear()`, `view.chevron()`, `view.status()`, `view.chosen()`, `view.shown()`, `view.query()` and `view.select(style, trigger)`.

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
  .tokens(|text| {
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
| `draft(Signal<String>)` | What is typed and is no item yet: read for suggestions, and what the app sets on it is what the input shows after its tokens |
| `unique(bool)` | An item the list already has is not added again; what was typed is dropped |
| `commit_on_blur(bool)` | The draft becomes an item when the focus leaves, in place of staying typed |
| `token(impl Fn(&str, Token) -> Token)` | Dresses the token of each item of a list: a variant by what the item is, or no cross |
| `on_change(impl Fn(&[String]))` | Called with the list when a person changed it: an item made of the draft, pasted, or taken away |
| `on_draft(impl Fn(&str))` | Called with the draft when a person changed it |
| `new(Signal<String>)` | A text, tokens and all, where the text around the tokens is the value too; what is typed is set on it, and what the app sets on it is shown |
| `tokens(impl Fn(&str) -> Vec<Token>)` | How the app finds its tokens in a text, called in every render. Tokens that overlap one before them, hold a line break or are not on characters are left out. Not asked over a list |
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
  a selected token replaces it with a draft. `on_change` hears the list and `on_draft` the draft; `on_input` still
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
  `TokenHovered` and `TokenSelected`. A token's parts are resolved with its variant and its state as the case, so a
  recipe styles a variant's tokens with `.variant(..)` and their looks with `.state(TokenInputState::TokenHovered, ..)`.
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
| `Mention::new(label)` | An offer: `text(&str)` is what it puts in the field (the trigger and the label by default), `avatar(Avatar)` or `icon(name)` before its label |
| `on_mention(handler)` | Called with the text of an offer as it is chosen |
| `tokens(..)` | Other tokens of the app's in the text, as a [token input](#token-input) finds them |
| `rows(min, max)` | 2 and 8 unless told |
| `full_width(bool)` | As wide as what holds it, in place of 280 |
| `input_rules(&recipe)` | Rules laid over the token input it is |
| `placeholder`, `size`, `invalid`, `warning`, `valid`, `read_only`, `disabled`, `label`, `name`, `on_input`, `state`, `style`, `rules`, `force`, `id`, `key` | As a token input has them |

- Typing a trigger at the start of the text or after a space opens the list under it while it offers something. Up and
  Down move the offer that Enter (or Tab) chooses, a click chooses one, the pointer moves the active one, and Escape
  puts the list away while the caret stays in that word, so that Enter is a new line again. The keys stay in the
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
  `List`, `Option`, `OptionAvatar`, `OptionIcon`, `OptionLabel`; the mentions themselves are the token input's parts in
  the variant `MENTION`. States: `MentionsState::Open`, `Active` (an offer's own).
- In the design the box is 280 wide and at least 64 high, padded 12 across and 8 down, its text 14 with lines of 1.5.
  The list hangs 4 under the trigger: 220 wide, `Popover`, `radius-lg`, an edge of 1 in `Border`, the medium shadow,
  padded 4. An offer is 36 high, padded 8, `radius-sm`, an avatar of 24 (or an icon of 16 in `MutedForeground`) 8
  before its label in the body's type; the active one, and the one under the pointer, are on `Highlight`. A mention is
  in the text's type on `InfoMuted`, its text in `Info`, 20 high and padded 4.

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
```

| Builder | |
| --- | --- |
| `Alert::new(&str)` | The title: what it is about. It is what the alert is called |
| `description(&str)` | More of it, under the title |
| `variant(AlertVariant)` | `Neutral` (the default), `Info`, `Success`, `Warning`, `Destructive` |
| `icon(..)` | An icon of the theme's set in place of the variant's (`icon(icons::DOWNLOAD)`), or none (`icon(None)`) |
| `action(element)` | What can be done about it, at its end before the close control: a button, a link |
| `child(element)`, `children(..)` | Elements under the description, for more than text |
| `on_close(handler)` | Draws the close control, called on a press of it. The app takes the alert away |
| `role(&str)` | What it is to tooling, in place of the variant's |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` draws the close control's states; `id` is the id of the box |

- It is read where it stands and is not an overlay. It is as wide as the room it is given, and its title and description wrap in the room the icon, the action and the close control leave them.
- The close control is a stop of Tab with the focus ring; Enter, Space and a click press it.
- To what reads the tree (`lurq_snapshot` of the MCP server) a warning and a destructive alert are an `alert`, which a screen reader says at once, and the others a `status`, said when the reader is done; each is called by its title. `role("status")` says a warning is no news, `role("alert")` that an informative one is urgent. `AlertVariant::role()` is the variant's.
- Parts: `AlertPart::Root` (the box), `Icon`, `Text` (the title, the description and the body, one over the other), `Title`, `Description`, `Body`, `Action`, `Close`. States: `AlertState::Action`, which centres what is in the box on the action. Variants by `AlertVariant`.
- In the design the box is in the `Card` color with an edge of 1 in `Border`, corners of `radius-lg`, padded 16. The icon is 16, 12 from the title (14, medium, `Foreground`) over the description (13, 1.5 high, `MutedForeground`), 4 apart. An informative, a successful and a warning alert are in `InfoMuted`, `SuccessMuted`, `WarningMuted`, with an edge and an icon of `Info`, `Success`, `Warning` and a description in `Foreground`. A destructive one keeps the `Card` color, with an edge, an icon and a title of `Destructive`. The icon is the information sign for a neutral and an informative alert, the check in a circle for a successful one, the warning sign for a warning and a destructive one. The close control is 16 in `MutedForeground`, in `Foreground` under the pointer.
- `AlertVariant` is shared with the banner and the callout: `icon()`, `accent()` (the color of its icon, edge, bar and link) and `tint()` (its fill) say the same for the three.
- A renderer that draws another alert starts from `view.text()`, `view.icon()`, `view.action()` and `view.close()` (the behavior), with `view.role` and `view.closable`.

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
| `variant(AlertVariant)` | As an [alert](#alert)'s. A neutral and an informative banner have the megaphone |
| `icon(..)` | An icon of its own, or none: `icon(None)` |
| `link(Link)` | A link after the text, in the variant's color and semibold |
| `action(element)` | Any element after the text in place of a link: a button |
| `on_close(handler)` | Draws the close control, called on a press of it |
| `open(bool)` | Whether it is shown. It is unless it says |
| `centered(bool)` | Its icon, text and action in its middle rather than from its start |
| `sticky(bool)` | It sticks to the top of what scrolls the page it is on |
| `role(&str)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As an alert has them. `id` is the id of the bar, `key` the key of what stands in the page |

- It stays in the tree whether it is shown or not, so it fades in as `open(true)` shows it and out as `open(false)` takes it away (the `Panel` part's `enter` and `exit`: `ui::FADE_IN` over the theme's base duration, `ui::FADE_OUT` over its fast one). Taken away it is no height, and what is under it moves up.
- A text too long for one line goes on to the next in the room the rest leaves it; a centred one is as wide as it is, and no wider than that room.
- `sticky(true)` gives what stands in the page the engine's `Sticky::top(0.0)`: in a scroll container it stays at the top while the page scrolls under it.
- The link and the close control are stops of Tab, in that order. To what reads the tree it is an `alert` or a `status` as an alert is, called by its text.
- Parts: `BannerPart::Root` (what stands in the page, sticky or not), `Panel` (the bar), `Icon`, `Text`, `Action`, `Link` (the style a link given by `link` takes over the theme's for a link), `Close`. States: `BannerState::Centered`, `Sticky`.
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
| `variant(AlertVariant)`, `icon(..)` | As an [alert](#alert) has them |
| `bar(bool)` | A bar along its start in the variant's color, and the icon before a title in that color |
| `role(&str)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As an alert has them |

- It is read, and is no stop of Tab. To what reads the tree it is a `note`, called by its title where it has one.
- Without a bar, the icon stands at its start and the title, the text and the elements beside it. With a bar and a title, the icon and the title are a heading over the text; with a bar and no title, the icon stands at the start as without one.
- Parts: `CalloutPart::Root` (the box), `Icon`, `Heading` (the icon and the title, with a bar), `Content` (the title, the text and the elements), `Title`, `Text`. States: `CalloutState::Bar`, `Titled`.
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
| `separator(&str)` | What stands between the steps of a sequence: "then" unless told |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [badge](#badge) has them. `id` is the id of the root |

- Keys are named as lurq names them in a `KeyboardEvent` (`Escape`, `ArrowUp`, `Backspace`, `PageUp`) or as a person writes them, in any case and with spaces or none: `Esc`, `Del`, `Page Up`. `Mod` is Ctrl, and ⌘ on a Mac; `Meta`, `Cmd`, `Super` and `Win` are ⌘ on a Mac. Arrows are drawn as ↑ ↓ ← →. A single letter is drawn in capitals; a key lurq does not name is drawn as it is written (`Up` stays `Up`).
- On a Mac the modifiers are glyphs and stand first, in the order `Accelerator::display` writes them: ⌃ ⌥ ⇧ ⌘. In words they keep the order they are given in.
- Keys are read and not operated: they take no press, no focus and no stop of Tab. To what reads the tree (`lurq_snapshot` of the MCP server) they are one `text`, called by their keys in words: "Ctrl+K", "Cmd+Shift+P", "G then D".
- Parts: `KbdPart::Root` (the row of keys, its gap), `Key` (a key's box), `Label` (the text on a key, or of a plain step), `Separator` (what stands between steps). Variants: `KbdVariant`. Sizes: `KbdSize`. States: `KbdState::Sequence`, `Mac`.
- In the design a key is 20 high, padded 6, on `Muted` with an edge of 1 in `Border` and the radius `Sm`, its text 11 in the family of code in `MutedForeground`. The keys of a shortcut are 4 apart with nothing between them; large ones are 24 high, padded 7, at 12 and 6 apart, as the list of an app's shortcuts has them. Plain is the text of a menu's shortcut: 11 in the family of code in `MutedForeground`.
- A renderer that draws other keys has `view.steps` (each step's keys, with what is drawn on each and what it is called), `view.root()`, `view.key_cap(key)`, `view.label(text)`, `view.between()` and `view.written(step)` (a step as one text).
- `InlineCode::new(&str)` is a short text of code in a run of text: 13 in the family of code in `Foreground`, on `Muted` with the radius `Sm`, padded 5 across and 1 above and below. It is not cut, and is `code` to what reads the tree, called by its text. Parts: `InlineCodePart::Root`, `Label`.

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
| `description(&str)` | Why, or what to do, under the title |
| `icon(name)` | An icon of the theme's set over the title, in its well |
| `media(element)` | An element of the app's in place of the icon and its well: an illustration, a picture |
| `action(element)` | Something to do under the text; called again, a second one after it, the main one last |
| `size(EmptyStateSize)` | `Sm` (the icon alone, 22, everything 10 apart), `Md` (a well of 40, in an edge), `Lg` (a round well of 64, a title of 18) |
| `bordered(bool)` | An edge around it. `Md` has one unless told; `Sm` and `Lg` have none |
| `height(Length)` | A height of its own, in which what it holds is centred: `Length::Fill` for all of the room a parent that stretches it gives |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [badge](#badge) has them. `id` is the id of the root |

- It is as wide as the room it is given and as high as what is in it, all of it centred; the title and the description wrap in the width there is. Its actions are the app's elements, as large as the place has them: a small button in a table.
- It is read; what it gives to do is operated as itself. To what reads the tree it is a `status`, called by its title, with its description as `a11y_description`.
- Parts: `EmptyStatePart::Root`, `Media` (the well), `Icon`, `Text` (the title over the description), `Title`, `Description`, `Actions`. Sizes: `EmptyStateSize`. States: `EmptyStateState::Bordered`.
- In the design the medium one is padded 32, with 16 between the well, the text and the actions, in an edge of 1 in `Border` with the radius `Xl`; the well is 40 on `Muted` with the radius `Lg`, its icon 20 in `Foreground`; the title is 14/500 in `Foreground`, the description 4 under it at 13 in `MutedForeground`; the actions are 8 apart. The small one is the empty table's: padded 32 and 16, everything 10 apart, the icon 22 in `MutedForeground` with no well. The large one is the phone's: padded 32 and 24, a round well of 64 with an icon of 28, a title of 18/600 and a description of 15, 6 apart.
- A renderer that draws another empty state has `view.root()`, `view.media()` (the app's element or the icon in its well), `view.text()` and `view.actions()`.

## Result

What a page or a dialog says when something is over: payment received, build failed, nothing here. It is `ui::Outcome` in the code, so that `use lurq::ui::*` does not hide `std::result::Result`.

```rust
use lurq::ui::{Button, ButtonVariant, Outcome, OutcomeVariant};

Outcome::new(OutcomeVariant::Success, "Build succeeded")
  .description("412 tests passed in 38 seconds.")
  .action(Button::new("View log").variant(ButtonVariant::Outline))
  .action(Button::new("Deploy"));

// On a page of its own, or in a dialog: no box.
Outcome::new(OutcomeVariant::NotFound, "Nothing here")
  .description("This page was moved or never existed.")
  .bordered(false)
  .action(Button::new("Home"));
```

| Builder | |
| --- | --- |
| `Outcome::new(OutcomeVariant, &str)` | What came of it, and its title, which is what it is called. `Success`, `Error`, `Warning`, `Info` or `NotFound` (a 404) |
| `description(&str)` | More of it, under the title |
| `icon(name)` | Another icon of the theme's set in the mark, in the variant's colors |
| `media(element)` | An element of the app's in place of the mark |
| `extra(element)` | What stands under the text and over the buttons, as wide as the outcome: a summary, a list |
| `action(element)` | A button at the bottom, after those there: the least first, the main one last |
| `bordered(bool)` | A box of its own with a fill and an edge, as the design draws it: true unless told |
| `width(Length)` | A width of its own in place of the design's 300 |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [badge](#badge) has them. `id` is the id of the root |

- Everything in it is centred; its title and description wrap in its width. With no box it has no padding either, and stands where what holds it puts it: a column that centres it on a page, the body of a [dialog](#dialog).
- It is read; its buttons are operated as themselves. To what reads the tree it is a `status`, called by its title, with its description as `a11y_description`.
- Parts: `OutcomePart::Root`, `Mark`, `Icon`, `Text`, `Title`, `Description`, `Extra`, `Actions`. Variants: `OutcomeVariant` (`OutcomeVariant::icon()` is the icon each is marked with). States: `OutcomeState::Bordered`.
- In the design it is 300 wide, padded 24 with 16 between the mark, the text and the buttons, on `Background` in an edge of 1 in `Border` with the radius `Lg`. The mark is a circle of 48 in the variant's muted color, its icon 24 in the variant's color: a check in `Success`, a cross in `Destructive`, a warning in `Warning`, an `i` in `Info`, and a crossed magnifier in `Foreground` on `Muted` for not found. The title is 16/600 in `Foreground`, the description 4 under it at 13 in `MutedForeground`; the buttons are 8 apart.
- A renderer that draws another outcome has `view.root()`, `view.mark()`, `view.text()`, `view.extra()` and `view.actions()`.

## Message Box

A dialog that asks one question or says one thing and waits for the answer: replace a file, save the changes, delete projects.

```rust
use lurq::ui::{MessageBox, MessageBoxKind, MessageBoxRole, icons};

MessageBox::new(open.clone())
  .kind(MessageBoxKind::Warning)
  .title("Replace existing file?")
  .message("“theme.json” already exists in this folder. Replacing it overwrites its contents.")
  .checkbox("Don't ask again", again.clone())
  .answer("Cancel", MessageBoxRole::Cancel, || {})
  .answer("Replace", MessageBoxRole::Default, move || replace_file());

// A question that cannot be undone: the focus goes to Cancel.
MessageBox::new(open.clone())
  .icon(icons::TRASH)
  .title("Delete 3 projects?")
  .message("This cannot be undone.")
  .answer("Cancel", MessageBoxRole::Cancel, || {})
  .answer("Delete", MessageBoxRole::Destructive, move || delete_projects());
```

| Builder | |
| --- | --- |
| `MessageBox::new(Signal<bool>)` | Shown while the signal holds `true`. The message box does not keep it: every answer sets it to `false` before what the answer does runs |
| `kind(MessageBoxKind)` | `Information` (the default, `info`), `Question` (`help-circle`), `Warning` (`warning`), `Error` (`alert-circle`): the icon in the well and their colors. A theme may add kinds by name |
| `icon(&str)` | An icon of the theme's set in place of the kind's: `trash` for a question that deletes |
| `title(&str)`, `message(&str)` | What it asks or says, and more of it in the muted color. The title is what it is called, the message what describes it |
| `detail(&str, Signal<bool>)` | A text behind "Details" under the message, open while the signal holds `true`; `detail_label(&str)` names the trigger |
| `checkbox(&str, Signal<bool>)` | A checkbox at the start of the footer: "Don't ask again" |
| `answer(&str, MessageBoxRole, handler)` | An answer. `Default`: filled, Enter gives it. `Cancel`: outlined, Escape and a press of the veil give it. `Destructive`: in the destructive fill. `Plain`: outlined, at the start of the footer |
| `width(f32)` | A width of its own over the board's 400 |
| `inline(bool)` | The panel where it is declared, without the veil: for a gallery, a screenshot, a test |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [dialog](#dialog) has them. `id` is the id of the panel |

- It is a [dialog](#dialog): the veil, the focus going in as it opens and back as it closes, and Tab going round its controls are the dialog's. As it opens the focus goes to the default answer, or to the cancel one where an answer is destructive.
- Enter gives the default answer wherever the focus is, on the checkbox too, unless it is on another button, which Enter presses. Escape and a press of the veil give the cancel answer where there is one and do nothing where there is none.
- The plain answers stand at the start of the footer, after the checkbox, in their order; at its end the cancel answer stands first and the others after it, whatever order they were given in.
- Parts: `MessageBoxPart::Veil` and `Panel` (laid over the dialog's), `Content`, `Body`, `IconWell`, `Icon`, `Text`, `Title`, `Message`, `DetailTrigger`, `DetailIcon`, `DetailLabel`, `DetailPanel`, `Detail`, `Footer`, `Option`, `Checkbox` and `CheckboxLabel` (laid over the checkbox's root and label), `Actions`, and for each role a part laid over the button's root and one over its label: `DefaultButton`, `DefaultLabel`, `CancelButton`, `CancelLabel`, `DestructiveButton`, `DestructiveLabel`, `PlainButton`, `PlainLabel`. Variants: the kinds. States: `MessageBoxState::Destructive` (an answer is destructive: the icon in the destructive colors), `DetailOpen`.
- So a theme gives the two buttons of a message box fonts or fills of their own without touching a button anywhere else: `theme.patch_recipe::<MessageBox>(|rules| rules.base(MessageBoxPart::DefaultLabel, |s| s.font_family("Georgia")).base(MessageBoxPart::CancelLabel, |s| s.font_family(FontFamily::Mono)))`.
- In the design the panel is 400 wide, padded 20 with 16 between the body and the footer; the well is 36 and round, in the kind's muted color (`InfoMuted`, `Secondary` for a question, `WarningMuted`, `DestructiveMuted`), its icon 18 in the kind's color; the title is 15/600, 14 after the well, over the message of 13 in `MutedForeground`, 4 apart; the buttons are 32 high, padded 14, 8 apart; the checkbox's label is 13. "Details" is a chevron of 14 in the muted color 4 before its label of 13, and the detail opens 8 under it, in the mono type of 11 on the `Muted` fill, with its height over the theme's base duration.
- To what reads the tree it is an `alertdialog` called by its title and described by its message.
- A renderer that draws another message box (an iOS alert: the text in the middle, the answers as cells side by side) has the pieces: `view.content()` (with Enter), `view.title()`, `view.message()`, `view.find(role)`, `view.press(index)` (it closes the box and runs the answer), `view.button(index)` (the default renderer's button, which takes the focus first where it should), and `view.dialog(content)`, which puts what it built in the dialog with Escape, the veil and what tooling is told.

## Popconfirm

A question in a small panel hung from what was pressed, answered by one of two buttons: delete a file, leave a channel.

```rust
use lurq::ui::{Button, ButtonVariant, Popconfirm};

Popconfirm::new(anchor.clone(), open.clone())
  .trigger(Button::new("Delete file").variant(ButtonVariant::Outline))
  .title("Delete this file?")
  .description("It will be moved to the trash.")
  .destructive(true)
  .confirm("Delete", move || delete_file())
  .cancel("Keep", || {});
```

| Builder | |
| --- | --- |
| `Popconfirm::new(anchor, Signal<bool>)` | Hung from `anchor`, shown while the signal holds `true`. Keep the anchor from one render to the next, as for a [popover](#popover) |
| `trigger(element)` | A button that asks: it is the anchor, and a press of it turns the signal |
| `title(&str)`, `description(&str)` | The question, and more of it in the muted color |
| `icon(&str)` | An icon of the theme's set before the title, in place of the warning sign |
| `confirm(&str, handler)`, `cancel(&str, handler)` | The two buttons: their labels ("Yes" and "No" when not given) and what each runs after it closes the popconfirm. Escape and a press outside run the cancel one |
| `destructive(bool)` | The confirm button and the icon in the destructive color |
| `placement(Placement)`, `offset(..)` | Where it is beside its anchor: under it at its start, 4 from it, by default |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [popover](#popover) has them. `id` is the id of the panel |

- It is a [popover](#popover): kept in the window, gone to the other side where there is no room, the focus back on the anchor as it closes. As it opens the focus goes to the cancel button, so that Enter or Space does not confirm what was not read.
- Escape and a press outside close it and run what the cancel button runs.
- Parts: `PopconfirmPart::Panel` and `Body` (laid over the popover's root and body), `Content`, `Message`, `Icon`, `Text`, `Title`, `Description`, `Actions`, `CancelButton`, `CancelLabel`, `ConfirmButton`, `ConfirmLabel`. States: `PopconfirmState::Destructive`.
- In the design the panel is 250 wide, padded 12 with 12 between the message and the buttons, in the popover's colors, edge, large radius and medium shadow; the icon is the warning sign of 16 in `Warning`, 8 before the title of 13/500 over the description of 12 in `MutedForeground`, 2 apart; the buttons are 28 high, padded 10, their labels 13, 8 apart at the end.
- To what reads the tree it is an `alertdialog` called by its title and described by its description.
- A theme's renderer builds what the panel holds from `PopconfirmView` (`message()`, `cancel()`, `confirm()`, `actions()`, `content()` with Escape, `confirmer()`, `canceller()`); where the panel is shown, and when, stays the popover's.

## Progress Dialog

A dialog over the window while a piece of work runs: what is being done, how far it is, and a way to stop it.

```rust
use lurq::ui::{ProgressDialog, icons};

ProgressDialog::new(open.clone())
  .icon(icons::COPY)
  .title("Copying 128 items")
  .message("from Downloads to projects/lurq/assets")
  .value(copied.get())
  .max(128.0)
  .format(|done, all| format!("{done} of {all}"))
  .status("about 2 min left")
  .detail(&current_file.get(), details.clone())
  .action("Pause", move || pause())
  .on_cancel(move || stop());
```

| Builder | |
| --- | --- |
| `ProgressDialog::new(Signal<bool>)` | Shown while the signal holds `true`. It closes when the app sets it to `false`: Escape and a press of the veil do not close it |
| `icon(&str)`, `title(&str)`, `message(&str)` | What kind of work it is, what is being done (what it is called), and a line more of it in the muted color |
| `value(f64)`, `max(f64)` | How far the work is, a fraction of one unless `max` says otherwise. Without a value its end is not known, and the bar runs a segment across it |
| `format(Fn(f32, f32) -> String)` | Says how much is done under the start of the bar: "42 of 128". A percentage when not given |
| `status(&str)` | What the app says of the rest, under the end of the bar |
| `detail(&str, Signal<bool>)` | A text behind "Details" at the start of the footer, open under the footer while the signal holds `true`; `detail_label(&str)` names the trigger |
| `action(&str, handler)` | A button before the cancel one: "Pause". It does not close the dialog |
| `on_cancel(handler)`, `cancel_label(&str)` | A cancel button, which has the focus as the dialog opens and which Escape presses. It does not close the dialog: the app does once the work has stopped |
| `width(f32)`, `inline(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [message box](#message-box) has them |

- It is a [dialog](#dialog) with its veil, its focus and its Tab, which nothing but the app closes.
- Parts: `ProgressDialogPart::Veil` and `Panel` (laid over the dialog's), `Content`, `Header`, `Icon`, `Text`, `Title`, `Message`, `Progress` (laid over the [progress](#progress)'s root), `Meta`, `Count`, `Status`, `Footer`, `DetailTrigger`, `DetailIcon`, `DetailLabel`, `DetailPanel`, `Detail`, `Actions`, `ActionButton`, `ActionLabel`, `CancelButton`, `CancelLabel`. States: `ProgressDialogState::Indeterminate`, `DetailOpen`.
- In the design the panel is 420 wide, padded 20 with 12 between its parts; the icon is 20, 12 before the title of 15/600 over the message of 12 in `MutedForeground`, 2 apart; the bar is the progress's of 6 across the panel; what is done and what is left are in the mono type of 11 in `MutedForeground`; the footer, 4 further down, has "Details" at its start and the outlined buttons of 32, padded 14, 8 apart, at its end.
- To what reads the tree it is a `dialog` called by its title and described by its message, with the `progressbar` in it.

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
| `close_button(bool)` | A ghost button with the close icon at the end of the header |
| `grabber(bool)` | The handle at the top. A sheet at the bottom has it unless told otherwise |
| `dismiss_on_veil(bool)`, `dismiss_on_escape(bool)` | Whether a press on the veil (and, mounted, a drag down), and Escape, close it. Both on by default |
| `detents([SheetDetent])`, `detent(Signal<usize>)`, `scroll(ScrollState)` | For a mounted sheet at the bottom: the heights it stands at, which it is at, and the body's scrolling, which hands a finger to the sheet once the body is at its top |
| `body_scrolls(bool)` | Whether the body scrolls. It does unless told; one that does not is laid out in the room the panel has for it, at a side the panel's height less the header and the footer, for what holds a scroll of its own. A drawer over the page with `scroll(false)` gives its sheet this |
| `inline(bool)` | The panel where it is declared, without the veil and without taking the window: for a gallery, a screenshot, a test |
| `mount(ctx)` | At the bottom, the engine's draggable `Sheet` in this look; at another edge, the same as converting it |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's; `key` tells two sheets of one component apart |

- It is the engine's `Modal` with `target(Root)` and `takes_focus(true)`, as a [dialog](#dialog) is: declared anywhere, it takes no room there and covers the window under the window's own title bar while it is open. As it opens the focus goes to its first control (the close button where it has one), and as it closes back to what had it. Nothing under the veil takes the pointer, and Tab and Shift+Tab go round the sheet's own controls. A press on the panel goes no further.
- It slides in from its edge over the theme's slow duration in the entering curve as the veil fades in, and out to it over the base duration in the leaving curve: keyframes that move the panel by its own width or height (`ui::SHEET_IN_FROM_START` .. `ui::SHEET_OUT_TO_BOTTOM`). With motion reduced (`Theme::reduced_motion`, state `Modal` with `ReducedMotion`) it fades where it stands.
- At the start and the end the body takes the room the header and the footer leave, so the footer stands at the panel's bottom; at the top and the bottom the sheet is as high as what it holds while that fits, and the body gives way and scrolls when it does not.
- `mount(ctx)` puts a sheet at the bottom on the engine's [sheet](../modals/#sheets): a finger drags it between its `detents` (or the height its size gives it, or its content's) and throws it down to close it, it rises over the on-screen keyboard and keeps clear of a phone's edges, and it comes up with a spring. The engine lays out the panel and the backdrop; the recipe's `Panel` and `Veil` dress them (`SheetProps::dress`), and what the panel holds is the sheet's. A theme's renderer builds the sheet that is converted, not the mounted one, which is made of the view's parts.
- Parts: `SheetPart::Veil`, `Panel`, `Grabber` (the strip at the top of a bottom sheet), `Handle` (its bar), `Header`, `Heading`, `Title`, `Description`, `Body`, `Footer`, `Close` (a [button](#button) restyled), `Separator` (the [separator](#separator) over the footer, restyled). Variants: `SheetSide`. Sizes: `SheetSize`. States: `SheetState::Modal` (over the window, not inline), `ReducedMotion`.
- In the design a sheet at a side is in `Background` with a line of 1 in `Border` along the edge it shows the page and the large shadow. The header is padded 20 at the top and the sides, the title 16/600 in `Foreground` over the description in 13 `MutedForeground`, 4 apart, the close button's icon of 16 in `MutedForeground` at its end; the body is padded 20 with 16 between what it holds; the footer is padded 16 and 20, its buttons 8 apart at the end, under a line of `Border`. At the top the line is along its bottom. At the bottom it is the board's drawer: a line all round, corners of `radius-xl` (12) at its top, a handle of 40 by 4 in `Muted` 8 under its top edge, the heading in the middle 20 under it, and its buttons as wide as it, 8 apart, 20 from the edges. The veil is `Scrim`.
- The board's phone and tablet sheets are the same parts restyled: the phone's bottom sheet a panel of 360 round 20 at its top with a handle of 36 in `Input`, its title 18 and its description 14, padded 16; the tablet's form sheet a panel of 520 by 360 round 16 with an edge all round. The gallery's page has them.
- To what reads the tree (`lurq_snapshot` of the MCP server) the panel is a `dialog` called by its title, or by its label.
- A renderer that draws another sheet has `view.panel()`, `view.grabber()`, `view.heading()`, `view.close()`, `view.header()`, `view.body()`, `view.footer()`, `view.separator()`, `view.contents()` (all of them in order), `view.veil(panel)` and `view.modal(panel)`.

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
| `width(f32)`, `height(f32)` | A width of its own at a side (300 without one), a height of its own at the top or the bottom |
| `overlay(bool)` | Over the page with a veil, as a [sheet](#sheet) at its edge, rather than beside it: for a window too narrow to push the page aside |
| `scroll(bool)` | Whether what it holds scrolls where it is too small. It does unless told; what does not is laid out at the panel's height, beside the page and over it, for what holds a scroll of its own with something that stays beside it: a [sidebar](#sidebar) with its header and its footer |
| `dismiss_on_veil(bool)`, `dismiss_on_escape(bool)` | Over the page, whether a press on the veil, and Escape, close it. Both on by default |
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
  .fact(icons::CALENDAR_DAYS, "Joined March 2024")
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
| `media(element)` | What stands at its start, beside the text: an avatar, the icon of a file |
| `fact(icon, &str)`, `plain_fact(&str)` | A fact in the row under the description, with an icon of the theme's set or without one |
| `child(element)`, `children(..)` | What it holds under its text: a link, a row of buttons |
| `placement(Placement)` | The side of the element it is on and the edge it lines up with: `Bottom`, centered, by default |
| `arrow(bool)` | A triangle with the panel's edge from the panel to the element |
| `open_delay(..)`, `close_delay(..)` | A `MotionDuration` of the theme's or a `Duration`: how long the pointer rests before it opens (`Slower`, 400 ms) and how long it stays after the pointer left (`Slow`, 240 ms) |
| `open(Option<bool>)` | `Some(true)` shows it whatever the pointer and the focus do, `Some(false)` never |
| `disabled(bool)` | It never opens, and the element is as it would be without it |
| `width(..)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [popover](#popover) has them. `id` is the id of what shows |

- It opens when the pointer has rested on the element for its open delay, and at once when the keyboard's focus comes to the element or to anything in it. It stays open while the pointer is on the element, on the card or crossing the room between them (the room is the card's root, which the pointer is on), and while the focus is in either; it closes when the pointer has left both for its close delay, when the focus leaves both, and on Escape, which it takes before anything under it. After Escape it stays closed until the pointer comes again or both the pointer and the focus have left.
- Tab goes from the element into the card's controls and on to what follows the element (`Overlay::tab_after_anchor`): the focus is not moved into it as it opens, which is what `Overlay::takes_focus` would do. As it closes with the focus inside, the focus goes back to the element.
- One hover card of the pointer shows at a time. Where there is no room on its side it goes to the other, its arrow turned with it, and where the window pushes it along, its arrow still points at the middle of the element.
- To what reads the tree it is a `dialog` where it holds a control and a `tooltip` where it does not, called by its title; the element is described by the title and the description (`a11y_description`), and says it has a dialog where the card has controls.
- It is a mounted component, as the tooltip is. Converted into an element on its own, a `HoverCard` is its panel as it shows, anchored to nothing.
- Parts: `HoverCardPart::Root` (the panel and its arrow, and the room between them and the element), `Panel`, `Media`, `Text`, `Title`, `Description`, `Facts`, `Fact`, `FactIcon`, `Arrow`. States: `HoverCardState::Arrow`, `Controls`, and the side it is on by the names of `TooltipSide` (`"top"`, `"bottom"`, `"left"`, `"right"`).
- In the design the panel is 300 wide on `Popover` in an edge of 1 in `Border`, `Lg` round, with the `Md` shadow and 16 of padding, the media 12 before the text. The text is 6 apart: the title in 14 at medium weight, the description in 13 of `MutedForeground` in lines of 1.5, and the facts 12 apart, each an icon of 14 6 before its text in 12, both muted. It stands 8 from the element; the arrow is 18 along the panel and 9 out from it. It fades in over the base duration and out over the fast one.

## Toggletip

A tooltip opened by a press: a small button beside a label that opens a few words about it, for what must be reachable by a finger and told by a screen reader.

```rust
use lurq::ui::Toggletip;

let help = Toggletip::new("What is a workspace?")
  .content("A workspace holds the projects a team shares.")
  .link("Learn more", |_| open_docs())
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `Toggletip::new(&str)` | What its button is called to tooling: the question the panel answers |
| `mount(ctx)` | Puts its button where it goes, and its panel over the window while it is open |
| `content(&str)` | What the panel says |
| `link(&str, handler)` | A link under the text, with an arrow after it |
| `child(element)`, `children(..)` | What the panel holds under its text: a button |
| `icon(name)` | The button's icon, `icons::INFO` unless this says otherwise |
| `placement(Placement)` | The side of the button the panel is on: `Top` by default |
| `arrow(bool)` | A triangle from the panel to the button: the tooltip's |
| `open(Option<bool>)` | `Some(true)` shows the panel whatever the button says, `Some(false)` never |
| `disabled(bool)` | The button takes nothing |
| `width(..)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `key` | As a [popover](#popover) has them |
| `id` | The id of the button; the root of the panel is `<id>-panel` |

- A press of the button opens the panel and another closes it; a press outside it and Escape close it too. The focus stays on the button as it opens, and Tab goes from the button into the panel's link and on; as the panel closes with the focus in it, the focus goes back to the button.
- To what reads the tree the button is called by its label and says whether it is expanded (`a11y_expanded`), and the panel is a `status`.
- It is a mounted component: it keeps whether it is open. Converted into an element on its own, a `Toggletip` is its panel as it shows.
- Parts: `ToggletipPart::Root`, `Trigger` and `TriggerIcon` (what it gives the theme's button and its icon), `Panel`, `Text`, `Link` and `LinkLabel` (what it gives the theme's link and its label), `Arrow`. States: `ToggletipState::Open`, `Arrow`, `Disabled`, and the side as a hover card has it.
- In the design the panel is 230 wide on `Primary`, `Md` round, with 12 of padding and 8 between what it holds. Its text is 13 in `PrimaryForeground` in lines of 1.4, and its link 13 at 600 with an arrow up and right of 14, underlined in `PrimaryForeground` under the pointer. The button is a ghost one of 20 that is a circle, its icon of 14 in `MutedForeground`, in `Foreground` under the pointer and while the panel is open. The panel stands 6 from the button, and its arrow is the tooltip's, 12 along and 6 out.

## Teaching Tip

A panel that points at one thing in the app and says what it is for, the first time a person comes to it or when it is new, without taking the focus from what they are doing. A tour shows several, one after another.

```rust
use lurq::ui::{Icon, TeachingTip, TeachingTipVariant, Tour, TourStep, icons};

// anchor: ElementRef, open: Signal<bool>, kept by the app.
let close = open.clone();
let dock = TeachingTip::new(anchor.clone(), open.clone())
  .media(Icon::new(icons::SPARKLES).size(24.0))
  .title("Dock panels anywhere")
  .description("Drag a panel by its header and drop it on any edge of the window.")
  .primary("Got it", move |_| close.set(false))
  .mount(ctx);

// step: Signal<Option<usize>>, which step shows; None while the tour is not on.
let tour = Tour::new(step.clone())
  .step(TourStep::new(theme_button, "Switch themes here").description("Light and dark palettes share the same roles."))
  .step(TourStep::new(panels, "Dock panels anywhere"))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `TeachingTip::new(anchor, open)` | Pointing at `anchor` while the `Signal<bool>` is `true` |
| `mount(ctx)` | Puts it in the app; the element it returns has no size and goes anywhere |
| `title(&str)`, `description(&str)` | What it points at, and what it is for |
| `media(element)` | A picture or an icon at its top, on a muted fill as wide as it |
| `child(element)` | Something it holds under the description |
| `step(n, of)` | That it is a step of several: "2 of 4" in its footer, in the tour's look |
| `primary(&str, handler)`, `secondary(&str, handler)` | The button that goes on, and the one before it |
| `on_close(Fn())` | Told as its close control or Escape closes it, after which the signal is `false` |
| `close_button(bool)` | A close control at the end of its title: on by default |
| `variant(TeachingTipVariant)` | `Plain` (the default), in the popover's colors, or `Primary` |
| `placement(Placement)` | The side of what it points at it is on: `Bottom` by default |
| `arrow(bool)` | A triangle with the panel's edge pointing at what it is about: on by default |
| `takes_focus(bool)` | The focus goes into it as it opens and back as it closes. Off by default |
| `width(..)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `key` | As a [popover](#popover) has them |
| `id` | The id of what shows; its close control is `<id>-close`, its buttons `<id>-primary` and `<id>-secondary` |

- It does not keep whether it is open: the app's signal does. Its close control and Escape close it, set the signal to `false` and tell `on_close`; a press elsewhere does not close it, so a person can go on with what they are doing beside it. Its own buttons do what their handlers say.
- It does not take the focus unless the app says so: Tab goes from what it points at into its buttons and on (`Overlay::tab_after_anchor`). With `takes_focus(true)` the focus goes into it as it opens and back to what had it as it closes.
- To what reads the tree it is a `dialog` called by its title. Where there is no room on its side it goes to the other, its arrow turned with it, and the arrow points at the middle of what it is about.
- `Tour::new(Signal<Option<usize>>)` shows the step the signal says, counted from 0, as a teaching tip with its count and no close control, with "Back" from the second step on and "Next", or "Done" on the last (`labels(back, next, done)`). Its buttons set the signal; Done and Escape set it to `None`, and Done tells `on_finish`. `TourStep::new(anchor, title)` has `description`, `media` and `placement`; the tour has `variant`, `placement` and `arrow` for all its steps.
- Teaching tips and tours are mounted components: a tip keeps which side the window let it take, and what Escape tells. Converted into an element on its own, a `TeachingTip` is its panel as it shows.
- Parts: `TeachingTipPart::Root`, `Panel`, `Media`, `Content` (the text over the footer), `Text`, `Header` (the title and the close control), `Title`, `Close` (what it gives the theme's button), `Description`, `Footer`, `Step`, `Actions`, `PrimaryAction` and `SecondaryAction` (what it gives the theme's buttons), `Arrow`. Variants: `TeachingTipVariant::Plain`, `Primary`. States: `TeachingTipState::Arrow`, `Stepped` (it has a count), `Media`, and the side as a hover card has it.
- In the design a teaching tip is 300 wide on `Popover` in an edge of 1 in `Border`, `Lg` round, with the `Lg` shadow: media 96 high on `Muted`, then 14 of padding around the text and the footer 10 under it. The title is 14 at 600 with a ghost close control of 20 at its end, its icon of 16 in `MutedForeground`; the description 6 under it is 13 of `MutedForeground` in lines of 1.45; the buttons, 8 apart at the end of the footer, are 30 high and 12 from their sides, the primary one `Primary` and the other `Outline`. It stands 4 from what it points at; its arrow is 18 along the panel and 9 out from it, with the panel's edge. A step of a tour is the board's Tour: 280 wide, padded 16, its title at 500 over the description 4 under it in lines of 1.5, its footer 12 under them with the count in mono 11 of `MutedForeground` at its start and buttons 28 high and 10 from their sides. In the `Primary` look the panel is `Primary` in its own color's edge, all it says in `PrimaryForeground`, the media on a white of 10%, and the buttons `Secondary` and `Ghost`. It fades in and out as the popover does.

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
  .footer(true);

// Commands an app fetches as a person types: it keeps every one it was given.
CommandPalette::new(open.clone())
  .query(query.clone())
  .on_input(move |typed| fetched.set(search(typed)))
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
| `on_input(Fn(&str))` | Called with what the input will hold at each edit: for an app that fetches its commands |
| `query(Signal<String>)` | Where the app keeps what is typed. Without one the palette keeps it while it is open, and it is empty when it opens again |
| `loading(bool)` | The list ends with a spinner and "Loading…" |
| `empty(&str)` | What the list says when nothing is found: "No results" |
| `footer(bool)` | A row at the bottom, under a line, of the keys that work in it: ↑ ↓ to move, ↵ to run, Esc to close |
| `inline(bool)` | The panel drawn where it is declared, open or not, without the veil: for a gallery or a test |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's |
| `CommandItem::new(label)` | A command: what it does, or where it goes |
| `icon(name)`, `detail(&str)`, `shortcut(&str)` | An icon of 16 before the label; a second text after it, muted; the keys that do the same at the end, as a [menu](#menu) writes them (the app binds them) |
| `keywords([..])` | Words that find it and are not shown |
| `disabled(bool)`, `on_select(Fn())`, `id`, `key` | Dimmed, and passed by; called when it is run, before the palette closes; the row's id and key |

- It stands near the top of the window over the veil, as the engine's `Modal`: the focus goes to its input as it opens and back to what had it as it closes, and Tab stays inside it, as a [dialog](#dialog)'s does.
- The focus stays in the input. Down and Up move the highlight through the rows that are shown, across groups and round at the ends, past a disabled one, and scroll the list to it; the pointer moves it too, and a row shows it as a menu item shows the keyboard's focus. Enter runs the highlighted command and closes the palette; a click on a row does the same. Escape clears what was typed, and closes the palette when nothing is. Typing puts the highlight on the first row it shows.
- To what reads the tree it is a `dialog` holding a `combobox` (the input, `expanded`, with the highlighted command's label as its `a11y_activedescendant`) and a `listbox` of `option`s with `a11y_selected`.
- Parts: `CommandPalettePart::Veil`, `Panel`, `Search` (the row at the top), `SearchIcon`, `Input` (laid over the theme's input), `Key` (laid over the [Kbd](#kbd) key of the search row's Escape and of the footer), `Results` (what scrolls: its most height), `List` (the room around the rows), `GroupLabel`, `Row` (laid over a menu item's root), `Detail`, `Empty`, `Status`, `Footer`, `FooterHint`. The rows are a [menu](#menu)'s items: a theme's rules for `MenuItem` style them too. States: `CommandPaletteState::Loading`, `Empty`.
- In the design the panel is 420 wide on `Popover` in an edge of 1 in `Border`, `radius-xl`, the large shadow, 72 from the top of the window over `Scrim`. The search row is 44 high, 14 in, with 10 between the magnifier of 16 in `MutedForeground`, the input in the body's type (no edge or fill of its own) and the key, and a line of `Border` under it. The list is padded 4 and scrolls past 320; a group's label is 12/500 in `MutedForeground`, padded 6 and 8; a row is a menu item, 32 high on `Highlight` while it is highlighted. The keys are the theme's [Kbd](#kbd), in the platform's names (`Enter` is ↩ on a Mac), and a row's shortcut is a Kbd's plain text, as a menu writes it, with the menu item's `Shortcut` part laid over its label.
- A renderer that draws another palette has `view.panel()`, `view.search()`, `view.input()`, `view.key(label)`, `view.results()`, `view.group_label(label)`, `view.row(index)`, `view.empty_state()`, `view.status()`, `view.footer_row()`, `view.veil(panel)` and `view.modal(panel)`, with `view.shown()`, `view.command(index)` and `view.highlighted`.

## Context Menu

A [menu](#menu) of what can be done with an element, opened where the element is pressed with the secondary button or held with a finger: a file in a list, a node of a canvas, selected text.

```rust
use lurq::ui::{ContextMenu, MenuItem, icons};

ContextMenu::new(file_row, open.clone())
  .item(MenuItem::new("Open").icon(icons::FOLDER_OPEN).on_select(open_file))
  .item(MenuItem::new("Rename").icon(icons::PENCIL).shortcut("F2").on_select(rename))
  .separator()
  .item(MenuItem::new("Delete").icon(icons::TRASH).destructive(true).on_select(delete));

// The board's edit menu: the commands of selected text in one row.
ContextMenu::new(editor, editing.clone())
  .bar(true)
  .items(["Cut", "Copy", "Paste", "Select All"].map(MenuItem::new));
```

| Builder | |
| --- | --- |
| `ContextMenu::new(target, open)` | The element the menu is for, which lays out as it would without it, and the app's `Signal<bool>`: what asks for the menu sets it, and choosing an item, a press outside and Escape clear it |
| `item(MenuItem)`, `items(..)`, `separator()`, `label(&str)` | As a [menu](#menu) has them: icons, shortcuts, checks, radios, submenus, a destructive item |
| `bar(bool)` | The commands in one row, as the board's edit menu over selected text has them. Left and Right move between them; a separator is a line down the row, and a label is left out |
| `disabled(bool)` | The element has no menu now |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the panel's |

- It opens at the pointer on a press of the secondary button on the element (and on Control and a press on macOS), at the finger on a long press, and under the element from its start on the Menu key or Shift+F10 while the focus is in the element. A press asks for the menu of the innermost element that has one; asked for again on its element, it moves to the new point.
- It stays inside the window: where there is no room down or toward the end of the point, it opens up or back from it. One context menu is open at a time: asking for another closes the first.
- It is a [menu](#menu): the focus goes to its first item as it opens and back to what had it as it closes, Up and Down move between the items, a letter goes to the item it starts, Enter chooses, a submenu opens beside its item. To what reads the tree it is a `menu`, and the element says it has one (`a11y_haspopup`).
- Parts: `ContextMenuPart::Panel` (laid over the menu's panel), `Bar` (the panel of a bar), `BarItem` and `BarLabel` (laid over a menu item's root and label in a bar); the menu's and its items' parts are a menu's. State: `ContextMenuState::Bar`.
- In the design the menu is a menu at least 220 wide, its corner at the pointer. The bar is a row on `Popover` padded 3, `radius-lg`, in an edge of `Border` with the large shadow; its items are 28 high, padded 10, their labels 13.
- The engine places it: `Popup::at(Some((x, y)))` (`Overlay::at`) stands a popup at a point of the window and keeps it inside, which an app may use for a popup of its own.

## Sidebar

The way round an app, down one side of its window: its groups of places, the one the app is at, a header with the app or the workspace and a footer with who is signed in.

```rust
use lurq::ui::{Avatar, Sidebar, SidebarGroup, SidebarHeading, SidebarHeadingVariant, SidebarItem, SidebarTrigger, icons};

let heading = |collapsed| SidebarHeading::new("lurq").description("Workspace").icon(icons::BOX).collapsed(collapsed);
let account = SidebarHeading::new("Ivan Zaida")
  .variant(SidebarHeadingVariant::Account)
  .description("ivan@lurq.dev")
  .mark(Avatar::new("Ivan Zaida"));

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
  .collapsed(collapsed.clone())
  .open(open.clone())
  .mount(ctx);

// Wherever the app puts it: collapses the sidebar, or opens it over the page on a narrow window.
SidebarTrigger::new(collapsed.clone()).open(open.clone());
```

| Builder | |
| --- | --- |
| `label(&str)` | What the navigation region is called to tooling. "Sidebar" unless told |
| `header(element)`, `footer(element)` | What stays at its top and its bottom while its items scroll: a `SidebarHeading`, a search. The footer has a line over it |
| `header_collapsed(element)`, `footer_collapsed(element)` | What the rail shows in their place: the mark alone, an avatar. Nothing unless told |
| `group(SidebarGroup)`, `groups(..)`, `item(SidebarItem)` | Groups of items; `item` adds to a group of no label |
| `collapsed(bool \| Signal<bool>)` | The rail of the items' icons, 52 wide. Its width moves between the two over the theme's slow duration |
| `open(Signal<bool>)` | On a narrow window, whether it is open over the page. `SidebarTrigger` sets it; choosing an item, Escape and a press on the veil clear it |
| `narrow(bool)` | Whether it lies over the page whatever the window's width: it does below the `md` breakpoint unless told |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the root |
| `mount(ctx)` | Puts it where it goes. Converted into an element on its own it is drawn as it is at rest, with no tooltips on the rail: for a gallery or a test |

| `SidebarGroup` | |
| --- | --- |
| `new(&str)` | A group under a label, written in capitals; an empty one has none |
| `count(usize)` | How many it has, at the end of its label |
| `action(element)` | An action at the end of its label: a small ghost button. The arrow keys of the items pass it by |
| `collapsible(bool)`, `open(bool)` | Its label opens and closes it; `open` is how it starts, and the sidebar keeps it from then on |
| `item(SidebarItem)`, `items(..)`, `key` | Its items; `key` tells two groups of one label apart |

| `SidebarItem` | |
| --- | --- |
| `new(&str)`, `icon(name)` | Its name, and the icon before it: what the rail shows of it (the first letter of its name where it has none) |
| `count(usize)`, `badge(element)`, `shortcut(&str)` | At its end: a count in the mono family, a `Badge`, the keys that go to it as `Kbd::shortcut` writes them |
| `selected(bool)` | It is where the app is: the app's, following its route |
| `disabled(bool)` | It is dimmed and takes nothing |
| `on_select(handler)`, `href(path)` | What choosing it does: the handler, and with the `router` feature the path the router goes to. A press, Enter and Space choose |
| `child(SidebarItem)`, `children(..)`, `open(bool)` | Nested items, which open and close with their height under it; `open` is how they start, open where one of them is selected unless told |
| `force(..)`, `id`, `key` | As a button's. `key` tells two items of one name apart |

- The items are a tree to the keyboard (`FocusGroup::vertical().one_stop(true).typeahead(true).nested(true)`, each item at its `focus_level`, one with nested items `focus_expanded`): Tab stops at it once, on the selected item; Up and Down move through the items shown, nested ones included, Home and End go to the first and the last, and a letter to the item whose name starts with it. Right opens a closed item and on an open one goes to its first nested item; Left closes an open one and on a nested item goes to the item it is in. A group that opens and closes is an item its items are in. On the rail the items are a list, and Right opens an item's menu.
- Collapsed, it is the rail: each item is its icon, says its name (and its keys) in a `Tooltip` at its right, and an item with nested items opens them in a `Menu` there, the selected one marked. An item whose nested item is selected is the selected one on the rail.
- On a window narrower than the `md` breakpoint it lies over the page in a `Drawer` with `overlay(true)` and `scroll(false)`, as wide as it is beside the page, its header and footer staying while its items scroll; whatever `collapsed` says, it is whole there. Choosing an item closes it.
- Which item is selected is the app's: with a router, `.selected(ctx.route_path() == path)` on each, and `.href(path)` to go there. The gallery's shell does this for its pages.
- `SidebarTrigger::new(collapsed).open(open)` is a ghost button of the `panel-left` icon, called by which way it goes: "Collapse sidebar", "Expand sidebar", or on a narrow window "Open sidebar", "Close sidebar". `narrow(bool)` tells it as the sidebar is told.
- `SidebarHeading::new(title)` is the row of the header and the footer: `description`, `icon(name)` (a mark on the primary fill) or `mark(element)` (an avatar), `trailing_icon(name)`, `variant(SidebarHeadingVariant::Workspace | Account)`, `collapsed(bool)` (the mark alone, for the rail), `on_click` and `anchor(ElementRef)` (a button, from which a switcher's menu can hang).
- Parts: `SidebarPart::Root`, `Header`, `Body` (what scrolls), `Footer` (its room at the sidebar's sides), `FooterContent` (the line and what is under it), `Group`, `GroupLabel`, `GroupText`, `GroupCount`, `GroupChevron`, `GroupAction`, `Item`, `ItemIcon`, `ItemLabel`, `ItemCount`, `ItemBadge`, `ItemShortcut`, `ItemChevron`, `ItemIndent` (one for each level an item is nested), `Nested`, `NestedContent`, `Heading`, `HeadingMark`, `HeadingText`, `HeadingTitle`, `HeadingDescription`, `HeadingTrailing`. States: `SidebarState::Collapsed`, `Overlay`, `Selected`, `Disabled`, `Open`, `Interactive` (a group's label that is pressed, a heading with a handler). Variants of a heading: `SidebarHeadingVariant::Workspace`, `Account`.
- In the design the sidebar is 260 wide on `Muted` with a line of 1 in `Border` along its end, padded 8, 2 between rows. The 8 at its sides is its header's, body's and footer's: the body reaches the sidebar's edge, and at its end it has the lane of the theme's scroll bar (its width and its inset on either side, 12 with the default bar) where that is more than 8, so that the bar, which is drawn over what scrolls, lies beside the rows and not over them. The rail shows no bar: it scrolls with the wheel and the keys. An item is 32 high (it follows the density: 24, 40 and 44 at compact, comfortable and touch), padded 8 across, 8 between an icon of 16 in `MutedForeground` and its name in 13; the selected one is on `Background` with an edge of `Border`, its name in the medium weight and its icon in `Foreground`. Under the pointer an item has the `StateHover` veil, held down `StatePressed`, and the ring of the focus is inside it. A group's label is 11/600 in capitals, 0.6 apart, in `MutedForeground`, 10 over it and 4 under it. A nested item is 18 further in, after the chevron of 14 of the item it is in. The heading of the header is 44 high with a mark of 28 on `Primary`, its title 13/600 over a description of 11 in `MutedForeground`; the footer's is under a line of 1 and 48 high, its name in the medium weight. The rail is 52 wide, its items 36 by 32, 4 apart.
- It is a mounted component: the tooltips of the rail are mounted, and it keeps which nested lists and groups are open and the anchor and the open signal of each menu of the rail.
- To what reads the tree it is a `navigation` called by its label, its items `link` (with `href`) or `button`, the selected one with `a11y_current` `page`, an item with nested items with `a11y_expanded`, and a nested list a `group`.
- A theme's renderer (`SidebarRenderer`) is given a `SidebarView` with the groups built, the header and the footer as the sidebar is now, and `header()`, `body()`, `footer()` and `root(children)` to lay them out; `render()` is the default.

## Menubar

The menus of an app in a row, File, Edit, View, each opened from its title, as the title bar of a window holds them on Windows and Linux.

```rust
use lurq::ui::{Menu, MenuEntry, MenuItem, Menubar, icons};

// open: Signal<Option<usize>>, the menu open now; export: ElementRef, exporting: Signal<bool> for a submenu.
Menubar::new(open)
  .menu("File", [
    MenuItem::new("New").icon(icons::FILE).shortcut("Ctrl+N").on_select(move || new_file()).into(),
    MenuItem::new("Open…").shortcut("Ctrl+O").on_select(move || open_file()).into(),
    MenuEntry::Separator,
    MenuItem::new("Export as").submenu(Menu::new(export, exporting).items(["PNG", "SVG", "PDF"].map(MenuItem::new))).into(),
    MenuEntry::Separator,
    MenuItem::new("Quit").shortcut("Ctrl+Q").on_select(move || quit()).into(),
  ])
  .menu("Edit", [
    MenuItem::new("Undo").icon(icons::UNDO).shortcut("Ctrl+Z").on_select(move || undo()),
    MenuItem::new("Redo").shortcut("Ctrl+Y").disabled(true),
  ])
  .menu("View", [MenuItem::new("Show grid").checked(grid.get()).on_select(move || toggle_grid())])
  .menu("Help", [MenuItem::new("About")])
  .title_disabled("Help", !online.get());
```

| `Menubar` | |
| --- | --- |
| `new(open)` | Which menu is open, by its index among the bar's: the app's `Signal<Option<usize>>`. A press of a title sets it; choosing an item, a press outside and Escape set it to `None`; the app may set it too |
| `menu(title, entries)` | A menu called `title`, after the bar's others, holding [menu](#menu) items: `MenuItem`s, with `MenuEntry::Separator` and `MenuEntry::Label` between them (`.into()` each where the list mixes them) |
| `title_disabled(title, bool)` | The menu of that title cannot be opened: its title is dimmed, and the pointer, the arrows and Alt pass it by |
| `disabled(bool)` | No menu of the bar can be opened |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the bar's; its titles are `{id}-0`, `{id}-1` and on, and each menu's panel `{id}-0-menu` |

- Its menus are [menus](#menu), with all their items have: icons, shortcuts, checks, radio items, submenus, disabled and destructive items, separators and labels. One is open at a time.
- The pointer: a press of a title opens its menu, and a press of it again closes it. While a menu is open the pointer coming onto another title opens that one, with no press. Choosing an item, a press outside and Escape close the menu.
- The keyboard: the bar is one stop of Tab (a [focus group](../focus-navigation/#focus-groups) of one stop). Left and Right move between the titles and go round at the ends, a letter goes to the title that starts with it, and a disabled title is passed by. Down, Enter and Space open the focused title's menu with the focus on its first item, Up on its last that can be chosen. In an open menu Left and Right close it and open the menu beside it, at its first item: Left in a submenu closes the submenu first, Right on an item with a submenu opens it, and Right on one without goes to the next title from a submenu too. Escape closes the menu and gives the focus back to its title.
- Alt: pressed and let go alone, Alt puts the focus on the first title and underlines the first letter of each title; Alt alone again gives the focus back to what had it, closing a menu that was open. The letters show while Alt is held down too. Alt with a title's first letter opens its menu from wherever the focus is, and from a menu of the bar opens the other; several titles with one first letter are gone round. On a Mac, where Option with a letter types a character and the app's menus are the system's, Alt does neither.
- Under the pointer, focused from the keyboard, or with its menu open, a title is filled with `Highlight`, the board's `accent`. In the design the bar is 36 tall in `Background`, padded 4, with the medium radius and an edge of 1 in `Border`, its titles 2 apart; a title is 28 tall, padded 10 at its sides, with the small radius, its text 13/500 in `Foreground`. A disabled title is dimmed. A menu opens under its title from its start, 4 below it.
- The engine's part is `FocusGroup::menu_row(true)`, which the bar's row has: a row of elements each of which opens a menu hung from it when it is pressed. While one of those menus has the focus, the Left and Right that nothing in the menu takes close it, with the menus opened from it, and press the element beside it; Alt with a letter and Alt alone act on the first such row of the window. A bar of your own, of buttons and [menus](#menu), gets the same keys from it.
- The bar keeps, for the signal it is given, a signal for each menu kept as one with the app's, the anchor each menu hangs from, and what Alt shows, while it is in the tree.
- Parts: `MenubarPart::Bar`, `Title` (what is pressed, and what is lit), `Label` (its text). States: `MenubarState::Open` (the title's menu is open) and `Disabled` (the bar, or the title, cannot be opened); a title is resolved in the states of the bar and its own.
- To what reads the tree the bar is a `menubar`, each title a `menuitem` called by its text with `a11y_haspopup` `menu` and `a11y_expanded`.
- A renderer for `Menubar` gets a `MenubarView`: `view.bar()` is the row with its style, its focus group and its handlers; `view.title(index)` a title's behavior with its style, `view.label(index)` its text, and `view.menu(index)` its menu, hung from the title, to put after it; `view.title_part(index, part)` is the style of a part of one title. `view.render()` builds the bar.

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
  .child(ToolbarItem::from(Toggle::new("Bold").icon(icons::BOLD).icon_only(true).on(bold.get())).shortcut("Ctrl+B"))
  .child(Toggle::new("Italic").icon(icons::ITALIC).icon_only(true).on(italic.get()))
  .child(Toolbar::separator())
  .child(Button::new("Link").icon(icons::LINK).icon_only(true).on_click(link))
  .child(Toolbar::spacer())
  .child(ToolbarItem::from(Button::new("Search").icon(icons::SEARCH).icon_only(true)).stays(true))
  .overflow(true)
  .mount(ctx);

// A command bar's groups: a label under each, and the group goes into the menu whole.
Toolbar::new()
  .variant(ToolbarVariant::Plain)
  .child(ToolbarGroup::new().label("Clipboard").child(Button::new("Paste").icon(icons::PASTE)))
  .mount(ctx);
```

| Builder | |
| --- | --- |
| `Toolbar::new()` | An empty bar on a surface, its controls of the small size |
| `child(impl Into<ToolbarItem>)`, `children(..)` | A `Button`, a `Toggle`, a `ToggleGroup`, a `Select`, a `ToolbarGroup`, `Toolbar::separator()`, `Toolbar::spacer()`, an `Element`, or a `ToolbarItem` made of any of them |
| `mount(ctx)` | Puts the bar in the tree of `ctx`'s component: it keeps the menu of what does not fit and the control the focus left. `build(theme)` and `From<Toolbar> for Element` draw it once with nothing kept: no tooltips, no menu |
| `variant(ToolbarVariant)` | `Surface` (the default): the background's fill and an edge. `Plain`: neither. `Floating`: the popover's fill, an edge and the large shadow |
| `size(ButtonSize)` | The size of its buttons and toggles: `Sm` by default, so the bar is 40 high |
| `overflow(bool)` | Where the bar is narrower than its controls, those that do not fit go into a menu behind a More button at its end, the last first. Without it they are cut off |
| `label(&str)` | What the bar is called, to what reads the tree |
| `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. The More button has the id with `-more` after it, its menu `-menu` |

| `ToolbarItem` | |
| --- | --- |
| `shortcut(&str)` | Keys shown in the tooltip of an icon alone and beside its item in the menu |
| `stays(bool)` | The control never goes into the menu |
| `as_given(bool)` | The button or toggle keeps its own variant, size and style over the bar's: the outline Export of a table's bar |
| `ToolbarItem::element(el)`, `in_menu(ToolbarCommand)` | Any element: it stays in the bar unless it has a command to stand for it in the menu |

- The bar gives its buttons its size and makes them ghost ones, and its toggles its size, unless an item is `as_given`. A button or toggle that is its icon alone is called by its label and has a tooltip (below it) of its label and its shortcut.
- It is one stop of Tab: Tab enters it at the control the focus last left (the first, the first time), and leaves it at once. Left and Right move between its controls, the toggles of a group in it among them; Home and End go to the first and the last; a disabled one is passed by. A field or a select in it keeps its own keys while it is focused.
- With `overflow`, a control that does not fit drops out of the bar whole, the last first, and the More button (`more-horizontal`, after a line) appears at the end; its menu has an item for each control that went, in the order of the bar: its label, its icon, its shortcut, a check for a toggle (each toggle of a group, after a line), and its press. A labelled group goes whole, under its label. A select or a field stays unless it has `in_menu`. The bar learns what went from the layout (the engine's `ShrinkLimit::Drop` and `drop_marker`), and its menu follows it a pass later.
- Parts: `ToolbarPart::Root`, `Item` (laid over each button's and toggle's root), `Slot` (a control with the line before it, 2 apart), `Separator` (laid over a separator's root), `Spacer`, `Group`, `GroupItems`, `GroupLabel`, `More`. State: `ToolbarState::Overflowing`. Variants: `ToolbarVariant::Plain`, `Surface`, `Floating`.
- In the design the bar is padded 4, its controls 2 apart; a button is a ghost one of 32 with an icon of 16; a toggle that is on has the fill of what is picked out; a line of 20 in the border's color stands between two groups with 4 on either side. On a surface it has the background's fill, an edge of 1 in the border's color and the middle radius. A group of a command bar is padded 10 with a line at its end and its label of 10 in the muted foreground under it. A focused control has its ring inside it, where the bar does not cut it off.
- To what reads the tree it is a `toolbar` called by its label, with `a11y_orientation`; the More button says `a11y_haspopup` and `a11y_expanded`.
- A renderer that draws another bar gets a `ToolbarView`: `view.slot(index)` is a control as the bar lays it out (dropping when it does not fit), `view.more()` the More button with its menu, `view.menu()` the menu alone, `view.dropped` which slots went.

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
      .escape_clears(true),
  );
```

| Builder | |
| --- | --- |
| `ActionBar::new(count)` | The bar of a selection of `count`: shown while it is more than none |
| `text(&str)` | What it says, in place of "3 selected" |
| `action(Button)`, `destructive_action(Button)` | An action on what is selected: a round ghost button; one that destroys in the destructive color |
| `on_clear(handler)` | What clearing does: the round button with a cross at the end (none without it), and Escape where it clears |
| `escape_clears(bool)` | Escape clears the selection while the bar is shown, wherever the focus is, unless what has it took the key |
| `placement(ActionBarPlacement)` | `Bottom` (the default) or `Top` of the room it is given |
| `clear_label(&str)` | What the clear button is called: "Clear selection" |
| `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. The clear button has the id with `-clear` after it |

- It is as large as the room it is given and takes no press itself: put it over what it acts on, in a `Stack`. The bar stands 16 from the bottom (or the top) of that room, in the middle; it comes in with a fade and a move of 16 from that edge, and leaves the same way as the count goes to none.
- It is a toolbar to what reads the tree, called by what it says, and one stop of Tab whose Left and Right move between its buttons.
- Parts: `ActionBarPart::Root`, `Bar`, `Count`, `Separator` (laid over a separator's root), `Action`, `ActionIcon`, `ActionLabel`, `Destructive` (laid over a button's root, icon and label), `Clear`, `ClearIcon`. State: `ActionBarState::Empty`. Variants: `ActionBarPlacement::Bottom`, `Top`.
- In the design the bar is 44 high, of the popover's color, round at its ends, with an edge of 1 in the border's color and the large shadow, padded 16 before its count and 6 after its clear button, its parts 4 apart. The count is 13/600; a line of 20 with 6 on either side sets it apart; an action is 32 high, padded 10, its icon 14 and its label 13, 6 apart; the clear button is a round 32 with a cross of 15. A phone's selection bar is the same parts restyled: across the top, 56 high, in the fill of what is picked out.

## Splitter

Panes that share a room by the dividers between them, which a person drags: the side bar, the editor and the terminal of a window.

```rust
use lurq::ui::{Splitter, SplitterPane};

// layout: Signal<Vec<f32>>, saved with the window.
Splitter::horizontal()
  .sizes(layout.clone())
  .pane(SplitterPane::new(files).size(240.0).min(160.0).max(480.0).collapsible(true).label("Files"))
  .pane(SplitterPane::new(
    Splitter::vertical()
      .pane(SplitterPane::new(editor).min(120.0))
      .pane(SplitterPane::new(terminal).size(160.0).min(80.0).collapsible(true))
      .mount(ctx),
  ))
  .pane(SplitterPane::new(outline).share(0.2).min(100.0))
  .on_resize_end(|sizes| save(sizes))
  .mount(ctx);

// The board's Resizable: a grip on each divider, in a box of its own.
Splitter::horizontal().grip(true).framed(true).pane(sidebar).pane(content);
```

| Builder | |
| --- | --- |
| `Splitter::horizontal()`, `vertical()`, `new(orientation)` | Panes side by side, or one over the other |
| `pane(SplitterPane)`, `panes(..)` | A pane after the others, with a divider before it |
| `SplitterPane::new(content)` | A pane holding `content`, laid out in its whole width from its top: a height of `Dimension::Pct(100.0)` fills it |
| `SplitterPane::size(px)`, `share(0..1)` | How large it is to begin with: pixels it keeps as the splitter grows and shrinks, or a share it keeps. A pane with neither takes the rest; where all have one, the last does |
| `SplitterPane::min(px)`, `max(px)` | The least and the most it is |
| `SplitterPane::collapsible(bool)` | It closes when a divider is dragged under half its least size, and opens by a drag back, a double press on the divider or Enter. The pane that takes the rest does not close |
| `SplitterPane::label(&str)`, `key(..)` | What it is called: the divider after it is called so to tooling |
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
- Converted into an element, a splitter keeps its sizes while what holds it does not render again, unless it is given `sizes`; mounted, it keeps them, and an app that sets `sizes` from outside (to restore a layout) sees the panes take them. Pixels set from a saved layout are the pane's pixels, and a pane of a share takes the share its pixels were of the room they were saved in.
- A drag renders nothing again while it moves: the panes are resized by sizes their element refs are given (`ElementRefMut::set_size`), and what is in them is only laid out again at the next pass. Nor does letting it go: tooling reads where it stands from the divider at once (`ElementRefMut::describe`).
- Parts: `SplitterPart::Root`, `Pane`, `Handle` (the room around a divider's line that takes the pointer and the focus), `Line`, `Bar` (the wider line under the pointer, dragged and focused), `Grip`, `GripIcon`. States: `SplitterState::Vertical`, `Framed`, `Disabled`.
- In the design a divider is a line of 1 in the `Border` color; under the pointer and focused from the keyboard it is 3 wide in `Ring`, and dragged in `Primary`. The room that takes the pointer is 12 across the line, over the edges of the panes beside it. The grip is 12 by 20 (20 by 12 across a vertical splitter), round by the small radius, in `Border`, with a grip icon of 10 in `Foreground`. Framed, the splitter has the `Background` fill, an edge of 1 in `Border` and the large radius.
- To what reads the tree a divider is a `separator` that can be focused, with `a11y_orientation` (that of its line), `a11y_valuenow` (the pixels of the pane before it), `a11y_valuemin`, `a11y_valuemax` (where it has a most) and the name of the pane before it, or the splitter's `label`. `a11y_valuenow` follows the divider as it is dragged and moved by the keys.
- Splitters nest, one orientation in the other: a pane holds a splitter of its own.
- A renderer that draws another splitter is given the view: `view.part(..)`, `view.thickness()` (the room between two panes), `view.sizes()`, and `view.render()` the default.

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
