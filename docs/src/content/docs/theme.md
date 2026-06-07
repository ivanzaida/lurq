---
title: Theme
description: Strict palette, typography, radius, spacing, border size, and form theme roles.
---

# Theme

The runtime theme is a strict set of semantic roles. There is no dynamic token registry: palette colors, typography styles, radius sizes, spacing sizes, and border sizes are closed enums with matching fields on the theme structs.

Use concrete colors and dimensions for one-off visuals. Use theme roles when the value should follow the active runtime theme.

```rust
use lurq::{
  app::theme::{BorderSize, PaletteColor, RadiusSize, SpacingSize, TypographyStyle},
  components::{Column, Rect, Text},
};

Column::new()
  .spacing(SpacingSize::Md)
  .padding(SpacingSize::Lg)
  .child(Text::new("Settings").variant(TypographyStyle::Title))
  .child(
    Rect::new(120.0, 36.0)
      .background(PaletteColor::Accent)
      .border_inside(BorderSize::Sm, PaletteColor::Border)
      .rounded(RadiusSize::Md),
  )
```

## Runtime Theme

`App::theme()` returns the app theme. `Tree::mount_root` passes that theme into the root context, and descendants read it with `ctx.theme()`.

```rust
let theme = app.theme();
tree.mount_root::<Root>(theme.clone(), ());
```

Inside components:

```rust
let accent = ctx.theme().palette_color(PaletteColor::Accent);
let gap = ctx.theme().spacing_value(SpacingSize::Sm);
```

Calling `ctx.theme()` during render subscribes that component to theme version changes. Mutating the theme rerenders subscribers on the next pass.

The main theme accessors are:

| Method | Purpose |
| --- | --- |
| `theme.palette()` / `theme.set_palette(...)` | Read or replace `ThemePalette`. |
| `theme.palette_color(key)` / `theme.set_palette_color(key, color)` | Read or set one palette role. |
| `theme.typography()` / `theme.set_typography(...)` | Read or replace `ThemeTypography`. |
| `theme.typography_style(key)` / `theme.set_typography_style(key, style)` | Read or set one typography role. |
| `theme.default_text_style()` / `theme.set_default_text_style(style)` | Compatibility alias for the `body` typography style. |
| `theme.radii()` / `theme.set_radii(...)` | Read or replace `ThemeRadii`. |
| `theme.radius_value(key)` / `theme.set_radius_value(key, value)` | Read or set one radius role. |
| `theme.spacing()` / `theme.set_spacing(...)` | Read or replace `ThemeSpacing`. |
| `theme.spacing_value(key)` / `theme.set_spacing_value(key, value)` | Read or set one spacing role. |
| `theme.border_sizes()` / `theme.set_border_sizes(...)` | Read or replace `ThemeBorderSizes`. |
| `theme.border_size_value(key)` / `theme.set_border_size_value(key, value)` | Read or set one border-size role. |
| `theme.breakpoints()` / `theme.set_breakpoints(...)` | Read or replace `ThemeBreakpoints`. |
| `theme.breakpoint_value(key)` / `theme.set_breakpoint_value(key, value)` | Read or set one breakpoint threshold. |
| `theme.form()` / `theme.set_form(...)` | Read or replace `FormTheme`; requires the `form` feature. |

Use `theme.lens(getter, setter)` when UI code needs a focused mutable handle for one theme value:

```rust
use lurq::{app::theme::PaletteColor, node::color::Color};

let brand = ctx.theme().lens(
  |theme| theme.palette_color(PaletteColor::Accent),
  |theme, color| theme.set_palette_color(PaletteColor::Accent, color),
);

brand.set(Color::from_hex("#2563eb"));
```

## Palette

Palette roles are named by `PaletteColor` and stored as public fields on `ThemePalette`.

```rust
use lurq::{app::theme::PaletteColor, node::color::Color};

app.theme().set_palette_color(PaletteColor::Accent, Color::from_hex("#2563eb"));

let mut palette = lurq::app::theme::ThemePalette::default();
palette.surface_base = Color::from_hex("#ffffff");
palette.border_focus = Color::from_hex("#2563eb");
app.theme().set_palette(palette);
```

Available roles:

| `PaletteColor` | `ThemePalette` field | Default |
| --- | --- | --- |
| `Accent` | `accent` | `#2563eb` |
| `AccentHover` | `accent_hover` | `#1d4ed8` |
| `AccentMuted` | `accent_muted` | `#dbeafe` |
| `SurfaceBase` | `surface_base` | `#ffffff` |
| `SurfacePanel` | `surface_panel` | `#f8fafc` |
| `SurfaceRaised` | `surface_raised` | `#ffffff` |
| `SurfaceInput` | `surface_input` | `#ffffff` |
| `Border` | `border` | `#e2e8f0` |
| `BorderStrong` | `border_strong` | `#94a3b8` |
| `BorderFocus` | `border_focus` | `#2563eb` |
| `TextPrimary` | `text_primary` | `#0f172a` |
| `TextSecondary` | `text_secondary` | `#334155` |
| `TextMuted` | `text_muted` | `#64748b` |
| `TextInverse` | `text_inverse` | `#ffffff` |
| `Success` | `success` | `#16a34a` |
| `SuccessMuted` | `success_muted` | `#dcfce7` |
| `Warning` | `warning` | `#d97706` |
| `WarningMuted` | `warning_muted` | `#fef3c7` |
| `Danger` | `danger` | `#dc2626` |
| `DangerMuted` | `danger_muted` | `#fee2e2` |
| `Info` | `info` | `#0284c7` |
| `InfoMuted` | `info_muted` | `#e0f2fe` |

Palette roles can be passed anywhere a background or text color accepts a theme color:

```rust
use lurq::{app::theme::PaletteColor, components::{Rect, Text}};

Rect::new(80.0, 32.0).background(PaletteColor::Accent);
Text::new("Muted").color(PaletteColor::TextMuted);
```

## Typography

Typography roles are named by `TypographyStyle` and stored as public fields on `ThemeTypography`.

```rust
use lurq::{
  app::theme::TypographyStyle,
  layout::text_style::{FontWeight, TextAlign, TextStyle},
};

app.theme().set_typography_style(TypographyStyle::Heading, TextStyle {
  font_size: 28.0,
  weight: FontWeight::Bold,
  text_align: TextAlign::Left,
  ..TextStyle::default()
});
```

Available roles:

| `TypographyStyle` | `ThemeTypography` field | Default |
| --- | --- | --- |
| `Heading` | `heading` | `24px`, bold |
| `Title` | `title` | `20px`, bold |
| `Body` | `body` | `TextStyle::default()` |
| `Description` | `description` | `14px` |
| `Caption` | `caption` | `12px` |
| `Label` | `label` | `13px`, medium |
| `FieldLabel` | `field_label` | `13px`, medium |
| `Button` | `button` | `13px`, medium |
| `Link` | `link` | body defaults |
| `Mono` | `mono` | body defaults with `monospace` family |

`Text::new` uses `TypographyStyle::Body`. Use `.variant(...)` for themed text, and `Text::styled(...)` for a one-off style that should not follow a typography role.

`TextStyle::text_align` supports `TextAlign::Left`, `Center`, `Right`, `Justified`, and `End`. `Text::text_align(...)` aligns text inside the text node's box, and `TextInput::text_align(...)` applies the same alignment to value and placeholder text inside the input content box. Both builders also accept layout `Alignment`.

`Text::text_overflow(...)` accepts `TextOverflow::Clip` or `TextOverflow::Elipsis`. `Clip` is the default. `Elipsis` renders a single-line text quad with `…` when the text is wider than its available width.

```rust
use lurq::{
  app::theme::TypographyStyle,
  components::{Text, TextOverflow},
};

Text::new("Headline").variant(TypographyStyle::Heading);
Text::new("Caption").variant(TypographyStyle::Caption);
Text::new("Long endpoint name").text_overflow(TextOverflow::Elipsis);
```

`ThemeFonts` remains as a compatibility shape with `body`, `heading`, and `mono`. Converting it into `ThemeTypography` only fills those three roles and leaves the rest at defaults.

## Radius

Radius roles are named by `RadiusSize` and stored as public fields on `ThemeRadii`.

| `RadiusSize` | `ThemeRadii` field | Default |
| --- | --- | --- |
| `Sm` | `sm` | `3.0` |
| `Md` | `md` | `5.0` |
| `Lg` | `lg` | `6.0` |

Use radius roles anywhere a corner radius accepts a `RadiusValue`:

```rust
use lurq::{app::theme::RadiusSize, components::Rect};

app.theme().set_radius_value(RadiusSize::Md, 5.0);

Rect::new(100.0, 40.0)
  .rounded(RadiusSize::Md)
  .corner_radius_top_left(RadiusSize::Sm);
```

## Spacing

Spacing roles are named by `SpacingSize` and stored as public fields on `ThemeSpacing`.

| `SpacingSize` | `ThemeSpacing` field | Default |
| --- | --- | --- |
| `Xs` | `xs` | `4px` |
| `Sm` | `sm` | `8px` |
| `Md` | `md` | `12px` |
| `Lg` | `lg` | `16px` |
| `Xl` | `xl` | `24px` |
| `Section` | `section` | `32px` |

Use spacing roles for container gaps and padding:

```rust
use lurq::{app::theme::SpacingSize, components::Column};

app.theme().set_spacing_value(SpacingSize::Section, 40.0);

Column::new()
  .spacing(SpacingSize::Sm)
  .padding(SpacingSize::Lg);
```

Spacing values are `Dimension`s, so a role can be set to pixel, percentage, or auto dimensions where that makes sense:

```rust
use lurq::{app::theme::SpacingSize, node::dimension::Dimension};

app.theme().set_spacing_value(SpacingSize::Md, Dimension::Px(14.0));
```

## Border Size

Border-size roles are named by `BorderSize` and stored as public fields on `ThemeBorderSizes`.

| `BorderSize` | `ThemeBorderSizes` field | Default |
| --- | --- | --- |
| `Sm` | `sm` | `1px` |
| `Md` | `md` | `2px` |
| `Lg` | `lg` | `3px` |

Use border-size roles anywhere a border width accepts a `BorderSizeValue`:

```rust
use lurq::{app::theme::{BorderSize, PaletteColor}, components::Rect};

app.theme().set_border_size_value(BorderSize::Md, 2.0);

Rect::new(100.0, 40.0)
  .border_inside(BorderSize::Sm, PaletteColor::Border)
  .focused(|style| style.border_inside(BorderSize::Md, PaletteColor::BorderFocus));
```

## Breakpoints

Breakpoints are named viewport-width thresholds, stored as public fields on `ThemeBreakpoints` and keyed by `Breakpoint`. Thresholds are logical pixels and expected to be non-decreasing.

| `Breakpoint` | `ThemeBreakpoints` field | Default |
| --- | --- | --- |
| `Sm` | `sm` | `640.0` |
| `Md` | `md` | `768.0` |
| `Lg` | `lg` | `1024.0` |
| `Xl` | `xl` | `1280.0` |

Configure thresholds through the theme, one role or all at once:

```rust
use lurq::app::theme::{Breakpoint, ThemeBreakpoints};

app.theme().set_breakpoint_value(Breakpoint::Md, 820.0);

app.theme().set_breakpoints(ThemeBreakpoints {
  sm: 600.0,
  md: 900.0,
  lg: 1200.0,
  xl: 1600.0,
});
```

Inside components, read the current breakpoint with `ctx.breakpoint()`. It resolves the window's logical width against the theme thresholds and returns `Option<Breakpoint>`, where `None` is the base tier (narrower than `Sm`). Reading it subscribes the component to breakpoint changes only — it rerenders when the resolved breakpoint crosses a threshold, not on every resize.

```rust
use lurq::{app::theme::Breakpoint, components::{Column, Row}};

fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
  if ctx.breakpoint() >= Some(Breakpoint::Lg) {
    Row::new().child(nav).child(content)
  } else {
    Column::new().child(nav).child(content)
  }
}
```

### Responsive Values

`Responsive<T>` holds a base value plus per-breakpoint overrides. Resolution is mobile-first: for the current breakpoint it uses the value set there or, if unset, the nearest smaller breakpoint that is set, falling back to `base`. Resolve it with `ctx.responsive(...)`.

```rust
use lurq::responsive::Responsive;

let columns = Responsive::new(1).md(2).lg(3).xl(4);
let count = ctx.responsive(&columns); // 1 below md, 2 at md, 3 at lg, 4 at xl
```

Any `T` works, so the same pattern drives padding, font sizes, widths, or whole layout values. Like `ctx.breakpoint()`, `ctx.responsive(...)` only rerenders the component when the resolved breakpoint changes.

## Form Theme

Form theme roles require the `form` feature:

```toml
lurq = { version = "0.7", features = ["form"] }
```

`FormTheme` groups compound form styling into semantic roles:

| Field | Purpose |
| --- | --- |
| `form.field` | Label, hint, error text, and field spacing. |
| `form.input` | Text input frame, text, placeholder, caret, focus, and error colors. |
| `form.checkbox` | Compound checkbox colors and radius. |
| `form.slider` | Compound slider track and thumb colors. |
| `form.button` | Primary and secondary compound button roles. |

Form text roles use typography and palette together:

```rust
use lurq::app::theme::{FormTextRole, PaletteColor, TypographyStyle};

FormTextRole {
  typography: TypographyStyle::Caption,
  color: PaletteColor::TextMuted,
};
```

### Field Roles

`theme.form().field` is a `FormFieldTheme`.

| Field | Default |
| --- | --- |
| `spacing` | `SpacingSize::Xs` |
| `label` | `TypographyStyle::FieldLabel` + `PaletteColor::TextPrimary` |
| `hint` | `TypographyStyle::Caption` + `PaletteColor::TextMuted` |
| `error` | `TypographyStyle::Caption` + `PaletteColor::Danger` |

### Input Roles

`theme.form().input` is a `FormInputTheme`.

| Field | Default |
| --- | --- |
| `height` | `36px` |
| `padding` | horizontal `10px`, vertical `8px` |
| `radius` | `RadiusSize::Md` |
| `background` | `PaletteColor::SurfaceInput` |
| `border` | `PaletteColor::Border` |
| `border_focus` | `PaletteColor::BorderFocus` |
| `background_error` | `PaletteColor::DangerMuted` |
| `border_error` | `PaletteColor::Danger` |
| `text` | `TypographyStyle::Body` + `PaletteColor::TextPrimary` |
| `placeholder` | `TypographyStyle::Body` + `PaletteColor::TextMuted` |
| `caret` | `PaletteColor::BorderFocus` |

### Button Roles

`theme.form().button` is a `FormButtonTheme` with `primary` and `secondary` `FormButtonRole`s.

Both button roles own layout values (`width`, `height`, `padding`) and semantic theme references (`radius`, background roles, border roles, text role). Compound form controls draw their default borders with `BorderSize::Sm`. Defaults:

| Role | Background | Border | Text |
| --- | --- | --- | --- |
| `primary` | `Accent` | `Accent` | `Button` + `TextInverse` |
| `secondary` | `SurfaceInput` | `BorderStrong` | `Button` + `TextPrimary` |

Primary hover and active states use `AccentHover`. Secondary hover uses `SurfacePanel` and active uses `Border`.

### Checkbox And Slider Roles

Compound checkbox defaults:

| Field | Default |
| --- | --- |
| `background` | `SurfaceInput` |
| `border` | `Border` |
| `border_hover` | `BorderFocus` |
| `checked_background` | `Accent` |
| `checked_border` | `Accent` |
| `checked_background_hover` | `AccentHover` |
| `radius` | `RadiusSize::Sm` |

Compound slider defaults:

| Field | Default |
| --- | --- |
| `track` | `Border` |
| `track_hover` | `BorderStrong` |
| `thumb` | `Accent` |
| `thumb_hover` | `AccentHover` |

### Updating Form Roles

Read the current form theme, modify the semantic roles, then replace it:

```rust
use lurq::app::theme::{
  FormTextRole, PaletteColor, RadiusSize, TypographyStyle,
};

let mut form = app.theme().form().clone();

form.field.label = FormTextRole {
  typography: TypographyStyle::Label,
  color: PaletteColor::TextSecondary,
};

form.input.radius = RadiusSize::Lg;
form.input.border_focus = PaletteColor::Info;
form.button.primary.background = PaletteColor::Success;
form.button.primary.background_hover = PaletteColor::AccentHover;

app.theme().set_form(form);
```

## Concrete Values

Theme roles are for shared semantics. Component APIs still accept concrete values where a local override is clearer:

```rust
use lurq::{components::Rect, node::color::Color};

Rect::new(80.0, 32.0)
  .background("#0f172a")
  .rounded(4.0)
  .border_inside(1.0, Color::from_hex("#334155"));
```

Prefer concrete values for isolated drawings, debug visuals, or one-off component details. Prefer theme roles for app surfaces, text, controls, repeated spacing, repeated border widths, and reusable component defaults.
