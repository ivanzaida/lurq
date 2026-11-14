//! The page of the banner: its playground, the board's banner, its
//! variants, a bar across a window with its text centered, and a banner the
//! app shows and takes away.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  node::Element,
  ui::{AlertVariant, Banner, BannerPart, Button, ButtonVariant, Link, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  pages::alert::VARIANTS,
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// A banner on a stage with what it is under it, as wide as the stage.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(FILL)
    .spacing(10.0)
    .child(shown)
    .child(quiet(caption, 12.0))
}

pub struct BannerPage {
  frame: Frame,
  playground: Playground,
  text: Signal<String>,
  variant: Signal<usize>,
  icon: Signal<bool>,
  link: Signal<bool>,
  closable: Signal<bool>,
  centered: Signal<bool>,
  open: Signal<bool>,
  shown: Signal<bool>,
}

impl BannerPage {
  /// The playground: the banner as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let text = self.text.get();
    let (variant, variant_name, variant_code) = VARIANTS[self.variant.get().min(VARIANTS.len() - 1)];
    let (icon, link, closable, centered, open) = (
      self.icon.get(),
      self.link.get(),
      self.closable.get(),
      self.centered.get(),
      self.open.get(),
    );
    let plain = {
      let text = text.clone();
      move || {
        let mut banner = Banner::new(&text).variant(variant).centered(centered).open(open);
        if !icon {
          banner = banner.icon(None);
        }
        if link {
          banner = banner.link(Link::new("Update now"));
        }
        banner
      }
    };
    let record = self.playground.recorder();
    let mut banner = plain().force(self.playground.forced());
    if closable {
      let close = self.open.clone();
      banner = banner.on_close(move || {
        record("on_close".to_owned());
        close.set(false);
      });
    }
    let make = {
      let plain = plain.clone();
      move || {
        let banner = plain();
        Column::new()
          .width(FILL)
          .child(if closable { banner.on_close(|| ()) } else { banner })
          .into()
      }
    };

    let mut code = vec![format!("Banner::new({text:?})")];
    if variant != AlertVariant::Neutral {
      code.push(format!("    .variant({variant_code})"));
    }
    if !icon {
      code.push("    .icon(None)".to_owned());
    }
    if link {
      code.push("    .link(Link::new(\"Update now\").on_click(|_| update()))".to_owned());
    }
    if centered {
      code.push("    .centered(true)".to_owned());
    }
    code.push("    .open(open.get())".to_owned());
    if closable {
      code.push("    .on_close(move || open.set(false))".to_owned());
    }

    let mut tree = vec![
      format!("root           {}", self.playground.measured()),
      format!(
        "└─ panel       {variant_name} · radius 6 · padded 10 16{}",
        if open { "" } else { " · taken away" }
      ),
    ];
    if icon {
      tree.push("   ├─ icon     16".to_owned());
    }
    tree.push(format!(
      "   {} text     13{}",
      if link || closable { "├─" } else { "└─" },
      if centered { " · centered" } else { "" }
    ));
    if link {
      tree.push(format!(
        "   {} link     Update now · 13/600",
        if closable { "├─" } else { "└─" }
      ));
    }
    if closable {
      tree.push("   └─ close    16".to_owned());
    }

    let to = (self.text.clone(), self.variant.clone());
    let flags = [
      self.icon.clone(),
      self.link.clone(),
      self.closable.clone(),
      self.centered.clone(),
      self.open.clone(),
    ];
    let choose = self.variant.clone();
    let variants: Vec<(&str, usize)> = VARIANTS.iter().enumerate().map(|(index, v)| (v.1, index)).collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Banner",
        make: Arc::new(make),
        shown: Column::new().width(FILL).child(banner).into(),
        fill: true,
        forceable: closable,
        facts: format!("{variant_name} · {}", variant.role()),
        event: "on_close",
        code,
        tree,
        controls: vec![
          prop("new", "&str", field(&self.text)),
          prop(
            "variant",
            "AlertVariant",
            segmented_fill(&variants, &self.variant.get(), move |index| choose.set(index)),
          ),
          divider(),
          flag("icon", &self.icon),
          flag("link", &self.link),
          flag("on_close", &self.closable),
          flag("centered", &self.centered),
          flag("open", &self.open),
        ],
        changed: [
          text != "lurq 0.61 is available with faster text shaping.",
          variant != AlertVariant::Info,
          !icon,
          !link,
          !closable,
          centered,
          !open,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0.set("lurq 0.61 is available with faster text shaping.".to_owned());
          to.1.set(1);
          for flag in &flags {
            flag.set(true);
          }
          flags[3].set(false);
        }),
      },
    )
  }
}

impl Component for BannerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      text: ctx.signal("lurq 0.61 is available with faster text shaping.".to_owned()),
      variant: ctx.signal(1),
      icon: ctx.signal(true),
      link: ctx.signal(true),
      closable: ctx.signal(true),
      centered: ctx.signal(false),
      open: ctx.signal(true),
      shown: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = item(
      Banner::new("lurq 0.61 is available with faster text shaping.")
        .variant(AlertVariant::Info)
        .link(Link::new("Update now"))
        .on_close(|| ()),
      "banner",
    );
    let variants = Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(item(Banner::new("Maintenance is planned for Sunday night."), "neutral"))
      .child(item(
        Banner::new("Your changes were published.").variant(AlertVariant::Success),
        "success",
      ))
      .child(item(
        Banner::new("Your trial ends in 3 days.")
          .variant(AlertVariant::Warning)
          .link(Link::new("Upgrade")),
        "warning",
      ))
      .child(item(
        Banner::new("Payments are failing for some cards.").variant(AlertVariant::Destructive),
        "destructive",
      ));
    let across = item(
      Banner::new("No connection. Changes will sync later.")
        .variant(AlertVariant::Warning)
        .icon(icons::WIFI_OFF)
        .centered(true)
        .style(BannerPart::Panel, |s| s.radius(0.0)),
      "centered · no corners",
    );
    let (shown, hide) = (self.shown.clone(), self.shown.clone());
    let taken = Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        Banner::new("Read-only: this workspace is archived.")
          .variant(AlertVariant::Info)
          .open(self.shown.get())
          .on_close(move || hide.set(false)),
      )
      .child(
        Row::new().child(
          Button::new(if self.shown.get() { "Take it away" } else { "Show it" })
            .variant(ButtonVariant::Outline)
            .on_click(move |_| shown.update(|open| *open = !*open)),
        ),
      );
    let rows = [
      ("new", "&str", "", "What it says, 13, on one line where it fits."),
      (
        "variant",
        "AlertVariant",
        "Neutral",
        "The fill in the muted form of its color, the icon and the link in its color.",
      ),
      (
        "icon",
        "&str | None",
        "the variant's",
        "The megaphone for Neutral and Info; another icon, or none.",
      ),
      (
        "link",
        "Link",
        "",
        "A link after the text in the variant's color, semibold: \"Update now\".",
      ),
      (
        "action",
        "impl Into<Element>",
        "",
        "Any element after the text in place of a link: a button.",
      ),
      (
        "on_close",
        "impl Fn()",
        "",
        "Draws the close control and is called on a press of it.",
      ),
      (
        "open",
        "bool",
        "true",
        "Whether it is shown. It fades in as it is shown and out as it is taken away.",
      ),
      (
        "centered",
        "bool",
        "false",
        "Its icon, text and action in its middle, as a bar across a window has them.",
      ),
      (
        "sticky",
        "bool",
        "false",
        "It sticks to the top of what scrolls the page.",
      ),
      (
        "role",
        "&str",
        "alert · status",
        "What it is to tooling, as an alert's.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Banner"],
        title: "Banner",
        description: "A message across the full width of what holds it, at the top of a window or a page: a new \
                      version, a connection lost, a trial that ends. The app shows it and takes it away.",
        import: Some("use lurq::ui::{AlertVariant, Banner};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Banner is a control on the right. Its close control takes it away: open brings it back.",
          playground,
        ),
        Section::new(
          "Banner",
          "The board's banner: the megaphone, what is new, a link to act on it and the close control.",
          self.frame.demo(
            0,
            board,
            "Banner::new(..).variant(..).link(..).on_close(..)",
            &[
              "Banner::new(\"lurq 0.61 is available with faster text shaping.\")",
              "    .variant(AlertVariant::Info)",
              "    .link(Link::new(\"Update now\").on_click(|_| update()))",
              "    .on_close(move || open.set(false))",
            ],
          ),
        ),
        Section::new(
          "Variants",
          "The fill is the muted form of the variant's color, its icon and its link the color itself.",
          self.frame.demo(
            1,
            variants,
            ".variant(AlertVariant::Warning)",
            &[
              "Banner::new(\"Maintenance is planned for Sunday night.\")",
              "Banner::new(\"Your changes were published.\").variant(AlertVariant::Success)",
              "Banner::new(\"Your trial ends in 3 days.\").variant(AlertVariant::Warning).link(Link::new(\"Upgrade\"))",
              "Banner::new(\"Payments are failing for some cards.\").variant(AlertVariant::Destructive)",
            ],
          ),
        ),
        Section::new(
          "Across a window",
          "Along the top edge of a window a banner has its text in its middle and no corners of its own.",
          self.frame.demo(
            2,
            across,
            ".centered(true)",
            &[
              "Banner::new(\"No connection. Changes will sync later.\")",
              "    .variant(AlertVariant::Warning)",
              "    .icon(icons::WIFI_OFF)",
              "    .centered(true)",
              "    .style(BannerPart::Panel, |s| s.radius(0.0))",
            ],
          ),
        ),
        Section::new(
          "Shown and taken away",
          "It stays in the tree with open(false): it fades out, and what is under it moves up.",
          self.frame.demo(
            3,
            taken,
            ".open(shown.get())",
            &[
              "// shown: Signal<bool>",
              "Banner::new(\"Read-only: this workspace is archived.\")",
              "    .variant(AlertVariant::Info)",
              "    .open(shown.get())",
              "    .on_close(move || shown.set(false))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a banner is given. Its text is all it needs.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
