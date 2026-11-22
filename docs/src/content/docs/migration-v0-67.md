---
title: Migration To v0.67
description: What changes for an app written for lurq 0.66. Everything 0.63 deprecated is removed, and a few types gained a field or a variant for glass. What stops the build, and what to write instead.
---

# Migration To v0.67

lurq 0.67 removes what 0.63 deprecated. Since 0.63 each of these compiled with a warning that named what to write; in 0.67 they do not compile. Nothing on the screen changes for an app that had no such warning.

## Before You Update

Build the app on 0.66 first and fix every deprecation warning there: the warning names the replacement, and the app still runs while you work through them.

```sh
cargo build 2>&1 | grep -A4 "use of deprecated"
```

Two warnings stay in 0.67, because their types are still there: `lurq::components::Image` and `lurq::components::Checkbox`. See [What Is Still Deprecated](#what-is-still-deprecated).

## In Short

**What stops the build**

- A palette role of 0.61 (`PaletteColor::Accent`, `palette.text_muted`, ...): see [Palette Roles](#palette-roles).
- `FormTheme`, `theme.form()` and `theme.set_form(..)`: see [The Form Theme](#the-form-theme).
- `dark()` and `for_appearance(..)` of the phone's pieces: see [The Phone's Pieces](#the-phones-pieces).
- `components::ActivityIndicator`: see [The Spinner](#the-spinner).
- `WinitWindow::on_frame`: see [The Window's Frame Callback](#the-windows-frame-callback).
- A struct written out field by field, or a `match` with no `_` arm, over a type that gained a field or a variant: see [the table](#types-that-gained-a-variant-or-a-field).

**What looks different**

- Nothing, in the design's themes.
- In `ThemeDefinition::legacy()`, only what an app itself painted with `TextPrimary` or `TextSecondary`: see [The Legacy Palette](#the-legacy-palette).

## Palette Roles

The fifteen roles of 0.61 are gone, with their fields of `ThemePalette`. Each has one role of the design in its place:

| Removed | Write |
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

The fields of `ThemePalette` follow the same table: `palette.accent` is `palette.primary`, `palette.text_muted` is `palette.muted_foreground`.

Before:

```rust
app.theme().set_palette_color(PaletteColor::Accent, brand);
Rect::new(80.0, 32.0).background(PaletteColor::SurfaceRaised);
Text::new("Saved").color(PaletteColor::TextMuted);
let edge = palette.border_strong;
```

After:

```rust
app.theme().set_palette_color(PaletteColor::Primary, brand);
Rect::new(80.0, 32.0).background(PaletteColor::Card);
Text::new("Saved").color(PaletteColor::MutedForeground);
let edge = palette.input;
```

### Two Roles That Became One

`TextSecondary` and `TextMuted` are both `MutedForeground`, and `SurfaceBase` and `SurfaceInput` are both `Background`. An app that gave the two of a pair different colors keeps a name of its own for one of them:

```rust
let secondary = PaletteColor::extra("text-secondary");
app.theme().set_palette_color(secondary.clone(), Color::from_hex("#334155"));
Text::new("Updated a minute ago").color(secondary);
```

### A Role Set By Its Old Name

In 0.63 to 0.66, setting a new role also set the old roles it replaced, and setting an old role set only that one. An app that set `Accent` to restyle what still read it (the engine's select, a checkbox or a slider with no style of its own, a link in a text) sets `Primary` now: those read the new roles.

| What lurq draws | Read | Reads |
| --- | --- | --- |
| The engine's select: trigger, focus, menu, hovered and selected option, muted text | `SurfaceInput`, `BorderFocus`, `SurfaceRaised`, `SurfacePanel`, `Accent`, `AccentHover`, `TextMuted` | `Background`, `Ring`, `Card`, `Canvas`, `Primary`, `PrimaryHover`, `MutedForeground` |
| A checkbox and a slider node with no style of their own | `SurfaceInput`, `Accent` | `Background`, `Primary` |
| A text's code mark and link | `SurfaceRaised`, `Accent` | `Card`, `Primary` |

### `PaletteColor::replacement` And `ThemePalette::roles`

`PaletteColor::replacement()` told the role that replaces a deprecated one. With no deprecated role left it is removed. `ThemePalette::roles()` lists the design's roles only, so a loop that skipped the deprecated ones drops the filter:

```rust
// 0.66
for role in ThemePalette::roles().into_iter().filter(|role| role.replacement().is_none()) { .. }
// 0.67
for role in ThemePalette::roles() { .. }
```

### The Legacy Palette

`ThemePalette::legacy()` and `ThemeDefinition::legacy()` stay, with 0.61's colors in the roles that replaced the old ones: `primary` is `#2563eb`, `canvas` `#f8fafc`, `input` `#94a3b8`, `muted_foreground` `#64748b`.

Two old roles had a color there that no role has now: `TextPrimary` (`#0f172a`) and `TextSecondary` (`#334155`). Nothing lurq draws read them. An app on the legacy theme that painted with them itself gets `Foreground` (black) and `MutedForeground` (`#64748b`) from the table above; to keep the old colors, give them names of the app's own, as in [Two Roles That Became One](#two-roles-that-became-one).

## The Form Theme

`FormTheme` and its parts (`FormFieldTheme`, `FormInputTheme`, `FormCheckboxStyle`, `FormSliderStyle`, `FormButtonTheme`, `FormButtonRole`, `FormTextRole`), `theme.form()`, `theme.set_form(..)` and the field `ThemeDefinition::form` are gone. Nothing read them since 0.63, so removing the calls changes nothing on the screen.

The form controls are the design system's components, and a theme styles them with their recipes:

| `FormTheme` | The recipe |
| --- | --- |
| `field.label`, `field.hint`, `field.error`, `field.spacing` | `ui::Field`: `FieldPart::Label`, `Description` (with `FieldState::Invalid` for an error), the gap of `Root` |
| `input` | `ui::Input` and `ui::Textarea`: `InputPart::Control`, `Placeholder`, the states `Invalid` and `Disabled` |
| `checkbox` | `ui::Checkbox`: `CheckboxPart::Box`, the states `Checked` and `Invalid` |
| `slider` | `ui::Slider`: `SliderPart::Track`, `Fill`, `Thumb` |
| `button.primary`, `button.secondary` | `ui::Button`: `ButtonPart::Root` and `Label` of the variants `ButtonVariant::Primary` and `Outline` |

Before:

```rust
let mut form = app.theme().form().clone();
form.input.radius = RadiusSize::Lg;
app.theme().set_form(form);
```

After:

```rust
use lurq::{app::theme::RadiusSize, ui::{self, InputPart}};

app.theme().patch_recipe::<ui::Input>(|recipe| recipe.base(InputPart::Control, |s| s.radius(RadiusSize::Lg)));
```

A `ThemeDefinition` written out field by field drops its `form` field. See [Form Theme](../theme/#form-theme).

## The Phone's Pieces

`dark()` and `for_appearance(..)` of `TabBar`, `SheetStyle` and `NavigationBar`, and `PullToRefreshStyle::dark()`, are gone. `new()` reads the theme where it is made, and is light or dark as the theme is.

```rust
// 0.66
let bar = TabBar::for_appearance(ctx.window().system.appearance);
let sheet = SheetStyle::dark().grabber(true);
let refresh = PullToRefreshStyle::dark();
// 0.67
let bar = TabBar::new();
let sheet = SheetStyle::new().grabber(true);
let refresh = PullToRefreshStyle::new();
```

Make them in `render`, which runs again when the system's appearance changes. A color the app wants whatever the theme is a field set on the style (`.background(..)`, `.tint(..)`); to restyle every one, patch its recipe: see [Colors from the theme](../ios/#colors-from-the-theme).

The removed constructors gave 0.62's own colors, not the theme's. An app that wants those exact colors sets them:

| | 0.62's dark colors |
| --- | --- |
| `SheetStyle` | background `#1C1C1E`, grabber `rgba(235, 235, 245, 0.3)`, backdrop black, dim `0.5` |
| `TabBar` | background `rgba(22, 22, 22, 0.8)`, border `rgba(84, 84, 88, 0.6)`, tint `#0A84FF`, color `#8E8E93`, contrast color `#C7C7CC` |
| `NavigationBar` | background `rgba(28, 28, 30, 0.8)`, tint `#0a84ff`, title `#ffffff`, border `#38383a` |
| `PullToRefreshStyle` | spinner `rgba(235, 235, 245, 0.6)` |

## The Spinner

`lurq::components::ActivityIndicator` is no longer exported. An app's spinner is `lurq::ui::Spinner`, the design's, drawn in the color of the text around it:

```rust
// 0.66
ActivityIndicator::mount(ctx, ActivityIndicatorProps::new().size(20.0))
// 0.67
Spinner::new().size(SpinnerSize::Md).label("Loading")
```

`PullToRefresh` still draws the platform's spokes over a list that is pulled, in `PullToRefreshStyle::spinner_color`. `ActivityIndicatorProps` is as it was.

## The Window's Frame Callback

`WinitWindow::on_frame` is `on_paint`: the same callback, after a frame is presented.

```rust
// 0.66
WinitWindow::new(app, tree).on_frame(|tree, delta, report| { .. })
// 0.67
WinitWindow::new(app, tree).on_paint(|tree, delta, report| { .. })
```

`ctx.on_frame(..)` and `window.on_frame(..)`, an app's own frames, are other things and stay.

## Types That Gained A Variant Or A Field

Glass (see [Glass](../glass/)) added to types an app may write out. A struct written field by field adds the field; a `match` with no `_` arm adds the arm.

| Type | Gained |
| --- | --- |
| `RenderList` | the field `glasses` |
| `BackdropCmd` | the fields `glass`, `reach` and `spread` |
| `QuadContent::Backdrop` | the field `glass` |
| `LayerBlur` | the field `coarse` |
| `AnimatableProperty`, `AnimatableValue` | the variant `Glass` |
| `TabBar`, `NavigationBar`, `SheetStyle` | the field `glass` |

`TabBar::floating()` is of glass now, and on iOS a `NavigationBar` and a `Sheet` are. `.glass(None)` on each gives the blurred bar of 0.66 back.

## What Is Still Deprecated

`lurq::components::Image` and `lurq::components::Checkbox` compile in 0.67, with the warning they had: an app's picture is `ui::Picture`, and its checkbox `ui::Checkbox`. They are the engine's own image and checkbox elements, which `ui::Picture`, Markdown and the form's checkbox draw with, so they are not removed with the rest. Moving to the `ui` components is in the 0.63 guide: [Images And Loading](../migration-v0-63/#images-and-loading) and [Components That Moved To `lurq::ui`](../migration-v0-63/#components-that-moved-to-lurqui).
