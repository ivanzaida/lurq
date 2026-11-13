# Badge: hand-off

`ui::Badge`, from the board's `Badge` (`WQioP`), `Badge/Dot` (`AfKmS`) and `Counter Badge` (`XD1IY`) in the Badges
frame of Components / light (`Ovt1D`, `UntLO`: the variants and "Sizes & marks") and of Components / dark
(`lWGjQ`), and from `Indicator` (`s0vDd6`) in Data Display's "Marks" (`KBUhC`, `B2R3E`). The Indicator is a dot of
12 on the top end corner of a button of 36: it is built here, as `Badge::on(element)`. The board also uses `Badge`
in its Table, Data Grid, Data List and Stat Card (a status cell, "+12.5%"); those are what the variants are for, and
were read, not built.

Files: `crates/lurq/src/ui/badge.rs`, `crates/lurq/tests/ui/badge.rs`, `crates/gallery/src/pages/badge.rs` (page
"Badge", group Data), and lines in the three `mod.rs` files.

## Changed outside the component's own files

- `crates/gallery/src/kit.rs`: `badge` and `status_badge` are gone, as the kit says its parts go when their
  component lands. Their three uses build a `ui::Badge`: the version in the shell's header
  (`crates/gallery/src/shell.rs`, secondary, its label in the family of code at 11 as before), the status of a
  page's header (`crates/gallery/src/doc.rs`, success) and the count of events of a playground
  (`crates/gallery/src/playground.rs`, `Badge::count(n)` in the secondary variant, called "events").
- `crates/gallery/src/playground.rs`: a `Spec` whose `event` is empty draws no counter of events under the stage
  ("× 0" said nothing of a component that is read and not operated). Every page that had an event still has one.
  The other components of the display batch that are read (Progress, Spinner, Skeleton, Separator, Avatar) can give
  `event: ""` too.

## For `docs/src/content/docs/design-system.md`

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
- On an element (`on`), the badge and the element are one element as large as the element; the round end of the badge is centred 3 in from the element's top end corner, so a dot of 12 is at 27, -3 on a box of 36 (the design's Indicator) and a count of 18 reaches 6 past the corner each way. It has a ring of 2 in `Background` that parts it from what it is on. It reaches past the element, so what holds them gives it room or does not clip (`overflow_visible()`). A count that is not drawn leaves the element alone.
- Parts: `BadgePart::Root` (the badge: its fill, edge and height; a dot alone is its root), `Label`, `Icon`, `Dot` (the dot before the text), `Anchor` (what holds the element and the badge on it) and `Corner` (a square at the element's top end corner whose centre the badge's round end is centred on: its width is all that is read of it). States: `BadgeState::Dot`, `DotOnly`, `Count`, `OnCorner`, `Square`. The variant and the size are the recipe's.
- In the design a badge is 22 high and round, padded 8, its text 12/500 (the caption's type at medium), an icon of 12 and 4 before the text; small is 18, padded 6, at 11 with an icon of 10; large is 26, padded 10, at 13 with an icon of 14. With a dot it is in `Background` with a 1 px edge of `Border` and its text in `Foreground`, the dot 7, 6 before the text, the badge padded 8 before and 10 after. A count is at 600 and at least as wide as it is high. A dot alone is 8 (6 small, 10 large), and 12 on a corner (10 small, 14 large).
- To what reads the tree (`lurq_snapshot` of the MCP server) a badge is a `status` called by its text; a count is called by its number with what it counts ("120 unread messages"), the whole number however it is shown.
- A renderer that draws another badge has `view.root()` (the root with its style and its name to tooling), `view.badge()` (the default badge), `view.text` (what is drawn: "99+"), `view.name()`, `view.shown` (false for a count of 0 not drawn), and `view.place(element, badge)` with `view.corner_offset()` to put it on `view.on`.

## For `CHANGELOG.md`

- `ui::Badge`: a short label of a state or a count in seven variants and three sizes, round or square, with an icon or a dot before its text, a dot alone, a count with a maximum ("99+") that is nothing at 0, and a badge on the top end corner of another element (the design's Indicator).

## Decided

- **A builder, and not a node of the engine**: a badge is a row with its text, which the engine has no component for. The engine's tab bar draws a badge of its own (`components::tabs`, `TabItem::badge`); it is the platform tab bar's and was left as it is.
- **A count is a badge** (`Badge::count(n)`), not a component of its own: it takes the variants, sizes and placement of any badge. It starts in `Destructive` and `Sm`, which is the design's Counter Badge (18 high, padded 6, at 11/600, `Destructive` with `DestructiveForeground`). A count in `Destructive` is solid where a text badge in `Destructive` is muted, as the design draws "Failed" muted and the counter solid; a count of another variant has that variant's colors.
- **The maximum is 99 unless told**, so a count does not grow past three digits by itself; `max` changes it. A count of 0 is nothing (`Element::new()`, or the element it is on) unless `show_zero(true)`.
- **A dot before the text takes the variant's color, and the badge has an edge and no fill**, as the design's Badge/Dot ("Online" with a dot in `Success`, "Away" in `Warning`). The variant is the dot's: `Default` `Primary`, `Secondary` `SecondaryForeground`, `Outline` `Foreground`, a status its strong color.
- **A dot alone is a badge with `dot_only(true)`**, called by its text, in the variant's strong color. The design draws a dot alone only on a corner (12); a dot alone in a row is 8, the size the design's lists have for an unread dot (Notification Row, the mobile tab bar), 6 small and 10 large, and on a corner 12, 10 small and 14 large.
- **`Badge::on(element)`** is the Indicator: the element and the badge in a `Stack` aligned to the top end that does not clip, the badge moved by an offset, so the element keeps its own size and place. The offset is computed so that the badge's round end is centred on the centre of the `Corner` part (6 square: 3 in from the corner), which gives the design's dot at 27, -3 exactly and keeps a count of any width growing inward from the same point. The ring of 2 in `Background` is an outline of the badge, outside its box; the design draws it on the dot, and a count on a corner has it too.
- **The square shape** has the theme's small radius (4). The design's square badge ("v0.60") is also in the family of code: that is the content's, done with `.style(BadgePart::Label, |s| s.font_family(FontFamily::Mono))`, as the gallery shows.
- **An icon** is 12 in the default size (the caption's size), 10 small and 14 large, in the badge's own colors: the design draws no icon in a badge, and its gap of 4 is what the icon stands at.
- **Its type** is the theme's caption (12) at medium, so a theme that changes its caption changes its badges.

## Not done

Nothing. The tests in `crates/lurq/tests/ui/badge.rs` were written and compiled, not run, as the batch's flow has it.

## Differs from the board

- The board draws no badge with an icon, no dot alone off a corner, no count of other variants or sizes, no count past a maximum, and no small or large dot: what lurq has for them is under "Decided".
- A count on a corner has the ring of the background that the board draws only around the Indicator's dot.
- The board's count of "8" is 20 wide (padded 6 around the digit); lurq's is that wide too, and a count is never narrower than it is high, so a "1" is a circle of 18.
