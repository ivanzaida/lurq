# Lightbox

`ui::Lightbox`, from the board's `Lightbox` (`Nn4rG`, 420 × 290) in the section `Picture` (`wfriA`, dark `Gg1rp`) of
`Components / light`. Branch `claude/picture`. Its props as they were agreed: `image-design.md` in the sessions'
shared notes ("`ui::Lightbox` and `ui::Gallery`: the props"); how it is built: `picture-code.md` there.

## Changed outside the component's own files

- `crates/lurq/src/ui/mod.rs`, `crates/lurq/tests/ui/mod.rs`, `crates/gallery/src/pages/mod.rs`: the lines of
  `lightbox`.
- The engine: `ZoomScrollProps::scrollbars(bool)` (`components/zoom.rs`), on unless told: the lightbox's
  zoomed picture shows none.
- The engine: `on_wheel` / `off_wheel` on every component, called for a wheel or a trackpad before anything scrolls
  and whether or not anything can (`prevent_default` stops the scroll); `ZoomScrollProps::wheel_zoom(step)` and
  `drag_pan(bool)`, and a scale set from outside keeps the middle of the view. A picture at the size it fits lies in
  a list that cannot move, over which `on_scroll` is not called: the wheel could not bring it closer, in a lightbox
  or in `ZoomScroll` itself. Tests `tests/runtime/wheel_handler.rs`, `tests/components/zoom.rs`; docs
  `styling-events.md`, `ios.md`.
- `ui::Picture`: `in_viewer`, `as_thumb`, `caption_of`, `label_of` (crate-private): the picture as a lightbox shows
  it and as its strip does.

## For `docs/src/content/docs/design-system.md`

## Lightbox

Pictures one at a time, whole, over the window: to look at closely, to bring closer, to turn through.

```rust
use lurq::ui::{Lightbox, Picture};

// open: Signal<bool>. Open while it says so; Escape, a press beside the picture and Close set it to false.
Lightbox::new(open.clone(), photos.iter().map(|photo| Picture::from_file(&photo.path).title(&photo.name)))
  .index(shown.clone())
  .wrap(true)
  .on_download(move |which| save(which));

// One picture needs none of this: Picture::lightbox(true). A grid of them: Gallery.
```

| Builder | |
| --- | --- |
| `new(open, pictures)`, `picture(Picture)`, `pictures(..)` | Open while the signal says so, with the pictures it shows. Each is shown whole (`Contain`), with its `title` and `description` over its lower edge; what else a picture has over it (a bar, a badge, a mark) is not shown |
| `index(Signal<usize>)`, `index_controlled(index, on_change)`, `on_index_change(Fn(usize))` | Which picture is shown, counted from 0. Without either the lightbox keeps it and starts at the first |
| `zoom(Signal<f32>)`, `on_zoom_change(Fn(f64))`, `zoom_max(f32)` | How close the picture is, 1 where it fits; and the closest it is brought, 4 unless set. Turning to another picture fits it again |
| `variant(LightboxVariant)`, `on_share(Fn(usize))`, `menu_item(MenuItem)`, `menu_items(..)`, `share_label`, `more_label` | `Desktop`, or `Phone`: the phone's viewer, which a lightbox is under a finger (`Pointer::Coarse`, `ui::touch_form()`) at any width and density, unless told. On a phone, the Share button with the picture shown, and the menu behind More, which holds Download first where it is given |
| `preload(usize)` | How many pictures each way of the one shown are brought on once its own pixels came, read by the app's loader and decoded, so that an arrow shows the next at once: 1 unless told, 0 for none, round the ends where it wraps |
| `wrap(bool)` | Whether the first picture comes after the last |
| `filmstrip(bool)`, `counter(bool)`, `counter_text(Fn(usize, usize) -> String)` | The strip of small pictures and the counter: both show with more than one picture unless told. The counter says "3 / 12" |
| `on_download(Fn(usize))` | Shows the Download button, and is called with the picture shown |
| `action(element)`, `actions(..)` | Buttons of the app's in the top row, before Download |
| `inline(bool)`, `width(Length)`, `height(Length)` | Stands in the page where it is declared, open or not, in place of over the window: for a gallery, a screenshot, a test, a viewer that is part of a page. Its size there; the width is the most it takes |
| `on_dismiss(Fn())`, `dismiss_on_escape(bool)`, `dismiss_on_veil(bool)`, `close_button(bool)` | Told when a person closes it; and whether Escape, a press beside the picture and a Close button do |
| `label`, `close_label`, `previous_label`, `next_label`, `zoom_in_label`, `zoom_out_label`, `fit_label`, `download_label` | Its words to tooling, the app's locale's (`Locale::navigation` and `common`), in English "Pictures", "Close", "Previous picture", "Next picture", "Zoom in", "Zoom out", "Fit to the window", "Download" |
| `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the panel; its parts are `<id>-close`, `-next`, `-previous`, `-zoom-in`, `-zoom-out`, `-level`, `-download`, `-stage` |

- It is a component wherever it is put, as a [picture](#picture) is: it keeps which picture is shown, how close, and
  where the view is.
- The wheel brings the picture closer about the pointer and a drag moves it; two fingers on a touch screen pinch and
  pan. A double click goes to twice the size it fits and back.
- Keys, while the focus is in it: the left and right arrows turn to the picture before and after, and move a
  picture that is closer than it fits, as the up and down arrows do; `+` and `-` bring it closer and further, `0`
  fits it; Escape closes. Over the window it takes the focus.
- The level says how close the picture is as a share of the size it fits at (100% fitted), and a press of it fits
  the picture again. In a panel too narrow for the whole top row the level is not there.
- The strip shows as many small pictures as it has room for, about the one shown.
- Parts: `LightboxPart::Veil` (what covers the window), `Root` (the panel), `Top`, `Counter`, `Total`, `Control`,
  `ControlIcon`, `Level`, `Stage`, `Step`, `StepIcon`, `Side`, `Caption`, `Title`, `Description`, `Filmstrip`,
  `Thumb`. States: `LightboxState::Inline`, `Current` (the small picture of the one shown).
- In the design it is white on near black (`#09090BF2`) in both themes, since a picture is not themed: a top row of
  52 with the counter in the mono face and buttons of 36, round buttons of 36 on black at 40% over the sides of
  the stage, a caption on a gradient to black at 70%, and a strip of 52 with pictures of 44 by 32, the one shown
  whole and ringed in white, the others at 60%.
- To what reads the tree its buttons are `button`s called by their labels.
- A renderer that draws another lightbox has `view.top()`, `view.stage()`, `view.caption()`, `view.filmstrip()`,
  `view.panel()` (the three with the keys) and `view.dismisser()`.

## For `CHANGELOG.md`

- `ui::Lightbox` is a component of the design system: pictures one at a time over the window or in a page, brought
  closer by the wheel, a pinch, its buttons and the keys, turned through by round buttons, the arrows and a strip of
  small pictures, with a counter, a caption, a download button and the app's own.

## Decided

- The zoom is `ZoomScroll`'s, made to take a mouse, not the lightbox's own: a first version moved the picture by a
  transform because the wheel did not reach a list that cannot move, and that was a flaw of the engine to fix, not
  to go around (Ivan, 2026-11-16).
- The level is a share of the size the picture fits at, and a double click goes to twice that: not the picture's
  own pixels, which a picture from an address does not have before it is there.
- Keys are the lightbox's only while the focus is in it: a key is told to every element of the tree, and a lightbox
  in a page would turn for a key pressed anywhere.
- The strip is a window of the pictures about the one shown: twelve small pictures do not fit 420.
- Where the top row has no room, the level gives way first: the buttons about it, a double click and `0` do what it
  does.

- The small pictures of the strip are decoded small (four times the longer side of their box, both ways),
  not as the picture itself: a strip of a dozen photos held a dozen photos' pixels, and asked for the
  pictures about the one shown before it came.
- Seen by hand through the gallery's MCP server, over the window: the right arrow twice turns to 3 / 8 and
  the left arrow back to 2 / 8; `+` brings the picture to 150%, where the right arrow moves it and does not
  turn; `0` fits it again at 100%; Escape closes it.
- Opened over the window, the stage takes the focus: the picture's room, a `group` called by the picture's label.
  The arrows, `+`, `-`, `0` and Escape work at once, and Tab goes on to the round buttons over it, the strip and
  the top row. Its ring, white and inside its edge, shows where the keyboard gave the focus (the engine's rule:
  the modality of the last input), so a lightbox opened by a click shows none and one opened by Enter shows it.
  Seen in the gallery's window both ways.
- Closer than it fits, the picture shows no scroll bars, as the board's "zoomed to 300%" draws it: it pans by a
  drag and by the arrows. `ZoomScrollProps::scrollbars(false)` is new in the engine for it; bars stay on by
  default.
- Opened again, the picture is fitted, as it is the first time, unless the app keeps how close it is in a signal
  of its own.

- On a phone (`LightboxVariant::Phone`, the board's `Mobile/Picture/Viewer`, `nBrZz`) the lightbox is black and
  the whole screen, clear of what the system draws over its edges (`Ctx::safe_area`): a top row of 44 with Close,
  the counter in its middle, the app's buttons, Share and More, all 44 with glyphs of 24; the picture; its title
  (15 medium) and line (13 at 60% white) 16 in; a dot of 8 for each picture, 8 apart, at most nine about the one
  shown. The pictures are pages of a `ui::Carousel`, so a swipe is the engine's paging scroll; the one shown is in
  `ZoomScroll` (a pinch and a double tap), whose drag pans only once it is closer. A fitted picture dragged down
  more than across follows the finger, the black under it fading, and let go 120 down or thrown at 900 a second
  it closes the lightbox. No zoom buttons, round buttons or strip. Seen in the gallery's window at 390 by 844 and
  in a page at 360 by 640: as the board draws it.
- The form follows the window, not the lightbox's room: over the window the two are one, and a lightbox in a page
  is the desktop's unless told.

## Not done

- Not seen by hand: a pinch on a touch screen. The gallery's MCP server sends no touch, and this machine has
  no touch screen; the pinch is `ZoomScroll`'s, which the engine's touch tests cover.

## Differs from the board

- The board's counter is `Media/Time`; lurq's is two texts of the same style, the total at 60% white.
- The board draws twelve small pictures in the strip of 420; lurq shows the eight that fit, about the one shown.
