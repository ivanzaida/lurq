# Decisions

What was decided about lurq, by whom and why, newest first. A decision is written here the day it is taken, before
the work on it starts, so that the work, its review and a later session read the same thing. The design system's
first decisions (D1–D13: where components live, palette roles, recipes, breakpoints) are in
[design-system-plan.md](design-system-plan.md#3-decisions) and are not repeated; a decision here that changes one of
them says so. Decisions about phones and tablets are in [mobile-decisions.md](mobile-decisions.md).

Each entry: **the question**, **the decision**, **why**, and its **state** (decided, in work on a branch, built,
dropped). When a decision is replaced, the old entry stays and says what replaced it.

## 2026-11-17

### Link Preview works from the link alone; every field overrides

- **Question.** Where does a link preview's site, title, description and picture come from?
- **Decision (Ivan).** `ui::LinkPreview::new(url)` fetches what the page says about itself and shows it;
  `.title`, `.description`, `.site`, `.image`, `.icon`, `.video` each override the fetched field; when every field it
  shows is given, nothing is fetched. The fetcher is lurq's own, behind `http` (the start of the page; `<title>`,
  OpenGraph, Twitter Card, the icon; a small scanner of the `<head>`, no HTML-parser crate; a cache by URL that an app
  can replace). Without `http` it shows what it was given and the address.
- **With it.** Read-only styled text holds inline elements in its line (the inline form "in a sentence"), built on
  the inline objects of the text input; `open_url` in the engine, by each platform's own call, which the preview uses
  unless `on_open` is given.
- **State.** In work on `claude/link-preview`.

### Cargo features

- **Question.** 28 features: five pull no dependency, pictures spread over six, two localization systems, the `ui`
  charts behind `canvas`, `default = []`.
- **Decision (Ivan).** `form`, `router`, `query` stay features and go into `default`. `i18n` goes: `I18n` (an app's
  keyed translations) is always built, beside `core::Locale` (lurq's own words). `default` is the desktop set:
  `winit`, `wgpu`, `clipboard`, `file_dialog`, `image`, `canvas`, `form`, `router`, `query`. Two features for
  pictures and drawing: `image` (codecs) and `canvas` (drawing, with `svg` folded in); `raster` and `render`
  internal; `screenshot` uses `image`; one `png`. `resources` goes. `devtools` and `perf_profile` stay, their
  call-site cfgs behind macros. The browser keeps an explicit list. Tests: seven runs (A–G) in place of five feature
  combinations.
- **State.** In work in the session "Clean up lurq's Cargo features" (`feature-cleanup`); first merges the 0.62.0
  release from `master`.

### Editing commands in one place

- **Question.** Ctrl+Z on the signature pad did nothing after a stroke, Ctrl+Shift+Z did nothing, and nothing worked
  in a Ukrainian layout; every component matched keys of its own.
- **Decision (Ivan: "undo & redo in one place", then "undo, redo, copy, paste").** `app::events::EditCommand`
  (`Undo`, `Redo`, `Cut`, `Copy`, `Paste`, `SelectAll`) and `KeyboardEvent::edit_command()` are the only code that
  knows those keys: Ctrl (Cmd on Apple), by the Latin letter or by the key's place in any layout. A focused text
  input does it first, then `on_undo` / `on_redo` / `on_select_all` / `on_copy` / `on_paste` from the focused element
  outward, a handler with nothing to do passing it on. `Ctx::edit`, `Ctx::edit_runner`, `WindowHandle::edit` run the
  same from a menu or a toolbar. A press that begins a drag focuses what the element `focuses`.
- **State.** Built, on `feat/design-system` (`77ffe27c`).

### The old `lurq::components` meet the design system

- **Question.** Plan decision D7 (form controls on the new components, `FormTheme` deprecated) was never done, and
  the old styled components live on beside their `ui` twins.
- **Decision (Ivan: "давай роби").** The form controls are rebuilt on `ui` and `FormTheme` deprecated. Every old
  type is an engine primitive (stays, its doc names the `ui` component an app wants), a styled duplicate (deprecated
  in favour of `ui::X`, as `components::Image` is for `ui::Picture`), or styled with hard-coded colors (`new()` /
  `dark()` become recipes). Each break in `migration-v0-63.md`.
- **State.** The old styled components: built (`claude/legacy-styled`, merged 2026-11-17: `Checkbox` and `ActivityIndicator` deprecated for `ui::Checkbox` and `ui::Spinner`; the phone pieces, `SelectStyle`, the stack's shades and the unstyled checkbox and slider nodes on recipes, the legacy theme keeping 0.62's look). The phone pieces as real `ui` components (Tab Bar, App Bar, Large Title, Bottom Sheet, Swipe Row, Pull To Refresh, as the board draws them) are not built. The form controls: in work on `claude/legacy-form`.

### Words lurq writes are in the locale

- **Decision (Ivan, `AGENTS.md`).** Every word a person sees or a tool reads is a field of `core::Locale`, in six
  languages; a component takes it from `ui::current_locale()`; a single-word method on the component wins.
- **State.** Built: `claude/ds-locale-words` and the thirteenth batch's words, merged.

## 2026-11-16

### Every family on the board is built

- **Decision (Ivan: "давай ці 6 доробимо").** The six families held for a request (Signature Pad, Angle Slider, QR
  Code, Watermark, Transfer, Cascader) were built. With them every family the board draws has a type in `lurq::ui`.
- **State.** Built; see the plan's 14.27.
