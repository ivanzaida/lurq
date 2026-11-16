---
title: Migration To v0.63
description: What changes for an app written for lurq 0.62. The design system's theme becomes the default, Markdown is drawn with it, and the engine changed under the design system's components. What stops the build, what looks or behaves differently, and what to write.
---

# Migration To v0.63

lurq 0.63 brings the design system: the components of `lurq::ui`, a theme that is the design's, and the changes the engine needed for them. This page is for an app written for 0.62. It says what stops the build, what looks or behaves differently with no build error, and what to write for each.

## In Short

**What stops the build**

- The Markdown theme. Its colors are `TextColor`s, eight of its fields are gone, and `MarkdownProps::on_link_click` is `on_link_select`: see [Text And Markdown](#text-and-markdown).
- A struct written out field by field, or a `match` with no `_` arm, over a type that gained a field or a variant: see [the table](#types-that-gained-a-variant-or-a-field).

The palette roles of 0.62 still compile. Each gives a deprecation warning that names its replacement: see [Renamed Palette Roles](#renamed-palette-roles).

**What changes with no build error**

- The look: the palette, a dark look on a system that is dark, body text of 14, radii, shadows, the scroll bar, the color a window is cleared to. See [The Theme And Palette](#the-theme-and-palette).
- A Markdown document: its colors, its type, its code blocks and quotes, footnotes, bullets and task boxes.
- The engine's custom title bar, which is drawn in the theme's roles: see [Windows And Chrome](#windows-and-chrome).
- A growing child with a flex basis, and the least width of a `Column` in a row that shrinks to its content: see [Layout](#layout).
- A notch of a mouse's wheel, which is animated: see [Focus, Keys And Pointer](#focus-keys-and-pointer).
- A child of a `presence()` container added again while it leaves, and a transition when the theme changes: see [Animation And Presence](#animation-and-presence).

**The quickest way to keep the 0.62 look**

```rust
use lurq::app::theme::ThemeDefinition;

app.theme().apply(ThemeDefinition::legacy());
```

That brings back the palette, the typography, the radii, the shadows and the shape of the scroll bar. It does not bring back the 0.62 look of a Markdown document or of the engine's title bar, and it changes nothing of how the engine behaves: each of those has its own section below.

## The Theme And Palette

### What Changes Without A Line Of Code

| | lurq 0.62 | lurq 0.63 |
| --- | --- | --- |
| Palette | 23 roles on a blue accent | The design's 42 roles on a neutral primary |
| Dark look | None: one palette | A dark palette, taken when the system is dark |
| Text that names no color | Black | `Foreground`: near-black in the light palette, near-white in the dark one |
| Body text | 16 | 14 |
| Headings | Bold | Semibold, with tighter letter spacing |
| Code (`TypographyStyle::Mono`) | 16 on lines of 1.2 | 13 on lines of 1.3 |
| Shadows (`ShadowStyle::Md`, `Lg`) | Tight, with a negative spread | Softer and a little larger |
| Radii | `Sm` 3, `Md` 5, `Lg` 6 | `Sm` 4, `Md` 6, `Lg` 8, and `Xl` 12, `Full` |
| Scroll bar | A thumb of 8, black at a third of its opacity | A thumb of 6 in a lane of 8, round, in `Ring` |
| A window whose root paints no background | Cleared to white | Cleared to `Background`: white in the light palette, near-black in the dark one |

The one that can make an app unreadable is the dark look. On a system in its dark look, text that names no color is drawn near-white. An app that paints light backgrounds of its own under such text has near-white text on them.

In the light palette `Background` and `Card` are both white. `Canvas` (`#e6e6eb`) is what is behind panels and cards: an app that paints its window with `Background` and its cards with `Card` has white on white.

### Keep The 0.62 Look

One line, before the root is mounted:

```rust
use lurq::app::theme::ThemeDefinition;

app.theme().apply(ThemeDefinition::legacy());
```

`ThemeDefinition::legacy()` is the theme 0.61 and 0.62 had: its palette color for color, with text that names no color in black, its typography (body text of 16), its radii, its shadows and a scroll bar whose thumb is as wide as its lane. It has one palette, so the app is light whatever the system is in. The theme goes on taking the system's text size and bold text, which only a phone reports.

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

### Follow The System Properly

The other way is to let the app be dark when the system is. Then every color the app names has to come from the theme:

- Paint surfaces with roles, not with concrete colors: `PaletteColor::Background` for the window, `Canvas` for what is behind panels, `Card` for what is raised off it, `Popover` for what floats.
- Give text that sits on a filled surface that surface's foreground: `.foreground(PaletteColor::PrimaryForeground)` on a primary button colors every text in it.
- Leave ordinary text with no color: it takes `Foreground`.

### Renamed Palette Roles

The roles of 0.62 are deprecated. They compile with a warning that names the replacement.

| 0.62 | 0.63 |
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

`Border`, `Foreground`, `Success`, `SuccessMuted`, `Warning`, `WarningMuted`, `Info` and `InfoMuted` keep their names. The same holds for the fields of `ThemePalette`: `palette.accent` becomes `palette.primary`, and so on.

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
- **A deprecated role still has a color of its own.** In the design's palettes it is the color of its replacement. In `ThemePalette::legacy()` it is the color it had in 0.62.
- **Setting a role sets the deprecated roles it replaces.** `Select`, the form controls, and a checkbox or a slider with no style of its own still read the deprecated roles, and follow a theme set through the new ones this way.
- **Setting a deprecated role sets only that role.** An app that sets `Accent` and uses a role's replacement elsewhere sets `Primary`.

### A Color Set Once

`theme.set_palette_color(role, color)` sets the role in the light palette and in the dark one: a brand color set once is the brand color in both looks. `theme.set_palette_color_for(appearance, role, color)` sets it in one. `theme.set_palette(palette)` sets the palette of the appearance the theme is in and leaves the other as it is; `theme.set_palettes(light, dark)` sets both.

In 0.62, on a theme given two palettes with `set_palettes`, a color or a palette set this way was lost at the next change of appearance. An app that set it again after each change can stop.

### A Palette Written Out Field By Field

`ThemePalette` has a field for each of the design's 42 roles, 34 of them new. A palette built with every field named and no `..ThemePalette::default()` no longer compiles. Start from one of the built-in palettes:

```rust
use lurq::{app::theme::ThemePalette, node::color::Color};

let mut light = ThemePalette::light();
light.primary = Color::from_hex("#7c3aed");
let mut dark = ThemePalette::dark();
dark.primary = Color::from_hex("#a78bfa");
app.theme().set_palettes(light, dark);
```

`ThemePalette::default()` is now the design's light palette, not the blue one: a palette built on it with `..ThemePalette::default()` takes the design's colors for the fields it does not name. `ThemePalette::legacy()` is the blue one.

`PaletteColor` has a variant for each new role: a `match` over it with no `_` arm must take them.

### A Window With No Background

A window whose root paints no background is cleared to the theme's `Background`. It was always white. In the light palette nothing changes; in the dark one the window is near-black. An app that wants white whatever the theme is paints its root: `.background("#ffffff")`.

## Typography

### Smaller Body Text

`Text::new(...)` with no style is `TypographyStyle::Body`, which is 14 where it was 16: a layout that counted on the height of a line of text moves. An app that wants the new look at the old text size sets the one style:

```rust
use lurq::app::theme::TypographyStyle;

let mut body = app.theme().typography_style(TypographyStyle::Body);
body.font_size = 16.0;
app.theme().set_typography_style(TypographyStyle::Body, body);
```

`TextStyle::default()` is still 16: a text built with `Text::styled(text, TextStyle::default())` keeps its size.

### Code

`TypographyStyle::Mono` is 13 on lines of 1.3, the line its family draws: a line of code is 16.9 high where it was 19.2 (16 on lines of 1.2). Every text in `Mono` moves with it. `ThemeTypography::legacy()` keeps 16 on lines of 1.2.

### The Theme's Type Written Out Field By Field

`ThemeTypography` has ten fields more: `sans` and `mono_family`, the theme's two font families, and a style for each of the eight new roles, `heading1` to `heading6`, `prose` and `article`. `TypographyStyle` has the eight roles as variants, `Heading1` to `Heading6`, `Prose` and `Article`. `TextStyle` has two fields more, `case` and `baseline_shift`.

Before:

```rust
let caption = TextStyle {
  font_family: family,
  font_size: 12.0,
  line_height: 1.2,
  // ... every other field of 0.62
  strikethrough: None,
};
```

After:

```rust
let caption = TextStyle {
  font_family: family,
  font_size: 12.0,
  ..TextStyle::default()
};
```

A `ThemeTypography` ends with `..ThemeTypography::default()` for the design's scale or `..ThemeTypography::legacy()` for the one of 0.62. `ThemeTypography::default()` is the design's now: a value built on it takes body text of 14 for the styles it does not name.

### A Styled Text Takes The Foreground Above It

A text built with `Text::styled(..)` from a style that keeps the default text color names no color. It now takes the `.foreground(..)` of an element above it, as a text with no style does. It took the theme's foreground whatever was above it. A text that must not follow names its color:

```rust
Text::styled("Saved", style).color(PaletteColor::Foreground)
```

## Layout

### A Flex Basis, As On The Web

A growing child with a basis in a `Row` or a `Column` is its basis and its share of the room the bases leave. The free room of a line is its length less the gaps, the children sized on their own and the bases. In 0.62 the bases were left in the free room, so a growing child came out longer by its basis and could run out of the line.

```rust
// A row of 200. The first child was 200 wide in 0.62 and ran out of the row; it is 150.
Row::new()
  .width(200.0)
  .child(Spacer::new().flex_full(1.0, 0.0, Some(50.0)))
  .child(Spacer::new().width(50.0))
```

A basis of 0, the common `flex_full(1.0, 1.0, Some(0.0))`, lays out as before. Look at each `flex_full` with a grow factor and a basis other than 0: a child whose size was tuned against the old result comes out shorter. See [Flex](../layout/#flex).

### The Least Width Of A Column In A Row

Under `ShrinkLimit::Content` a child shrinks down to its content minimum. For a line on the other axis, a `Column` in a row, that minimum is now the widest of its children's, where a child with a `min_width` of its own counts at it. It was the column's natural width, so such a column did not shrink at all. A column with a text that says `min_width(0.0)` now gives way where it held its width.

To hold a column at a width, give the column itself a `min_width`: a child never shrinks below an explicit one. See [Give-way order and limits](../layout/#give-way-order-and-limits).

### Scroll Bars

The design system's theme draws the design's bar: a lane of 8 with a thumb of 6 in it, round, in `Ring`, and `MutedForeground` under the pointer and while it is dragged. The bar of 0.62 had a thumb as wide as its lane, the same under the pointer. Each bar of a container that scrolls both ways has its own state, and where both are there each track ends before the other's lane.

In the design system's theme a bar that sets no thumb color is drawn in the theme's `Ring`, where it was black at a third of its opacity. `ThemeDefinition::legacy()` keeps that black: its bar is the one 0.62 drew. A style whose `thumb_color` is a color of its own keeps it, 0.62's black included: the color a style that sets none has (`ScrollBarStyle::THEME_THUMB`) is a mark of its own.

`ScrollBarStyle::default()` is still the bar of 0.62 in its shape, and `ThemeDefinition::legacy()` has it. To have it in the design's theme:

```rust
use lurq::layout::scrollbar::ScrollBarStyle;

app.theme().set_scrollbar(ScrollBarStyle::default());
```

`ScrollBarStyle` has three fields more, `thumb_inset`, `thumb_hovered_color` and `thumb_pressed_color`. A style written out field by field must add them, or end with `..ScrollBarStyle::default()`. See [Scroll](../styling-events/#scroll).

### A Select's Checkmark After The Label

A single-select whose checkmark is after the label (`SelectCheckmarkPosition::Trailing`) keeps the checkmark's room on the selected row only. The labels and trailing elements of the other rows run to the row's end, where every row kept an empty slot of the checkmark's width. A checkmark before the label, and a multi-select, are as they were.

### Borders

The border that goes on round a rounded corner, and `StrokeStyle::side_colors`, came with 0.62.0. Nothing of them changes in 0.63.

## Text And Markdown

### Markdown Colors Are Roles

`ThemeMarkdown`'s colors are `TextColor`s, a palette role or a color: `MarkdownTextStyle::color`, `MarkdownBlockStyle::background` and `border_color`, the four colors of `MarkdownLinkState`, and `MarkdownFocusRing::color`. A document is drawn in the theme it is in, light and dark.

Before:

```rust
markdown.link.text.color = Some(Color::from_hex("#2563eb"));
markdown.table_header_box.background = Some(Color::from_hex("#f8fafc"));
```

After, with a role that follows the theme or with the same color:

```rust
markdown.link.text.color = Some(PaletteColor::Info.into());
markdown.table_header_box.background = Some(Color::from_hex("#f8fafc").into());
```

- The setters of `MarkdownLinkState` (`color`, `underline_color`, `background`, `trailing_color`) and `MarkdownFocusRing::new(width, color)` take a color or a role: a call that passes a `Color` compiles as it did.
- `MarkdownLinkState` and `MarkdownFocusRing` are `Clone` and no longer `Copy`. Clone one where a copy was taken.
- Code that reads a color back gets a `TextColor`. `ThemeMarkdown::resolve(&palette)` gives the theme with every role turned into the color the palette has for it.

### The Default Look Of A Document

A document's default look is the design's, on the theme's roles. It is no longer dark code blocks and slate grays. `ThemeDefinition::legacy()` does not bring the old look back: `ThemeMarkdown::default()` is the design's in both themes. An app that wants its own look sets the fields it cares about on a `ThemeMarkdown` and gives it to the theme (`theme.set_markdown(..)`) or to a document (`MarkdownProps::theme(..)`).

A document with no style of its own is in the theme's `TypographyStyle::Article`, 15 on lines of 1.65, where it was in `TextStyle::default()`, 16 on lines of 1.2. Its headings are the theme's `Heading1` to `Heading6`: 30, 24, 20, 16, 14 and 12. They were 1.85, 1.55, 1.3, 1.15, 1 and 0.95 of the document's text. To keep the old text, set the role:

```rust
use lurq::{app::theme::TypographyStyle, layout::text_style::TextStyle};

app.theme().set_typography_style(TypographyStyle::Article, TextStyle::default());
```

A document given a style with `MarkdownProps::style(..)` that is not `TextStyle::default()` keeps that style. The headings are changed the same way, one role each: `TypographyStyle::Heading1` and the rest.

### Code Blocks And Quotes

A Markdown code block is a `ui::CodeBlock` and a quote a `ui::Blockquote`: the code under a bar named by its language, with a button that copies it, and the quote's words 14 in italics in the muted foreground after a line of 2.

`ThemeMarkdown` has no fields for them any more. These are gone: `code_block`, `code_block_label`, `code_block_spacing`, `code_block_box`, `code_block_scrollbar`, `blockquote_box`, `blockquote_marker` and `blockquote_gap`. Style the theme's recipes in their place.

Before:

```rust
markdown.code_block_box.background = Some(Color::from_hex("#0f172a"));
markdown.blockquote_marker.text.color = Some(Color::from_hex("#64748b"));
```

After:

```rust
use lurq::{
  app::theme::PaletteColor,
  layout::text_style::FontStyle,
  ui::{Blockquote, BlockquotePart, CodeBlock, CodeBlockPart},
};

// The bar over the code, and the quote's words upright.
app.theme().patch_recipe::<CodeBlock>(|rules| rules.base(CodeBlockPart::Header, |s| s.background(PaletteColor::Canvas)));
app.theme().patch_recipe::<Blockquote>(|rules| rules.base(BlockquotePart::Label, |s| s.font_style(FontStyle::Normal)));
```

`CodeBlockPart` names the parts of a code block: `Root`, `Header`, `Title`, `Copy`, `CopyIcon`, `Body`, and those of its code and tokens. A quote's are `BlockquotePart::Root`, `Label` and `Body`. On a `ThemeDefinition` the same is `definition.ui.patch::<CodeBlock>(..)`. See [What A Theme Changes](../design-system/#what-a-theme-changes).

A code block's lines are where the code block puts them, under its bar: an element lookup that counted on the old box sees the code block's parts. A selection across a document copies a code block's language and its lines, as before, and none of the glyphs of the document's controls.

### A Followed Link

`MarkdownProps::on_link_click` is `on_link_select`. `on_link_activate` is as it was.

```rust
// Before
MarkdownProps::new(text).on_link_click(|link| open(link.destination()))
// After
MarkdownProps::new(text).on_link_select(|link| open(link.destination()))
```

### Notes, Details And Keys

The parser tells apart three things that were a quote or HTML:

| Markdown | 0.62 | 0.63 |
| --- | --- | --- |
| A quote that starts `> [!NOTE]`, or one of the four others GitHub writes | `MarkdownBlock::BlockQuote` | `MarkdownBlock::Admonition { kind, blocks }` |
| `<details>` and `<summary>` HTML blocks around Markdown | `MarkdownBlock::Html` | `MarkdownBlock::Details { summary, open, blocks }` |
| `<kbd>` in a sentence | HTML | `MarkdownInline::Kbd(String)` |

A `match` on `MarkdownBlock` or `MarkdownInline` with no `_` arm must take them. Code that walks a parsed document for quotes or HTML no longer meets these there.

### Footnotes, Bullets And Tasks

- A footnote is drawn at the end of the document, numbered by the first reference to it, and a reference is its number: `rendered_text()` has `1` where it had `[^1]`.
- A bulleted item's marker is a dot. A selection still copies its `-`.
- A task's `[x]` is a checkbox, no longer text.

A test that compares `rendered_text()` with a string, or looks for these glyphs, changes with them.

### A Highlight Written Out Field By Field

`TextHighlight` has two fields more, `padding` and `border`. Make one with its constructor:

```rust
// Before
TextHighlight { range: 4..9, color: yellow.into(), radius: 3.0 }
// After
TextHighlight::new(4..9, yellow).radius(3.0)
```

## Focus, Keys And Pointer

### The Mouse Wheel Is Animated

A notch of a mouse's wheel is animated: the content is where the notch sent it about 150 ms later and not in the next frame. `Tree::scroll`, a touchpad and a finger scroll at once as before. What could rely on the old way:

- An `on_scroll` handler also hears an event with a zero delta for every step of the animation, where it heard the wheel's event alone. A handler that counts events, or acts on each, checks the delta.
- `on_scroll_reach_top` and `on_scroll_reach_bottom` run when the content gets there, not when the wheel turns.
- `Tree::has_active_timeline()` is true while the content is on its way.
- A harness that sends `SyntheticInput::wheel`, or the MCP server's `wheel`, and reads the offset at once reads it on its way. It waits for the window to be idle first (`lurq_wait`), or reads `ScrollState::scroll_target()`.

To have the old way back for the wheel:

```rust
app.theme().set_smooth_scroll(false);
```

It is the app's setting and not part of a `ThemeDefinition`: applying one leaves it. See [The mouse wheel](../styling-events/#the-mouse-wheel).

### A Disabled Element Hears The Pointer Come And Go

The `on_mouse_enter` and `on_mouse_leave` of a disabled element run as the pointer comes onto it and leaves it, so a tooltip can say why it is disabled. They did not run. Nothing else of the pointer reaches it: it is not hovered, and no press, click or drop reaches it.

A handler that must not run while its element is disabled checks for itself, with the value the app disabled it by or with `InteractionState::is_disabled()`. See [Disabled](../styling-events/#disabled).

## Windows And Chrome

### A Modal Over The Window Ends At The Title Bar

A modal over the whole window (`Modal::target(Root)`) in a window with a `WindowChrome` covers the window under the title bar. It covered all of it, with its top under the bar. A dialog is centred in what is seen, and a veil ends at the bar.

What must cover the bar too is passed to `WindowChrome::overlay`, which still has all of the window. See [Targets](../modals/#targets).

### The Title Bar Is In The Theme's Roles

The engine's custom title bar follows the theme, where it was dark on every theme:

| | 0.62 | 0.63 |
| --- | --- | --- |
| `ChromeTitleBar::new()` | `#101215` under a line of `#252a32` | `Muted` under a line of `Border` |
| `ChromeBorderPolicy::PlatformDefault` | Paints `#252a32` | Paints `Border` |
| Minimize, maximize and restore | `#d9dee7` on `#232934`, `#2d3440` held | `Foreground` on `SecondaryHover` |
| Close | `#c0392b` under the pointer, `#922b21` held | `Destructive`, `DestructiveHover` held, its glyph `DestructiveForeground` |

`ThemeDefinition::legacy()` does not change this: under it the bar is the legacy palette's `Muted`. An app that wants the old dark bar on every theme writes it:

```rust
use lurq::{
  components::{ChromeTitleBar, WindowControlColors, WindowControlKind, WindowControls},
  node::{border::Border, color::Color},
};

ChromeTitleBar::new()
  .background("#101215")
  .border_bottom(Border::inside(1.0, Color::from_hex("#252a32")))
  .controls(
    WindowControls::new()
      .colors(WindowControlColors::new("#d9dee7", "#232934", "#2d3440"))
      .control_colors(WindowControlKind::Close, WindowControlColors::new("#d9dee7", "#c0392b", "#922b21"))
      .hover_foreground(WindowControlKind::Close, "#d9dee7"),
  )
```

and gives the window's edge its color with `WindowChrome::border(ChromeBorderPolicy::Visible { size: 1.0, color: Color::from_hex("#252a32").into() })`.

## Animation And Presence

### A Child Added Again While It Leaves

A child of a container with `presence()` that is added again, by its key or its id, while it leaves ends its leaving and is the one added: the same element, with its state. In 0.62 it was a new one beside the one leaving until that one's exit ended, so a banner shown again while it faded out stood twice in the page, and a panel opened again while its rows faded out held them twice.

What wants the old way gives the one coming back a key of its own each time it is shown:

```rust
Row::new()
  .key(format!("saved-{shown}"))
  .exit_animation(fade_out)
```

with `shown` counted up by the app. A layer opened again while it leaves does the same as a child. See [Children of a container](../animation-transforms/#children-of-a-container).

### A Change Of Theme Is Applied At Once

A value that changes because the theme does, a color of the other appearance, is applied at once to an element that has a transition for it, as it is to everything else in the window. Such an element used to fade from its old color while the window around it had already changed: a button stayed light for its transition's length on a window that had turned dark.

A transition is for what the element does itself: a state, a style set on it. An app that animated a change of theme through transitions animates it another way.

## Types That Gained A Variant Or A Field

A `match` with no `_` arm must take a new variant. A struct written out field by field must add a new field, or end with the rest from a default.

| Type | Added | What to write |
| --- | --- | --- |
| `ThemePalette` | A field for each of 34 new roles | Start from `ThemePalette::light()`, `dark()` or `legacy()` |
| `PaletteColor` | A variant for each of 34 new roles | A `match` arm |
| `ThemeTypography` | `sans`, `mono_family`, `heading1` to `heading6`, `prose`, `article` | `..ThemeTypography::default()` or `..ThemeTypography::legacy()` |
| `TypographyStyle` | `Heading1` to `Heading6`, `Prose`, `Article` | A `match` arm |
| `TextStyle` | `case`, `baseline_shift` | `..TextStyle::default()` |
| `ThemeRadii` | `xl`, `full` | `..ThemeRadii::default()` or `..ThemeRadii::legacy()` |
| `RadiusSize` | `Xl`, `Full` | A `match` arm |
| `ScrollBarStyle` | `thumb_inset`, `thumb_hovered_color`, `thumb_pressed_color` | `..ScrollBarStyle::default()` |
| `ShrinkLimit` | `ContentFirst` | A `match` arm |
| `BackgroundColor` | `CurrentForeground` | A `match` arm |
| `GradientKind` | `Corners` | A `match` arm |
| `AnimatableValue` | `Percent(f32)`, `Dash { dash, gap }`, `Fit { px, content }` | A `match` arm |
| `AnimatableProperty` | `BorderDash` | A `match` arm |
| `FormValue` | `Range(f64, f64)` | A `match` arm |
| `MarkdownBlock` | `Admonition { kind, blocks }`, `Details { summary, open, blocks }` | A `match` arm |
| `MarkdownInline` | `Kbd(String)` | A `match` arm |
| `MarkdownTextStyle` | `baseline_shift` | `..Default::default()` |
| `SpanStyle` | `underline` | `SpanStyle::new()` and its methods, or `..SpanStyle::default()` |
| `SelectionStyle` | `underline` | `..SelectionStyle::default()` |
| `ThemeTextMarks` | `underline` | `..ThemeTextMarks::default()` |
| `TextMark` | `Underline`; `TextMark::ALL` is five long | A `match` arm |
| `StyledEdit` | `Paragraphs { range }` | A `match` arm: the kinds of the paragraphs changed, the text did not |
| `ThemeMarkdown` | `task_item_spacing`, `bullet_gap`, `box_gap`, `bullet_indent`, `bullet_size`, `table_stripe`, `heading_2_rule`, `kbd`, `task_done`, `rule`, `image_box`, `image_height`, `image_caption`, `details_box`, `details_summary_box`, `details_summary`, `details_padding`, `footnote_reference`, `footnote`, `footnotes_rule_width` | `..ThemeMarkdown::default()` |
| `TextHighlight` | `padding`, `border` | `TextHighlight::new(range, color)` |
| `EventHandlers` | `press_repeats`, `on_copy`, `focuses`, `on_press_end` | `..Default::default()` |
| `SelectTriggerState` | `input` | `input: None` |
| `DragEvent` | `outside_window` | `outside_window: false` |
