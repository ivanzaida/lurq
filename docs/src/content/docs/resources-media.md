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
| `http` | Addresses on the network, for everything that loads: the client and its TLS. A file, bytes and a resource of the app are loaded by every build. Until 0.63 it was `resources`, which still turns it on. |
| `markdown` | Markdown parsing and `Markdown::mount`. |

## Fonts

`App` owns the glyph engine. Load fonts before running the window.

```rust
let mut app = lurq::app::App::new();

app.load_font_file(std::path::Path::new("assets/Inter.ttf"));
app.load_fonts_dir(std::path::Path::new("assets/fonts"));
app.register_font("ui", "Inter");
```

`App::load_font(bytes)` loads a font from memory: one put into the binary with `include_bytes!`, or one a page fetched.

A family may also be a CSS generic family (`"serif"`, `"sans-serif"`, `"monospace"`, `"cursive"`, `"fantasy"`), drawn with the platform's font for it; see [Generic Families](../theme/#generic-families) and [`GenericFamily`](https://docs.rs/lurq/latest/lurq/layout/text_style/enum.GenericFamily.html) for the fonts of each platform. A bundled font replaces one by an alias of the keyword: `app.register_font("monospace", "JetBrains Mono")`.

`App::set_generic_family(generic, family)` names the font a generic family draws with, in place of the one chosen from the system's fonts. `GenericFamily::SansSerif` is also the font of text with no family, so this is how an app draws all its text in a font it ships:

```rust
use lurq::layout::text_style::GenericFamily;

app.load_font(include_bytes!("../assets/fonts/Inter.ttf").to_vec());
app.set_generic_family(GenericFamily::SansSerif, "Inter");
```

A browser has no system fonts a page can read: an app that runs in one loads its fonts and names the sans-serif one at least. See [Web](../web/#the-fonts).

### Hinted Fonts

Text is drawn unhinted: a letter falls at any fraction of a pixel, and looks the same wherever it falls. An icon font is the opposite case. Its glyphs are drawn on a pixel grid, and a stroke a pixel and a half wide spread over three pixels is a blur. Load such a font with `load_hinted_font`:

```rust
app.load_hinted_font(include_bytes!("../assets/icons.ttf").to_vec());
```

Its glyphs are then drawn grid-fitted: at the nearest whole pixel size and position, with the font's TrueType hints applied along both axes. The font has to carry hints for this to do anything, and to ask for native ClearType in its `prep` program for the hints along x to count. A glyph under a transform, or in a text that is not snapped to pixels, is drawn unhinted like any other.

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

:::note
`lurq::components::Image`, which this section and the next two describe, is deprecated in 0.63 in favor of
`lurq::ui::Picture`: the same sources, in a box of the design system, with a load of its own. It is the engine's image
node and stays what a picture draws with. See the [migration guide](../migration-v0-63/).
:::

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

## Runtime Images

An image decoded at runtime from encoded bytes or a file, for attachments, thumbnails and screenshots that are not resources of the app:

```rust
use lurq::{components::{Image, Rect, Text}, images::ImageKey, node::BackgroundSize};

Image::from_bytes(attachment.bytes.clone())
  .cache_key(ImageKey::new(attachment.hash))
  .downscale_to(256, 256)
  .fit(BackgroundSize::Cover)
  .placeholder(Rect::new(96.0, 96.0).background("#374151"))
  .error_view(|error| Text::new(error.to_string()))
  .size(96.0, 96.0)
  .rounded(8.0)
```

`Image::from_bytes(bytes)` decodes PNG, JPEG, the first frame of a GIF or WebP, BMP and TIFF; `Image::from_file(path)` reads the file first; `Image::from_source(source, key)` takes either with a key. The decode runs on worker threads, off the UI thread: the node shows its `placeholder` (and its own background) until the pixels are there, then the image, and the `error_view` built from the `ImageDecodeError` for a broken file, instead of a panic. `downscale_to(width, height)` caps the decoded size, keeping the aspect ratio and never upscaling, so a thumbnail of a 4K screenshot does not keep its 30 MB of RGBA. `fit` places the pixels in the node's box: `Stretch` (the default, as `Image::new` always drew), `Contain` fits the whole image centered, `Cover` fills the box and crops the overflow; it works on every image node.

Decoding runs on worker threads, and in a browser on the page's one thread, after the turn of the loop that asked for the image.

Decoded pixels are kept in the process-wide cache, `lurq::images::decoded_images()`, keyed by content and size cap: `from_bytes` hashes the bytes (`ImageKey::of_bytes`), `from_file` keys the path with the file's size and modification time (`ImageKey::of_path`), and an app that stores a hash of its own builds the node with `Image::from_source(ImageSource::Bytes(arc), ImageKey::new(hash))`, which neither hashes nor copies the bytes (`cache_key` replaces the key after `from_bytes` has hashed). A chat laid out again, or a `VirtualizedList` row built again as it scrolls back in, finds its image ready in the first pass and decodes nothing. The cache keeps at most `DecodedImages::budget()` bytes of RGBA (128 MiB by default, `set_budget`), dropping the least recently used ready images; a node that still shows one keeps it. `decoded_images().state(key, cap)` reads an entry without decoding, `insert` puts pixels the app decoded itself, and `decode_image(&source, cap)` decodes on the calling thread. A test waits with `lurq::images::wait_for_decoded_images(timeout)`; see [Testing](../testing/#pastes-file-drags-and-runtime-images).

## Clipboard

With `clipboard`, `lurq::clipboard` reads and writes the OS clipboard. Besides the text functions (`copy_to_clipboard`, `read_from_clipboard`, `clear_clipboard`), `read_clipboard()` reads everything it holds at once as a `ClipboardContent`: the text, the image a screenshot tool or a browser put there (Windows `CF_DIBV5`/`CF_DIB`, macOS `public.png`/`public.tiff`, as RGBA), and the files copied in Explorer or Finder (`CF_HDROP`, file URLs). A clipboard may hold text and an image at once; `kinds()` tells which.

```rust
use lurq::clipboard::{copy_image_to_clipboard, read_clipboard};

if let Some(content) = read_clipboard() {
  if let Some(image) = content.image_content() {
    let png: Vec<u8> = image.to_png()?; // to store it
  }
  for path in content.file_paths() {
    // a file copied in Explorer or Finder
  }
}
copy_image_to_clipboard(width, height, &rgba);
```

`ClipboardImage::from_rgba(width, height, rgba)` builds the image a test or an app pastes; `copy_files_to_clipboard(&paths)` puts file paths there. A paste into a text input delivers the same `ClipboardContent` through a `PasteEvent`; see [Paste](../styling-events/#paste).

In a browser a page writes the clipboard at once and reads it only by waiting: `copy_to_clipboard` writes the system clipboard, the reading functions return what the page copied last, and only text crosses. A paste shortcut does read the system clipboard. See [Web](../web/#the-clipboard).

## A Color From The Screen

`lurq::eyedropper::EyeDropper` lets a person pick a color from anywhere on the screen: another app's window, a picture, the app itself. The pick runs off the loop, and its future does not borrow the window, so it is what a `ctx.future_action` runs, as the [file dialog](../styling-events/#files-a-person-gives) is:

```rust
use lurq::eyedropper::EyeDropper;

let window = ctx.window();
let pick = ctx.future_action(move |()| EyeDropper::new().pick(&window));
// In a handler: `pick.run(())`. `pick.state()` then holds `Ok(Some(color))`, `Ok(None)` where the person
// gave up, or `Err(Unsupported)` where the platform has no picking.
```

While it picks, the pointer is a cross and a loupe follows it: the pixels around the pointer magnified, the one taken outlined in the middle, its hex beside it, in the popover colors of the theme given with `.theme(&theme)`. A press takes the color; Escape, the right button or another window taken to the front gives up. The arrow keys move the pointer by a pixel, by ten with Shift, and Enter or Space take the color, for the pixel a hand cannot hit.

| Platform | |
| --- | --- |
| Windows | A thread of its own with the platform's means, as PowerToys' color picker and a browser's eyedropper do: a window over every monitor, all but transparent, takes the pointer and the keys, and a small layered one shows the loupe, drawn whole into a bitmap and given to it with its place in one call, so it never shows half drawn. Both are kept out of what is read of the screen (`WDA_EXCLUDEFROMCAPTURE`), so the pixels under the pointer are read live with GDI as it moves, a plain read of the composed desktop (not `CAPTUREBLT`, which hides the cursor for each read) that has other apps' layered windows in it; before Windows 10 2004 the screen is read once as the pick starts. The thread is aware of each monitor's scale, so the pixel taken is the physical one under the pointer at 150% and across monitors of different scales, and the loupe is as large on each |
| macOS | The system's own sampler (`NSColorSampler`), shown from the main thread: its own loupe, so `.theme(..)` is not used, and the system's keys (Escape gives up). The color it gives is converted to sRGB |
| Linux, iOS, a browser | Not yet: `EyeDropper::is_supported()` is false and `pick` gives `Err(Unsupported)` at once; the color picker and the color field show no eyedropper |

`EyeDropper::is_supported()` is what a component asks before it shows an eyedropper; the [color picker](../design-system/#color-picker) and the [color field](../design-system/#color-field) show theirs only where it is true. A test, or a tool driving the app, sets `EyeDropper::stand_in(|| Ok(Some(color)))`, which answers the picks of its thread until the guard it gives is dropped. Setting `LURQ_EYEDROPPER_SNAPSHOT` in the environment takes the way of a system that cannot keep the windows out of a capture: to check it, and to see the loupe in a screenshot.

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

## Loading

Everything that loads is given a `Source`, and the app's `Loader` reads it. A picture is the first to use it; a video
and a sound will be given the same source and the same loader.

```rust
use lurq::resources::Source;

Source::file("photos/lake.jpg");
Source::bytes(png_bytes);
Source::resource("brand/logo.png"); // a path under the app's resource root
Source::url("https://example.com/lake.jpg").header("Authorization", "Bearer token");
```

A source has a key, which tells it from another: a file's is its path with its size and the time it was changed,
bytes are hashed once, a resource's and an address's is their text. `.key(hash)` gives a key of the app's own in
its place.

`app.loader()` is the app's loader. It holds the resource root and what the network may do, reads on threads of its
own, makes one load of a source asked for twice, and stops a load nobody waits for any more:

```rust
use std::time::Duration;

app
  .loader()
  .root("assets")                         // what `Source::resource` is a path under
  .timeout(Duration::from_secs(10))       // of a load of an address; 30 seconds unless told
  .max_bytes(16 << 20)                    // the most `load` reads of one source; 64 MiB unless told
  .allow(|address| address.starts_with("https://cdn.example.com/"));
```

- `allow` says which addresses may be loaded: every one the app asks for, unless told. One it refuses fails with
  `LoadError::Denied` and nothing is sent. It is what keeps a text from a stranger, shown by the app, from fetching
  what the stranger chose.
- `loader.load(&source)` loads all the bytes of a source and gives a `Load`. Its `state()` is
  `Loading { loaded, total }`, `Ready(bytes)` or `Failed(LoadError)`; `wake_with` is called when it ends and as it
  comes further. A load that failed stays failed, so that what asks for it again is told so and not sent to the
  network again; `loader.forget(&source)` forgets it, and the next `load` tries anew.
- `loader.open(&source)` gives a `Reader`: `len()`, `read_at(offset, buf)` and `buffered()`, for what reads a part
  of a source and not all of it. A file, a resource and bytes are read where they are. A reader of an address is not
  built yet.
- The loader keeps no bytes in memory. What is decoded from them is kept by who decodes: a picture's pixels are in
  `lurq::images::decoded_images()`.

What is loaded from an address is kept on the disk where the app gives a folder for it, and is then there for the
next run of the app:

```rust
// The app's own cache folder: lurq picks none. Not called, nothing is written to the disk.
let app = App::new().enable_media_disk_cache(cache_folder);
let kept = app.loader().media_disk_cache();
kept.budget(2 * 1024 * 1024 * 1024); // 512 MiB unless told
kept.forget(&source);                // or `app.loader().forget(&source)`: it is loaded from its address again
kept.clear();
```

- A load of an address writes a file there, named by the source's key and its length, and a load of the same source
  reads the file and asks the network for nothing. A file is written under another name and renamed when it is
  whole. One that is gone, or is not as long as its name says, is loaded again.
- Over the budget the file used longest ago is deleted. Only the cache's own files are counted and deleted: the
  folder may hold other things.
- Nothing asks the server whether a file has changed, and nothing is let go for being old: an app that knows a
  source changed forgets it, or gives it a key of its own. What came with a header is kept as it came.
- `kept.holds(&source)`, `bytes()`, `len()`, `folder()` and `is_on()` say what it keeps. A folder that cannot be
  written is one warning in the log, and nothing is kept there.
- A sound at an address plays from its file ([playing a sound](../design-system/#playing-a-sound)).

With the feature `tokio`, an app that gives its runtime (`App::with_tokio_handle(handle)`) has its loader read on
that: a load is a blocking task of the runtime, four at a time, and the loader starts no threads of its own. A
loader made by itself is told with `loader.tokio_handle(handle)`. The runtime is to run for as long as the app
loads: a load given to one that has stopped fails with `LoadError::Cancelled`, and `App::clear_tokio_handle()`
gives the loader its own threads back.

A `LoadError` says why: `NotFound`, `Denied`, `Status(code)`, `TooLarge { limit }`, `TimedOut`, `Cancelled`, `Io`, and
`Unsupported`, which is what a build that cannot load such a source answers. An address in a build without the
feature `http` is one: it compiles, and fails at run time with the reason and one warning in the log. In a browser
a file and an address are not loaded yet.

## Resource Loader

`ResourceLoader` is the loader before `app.loader()`, which the feature `http` keeps: it is polled, and a resource
image built with `Image::from_resource`, a background image and a resource SVG still use it.


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
    .on_link_select(|link| println!("{}", link.destination())),
)
```

Use `MarkdownProps::style(...)` for the base text style (the theme's `TypographyStyle::Article`, 15 on lines of 1.65, where it gives none), `.theme(...)` for block styling, and `.width(...)` for a width constraint. Link callbacks decide how the application opens or routes destinations. The gallery's Markdown page (in Data) shows every block.

`MarkdownProps::build(theme)`, and `Element::from(props)`, draw the document once with nothing kept: no link tooltips, every `<details>` as it begins, no jump to a footnote. Mounted, the component keeps what a person does to it.

### The blocks

The document draws every block of the design's (`pencil/lurq-design-system.pen`, Markdown):

- **Headings** in the theme's type scale (`TypographyStyle::Heading1` to `Heading6`): 30 bold, 24 with a line under it, 20, 16, 14, and 12 in capitals in the muted foreground.
- **Lists**: a number at the end of a column of 18 in the muted foreground; a bullet of 5, a dot at the first level and a ring under it (a selection over a bulleted item still copies its `-`). **Tasks** (`- [ ]`, `- [x]`) are the checkbox's look, a done one's words in the muted foreground. A task's box is read only, taking neither the pointer nor the focus, unless the app takes its change: `.on_task_change(|index, checked| ..)` is told the task's index among the document's tasks and whether it is checked now, and the app writes that into its source.
- **Quotes** are `ui::Blockquote`: a line of 2 at their start, their words 14 in italics in the muted foreground. **Code blocks** are `ui::CodeBlock`: a bar named by the language, with the button that copies the code, over the code in the family of code, colored by its language's highlighter in the theme's code colors (see [Highlighting code](#highlighting-code)). Both look as the theme's `blockquote` and `code_block` recipes say, one look wherever they are.
- **Tables**: a box of the middle radius, the header on the muted fill, every other body row striped, a line between two columns, the columns of one width and each aligned as its delimiter row says.
- **The rule** (`---`): a line of 1 in the border's color. **An image** that stands alone: the image where the app draws images (`image` and `resources`), or else a box of its place, with its caption under it: its title, or its words.
- **Admonitions**, GitHub's: `> [!NOTE]`, `> [!TIP]`, `> [!IMPORTANT]`, `> [!WARNING]`, `> [!CAUTION]` on a quote's first line. Each is a `ui::Callout` with a bar, in the tone of its kind: info, success, neutral, warning, destructive.
- **Details**: `<details>` with a `<summary>`, which opens and closes what it holds; `<details open>` is open to begin with. The parser reads them from the HTML blocks around the Markdown in them.
- **Footnotes**: a reference is its number in the info color, numbered by the first reference to each, and goes to its footnote; the footnotes stand at the end of the document under a short rule, each with the way back to where the text first refers to it.
- **In a sentence**: code on the box of `ui::InlineCode`, keys (`<kbd>Ctrl</kbd>`) on the box of `ui::Kbd`, struck words in the muted foreground, links in the info color at 500.

The parser reads them all from `pulldown-cmark`'s events: `MarkdownBlock::Admonition` with its `MarkdownAdmonitionKind`, `MarkdownBlock::Details`, and `MarkdownInline::Kbd` are new.

### The theme

`ThemeMarkdown`'s colors are the palette's roles (`TextColor::Palette`) or colors an app gives (`TextColor::Color`): a document is drawn in the theme it is in, light and dark, and in a theme an app makes. Its sizes are the design's, relative to the document's text where they are of type (inline code is 13 where the text is 15). `ThemeMarkdown::resolve(&palette)` gives the theme with every role turned into the color the palette has for it.

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
- A code block does not wrap: it is as wide as its longest line up to the container's width, and a longer line scrolls inside its box, as when filling.
- A table still takes the container's full width: its columns share that width so that they line up across its rows.
- An explicit `.width(...)` still sets the document's width.
- In a `Column`, the container's width is the column's. A `Row` gives its children unbounded width, so a bubble in a row needs `.max_width(Dimension::Pct(100.0))` (or a `flex_shrink` factor) for its text to wrap at the row's width.

### Code blocks

A code block (`ui::CodeBlock`) never wraps its lines. A line wider than the block's box scrolls sideways inside it: the box, its bar and the document keep their width, and the block's scroll area shows its bar over the code while it overflows. Shift+wheel and a trackpad's sideways swipe scroll the block; a plain or slightly slanted vertical wheel over it scrolls the page around it, as it always did, so a chat's follow-to-end is not caught by a block that only overflows sideways. The block's own scroll is kept across renders, keyed by the block's ordinal among the document's code blocks and its language: a streaming reply and paragraphs added around it keep its place (a code block inserted above it shifts the ordinals). A math block (`$$`) scrolls the same way. A block whose lines fit has no scrollbar. Selection and copy cross a scrolled block like any other block, the lines out of view included, and its language with them; the glyphs of the document's controls (the copy button, a task's check, an admonition's icon) are never part of what a selection copies.

### Highlighting code

A code block, Markdown's fenced code, `ui::InlineCode` given a language and a code field color their code the same way: a **highlighter** tells the code's tokens apart and names each, and the theme's **code colors** draw each name. Neither needs the `markdown` feature (`lurq::code`).

A token's name is dotted, as TextMate and tree-sitter name them: `keyword`, `keyword.control`, `string`, `string.escape`, `number`, `comment`, `comment.doc`, `type`, `function`, `function.macro`, `variable`, `variable.builtin`, `constant`, `property`, `attribute`, `operator`, `punctuation`, `tag`, `regex`, `namespace`, `label` (`lurq::code::token` has them as constants), and any other a highlighter gives. A name the colors have nothing for is looked up dropping its tail: `keyword.control.rust`, then `keyword.control`, then `keyword`, then the code's own color.

**One highlighter for each language.** lurq registers its own at start, for Rust (`rust`, `rs`), TOML, JSON (`jsonc`), JavaScript (`js`, `jsx`), TypeScript (`ts`, `tsx`), shells (`sh`, `bash`, `zsh`, `console`), Python (`py`), CSS (`scss`, `less`), HTML (`htm`, `vue`), XML (`svg`), YAML (`yml`), SQL (`postgres`, `mysql`, `sqlite`) and Markdown (`md`). A language with none is plain text. An app registers its own for any language; one for a language that has one replaces it, lurq's included, under each of its names, and whatever shows that language's code is drawn again:

```rust
use lurq::code::{CodeToken, token};

// Any Fn(&str) -> Vec<CodeToken> is a highlighter: here, every word that starts with `$` is a variable.
app.register_highlighter("dollars", |code: &str| {
  let mut found = Vec::new();
  let mut at = 0;
  for word in code.split(' ') {
    if word.starts_with('$') {
      found.push(CodeToken::new(at..at + word.len(), token::VARIABLE));
    }
    at += word.len() + 1;
  }
  found
});

// Other names of the language, which a fence (```psql) may use.
app.register_highlighter_with_aliases("sql", &["psql", "postgres"], my_sql_highlighter);
```

- A highlighter is a `lurq::code::Highlighter` (any `Fn(&str) -> Vec<CodeToken>` is one): its tokens are in bytes of the code, and need not cover it; what none covers is plain. Tokens that are empty, out of the code, off a character's boundary or over another are left out.
- `CodeHighlighter::new(h)` makes one to keep and clone: `CodeBlock::highlighter(h.clone())` gives a block its own over the registry's, and what it gave for a code is not asked again. `lurq::code::register_highlighter`, `remove_highlighter`, `highlighter(language)`, `knows(language)`, `languages()` and `tokens(code, language)` are the registry itself; it is the process's, as an app is one. `App::register_highlighter` says the same.
- What a highlighter gave is kept, by the highlighter and the code (a few hundred codes, a few megabytes of them): a render that changes nothing of a block's code does not tokenize it again, and a highlighter registered in place of another forgets what the other gave.
- A Markdown fence's info string is read up to its first space or comma: ```` ```rust,ignore ```` is Rust.

**lurq's own are tables.** One scanner reads every language from a `CodeLanguage`: its names and aliases, its words by name (`.words(token::KEYWORD_CONTROL, &["if", "else"])`, in any case for SQL), line comments and block comments (nested, and documenting ones), forms of strings (prefixes, raw strings with their `#`s, three quotes, escapes, a quote doubled), the forms of numbers, what makes a type (a capital), a namespace (a name before `::`), a call (a name before `(`), a macro (`name!`), an attribute or a decorator (`#[`, `@`), a key (before `:` or `=`), a section (`[table]`), a variable (`$HOME`), a command, a tag. It reads line by line, from the state a line starts in (inside a block comment or a string an earlier line opened: `CodeLanguage::states`, `line_tokens`), so a long text can be read from any line. It has no dependency and builds for the browser. An app describes a language of its own with the same table, or changes one of lurq's:

```rust
use lurq::code::{CodeLanguage, KeyRule, StringForm, token};

let ini = CodeLanguage::new("ini")
  .line_comments(&[";", "#"])
  .strings(vec![StringForm::new("\"", "\"")])
  .words(token::CONSTANT_LANGUAGE, &["true", "false"])
  .keys(KeyRule::Equals)
  .sections(true);
app.register_highlighter("ini", ini);
app.register_highlighter("rust", CodeLanguage::rust().words(token::KEYWORD, &["yeet"]));
```

**A tokenizer of another crate** is plugged in by an adapter that names its runs. The gallery does it with `syntect` (`crates/gallery/src/syntect_code.rs`), which lurq does not depend on: syntect's scopes (`keyword.control.rust`, `string.quoted.double`, `entity.name.function`) mapped to lurq's names, registered for Go, C and Java, and given to one block of the Code Block page in place of lurq's Rust.

**The colors are the theme's**: a `CodeScheme`, a style (color, weight, leaning) for each name, one for the light appearance and one for the dark ([Code](../theme/#code)). `CodeScheme::lurq()` is the theme's unless told, in the palette's roles, so code looks as the rest of the theme does in both appearances. A block styles a name over them with `.token_style(name, |s| s.foreground(..).bold().italic())`, or draws in a scheme of its own with `.code_scheme(..)`.

**A long block**: a code block of more than 400 lines lays out and paints the lines in view and a chunk around them, whatever its length: in its own room where it has a greatest height, and in the window where it has none, as a long fence of a document has. Such a fence is selected on its own (select all takes its code whole) and is not one text with the document's highlights. Ten thousand lines of Rust take what a screen of them takes: the frame paints ten quads and some 13 KB of text, where it painted 140 009 quads and all 600 KB before. See [Code Block](../design-system/#code-block).

**A code field** colors its value by the same names, colors, weights and leaning, in its own type, again after every edit: `TextInput::code_language("sql")`, or `.code_highlighter(h)`. It is built on `TextInput::tokens_with(f)`, tokens asked of each value the input holds ([Source-backed input tokens](../text-input-tokens/)).

### Links

A paragraph is one text, links included: it wraps and selects as one. Each link in it is a Tab stop in document order; a click and Enter follow it (Space does not), the pointer shows a pointer cursor, and an MCP client sees it as a `link` named by its words. `on_link_select` runs inside the input's dispatch, so `lurq::app::events::is_synthetic_input()` tells a person's press from an MCP client's, and `MarkdownLink::index()` says which link it was.

`on_link_select` gets left clicks and Enter. To tell how a link was activated, use `.on_link_activate(...)`: it gets the link and the `MouseEvent` that activated it, for a left click, a middle click, or Enter on the focused link. The event's `button` is `Left` or `Middle` (`Left` for Enter), `from_keyboard` is set for Enter, and `shift`, `ctrl`, `alt` and `meta` are the modifiers held (`meta` is Cmd on macOS and the Windows key on Windows). Browser rules for a chat's links, for example:

```rust
use lurq::app::events::MouseButton;

MarkdownProps::new(reply).on_link_activate(move |link, event| {
  let shortcut = if cfg!(target_os = "macos") { event.meta } else { event.ctrl };
  if event.button == MouseButton::Middle || (shortcut && !event.shift) {
    tabs.open_in_background(link.destination());
  } else if shortcut && event.shift {
    tabs.open_and_switch(link.destination());
  } else {
    tabs.open_here(link.destination());
  }
})
```

- A middle click is a press and a release of the middle button on the same link, like a left click: a press released off the link activates nothing, and neither does a press on one link released on another more than a few pixels away. Closer than that, the link under the release activates, as with a left click. The press starts no text selection, no drag of what the link is in and no [middle-button autoscroll](../layout/#middle-button-autoscroll) of the container around it, and lurq has no middle-click paste for it to start.
- `on_link_activate` runs before `on_link_select`, which still gets the left clicks and Enter when both are set. With `on_link_activate` set, an in-app link no longer navigates the router by itself. Without it, a middle click on a link does nothing.
- It runs inside the input's dispatch, like `on_link_select`: an MCP client's `lurq_interact` click with `"button": "middle"` or `"modifiers": ["ctrl", "meta"]` reaches it with those, and `is_synthetic_input()` is `true`.

`.link_presentation(...)` decides how each link is shown. It is called once per link and render with the same `MarkdownLink` that `on_link_select` gets, so what a link shows always comes from the destination it opens:

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
  .on_link_select(move |link| open(link.destination(), lurq::app::events::is_synthetic_input()))
```

- `link()` draws the words in the theme's `link` style, or the presentation's `style(...)` over the text around them. `text()` draws them as the surrounding text: nothing to press or focus.
- `trailing(...)` runs follow the words in the same paragraph, so they wrap with it; each run's style applies over the text around the link. After a link, the pointer takes them as part of the link. Like CSS `::after` content they are presentation: copying a selection, and inspection tools (MCP, DevTools), leave them out, and a selection highlight skips them.
- The words and trailing runs are drawn between invisible bidi marks, and a paragraph with a link drops every directional control (U+202A–U+202E, U+2066–U+2069) and directional mark (U+200E, U+200F, U+061C) from its text, so lurq's marks are the only ones left: a right-to-left override in the words or anywhere in the paragraph cannot draw a host after them backwards, not even past Unicode bidi's nesting limit. In a right-to-left paragraph (Hebrew or Arabic text) the paragraph still reads right to left around each link, and a link's trailing runs still follow its words, left to right. At any width, a laid-out line puts a link's own runs, or nothing, right after its words. A link stays on one line where it fits; its words are one line of text (a hard break in them is a space). A link whose words draw nothing (none, or only spaces, invisible characters and combining marks with no letter) is not presented: it is its words as text, with no runs and nothing to press. The marks are invisible, are not copied and are not part of the paragraph's `text_content`. Text you build yourself from a destination outside a run (a tooltip, say) needs the same care: `lurq::markdown::isolate_bidi` strips the controls and marks and wraps the text in an isolate; `strip_bidi_controls` only strips.
- A link's name (`MarkdownLink::text()`, and its MCP name) is its words as drawn: inline HTML as its text, an image as its alt text, without directional controls. An email autolink (`<team@example.com>`) reaches the app as `mailto:team@example.com`.
- `hovered(...)` and `focused(...)` (`MarkdownLinkState`) recolor the words, their underline and the trailing runs, and can fill behind the words, one rectangle per line with the link's `radius(...)`. Keyboard focus (focus-visible) also draws the `focus_ring(...)` around each line of the words, outside them on all sides and over the text. The document's own containers do not clip it; a code block, a table or the app's containers do.
- `tooltip(...)` shows under the link after the pointer rests on it for `ThemeMarkdown::link_tooltip_delay`, and at once while it has keyboard focus. It lets the pointer through.
- `pointer_only()` leaves a link to the pointer, for an app whose reading text is not part of its Tab order: the link is no Tab stop, a press on it leaves the focus where it was, so Enter never reaches it and it never shows its focused look, its ring or a tooltip from the keyboard. A click and a middle click follow it, and the pointer over it shows its hovered look and its tooltip, as on any link.
- Unset looks are the theme's: `ThemeMarkdown::link`, `link_hovered`, `link_focused`, `link_focus_ring`, `link_radius`, `link_tooltip` and `link_tooltip_box`.
- Underlines (`TextStyle::underline`, `MarkdownTextStyle::underline`: color, thickness and offset, each defaulting to the text color and the font's metrics) are drawn under every line a span covers, and lines through text (`strikethrough`: color and thickness) the same way. `~~struck~~` text has one by default, from `ThemeMarkdown::strikethrough`. A plain `Text` draws both too; see [layout](../layout/#underline-and-strikethrough).

`.list_view(...)` lets the app draw a list block itself. It is called for every list with a `MarkdownList` and returns the element to draw, or `None` to draw the list as usual. `MarkdownList::inlines` and `blocks` draw content as the document does, links included; decide from `items()` before drawing with them, since drawing numbers the links.
