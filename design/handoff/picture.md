# Picture

`ui::Picture`, from the board's `Picture` (`o1gLD`) in the section `Picture` (`wfriA`, dark `Gg1rp`) of
`Components / light`. Branch `claude/picture`. The design, its decisions and what was asked of Ivan:
`image-design.md` in the sessions' shared notes; the code and what the engine got for it: `picture-code.md`,
`loading-layer-design.md` and `declared-components.md` there.

## Changed outside the component's own files

- `crates/lurq/src/ui/mod.rs`, `crates/lurq/tests/ui/mod.rs`, `crates/gallery/src/pages/mod.rs`: the lines of
  `picture`, `lightbox` and `gallery`.
- The engine, each with its tests and its docs:
  - `.aspect_ratio(ratio)` on every component, `.drop_outside(SizeBounds)` on a child of a `Stack`,
    `BackgroundSize::Natural`, `.image_anchor(x, y)` (`layout.md`).
  - `lurq::resources`: `Source`, `Reader`, `Loader`, `Load`, `LoadState`, `LoadError`, `app.loader()`, the feature
    `http` (`resources-media.md`, "Loading").
  - `Element::component::<C>(props)` and `component_keyed`: a component declared where an element goes, mounted by
    the component whose render returns it; an unchanged one is held, not copied (`components.md`, "Declared
    Components").
  - `on_wheel` / `off_wheel` on every component (`styling-events.md`); `ZoomScroll::wheel_zoom(step)` and
    `drag_pan(bool)` (`ios.md`).
- `components::Image` is deprecated since 0.63 (`migration-v0-63.md`, "Images And Loading").
- `ui::Avatar`: `picture(impl Into<Picture>)` in place of `image(Image)`. `ui::Popover`, `ui::Toggletip`,
  `ui::MessageBox`, `components::DragContainer`: what they change on a root goes through `decorate_root`, so that it
  reaches a root that is a declared component.
- `#[derive(Clone)]` on `ui::Menu`, `ui::MenuItem`, `MenuEntry`.

## For `docs/src/content/docs/design-system.md`

## Picture

An image in its box: it covers the box unless told otherwise, shows a skeleton while it loads and its name when it
cannot be shown, and carries a badge, a bar and a layer of the app's over it.

```rust
use lurq::ui::{Picture, PictureFit, PicturePlaceholder};

// Pixels the app has, a file, bytes, a resource of the app, an address.
Picture::from_file("photos/lake.jpg")
  .label("The lake at dusk")
  .size_px(640.0, 360.0)
  .lightbox(true);

// As wide as its room, 3 by 2 before its pixels are there, with a small version blurred while they come.
Picture::from_url("https://example.com/lake.jpg")
  .label("The lake at dusk")
  .aspect_ratio(3.0 / 2.0)
  .placeholder(PicturePlaceholder::blur("photos/lake-small.jpg"))
  .on_error(|why| tracing::warn!("{why}"));

// A figure: the picture with a caption under it.
Picture::from_resource("images/lake.jpg")
  .width(320.0)
  .caption("Figure 1. The lake at dusk.");
```

| Builder | |
| --- | --- |
| `new(ImageData)`, `from_file(path)`, `from_bytes(bytes)`, `from_resource(path)`, `from_url(url)`, `from_source(source)`, `empty()` | Pixels the app has; a file or bytes, decoded off the UI thread; a resource of the app or an address, read by the app's `Loader`; any `resources::Source`; or no source: a box that says it has no picture. `ImageData` converts into a picture. An address outside a browser needs the feature `http`, decoding the feature `image`: without it the box says the picture did not load, and why |
| `cache_key(u64)`, `downscale_to(width, height)` | The key the decoded pixels are kept under, for bytes the app tells apart itself; and the most pixels that are kept of a large picture |
| `label(&str)`, `decorative(bool)` | What the picture is called: what it shows of itself where there are no pixels, and its name to tooling. A decorative one says nothing to tooling |
| `width(Length)`, `height(Length)`, `size_px(width, height)`, `aspect_ratio(f32)` | As a browser's `img`: with neither side it is as large as its pixels, with one the other follows their ratio, with both it is a box. A ratio gives the side that is not said, and with neither makes the box as wide as its room; it holds the box of a picture whose pixels are not there yet. The width is the most the box takes: in a narrower room it is as wide as the room, and a box of pixels keeps its shape |
| `fit(PictureFit)`, `anchor(StackAlignment)` | `Cover` (the default), `Contain`, `Fill`, `None` (its own size); and which side is kept where the pixels are cut or do not fill the box |
| `shape(PictureShape)`, `bare(bool)` | `Rounded` or `Circle`; bare is the pixels alone, with no fill, no corners and no line |
| `size(PictureSize)` | The form: `Lg` from 240 wide, `Md` under it, `Sm` at 96 and under, `Xs` at 40 and under. Not said, it follows the width the box gets: the width in pixels it was told, or less where its room is narrower, or the width of its room |
| `placeholder(PicturePlaceholder)` | What the box shows while the pixels are on their way: `Skeleton` (the sheen), `blur(source)` or `blur_pixels(data)` (a small version of the picture, blurred), `None` (the box alone) |
| `loading(bool)`, `progress(Option<f64>)`, `progress_text(Fn(PictureLoad) -> String)` | The box as while its pixels are on their way, for a picture the app is still fetching; how far the load has come, of 1, which draws a rail in place of the sheen; and the words under the rail. A picture that loads from an address shows its own progress where the server says how long it is |
| `on_load(Fn())`, `on_error(Fn(&str))`, `on_retry(Fn())` | Told once the pixels are there; once the picture did not come, with why; and on a press of "Try again", before the picture asks again |
| `error_description(&str)`, `retry_label(&str)`, `download_label(&str)`, `more_label(&str)`, `open_label(&str)` | Its words, the app's locale's (`Locale::common`: `picture_failed`, `try_again`, `download`, `more`, `open`), in English "The picture didn't load.", "Try again", "Download", "More", "Open"; the progress under the rail writes its megabytes in the locale's unit |
| `title(&str)`, `description(&str)`, `badge(element)` | What the bar along the bottom says, and what stands at the top, at the start. The bar shows under the pointer and with the focus |
| `on_download(Fn())`, `menu_item(MenuItem)`, `menu_items(..)`, `action(element)`, `actions(..)` | The bar's Download button, the menu behind More, and buttons of the app's |
| `bar(bool)`, `force_bar_shown(Option<bool>)` | Whether the bar is drawn, and whether it shows whatever the pointer does |
| `child(element)`, `children(..)` | The app's own layer over the whole picture |
| `caption(&str)` | A caption under the picture, which makes it a figure |
| `on_click(handler)` | A press of the picture as a whole |
| `selectable(bool)`, `selected(bool)`, `on_change(Fn(bool))` | A picture that can be chosen, whether it is, and what a press would make it. The picture does not keep it |
| `lightbox(bool)`, `open(Signal<bool>)`, `on_open_change(Fn(bool))` | A press, and the last button of a large picture's bar, open the picture alone in a [lightbox](#lightbox) over the window, with its title and description; whether it is open, to read and to set; and what tells of it. Without a signal the picture keeps it. A picture that is chosen by a press is opened by the button |
| `disabled(bool)`, `state(name, on)`, `style(part, ..)`, `rules(&recipe)`, `force(..)`, `id`, `key` | As a [button](#button) has them. `id` is the id of the box |

- A picture is a component of its own wherever it is put: `Row::new().child(Picture::from_url(..))`. It is mounted
  by the component whose render returns it, keeps its load, its menu and its lightbox there, and is told apart from
  its like by its place or its `key`. In a tree no component renders (`Tree::set_root(element)`) it shows nothing.
  `mount(ctx)` mounts it at once, for a place that needs the element.
- A picture made of the same plain things as the last time is not rendered again with what holds it: a list of five
  hundred costs what its changed pictures cost. A handler, a child, a badge or a menu makes it render with its
  parent.
- While a box that does not wait for its pixels loads, what stands in for them goes out over them as they come;
  a box the pixels size drops it at once.
- A press, Enter and Space press a picture that takes a press; Tab reaches it, and the ring of the focus is drawn
  from the keyboard. The buttons of the bar are pressed on their own, and the picture is not.
- Parts: `PicturePart::Figure`, `Root`, `Image`, `Edge`, `StandIn`, `Sheen`, `Veil`, `Overlay`, `Center`, `Mark`,
  `Label`, `ErrorIcon`, `ErrorLabel`, `ErrorDescription`, `Retry`, `Progress`, `Track`, `Fill`, `ProgressLabel`,
  `ProgressLine`, `Count`, `Top`, `Select`, `SelectIcon`, `Controls`, `Text`, `Title`, `Description`, `Actions`,
  `Control`, `ControlIcon`, `MoreMenu`, `Caption`. States: `PictureState::Bare`, `Circle`, `Pressable`,
  `Selectable`, `Selected`, `Disabled`, `Shown` (the bar is held shown), `On` (a button of the bar whose menu is
  open). Sizes
  `PictureSize::Xs`, `Sm`, `Md`, `Lg`.
- In the design the box is `Muted` with a line of `Border` inside it, round by `RadiusSize::Lg` (`Md` for a small
  one); the bar is white on a gradient to black at 70% in both themes, since the picture is not themed. A chosen
  picture has a disc of `Primary` with a check at its top end and a line of `Primary` about it.
- To what reads the tree it is an `img` called by its label, a `button` where it is pressed, a `checkbox` where it
  is chosen, and a `figure` about the two where it has a caption.
- A renderer that draws another picture has `view.frame()` (the box with everything in it), and the pieces it is
  made of: `view.image()`, `view.empty()`, `view.overlay()`, `view.top()`, `view.controls()`, `view.edge()`.

## For `CHANGELOG.md`

- `ui::Picture` is a component of the design system: an image from pixels, a file, bytes, a resource or an address,
  in a box it covers, is contained in, fills or keeps its own size in; with a skeleton, a blurred small version or a
  rail of progress while it loads, its name and "Try again" when it did not, a bar with a title, a download button
  and a menu, a badge, a caption that makes it a figure, a mark where it can be chosen, and a lightbox it opens
  itself in.
- `lurq::resources` reads what an app shows from a file, bytes, a resource of the app or an address, off the UI
  thread, with progress: `Source`, `Loader`, `Load`. The feature `http` reads addresses outside a browser.
- `Element::component::<C>(props)` declares a component where an element goes: it is mounted by the component whose
  render returns it, and an unchanged one is neither rendered nor copied again.
- `on_wheel` is called for a wheel or a trackpad turned over an element, before anything scrolls and whether or not
  anything can; `ZoomScroll` takes a mouse (`wheel_zoom`, `drag_pan`).
- `components::Image` is deprecated: `ui::Picture` takes its place.
- `ui::Avatar::picture` takes a `Picture` (it was `image(Image)`).

## Decided

- A picture is a component, not a builder with a still form: what it keeps (its load, the menu, the tooltips, its
  lightbox) lives in the component, and there is no second form that looks the same and does less (Ivan,
  2026-11-16). `Into<Element>` declares the component.
- A box of px by px keeps its shape in a room narrower than it, as `max-width: 100%; height: auto` does: it has a
  ratio a hair under its own and its height as the most, so that at its full width it is exactly as high as said.
- A picture given a width in pixels builds the one form of that width. Where its room makes the box narrower, the
  box is measured before paint and the form of the width it got is built in its place, from the first frame; a
  box at its width builds the one form it always did. Building every smaller form for each would double to
  quadruple what a fixed thumbnail builds.
- What stands in the middle of a box keeps 8 over and under it (the board pads 24): it is centered, so a box of the
  board's size looks the same, and a failed picture of 252 by 142 shows all four of its lines.
- The color of a chosen picture is `Primary` (Ivan), the disc at the top end.
- The "+5" count is the gallery's (`Gallery::max_shown`); the veil and the count are parts of a picture that the
  gallery turns on.
- Loading is `lurq::resources`, not the picture's own: a video, a sound and a font are read the same way.

## Not done

- The board's Mobile Picture (`VmTdR`, dark `zzLvV`): the phone's forms are not built.
- `ctx.mount` does not hold an unchanged component's nodes as `Element::component` does: the design is in
  `held-components-for-every-mount-handover.md`.

## Differs from the board

- The board draws the ring of the focus as a stroke on the box; lurq draws the theme's ring, 2 out from it, as every
  control of the design system does.
- The board's compact picture has download and open in its menu; lurq's menu holds the app's items alone, and a
  compact picture is opened by a press.
