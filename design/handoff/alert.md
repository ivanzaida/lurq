# Alert, Banner and Callout

## Changed outside the component's own files

- `crates/lurq/src/ui/mod.rs`, `crates/lurq/tests/ui/mod.rs`, `crates/gallery/src/pages/mod.rs`: the modules, their exports and the gallery's three page entries (Alert, Banner, Callout, after Progress at the end of `Group::Data`), nothing else.
- No shared code, no engine change, no keyframes: the banner fades with `ui::FADE_IN` and `ui::FADE_OUT`, which every window has.
- `crates/gallery/src/pages/banner.rs` and `callout.rs` use the alert page's `VARIANTS` (`pages::alert::VARIANTS`, `pub(crate)`), so the three pages name the variants alike.

## For `docs/src/content/docs/design-system.md`

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

## For `CHANGELOG.md`

- `ui::Alert`, a message of the design system in the flow of a page: a title, a description, an icon by its variant (`AlertVariant`: neutral, info, success, warning, destructive) or one of its own, an action, a close control and elements of the app's under its text; an `alert` to tooling for a warning and a destructive one, a `status` for the others.
- `ui::Banner`, a message across the full width of what holds it at the top of a window or a page: its icon, its text on one line where it fits, a link in its variant's color or another action, a close control, its content centred or from its start, and `open` and `sticky`; it fades in as it is shown and out as it is taken away.
- `ui::Callout`, a note set apart inside prose or a form: the icon of its variant, a title, its text and elements of the app's, and with `bar(true)` a document's note with a bar along its start.

## Decided

- **One variant type for the three, with the colors and the icon in one place.** `AlertVariant` has `icon()`, `accent()` and `tint()`; each recipe styles its parts from them in a loop over the variants, so a theme that changes what `Info` means for one changes it by the same rule for all three. The toast's `ToastVariant` stays the toast's: its neutral toast has no icon and its error has the cross in a circle, where the board's destructive alert has the warning sign, so the two icon functions are not one function apart. Merging them would change the toast's icons or the board's alert.
- **Roles.** `alert` for `Warning` and `Destructive`, `status` for `Neutral`, `Info` and `Success` (ARIA's live regions: an `alert` is said at once); `role(&str)` lets the app say otherwise. A callout is a `note` (ARIA's role for a note set apart from the main content).
- **An action is an element**, and the alert's close control is drawn once a handler is given (`on_close`), not by a flag: an alert that cannot be closed is what most pages have, and one that can always does something when it is.
- **The banner's link takes the variant's color** by `Banner::link(Link)`: the banner lays the style of its `Link` part over the link's `Root` (the color) and `Label` (13, semibold) with `Link::style`. `action(element)` takes anything else, which keeps its own color. The link's underline under the pointer is the link's own, in `Foreground`: `Link` takes rules only at rest (`style`), not for the pointer (it has no `rules`).
- **The banner is shown and taken away by `open(bool)`**, and stays in the tree either way: what stands in the page has `presence()` and the bar a key, so the bar's exit animation runs when `open` turns false. A banner the app drops from the tree leaves at once, as any element does.
- **The banner's icon for `Neutral` and `Info` is the megaphone**, as the board draws it; the other variants have their alert icon (a check in a circle, a warning sign).
- **The banner keeps the board's corners** (`radius-md`); across a window's top edge the app takes them off with `style(BannerPart::Panel, |s| s.radius(0.0))`, as the gallery's "Across a window" shows. The board's 640 is the width of its example: the bar is as wide as the room it is given.
- **A centred banner** has its text as wide as it is (a column that shrinks, `flex_full(0, 1)`), so the group is centred on the bar; a long one still shrinks to the room and wraps.
- **A callout's bar is a state, not a variant**: the board's callout (an icon at its start, a rounded box) and its note of a document (a bar of 3 along its start, the icon in the title row) differ in layout as well as style, so `bar(true)` chooses the layout and the recipe's `Bar` state the style. The board's notes use the lightbulb for a tip and the octagon for a caution; a callout keeps the variant's icon unless told (`icon(icons::LIGHTBULB)`), as the gallery and the docs show, since the same success color is a tip in a document and a done thing elsewhere.
- **A neutral callout and banner** are in `Muted` with their icon in `Foreground`; a neutral alert keeps the board's card color and edge. The board draws no neutral banner or callout.
- **A callout's title without a bar** is 13 and semibold in `Foreground`, over the text: the board's callout has no title.
- **Group**: the three are in `Group::Data` in the gallery, as the brief says; the board has the alert under "Feedback & Small Parts", the banner under "Data Display" and the callout under "Typography / Prose".

## Not done

Nothing.

## Differs from the board

- The board's alert is 360 wide, its banner 640 and its callout 380: each is as wide as the room it is given, and the gallery gives them those widths.
- The icons are glyphs of the theme's icon font, of one weight; the board's are strokes of 1.333.
- The banner's close control has no change of color under the pointer: the board draws it in `Foreground`, which the muted close control of an alert turns to under the pointer. It has the focus ring from the keyboard.
- The board's Mobile/Banner and Mobile/Offline Bar were read and not built, as the brief says; the offline bar is what a centred warning banner with `icon(icons::WIFI_OFF)` and no corners gives, in the banner's 13 and regular type where the bar's is 13 and medium in `Warning`.
