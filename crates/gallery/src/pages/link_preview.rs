//! The page of the link preview: its playground, the board's card, row and inline link, the states of each, and
//! an inline one in a sentence.
//!
//! The gallery fetches nothing from the network unless asked: its previews are given a fetcher of the page's own
//! (`fake`), which answers for a few addresses at once, as an app's own server would. The playground has a switch
//! that gives the preview lurq's own fetcher, which reads the page at the address for real.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, TypographyStyle},
  },
  components::{Column, Row, Text},
  core::Signal,
  images::ImageData,
  layout::{Alignment, quad::RichTextSpan, text_style::TextStyle},
  node::Element,
  resources::{LinkFetch, LinkMeta, LinkPreviewError},
  ui::{Interaction, Length, LinkPreview, LinkPreviewVariant, Picture, current_theme},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The addresses the page's fetcher knows.
const LAYOUT: &str = "https://lurq.dev/docs/layout/rows-and-columns";
const VIDEO: &str = "https://lurq.dev/guides/a-gallery-page";
const SLOW: &str = "https://lurq.dev/docs/being-read";
const BARE: &str = "https://lurq.dev/docs/layout/gaps";
const MISSING: &str = "https://lurq.dev/docs/gone";

/// The pages the playground offers: what each is called, and its address.
const PAGES: [(&str, &str); 5] = [
  ("A guide", LAYOUT),
  ("A video", VIDEO),
  ("Being read", SLOW),
  ("Tells nothing", BARE),
  ("Not found", MISSING),
];

/// The forms, as the board names them and as the code does.
const VARIANTS: [(LinkPreviewVariant, &str, &str); 3] = [
  (LinkPreviewVariant::Card, "card", "LinkPreviewVariant::Card"),
  (LinkPreviewVariant::Compact, "compact", "LinkPreviewVariant::Compact"),
  (LinkPreviewVariant::Inline, "inline", "LinkPreviewVariant::Inline"),
];

/// What the page's own fetcher answers for `url`: what the board's pages say about themselves, a page that is
/// read for ever, one that tells nothing and one that is not there. It answers at once, as a fetcher that holds
/// what it knows does.
fn fake(url: &str) -> LinkFetch {
  match url {
    LAYOUT => LinkFetch::ready(
      LinkMeta::new(url)
        .title("Layout in lurq: rows, columns and the space between")
        .description("How a row shares its width, what a gap is, and when a child is let to grow.")
        .site("lurq.dev"),
    ),
    VIDEO => LinkFetch::ready(
      LinkMeta::new(url)
        .title("A gallery page in ten minutes")
        .description("From an empty file to a page with a playground.")
        .site("lurq.dev")
        .video(250.0),
    ),
    SLOW => LinkFetch::Loading,
    MISSING => LinkFetch::Failed(LinkPreviewError::NotFound),
    _ => LinkFetch::ready(LinkMeta::new(url)),
  }
}

/// The site's icon the board draws: a square of the primary color with a mark of the foreground's on it.
fn favicon() -> ImageData {
  let theme = current_theme();
  let (fill, mark) = (
    theme.palette().get(PaletteColor::Primary),
    theme.palette().get(PaletteColor::PrimaryForeground),
  );
  let side = 32u32;
  let mut rgba = Vec::with_capacity((side * side * 4) as usize);
  for y in 0..side {
    for x in 0..side {
      // A bolt: a slanted band across the middle.
      let (fx, fy) = (x as f32 / side as f32, y as f32 / side as f32);
      let on = (0.2..0.8).contains(&fy) && ((fx + fy * 0.5) - 0.75).abs() < 0.09;
      let color = if on { mark } else { fill };
      rgba.extend_from_slice(&[color.r(), color.g(), color.b(), 255]);
    }
  }
  ImageData::from_rgba(rgba, side, side)
}

/// A preview of `url` with the page's fetcher, and the gallery's pictures where the board draws them: a page on
/// the web gives its own.
fn preview(url: &str, pictures: bool) -> LinkPreview {
  let mut preview = LinkPreview::new(url).fetcher(fake);
  if pictures && matches!(url, LAYOUT | VIDEO) {
    preview = preview
      .image(Picture::new(super::picture::lake(640, 336)))
      .icon(Picture::new(favicon()));
  }
  preview
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(10.0)
    .overflow_visible()
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(24.0)
    .wrap()
    .overflow_visible()
    .align_items(Alignment::Start)
}

/// A preview at the width the board draws it, and no wider than what holds it. It cuts nothing off: its shadow
/// and its ring fall outside it.
fn held(width: f32, preview: LinkPreview) -> Column {
  Column::new()
    .width(width)
    .max_width(FILL)
    .overflow_visible()
    .child(preview.width(Length::Fill))
}

/// The board's sentence with an inline preview in it.
fn sentence() -> Text {
  let theme = current_theme();
  let mut body: TextStyle = theme.typography_style(TypographyStyle::Body);
  body.color = theme.palette().get(PaletteColor::Foreground);
  Text::rich([
    RichTextSpan::new("The gaps are explained in ", body.clone()),
    RichTextSpan::object(body.clone()),
    RichTextSpan::new(" — read it before the review.", body),
  ])
  .object(
    preview(LAYOUT, true)
      .variant(LinkPreviewVariant::Inline)
      .title("Layout in lurq"),
  )
}

pub struct LinkPreviewPage {
  frame: Frame,
  playground: Playground,
  page: Signal<usize>,
  variant: Signal<usize>,
  title: Signal<String>,
  description: Signal<String>,
  pictures: Signal<bool>,
  removable: Signal<bool>,
  decides: Signal<bool>,
  disabled: Signal<bool>,
  real: Signal<bool>,
  address: Signal<String>,
}

impl LinkPreviewPage {
  /// The playground: the preview as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (page_name, fake_url) = PAGES[self.page.get().min(PAGES.len() - 1)];
    let (variant, variant_name, variant_code) = VARIANTS[self.variant.get().min(VARIANTS.len() - 1)];
    let (title, description) = (self.title.get(), self.description.get());
    let (pictures, removable, decides, disabled) = (
      self.pictures.get(),
      self.removable.get(),
      self.decides.get(),
      self.disabled.get(),
    );
    let (real, address) = (self.real.get(), self.address.get());
    let url = if real { address.clone() } else { fake_url.to_owned() };
    let plain = {
      let (title, description, url) = (title.clone(), description.clone(), url.clone());
      move || {
        let mut preview = if real {
          LinkPreview::new(&url)
        } else {
          preview(&url, pictures)
        };
        preview = preview.variant(variant).disabled(disabled);
        if !title.is_empty() {
          preview = preview.title(&title);
        }
        if !description.is_empty() {
          preview = preview.description(&description);
        }
        preview
      }
    };
    let record = self.playground.recorder();
    let mut shown = plain().force(self.playground.forced());
    if decides {
      let record = record.clone();
      shown = shown.on_open(move |url| record(format!("on_open({url:?})")));
    }
    if removable {
      shown = shown.on_remove(move || record("on_remove".to_owned()));
    }
    let make = {
      let plain = plain.clone();
      move || {
        let mut preview = plain();
        if decides {
          preview = preview.on_open(|_| ());
        }
        if removable {
          preview = preview.on_remove(|| ());
        }
        preview.into()
      }
    };

    let mut code = vec![format!("LinkPreview::new({url:?})")];
    if !title.is_empty() {
      code.push(format!("    .title({title:?})"));
    }
    if !description.is_empty() {
      code.push(format!("    .description({description:?})"));
    }
    if variant != LinkPreviewVariant::Card {
      code.push(format!("    .variant({variant_code})"));
    }
    if !real {
      code.push("    // The gallery's own pages: lurq's fetcher reads the page at the address.".to_owned());
      code.push("    .fetcher(pages)".to_owned());
      if pictures && matches!(fake_url, LAYOUT | VIDEO) {
        code.push("    .image(Picture::new(lake))".to_owned());
        code.push("    .icon(Picture::new(favicon))".to_owned());
      }
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    if decides {
      code.push("    .on_open(|url| go_to(url))".to_owned());
    }
    if removable {
      code.push("    .on_remove(move || draft.drop_preview())".to_owned());
    }

    let mut tree = vec![format!(
      "root        {}   {variant_name} · one link{}",
      self.playground.measured(),
      if removable { " · removable" } else { "" }
    )];
    match variant {
      LinkPreviewVariant::Card => {
        tree.push("├─ media     the picture, 1.91 : 1 · play and length of a video".to_owned());
        tree.push("└─ text      site (icon · domain) · title · description".to_owned());
      }
      LinkPreviewVariant::Compact => {
        tree.push("├─ text      site (icon · domain) · title · description".to_owned());
        tree.push("└─ media     the picture, 72 square".to_owned());
      }
      LinkPreviewVariant::Inline => tree.push("└─ icon · title · domain".to_owned()),
    }
    if removable {
      tree.push("   close     its own stop of Tab".to_owned());
    }

    let (to_page, to_variant) = (self.page.clone(), self.variant.clone());
    let pages: Vec<(&str, usize)> = PAGES.iter().enumerate().map(|(index, page)| (page.0, index)).collect();
    let variants: Vec<(&str, usize)> = VARIANTS
      .iter()
      .enumerate()
      .map(|(index, variant)| (variant.1, index))
      .collect();
    let mut controls = vec![
      prop(
        "page",
        "fetched",
        self
          .playground
          .select(0, &pages, &self.page.get(), move |index| to_page.set(index)),
      ),
      prop(
        "variant",
        "LinkPreviewVariant",
        segmented_fill(&variants, &self.variant.get(), move |index| to_variant.set(index)),
      ),
      prop("title", "&str", field(&self.title)),
      prop("description", "&str", field(&self.description)),
      divider(),
      flag("image / icon", &self.pictures),
      flag("on_open", &self.decides),
      flag("on_remove", &self.removable),
      flag("disabled", &self.disabled),
      divider(),
      flag("fetch for real", &self.real),
    ];
    if real {
      controls.push(prop("url", "&str", field(&self.address)));
    }
    let reset = {
      let (page, variant) = (self.page.clone(), self.variant.clone());
      let (title, description, address) = (self.title.clone(), self.description.clone(), self.address.clone());
      let (pictures, removable, decides, disabled, real) = (
        self.pictures.clone(),
        self.removable.clone(),
        self.decides.clone(),
        self.disabled.clone(),
        self.real.clone(),
      );
      Arc::new(move || {
        page.set(0);
        variant.set(0);
        title.set(String::new());
        description.set(String::new());
        address.set("https://github.com/ivanzaida/lurq".to_owned());
        pictures.set(true);
        removable.set(false);
        decides.set(true);
        disabled.set(false);
        real.set(false);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Link Preview",
        make: Arc::new(make),
        shown: Shown::of(self.playground.styled(shown)),
        fill: false,
        forceable: true,
        facts: format!("{variant_name} · {page_name}"),
        event: "on_open",
        code,
        tree,
        controls,
        changed: [
          self.page.get() != 0,
          variant != LinkPreviewVariant::Card,
          !title.is_empty(),
          !description.is_empty(),
          !pictures,
          !decides,
          removable,
          disabled,
          real,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }
}

impl Component for LinkPreviewPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      page: ctx.signal(0),
      variant: ctx.signal(0),
      title: ctx.signal(String::new()),
      description: ctx.signal(String::new()),
      pictures: ctx.signal(true),
      removable: ctx.signal(false),
      decides: ctx.signal(true),
      disabled: ctx.signal(false),
      real: ctx.signal(false),
      address: ctx.signal("https://github.com/ivanzaida/lurq".to_owned()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let card = |url: &str| preview(url, true);
    let compact = |url: &str| preview(url, true).variant(LinkPreviewVariant::Compact);
    let inline = |url: &str| {
      preview(url, true)
        .variant(LinkPreviewVariant::Inline)
        .title("Layout in lurq")
    };
    let large = items()
      .child(item(
        held(360.0, card(LAYOUT)),
        "link preview · 360 · picture, site, title, a line",
      ))
      .child(
        Column::new()
          .max_width(FILL)
          .spacing(24.0)
          .overflow_visible()
          .child(item(
            held(
              432.0,
              compact(LAYOUT)
                .title("Layout in lurq: rows, columns and gaps")
                .description("How a row shares its width."),
            ),
            "compact · a row, the picture a 72 square",
          ))
          .child(item(inline(LAYOUT), "inline · in a sentence, 24 high")),
      );
    let states = items()
      .child(item(held(280.0, card(SLOW)), "loading · the skeleton"))
      .child(item(
        held(280.0, card(LAYOUT).no_image()),
        "no picture · the text alone",
      ))
      .child(item(held(280.0, card(VIDEO)), "a video · the mark and its length"))
      .child(item(
        held(280.0, card(LAYOUT).force(Some(Interaction::Hovered))),
        "hover · lifted, the title underlined",
      ))
      .child(item(
        held(280.0, card(LAYOUT).force(Some(Interaction::FocusVisible))),
        "focus-visible · a ring",
      ))
      .child(item(
        held(280.0, card(LAYOUT).on_remove(|| ())),
        "in a composer · a close drops it",
      ));
    let row = |preview: LinkPreview, forced: Option<Interaction>| {
      held(
        432.0,
        preview
          .title("Layout in lurq: rows, columns and gaps")
          .description("How a row shares its width.")
          .force(forced),
      )
    };
    let rows = items()
      .child(item(held(432.0, compact(SLOW)), "compact · loading"))
      .child(item(
        held(
          432.0,
          compact(LAYOUT)
            .no_image()
            .description("How a row shares its width, and what a gap is."),
        ),
        "compact · no picture, the text takes the row",
      ))
      .child(item(
        held(432.0, compact(BARE)),
        "nothing to show · the address, a link mark",
      ))
      .child(item(
        row(compact(LAYOUT), Some(Interaction::Hovered)),
        "compact · hover",
      ))
      .child(item(
        row(compact(LAYOUT), Some(Interaction::FocusVisible)),
        "compact · focus-visible",
      ))
      .child(item(
        row(compact(LAYOUT).on_remove(|| ()), None),
        "compact · in a composer",
      ));
    let inlines = items()
      .child(item(inline(LAYOUT), "inline · at rest"))
      .child(item(inline(LAYOUT).force(Some(Interaction::Hovered)), "hover"))
      .child(item(
        inline(LAYOUT).force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(preview(SLOW, true).variant(LinkPreviewVariant::Inline), "loading"))
      .child(item(inline(LAYOUT).no_icon(), "no icon · a link mark"))
      .child(item(inline(LAYOUT).no_site(), "the title alone"));
    let in_a_sentence = Column::new()
      .width(FILL)
      .spacing(10.0)
      .overflow_visible()
      .child(sentence())
      .child(quiet("in a sentence · it keeps the line's height", 12.0));
    let rows_of_props = [
      (
        "new",
        "&str",
        "",
        "The address. What the page says about itself is fetched and shown: the skeleton while it is read.",
      ),
      (
        "title / description / site",
        "&str",
        "the page's",
        "Shown in place of the page's, one by one; the rest is still fetched. The site is the domain where \
         neither names one.",
      ),
      (
        "image / icon",
        "impl Into<LinkImage>",
        "the page's",
        "An address, a Source or a Picture, in place of the page's picture or the site's icon.",
      ),
      (
        "no_image / no_icon / no_site",
        "",
        "",
        "None of it, whatever the page has: the text alone, the link mark, the title alone.",
      ),
      (
        "video",
        "impl Into<Option<f64>>",
        "the page's",
        "A video of so many seconds: the play mark and its length over the picture.",
      ),
      (
        "variant",
        "LinkPreviewVariant",
        "Card",
        "Card: 360, the picture over the text. Compact: a row of 432 with a picture of 72. Inline: 24 high, in \
         a sentence.",
      ),
      (
        "fetcher",
        "impl LinkFetcher",
        "lurq's own",
        "What fetches the page for this preview: any Fn(&str) -> LinkFetch. set_link_fetcher gives every \
         preview one.",
      ),
      (
        "on_open",
        "impl Fn(&str)",
        "opens it",
        "Told the address on a click, Enter or Space, in place of opening it in the browser.",
      ),
      (
        "on_remove",
        "impl Fn()",
        "",
        "Gives it a close button, a stop of Tab of its own: the app drops the preview.",
      ),
      (
        "width",
        "impl Into<Length>",
        "the form's",
        "A width of its own; it is never wider than its room.",
      ),
      ("loading", "bool", "false", "The skeleton, whatever is known."),
      (
        "disabled",
        "bool",
        "false",
        "Dimmed; opens nothing and is no stop of Tab.",
      ),
      (
        "label",
        "&str",
        "the title",
        "What it is called to tooling, which reads one link described by its site and description.",
      ),
      (
        "no_preview_text / remove_label",
        "&str",
        "the locale's",
        "\"No preview for this page.\" and \"Remove preview\", from the locale unless told.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Media", "Link Preview"],
        title: "Link Preview",
        description: "What a link says about itself, shown before it is followed: the site, the title, a line of \
                      description and its picture. The whole preview is one link. While the page is being read it \
                      is a skeleton; a page with no picture is the text alone, and a page that tells nothing is \
                      its address. The compact one is a row for lists and messages, the inline one sits in a \
                      sentence.",
        import: Some("use lurq::ui::LinkPreview;"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The page is one the gallery's own fetcher knows: nothing goes to the network. Fetch for real gives the \
           preview lurq's own fetcher, which reads the address in the field. A title or a description typed in \
           is shown over the page's.",
          playground,
        ),
        Section::new(
          "Link Preview",
          "The card of 360, the compact row of 432 and the inline link of 24, each one link.",
          self.frame.demo(
            0,
            large,
            "LinkPreview::new(url)",
            &[
              "LinkPreview::new(\"https://lurq.dev/docs/layout/rows-and-columns\")",
              "",
              "LinkPreview::new(url).variant(LinkPreviewVariant::Compact)",
              "LinkPreview::new(url).variant(LinkPreviewVariant::Inline).title(\"Layout in lurq\")",
            ],
          ),
        ),
        Section::new(
          "States",
          "While the page is read, the skeleton. A page with no picture is its text; a video has the play mark and \
           its length. Under the pointer it lifts and its title is underlined; from the keyboard it has the ring. \
           In a composer, a close drops it.",
          self.frame.demo(
            1,
            states,
            ".on_remove(move || draft.drop_preview())",
            &[
              "LinkPreview::new(url).no_image()",
              "LinkPreview::new(url).video(250.0)",
              "LinkPreview::new(url).on_remove(move || draft.drop_preview())",
              "",
              "// Drawn under the pointer with nothing pointing at it: for a gallery, a test.",
              "LinkPreview::new(url).force(Some(Interaction::Hovered))",
            ],
          ),
        ),
        Section::new(
          "Compact",
          "A row for lists and messages: the text takes the row where there is no picture. A page that tells \
           nothing shows its address, the link mark and a line that says so.",
          self.frame.demo(
            2,
            rows,
            ".variant(LinkPreviewVariant::Compact)",
            &[
              "LinkPreview::new(url).variant(LinkPreviewVariant::Compact)",
              "LinkPreview::new(url).variant(LinkPreviewVariant::Compact).no_image()",
            ],
          ),
        ),
        Section::new(
          "Inline",
          "24 high: the site's icon, the title and the site. Without an icon it is the link mark.",
          self.frame.demo(
            3,
            inlines,
            ".variant(LinkPreviewVariant::Inline)",
            &[
              "LinkPreview::new(url).variant(LinkPreviewVariant::Inline).title(\"Layout in lurq\")",
              "LinkPreview::new(url).variant(LinkPreviewVariant::Inline).no_icon()",
              "LinkPreview::new(url).variant(LinkPreviewVariant::Inline).no_site()",
            ],
          ),
        ),
        Section::new(
          "In a sentence",
          "A rich text holds it in its line: it is as wide as it is, wraps with the words, and the line keeps its \
           height. It is a stop of Tab in the order of the text.",
          self.frame.demo(
            4,
            in_a_sentence,
            "Text::rich(spans).object(preview)",
            &[
              "Text::rich([",
              "    RichTextSpan::new(\"The gaps are explained in \", body.clone()),",
              "    RichTextSpan::object(body.clone()),",
              "    RichTextSpan::new(\" — read it before the review.\", body),",
              "])",
              ".object(LinkPreview::new(url).variant(LinkPreviewVariant::Inline))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a preview is given. Every field is an override of what the page says.",
          props_table(&rows_of_props, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
