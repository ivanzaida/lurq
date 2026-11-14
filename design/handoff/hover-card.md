# Hover Card, Toggletip and Teaching Tip: hand-off

`ui::HoverCard`, `ui::Toggletip`, `ui::TeachingTip` and `ui::Tour`, from the board's `Hover Card` (`yMLPp`),
`Toggletip` (`aJfiv`), `Teaching Tip` (`FlXjX`) and `Tour` (`XnR5D`) in Components / light (`Ovt1D`), and the tooltip's
arrow (`dPvGf`). On the board each is drawn once, in Surfaces / Floating (`p1kLyo`), Overlays / Teaching (`PgUzg`) and
Guidance / Popovers (`MOmrd`, `xJNTI`), with its twin in Components / dark; nothing on the board uses them elsewhere.

Files: `crates/lurq/src/ui/hover_card.rs`, `toggletip.rs`, `teaching_tip.rs` (with `Tour`), and `tip.rs`, which the three
share (the side a panel is on, reading where the overlay put it, the arrow with an edge, the frame of a panel and its
arrow); `crates/lurq/tests/ui/hover_card.rs`, `toggletip.rs`, `teaching_tip.rs`; `crates/gallery/src/pages/hover_card.rs`,
`toggletip.rs`, `teaching_tip.rs` (pages "Hover Card", "Toggletip", "Teaching Tip", group Surfaces); this file; and lines
in the three `mod.rs` files.

## Changed outside the component's own files

This session owns an overlay that opens when the pointer rests on its anchor and stays while the pointer is on it; the
engine change is of that kind.

- **Engine, `Overlay::tab_after_anchor(bool)`** (and `Popup::tab_after_anchor`), off by default: Tab goes through the
  overlay's elements right after its anchor's, as with `takes_focus`, but the focus is not moved into it as it opens. As
  it closes with the focus inside, the focus goes back to its anchor (not to what had the focus when it opened, which
  for an overlay that took nothing may be anything). Files:
  - `crates/lurq/src/app/ctx.rs`: the field on `Overlay` and `OverlaySpec`, the builder, `into_spec`, `clone_with_node`.
  - `crates/lurq/src/components/popup.rs`: the builder.
  - `crates/lurq/src/app/runtime.rs`: in `build_overlays_from_layout_index`'s `Overlay` arm an overlay with either flag
    is pushed to `layers.focus_movers` (what Tab walks after the anchor, and what gives the focus back), and only one that
    takes the focus to `layers.focus_takers`; the two calls of `note_focus_returns` pass the layers.
  - `crates/lurq/src/app/runtime/overlay_layers.rs`: `OverlayLayers::focus_takers`; `opened_overlay_focus` moves the
    focus into the takers alone; `note_focus_returns(&OverlayLayers, ..)` remembers what had the focus only for a taker.
  Nothing an overlay that takes the focus does changed.
- `crates/lurq/src/ui/link.rs`: `Link::rules(&Recipe)`, as the other components have it, so that the toggletip gives its
  link a color under the pointer too (`style` sets only the rest).
- `crates/lurq/src/ui/part_style.rs`: `impl From<MotionDuration> for TransitionDuration` and
  `impl From<Duration> for TransitionDuration`, so that `HoverCard::open_delay(MotionDuration::Slower)` and
  `open_delay(Duration::from_millis(700))` both read. If another session of the batch adds the same impls, keep one.

No keyframes were added: the three fade in and out with `ui::FADE_IN` and `ui::FADE_OUT`, as the popover does.
`0xFF61` to `0xFF68` are free.

## For `docs/src/content/docs/design-system.md`

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

## For `CHANGELOG.md`

- `ui::HoverCard`: more about what the pointer rests on in a panel beside it, with media, a title, a description, a row of facts and anything it holds, which opens after the theme's slower duration or at once for the keyboard's focus, stays while the pointer crosses to it, closes a moment after it left both and on Escape, lets Tab go from the element into its controls and on (`Overlay::tab_after_anchor`, new: an overlay that Tab goes through after its anchor without taking the focus), and is a dialog or a tooltip to tooling by whether it has controls.
- `ui::Toggletip`: a small button with an "i" that opens a few words in the primary color, with a link or a button in them, closed by another press, a press outside or Escape, the focus staying on the button and Tab going into the panel; to tooling the button says it is expanded and the panel is a status.
- `ui::TeachingTip` and `ui::Tour`: a panel that points at one thing in the app with an arrow that has its edge, media, a title, a description, a close control, a step count and two buttons, in a plain and a primary look, which does not take the focus unless the app says so and is closed by its close control or Escape and not by a press elsewhere; a tour shows the app's steps one after another with Back, Next and Done.

## Decided

- **Mounted, all three, and the hover card wraps its trigger as the tooltip does**: `HoverCard::new().title(..).mount(ctx, trigger)`, not `HoverCard::new(trigger, content)`, so that a tooltip and a hover card are put on an element the same way. Each keeps what a builder cannot: the hover card its timers and where the pointer is; the toggletip whether it is open; the teaching tip which side the window let it take and what Escape tells. What is open for a teaching tip and which step for a tour stay the app's signals, as the brief has it.
- **The content of a hover card is built from parts, with anything after them**: `title`, `description`, `media`, `fact` give the board's card with the board's type, and `child` adds anything (the controls). A card given only children is a panel of them.
- **Focus: `Overlay::tab_after_anchor`, not `takes_focus`**. `takes_focus` moves the focus into the overlay as it opens, which a hover card opened by Tab on its trigger must not do: the person tabbed to the trigger, not into the card. The new flag gives the rest of what `takes_focus` gives (the overlay's stops right after the anchor's, the focus back to the anchor as it closes) and nothing more. The hover card stays open while the focus is in it by `ElementRef::focus_visible_within()` of its own root, beside its trigger's. The toggletip and the teaching tip that does not take the focus use it too.
- **Toggletip and teaching tip on the engine's `Popup`, as `ui::Popover` is built, not on `ui::Popover` itself**: the popover moves the focus into itself (`takes_focus(true)`, fixed), is a `dialog` called by its title (the toggletip's panel is a `status`), has no arrow and no way to wrap its panel in one, and does not say which side it went to, so its arrow could not turn. Each of these would be a builder of the popover's that only these two would use. The two take from the popover what it is: a `Popup` by the anchor with `FlipThenClamp`, the toggle and `a11y_expanded` on the trigger, the fade in over the base duration and out over the fast one.
- **One arrow for the three, `ui/tip.rs`**: the tooltip's square turned by 45°, half of it shown, in the panel's fill; where the panel has an edge the square has it too, and it reaches over the panel's edge by the edge's width and is drawn after the panel, so the panel is open where the arrow leaves it. The panel and the arrow are a `Stack`, aligned to the side toward the anchor, for that order on every side. Which side the panel is on and how far along its edge the arrow is moved are read back from the panel's rect (`Placed`), as the tooltip reads them: the overlay does not say. The tooltip keeps its own arrow: it is another component's file, and it has no edge.
- **The side is a state, not the variant** (`"top"`, `"bottom"`, `"left"`, `"right"`, by `TooltipSide`'s names): the teaching tip's variant is its look. The room between a panel and its anchor is the root's padding on that side, the pointer's way across for the hover card.
- **The board's Tour is a teaching tip that is a step**: the same panel with a count and two buttons and no media or close control, at other numbers (280, padding 16, title at 500, buttons 28). So `TeachingTipState::Stepped` carries the Tour's numbers, and `Tour` is the small helper that shows the app's `TourStep`s one after another with Back, Next and Done, setting the app's `Signal<Option<usize>>`; it is nothing more than the board's Tour shown in turn.
- **A teaching tip is not closed by a press elsewhere**, only by its close control, Escape or the app: it points at something a person is to try while it shows. Escape closes it through a signal of the layer's own, which the runtime turns off, so that `on_close` is told of Escape as of the close control and not of the app closing it.
- **Delays from the theme's durations**: the hover card opens after `MotionDuration::Slower` (400 ms in the design's theme) and stays `Slow` (240 ms) after the pointer left, a theme that changes its durations changing them; `open_delay` and `close_delay` take a role or a `Duration`.
- **One hover card of the pointer at a time**, as one tooltip: the one before goes at once when the pointer opens another.
- **The toggletip's button**: a ghost button of 20 that is a circle, with an icon of 14 in the muted foreground, and `icons::INFO`; the board draws the panel only.

## Not done

Nothing.

## Differs from the board

- **The toggletip's button, a question mark, and the teaching tip's primary look are not on the board**: the board draws the toggletip's panel, and the teaching tip and the tour in the popover's colors only. The button is a ghost icon button of 20 with `icons::INFO`; the theme's icon set has no question mark, and `Toggletip::icon` takes any name an app adds. The primary look is the panel in `Primary` with all it says in `PrimaryForeground`, as the toggletip is, its media on a white of 10%, the button that goes on `Secondary` and the other `Ghost`.
- **The arrow points at the middle of what it is about**: the board draws the teaching tip's 28 from the panel's start, which is the middle of an anchor 74 wide under a panel placed at the anchor's start. The hover card and the toggletip have no arrow on the board; they take the teaching tip's (18 by 9, with the edge) and the tooltip's (12 by 6), off by default as the board draws them.
- **Where a panel stands from its anchor** is not drawn: the hover card 8, the toggletip 6 (the tooltip's), the teaching tip 4 (the popover's) and its arrow.
- **The hover card's facts are a row**: the board draws one fact, "Joined March 2024"; more stand 12 apart and go on to the next line where they do not fit.
- **The tour has an arrow** where the board's Tour draws none: a step points at what it teaches, as a teaching tip does. `Tour::arrow(false)` draws the board's.
- **The close control is a button of 20** around the board's icon of 16, so that it can be pressed and reached by Tab.
