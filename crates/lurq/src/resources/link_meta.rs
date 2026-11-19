//! What a page says about itself, read from the start of its HTML: the words and the pictures a preview of a link
//! shows.
//!
//! [`LinkMeta::from_html`] scans the head of a page with a small reader of its own, no HTML parser: it finds the
//! `<title>`, the `<meta>` tags of OpenGraph (`og:title`, `og:description`, `og:image`, `og:site_name`, `og:video`,
//! `og:video:duration`), of a Twitter Card and the plain `description`, the `<link>`s of an icon, and a `<base>`.
//! It takes what a page gets wrong in its stride (attributes without quotes, tags left open, names in capitals, a
//! head with no end), reads UTF-8 and the charsets a `<meta charset>` names that it knows (Windows-1252, ISO 8859-1,
//! Windows-1251, KOI8-R and KOI8-U), and makes every address in it whole from the page's own.

use std::sync::Arc;

/// What a page says about itself: what a link preview shows of it. Every field is what the page gave, or none.
///
/// ```
/// use lurq::resources::LinkMeta;
///
/// let page = br#"<html><head>
///   <title>Layout in lurq</title>
///   <meta property="og:site_name" content="lurq.dev">
///   <meta property="og:image" content="/shots/rows.png">
///   <link rel="icon" href="favicon.png">
/// </head><body>...</body></html>"#;
/// let meta = LinkMeta::from_html("https://lurq.dev/docs/layout", page);
/// assert_eq!(meta.title.as_deref(), Some("Layout in lurq"));
/// assert_eq!(meta.image.as_deref(), Some("https://lurq.dev/shots/rows.png"));
/// assert_eq!(meta.icon.as_deref(), Some("https://lurq.dev/docs/favicon.png"));
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LinkMeta {
  /// The address the page was asked for.
  pub url: Arc<str>,
  pub title: Option<Arc<str>>,
  /// A line or two about it.
  pub description: Option<Arc<str>>,
  /// The name of the site it is on: "lurq.dev", "GitHub".
  pub site: Option<Arc<str>>,
  /// The address of its picture.
  pub image: Option<Arc<str>>,
  /// The address of the site's icon: the first of [`icons`](Self::icons).
  pub icon: Option<Arc<str>>,
  /// Every icon the page names that a preview may draw, the best first: one of pixels with the size nearest the
  /// 16 of a preview's icon (32 on a screen of twice the pixels) before one of no size the page says, then the
  /// large icon of a phone's home screen, then an SVG (with the feature `canvas`; a build without it draws none).
  /// A preview draws the first that loads.
  pub icons: Vec<Arc<str>>,
  /// Whether the page is a video: it gave one, or said its kind is one.
  pub video: bool,
  /// How long its video is, in seconds, where it said.
  pub video_length: Option<f64>,
}

impl LinkMeta {
  /// What is known of `url` before its page is read: nothing but the address.
  pub fn new(url: &str) -> Self {
    Self {
      url: Arc::from(url),
      ..Self::default()
    }
  }

  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  pub fn site(mut self, site: &str) -> Self {
    self.site = Some(Arc::from(site));
    self
  }

  pub fn image(mut self, image: &str) -> Self {
    self.image = Some(Arc::from(image));
    self
  }

  /// The site's icon, first of its icons.
  pub fn icon(mut self, icon: &str) -> Self {
    let icon: Arc<str> = Arc::from(icon);
    self.icons.retain(|known| *known != icon);
    self.icons.insert(0, icon.clone());
    self.icon = Some(icon);
    self
  }

  /// The page is a video, `length` seconds long where that is known.
  pub fn video(mut self, length: impl Into<Option<f64>>) -> Self {
    self.video = true;
    self.video_length = length.into();
    self
  }

  /// Whether the page told nothing a preview shows: no title, no description and no picture.
  pub fn is_empty(&self) -> bool {
    self.title.is_none() && self.description.is_none() && self.image.is_none()
  }

  /// What the page at `url` says about itself, read from the start of its HTML (`bytes`, as many as were read).
  pub fn from_html(url: &str, bytes: &[u8]) -> Self {
    let text = decode(bytes);
    let mut found = Found::default();
    scan(&text, &mut found);
    let base = found
      .base
      .as_deref()
      .map(|base| resolve(url, base))
      .unwrap_or_else(|| url.to_owned());
    let whole = |address: Option<String>| -> Option<Arc<str>> {
      address
        .map(|address| address.trim().to_owned())
        .filter(|address| !address.is_empty())
        .map(|address| Arc::from(resolve(&base, &address)))
    };
    let words = |text: Option<String>| -> Option<Arc<str>> {
      text
        .map(|text| collapse(&text))
        .filter(|text| !text.is_empty())
        .map(Arc::from)
    };
    let video_length = found.video_length.as_deref().and_then(seconds_of);
    let mut icons: Vec<Arc<str>> = Vec::new();
    for link in ranked(found.icons) {
      if let Some(address) = whole(Some(link.href))
        && !icons.contains(&address)
      {
        icons.push(address);
      }
    }
    Self {
      url: Arc::from(url),
      title: words(found.og_title.or(found.twitter_title).or(found.title)),
      description: words(found.og_description.or(found.twitter_description).or(found.description)),
      site: words(found.site),
      image: whole(found.og_image.or(found.twitter_image)),
      icon: icons.first().cloned(),
      icons,
      video: found.video.is_some() || found.kind.as_deref().is_some_and(|kind| kind.starts_with("video")),
      video_length,
    }
  }
}

/// What the head said, by where it said it: the first of each is kept.
#[derive(Default)]
struct Found {
  title: Option<String>,
  og_title: Option<String>,
  twitter_title: Option<String>,
  description: Option<String>,
  og_description: Option<String>,
  twitter_description: Option<String>,
  site: Option<String>,
  og_image: Option<String>,
  twitter_image: Option<String>,
  icons: Vec<IconLink>,
  kind: Option<String>,
  video: Option<String>,
  video_length: Option<String>,
  base: Option<String>,
}

fn keep(slot: &mut Option<String>, value: Option<String>) {
  if slot.is_none()
    && let Some(value) = value.filter(|value| !value.trim().is_empty())
  {
    *slot = Some(value);
  }
}

impl Found {
  fn meta(&mut self, key: &str, content: Option<String>) {
    let slot = match key {
      "og:title" => &mut self.og_title,
      "twitter:title" => &mut self.twitter_title,
      "description" => &mut self.description,
      "og:description" => &mut self.og_description,
      "twitter:description" => &mut self.twitter_description,
      "og:site_name" => &mut self.site,
      "og:image" | "og:image:url" | "og:image:secure_url" => &mut self.og_image,
      "twitter:image" | "twitter:image:src" => &mut self.twitter_image,
      "og:type" => &mut self.kind,
      "og:video" | "og:video:url" | "og:video:secure_url" | "twitter:player" => &mut self.video,
      "og:video:duration" | "video:duration" | "duration" => &mut self.video_length,
      _ => return,
    };
    keep(slot, content);
  }
}

/// The side in pixels an icon is best drawn from: a preview's icon is 16, on a screen that may have twice the
/// pixels.
const ICON_PIXELS: u32 = 32;

/// An icon a page names: a `<link>` of `rel="icon"`, or of a phone's home screen.
struct IconLink {
  href: String,
  /// It is an SVG, by its type or its address.
  svg: bool,
  /// It is the icon of a phone's home screen (`apple-touch-icon`): large, on a ground of its own.
  touch: bool,
  /// The side of the size it names that is nearest [`ICON_PIXELS`]; none where it names none, or `any`.
  side: Option<u32>,
}

/// How far an icon of `side` pixels is from the ones it is drawn with: a smaller one is enlarged and blurs, so it
/// is twice as far.
fn distance(side: u32) -> u32 {
  if side >= ICON_PIXELS {
    side - ICON_PIXELS
  } else {
    (ICON_PIXELS - side) * 2
  }
}

/// Of the sizes a `sizes` attribute names (`16x16 32x32`), the longer side of the one nearest [`ICON_PIXELS`].
fn nearest_side(sizes: &str) -> Option<u32> {
  sizes
    .split_ascii_whitespace()
    .filter_map(|size| {
      let (width, height) = size.split_once(['x', 'X'])?;
      Some(width.parse::<u32>().ok()?.max(height.parse::<u32>().ok()?))
    })
    .filter(|side| *side > 0)
    .min_by_key(|side| distance(*side))
}

/// The icons a page names, the best to draw first: one of pixels of a size near the drawn one (16 to 96), then one
/// of no size the page says (an `.ico` holds several) or of another, then a phone's, the nearest first, then an
/// SVG, which lurq draws with the feature `canvas` and only in solid colors. A build without `canvas` has no SVG
/// in the list. The page's order decides between the alike.
fn ranked(mut icons: Vec<IconLink>) -> Vec<IconLink> {
  icons.retain(|icon| !icon.svg || cfg!(feature = "canvas"));
  icons.sort_by_key(|icon| match icon {
    IconLink { svg: true, .. } => (3, 0),
    IconLink { touch: true, side, .. } => (2, side.map_or(0, distance)),
    IconLink { side: Some(side), .. } if (16..=96).contains(side) => (0, distance(*side)),
    IconLink { side, .. } => (1, side.map_or(0, distance)),
  });
  icons
}

/// Walks the tags of `html` up to the end of its head, noting what [`Found`] keeps.
fn scan(html: &str, found: &mut Found) {
  let mut at = 0;
  while let Some(open) = html[at..].find('<').map(|offset| at + offset) {
    let rest = &html[open..];
    if rest.starts_with("<!--") {
      at = rest.find("-->").map_or(html.len(), |end| open + end + 3);
      continue;
    }
    let Some((name, attributes, end)) = tag(html, open) else {
      at = open + 1;
      continue;
    };
    at = end;
    match name.as_str() {
      "/head" | "body" => return,
      "script" | "style" | "noscript" | "template" => {
        at = closing(html, at, &name).unwrap_or(html.len());
      }
      "title" => {
        let close = closing_start(html, at, "title");
        let text = &html[at..close.unwrap_or(html.len())];
        keep(&mut found.title, Some(entities(text)));
        at = close.map_or(html.len(), |close| close + 1);
      }
      "meta" => {
        let content = attribute(&attributes, "content").map(|content| entities(&content));
        for key in ["property", "name", "itemprop"] {
          if let Some(key) = attribute(&attributes, key) {
            found.meta(&key.trim().to_ascii_lowercase(), content.clone());
          }
        }
      }
      "link" => {
        let rel = attribute(&attributes, "rel").unwrap_or_default().to_ascii_lowercase();
        let rels: Vec<&str> = rel.split_ascii_whitespace().collect();
        let touch = rels.iter().any(|rel| rel.starts_with("apple-touch-icon"));
        if (rels.contains(&"icon") || touch)
          && let Some(href) = attribute(&attributes, "href")
            .map(|href| entities(&href))
            .filter(|href| !href.trim().is_empty())
        {
          let kind = attribute(&attributes, "type").unwrap_or_default().to_ascii_lowercase();
          let path = href.split(['?', '#']).next().unwrap_or_default().to_ascii_lowercase();
          found.icons.push(IconLink {
            svg: kind.contains("svg") || path.ends_with(".svg"),
            touch,
            side: attribute(&attributes, "sizes").and_then(|sizes| nearest_side(&sizes)),
            href,
          });
        }
      }
      "base" => keep(
        &mut found.base,
        attribute(&attributes, "href").map(|href| entities(&href)),
      ),
      _ => {}
    }
  }
}

/// The tag that starts at `open` (`<name ...>`): its name in small letters, what follows the name, and where it
/// ends. `None` where `<` starts no tag.
fn tag(html: &str, open: usize) -> Option<(String, String, usize)> {
  let after = &html[open + 1..];
  let name_len = after
    .char_indices()
    .find(|(index, c)| !(c.is_ascii_alphanumeric() || (*index == 0 && (*c == '/' || *c == '!'))))
    .map_or(after.len(), |(index, _)| index);
  let name = after[..name_len].to_ascii_lowercase();
  if name.is_empty() || name == "/" || name == "!" {
    return None;
  }
  // The end of the tag: the first `>` outside a quoted value.
  let mut quote: Option<char> = None;
  let mut end = None;
  for (index, c) in after[name_len..].char_indices() {
    match (quote, c) {
      (Some(open), c) if c == open => quote = None,
      (Some(_), _) => {}
      (None, '"' | '\'') => quote = Some(c),
      (None, '>') => {
        end = Some(open + 1 + name_len + index);
        break;
      }
      _ => {}
    }
  }
  let end = end.unwrap_or(html.len());
  let attributes = html[open + 1 + name_len..end].to_owned();
  Some((name, attributes, (end + 1).min(html.len())))
}

/// Where the closing tag of `name` after `from` ends, past its `>`.
fn closing(html: &str, from: usize, name: &str) -> Option<usize> {
  let start = closing_start(html, from, name)?;
  Some(html[start..].find('>').map_or(html.len(), |end| start + end + 1))
}

/// Where the closing tag of `name` after `from` starts: its `<`.
fn closing_start(html: &str, from: usize, name: &str) -> Option<usize> {
  let needle = name.as_bytes();
  let mut at = from;
  while let Some(offset) = html[at..].find("</") {
    let start = at + offset;
    let after = &html.as_bytes()[start + 2..];
    if after.len() >= needle.len() && after[..needle.len()].eq_ignore_ascii_case(needle) {
      return Some(start);
    }
    at = start + 2;
  }
  None
}

/// The value of the attribute `name` in what follows a tag's name, as it is written: quoted with `"` or `'`, or
/// up to the next space.
fn attribute(attributes: &str, name: &str) -> Option<String> {
  let bytes = attributes.as_bytes();
  let mut at = 0;
  while at < bytes.len() {
    while at < bytes.len() && (bytes[at].is_ascii_whitespace() || bytes[at] == b'/') {
      at += 1;
    }
    let start = at;
    while at < bytes.len() && !bytes[at].is_ascii_whitespace() && bytes[at] != b'=' && bytes[at] != b'/' {
      at += 1;
    }
    let key = &attributes[start..at];
    while at < bytes.len() && bytes[at].is_ascii_whitespace() {
      at += 1;
    }
    let value = if bytes.get(at) == Some(&b'=') {
      at += 1;
      while at < bytes.len() && bytes[at].is_ascii_whitespace() {
        at += 1;
      }
      match bytes.get(at) {
        Some(&quote @ (b'"' | b'\'')) => {
          let from = at + 1;
          let to = attributes[from..]
            .find(quote as char)
            .map_or(attributes.len(), |offset| from + offset);
          at = (to + 1).min(attributes.len());
          Some(&attributes[from..to])
        }
        _ => {
          let from = at;
          while at < bytes.len() && !bytes[at].is_ascii_whitespace() {
            at += 1;
          }
          Some(&attributes[from..at])
        }
      }
    } else {
      None
    };
    if key.is_empty() {
      at += 1;
      continue;
    }
    if key.eq_ignore_ascii_case(name) {
      return Some(value.unwrap_or("").to_owned());
    }
  }
  None
}

/// `text` with its character references read: `&amp;`, `&#39;`, `&#x2014;` and the named ones a title is likely
/// to hold. One it does not know is left as it is.
fn entities(text: &str) -> String {
  if !text.contains('&') {
    return text.to_owned();
  }
  let mut out = String::with_capacity(text.len());
  let mut rest = text;
  while let Some(amp) = rest.find('&') {
    out.push_str(&rest[..amp]);
    rest = &rest[amp..];
    let end = rest[1..]
      .char_indices()
      .take(32)
      .find(|(_, c)| *c == ';' || c.is_whitespace() || *c == '&')
      .map(|(index, c)| (index + 1, c));
    let Some((end, ';')) = end else {
      out.push('&');
      rest = &rest[1..];
      continue;
    };
    let name = &rest[1..end];
    let known = if let Some(number) = name.strip_prefix('#') {
      let code = match number.strip_prefix(['x', 'X']) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => number.parse::<u32>().ok(),
      };
      code.and_then(char::from_u32)
    } else {
      named(name)
    };
    match known {
      Some(c) => {
        out.push(c);
        rest = &rest[end + 1..];
      }
      None => {
        out.push('&');
        rest = &rest[1..];
      }
    }
  }
  out.push_str(rest);
  out
}

fn named(name: &str) -> Option<char> {
  Some(match name {
    "amp" => '&',
    "lt" => '<',
    "gt" => '>',
    "quot" => '"',
    "apos" => '\'',
    "nbsp" => '\u{a0}',
    "ndash" => '–',
    "mdash" => '—',
    "hellip" => '…',
    "lsquo" => '‘',
    "rsquo" => '’',
    "ldquo" => '“',
    "rdquo" => '”',
    "laquo" => '«',
    "raquo" => '»',
    "middot" => '·',
    "bull" => '•',
    "copy" => '©',
    "reg" => '®',
    "trade" => '™',
    "times" => '×',
    _ => return None,
  })
}

/// `text` on one line: runs of white space as one space, none at the ends.
fn collapse(text: &str) -> String {
  text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A video's length as a page writes it: whole seconds (`250`), or an ISO 8601 duration (`PT4M10S`).
fn seconds_of(text: &str) -> Option<f64> {
  let text = text.trim();
  if let Ok(seconds) = text.parse::<f64>() {
    return (seconds.is_finite() && seconds >= 0.0).then_some(seconds);
  }
  let rest = text.strip_prefix(['P', 'p'])?;
  let (days, time) = match rest.split_once(['T', 't']) {
    Some((days, time)) => (days, time),
    None => (rest, ""),
  };
  let mut total = 0.0;
  let mut add = |part: &str, units: &[(char, f64)]| -> Option<()> {
    let mut number = String::new();
    for c in part.chars() {
      if c.is_ascii_digit() || c == '.' {
        number.push(c);
      } else {
        let unit = units.iter().find(|(name, _)| name.eq_ignore_ascii_case(&c))?;
        total += number.parse::<f64>().ok()? * unit.1;
        number.clear();
      }
    }
    number.is_empty().then_some(())
  };
  add(days, &[('D', 86_400.0), ('W', 604_800.0)])?;
  add(time, &[('H', 3_600.0), ('M', 60.0), ('S', 1.0)])?;
  Some(total)
}

/// `address` made whole from `base`, the address of the page it is on: an address with a scheme is as it is, one
/// that starts with `//` takes the base's scheme, one with `/` its origin, and any other its folder.
pub(crate) fn resolve(base: &str, address: &str) -> String {
  let address = address.trim();
  let has_scheme = address.split_once(':').is_some_and(|(scheme, _)| {
    !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
  });
  if has_scheme {
    return address.to_owned();
  }
  let Some((scheme, after)) = base.split_once("://") else {
    return address.to_owned();
  };
  if let Some(rest) = address.strip_prefix("//") {
    return format!("{scheme}://{rest}");
  }
  let host_end = after.find(['/', '?', '#']).unwrap_or(after.len());
  let origin = format!("{scheme}://{}", &after[..host_end]);
  let path_and_more = &after[host_end..];
  let path = &path_and_more[..path_and_more.find(['?', '#']).unwrap_or(path_and_more.len())];
  if address.is_empty() {
    return base.to_owned();
  }
  if address.starts_with('#') {
    return format!(
      "{origin}{}{address}",
      &path_and_more[..path_and_more.find('#').unwrap_or(path_and_more.len())]
    );
  }
  if address.starts_with('?') {
    return format!("{origin}{path}{address}");
  }
  let joined = if address.starts_with('/') {
    address.to_owned()
  } else {
    let folder = &path[..path.rfind('/').map_or(0, |slash| slash + 1)];
    let folder = if folder.is_empty() { "/" } else { folder };
    format!("{folder}{address}")
  };
  // The path's `.` and `..` read, its query and fragment left as they are.
  let split = joined.find(['?', '#']).unwrap_or(joined.len());
  let (path, tail) = joined.split_at(split);
  let mut segments: Vec<&str> = Vec::new();
  let parts: Vec<&str> = path.split('/').skip(1).collect();
  for (index, segment) in parts.iter().enumerate() {
    let last = index + 1 == parts.len();
    match *segment {
      "." => {
        if last {
          segments.push("");
        }
      }
      ".." => {
        segments.pop();
        if last {
          segments.push("");
        }
      }
      segment => segments.push(segment),
    }
  }
  format!("{origin}/{}{tail}", segments.join("/"))
}

/// The text of `bytes`: UTF-8 unless a byte-order mark or a `<meta charset>` in its first kilobytes names another
/// that is known here. A byte that is no UTF-8 is a replacement character.
fn decode(bytes: &[u8]) -> String {
  if let Some(rest) = bytes.strip_prefix(b"\xEF\xBB\xBF") {
    return String::from_utf8_lossy(rest).into_owned();
  }
  match charset(&bytes[..bytes.len().min(4096)]).as_deref() {
    Some("windows-1252" | "iso-8859-1" | "latin1" | "iso8859-1" | "cp1252" | "us-ascii" | "ascii") => {
      bytes.iter().map(|&byte| windows_1252(byte)).collect()
    }
    Some("windows-1251" | "cp1251") => bytes.iter().map(|&byte| windows_1251(byte)).collect(),
    Some("koi8-r") => bytes.iter().map(|&byte| koi8(byte, false)).collect(),
    Some("koi8-u") => bytes.iter().map(|&byte| koi8(byte, true)).collect(),
    _ => String::from_utf8_lossy(bytes).into_owned(),
  }
}

/// The charset the start of a page names, in small letters: `<meta charset="..">`, or the `charset=` of a
/// `<meta http-equiv="Content-Type" content="..">`.
fn charset(start: &[u8]) -> Option<String> {
  let ascii: String = start
    .iter()
    .map(|&byte| {
      if byte.is_ascii() {
        byte.to_ascii_lowercase() as char
      } else {
        ' '
      }
    })
    .collect();
  let mut at = 0;
  while let Some(offset) = ascii[at..].find("<meta") {
    let open = at + offset;
    let (_, attributes, end) = tag(&ascii, open)?;
    at = end;
    let named = attribute(&attributes, "charset").or_else(|| {
      let content = attribute(&attributes, "content")?;
      let (_, after) = content.split_once("charset=")?;
      Some(after.to_owned())
    });
    if let Some(name) = named {
      let name = name
        .trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == ';')
        .to_owned();
      if !name.is_empty() {
        return Some(name);
      }
    }
  }
  None
}

fn windows_1252(byte: u8) -> char {
  const HIGH: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}', '\u{90}', '‘', '’',
    '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
  ];
  match byte {
    0x80..=0x9F => HIGH[usize::from(byte - 0x80)],
    _ => char::from(byte),
  }
}

fn windows_1251(byte: u8) -> char {
  const HIGH: [char; 64] = [
    'Ђ', 'Ѓ', '‚', 'ѓ', '„', '…', '†', '‡', '€', '‰', 'Љ', '‹', 'Њ', 'Ќ', 'Ћ', 'Џ', 'ђ', '‘', '’', '“', '”', '•', '–',
    '—', '\u{98}', '™', 'љ', '›', 'њ', 'ќ', 'ћ', 'џ', '\u{a0}', 'Ў', 'ў', 'Ј', '¤', 'Ґ', '¦', '§', 'Ё', '©', 'Є', '«',
    '¬', '\u{ad}', '®', 'Ї', '°', '±', 'І', 'і', 'ґ', 'µ', '¶', '·', 'ё', '№', 'є', '»', 'ј', 'Ѕ', 'ѕ', 'ї',
  ];
  match byte {
    0x80..=0xBF => HIGH[usize::from(byte - 0x80)],
    // А..я follow one another from 0xC0.
    0xC0..=0xFF => char::from_u32(0x0410 + u32::from(byte - 0xC0)).unwrap_or('\u{fffd}'),
    _ => char::from(byte),
  }
}

/// KOI8-R, and KOI8-U where `ukrainian`: the Cyrillic letters, in the order of their Latin sounds, from 0xC0.
/// The lines and blocks of 0x80 to 0xBF are a replacement character, but for KOI8-U's letters and the few signs
/// a title holds.
fn koi8(byte: u8, ukrainian: bool) -> char {
  const LETTERS: &str = "юабцдефгхийклмнопярстужвьызшэщчъЮАБЦДЕФГХИЙКЛМНОПЯРСТУЖВЬЫЗШЭЩЧЪ";
  match byte {
    0xC0..=0xFF => LETTERS.chars().nth(usize::from(byte - 0xC0)).unwrap_or('\u{fffd}'),
    0xA3 => 'ё',
    0xB3 => 'Ё',
    0x9A => '\u{a0}',
    0xBF => '©',
    0xA4 if ukrainian => 'є',
    0xA6 if ukrainian => 'і',
    0xA7 if ukrainian => 'ї',
    0xAD if ukrainian => 'ґ',
    0xB4 if ukrainian => 'Є',
    0xB6 if ukrainian => 'І',
    0xB7 if ukrainian => 'Ї',
    0xBD if ukrainian => 'Ґ',
    0x80..=0xBF => '\u{fffd}',
    _ => char::from(byte),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  macro_rules! page {
    ($name:literal) => {
      include_bytes!(concat!("../../tests/assets/link_preview/", $name))
    };
  }

  #[test]
  fn opengraph_comes_before_twitter_and_the_title_and_the_body_is_not_read() {
    let meta = LinkMeta::from_html("https://lurq.dev/docs/layout/rows", page!("opengraph.html"));
    assert_eq!(
      meta.title.as_deref(),
      Some("Layout in lurq: rows, columns and the space between")
    );
    assert_eq!(
      meta.description.as_deref(),
      Some("How a row shares its width, what a gap is, and when a child is let to grow.")
    );
    assert_eq!(meta.site.as_deref(), Some("lurq.dev"));
    assert_eq!(meta.image.as_deref(), Some("https://lurq.dev/docs/shots/rows.png?v=2"));
    // An icon of `rel="shortcut icon"` comes before the touch icon.
    assert_eq!(meta.icon.as_deref(), Some("https://lurq.dev/favicon.ico"));
    assert!(!meta.video);
    assert!(!meta.is_empty());
  }

  #[test]
  fn a_card_in_capitals_without_quotes_with_references_and_a_base() {
    let meta = LinkMeta::from_html("https://example.com/post/42", page!("twitter.html"));
    assert_eq!(meta.title.as_deref(), Some("Rock & roll — a \"card\" — © 2026"));
    assert_eq!(meta.description.as_deref(), Some("Spread over lines"));
    assert_eq!(meta.image.as_deref(), Some("https://cdn.example.com/card.jpg"));
    assert_eq!(meta.icon.as_deref(), Some("https://example.com/blog/icons/site.png"));
    assert_eq!(meta.site, None);
  }

  #[test]
  fn githubs_page_gives_its_picture_and_its_png_icon_before_its_svg_one() {
    let meta = LinkMeta::from_html("https://github.com/ivanzaida/lurq", page!("github.html"));
    assert_eq!(meta.title.as_deref(), Some("GitHub - ivanzaida/lurq"));
    assert_eq!(meta.site.as_deref(), Some("GitHub"));
    assert_eq!(
      meta.image.as_deref(),
      Some(
        "https://opengraph.githubassets.com/2d62901bfc9f6e2966648a4a1f90879fbc91ac9f4e750c54aa814d1061b81963/\
         ivanzaida/lurq"
      )
    );
    // `alternate icon` is an icon; `mask-icon` and `fluid-icon` are not.
    let mut icons = vec!["https://github.githubassets.com/favicons/favicon.png"];
    if cfg!(feature = "canvas") {
      icons.push("https://github.githubassets.com/favicons/favicon.svg");
    }
    assert_eq!(meta.icons.iter().map(|icon| &**icon).collect::<Vec<_>>(), icons);
    assert_eq!(meta.icon.as_deref(), Some(icons[0]));
  }

  #[test]
  fn the_icon_of_the_size_nearest_the_drawn_one_comes_first_and_an_svg_alone_is_kept() {
    let page = br#"<head>
      <link rel="apple-touch-icon" href="/touch.png" sizes="180x180">
      <link rel="icon" href="/icon.svg" type="image/svg+xml">
      <link rel="icon" href="/favicon.ico">
      <link rel="icon" href="/big.png" sizes="192x192">
      <link rel="icon" href="/small.png" sizes="16x16">
      <link rel="icon" href="/sharp.png" sizes="16x16 32x32">
      <link rel="icon" href="">
    </head>"#;
    let meta = LinkMeta::from_html("https://a.example/x", page);
    let mut icons = vec![
      "https://a.example/sharp.png",
      "https://a.example/small.png",
      "https://a.example/favicon.ico",
      "https://a.example/big.png",
      "https://a.example/touch.png",
    ];
    if cfg!(feature = "canvas") {
      icons.push("https://a.example/icon.svg");
    }
    assert_eq!(meta.icons.iter().map(|icon| &**icon).collect::<Vec<_>>(), icons);
    let svg_only = LinkMeta::from_html("https://a.example/", br#"<link rel=icon href="logo.svg?v=3">"#);
    assert_eq!(
      svg_only.icon.as_deref(),
      cfg!(feature = "canvas").then_some("https://a.example/logo.svg?v=3")
    );
    // An icon the app names is first of them.
    let named = LinkMeta::new("https://a.example/").icon("https://a.example/mine.png");
    assert_eq!(named.icons, [Arc::from("https://a.example/mine.png")]);
  }

  #[test]
  fn a_title_and_a_description_alone_and_a_head_that_never_ends() {
    let meta = LinkMeta::from_html("http://plain.example/", page!("plain.html"));
    assert_eq!(meta.title.as_deref(), Some("Just a title"));
    assert_eq!(meta.description.as_deref(), Some("And a line."));
    assert_eq!(meta.image, None, "what is in the body is not read");
  }

  #[test]
  fn a_video_and_its_length_in_seconds_or_as_a_duration() {
    let meta = LinkMeta::from_html("https://videos.example.com/watch?v=1", page!("video.html"));
    assert!(meta.video);
    assert_eq!(meta.video_length, Some(250.0));
    assert_eq!(meta.image.as_deref(), Some("https://videos.example.com/gallery.jpg"));
    let long = LinkMeta::from_html("https://videos.example.com/long", page!("iso_duration.html"));
    assert!(long.video, "a page whose kind is a video is one");
    assert_eq!(long.video_length, Some(3725.0));
  }

  #[test]
  fn a_page_that_tells_nothing_is_empty() {
    let meta = LinkMeta::from_html("https://lurq.dev/x", page!("nothing.html"));
    assert!(meta.is_empty());
    assert_eq!(&*meta.url, "https://lurq.dev/x");
    assert!(LinkMeta::from_html("https://lurq.dev/x", b"").is_empty());
    assert!(LinkMeta::from_html("https://lurq.dev/x", b"\x89PNG\r\n\x1a\n<<<>>> not a page").is_empty());
  }

  #[test]
  fn the_charsets_a_meta_names_are_read() {
    let cyrillic = LinkMeta::from_html("https://a.example/", page!("windows-1251.html"));
    assert_eq!(cyrillic.title.as_deref(), Some("Розмітка у lurq — Їжак і Ґанок"));
    let koi = LinkMeta::from_html("https://a.example/", page!("koi8-u.html"));
    assert_eq!(koi.title.as_deref(), Some("Привіт, Їжаче"));
    let western = LinkMeta::from_html("https://a.example/", page!("windows-1252.html"));
    assert_eq!(western.title.as_deref(), Some("Café “quoted” €5"));
    // UTF-8 with a byte that is none is read with a replacement character, and the rest is kept.
    let broken = LinkMeta::from_html("https://a.example/", b"<title>ok \xFF ok</title>");
    assert_eq!(broken.title.as_deref(), Some("ok \u{fffd} ok"));
  }

  #[test]
  fn an_address_is_made_whole_from_the_page() {
    let base = "https://lurq.dev/docs/layout/rows?x=1#top";
    for (address, whole) in [
      ("https://cdn.example/a.png", "https://cdn.example/a.png"),
      ("//cdn.example/a.png", "https://cdn.example/a.png"),
      ("/a.png", "https://lurq.dev/a.png"),
      ("a.png", "https://lurq.dev/docs/layout/a.png"),
      ("./a.png", "https://lurq.dev/docs/layout/a.png"),
      ("../../a.png", "https://lurq.dev/a.png"),
      ("../../../../a.png", "https://lurq.dev/a.png"),
      ("?y=2", "https://lurq.dev/docs/layout/rows?y=2"),
      ("data:image/png;base64,AA", "data:image/png;base64,AA"),
    ] {
      assert_eq!(resolve(base, address), whole, "{address}");
    }
    assert_eq!(resolve("https://lurq.dev", "a.png"), "https://lurq.dev/a.png");
  }

  #[test]
  fn a_tag_cut_by_the_end_of_what_was_read_is_no_harm() {
    let meta = LinkMeta::from_html(
      "https://a.example/",
      b"<head><title>Cut</title><meta property=\"og:descr",
    );
    assert_eq!(meta.title.as_deref(), Some("Cut"));
    let meta = LinkMeta::from_html("https://a.example/", b"<head><title>Never closed");
    assert_eq!(meta.title.as_deref(), Some("Never closed"));
  }
}
