# Design system implementation plan

Status: Phase 0 is built and released in lurq 0.61.0 (section 13). The tokens half of Phase 1 is built on `feat/design-system` (section 14). The rest is a plan.

Date: 2026-11-12. Repository baseline: lurq 0.60.0 plus Unreleased (`master` at `8770b598`).

Design source: [`pencil/lurq-design-system.pen`](../pencil/lurq-design-system.pen) — Foundations, 319 components for desktop, mobile and tablet, and Examples, in light and dark.

## 1. Goal

Ship the design system with lurq, driven entirely by the theme:

- **The Pencil design is the default.** Every lurq app has components that match the Pencil file in light and dark, with no feature to enable and no palette or state styling of its own.
- **The theme controls every component.** An app adjusts the global theme — tokens, per-component styling, and where needed a component's whole look — and every instance follows. Per-instance overrides exist but are the exception.
- **Everything can be overridden.** A developer can change all of it themselves: start from a theme with no styling at all, replace any recipe whole, replace how any part is built, add sizes and states of their own, and build a component of their own on the behavior lurq's components use (D8–D12). Nothing lurq draws is out of the theme's reach.
- **A theme can go as far as a native look.** A developer targeting iOS can make the same components look like the platform's own: different sizes and shapes, and different anatomy where the platform's control is built differently.

Non-goals for this plan:

- Anything the operating system draws: file, print and permission dialogs, native menus, the macOS menu bar, on-screen keyboards, home-screen widgets.
- A screen-reader bridge. It is a separate track; section 9 only keeps the door open.
- Android and web targets, and iPad support. Tablet components wait for iPad.
- Shipping finished native themes for every platform. The plan makes them possible and proves it with one iOS skin (section 7, step 2); complete platform themes are follow-up work.

## 2. Where the code stands

Findings from a read of `crates/lurq` (three inventories: components, theme, rendering and platform). File references are to `crates/lurq/src`.

**Solid base.** Gradients, multiple and inset box shadows, element and backdrop blur, rounded clips, popup anchoring with flip and clamp, a Tab-trapping modal layer, virtualized lists, flex wrap, transitions and keyframes, 35 cursor icons, breakpoints with `Responsive<T>`, and a reactive theme with role enums and `Extra` names.

**Three gaps sit between that base and the design.**

1. **State styles are narrow.** `Style` (`node/style.rs:20`) holds background, radius, borders, shadow, cursor, frame, padding and flex. A hover, pressed or focus style cannot change text color, opacity, gradient or transform, and there are only five states: hovered, active, focused, focus-visible, drag-over. There is no node-level disabled state; `.focusable(false)` blocks focus but clicks still run.
2. **The theme is one light palette.** 22 roles on a blue accent (`app/theme/palette.rs`), no dark palette, no scheme switch, no system appearance (winit's `ThemeChanged` is only named in a debug match, `app/winit_shell.rs:1953`). No tokens for motion, layers, opacity, control size, density, icon size, focus ring or materials. Text defaults to pure black rather than a palette role, and text color is not inherited. There is one app-wide theme and no public way to override it for a subtree.
3. **Few styled components, and no common way to theme them.** `Select`, the `form` compound controls and `Markdown` read the theme, each through its own hand-written struct (`SelectStyle`, `FormTheme`, `ThemeMarkdown`). `Button`, `TextInput`, `Link`, `Popup` and `Modal` are unstyled primitives. `Sheet`, `TabBar`, `NavigationBar` and the window chrome carry hard-coded colors with a `dark()` constructor each. Radio, switch, tooltip, menu, toast, progress, tabs for desktop, table, tree, calendar and about thirty more do not exist.

## 3. Decisions

Agreed on 2026-11-12. The phases below build on them.

| # | Decision | Agreed | Why |
| --- | --- | --- | --- |
| D1 | Where components live | A `lurq::ui` module in the `lurq` crate, always compiled — no feature flag | They are what lurq is by default, and they need crate-private pieces (rich text, overlay layers, interaction state). Components that build on an optional feature (`markdown`, `canvas`, `router`) stay behind that feature. |
| D2 | Palette role names | The design's names become the roles. Old roles that are no longer needed are deprecated, and a migration guide maps each one to its replacement | Code and the Pencil file use one vocabulary. lurq is pre-1.0 and has shipped migrations before (`docs/…/migration-v0-13.md`). |
| D3 | The `accent` collision | The design's `accent` and `accent-foreground` are renamed `highlight` and `highlight-foreground`, in the Pencil file and in code | In lurq `Accent` is the brand color; in the design `accent` is a subtle hover fill. With the rename, deprecated `Accent` can point at `Primary` without silently changing meaning. |
| D4 | Fonts | System fonts by default, nothing bundled. Typography — family included — is settable at every level of the theme: globally, per component, per variant, per size, per state, per part, and per component nested inside another. Two buttons in one message box can use two different fonts | Font choice is part of a component's look, not a global constant. See 4.2 and 4.3. |
| D5 | Icons | lurq draws its own glyphs for the icons its components need, and does not depend on Lucide. An `Icon` component takes a name; the theme maps names to glyphs, so an app or a theme can plug in any set | No third-party asset to license or track. A native iOS theme maps the same names to SF Symbols. See the icon notes below. |
| D6 | How components are themed | Every component is described by a **recipe in the theme**: styles for each of its parts, per variant, size and state. A theme may also replace a component's **renderer**. See section 4 | This is the requirement in section 1. It replaces one-off structs like `FormTheme` with one model every component shares. |
| D7 | The `form` controls | `FormTextInput`, `FormCheckboxInput`, `FormSliderInput` and the two buttons are rebuilt on the new components; `FormTheme` is deprecated in favor of recipes | One implementation of a button, and one way to theme it. |
| D8 | Starting from nothing | `Theme::bare()` is a definition with no styling: the token tables and the default renderers, and no recipe, so every part comes out unstyled and works. A recipe can be replaced whole as well as patched | A developer who wants a look of their own should not have to overrule the default one value at a time. |
| D9 | Renderers per part | A theme may replace the renderer of one part of a component, as well as the renderer of the whole component | Changing a switch's thumb should not mean writing the switch again. |
| D10 | Behavior | A theme cannot replace it. It is public instead: the state, keyboard handling, focus and semantics of each component are building blocks an app builds a component of its own on | A theme that replaced behavior could break focus or the keyboard in ways that are hard to find. An app that needs other behavior writes a component, from the pieces lurq's own are made of. |
| D11 | Open sets | Sizes and component states are keyed by name, as variants are, so an app adds its own. A renderer that replaces a component's may declare parts of its own, and a recipe addresses them by name | The fixed sets are lurq's defaults, not a limit. Settles open question 3. |
| D12 | Nothing hard-coded | Everything lurq draws takes its colors, sizes and durations from the theme, what an app never builds itself included: scroll bars, the select menu, text selection, the caret, tooltips, window chrome, enter and exit animations. A test fails on a literal color, size or duration in `lurq::ui` | "Everything" has to hold for what an app cannot reach any other way. |

| D13 | Breakpoints | A rule of a recipe may be for a breakpoint: it holds in a window that wide or wider, as `Responsive` values resolve. A component takes the breakpoint of the window it is built in and is built again when the width crosses one. A component that changes form with the width (a dialog that is a sheet on a phone, a side bar that folds) says so through its presentation, which may depend on the breakpoint too | The plan named breakpoints only as something lurq had. The design has sheets for a phone and a tablet, and a theme has to be able to say what a component is at each width. |

D8–D12 were agreed on 2026-11-13: a developer must be able to change everything themselves. D13 was added the same day, when breakpoints turned out to be missing from the plan.

**Icon notes (D5).**

- **The set is small.** Only what components draw themselves: chevrons and arrows, check, minus, plus, close, search, the status marks (info, success, warning, error), spinner, ellipsis, grip, calendar, clock, eye, copy, upload, external link, window controls and a few more — on the order of fifty. Icons an app shows in its own content are the app's to bring.
- **They are drawn in the Pencil file**: 152 glyphs on a 24-unit grid with a 2-unit stroke, one `Icon/<name>` component each, on the Icons pages next to a table from Lucide names to lurq names. Every icon in the file is an instance of one of them; no Lucide glyph is left. A few need a second pass: settings, airplane, quote, strikethrough, bug, the three hands, megaphone, replace and replace-all.
- **They reach the screen as a hinted icon font.** The sources are in the repository: one SVG of filled outlines per glyph in `design/icons/svg`, exported from the Pencil file. `scripts/build-icon-font.py` builds `crates/lurq/assets/lurq-icons.ttf` from them and writes the TrueType hints itself; with `--from-board` it first exports the glyphs from the Pencil file, through the running Pencil app, so one command goes from the board to the font. `design/icons/codepoints.json` gives each name a codepoint from U+E000 up; a name keeps its codepoint for good, and a new glyph takes the next free one. A font needs no optional feature because text rendering is always present.
- **What the hints do.** A straight stroke is one pixel wide up to 17 px. Above that it keeps the width it was drawn with, with the edge that faces out of the glyph on the pixel grid, so it weighs what the diagonal next to it does. Strokes are placed from the middle of the glyph outward and keep a whole pixel of white between them; where the white is too narrow for that, a stroke is drawn in whole pixels. Diagonals and free stroke ends are left alone. None of it shows until the glyph engine renders this font hinted (E9 in section 5).
- **Lucide today.** lurq already ships `assets/lucide.ttf` in the published crate, for DevTools, with no license notice in the repository. Lucide is ISC and its Feather-derived icons are MIT; both allow this but require their notices to accompany copies. Either DevTools moves to the new glyphs and the file is removed, or a notice file is added until it does. A wider license audit is under way separately.

## 4. Theming model

Three layers. Each is set on the theme; each has the Pencil design as its default.

```
Tokens      palette, radius, spacing, type, motion…   "what blue is, how round is md"
   ↓ referenced by
Recipes     per component: parts × variant × size × state   "a primary button is filled, 36 tall"
   ↓ consumed by
Renderers   per component: builds the node tree from parts   "a switch is a pill with a thumb"
```

Most apps touch only tokens. A brand refresh edits recipes. A native platform look replaces renderers for the controls whose anatomy differs.

### 4.1 Tokens

The global scales and roles of section 6. Changing a token changes every recipe that refers to it, so "make everything rounder" or "use our blue" is one line.

### 4.2 Recipes

A component is a set of named **parts** — a button has `root`, `label`, `icon` and `spinner`; a switch has `track` and `thumb`. A recipe gives each part a style for every combination the component supports:

- **Base**: applies always.
- **Variant**: `Primary`, `Secondary`, `Outline`, `Ghost`, `Destructive`… Variants are keyed by name, so an app can add its own.
- **Size**: `Sm`, `Md`, `Lg`, or a density. Sizes are keyed by name too, so an app can add its own (D11).
- **Breakpoint**: a rule for a window at a breakpoint or wider (D13): a part that is one size on a phone and another on a desktop.
- **State**: `hovered`, `pressed`, `focus_visible`, `disabled`, and component states such as `checked`, `selected`, `invalid`, `loading`, `open`. A state can also be one an app names: an instance turns it on with `.state(name, on)`, and a recipe styles it like any other (D11).

All of them use one shared type, a **part style**: an all-optional patch of background, foreground, border, radius, shadow, outline, opacity, padding, gap, height, width, typography and transition. Values are token roles or concrete values. Because every component uses the same patch type, learning to theme one component is learning to theme all of them.

Typography in a part style is the whole text style, each field optional: font family, size, weight, style, line height, letter spacing and font features. A typography role sets all of them at once; any single field can then be overridden. So a font can differ per component, per variant, per size, per state and per part — a primary button in one family and an outline button in another is two lines in the theme.

Resolution order, later wins: base → variant → size → breakpoint, a wider one over a narrower → state → the enclosing component's patch (4.3) → the instance's own override.

```rust
// Illustrative — the exact API is settled in step 2 of section 7.
let ui = app.theme().ui();

// Every primary button becomes a pill.
ui.button().variant(ButtonVariant::Primary).part(ButtonPart::Root, |s| s.radius(RadiusSize::Full));

// Medium controls are 44 tall everywhere.
ui.button().size(ControlSize::Md).part(ButtonPart::Root, |s| s.height(44.0));

// An app-defined variant.
ui.button().variant("brand").part(ButtonPart::Root, |s| s.background("brand").foreground("on-brand"));
```

A recipe is patched, as above, or replaced whole: `ui.button().replace(recipe)` drops everything the definition under it said about the component, so a look can be written from nothing for one component without starting the whole theme from `Theme::bare()` (D8).

Pointer and focus states in a recipe compile to node state styles (section 5, E2–E4), so a hover never re-renders the component. Component states are resolved when the component renders, which already happens when they change.

### 4.3 Components inside components

A message box contains two buttons; a date picker contains a field, a calendar and buttons; a table toolbar contains a search field and a menu button. A composite names each inner component as a **slot**, and its recipe can patch the recipe of whatever sits in that slot.

```rust
// Illustrative. In every message box, the confirm button is set in a serif face
// and the cancel button in the theme's mono family.
let message_box = ui.message_box();
message_box.slot(MessageBoxSlot::Confirm, |button| {
  button.part(ButtonPart::Label, |s| s.font_family("Georgia").weight(FontWeight::Bold))
});
message_box.slot(MessageBoxSlot::Cancel, |button| {
  button.part(ButtonPart::Label, |s| s.font_family(FontFamily::Mono))
});
```

- A slot patch applies only to that component in that place. Buttons elsewhere keep the button recipe.
- Slots nest: a preferences window can reach the button inside the message box it opens.
- A slot also fixes defaults the composite chooses, such as "the confirm slot is a primary button", and the theme can change that choice.
- The same mechanism serves an app's own composites, so app-level components are themable the same way.

### 4.4 Renderers

A recipe can restyle a control but cannot change what it is made of. An iOS switch, a segmented control, a navigation bar or an alert with hairline-separated buttons differs in anatomy, not only in color.

So each component is split in two:

- **Behavior** — state, keyboard handling, focus, semantics, events. Owned by lurq: a theme cannot replace it (D10).
- **Renderer** — a function from the component's resolved state and recipe to a node tree. The default renderer draws the Pencil design. A theme may register another.

```rust
// Illustrative.
ui.switch().renderer(|switch, recipe, ctx| {
  // Build an iOS-shaped track and thumb from `switch.checked()`, `switch.interaction()` and `recipe`.
});
```

A replaced renderer still gets the recipe, so tokens and recipes keep working inside a native skin.

- **One part at a time (D9).** A theme may replace the renderer of a single part: `ui.switch().part_renderer(SwitchPart::Thumb, |thumb, style, ctx| ...)`. The rest of the component is still drawn by the renderer it had.
- **Parts of its own (D11).** A renderer that replaces a component's may build parts the default one does not have. It declares them by name, and a recipe styles them by that name. The parts lurq names are the ones every renderer of the component is expected to have, so a recipe written for one renderer still applies under another.
- **Behavior is public (D10).** What a renderer is handed, the component's state, its keyboard handling, its focus and its semantics, is public API under `lurq::ui`: an app that needs a control lurq does not have, or one that behaves differently, builds it from the same pieces and gives it a recipe and slots like any component of lurq's.

### 4.5 Presentation

Some differences are behavioral conventions rather than looks: whether a select opens a menu or a bottom sheet, the order of confirm and cancel in a dialog, whether a tooltip exists on touch. The theme carries these as per-component **presentation** options with platform-appropriate defaults, for example `SelectPresentation::{Menu, Sheet}`.

### 4.6 Whole themes, platforms and scopes

- **Theme definitions.** A definition bundles tokens for light and dark, recipes, renderers and presentation. `Theme::apply(definition)` installs one atomically. `Theme::neutral()` — the Pencil design — is the default.
- **A bare theme (D8).** `Theme::bare()` has the token tables and the default renderers and no recipe: a button is its label, a switch is a track and a thumb with no fill. Every component still works. It is the base for a look that shares nothing with the default.
- **Derived themes.** A definition can extend another and override part of it, so an app's theme is "neutral, plus these changes" rather than a copy.
- **Per platform.** An app picks a definition per target, for example the neutral theme on desktop and a native-looking one on iOS. lurq does not switch looks by platform on its own.
- **Scoped themes.** A public way to override the theme for a subtree — a dark sidebar in a light window, or a compact-density inspector. Today `Ctx::with_theme` is crate-private.

### 4.7 What this replaces

`FormTheme`, `SelectStyle`'s themed defaults, and the hard-coded `new()`/`dark()` pairs on `Sheet`, `TabBar`, `NavigationBar` and the window controls all become recipes. `ThemeMarkdown` moves from concrete colors to roles.

### 4.8 Nothing drawn outside the theme

D12. What lurq draws without an app building it is themed like a component: scroll bars, the menu of a select, text selection and the caret, tooltips, window chrome, and the animations layers and list items enter and leave with. Each has a recipe or tokens, and where its anatomy can differ, a renderer. In `lurq::ui` no color, size or duration is written as a literal: each comes from a token or a recipe, and a test (section 8) fails on one that does not. Primitives outside `lurq::ui` (`Rect`, `Text`, `Row`) take what they are given and have no look to override.

## 5. Phase 0 — Style engine

Small, self-contained changes to nodes. Each lands with tests and a docs section, and none depends on the others. Recipes need E1–E4 to express their states without re-rendering.

All of them are built. Section 13 says where each stands, what was decided while building it, and what still has to be checked on platforms it was not built on.

| Item | Change | Notes |
| --- | --- | --- |
| E1 Foreground color | An inherited `foreground` on nodes: `.foreground(color)`. `Text` and `Icon` without an explicit color use the nearest ancestor's foreground, then the theme's `Foreground` role | Fixes the pure-black default and lets one part recolor its label and icon together. |
| E2 State styles | Add `foreground` and `opacity` to `Style` | Paint-only, so no relayout. A ghost button's label or a menu item's text can change on hover without a re-render. |
| E3 Disabled | `.disabled(bool)` on nodes, a `disabled` bit in `InteractionState`, and a `.disabled_style(...)` slot. A disabled node takes no pointer or keyboard input, leaves the tab order, and passes the state to its subtree | Today only `SelectOption` and form `Control` know the word. |
| E4 Outline | `.outline(width, color, offset)`, also settable in state styles. Painted outside the border, takes no layout space | The focus ring in the design sits two pixels off the control. A border cannot do that without moving things. Containers clip by default, so the ring needs the same care as shadows: document it, and consider painting outlines in the overlay pass. |
| E5 Text decoration | Draw `underline` on plain `Text`; add `strikethrough` to `TextStyle` and to `MarkdownTextStyle`; add `max_lines` with an ellipsis on the last line | Links, struck task items in Markdown, and two-line card descriptions all need these. |
| E6 Transitions | Make `foreground`, `box_shadow` and `outline` animatable | Opacity, background, border and transform already are. |
| E7 Presence | An enter and exit animation hook for layers: content stays mounted until its exit animation ends | Popovers, dialogs and toasts fade and slide in the design. Today only `Sheet` and `StackOutlet` animate, each with its own code. |
| E8 Reduced motion | Read the system setting where the platform has one; expose `ctx.reduced_motion()`; durations resolve to zero or a fade | Docs currently state lurq does not read it. |
| E9 Hinted icons | The glyph engine renders the icon font hinted and snapped to whole pixels: whole sizes, whole positions. Today `render_glyph_image` forces hinting off for all fonts, which is right for text placed at fractions of a pixel and stays so | The font opts into native ClearType in `prep`; without that a smooth rasterizer ignores hints along x. swash's own scaler ignores them either way, because it pins `preserve_linear_metrics`. So the icon path hints the outline through skrifa (smooth mode without linear metrics) and hands it to zeno, the rasterizer swash uses. |

Dashed borders, a single-side border that follows the corner radius, grid layout and sticky positioning were deferred until a component needed them, and were then built with the rest: section 13 has them. No z-index — overlay layers already cover it.

## 6. Phase 1 — Theme

**Palette roles.** Replace the palette with the design's set, with light and dark values:

- Surfaces and content, in pairs: `Background`/`Foreground`, `Card`, `Popover`, `Primary`, `Secondary`, `Muted`, `Highlight`, `Destructive`, each with its `…Foreground`.
- States: `PrimaryHover`, `SecondaryHover`, `DestructiveHover`, `StateHover`, `StatePressed`.
- Status: `Success`, `Warning`, `Info`, `Destructive`, each with a `…Muted`.
- Lines: `Border`, `Input`, `Ring`.
- Extras: `Canvas`, `Scrim`, `Selection`, `Chart1`…`Chart5`, `GlassThin`, `GlassRegular`, `GlassThick`.

Deprecated roles and their replacements, for the migration guide (D2): `Accent` → `Primary`, `SurfaceBase` → `Background`, `SurfacePanel` → `Canvas`, `SurfaceRaised` → `Card`, `SurfaceInput` → `Background`, `BorderStrong` → `Input`, `BorderFocus` → `Ring`, `TextPrimary` → `Foreground`, `TextSecondary` and `TextMuted` → `MutedForeground`, `TextInverse` → `PrimaryForeground`, `Danger` → `Destructive`, `AccentHover` → `PrimaryHover`. `AccentMuted` has no exact match; it maps to `Highlight`.

**Schemes.**

- `ColorScheme { Light, Dark }` and `Appearance { Light, Dark, System }`.
- A theme definition holds both palettes; `theme.set_appearance(...)` switches in one version bump.
- System appearance on Windows, macOS and Linux through winit, and on iOS through the trait collection; `System` follows changes live.
- Changing the default colors is a visible break for apps that never set a palette. `Theme::legacy()` restores the 0.60 look; the change goes in the migration guide.

**New token tables**, each a role enum with an `Extra` variant, following the existing pattern:

| Table | Roles | Values from the design |
| --- | --- | --- |
| Radius | add `Xl`, `Full` | 4, 6, 8, 12, pill (today 3, 5, 6) |
| Control size | `Sm`, `Md`, `Lg` | 32, 36, 40 |
| Icon size | `Xs` … `Xl` | 12, 14, 16, 20, 24 |
| Duration | `Fast`, `Base`, `Slow`, `Slower` | 100, 160, 240, 400 ms |
| Easing | `Standard`, `Enter`, `Exit` | the three curves in Foundations |
| Opacity | `Disabled`, `Muted` | 0.5, 0.7 |
| Blur | `Thin`, `Regular`, `Thick` | 8, 20, 40 |
| Focus ring | width, offset | 2, 2 |
| Density | `Compact`, `Default`, `Comfortable`, `Touch` | 28, 36, 44, 48 |

**Typography.** Give each role its own line height; point `Body` at the palette foreground through E1; add the heading scale used by Markdown. Font families become two theme values, `sans` and `mono`, defaulting to the system's generic families (D4); every typography role refers to one of them, so substituting a font is one setting.

**Pencil file.** Rename the `accent` and `accent-foreground` variables to `highlight` and `highlight-foreground` (D3), so the parity test in section 8 compares like with like.

**Theme machinery** from section 4:

- The part-style patch type and recipe storage, with resolved recipes cached per theme version so a render does not re-merge them.
- Renderer registration and presentation options.
- Theme definitions, `Theme::apply`, derived definitions.
- Public scoped themes.
- `Theme::bare()`, and replacing a recipe whole (D8).
- Renderers per part, and parts a renderer declares (D9, D11).
- Sizes and states by name (D11).
- Recipes and tokens for what lurq draws itself: scroll bars, text selection, the caret (D12). The select menu, tooltips and window chrome follow with their components in phases 2, 3 and 6.

## 7. Phases 2–7 — Components

Every component ships with the same six things: behavior separate from its default renderer; a recipe with the Pencil values; `variant` and `size` where the design has them and every state the design shows; keyboard behavior; tests under `crates/lurq/tests/ui/`; a docs page and a gallery entry (section 8).

**Step 2 — Prove the model before building on it.** Four components chosen to stress it differently: `Button` (variants, sizes and pointer states), `Switch` (anatomy that differs by platform), `Tabs` (several parts and a selected state) and `MessageBox` (a composite with two buttons in slots). For those four:

1. Build the default renderers and recipes from the Pencil design.
2. Write an iOS-looking skin for them in `crates/mobile_demo` using only public theme API: tokens, recipes, and a renderer for `Switch`.
3. Restyle them for a made-up brand using tokens and recipes alone.
4. From the theme alone, give the message box's two buttons different fonts, and give primary and outline buttons different fonts everywhere else.

5. Build the four from `Theme::bare()` to a look that shares nothing with the default, with public API alone (D8).
6. Replace only the thumb of the switch (D9), give the button a size and a state of the app's own (D11), and write a control lurq does not have on the public behavior pieces, themed by a recipe (D10).

If any of these needs something the theme cannot express, fix the model here. The recipe, slot and renderer API is frozen only after this step. Nothing else in phases 2–7 starts before it.

**Phase 2 — Core controls.** The pieces everything else is built from.

The glyph set from D5, then `Icon`, `Button` (five variants; sizes; icon, icon-only, loading, disabled), `Link`, `Input` and `Field` (sizes; error, valid, read-only, disabled; leading icon, clear), `Textarea`, `Checkbox` (with indeterminate), `Radio` and `RadioGroup`, `Switch`, `Slider` (range, vertical, marks), `Select` rebuilt on a recipe (sizes, invalid, disabled, groups), `Badge`, `Tag`, `Chip`, `Avatar`, `Separator`, `Kbd`, `Spinner`, `Progress`, `Skeleton`, `Card`, `Alert`.

**Phase 3 — Overlays.** Built on `Popup`, `Modal` and E7.

`Tooltip`, `Popover`, `Menu` (dropdown and context; checkbox and radio items; submenus; shortcuts), `Dialog` and `AlertDialog` (scrim, sizes, scrolling body), `Sheet` on a recipe, `Toast` with a stack and positions, `HoverCard`, `Combobox`, `CommandPalette`.

**Phase 4 — Navigation and structure.**

`Tabs` (segmented, line, pill, vertical, overflow; optional counts), `Accordion`, `Collapsible`, `Breadcrumb`, `Pagination`, `Steps`, `Sidebar`, `Menubar`, `Toolbar`, `Splitter`, `TreeView`, `Navbar`.

**Phase 5 — Data and pickers.**

`Table` and `DataGrid` (sort, selection, empty, loading, toolbar), `ListItem`, `EmptyState`, `DataList`, `Timeline`, `Calendar`, `DatePicker`, `DateRangePicker`, `TimeField`, `ColorPicker`, `FileUpload`, `NumberInput`, `InputOTP`, `TokenInput` on a recipe, `Markdown` on roles. Charts last, on `Canvas`: bar, line, area, pie, with a shared legend and tooltip.

**Phase 6 — Desktop shell.**

Window chrome on recipes, with Windows, macOS, GNOME and KDE as four renderers of one component, and tabs in the title bar; `DocumentTabs`, `StatusBar`, `PropertyGrid`, `FindBar`, `ShortcutRecorder`, `MessageBox`, `DockLayout`. `DockLayout` is the largest single item in the plan and may deserve its own proposal.

**Phase 7 — Mobile.**

Touch sizes through the `Touch` density; `TabBar`, `NavigationBar` and `Sheet` on recipes, their current iOS look kept as a renderer rather than hard-coded colors; list rows with edit and select modes; swipe actions; pull to refresh; bottom sheet pickers; toast, banner and in-app notification. Swipe actions and pull to refresh are listed as unsupported in `docs/ios-gaps.md` and need gesture work first.

Not planned as components: the Examples page (whole windows and screens) stays as demos; rarely used items from the design — signature pad, angle slider, QR code, watermark, transfer, cascader — wait for a request.

## 8. Verification

- **Gallery.** The app drawn in `pencil/lurq-design-system-demo.pen` (section 15): a page per component, with a playground, a section per prop axis, a matrix of states and a table of props, and switches for the appearance and the density that reach every demo on the page. It takes the place of `crates/demo`. It is the place to compare against the design and the first consumer of every API, and it is built with the components: each component lands with its page.
- **Screenshot tests.** The `screenshot` feature renders each gallery page in both schemes; images are compared against stored references so a token or recipe change that shifts a component shows up in review.
- **Behavior tests.** Per component under `crates/lurq/tests/ui/`: states, keyboard, disabled, and focus order. They run against the behavior half, so they hold under any renderer.
- **Theme tests.** A second, deliberately different theme definition runs through the gallery in CI. A component that ignores its recipe — a hard-coded color, a fixed size — shows up as unchanged.
- **Nothing hard-coded (D12).** A test reads the sources of `lurq::ui` and fails on a literal color, size or duration outside the neutral theme's definition. The gallery also runs on `Theme::bare()`: every component has to work with no recipe at all.
- **Design parity.** Token values are exported from the Pencil file's variables and checked against the neutral theme's defaults in a test, so the two cannot drift.

## 9. Accessibility hook

No bridge in this plan, but every component declares a semantic role, name and state on its node from day one, in the behavior half so a replaced renderer cannot drop them. A later AccessKit integration then has the data it needs without touching each component again.

## 10. Order and size

| Step | Depends on | Rough size |
| --- | --- | --- |
| Phase 0, E1–E4 | — | small each; E1 touches text layout and is the largest |
| Phase 0, E5–E9 | — | small to medium; E7 is the largest |
| Phase 1, tokens and schemes | E1 | medium; the palette migration touches every themed component and the docs |
| Phase 1, theme machinery | E1–E4 | medium to large; it is new architecture |
| Step 2, proof on four components | Phase 1 | medium; expect to revise the machinery here |
| Glyph set (D5) | E9 to look right | drawn in Pencil (152 glyphs), with sources, generator and hinted font in the repository; left: a second pass on a few |
| Phase 2 | Step 2, glyph set | large; about 25 components |
| Phase 3 | Phase 2, E7 | large |
| Phase 4 | Phase 2 | medium |
| Phase 5 | Phases 2 and 3 | large; table and calendar dominate |
| Phase 6 | Phases 2–4 | large; dock layout dominates |
| Phase 7 | Phases 2 and 3 | medium, plus gesture work |

Phase 0 and the tokens half of Phase 1 can be reviewed and released on their own: they improve lurq for every app, including ones that never use `lurq::ui`. A first useful release of the components is Phase 2 plus `Tooltip`, `Menu`, `Dialog` and `Toast` from Phase 3.

## 11. Risks

- **The theming API is the hardest part to change later.** Every component and every app theme depends on it. Step 2 exists to find its flaws on four components instead of thirty.
- **Recipe surface area.** A patch type with fifteen optional fields, times parts, variants, sizes, states and nested slots, is a lot to document and keep stable. Keeping one shared patch type, and adding fields only when a component needs them, holds it down.
- **Public behavior is API too (D10).** The pieces a component is built on become something apps depend on, and are frozen with the recipe API after step 2. What is not ready to be promised stays crate-private until it is.
- **No feature flag.** Every app compiles every component. Unused ones do not reach the binary, but build time grows with the set. If it becomes a problem, the large groups (data grid, charts, dock layout) can move behind features later without changing the rest.
- **The icon font's hints are our own.** The 152 glyphs are in the repository as sources and as a font, and E9 draws them hinted. The hints are written by `scripts/build-icon-font.py`, not by a tool with a history, so every glyph added should be looked at on a comparison sheet at 12 to 24 px. A few glyphs still need redrawing. Nothing draws the font yet outside `examples/icon_hinting.rs`: DevTools is still on Lucide (question 8) and `Icon` is Phase 2.
- **Renderers and behavior drifting apart.** A custom renderer that forgets to wire a part breaks hit-testing or focus. The behavior half must own events and semantics, and hand the renderer parts that already carry them.
- **Resolution cost.** Merging base, variant, size and state on every render would be slow. Resolved recipes are cached per theme version and per combination.
- **Breaking changes.** New default colors, renamed palette roles, a deprecated `FormTheme`. Mitigation: deprecated roles keep compiling and resolve to their replacements, a migration guide, and `Theme::legacy()`.
- **Inherited foreground and caching.** Text layout caches key on style. Foreground is paint-only and must stay out of shaping keys, or a hover will reshape text.
- **Clipping.** Containers clip by default, so focus rings, shadows and tooltips near an edge are cut. E4 has to settle where outlines paint.
- **Scope.** The design has 319 components. Phases 2–4 cover what most apps use; phases 5–7 should each be re-planned when reached rather than committed to now.
- **Design drift.** The Pencil file will keep changing. The parity test in section 8 turns silent drift into a failing check.

## 12. Open questions

1. How long deprecated palette roles stay before removal: one release, or until 1.0?
2. Recipes as builder calls on the theme (as sketched in section 4) or as plain data structs an app constructs and installs? Data is easier to derive and diff; builders read better. Step 2 should try both.
3. Settled (D11): a renderer builds the parts lurq names for the component and may declare parts of its own, which recipes address by name.
4. Does lurq ship a native-looking iOS theme itself, after the proof in step 2, or leave it to apps and the community?
5. Should the new components be configured through builders like the existing `components`, or through props structs like the `form` compound controls?
6. Settled on 2026-11-13: the default theme is the design's from the release that brings its tokens, before the components. `ThemeDefinition::legacy()` keeps the look of 0.61, and the migration guide (`docs/…/migration-v0-62.md`) says what changes.
7. Settled: the glyphs reach the screen as a hinted icon font, built by `scripts/build-icon-font.py` from `design/icons/svg` (icon notes, D5). Vector paths would have needed the `svg` or `canvas` machinery, or a new path primitive.
8. Does DevTools move to the new glyphs at once, so `lucide.ttf` can be removed, or keep Lucide with a notice file for now?
9. Settled on 2026-11-13: the gallery is an app of its own, drawn in `pencil/lurq-design-system-demo.pen`, and takes the place of the demo (section 15).
10. Which platforms must screenshot tests run on in CI?

## 13. Phase 0: status and handover

Written when all of Phase 0 was built, on branch `feat/design-system`. It is for whoever picks the work up next, and for checking it on the platforms it was not built on.

### 13.1 Where each item stands

| Item | State | Commits |
| --- | --- | --- |
| E1 Foreground | built | `e6c8a343` |
| E2 State styles | built | `5d33f980` |
| E3 Disabled | built | `697fcfea`, then `7e3789bf` (scrolling), `b3db6430` (state for every element) |
| E4 Outline | built | `5d33f980`, then `d4a7bd32` (clipping) |
| E5 Text decoration | built | `1298fcad`, then `058784a4` (node size), `9b2c264e` (line limit as painted) |
| E6 Transitions | built | `29f95d08` |
| E7 Presence | built | `bee2b8d8` |
| E8 Reduced motion | built; macOS and iOS not built or run (13.3) | `9986db86` |
| E9 Hinted icons | built in its own session and merged | `19c57f9b`, `3d54ae3a`, merge `902c03e0` |
| Theme `Foreground` role | built | `dd5235bc` |
| Borders: one side follows its corners; dashes | built; Metal not looked at (13.3) | `96c04710` |
| Sticky positioning | built | `244c6b20` |
| Grid | built | `1122c430` |
| Presence for a container's children | built | `7f96b5a0` |

Everything was built and run on Windows only. The check is `scripts/check.ps1`: the main test suite, the suite with `devtools,mcp,perf_profile,counting_allocator`, the release stack-depth guard, and a build of the tests with the features of both suites together, which is where the size budgets of `size_budget.rs` are checked for a build that has everything on. All four passed at `2c40c46c`, and again, with the readback tests, at `a633412c`, where the branch was merged with its remote, which by then had master in it; start there when something fails later. The tests that read frames back from a GPU are ignored by default and run by hand: `cargo test -p lurq --features wgpu,dx12,screenshot --lib readback -- --ignored --test-threads=1`, on Windows only, since they open a hidden Win32 window. Clippy, over every target with every feature, is clean in the lines this branch wrote (`2c40c46c`) and reports 215 findings in the rest of the crate, which were there before and were left. `cargo fmt --check` reports differences in five files this work did not write (`winit_shell/frame_source.rs`, `winit_shell/touch.rs`, `components/sheet.rs`, `components/stack_outlet.rs`, `tests/components/sheet.rs`); they were there before.

`586e84c2` is not part of the plan: one line in a test that kept the second suite from compiling on the branch before this work began. Two more commits fix what later work found in earlier work: `c920668b` (a component that rendered again wrote into a layer on its way out) and `8982356f` (a node was over its size budget with every feature on, which no suite turned on together).

### 13.2 What was decided while building

Where the code differs from the table in section 5, the code is what was decided. User-facing behavior is in `docs/src/content/docs/styling-events.md`, `layout.md`, `animation-transforms.md` and `modals.md`, and in the changelog.

- **E1.** With no foreground above it a text keeps its typography style's color, so a tree that sets none paints as before. The theme's `Foreground` role (`PaletteColor::Foreground`) is the color of a text whose style keeps the default text color; it is black by default, so nothing changes until a theme sets it, and Phase 1 gives it the design's value. A `TextInput` and Markdown draw in their own styles and do not read the foreground, as an `<input>` does not inherit `color`. An overlay's content is painted outside the element that declares it and starts from no foreground.
- **E2.** State styles carry `foreground`, `opacity` and `outline`. Where several set a value the last applied wins: disabled, then drag-over, active, hovered, focus-visible, focused.
- **E3.** A press or click on a disabled element reaches nothing, the elements it is in included, as a disabled control on the web: a clickable card is not clicked through its disabled button. A pointer moving over it still reaches those elements. Scrolling is not input to a control and works as if nothing were disabled, inside the element too. Hit testing folds a disabled element's subtree into one hit; that is the mechanism to know before touching input code. The disabled style is paint-only. The state is written in the walk that assigns ids to a built tree, so it costs no walk of its own.
- **E4.** `Outline` is a type (`Outline::new(width, color).offset(px)`), not three arguments. Containers clip their content by default, so an outline takes the clip of its element let out by its own reach; scroll viewports and the window, which the paint walk tracks apart, cut it where they cut the element. An outline is painted over the elements painted after its own, at the end of the scope its element is in: a scroll container (under its border and bars), an element with an opacity or a blur (inside its layer), the siblings that scroll under an element that sticks, the page and each layer over it, or the tree. The paint walk keeps the outlines it meets (`pending_outlines`) and each scope paints those kept since it began.
- **E5.** `TextStyle` and `MarkdownTextStyle` gained a public field, `strikethrough`: a source break for code that names every field, accepted and listed in the changelog. Markdown now draws `~~struck~~` text struck. `Text::max_lines` holds in the layout that paints the text as well as the one that sizes it.
- **E6.** The outline animates as one value, and no outline is a value too, so a ring grows in and fades out. Transitions now read colors, widths and shadows given as theme roles; before, a color given as a role changed at once whatever its transition said. A role whose value changes with the theme runs the transition too. A list of more than four shadows changes at once.
- **E7.** There is no `Presence` type. An element enters with the animation it plays from its mount, which a layer's content already did, and `.exit_animation(animation)` on any element names what it leaves with. A closed layer with an exit animation anywhere in it stays in the overlay host until the longest ends, so a dialog's backdrop and panel leave each in its own way. What stays is the layer's nodes as they were when it closed: the app does not render them again. The layer is only painted: no pointer, focus or Tab, no modal trap, no lookup by id or ref (`node/presence.rs`, `app/runtime/leaving_layers.rs`). A layer opened again while it leaves is a new layer over the old one, not the old one turned around. A popup hidden with its anchor does not leave.
- **E7, layer identity.** Building it showed that open layers were matched between passes by their place in the host: when the first of two popups closed, the second took over its state. A layer is now the layer the same declaring node opened last pass, then the layer of its key, and by place only when the declaring node is new. A layer with an animation in it is never matched to another declaration's, which is what lets one popup leave while the next enters.
- **Presence for children.** A container asks for it with `.presence()`, and only a child with a key or an id and an exit animation of its own is kept: children without a key are matched by their place, where a removed one and a changed one look the same, and a list that mounts only the rows in view removes rows as it scrolls. The kept child stays in flow; an exit animation of its height lets its siblings close the gap. It is carried from the old tree to the new one in the walk that carries node ids (`Node::preserve_ids_from`), and the walk that carries state matches the other children as if it were not there. A popup that keeps one is measured and placed again.
- **The theme's foreground.** A style that keeps the color of `TextStyle::default()` names no color, and is drawn in `PaletteColor::Foreground`: a `Text`, a `TextInput` with its caret, a Markdown document. A typography style with a color of its own keeps it. The role is black by default, so nothing changes until a theme sets it; Phase 1 gives it the design's value. Telling "no color" by the default color is a convention, not a type: a text that must be exactly that black under another foreground names it with `Text::color`.
- **Borders.** A stroke is the ring between its outer edge and its hole, and the hole's corners are the ones CSS gives a padding box. The formulas are `layout/stroke.rs`; `quad.wgsl`, `quad.hlsl` and the software screenshot renderer paint with them, and `stroke_readback_tests.rs` compares both backends with them over thirteen scenes. A renderer takes the width of its anti-aliasing ramp from derivatives over two by two pixels, so the comparison is strict where a pixel is whole or empty and loose on an edge. Dashes run along the outline with the period fitted to it; they are not fitted per side as a browser fits them, which is what a design tool draws. `RectCmd` gained `stroke_style`.
- **Sticky.** Where a sticky element is drawn is written into its parent's layout when the scroll container is synced with its scroll, as the content's offset is, so every walk of the layout sees it there and scrolling lays nothing out. The setting is in the node's box of rare styles, not in `Position`, which every node holds. It is painted after its siblings and hit before them.
- **Grid.** A subset of CSS grid: tracks of pixels, shares and content; gaps; auto-placement row by row; spans; a named column or row. No named lines or areas, no `minmax`, no dense packing, no per-child alignment. A child with a size of its own is not stretched. A grid lays out again whenever a child changed. Placement and track sizes are `layout/grid.rs`, with no layout engine in them.
- **E8.** The setting lives in the theme: `Theme::reduced_motion()`, `Theme::set_reduced_motion(Option<bool>)`, `Ctx::reduced_motion()`. The shell reads the system when a window opens and when one takes focus. While motion is reduced, transitions of transform, offset, width and height end at once; fades and color changes run; keyframe animations are left to the app. A tree with no shell never reads the system, so tests do not depend on the machine.
- **E9.** `App::load_hinted_font(data)`. The hinted outline comes from skrifa directly and is rasterized by zeno, because swash's scaler drops hints along x. Only pixel-snapped runs are hinted; a glyph under a transform is drawn unhinted.

### 13.3 To check on macOS and iOS

None of this has been compiled for an Apple target.

**The reduce-motion readers** were written blind. The macOS one has since been built on a Mac, where it needed one fix (`fc30d2f6`): without the winit shell nothing linked AppKit, and the lookup of `NSWorkspace` failed in the lib's tests. Whether it reads the setting right in a window, and the iOS one, are still to check. As first written: `crates/lurq/src/app/theme/motion.rs` has one `platform` module per target. The Windows one is built and tested. The other two were written from the API documentation.

- macOS sends `+[NSWorkspace sharedWorkspace]` and `-accessibilityDisplayShouldReduceMotion` with `objc2::msg_send!`, through `class!(NSWorkspace)`. To check: that it compiles with objc2 0.6 (`msg_send!` on a `*mut AnyObject` receiver, returning `bool`); that `NSWorkspace` resolves, which needs AppKit linked, as `objc2-app-kit` already links it; that the call is safe from the thread the shell calls it on, the main thread.
- iOS declares `UIAccessibilityIsReduceMotionEnabled()` as `unsafe extern "C"` in UIKit, returning `bool`. To check: that it links, and that it is right on the simulator target as well as the device.

Steps on a Mac:

1. `cargo test -p lurq --lib motion`. `the_platform_has_the_setting` fails if the reader returns `None`.
2. `cargo check -p lurq --features winit,wgpu`, then the whole check: the three commands of `scripts/check.ps1`, run by hand, since the script is PowerShell.
3. Run an example with a transition of a transform or a size, turn on System Settings › Accessibility › Display › Reduce motion, click back into the window: the transition should end at once, and run again after the setting is turned off. The setting is read on focus, not while the window stays focused.
4. iOS: build `crates/mobile_demo` for the device and the simulator (`crates/mobile_demo/ios/build-rust.sh` is what Xcode runs), turn on Settings › Accessibility › Motion › Reduce Motion, come back to the app: a sheet and a stack push should switch without the slide and still fade.

**Built on Windows, not looked at on a Mac or a phone.**

- E3 on touch. The touch paths (`app/runtime/touch.rs`, `touch_pointer.rs`) take a disabled element out of taps, long presses and the pressed look, and let a finger scroll over and inside it. The tests drive these paths, but not through a real finger.
- E9. The hinted path was looked at in a window on Windows only. On macOS and iOS: `cargo run -p lurq --example icon_hinting --features winit,wgpu,screenshot` on a 1x display if one is at hand; on a 2x display hinting matters little.
- E4 and E6 paint through quads every backend already draws, so nothing there is specific to a platform; a look at a focus ring growing in on Metal is still worth a minute.
- **The stroke shader on Metal.** Borders are painted by `quad.wgsl`, which was rewritten for strokes and is turned into Metal's shading language on a Mac and a phone. It was compiled and compared with its formulas on Direct3D only: the readback tests need a Win32 window. On a Mac run `cargo run -p lurq --example engine_gallery --features winit,wgpu,screenshot -- --capture target/gallery.png` and look at the frame, and at the window without `--capture`. If the shader does not compile for Metal the app stops with a wgpu validation error as its window opens. Otherwise look at the six cards under "Borders": a blue bar on the left of the first that follows both corners and thins out; a thin line on top and a thick one below on the second, meeting in smooth corners; a ring around the third that is outside the card and rounder than it; dashes that go around the corners on the fourth and fifth; a dashed line under the sixth. Nothing should show along an edge that has no border.
- The same page shows the rest: a form in a grid whose labels are as wide as the widest, tiles that span two columns and two rows, a list whose section header stays at the top while it scrolls and leaves with its section, a list of tasks where a click removes a task that fades and then lets the others close up, and all text in slate, the theme's foreground, not black. These have no code of their own per platform.
- E7 has no code of its own per platform. Its tests move the tree's clock, and two of them run a model of the winit loop (`app/winit_shell/frame_pacing_tests.rs`). It was looked at in a window on Windows with `examples/presence.rs`: `cargo run -p lurq --example presence --features winit,wgpu,screenshot -- --capture <dir>` runs a script that opens and closes two menus and a dialog on 0.4 s animations, saves a frame at each step and writes `passes.txt` with the layout passes of each stretch. On Windows the frames showed each layer half faded on its way in and out, one menu leaving under the next entering, the frame after a layer left equal pixel for pixel to the page with nothing open, 26 to 28 passes per animation and none in 1.5 s with every layer gone. The first animation after the window opened took 13 passes, which was not looked into. On macOS run the same command and compare; without `--capture` the window stays open to be clicked, which is how to check what the script cannot: a click where a layer is leaving reaches the page (the counter on the page goes up). The script's window has to be on screen and not covered: a window the system counts as hidden draws no frames and its captures come out one pixel large. On iOS there is no example: give a popup and a dialog of `crates/mobile_demo` the animations, and also close a dialog that holds a focused text input: the keyboard should go down as the dialog begins to leave, since the focus is let go then.

**By hand, in the demo.** `crates/demo` has five pages for what Phase 0 built, at the end of its side bar: Borders, Grid, Sticky, Presence and Style. Each has controls for what it shows and says what to try. `cargo run -p demo`, or `cargo run -p demo -- --route /presence` to open on a page; with `--features mcp` the app can be driven through lurq's MCP server, which is how the pages were gone through on Windows at a display scale of 1.5: clicks, the wheel, Tab, Escape and typing, with a frame captured after each. All of it behaved as its tests say. Two things showed that are not in the tests:

- **An outline was painted with its element**, so the element painted next covered it where they touch: of two buttons side by side with no gap, the ring of the first was cut on the side of the second (the Style page, "At the edge"). Fixed: see E4 in 13.2.
- **Text is clipped a pixel wider than its clip** (`expand_text_clip_for_rasterization`, older than this work). At a fractional scale a scroll container's border can cover its first row of pixels only in part, and a line of text scrolled under that edge shows faintly in that one row. Seen on the Sticky page, where it looked like a fault of sticky positioning and is not: it is there without it.

**Linux.** No reader: there is no reduce-motion setting every desktop shares. GNOME's is `org.gnome.desktop.interface enable-animations`, reachable through the settings portal over D-Bus, which lurq does not link. Until someone adds that, motion is reduced there only when the app calls `set_reduced_motion`.

### 13.4 Left to do

- **Phase 0 is built**, with what section 5 had deferred. What is left of it is the check on macOS and iOS in 13.3, which is the last step before Phase 1.
- **Presence, what is left.** `Sheet` and `StackOutlet` still animate out with their own code; they could move to exit animations, and nothing needs them to. A `Select` menu has no exit animation: its menu is built by lurq, so Phase 3 gives it one from the theme.
- **DevTools on the new font** (question 8). The font holds the eight glyphs DevTools needed beyond the first 144. Moving DevTools to it lets `crates/lurq/assets/lucide.ttf` go, which is the one third-party asset the published crate still carries without a notice.
- **`Icon`** is Phase 2: a name, the theme's table from names to a font and a character, the hinted font loaded by default.
- **The Pencil file** has the DevTools card on its Icons pages (152 glyphs, `10e83706`). `bolt` and `external-link` there, and from the first set `settings`, `airplane`, `quote`, `strikethrough`, `bug`, the three hands, `megaphone`, `replace` and `replace-all`, are the glyphs to redraw. `pencil/lurq-devtools.pen` came from another session and was committed as it stood.
- **Not built, and not asked for:** grid lines and areas by name, `minmax` tracks, dense packing and per-child alignment in a grid; dotted borders; a reduce-motion reader on Linux.

## 14. Phase 1: status and handover

For whoever continues. The tokens half of Phase 1 is built on `feat/design-system`, and so is the core of the theme machinery with the first component on it (14.5).

### 14.1 Where each item stands

| Item of section 6 | Status |
| --- | --- |
| Palette roles, light and dark | built: 42 roles, `ThemePalette::light()` and `dark()` |
| Deprecated roles and the migration guide | built: 15 roles deprecated, `docs/…/migration-v0-62.md` |
| Schemes | built, on what the iOS work had added (14.2) |
| The default theme is the design's | built: `Theme::new()` is `ThemeDefinition::neutral()` |
| `Theme::legacy()` | built: `ThemeDefinition::legacy()`, the theme of 0.61 |
| Token tables | built: control size, icon size, density, duration, easing, opacity, blur, focus ring; radii `Xl` and `Full`; the design's shadows |
| Typography | built: the design's type scale and the theme's two families. Line heights and the Markdown heading scale are not (14.3) |
| Pencil: `accent` renamed `highlight` (D3) | not done (14.3) |
| Design parity test | built: `design/tokens/variables.tsv` and `tests/theme/parity` |
| Theme definitions, `Theme::apply`, derived definitions | built for tokens: `ThemeDefinition` |
| Part style and recipes, with the resolved styles kept per theme version | built: `ui::PartStyle`, `ui::Recipe` (14.5) |
| Renderers, for a component and for one part | built: `Theme::set_renderer`, `set_part_renderer` |
| `Theme::bare()`, a recipe replaced whole (D8) | built |
| Sizes, variants and states by name (D11) | built: they are names everywhere |
| Rules by breakpoint (D13) | built: `Recipe::at`, `Selector::breakpoint` |
| The density a theme is at, and rules by density | built: `Theme::set_density`, `Recipe::density`, `Selector::density`. No recipe of the design has such a rule yet |
| Scoped themes | built: `Ctx::in_theme` reaches what a component takes from the theme as it is built, and the element it returns carries the theme, which layout and paint read the roles of its nodes from (14.4) |
| Presentation options | not started: no component has one yet |
| Behavior as public pieces (D10) | started: a view hands a renderer the wired root (`ButtonView::root`); nothing of it is promised yet |
| Recipes for what lurq draws itself: scroll bars, selection, the caret (D12) | not started |
| `Icon`, and the theme's table of icons | built: `ui::Icon`, `ui::icons`, `ThemeIcons`, the font loaded in every app |
| Step 2: `Button` | built to the board's Buttons section (14.6), with its tests and its page of the docs. `Switch` is built (14.8), and the forms batch: `Input`, `Field`, `Textarea`, `Checkbox`, `Radio`, `RadioGroup`, `Slider` (14.9), and the overlays batch: `Tooltip`, `Popover`, `Menu`, `Dialog`, `Toast`, `Select` (14.11), and the navigation batch: `Tabs`, `Accordion`, `Collapsible`, `Breadcrumb`, `Pagination`, `Steps` (14.12), and the display batch: `Badge`, `Tag`, `Chip`, `Avatar`, `AvatarGroup`, `Card`, `Separator`, `Progress`, `RingProgress`, `Spinner`, `LoadingOverlay`, `Skeleton` (14.13), and more inputs: `PasswordInput`, `SearchField`, `InputGroup`, `NumberInput`, `InputOtp`, `CheckboxCard`, `RadioCard`, `Fieldset`, `ErrorSummary`, `Combobox`, `MultiSelect`, `TokenInput`, `Mentions` (14.14), and feedback with more overlays: `Alert`, `Banner`, `Callout`, `Kbd`, `InlineCode`, `EmptyState`, `Outcome`, `MessageBox`, `Popconfirm`, `ProgressDialog`, `Sheet`, `Drawer`, `HoverCard`, `Toggletip`, `TeachingTip`, `Tour`, `CommandPalette`, `ContextMenu` (14.15), and the structure of a window: `Sidebar`, `Menubar`, `Toolbar`, `ActionBar`, `TreeView`, `Splitter` (14.16) |
| The button family | built: `Toggle`, `ButtonGroup`, `ToggleGroup`, `SegmentedControl`, `Link`, `Fab`, and a button with a description (14.7) |
| The gallery | its shell is built, with a title bar of its own, and Overview, Tokens and Icons (15.5) |

### 14.2 What was decided while building

- **The design is the default now** (question 6). An app that never set a theme changes colors, radii and text size on upgrade, and is dark on a system in its dark look. `app.theme().apply(ThemeDefinition::legacy())` is the one line that keeps the look of 0.61.
- **A deprecated role keeps a color of its own.** The plan had deprecated roles resolve to their replacements. Two pairs of roles became one (`TextSecondary` and `TextMuted`, `SurfaceBase` and `SurfaceInput`) and `TextPrimary` was not the color of unnamed text, so one color per replacement could not hold the 0.61 look. Each deprecated role has its own field; in the design's palettes it has the color of its replacement, and in `ThemePalette::legacy()` the color it had.
- **Setting a role sets the deprecated roles it replaces.** `Select`, `FormTheme`, text marks and an unstyled checkbox or slider still read the deprecated roles, so that the legacy theme looks exactly as 0.61 did and an app that still sets `Accent` still colors them. A theme set through the new roles reaches them through this. They move to the new roles when they are rebuilt on recipes (D7), and the propagation goes when the deprecated roles do.
- **Schemes are what the iOS work built**, not the plan's names. There is `Appearance { Light, Dark }`, `Theme::set_palettes`, `follows_system` and `apply_system`; the plan's `Appearance::System` is `Theme::set_preferred_appearance(None)`, added here, which chooses the appearance and leaves the system's text size to the theme. No `ColorScheme` type was added.
- **A color set once holds in both appearances.** `set_palette_color` writes the light palette and the dark one, and `set_palette` writes the palette of the appearance the theme is in. Before, on a theme with two palettes, either was lost at the next change of appearance.
- **A window with no background is cleared to `Background`**, so an app that sets no colors is dark when the theme is.
- **`ThemeDefinition` is `#[non_exhaustive]` plain data** with public fields: an app's theme is `neutral()` or `legacy()` with fields changed, and recipes and renderers become fields of it in the machinery half. `Theme::apply` leaves what a person or the system set.
- **The token tables are read with getters**, `theme.duration_value(MotionDuration::Fast)`, and are not yet accepted by the node API as radii and spacings are: a transition still takes a `Duration`. Recipes resolve them when a component renders.
- **`ThemeFocusRing::outline()`** gives the ring as an `Outline` for `focus_visible`.
- **Fonts.** `ThemeTypography` has `sans` and `mono_family`; `set_sans` rewrites every style that was in the old family. The design is drawn in Inter and JetBrains Mono, and the theme names neither: the defaults are the system's font and the generic `monospace` (D4).
- **Not the theme's:** the board's `sheet` color is the backdrop of its own sheets, and its `z-*` ladder has nothing to map to, since lurq has no z-index. The parity test names both.
- **The demo keeps the legacy theme**, which its pages are styled for. Its Theme page applies either definition and draws every role and scale from the theme as it is.

### 14.3 Left to do in the tokens half

- **The Pencil file still calls the hover fill `accent`** (77 fills and 4 uses of `accent-foreground`). The parity test maps the two names to `Highlight`. The rename is a change to the board, which was open from another checkout while this was built.
- **Line heights.** The design names none, except 1.5 for a paragraph. Every typography style keeps lurq's 1.2.
- **Description and caption are not muted by their style.** A `TextStyle` holds a concrete color, not a palette role, so a typography style cannot follow the appearance. They name no color, and what uses them gives `MutedForeground`. A part style in the machinery half carries the foreground apart from the type.
- **The Markdown heading scale** and `ThemeMarkdown` on roles: with Markdown on roles in Phase 5, or sooner if the machinery needs it.
- **`variables.tsv` is written by hand** from `GetVariables()`. `scripts/build-icon-font.py` already talks to Pencil; a script that exports the variables the same way would keep the file from going stale.
- **macOS and iOS.** Nothing here has code of its own per platform, but the default theme now follows the appearance each shell reports: look at a window in the system's light and dark look on a Mac, and at `crates/mobile_demo`, whose dark palette is now built on the design's dark one.

### 14.4 Next

- **A theme for a subtree, down to the paint: built.** `Ctx::in_theme` was called built when it was half of it: a component built in the scope resolved its recipe, its sizes, its type and its icons in the scoped theme, and the roles left on its nodes were resolved by the layout engine with the values of the window's theme, so a dark side bar in a light window was light. Now a node carries the theme of its scope (`Node::theme_scope`, set by `Ctx::in_theme` on the element it returns, or by `.theme(&theme)` on any element), and `layout_engine/theme_scope.rs` puts that theme's values (palette, border sizes, spacing, radii, caret, scroll bar, typography, shadows, text marks) in the place of the window's while the node and what is in it are laid out and painted, and the window's back after it. The engine remembers the version of each such theme it laid out with: a change of one is a change of theme to the runtime, which lays the window out again. The transition engine enters the scope too, so a state fades between the colors of the subtree's theme. A layer drawn over the tree (the content of a popup, a modal, the menu of a select) is outside the element that carries the theme: the walk that indexes the layers after a layout knows the theme of the subtree it is in and gives it to what the layer shows. Found by the test of a popup in a subtree: `layout_flat_box` held the theme's spacing borrowed while it laid out its children, and a subtree with a theme under a box with a size or padding of its own panicked on the swap; the spacing is read and let go now, and a test lays a subtree with a theme out under a sized and padded box, a wrapping row, a stack and a scroll.
- **The rest of step 2**: `Tabs` (several parts and a selected state) and `MessageBox` (a composite with two buttons in places), each with its page in the gallery. Then the iOS-looking skin and the made-up brand of section 7, on public API alone. The recipe, slot and renderer API is frozen only after that.
- **Presentation options**, with the first component that has one.
- **What lurq draws itself** on recipes: scroll bars, selection, the caret (D12), and the test that fails on a literal color, size or duration in `lurq::ui`.

### 14.5 The theme machinery as built

- **A component is a builder, and takes the theme of what renders it.** `ui::Button::new("Save")` goes where an element goes, with no context passed to it. A render pushes its theme and the breakpoint of its window (`ui::theme`, a stack per thread), and a component built during it reads them: `ui::current_theme()`, `ui::current_breakpoint()`. Reading subscribes what is rendering, so a change of the theme or of the breakpoint builds the component again. Outside a render there is no theme, and a component built there is in a neutral one that follows nothing. This settles question 5: builders.
- **A recipe is a list of rules** (question 2: data, with a builder to write it). A rule is a case and the style of a part in it; a case is any of a variant, a size, a breakpoint, states of the component and one interaction. The rules that hold are sorted by how specific their case is and merged field by field. A compound case, "a primary button under the pointer", is one rule.
- **Names, not enums.** A variant, a size, a state and a part are interned names (`ui::Name`, the theme's `RoleName`). A component's own are enums that read as names, so an app's `"brand"` variant or `"pinned"` state needs nothing declared (D11).
- **Interactions become state styles.** The rules for hovered, pressed and focus-visible are resolved apart from the style at rest and given to the node as its state styles, so nothing renders under the pointer. They carry only what is painted. Disabled, checked, loading and the like are states of the component, resolved when it is built.
- **A veil is an inset shadow.** `PartStyle::veil(color)` lays a color over a fill, under the content: the theme's `StatePressed` darkens any button. It is painted as an inset shadow with a spread larger than any box.
- **Forcing a state** is the component's doing, not the node's: `Button::force(Some(Interaction::Hovered))` merges the hover rules into the style at rest. The gallery's state matrix and the tests use it, and the engine did not change for it.
- **The theme keeps what it resolved**, by component and case, until its version changes. An instance with rules of its own is resolved on its own.
- **A definition says of a component**: a recipe in place of the design's, rules over it, a renderer, renderers for parts. A bare definition is one in which a component that has no recipe set gets none, instead of the design's.
- **A slot is rules handed to an inner component** (`Button::rules(&recipe)`): the composite's recipe for its place, laid over the theme's for the component and under the instance's own. No composite exists yet to use it.
- **A text from a style that keeps the default color takes the foreground above it.** It took the theme's foreground whatever was above. A part's label is such a text, and has to take the color of its root.
- **A theme is at a density.** `Theme::density()` is `Default` unless set, and is part of a definition. A rule for a density holds while the theme is at it, and ranks between a size and a breakpoint. A case takes the density of the theme it is resolved in, so the theme's kept styles need no key for it: a change of density is a change of the theme. What a density does to a control is not decided: the scale is the height of a row, a button has its control sizes, and the design's `Button` has no rule for a density. The first component with rows (a list item, a menu item, a table row) is where the design's recipes start to use it, and `Touch` is how the mobile phase raises controls.
- **Not yet in a part style:** margins, a size per axis for icons, an overlay other than a veil, text decoration. They are added when a component needs them.

### 14.6 Icon, and the whole Button

Said on 2026-11-13: a component is built first, to what the board draws of it, and its page of the gallery comes after. `Button` had been called built with three sizes and no icon; the board's Buttons section draws more, and the plan's note that the control scale has three sizes and the page would show three was wrong.

- **`Icon` is a name.** `Icon::new(icons::SEARCH)` draws the glyph the theme has for the name (`ThemeIcons`, a table from a name to a font's family and a character), so a theme gives a name another drawing or adds names (D5). `ui::icons` is written by `scripts/build-icon-font.py` beside the font: a constant for each glyph and the table the theme starts from; a test holds it to `design/icons/codepoints.json`. A name with no glyph draws nothing and keeps its room. It is not a themed component: it has no recipe, and the table is the theme's hook.
- **The font is in every app.** The glyph engine loads `assets/lurq-icons.ttf` hinted when it is made, from the bytes in the binary, with no copy. It is 92 KB.
- **A spinner is an icon that turns.** `Icon::spin(true)` plays `ui::SPIN`, keyframes of a full turn that every window's animation engine has, in phase, once in a second. Keyframe identifiers from `0xFF00` up are lurq's. It turns under reduced motion too: it says that something is under way, and nothing else does.
- **Five sizes.** `Xs` and `Xl` are not steps of the control scale, which has three: the board draws them 28 and 44 high, the heights of a row at the compact and the comfortable density, and the recipe says so (`Density::Compact`, `Density::Comfortable`). Each size has its padding (10, 12, 16, 20, 24), and the two outer ones a label (12, 15) and an icon (14, 20) of their own; a large button's icon is 18, which is no step of the icon scale.
- **An icon by name, and an element.** `icon(name)` and `trailing_icon(name)` draw an icon of the theme's set in the part `Icon`, whose side the recipe gives per size; `leading(element)` and `trailing(element)` stay for what a name is not enough for.
- **An icon alone, a pill, loading** are states of the recipe (`IconOnly`, `Pill`, `Loading`), so a theme styles them as any other. An icon alone has no padding and the width of its height; a pill has the full radius, and an icon alone in it is a circle. A loading button shows the spinner where its icon was, or before its label, at three quarters of its opacity (the board's value; the opacity scale has no step for it), and is not pressed: its handlers are not attached and it does not submit its form.
- **Pressed is the veil.** The board's pressed row draws the hover fill; the recipe lays `StatePressed` over it, as decided in 14.5, so that a press shows on a button the pointer is already on.
- **The gallery's own icon, icon button and page actions are `Icon` and `Button` now.**

### 14.7 The button family

The board has eleven things that are pressed. Said on 2026-11-13: group them by what they are before building them. They are six components.

| On the board | What it is |
| --- | --- |
| Button, icon button, round | `Button` |
| Close Button, Menu Button | `Button`, as it is: a ghost icon alone at `Xs`; an outline with a trailing chevron |
| Compound Button | `Button` with `description(..)`: a part `Description`, a state `Compound` |
| FAB | `Fab`: a thin component over `Button`, whose recipe is laid over a button's |
| Link | `Link`: text with no box, and what a press does is where it goes |
| Toggle | `Toggle`: a button's behavior and a state `On` |
| Button Group, Split Button | `ButtonGroup`: outline, or filled and a pixel apart; a split button is a filled group of two |
| Toggle Group | `ToggleGroup`: a group's shell around toggles, and which of them are on |
| Segmented Control | `SegmentedControl`: one chosen, with a shell of its own |

- **One scale of sizes.** All of them take `ButtonSize`; a group gives its size to what it holds.
- **A component that holds another hands it rules** (`ResolvedRecipe::rules_for`): the group's recipe has a part `Item`, and what it says there is laid over the theme's recipe for a button's or a toggle's root. This is the slot of 4.3, used for the first time. `Fab` does the same with three parts.
- **One model of choice** for `ToggleGroup` and `SegmentedControl`: a `Signal<T>` (one), a `Signal<Vec<T>>` (any number), or held by the app. The components are not generic: a constructor takes the binding and the choices and keeps what each choice is and what a press of it does.
- **A part style has an underline**: a line along the bottom of a part, which a link has under the pointer. A border was every side.
- **A container clips what is in it** (the engine's default), so a `Fab`'s shadow needs room in what it is put in.

The arrow keys move within the groups since 14.10, and a split button opens a `ui::Menu` since 14.11.

### 14.8 Switch

The board's Switch: a track of 36 by 20, round, `$input` and `$primary` while it is on, a thumb of 16 in `$background` 2 from the edge, at the start or at the end; a label 8 beside it at 14; half the opacity for a disabled one with its label; a ring of 2 outside the track for the focus. One size. (`Mobile/Switch` is another drawing of it, for the skin of section 7.)

- **Parts**: `Root` (the track and the label side by side), `Track` (what is pressed, focused, and has the ring), `Thumb`, `Label`. States `On`, `Disabled`. The pressable is the track and not the root, so that the ring is where the board draws it; a press of the label is a press of the switch.
- **The thumb slides.** A part style has no place in its box to say, so the default renderer moves the thumb by an offset (`SwitchView::travel()`: the track's width less its padding and the thumb's, from the recipe), and the recipe gives the thumb a transition of its offset. Off it has an offset of nothing: a transition runs from one offset to another, not from none. A recipe that gives the track or the thumb no width in pixels gets the thumb laid out at the end, with no slide.
- **A renderer replaces the anatomy**: `view.track()` is the behavior in the track's style, and a theme's renderer puts in it what it likes (the test draws the words ON and OFF in the track). This is the case step 2 named Switch for.
- **Tab did not reach a button of the design system.** Found by the switch's test: lurq's plain `Button` is a stop of Tab only in a form or with a `tab_index`, and `ui::Button` was built on it with neither. The test of the focus ring had clicked a button and pressed Tab: the focus stayed where the click put it, the keyboard made it visible, and the ring was taken for the focus having moved. Now `shared::pressable` gives every control of the design system `tab_index(0)`, `ui::Button` is built on it as the rest are, and the tests press Space after Tab and say which button was pressed.

Not done: the switch has no size but the board's one. A press of the label turns the switch and does not move the focus to its track: a builder has no way to ask for the focus (`Ctx::focus` takes a component's `Ctx`).

### 14.9 Batches, and the forms batch

Said on 2026-11-13: built one at a time the board's components take years. A batch is four to six related components, each built by a session of its own in a worktree of its own from `design/component-playbook.md`, with the board read through Pencil; the sessions compile and run no tests, and leave the text for the docs and the changelog in a handoff file. The batch is then merged here, its ui tests are run once, each page is laid beside the board, and the handoffs are put in place.

The first batch, the board's Inputs and Toggles:

| Component | What it is |
| --- | --- |
| `Input` | The engine's single-line `TextInput` as the box itself, with sizes Sm, Md, Lg, an icon at the start, the judgments Invalid, Warning, Valid, read only (an `on_input` that refuses every edit), disabled |
| `Field` | A label above a control or beside it, a description or an error under it, a required mark; its control is any element |
| `Textarea` | The engine's multiline `TextInput` as the box itself, with the rows it shows at least and at most |
| `Checkbox` | A pressable box of its own with the theme's check and dash, as `Switch` is: the engine's `Checkbox` has no indeterminate state, takes an image for its mark and colors that are not roles, and keeps its own value |
| `Radio`, `RadioGroup` | A round mark with a dot; the group is one choice bound to a `Signal<T>` or held by the app, as `ToggleGroup` is |
| `Slider` | The engine's `Slider` with its track, fill and thumb in the recipe's style |

- **What the merge found.** Of 105 ui tests five failed on the first run. A disabled slider took a press: with nothing under the pointer the engine looked a slider up by its row alone, disabled or not (fixed in the runtime). `Input` drew its focus as a thicker edge, and the engine lays the style of the pointer over the style of the focus, so a focused input under the pointer lost it: the focus is now an outline of 2 inside the box, as `Textarea` had it, which is one look for both. The other three were the tests' own (a float compared exactly, a text looked up whole in a multiline input).
- **The engine's slider paints a ring**: `SliderPartStyle::outline`, so that the thumb's focused style is the theme's focus ring. Its tests, the outline tests and the tests of disabled input were run for it.
- **The focus of a text field is inside its box**; of a checkbox, a radio, a switch and a slider's thumb it is the theme's ring outside. The board draws both.

What the handoffs listed as not done is closed in 14.10, not kept as a list. `Checkbox Card`, `Radio Card`, `Password Input`, `Search Field`, `Number Input`, `Input Group` are other components of the board, for a batch of their own.

### 14.10 What a batch leaves open is closed when it is merged

Said on 2026-11-13, of the list of what the forms batch had not done: that is a bad way to work; when a batch is merged everything open in it is closed. A handoff's "not done" is a list of work for the merge, not a record. What needs the engine gets the engine changed, here or in a session of its own; a defect of the engine that the batch only uncovered goes to a session on master.

Closed for the forms batch:

- **Arrow keys within a group** (the engine): `FocusGroup`, a node attribute. With the focus on a stop of Tab inside the element, an arrow key of the group's axis moves the focus to the next stop inside it (`Tree::focus_group_key`, beside Tab in the key handling), Home and End to the first and the last; `activate` presses what the focus moved to; `one_stop` makes Tab keep one stop of the group (`collect_stops`): the focused one, so that Tab from inside leaves, else the one marked `focus_entry`, else the first. This was the thing 14.7 could not do: a key reaches every node, and now the runtime, which knows where the focus is, acts for the group. `RadioGroup` and `SegmentedControl` are one stop whose choice follows the focus, `ToggleGroup` one stop whose arrows only move, `ButtonGroup` moves with the arrows and keeps every button a stop.
- **`RadioGroup::horizontal`** and **`CheckboxGroup`** (bound to a `Signal<Vec<T>>` or held by the app).
- **A field finishes its control** (`FieldControl`): it names it by its label, judges it (invalid, warning, valid), disables it, and gives it its description, for every control of the design system a form holds; a press of its label focuses the control (`.focuses(&ElementRef)`, a node attribute the runtime honors on a click, since a builder has no `Ctx`). `Textarea` has read only, the sizes and the judgments, and what a box that is typed in or chosen from has is written once (`ui/text_box.rs`), for `Input`, `Textarea` and `Select`.
- **The vertical slider and the range slider**, in the engine (an orientation; two thumbs that do not cross) and in `ui` (`Slider::orientation`, `RangeSlider`), with Home, End and the page keys, a range slider in a form, and both values through the MCP server.
- **The text of a multiline input drawn outside its box after the page scrolls**, reported on the Textarea page: a defect of the engine's drawing, fixed in a session on master (pull request 124); it reaches this branch with the next merge of master.

### 14.11 The overlays batch

Six sessions: `Tooltip`, `Popover`, `Menu` with `MenuItem`, `Dialog`, `Toast` with its host, `Select`. Each is on an engine layer that was there: `Popup` for a tooltip, a popover and a menu, `Modal` for a dialog, the engine's `Select` for a select; a toast's host is a mounted component with the list the app holds (`Toasts`).

- **What the merge found.** Of 161 ui tests 13 failed on the first run, and 7 more of the toast's when it was merged. Most were the tests' own: the headless stage lays a scene out as small as its content, and a layer drawn outside the root's box is not painted, so a scene with a popup, a list or a dialog needs a root with a size of its own (the playbook says so now). One was the component's: a menu's panel was as wide as its window. The toast's keyframes had the identifiers of the built-in fades and overwrote them.
- **Two sessions built the same thing in the engine**: the focus going into an overlay as it opens and back as it closes, as `move_focus` for the popover and `takes_focus` for the menu. The merge kept both to get to green, and the menu's session made them one, `Overlay::takes_focus`, beside the dialog's `Modal::takes_focus`. Sessions of a batch that are likely to need the same change of the engine are told of each other from now on.
- **What the engine got for it**: an overlay and a modal that take the focus, Tab going through an overlay's stops right after its anchor; `FocusGroup::typeahead` and `follow_pointer` for a menu, with a `focus_label` on a node; `ElementRef::focus_visible_within` for a tooltip on a row; a disabled element hearing the pointer come and go, for a tooltip that says why it is disabled; `PartStyle::max_width`, `enter`, `exit` and a padding for each side; the built-in fades; the select's parts open to a recipe.
- **Closed at the merge, not listed**: a menu's arrow keys, its focus as it opens, its submenus with Right and Left, typing a letter; a tooltip on a disabled, stretched element; the Tab order through an open popover; the board's paddings where a part had one number for two sides.

- **A toast that comes while another is shown.** Said on 2026-11-13: the toasts stutter when one comes and there was one already. The toast in front was a child of the list by its id, so a newer one made it leave: it kept its room in the list while its exit animation ran, and for those frames stood a toast's height from its place. The toasts not under the pointer are a pile now, one child of the list whatever is in it, laid one over another: a toast that comes slides in over the one that was in front, which stays drawn where it is until the new one is in (the host remembers it and a timer of the enter's length lets it go), and a front toast that is closed leaves over the next.

Decided against the board, and kept: a disabled menu item is dimmed by the theme's 0.5 as every disabled control is (the board's 0.45); a toast's description is the sans family (the board's Rich toast shows a file name in mono); the scroll bar of a scrolling dialog is the engine's, until scroll bars are on recipes (14.4).

### 14.12 The navigation batch, and what was said of the overlays after their merge

Five sessions: `Tabs` with `Tab`, `Accordion` with `Collapsible`, `Breadcrumb`, `Pagination`, `Steps`. Each brief named the one session that owns each kind of engine change (focus groups and the Tab order: Tabs; a part that opens with its height: Accordion), and no two built the same thing. All 285 ui tests passed at the first run after the merge.

- **What the engine got for it**: a transition of `Height` between a height in pixels and the content's (`AnimatableValue::Fit`), for a panel that opens and closes; `PartStyle::start_line`; `state.selected` in `lurq_inspect`.
- **Closed at the merge, not listed**: the arrow keys of an accordion stopped at the controls inside an open item, and now pass them by (`FocusGroup::none()` around the content, which takes what is in it out of the group around it); the focus was lost when Enter on next took a pagination to its last page and disabled the button, and now goes to the button before it in the group; the overflowing tabs drew no line under the selected tab, which the bar's own line was painted over, and the bar's box is now under what it holds.
- **Decided against the board, and kept**: an accordion's open item has the `chevron-up` glyph and no turning of the down one; a pagination's default is one sibling on each side (`1 2 3 4 5 … 12`), the board's row is `siblings(0)`; previous is disabled on the first page; steps are padded 4 for their ring, so a row is 4 longer at each end than the board's.

Said of the toasts and the dialog on 2026-11-14, and fixed on this branch:

- **Toasts behind the front one appeared oddly under the pointer and jumped when it left**: laid out one over another they were a column, and stacked a stack, two layouts with nothing between them. They are one pile now, each toast in its place by its id, those behind moved out by an offset with a transition and fading as they go back, so no shadow is left to vanish.
- **A toast's text did not fit beside an action**: a row lays a text out as wide as it is. The title and the description are in a column of the room they have, and wrap.
- **A toast at the top and a dialog were under the gallery's own title bar**, and a scrollable dialog with little in it was as high as the window. A modal over the whole window starts under the title bar a `WindowChrome` draws (the window holds the bar's height, `WindowHandle::title_bar_inset`), and a scrolling body no longer grows.

### 14.13 The display batch

Six sessions: `Badge`; `Tag` with `Chip` and `ChipGroup`; `Avatar` with `AvatarGroup`; `Card` with `Separator`; `Progress` with `RingProgress`; `Spinner` with `LoadingOverlay`, and `Skeleton`. Engine changes had owners again (images: Avatar, which needed none; animating a size or a position: Progress; keyframes by ranges of identifiers), and nothing was built twice. Of 358 ui tests 4 failed at the first run, each sent to the session that owned it: a card pressed as a whole kept Tab from the buttons in it; three of the spinner's and the skeleton's, one of which read a turning glyph as if it stood still and passed or failed by the moment it ran.

- **What the engine got for it**: a width or a height in percent animated as a share of the parent's (`AnimatableValue::Percent`), which the indeterminate progress and the skeleton's sheen run on; a border's dash and gap animatable as one value, for the ring's arc; `BackgroundColor::CurrentForeground`, a fill in the foreground the element inherits; `.offset_by_size`, a move by shares of the element's own size; `.stops_inside(true)`, Tab stopping inside a button after it; `PartStyle::corners`; `state.pressed` in `lurq_inspect`; the focus staying in a focus group when the element it is on leaves the tree.
- **Closed at the merge, not listed**: the focus was lost when a tag was taken away with the keyboard; the dots and bars of a spinner did not take the color of the text around them; the skeleton's sheen crossed by pixels and not by the bone's width; the ring's arc moved at once and its middle was painted, and is two round borders now, the arc one dash of the fill's, with nothing in the middle; the top bar's fill was round at its start.
- **Said of it on 2026-11-14 and fixed**: a badge on a corner wider than what it is on grew over it and was cut by the playground's stage: its middle is on the corner's point now, and the stage cuts nothing off. The skeleton's playground offered the number of lines for a form that has none: a control is there only for what it changes, and a playground of a component that is read counts no events.
- **From master meanwhile**: content is cut by every rounded clip it is under (pull request 125), which the toast's timer and the progress's top bar across a rounded panel showed; a border on some sides of a rounded element blends round the corners in place of thinning out as CSS draws it (its own pull request).

### 14.14 More inputs, and no waiting between batches

Six sessions: `PasswordInput` with `SearchField` and `InputGroup`; `NumberInput`; `InputOtp`; `CheckboxCard` with `RadioCard`, `Fieldset` and `ErrorSummary`; `Combobox` with `MultiSelect`; `TokenInput` with `Mentions`. Each owned one part of the engine's text input or select, and four of them changed the engine beside one another without building anything twice. Of 488 ui tests 11 failed at the first run, in four sessions, which fixed them in one round each; 490 pass.

- **What the engine got for it**: a press that repeats while it is held (`.repeat_press`), since a builder has no timer; a text input that keeps what is half typed (`keep_text_while_focused`); a combobox mode of the engine's select, so that a combobox, a multi select and a select share one list of rows, groups, checks and adornments; tokens in a text input that take their look from a recipe, with an icon that removes, hovered and selected looks, and atomic tokens the caret does not enter; the caret's place for an overlay, resolved at layout; a masked input that copies nothing; `focuses` on an element that takes the focus itself; `Stretch` in a wrapping row; an eye with a line through it in the icon font.
- **Closed at the merge, not listed**: the icon for a password that is shown, drawn into the font from an SVG of the repo's (the board is not edited by a session); the list of mentions standing at its trigger right after a paste.
- **From now on a batch is started as the one before it is merged**, said on 2026-11-14 ("а решта компонентів з борди"): the next batch's sessions are cut from the branch as soon as the batch's code is in it, while its failures are fixed and its pages compared. The app runs a limited number of sessions at once, so a batch may start one short and get its last session when a slot is free.
- **Said of it on 2026-11-14 and fixed**: the token input held one text, so the tokens and what was being typed were the same thing to the app (`on_input("rust wgpu sds")`). It has a list now, `TokenInput::list(Signal<Vec<String>>)`: the items are the tokens, and the draft is apart, with `on_change` for the list and `on_draft` for what is typed. The text form stays for a template, where the text around the tokens is the value too. The input keeps the text the engine edits by the list's signal, as a combobox keeps what is typed.

### 14.15 Feedback and more overlays

Six sessions: `Alert` with `Banner` and `Callout`; `Kbd` with `InlineCode`, `EmptyState` and `Outcome` (the board's Result, named so that it does not clash with `std::result::Result`); `MessageBox` with `Popconfirm` and `ProgressDialog`; `Sheet` with `Drawer`; `HoverCard` with `Toggletip`, `TeachingTip` and `Tour`; `CommandPalette` with `ContextMenu`, which started last, when the app had a slot for it. `MessageBox` is the fourth of the components the plan's second step proves the model on: its buttons are styled through its own recipe, apart from buttons elsewhere.

- **What the merge found.** Of 614 ui tests 11 failed at the first run and three more with the command palette. Five of the first were the tests' own (a scene smaller than the headless window, so that a card had room where the test said it had none; text quads counted as lines); the rest were the components', fixed by their sessions in one round. Three sessions changed the overlay beside one another (`on_dismiss`, `tab_after_anchor`, `at`): their merge kept every side, and the last session read the merged files.
- **What the engine got for it**: `.autofocus`, the element the focus goes to first; `Overlay::on_dismiss`; `Overlay::tab_after_anchor`; an overlay at a point of the window; the engine's sheet dressed by a component and under the window's own title bar; a question mark in the icon font.
- **Closed at the merge, not listed**: the question mark for a message box that asks; a press outside a popconfirm giving the cancel answer; the palette's keys drawn by `ui::Kbd`; a message box's long message, which was cut and now wraps.
- **Said of it on 2026-11-14 and fixed in the engine**: a result's first button ran a color transition when the playground gave it a second one, since the new button stood where the old one did and took its node. Children that nothing names keep their identity now when a sibling is put in or taken out before them (`Places` in `node/node.rs`): no component's actions need keys for it.

### 14.16 The structure of a window

Five sessions at once, which is as many as the app runs: `Sidebar`; `Menubar`; `Toolbar` with `ActionBar`; `TreeView`; `Splitter`. `Navbar` with `NavigationMenu` started when the five were merged, with the first three sessions of the batch after it (`Table`; `ListItem` with `Listbox` and `DataList`; `Calendar` with `DatePicker`), since a session starts only in a turn Ivan began.

- **What the merge found.** Of 731 ui tests 13 failed at the first run, in four of the five sessions; the menubar's all passed, and so did every engine test the sessions wrote without running them (input, layout, drag). Two sessions changed focus groups beside one another (`nested` and `menu_row`) and one function of the typeahead: the merge kept both, and the menubar's call was moved to the tree's form of it.
- **What the engine got for it**: `FocusGroup::nested` with `.focus_level` and `.focus_expanded` (the keys of a tree: Right and Left open, close and go in and out, `*` opens the siblings); `FocusRows`, a focus group over rows a list builds only in part, so that the keys reach rows that are not built; `FocusGroup::menu_row` (an open menu handed over to its neighbour by Left, Right and the pointer, Alt and its letters); a row's drop marker for what does not fit; a drag that holds the pointer with one cursor for all of it, and a size set through a ref without a render; `.inert(bool)`; a description set through a ref; `.interaction_group` with `.within_style`, a part styled by the hover and the focus of an element it is in; `Drawer::scroll(false)`.
- **Closed at the merge, not listed**: the tree of thousands of rows (rows in view only, with every key reaching the rest); the action at a row's end shown under the pointer; what is in a closed pane of a splitter taking the focus; the value tooling reads of a splitter after a drag; the header and the footer of a sidebar scrolling with its items over the page. Each went back to the session that owned its part of the engine, and each session wrote the docs of its own engine part afterwards.
- **Said of it on 2026-11-14 and fixed**: a nested splitter's line stood off its neighbours (the splitter's own failing tests, fixed with them). The gallery's side bar changed color as it moved onto the component, which is on `Muted` as the board's is: the gallery has its own look back as a recipe over the component, and a brief that moves an app's own part onto a component now says what must look as it did.
- **What Ivan found in the gallery while the batch ran, all fixed in the engine or the playground and none in a page**: a button kept no identity when a sibling was put in before it (14.15); the scroll bar could not be seen on a dark theme and selected text was the light theme's blue on every theme, since both were literals of the engine and not the theme's (D12: the thumb is the palette's `Ring`, as the board's Scrollbar is, and the selection its `Selection`); a playground whose scene was wider than its stage was cut, and scrolls now; the choices of a panel were cut where their names did not fit one row, and go on as many rows as they need; the code under a stage could not be selected, and is one text now.
- **Still open of D12**: the scroll bar's size is the engine's (8 wide, 2 from the edge) and the board's is 6 in a lane of 8, 1 from the edge; it goes with the board's Scrollbar as a component.

## 15. The gallery

Asked for on 2026-11-13: `pencil/lurq-design-system-demo.pen` is the demo app, in place of the one there is. It is built from the moment work on components starts, a page at a time, so that each component can be looked at as it lands.

### 15.1 What the board draws

The board has the app's own parts (`Library`), one page built in full in both appearances (`Gallery / Button · light` and `· dark`), and a wireframe of the page with notes (`Wireframe / Component page · guide`). Button is the reference: every other page keeps its zones, order and sizes and swaps the content.

| Zone | Size | What it holds |
| --- | --- | --- |
| Title bar | 36 | The app's own window chrome: its name on the left, the window controls on the right |
| App header | 56 | Brand and version; search across components and props (`Ctrl K`); the appearance (System, Light, Dark) and the density, which apply to every demo on the page; settings |
| Sidebar | 248 | Components in the design system's groups, each with a count; rows of 32; the current page filled with the highlight |
| Page header | | Breadcrumb; the name at 32/600; a paragraph up to 620 wide; the import path, a status badge and the version it is there since; links to the source and to Pencil |
| Playground | card of 660 | Always first. The live component on `Canvas` with a switch that forces a state (default, hover, pressed, focus), its measured size and radius, and a counter of events. Under it tabs: the code that builds it, the events, the layout tree. On the right, 320 wide, a control per prop and a reset |
| Example sections | | One per prop axis (Variants, Sizes, Icons and shapes): a heading at 20/600, a line of guidance, a card with the stage on `Canvas` padded 32 and a footer with the builder call and Show code |
| State matrix | | Rows are variants, columns are states: default, hover, pressed, focus-visible, disabled, loading. A component drops the columns it has not |
| Props table | | Prop, type, default, description, in the order of the playground's panel; a required prop has a badge |
| On this page | 216 | An entry per section, sub-entries indented 24, the current one marked with a 2 px edge in `Primary` |

A prop's control follows its type: an enum of up to five is a segmented control and a longer one a select; `&str` is an input; `bool` a switch; `Option<T>` a select with None; a number a slider or a number input. Spacing: the main column is padded 32, 48 or 64; sections are 48 apart; a heading is 16 above its card; a card has `radius-lg` and a 1 px border; toolbars are 52 tall and footers 40.

The sidebar's groups, 43 pages: Getting started (Overview, Tokens, Icons); Actions (Button, Link, Toggle, Toggle Group, Segmented Control, Button Group, Split Button, FAB); Inputs (Input, Textarea, Select, Combobox, Checkbox, Radio, Switch, Slider, Date Picker); Navigation (Tabs, Breadcrumb, Pagination, Menu, Tree View, Steps); Surfaces and overlays (Card, Dialog, Drawer, Popover, Tooltip, Toast); Data (Table, Data Grid, List, Badge, Avatar, Chart); Desktop (Window Chrome, Toolbar, Command Bar, Property Grid, Settings Card).

### 15.2 What it asks of lurq

- **A state forced without the pointer.** The playground's switch and the state matrix show hover, pressed and focus-visible on a component nobody is pointing at. Nodes have no way to hold a state style on: an engine item of its own, which the recipes need as well to be tested.
- **The gallery is made of what it shows.** Its header, sidebar, playground panel and tables are a segmented control, a select, an input, switches, badges, a breadcrumb, tabs and a table. The shell starts on primitives and on the first components, and each part moves to its component when that lands.
- **A code view.** The Rust that builds the preview, colored, following the props, with Copy. Markdown already colors code; whether the gallery uses it or a view of its own is to settle with the first page.
- **Measured size and layout tree** come from what DevTools and `ElementRef::bounds` already read.
- **Density** is a switch in the header. The theme is at a density (`Theme::set_density`), and a recipe has rules for one (14.5). The design's `Button` has none: the switch moves rows, the gallery's own first.

### 15.3 The board is an example

Said on 2026-11-13: the board shows the shape of the app, and was drawn without regard to the real state of lurq. It is the reference for the layout: the zones, their order and sizes, what a page has. It is not the reference for what a component is. Where its content differs from the design system's board (`pencil/lurq-design-system.pen`), the theme and the code, those hold and the gallery shows them:

- **Sizes, variants and props** of a component are the design system's and the code's. The board's Button has five sizes, Xs 28 to Xl 44; the control scale has three, and the gallery's Button page shows the sizes Button has.
- **Names of tokens** are the theme's. The board fills the current sidebar row with `$accent`; the gallery uses `Highlight`.
- **Versions.** The board says `v0.61.0` and "since 0.61"; the gallery shows the crate's version and each component's own.
- **The list of pages** is what exists. A page appears in the sidebar when its component does; the board's 43 names are the order to keep, not a promise.
- **The demo's engine pages** (layout, scroll, text, canvas, borders, grid, sticky, presence) have no place in the board's sidebar. `crates/demo` stays for trying the engine by hand until the gallery has a place for them or they are dropped on purpose.

### 15.4 Order

The gallery is `crates/gallery`, beside `crates/demo`. Its shell came first, then a page with each component as it lands. The Tokens and Icons pages under Getting started need no component.

### 15.5 What is built

- **The window has no frame of the system's.** The gallery draws its title bar from the theme with `WindowChrome`: 36 high, the icon and name on the left, minimize, maximize and close on the right in icons of lurq's font, a close control that turns to the destructive color with white on it. `WindowControls::hover_foreground` was added for that: a control kept one foreground in every state.
- **The header**: brand and version; the search over the pages, by name and by what a page is about, which `Ctrl K` (`⌘ K` on a Mac) goes to from anywhere; the appearance (System, Light, Dark); the density, which is the theme's and moves the rows of the side bar and of the gallery's menus; and a gear that opens what else a theme can be told: reduced motion, text size, bold text. Narrower than the extra large breakpoint it keeps its controls and drops what names them.
- **The side bar**: the pages in the design system's groups with a count for each; a group with no page is not shown. A row is 32 high at the default density, as the board draws it, and moves with the density by as much as a row of the theme does.
- **The frame of a page** (`doc.rs`): its header (the way to it, name, description, import, status, the version it is in since, and two actions: its source and the design system's board, each opened in what the system opens it with), its sections 48 apart, and the list of them on the right from the large breakpoint up, sticky. The list marks the section being read as the page scrolls, and scrolls to the one that is clicked. The content is 880 wide at most, with 32 above it, 48 beside it (32 in a narrow window) and 64 under it.
- **Pages**: Overview; Tokens, which draws the palette, the type scale, radii, elevations and every scale from the theme as it is; Icons, the 152 icons of lurq's font with their names, at the sizes of the scale and in the foreground of what they are in.
- **Its own parts** (`kit.rs`): the segmented control, the select, the panel of a popup, badges, keys, cards, the stage and footer of a demo card. Each goes when the design system has the component.
- **The shell is measured against the board**: every size, gap, weight and role of the title bar, the header, the side bar, the page's header and the list of sections was read from the board's nodes, and a capture of the gallery was laid beside an export of the board at the same scale. What differs is the typeface: the board is drawn in Inter and JetBrains Mono, lurq bundles no font, and the gallery stays in the system's (decided on 2026-11-13: it carries no font of its own).
- **Surfaces.** Said on 2026-11-13: the light appearance was all white, and one thing was hard to tell from another. The board paints the page, the side bar and every card with `$background`, which in the light palette is the color of `card` too. The gallery uses the roles as the theme means them instead: the page and the side bar are on `Canvas` ("the area behind panels and cards"), and the header, a card, a table and the open page of the side bar are surfaces on it (`Background`, `Card`). The stage of a demo card is the card's own surface; the playground's stage stays a canvas inside its card. No value of the palette changed. In the dark appearance the canvas is the background's color and a card is lighter than both. Then, the same day, the greys of the light palette themselves were made darker, in the theme, in `design/tokens/variables.tsv` and in the board's variables: `canvas` `#E6E6EB` (was `#F0F0F3`), `muted`, `secondary` and `accent` `#D9D9E0` (was `#E4E4E9`), `secondary-hover` `#C8C8D1`, `border` `#BDBDC7` (was `#CDCDD3`), `muted-foreground` `#52525B` (was `#5C5C66`). Muted text is 5.5 to 1 on the muted fill and 6.2 to 1 on the canvas. The demo board (`pencil/lurq-design-system-demo.pen`) has variables of its own, which were left as they were.
- `--size 800x600` opens the window at a size, to see a breakpoint.

- **The Button page**: the playground (a state forced with nothing pointing at the button, its measured size and its radius, a count of its presses, the Rust that builds it, the log of its events and the parts it is built of, each copied to the clipboard, and a control for every prop with the count of those changed and a reset), then Variants, Sizes, the matrix of every variant in every state at a size, Icons & shapes, and the table of props. The copy button of a demo card's footer copies its call.
- **The playground is one module** (`playground.rs`): a page says what is on the stage, the Rust that builds it, the parts it is built of and the controls of its panel, and the rest is the same on every page: the switch of forced states, the measured size, the count and the log of what the component did, the tabs, Copy, the count of changed props and Reset. Said on 2026-11-13: a page of a component has a playground; the five pages below were first built without one.
- **The pages of the structure batch**: Sidebar, Menubar, Toolbar, Action Bar, Tree View (Navigation) and Splitter (Desktop), each with its playground. The gallery's own side bar is `ui::Sidebar`, in the gallery's colors by a recipe over it. A playground's stage scrolls what is larger than it, its code and its log are one text each that is selected and copied, and a panel's choices wrap onto rows.
- **The pages of the feedback batch**: Alert, Banner, Callout, Kbd, Empty State, Result (Data), and Message Box, Popconfirm, Progress Dialog, Sheet, Drawer, Hover Card, Toggletip, Teaching Tip, Command Palette, Context Menu (Surfaces & overlays), each with its playground. The gallery's keys are `ui::Kbd` and its search is `ui::CommandPalette`.
- **The pages of the inputs batch** (group Inputs): Password Input, Search Field, Input Group, Number Input, Input OTP, Checkbox Card, Radio Card, Fieldset, Combobox, Multi Select, Token Input, Mentions, each with its playground.
- **The pages of the display batch**: Badge, Tag (with Chip), Avatar, Separator, Progress, Spinner, Skeleton (Data), and Card (Surfaces & overlays), each with its playground. The gallery's own badges are `ui::Badge` now, and it has the `image` feature for the avatar's pictures.
- **The pages of the navigation batch** (group Navigation): Tabs, Accordion (with Collapsible), Breadcrumb, Pagination, Steps, each with its playground, built by the session that built the component.
- **The pages of the overlays batch**: Select (Inputs); Popover, Menu, Dialog, Tooltip, Toast (Surfaces & overlays), each with its playground, built by the session that built the component. The split button of the Menu page opens a menu. The Select page shows options with an element before and after the label (`SelectOption::leading`, `trailing`), which the engine's select has had since 0.61.
- **The pages of the forms batch** (group Inputs): Input (with Field), Textarea, Checkbox, Radio, Slider, each with its playground and its sections with their code, built by the session that built the component.
- **The Switch page** (group Inputs): the playground, the switch bound to a signal, its states, and the track alone in a row of settings. The switches of every playground's panel are `ui::Switch` now; the gallery's own is gone.
- **The pages of the button family**: Link, Toggle, Toggle Group, Segmented Control and Button Group, each with a playground, what it is bound to a signal of the page, its sizes, its states and the table of what it is given; a split button is a section of Button Group, and a button with a description and a floating action button are a section of Button. The gallery's own segmented control is `SegmentedControl` now: the appearance in the header, the settings, the switches of the playground.

- **The tools of the playground's stage**: a toggle that shows the stage in the appearance the page is not in (the stage in a theme of its own, `Ctx::in_theme`), and a button that opens the component as its props are in a window of its own (`ctx.window_opener()`), to see it at any size of window. The output under the stage (the code, the log of events, the parts) is 150 high whatever it shows and scrolls both ways; the log keeps the last 50 events. Said on 2026-11-13: a sixth event made the card grow.
- **"Show code" on a demo card** (`Frame::demo`): under the stage, the Rust that builds what the stage shows, written beside the stage in the page's source and kept equal to it by hand; Copy copies it. It scrolls sideways where a line is longer than the card.
- **A page in a narrow column.** Found on 2026-11-13 at 1100 by 800: the window is past the large breakpoint, and with the side bar and the list of sections the page's column is 540 wide. The frame measures how wide a section is (`Frame::room`) and the playground how wide its card is, and they lay out by that and not by the window: the actions of the header go under the title, the panel of the playground under its stage. What cannot be narrower than some width (the matrix of states, the row of variants, a table of props) keeps it and scrolls sideways (`kit::fit`): an axis that scrolls has no width to fill, so what is in it is given one. Rows of examples go on to the next line, and the call in a card's footer is cut where its tools begin.
- **The page starts at its top**: the scroll of a page is keyed by its route. Said on 2026-11-13: scrolled down on Button and gone to Link, Link was scrolled down as far.

Not built: the side bar does not fold in a narrow window: it waits for the component that does. Arrow keys do not move between the segments of a control or the toggles of a group: the engine sends a key to every node and has no notion of the focus being inside one. The split button opens nothing: the design system has no menu yet.
