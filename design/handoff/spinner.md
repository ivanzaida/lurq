# Spinner and Skeleton: hand-off

`ui::Spinner` (with `ui::LoadingOverlay`) and `ui::Skeleton`, from the board's `Spinner` (`J5HyLj`), `Loader/Dots`
(`Z30WHM`), `Loader/Bars` (`bIdzJ`), `Loader/Inline` (`QjMSn`) and `Loading Overlay` (`x9mDKD`) in Components / light
(`Ovt1D`) › Loading (`mKt0p`), and `Skeleton/Line` (`ULIGV`), `Circle` (`IEfBf`), `Block` (`U4sbQ`), `Shimmer`
(`y7eafw`), `Paragraph` (`eE4Ya`), `Media Row` (`aiayn`) and `Card` (`Wdbjc`) in › Skeleton (`VK6tf`). The spinner is
used at 16, 20, 24, 28 and 32 across the board (buttons, a page loader, a load-more row, a HUD); the skeleton's line is
used everywhere a screen is loading, at any width and at heights from 10 to 32. The board draws no motion: the dots and
the bars are a moment of a wave, and the shimmer is one frame of a sheen.

Files: `crates/lurq/src/ui/spinner.rs` (the spinner and the loading overlay), `crates/lurq/src/ui/skeleton.rs`,
`crates/lurq/tests/ui/spinner.rs`, `crates/lurq/tests/ui/skeleton.rs`, `crates/gallery/src/pages/spinner.rs` (page
"Spinner", group Data, with the overlay) and `crates/gallery/src/pages/skeleton.rs` (page "Skeleton", group Data, with
the media row and the card composed of the pieces), and lines in the three `mod.rs` files.

## Changed outside the component's own files

- **Keyframes** (`crates/lurq/src/animation/animation.rs`, `AnimationEngine::new`): one loop registers four keyframes in
  phase, after the toast's: `ui::SPINNER_DOTS` `0xFF31` (opacity 1 → 0.3 → 1), `ui::SPINNER_BARS` `0xFF32` (a scale of
  the height 1 → 0.4 → 1 about the middle), `ui::SKELETON_PULSE` `0xFF33` (opacity 1 → 0.5 → 1) and
  `ui::SKELETON_SHIMMER` `0xFF34` (a translation from −200 to 1200 along x). `0xFF35` to `0xFF38` are left free. The
  keyframes are built by `ui::spinner_keyframes()` and `ui::skeleton_keyframes()` (`pub(crate)`, as the toast's are).
  Nothing else of the animation engine changed.
- **The button's spinner is a `ui::Spinner`** (`crates/lurq/src/ui/button.rs`, `ButtonView::spinner`): the same glyph
  (`icons::SPINNER`), the same `SPIN` over `SPIN_PERIOD`, at the side the recipe gives `ButtonPart::Spinner`, in the
  label's color, with `.decorative(true)` so that it is no status of its own inside the button, whose label says what is
  under way. A theme's renderer for `ButtonPart::Spinner` still replaces it. The private `glyph` helper of the button
  lost its `spin` argument, which only the spinner used. What changes: a rule a theme lays over the spinner's recipe
  reaches the button's spinner too (a test says so). The button's own tests are untouched and should pass as they are.
- `crates/lurq/src/ui/mod.rs`: the `mod` lines, the `pub use` of both, and the two `pub(crate) use .. keyframes`.
- `crates/gallery/src/pages/mod.rs`: the two `mod` lines, and the two pages after Steps, as the first of group Data.

## For `docs/src/content/docs/design-system.md`

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
| `variant(name)` | `SpinnerVariant::Current` (the color of the text around it), `Muted`, `Primary` |
| `label(&str)` | What is being done, drawn 8 beside it, and what it is called |
| `label_hidden(bool)` | The label is its name and is not drawn |
| `decorative(bool)` | No status of its own, where what it is in says what is under way: a button that loads |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them |

- To what reads the tree a spinner is a `status`, called by its label, or "Loading" when it has none.
- The ring is the glyph a [button](#button) turns while it loads, and `Button::loading` draws a `Spinner`: a rule a theme lays over the spinner reaches the button's. It turns with `ui::SPIN` once in `SPIN_PERIOD`. The dots fade one after another with `ui::SPINNER_DOTS`, the bars shrink about their middle one after another with `ui::SPINNER_BARS`; all of them play on the animation clock, so every spinner on screen moves as one. A spinner moves whether or not motion is reduced: one that stood still would say nothing.
- The ring is drawn in the color of the text around it, so a spinner in a button is the color of its label. The dots and the bars are fills, which take no color from the text: they are in `Foreground` unless a variant says otherwise.
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
- A skeleton pulses with `ui::SKELETON_PULSE`, from whole to half and back in `SKELETON_PULSE_PERIOD`; one that shimmers has a sheen 200 wide cross it with `ui::SKELETON_SHIMMER` in `SKELETON_SHIMMER_PERIOD`, from before its start to `SKELETON_SHIMMER_TRAVEL` (1200) past it. Both play on the animation clock, so every skeleton on screen pulses as one, and the sheen crosses all of them at once. Where motion is reduced a skeleton stands still.
- Parts: `SkeletonPart::Root` (the box of a shape, or the column of a paragraph's lines), `Bone` (a shape), `Sheen` (its width, and the fill of its middle). The shapes are the variants of `SkeletonShape`: `Line`, `Circle`, `Block`, `Paragraph`.
- In the design every shape is in `Muted`; a line is 12 high and `radius-sm`, a circle `radius-full`, a block 200 by 112 and `radius-lg`; a paragraph's lines are 10 apart, its last two 92 and 60 hundredths of its width. The sheen runs from the shape's fill to `SecondaryHover` and back.
- The board's media row and card are built from these and are no components of their own: the gallery's Skeleton page has both, with their code.

## For `CHANGELOG.md`

- `ui::Spinner` and `ui::LoadingOverlay`: a ring, dots or bars that say something is being done, at three sizes, in the color of the text around it, with a label beside it, a status to tooling; and a veil with a spinner over a part of the page while it loads, which keeps it from the pointer and from Tab. `Button::loading` turns a `Spinner`.
- `ui::Skeleton`: a line, a circle, a block and a paragraph in the shape of what is loading, as wide as the room they have or as the app sizes them, pulsing or with a shimmer and still where motion is reduced, with `Skeleton::status` to say once what a group of them stands for.

## Decided

- **No mounted component.** Neither keeps anything: whether the overlay is loading is the app's (`new(loading, ..)`),
  and every motion is a keyframe animation on the animation clock, so builders do.
- **The kind is a state of the recipe** (`SpinnerState::Dots`, `Bars`), not its variant: the variant is the color, as
  the brief names it, and a rule for one kind at one size is `Selector::new().size(..).state(..)`.
- **The default size is `Sm`, 16**: the board's reusable spinner is 16 and so is every use of it beside text and in
  the overlay; 24 and 32 are `Md` and `Lg`. The dots and the bars, which the board draws at one size, grow with the
  ring's (×1.5 and ×2): dots of 12 and 16, bars 4.5 and 6 wide, 30 and 40 high.
- **The dots' and the bars' rest** is the board's moment (opacities 1, 0.6, 0.3; heights 8, 18, 12, 20, 10 of 20): it is
  what a run that has not started yet shows (each dot and bar starts a little after the one before it, by its `delay`),
  and the share of the height each bar is scaled about. The shares and the opacities are constants of the renderer, as
  the button's loading opacity is; the sizes are in the recipe.
- **The bars shrink by a scale**, not by their height, so nothing is laid out again on each frame.
- **In phase.** The four keyframes are registered with `register_keyframes_in_phase`, as `SPIN` is: spinners and
  skeletons that appear at different moments move together, and a page of skeletons pulses as one.
- **Spinners move where motion is reduced**, as the button's always did; skeletons stand still (`theme.reduced_motion()`
  at build, which every theme change renders again).
- **The overlay is a grid of one cell** (an `Auto` column and row) with the content and the veil both in it, stretched
  to it: the only box in the engine that is as large as the largest of what it holds and gives the others that size, so
  the veil covers content of any size. A `Stack` sizes a child of `Pct(100)` by what its parent offers, not by its own
  size. **The cell is as large as the content**, whatever room the overlay is offered: an overlay around a card covers
  the card and nothing beside it, and content that fills a column makes the cell, and the veil, as wide as the column.
  A parent that stretches the overlay (a column with `align_items(Stretch)`) stretches the root's box, not the cell, so
  the veil still covers the content alone. The root has `presence()`, so the veil (keyed `veil`) stays while it fades
  out.
- **The veil is one node**: the spinner in its middle, filled with the scrim's color at the scrim's opacity
  (`LoadingOverlayPart::Scrim`'s `Background` at 0.85, made one color by the renderer). It is not a scrim node with an
  opacity of its own, because a box of `Pct(100)` or a grid of `Fr` inside the veil is as wide as the room the root
  offers when the root measures its cell, which made the overlay fill its row; nor an opacity of the veil, which its
  fade from 0 to 1 would undo when it ends. The content is held in a column that is `disabled` while loading, which
  takes the pointer and Tab from all of it; it is not dimmed by that, the veil does.
- **The skeleton's sizes**: the app's `width` and `height` are a layer over the shape's in the recipe and under the
  app's `style`, so `Skeleton::block(Length::Fill, 132.0)` fills, and `style("bone", ..)` still wins. The board's card
  draws its button as a line 88 by 32 with `radius-md`; the gallery's card builds it as a block with that radius.
- **The sheen's gradient** is built by the renderer from the fills the recipe gives `Bone` and `Sheen`
  (`Muted` → `SecondaryHover` → `Muted`, the board's): `PartStyle` has no gradient, and no color is named in the
  renderer.
- **Nothing to tooling.** A skeleton is described with nothing: `lurq_read_tree` drops a node with no id, class, text,
  attribute or handler, which is what hides it. Marking it (with a role of `presentation`, say) would have made it show.

## Not done

- **The dots and the bars do not take the color of the text around them**, as the ring does: they are fills, and a fill
  names its color. The default variant paints them in `Foreground`, the board's. The smallest change: a
  `BackgroundColor` that is the foreground the node inherits (`BackgroundColor::CurrentForeground`, resolved where the
  quad is painted, as text resolves its inherited color); the recipe's `Dot` and `Bar` then take it in the `Current`
  variant. Paint is no session's in this batch.
- **The sheen crosses a skeleton by pixels, not by its width**: it travels from −200 to 1200 in every skeleton, so one
  narrower than that waits while it crosses the rest, and one wider than 1200 is not crossed to its end. The board's
  shimmer is a sheen as wide as the line. The smallest change: a translation of a keyframe in shares of the node's own
  width (a `Decomposed` translation in percent, or an `AnimatableProperty` for it), which the Progress session owns for
  the indeterminate bar; `SKELETON_SHIMMER` then goes from −100% to 100% and the sheen is as wide as the bone.

## Differs from the board

- **Motion** is not on the board: the turn of the ring is the button's, the dots and the bars move as a wave from their
  drawn moment, a skeleton pulses by default and shimmers when asked.
- **Sizes of the dots and the bars** past the board's one are lurq's (×1.5, ×2).
- **The overlay's content is the app's**: the board's overlay is a card of 220 by 120 with four lines under the veil;
  `LoadingOverlay` is as large as what it is given and has no fill, edge or radius of its own. The gallery builds the
  board's card under it, and rounds the overlay's root with the card.
- **A paragraph's lines** are shares of its width (all of it, then 92 and 60 hundredths for the last two) where the board
  gives 300, 300, 276 and 180 in a paragraph of 300: the same at 300, and in proportion at any other width.
- **The board's page loader, top bar, linear and ring progress** (`Loader/Page`, `Loader/Top Bar`, `g7K92c`, the
  progress loaders) and `Skeleton/Table` (`S1pEV`) are another session's or a later batch's, and not built here.
