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
