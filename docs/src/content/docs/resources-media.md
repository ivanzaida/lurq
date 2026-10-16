---
title: Resources And Media
description: Fonts, images, SVGs, background images, animated images, and async resources.
---

# Resources And Media

Media APIs are feature-gated. Enable only what the app needs.

| Feature | Public APIs |
| --- | --- |
| `raster` | `Image`, `ImageData`, `StreamingImage`, `Video`, and raw RGBA/native image transport. |
| `image` | Enables `raster` plus image decoding and image-backed style helpers. |
| `svg` | `Svg`, `SvgData`. |
| `resources` | `ResourceLoader`, resource-backed image/SVG constructors, resource roots. |
| `markdown` | Markdown parsing and `Markdown::mount`. |

## Fonts

`App` owns the glyph engine. Load fonts before running the window.

```rust
let mut app = lurq::app::App::new();

app.load_font_file(std::path::Path::new("assets/Inter.ttf"));
app.load_fonts_dir(std::path::Path::new("assets/fonts"));
app.register_font("ui", "Inter");
```

Text uses `TextStyle`.

```rust
use lurq::{
  components::Text,
  layout::text_style::{FontStyle, FontWeight, TextStyle},
  node::color::Color,
};

Text::styled(
  "Hello",
  TextStyle {
    font_family: "ui".into(),
    font_size: 18.0,
    line_height: 1.25,
    weight: FontWeight::Bold,
    style: FontStyle::Normal,
    color: Color::from_hex("#e5e7eb"),
    ..TextStyle::default()
  },
)
```

## Images

With `image`, load from bytes, files, or raw RGBA.

```rust
use lurq::{components::Image, images::ImageData};

let image = ImageData::from_file("assets/photo.jpg").unwrap();
Image::new(image).size(240.0, 160.0)
```

Supported formats come from the `image` dependency configuration: PNG, JPEG, WebP, GIF, BMP, and TIFF. GIF and animated WebP preserve animation frames.

Raw RGBA only requires `raster` (also enabled by `canvas` and `image`):

```rust
let pixels = vec![255; 64 * 64 * 4];
let image = ImageData::from_rgba(pixels, 64, 64);
```

Streaming RGBA uses a stable image identity and uploads new pixels when the buffer version changes:

```rust
use lurq::{components::Image, images::StreamingImage};

let stream = StreamingImage::new_rgba(vec![0; 64 * 64 * 4], 64, 64);

stream.update_rgba(|pixels| {
  pixels[0] = 255;
  pixels[1] = 128;
  pixels[2] = 0;
  pixels[3] = 255;
});

Image::new(stream.image_data())
```

## Resource Images

With `image` and `resources`, let the runtime load files relative to the resource root.

```rust
let mut app = lurq::app::App::new();
app.set_resource_root(std::path::PathBuf::from("assets"));

let avatar = lurq::components::Image::from_resource("avatar.png");
```

The first frame may render while the resource is pending. The loader caches successful results according to `ResourceConfig`.

Background images use the same feature pair:

```rust
use lurq::{components::Rect, node::BackgroundSize};

Rect::new(320.0, 180.0)
  .background_image("hero.jpg")
  .background_size(BackgroundSize::Cover)
```

Or use helper shortcuts:

```rust
Rect::new(320.0, 180.0).background_cover()
Rect::new(320.0, 180.0).background_contain()
```

Slider track and thumb styles can also use resource-backed background images:

```rust
use lurq::components::Slider;

Slider::new(value)
  .track(|style| style.background_image("ui/track.png").background_cover())
  .thumb(|style| style.background_image("ui/thumb.png").background_cover())
```

## SVG

With `svg`, construct SVGs from bytes or strings.

```rust
use lurq::{components::Svg, node::color::Color, svg::SvgData};

let icon = SvgData::from_str(r#"<svg viewBox="0 0 24 24"></svg>"#)
  .with_fill(Color::from_hex("#a855f7"));

Svg::new(icon).size(24.0, 24.0)
```

With `svg` and `resources`:

```rust
Svg::from_resource("icons/search.svg").size(18.0, 18.0)
```

`SvgData` supports fill, stroke, and opacity overrides.

## Resource Loader

`ResourceLoader::load_resource(path, config)` returns:

- `Pending` while async load is in progress,
- `Loaded(Arc<Vec<u8>>)` when bytes are ready,
- `Error(ResourceError)` for not found, network, OS, or unknown errors.

Local paths are resolved under `App::set_resource_root(...)` when set. Remote `http://` and `https://` URLs are loaded through `ureq`.

```rust
use std::sync::Arc;
use lurq::resources::{LoadResourceResult, ResourceConfig};

let loader = lurq::resources::ResourceLoader::new();
let path: Arc<str> = Arc::from("data.json");
match loader.load_resource(&path, Some(ResourceConfig { ttl: 60, retries: 1 })) {
  LoadResourceResult::Pending => {}
  LoadResourceResult::Loaded(bytes) => println!("{} bytes", bytes.len()),
  LoadResourceResult::Error(error) => println!("{error:?}"),
}
```

Most app code should prefer the higher-level resource-backed `Image` and `Svg` constructors.

## Markdown

Enable `markdown` to parse and render Markdown through a retained component:

```rust
use lurq::components::{Markdown, MarkdownProps};

Markdown::mount(
  ctx,
  MarkdownProps::new("# Notes\n\nRead the **release notes**.")
    .selectable(true)
    .on_link_click(|link| println!("{}", link.destination())),
)
```

Use `MarkdownProps::style(...)` for the base text style, `.theme(...)` for block styling, and `.width(...)` for a width constraint. Link callbacks decide how the application opens or routes destinations. The demo's `/markdown` route exercises the component.

By default the document fills its container's width, and so does every block in it. `.fit_content(true)` sizes it to its content instead, like CSS `width: fit-content; max-width: 100%`, so a chat bubble hugs its message:

```rust
Column::new()
  .align_items(Alignment::End) // own messages on the right
  .child(
    Column::new()
      .padding(8.0)
      .background(bubble_color)
      .child(Markdown::mount(ctx, MarkdownProps::new(message).fit_content(true))),
  )
```

- The document is as wide as its widest block (a short message as its longest line) and never wider than its container's width, where its text wraps. Paragraphs, headings, lists, quotes and footnotes all size to their content; a long list item wraps beside its marker.
- Every block is then stretched to the document's width, as CSS blocks are: a code block's box and a rule span the widest block.
- A code block does not wrap: it is as wide as its longest line up to the container's width, and a longer line overflows its box, as when filling.
- A table still takes the container's full width: its columns share that width so that they line up across its rows.
- An explicit `.width(...)` still sets the document's width.
- In a `Column`, the container's width is the column's. A `Row` gives its children unbounded width, so a bubble in a row needs `.max_width(Dimension::Pct(100.0))` (or a `flex_shrink` factor) for its text to wrap at the row's width.

### Links

A paragraph is one text, links included: it wraps and selects as one. Each link in it is a Tab stop in document order; a click and Enter follow it (Space does not), the pointer shows a pointer cursor, and an MCP client sees it as a `link` named by its words. `on_link_click` runs inside the input's dispatch, so `lurq::app::events::is_synthetic_input()` tells a person's press from an MCP client's, and `MarkdownLink::index()` says which link it was.

`.link_presentation(...)` decides how each link is shown. It is called once per link and render with the same `MarkdownLink` that `on_link_click` gets, so what a link shows always comes from the destination it opens:

```rust
use lurq::{
  app::theme::{MarkdownFocusRing, MarkdownInlineStyle, MarkdownLinkState, MarkdownTextStyle},
  components::{MarkdownLinkPresentation, MarkdownProps, MarkdownRun},
  layout::text_style::TextUnderline,
  markdown::isolate_bidi,
};

let words = MarkdownInlineStyle {
  text: MarkdownTextStyle {
    color: Some(accent),
    underline: Some(TextUnderline::new().color(muted).thickness(1.0)),
    ..Default::default()
  },
};
MarkdownProps::new(reply)
  .link_presentation(move |link| match kind_of(link.destination()) {
    // In the app: the link look, a tooltip naming the place.
    Kind::App(place) => MarkdownLinkPresentation::link()
      .style(words.clone())
      .hovered(MarkdownLinkState::new().color(accent_hover).background(hover_fill).underline_color(accent_hover))
      .focus_ring(Some(MarkdownFocusRing::new(2.0, focus).radius(3.0)))
      .tooltip(format!("{place} · opens in this tab")),
    // On the web: the host and an icon glyph after the words.
    Kind::Web(host) => MarkdownLinkPresentation::link()
      .style(words.clone())
      .trailing([MarkdownRun::new(format!("\u{a0}{host}\u{a0}\u{e04d}"), host_style.clone())])
      .hovered(MarkdownLinkState::new().trailing_color(secondary))
      // A destination is text the reply's author wrote: isolate it so its
      // directional controls cannot reorder the sentence around it.
      .tooltip(format!("Opens {} in your browser", isolate_bidi(link.destination()))),
    // Anything else is not followed: the words as text, then the address.
    Kind::Other => MarkdownLinkPresentation::text()
      // A run is stripped of directional controls and isolated by lurq.
      .trailing([MarkdownRun::new(format!(" ({})", link.destination()), address_style.clone())]),
  })
  .on_link_click(move |link| open(link.destination(), lurq::app::events::is_synthetic_input()))
```

- `link()` draws the words in the theme's `link` style, or the presentation's `style(...)` over the text around them. `text()` draws them as the surrounding text: nothing to press or focus.
- `trailing(...)` runs follow the words in the same paragraph, so they wrap with it; each run's style applies over the text around the link. After a link, the pointer takes them as part of the link. Like CSS `::after` content they are presentation: copying a selection, and inspection tools (MCP, DevTools), leave them out, and a selection highlight skips them.
- The words and trailing runs are drawn between invisible bidi marks, and a paragraph with a link drops every directional control (U+202A–U+202E, U+2066–U+2069) and directional mark (U+200E, U+200F, U+061C) from its text, so lurq's marks are the only ones left: a right-to-left override in the words or anywhere in the paragraph cannot draw a host after them backwards, not even past Unicode bidi's nesting limit. In a right-to-left paragraph (Hebrew or Arabic text) the paragraph still reads right to left around each link, and a link's trailing runs still follow its words, left to right. At any width, a laid-out line puts a link's own runs, or nothing, right after its words. A link stays on one line where it fits; its words are one line of text (a hard break in them is a space). A link whose words draw nothing (none, or only spaces and invisible characters) is not presented: it is its words as text, with no runs and nothing to press. The marks are invisible, are not copied and are not part of the paragraph's `text_content`. Text you build yourself from a destination outside a run (a tooltip, say) needs the same care: `lurq::markdown::isolate_bidi` strips the controls and marks and wraps the text in an isolate; `strip_bidi_controls` only strips.
- A link's name (`MarkdownLink::text()`, and its MCP name) is its words as drawn: inline HTML as its text, an image as its alt text, without directional controls. An email autolink (`<team@example.com>`) reaches the app as `mailto:team@example.com`.
- `hovered(...)` and `focused(...)` (`MarkdownLinkState`) recolor the words, their underline and the trailing runs, and can fill behind the words, one rectangle per line with the link's `radius(...)`. Keyboard focus (focus-visible) also draws the `focus_ring(...)` around each line of the words, outside them on all sides and over the text. The document's own containers do not clip it; a code block, a table or the app's containers do.
- `tooltip(...)` shows under the link after the pointer rests on it for `ThemeMarkdown::link_tooltip_delay`, and at once while it has keyboard focus. It lets the pointer through.
- Unset looks are the theme's: `ThemeMarkdown::link`, `link_hovered`, `link_focused`, `link_focus_ring`, `link_radius`, `link_tooltip` and `link_tooltip_box`.
- Underlines (`TextStyle::underline`, `MarkdownTextStyle::underline`: color, thickness and offset, each defaulting to the text color and the font's metrics) are drawn under every line a span covers. Rich text draws them; a plain `Text` does not yet.

`.list_view(...)` lets the app draw a list block itself. It is called for every list with a `MarkdownList` and returns the element to draw, or `None` to draw the list as usual. `MarkdownList::inlines` and `blocks` draw content as the document does, links included; decide from `items()` before drawing with them, since drawing numbers the links.
