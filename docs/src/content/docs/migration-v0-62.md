---
title: Migration To v0.62
description: The design system's theme becomes the default. What changes for an app written for lurq 0.61, how to keep the 0.61 look, and what to rename.
---

# Migration To v0.62

lurq 0.62 makes the design system's theme the default. An app written for 0.61 still compiles. What changes is how it looks until it says what it wants, and a set of palette roles that are deprecated in favor of the design's.

## What Changes Without A Line Of Code

| | lurq 0.61 | lurq 0.62 |
| --- | --- | --- |
| Palette | 23 roles on a blue accent | The design's 42 roles on a neutral primary |
| Dark look | None: one palette | A dark palette, taken when the system is dark |
| Text that names no color | Black | `Foreground`: near-black in the light palette, near-white in the dark one |
| Body text | 16 | 14 |
| Headings | Bold | Semibold, with tighter letter spacing |
| Code (`TypographyStyle::Mono`) | 16 | 13 |
| Radii | `Sm` 3, `Md` 5, `Lg` 6 | `Sm` 4, `Md` 6, `Lg` 8, and `Xl` 12, `Full` |
| A window whose root paints no background | Cleared to white | Cleared to `Background`: white in the light palette, near-black in the dark one |

The one that can make an app unreadable is the dark look. On a system in its dark look, text that names no color is drawn near-white. An app that paints light backgrounds of its own under such text has near-white text on them.

## Keep The 0.61 Look

One line, before the root is mounted:

```rust
use lurq::app::theme::ThemeDefinition;

app.theme().apply(ThemeDefinition::legacy());
```

`ThemeDefinition::legacy()` is the theme of 0.61: its palette color for color, with text that names no color in black, its typography (body text of 16) and its radii. It has one palette, so the app is light whatever the system is in. The theme goes on taking the system's text size and bold text, which only a phone reports.

An app that wants the new look and only its own light backgrounds kept readable stays in the light palette instead:

```rust
use lurq::app::window::Appearance;

app.theme().set_preferred_appearance(Some(Appearance::Light));
```

A change of a single value goes on a definition before it is applied, or on the theme after:

```rust
let mut theme = ThemeDefinition::legacy();
theme.light.primary = brand;
app.theme().apply(theme);
```

## Follow The System Properly

The other way is to let the app be dark when the system is. Then every color the app names has to come from the theme:

- Paint surfaces with roles, not with concrete colors: `PaletteColor::Background` for the window, `Card` for what is raised off it, `Popover` for what floats.
- Give text that sits on a filled surface that surface's foreground: `.foreground(PaletteColor::PrimaryForeground)` on a primary button colors every text in it.
- Leave ordinary text with no color: it takes `Foreground`.

## Renamed Palette Roles

The roles of 0.61 are deprecated. They compile with a warning that names the replacement.

| 0.61 | 0.62 |
| --- | --- |
| `Accent` | `Primary` |
| `AccentHover` | `PrimaryHover` |
| `AccentMuted` | `Highlight` |
| `SurfaceBase` | `Background` |
| `SurfacePanel` | `Canvas` |
| `SurfaceRaised` | `Card` |
| `SurfaceInput` | `Background` |
| `BorderStrong` | `Input` |
| `BorderFocus` | `Ring` |
| `TextPrimary` | `Foreground` |
| `TextSecondary` | `MutedForeground` |
| `TextMuted` | `MutedForeground` |
| `TextInverse` | `PrimaryForeground` |
| `Danger` | `Destructive` |
| `DangerMuted` | `DestructiveMuted` |

The same holds for the fields of `ThemePalette`: `palette.accent` becomes `palette.primary`, and so on.

Before:

```rust
app.theme().set_palette_color(PaletteColor::Accent, brand);
Rect::new(80.0, 32.0).background(PaletteColor::SurfaceRaised);
Text::new("Saved").color(PaletteColor::TextMuted);
```

After:

```rust
app.theme().set_palette_color(PaletteColor::Primary, brand);
Rect::new(80.0, 32.0).background(PaletteColor::Card);
Text::new("Saved").color(PaletteColor::MutedForeground);
```

What to know while both sets exist:

- **Two roles became one.** `TextSecondary` and `TextMuted` are both `MutedForeground`, and `SurfaceBase` and `SurfaceInput` are both `Background`. An app that gave the two different colors keeps a name of its own for one of them: `PaletteColor::extra("text-secondary")`.
- **A deprecated role still has a color of its own.** In the design's palettes it is the color of its replacement. In `ThemePalette::legacy()` it is the color it had in 0.61.
- **Setting a role sets the deprecated roles it replaces.** `Select`, the form controls, and a checkbox or a slider with no style of its own still read the deprecated roles, and follow a theme set through the new ones this way.
- **Setting a deprecated role sets only that role.** An app that sets `Accent` and uses a role's replacement elsewhere sets `Primary`.

## Smaller Body Text

`Text::new(...)` with no style is `TypographyStyle::Body`, which is 14 where it was 16: a layout that counted on the height of a line of text moves. An app that wants the new look at the old text size sets the one style:

```rust
use lurq::{app::theme::TypographyStyle, layout::text_style::TextStyle};

let mut body = app.theme().typography_style(TypographyStyle::Body);
body.font_size = 16.0;
app.theme().set_typography_style(TypographyStyle::Body, body);
```

A `ThemeTypography` written out field by field must add `sans` and `mono_family`, the theme's two font families; build one from `ThemeTypography::default()` or `legacy()` instead.

## A Color Set Once

`theme.set_palette_color(role, color)` now sets the role in the light palette and in the dark one: a brand color set once is the brand color in both looks. `theme.set_palette_color_for(appearance, role, color)` sets it in one. `theme.set_palette(palette)` sets the palette of the appearance the theme is in and leaves the other as it is; `theme.set_palettes(light, dark)` sets both.

## A Palette Written Out Field By Field

`ThemePalette` has a field for each new role. A palette built with every field named and no `..ThemePalette::default()` no longer compiles. Start from one of the built-in palettes:

```rust
use lurq::{app::theme::ThemePalette, node::color::Color};

let mut light = ThemePalette::light();
light.primary = Color::from_hex("#7c3aed");
let mut dark = ThemePalette::dark();
dark.primary = Color::from_hex("#a78bfa");
app.theme().set_palettes(light, dark);
```

`ThemePalette::default()` is now the design's light palette, not the blue one: a palette built on it with `..ThemePalette::default()` takes the design's colors for the fields it does not name.
