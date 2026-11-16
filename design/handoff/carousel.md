# Carousel

`ui::Carousel`, from the board's `Carousel` (`fO6fy`) in `Surfaces → Layout` of `Components / light` (`DzzOe`,
dark `D4794`). Branch `claude/picture`.

## Changed outside the component's own files

- `crates/lurq/src/ui/mod.rs`, `crates/lurq/tests/ui/mod.rs`, `crates/gallery/src/pages/mod.rs`: the lines of
  `carousel`; the page is in `Group::Surfaces`, where the board has the carousel.
- The engine: `Ctx::scroll_state_to(state, x, y)` and `Tree::scroll_state_to(state, x, y)`
  (`app/ctx.rs`, `app/runtime.rs`, `app/runtime/wheel_scroll.rs`). They send a scroll container to an offset over
  the frames that follow, on the curve a mouse's wheel scrolls by, and at once while motion is reduced, as
  `scroll_state_to_top` sends one to its top. `ScrollState` had no animated way to an offset: a turn of the
  carousel would have been a jump. Test `tests/runtime/smooth_wheel_scroll.rs`
  (`a_container_sent_to_an_offset_goes_there_over_the_frames_that_follow_held_to_its_content`). For
  `docs/src/content/docs/layout.md` or wherever `Ctx::scroll_to_top_of` is documented: "`ctx.scroll_state_to(&state,
  x, y)` scrolls a container to an offset over the next frames, held to its content."

## For `docs/src/content/docs/design-system.md`

## Carousel

Slides one after another in a room that shows one, or a few, at a time: turned by the round buttons beside them,
the dots under them, the arrow keys, a swipe or a drag.

```rust
use std::time::Duration;
use lurq::ui::{Carousel, Picture};

// Pictures as wide as the room, the first after the last.
Carousel::new(photos.iter().map(|photo| Picture::from_file(&photo.path).aspect_ratio(16.0 / 9.0)))
  .wrap(true)
  .label("Photos");

// Three cards in view, turning by itself every five seconds.
Carousel::new(cards)
  .per_view(3)
  .autoplay(Duration::from_secs(5))
  .index(shown.clone());
```

| Builder | |
| --- | --- |
| `new(slides)`, `slide(element)`, `slides(..)` | The slides: any elements, each laid out as wide as its room |
| `index(Signal<usize>)`, `index_controlled(index, on_change)`, `on_index_change(Fn(usize))` | Which slide it is at, counted from 0: the first one shown. Without either the carousel keeps it and starts at the first |
| `per_view(usize)`, `gap(f32)` | How many slides the room shows at once, each as wide as its share (1); and the room between two (12). A turn moves by one, and the last few slides are one place |
| `wrap(bool)` | Whether the first slide comes after the last, and the last before the first |
| `arrows(bool)`, `dots(bool)` | The round buttons beside the slides and the dots under them: both show unless told. With one place there are no dots |
| `autoplay(Duration)` | Turns to the next slide by itself, from the last to the first. It waits while the pointer is over it and while the focus is in it from the keyboard, and stops for good once a person turns it |
| `width(Length)` | How wide it is, its buttons with it: all the room there is unless told, and never wider than its room. The slides share what the buttons leave |
| `label(&str)`, `previous_label(&str)`, `next_label(&str)`, `slide_label(Fn(usize, usize) -> String)` | Its words to tooling: "Carousel", "Previous slide", "Next slide", "Slide 2 of 4" |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the carousel; its parts are `<id>-previous`, `-next`, `-viewport`, `-slide-<n>`, `-dot-<n>` |

- It is a component wherever it is put, as a [picture](#picture) is: it keeps which slide it is at.
- A press of a round button turns by one, a press of a dot goes to its place; a turn takes the slides there over a
  few frames, at once with motion reduced. A finger swipes a page at a time, as many slides as are shown. A drag
  of the mouse carries the slides, and let go a fifth of a slide away it goes on to the next. A trackpad scrolls
  them sideways and they come to rest on a slide.
- Keys: Tab reaches a round button, and the left and right arrow keys on it turn to the slide before and after;
  Tab reaches the dots once, the arrow keys move among them and Enter or Space goes to the one with the focus. A
  button that has nothing more to turn to takes nothing, and the focus it had goes to the other one. The slides
  that are not shown take neither the focus nor the pointer.
- Parts: `CarouselPart::Root`, `Track`, `Viewport`, `Slide`, `Arrow` (the round buttons, which are design-system
  [buttons](#button), `Outline`, round), `Dots`, `Dot`, `DotMark`. States: `CarouselState::Current` (the dot of the
  place it is at), `Disabled`.
- In the design the buttons are outline buttons of 36 with arrows of 16, 12 from the slides; 12 under the slides
  the dots are 6 across, 6 apart, in `Input`, the current one in `Primary`. A dot is pressed in a room 3 larger on
  every side.
- To what reads the tree it is a `region` described as a carousel, each slide a `group` described as a slide and
  called by its label, the dots a `tablist` of `tab`s.
- A renderer that draws another carousel has `view.previous()`, `view.next()`, `view.viewport()` (the slides, with
  what turns them) and `view.dots()`.

## For `CHANGELOG.md`

- `ui::Carousel` is a component of the design system: slides between two round buttons with dots under them,
  turned by the buttons, the dots, the arrow keys, a swipe, a drag of the mouse and by itself, with several in
  view where it is told.

## Decided

- The slides scroll in a list that goes a page at a time (`ScrollState::set_paging`), not by a transform: a
  finger's swipe, its momentum and its rest on a page are the engine's, as they are on a phone. A page is as many
  slides as are shown: each slide's room has the gap after it, and the list is one gap wider than the room that
  shows it, which cuts the gap off.
- The board draws one slide, the buttons and the dots, and no more. What a carousel is expected to do besides
  (several in view, wrap, a drag, the keys, autoplay, a disabled one) follows what the common carousels do; none
  of it changes what the board draws.
- Autoplay waits under the pointer and with the keyboard's focus, and stops for good once a person turns it: an
  auto-turning carousel with no way to stop it fails WCAG 2.2.2, and the board has no pause button.
- The width is the whole carousel's, its buttons with it, as every component's width is; the board's 336 is its
  240 of slide and the two buttons.
- In a narrow room the buttons stay beside the slides and the slides get narrower: in a window of 320 a slide is
  about 150 wide.

## Not done

- A vertical carousel, and buttons over the slides' sides in place of beside them: not on the board.
- Nothing is loaded ahead: a slide that is a picture loads when it is first laid out, which is at once, since
  every slide is in the list.
- Not seen by hand: a finger's swipe on a touch screen, a trackpad's sideways scroll coming to rest. The tests
  cover the buttons, the dots, the keys, a drag and autoplay; a press of a button was seen to turn in the gallery
  window.

## Differs from the board

- Nothing known.
