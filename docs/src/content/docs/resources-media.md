---
title: Resources And Media
description: Fonts, images, SVGs, background images, animated images, and async resources.
---

# Resources And Media

Media APIs are feature-gated. Enable only what the app needs.

| Feature | Public APIs |
| --- | --- |
| `image` | `Image`, `ImageData`, background images. |
| `svg` | `Svg`, `SvgData`. |
| `resources` | `ResourceLoader`, resource-backed image/SVG constructors, resource roots. |

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

Raw RGBA:

```rust
let pixels = vec![255; 64 * 64 * 4];
let image = ImageData::from_rgba(pixels, 64, 64);
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
  .background_image_resource("hero.jpg")
  .background_size(BackgroundSize::Cover)
```

Or use helper shortcuts:

```rust
Rect::new(320.0, 180.0).background_cover()
Rect::new(320.0, 180.0).background_contain()
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
