## Changed outside the component's own files

- `crates/lurq/src/ui/mod.rs`: the line `pub use media_time::{..}` exports `MediaTimePart`, `MediaTimeRenderer`, `MediaTimeSize`, `MediaTimeState` and `MediaTimeView` too.

## For `docs/src/content/docs/design-system.md`

## Media Time

The time a player reads out: how far it is, and how long the whole is.

```rust
use lurq::ui::{MediaTime, MediaTimeSize};

MediaTime::new(position, Some(duration));

// Over a video, turning between elapsed and remaining on a press.
MediaTime::new(position, Some(duration))
  .on_media(true)
  .remaining(left.get())
  .on_click(move |_| left.set(!left.get()));

// A compact player's; and a stream that is live.
MediaTime::new(position, Some(duration)).size(MediaTimeSize::Sm);
MediaTime::new(0.0, None).live(true);
```

| Builder | |
| --- | --- |
| `new(position, Option<f64>)` | The time at `position` seconds of a whole of that many seconds; `None` while the length is not known: it reads `-:-- / -:--` |
| `remaining(bool)` | It reads what is left, with a minus before it, alone |
| `on_media(bool)` | It stands over a picture: white, the "/" and the whole `#FFFFFF99`, the same in a light and a dark theme |
| `live(bool)`, `live_text(&str)` | A live stream: a badge, solid in `Destructive`, reads "LIVE" (or the text given) in place of the time |
| `size(MediaTimeSize)` | `Sm` (11, a compact player's and an audio player's), `Md` (12), `Lg` (13, a phone's player held across) |
| `on_click(handler)` | A press: a player turns the reading between elapsed and remaining. Given it, the time is a button, a stop of Tab with the ring of the focus, which Enter and Space press. Each call adds |
| `label(&str)` | What it is called, to what reads the tree: what it reads unless told ("1:24 of 4:10", "2:46 left") |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `force` shows only on a time that is pressed |

- From an hour up both readings have hours (`1:02:45 / 2:10:00`). `media_time_text(seconds, hours)` is a reading alone as a string, `media_rate_text(rate)` a speed as a menu names it, and `MEDIA_RATES` the speeds a player offers.
- Parts: `MediaTimePart::Root` (the row, or the button), `Elapsed`, `Divider`, `Total`, `Live` (what the recipe lays over the badge's root). States: `MediaTimeState::OnMedia`, `Remaining`, `Unknown`, `Live`, `Pressable`. Sizes: `MediaTimeSize::Sm`, `Md`, `Lg`.
- In the design (Media/Time) it is the mono type at 12, 4 between the readings, the elapsed time in `Foreground` and the "/" and the whole in `MutedForeground`; not loaded the elapsed `-:--` is muted too. The mono type keeps it from jumping as it counts.

## For `CHANGELOG.md`

- `ui::MediaTime` is built to the board: the mono readings with their roles, over a picture in white, sizes of 11, 12 and 13 (`MediaTimeSize`), a live badge (`live`, `live_text`), and a button with the ring of the focus where it is given `on_click`.

## Decided

- The board's live badge is a `Badge` with a solid `Destructive` fill and `DestructiveForeground` text, which `BadgeVariant::Destructive` is not (a muted fill); the time's recipe has a part `Live` that it lays over the badge's root with `rules_for`.
- A time given `on_click` is a button (a stop of Tab, a radius sm for the ring of the focus, the pointer's cursor); one given none is a plain row and no stop. The board draws the press ("remaining · on click") but no focus of it.
- Not loaded, the board's elapsed `-:--` is muted on a surface; over a picture it is `#FFFFFF99` as the whole is.
- A time over a picture that is not loaded or live keeps the badge themed (decision 2 of the design: the system's own opaque surfaces stay themed).

## Not done

## Differs from the board
