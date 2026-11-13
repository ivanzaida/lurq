//! The page of the badge: its playground, the variants and sizes the board
//! draws, a dot or an icon before the text, a dot alone, counts and their
//! maximum, and a badge on the corner of what it is about.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Badge, BadgePart, BadgeShape, BadgeSize, BadgeVariant, Button, ButtonVariant, FontFamily, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// The variants, as the code names them.
const VARIANTS: [(BadgeVariant, &str, &str); 7] = [
  (BadgeVariant::Default, "default", "BadgeVariant::Default"),
  (BadgeVariant::Secondary, "secondary", "BadgeVariant::Secondary"),
  (BadgeVariant::Outline, "outline", "BadgeVariant::Outline"),
  (BadgeVariant::Success, "success", "BadgeVariant::Success"),
  (BadgeVariant::Warning, "warning", "BadgeVariant::Warning"),
  (BadgeVariant::Info, "info", "BadgeVariant::Info"),
  (BadgeVariant::Destructive, "destructive", "BadgeVariant::Destructive"),
];

/// The sizes, as the code names them, and how high each is.
const SIZES: [(BadgeSize, &str, &str, u32); 3] = [
  (BadgeSize::Sm, "sm", "BadgeSize::Sm", 18),
  (BadgeSize::Md, "md", "BadgeSize::Md", 22),
  (BadgeSize::Lg, "lg", "BadgeSize::Lg", 26),
];

/// The icons the playground offers, as the code names them: none first.
const ICONS: [(&str, Option<&str>, &str); 5] = [
  ("none", None, ""),
  ("check", Some(icons::CHECK), "icons::CHECK"),
  ("refresh", Some(icons::REFRESH), "icons::REFRESH"),
  ("warning", Some(icons::WARNING), "icons::WARNING"),
  ("star", Some(icons::STAR), "icons::STAR"),
];

/// The counts the playground offers: none is a badge with text.
const COUNTS: [(&str, Option<usize>); 5] = [
  ("none", None),
  ("0", Some(0)),
  ("8", Some(8)),
  ("42", Some(42)),
  ("120", Some(120)),
];

/// The maximums the playground offers.
const MAXIMUMS: [(&str, usize); 3] = [("9", 9), ("99", 99), ("999", 999)];

/// Something on a stage with what it is under it. Neither clips, so a badge that reaches past the corner of what
/// it is on is drawn whole.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .overflow_visible()
    .child(Row::new().overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::End)
    .overflow_visible()
}

/// A button of an icon alone, outlined, as the board's Indicator is: what a dot or a count is put on.
fn bell(label: &str) -> Button {
  Button::new(label)
    .icon(icons::BELL)
    .icon_only(true)
    .variant(ButtonVariant::Outline)
}

pub struct BadgePage {
  frame: Frame,
  playground: Playground,
  text: Signal<String>,
  variant: Signal<usize>,
  size: Signal<usize>,
  icon: Signal<usize>,
  count: Signal<usize>,
  max: Signal<usize>,
  dot: Signal<bool>,
  dot_only: Signal<bool>,
  square: Signal<bool>,
  show_zero: Signal<bool>,
  on: Signal<bool>,
}

impl BadgePage {
  /// The playground: the badge as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let text = self.text.get();
    let (variant, variant_name, variant_code) = VARIANTS[self.variant.get().min(VARIANTS.len() - 1)];
    let (size, size_name, size_code, height) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let (_, icon, icon_code) = ICONS[self.icon.get().min(ICONS.len() - 1)];
    let (_, count) = COUNTS[self.count.get().min(COUNTS.len() - 1)];
    let (_, max) = MAXIMUMS[self.max.get().min(MAXIMUMS.len() - 1)];
    let (dot, dot_only, square) = (self.dot.get(), self.dot_only.get(), self.square.get());
    let (show_zero, on) = (self.show_zero.get(), self.on.get());

    let plain = {
      let text = text.clone();
      move || {
        let mut badge = match count {
          Some(n) => Badge::count(n).label(&text).max(max).show_zero(show_zero),
          None => Badge::new(&text),
        }
        .variant(variant)
        .size(size)
        .dot(dot)
        .dot_only(dot_only);
        if let Some(icon) = icon {
          badge = badge.icon(icon);
        }
        if square {
          badge = badge.shape(BadgeShape::Square);
        }
        let shown: Element = if on {
          // Room for what reaches past the corner: the stage clips what it holds.
          Row::new()
            .padding_top(6.0)
            .padding_right(6.0)
            .overflow_visible()
            .child(badge.on(bell("Notifications")))
            .into()
        } else {
          badge.into()
        };
        shown
      }
    };

    let mut code = vec![match count {
      Some(n) => format!("Badge::count({n})"),
      None => format!("Badge::new({text:?})"),
    }];
    if count.is_some() {
      code.push(format!("    .label({text:?})"));
      if max != 99 {
        code.push(format!("    .max({max})"));
      }
      if show_zero {
        code.push("    .show_zero(true)".to_owned());
      }
    }
    let default_variant = if count.is_some() {
      BadgeVariant::Destructive
    } else {
      BadgeVariant::Default
    };
    if variant != default_variant {
      code.push(format!("    .variant({variant_code})"));
    }
    let default_size = if count.is_some() { BadgeSize::Sm } else { BadgeSize::Md };
    if size != default_size {
      code.push(format!("    .size({size_code})"));
    }
    if square {
      code.push("    .shape(BadgeShape::Square)".to_owned());
    }
    if icon.is_some() {
      code.push(format!("    .icon({icon_code})"));
    }
    if dot {
      code.push("    .dot(true)".to_owned());
    }
    if dot_only {
      code.push("    .dot_only(true)".to_owned());
    }
    if on {
      code.push("    .on(Button::new(\"Notifications\").icon(icons::BELL).icon_only(true))".to_owned());
    }

    let shown_text = match count {
      Some(n) if n > max => format!("{max}+"),
      Some(n) => n.to_string(),
      None => text.clone(),
    };
    let hidden = count == Some(0) && !show_zero;
    let mut tree = vec![format!(
      "root      {}   {variant_name} · {size_name} · {height} px   status",
      self.playground.measured()
    )];
    if hidden {
      tree = vec!["nothing: a count of 0".to_owned()];
    } else if dot_only {
      tree[0] = format!(
        "root      {}   a dot alone · {variant_name}",
        self.playground.measured()
      );
    } else {
      if dot {
        tree.push("├─ dot     7 px".to_owned());
      }
      if icon.is_some() {
        tree.push("├─ icon".to_owned());
      }
      tree.push(format!("└─ label   {shown_text:?}"));
    }
    if on && !hidden {
      tree.push("on the top end corner of a button of 36, a ring of 2 around it".to_owned());
    }

    let variants: Vec<(&str, usize)> = VARIANTS.iter().enumerate().map(|(index, v)| (v.1, index)).collect();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, s)| (s.1, index)).collect();
    let glyphs: Vec<(&str, usize)> = ICONS.iter().enumerate().map(|(index, i)| (i.0, index)).collect();
    let counts: Vec<(&str, usize)> = COUNTS.iter().enumerate().map(|(index, c)| (c.0, index)).collect();
    let maximums: Vec<(&str, usize)> = MAXIMUMS.iter().enumerate().map(|(index, m)| (m.0, index)).collect();
    let (to_variant, to_size, to_icon) = (self.variant.clone(), self.size.clone(), self.icon.clone());
    let (to_count, to_max) = (self.count.clone(), self.max.clone());
    let reset = (
      self.text.clone(),
      self.variant.clone(),
      self.size.clone(),
      self.icon.clone(),
      self.count.clone(),
      self.max.clone(),
    );
    let flags = [
      self.dot.clone(),
      self.dot_only.clone(),
      self.square.clone(),
      self.show_zero.clone(),
      self.on.clone(),
    ];
    let make = plain.clone();
    self.playground.card(
      ctx,
      Spec {
        title: "Badge",
        make: Arc::new(move || make()),
        shown: plain(),
        fill: false,
        forceable: false,
        facts: format!("{variant_name} · {size_name}"),
        event: "",
        code,
        tree,
        controls: vec![
          prop("text", "&str", field(&self.text)),
          prop(
            "variant",
            "BadgeVariant",
            self
              .playground
              .select(0, &variants, &self.variant.get(), move |index| to_variant.set(index)),
          ),
          prop(
            "size",
            "BadgeSize",
            segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
          ),
          prop(
            "icon",
            "impl Into<Name>",
            self
              .playground
              .select(1, &glyphs, &self.icon.get(), move |index| to_icon.set(index)),
          ),
          divider(),
          prop(
            "count",
            "usize",
            segmented_fill(&counts, &self.count.get(), move |index| to_count.set(index)),
          ),
          prop(
            "max",
            "usize",
            segmented_fill(&maximums, &self.max.get(), move |index| to_max.set(index)),
          ),
          flag("show_zero", &self.show_zero),
          divider(),
          flag("dot", &self.dot),
          flag("dot_only", &self.dot_only),
          flag("square", &self.square),
          flag("on a button", &self.on),
        ],
        changed: [
          text != "Beta",
          self.variant.get() != 0,
          self.size.get() != 1,
          self.icon.get() != 0,
          count.is_some(),
          max != 99,
          show_zero,
          dot,
          dot_only,
          square,
          on,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          reset.0.set("Beta".to_owned());
          reset.1.set(0);
          reset.2.set(1);
          reset.3.set(0);
          reset.4.set(0);
          reset.5.set(1);
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for BadgePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      text: ctx.signal("Beta".to_owned()),
      variant: ctx.signal(0),
      size: ctx.signal(1),
      icon: ctx.signal(0),
      count: ctx.signal(0),
      max: ctx.signal(1),
      dot: ctx.signal(false),
      dot_only: ctx.signal(false),
      square: ctx.signal(false),
      show_zero: ctx.signal(false),
      on: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let texts = ["Badge", "Secondary", "Outline", "Success", "Warning", "Info", "Failed"];
    let variants = VARIANTS
      .iter()
      .zip(texts)
      .fold(items(), |row, ((variant, name, _), text)| {
        row.child(item(Badge::new(text).variant(*variant), name))
      });
    let sizes = items()
      .child(item(Badge::new("Small").size(BadgeSize::Sm), "sm · 18"))
      .child(item(Badge::new("Medium"), "md · 22"))
      .child(item(Badge::new("Large").size(BadgeSize::Lg), "lg · 26"))
      .child(item(
        Badge::new("v0.60")
          .variant(BadgeVariant::Secondary)
          .shape(BadgeShape::Square)
          .style(BadgePart::Label, |s| s.font_family(FontFamily::Mono)),
        "square",
      ));
    let marks = items()
      .child(item(
        Badge::new("Online").variant(BadgeVariant::Success).dot(true),
        "dot",
      ))
      .child(item(
        Badge::new("Away").variant(BadgeVariant::Warning).dot(true),
        "dot · away",
      ))
      .child(item(
        Badge::new("Passing").variant(BadgeVariant::Success).icon(icons::CHECK),
        "icon",
      ))
      .child(item(
        Badge::new("Building").variant(BadgeVariant::Info).icon(icons::REFRESH),
        "icon · info",
      ));
    let dots = [
      (BadgeVariant::Default, "default"),
      (BadgeVariant::Info, "info"),
      (BadgeVariant::Success, "success"),
      (BadgeVariant::Warning, "warning"),
      (BadgeVariant::Destructive, "destructive"),
    ]
    .into_iter()
    .fold(items(), |row, (variant, name)| {
      row.child(item(Badge::new("Unread").variant(variant).dot_only(true), name))
    })
    .child(item(Badge::new("Unread").size(BadgeSize::Lg).dot_only(true), "lg · 10"));
    let counts = items()
      .child(item(Badge::count(8), "8"))
      .child(item(Badge::count(42), "42"))
      .child(item(Badge::count(120), "120, past 99"))
      .child(item(Badge::count(12).max(9), "12, max 9"))
      .child(item(Badge::count(0).show_zero(true), "0, shown"))
      .child(item(
        Badge::count(4).variant(BadgeVariant::Secondary).size(BadgeSize::Md),
        "secondary · md",
      ));
    let corners = items()
      .child(item(
        Badge::new("New notifications")
          .variant(BadgeVariant::Destructive)
          .dot_only(true)
          .on(bell("Notifications")),
        "indicator",
      ))
      .child(item(
        Badge::count(3).label("notifications").on(bell("Notifications")),
        "count",
      ))
      .child(item(
        Badge::count(120).label("unread messages").on(bell("Messages")),
        "past 99",
      ))
      .child(item(
        Badge::count(0).label("notifications").on(bell("Notifications")),
        "0: the button alone",
      ));
    let rows = [
      (
        "label",
        "&str",
        "",
        "What the badge says, and what it is called. On a count, what it counts.",
      ),
      (
        "variant",
        "BadgeVariant",
        "Default",
        "Its colors: the primary color, the secondary fill, an edge, or a status's muted fill and strong text.",
      ),
      ("size", "BadgeSize", "Md", "18, 22 or 26 high, at 11, 12 or 13."),
      (
        "shape",
        "BadgeShape",
        "Pill",
        "Round, or with the theme's small radius.",
      ),
      (
        "icon",
        "impl Into<Name>",
        "",
        "An icon of the theme's set before the text, 4 before it.",
      ),
      (
        "dot",
        "bool",
        "false",
        "A dot of 7 in the variant's color before the text; the badge has an edge and no fill.",
      ),
      (
        "dot_only",
        "bool",
        "false",
        "A dot alone, in the variant's color. The text is what it is called.",
      ),
      (
        "Badge::count",
        "usize",
        "",
        "A count: a small round badge in the destructive color. Nothing is drawn at 0.",
      ),
      (
        "max",
        "usize",
        "99",
        "The most a count shows: past it, the maximum and a plus.",
      ),
      ("show_zero", "bool", "false", "Whether a count of 0 is drawn."),
      (
        "on",
        "impl Into<Element>",
        "",
        "Puts the badge on the top end corner of an element, with a ring of the background around it.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Badge"],
        title: "Badge",
        description: "A short label of a state or a count, read and not operated: beta, passing, 3 unread.",
        import: Some("use lurq::ui::Badge;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Badge is a control on the right. A badge takes no press and no focus: it is read.",
          playground,
        ),
        Section::new(
          "Variants",
          "The primary color, the secondary fill, an edge with no fill, and a status's muted fill with its strong \
           text.",
          self.frame.demo(
            0,
            variants,
            ".variant(BadgeVariant::Success)",
            &[
              "Badge::new(\"Badge\")",
              "Badge::new(\"Secondary\").variant(BadgeVariant::Secondary)",
              "Badge::new(\"Outline\").variant(BadgeVariant::Outline)",
              "Badge::new(\"Success\").variant(BadgeVariant::Success)",
              "Badge::new(\"Warning\").variant(BadgeVariant::Warning)",
              "Badge::new(\"Info\").variant(BadgeVariant::Info)",
              "Badge::new(\"Failed\").variant(BadgeVariant::Destructive)",
            ],
          ),
        ),
        Section::new(
          "Sizes and shape",
          "18, 22 and 26 high. A square badge has the theme's small radius; a version reads well in the family of \
           code.",
          self.frame.demo(
            1,
            sizes,
            ".size(BadgeSize::Sm)",
            &[
              "Badge::new(\"Small\").size(BadgeSize::Sm)",
              "Badge::new(\"Medium\")",
              "Badge::new(\"Large\").size(BadgeSize::Lg)",
              "Badge::new(\"v0.60\")",
              "    .variant(BadgeVariant::Secondary)",
              "    .shape(BadgeShape::Square)",
              "    .style(BadgePart::Label, |s| s.font_family(FontFamily::Mono))",
            ],
          ),
        ),
        Section::new(
          "A dot or an icon",
          "A dot before the text takes the variant's color, on a badge with an edge and no fill: online, away. An \
           icon stands 4 before the text in the badge's own colors.",
          self.frame.demo(
            2,
            marks,
            ".dot(true)",
            &[
              "Badge::new(\"Online\").variant(BadgeVariant::Success).dot(true)",
              "Badge::new(\"Away\").variant(BadgeVariant::Warning).dot(true)",
              "Badge::new(\"Passing\").variant(BadgeVariant::Success).icon(icons::CHECK)",
              "Badge::new(\"Building\").variant(BadgeVariant::Info).icon(icons::REFRESH)",
            ],
          ),
        ),
        Section::new(
          "A dot alone",
          "A circle of 8 in the variant's color, 6 small and 10 large. What it says is not drawn: it is what tooling \
           that reads the tree calls it.",
          self.frame.demo(
            3,
            dots,
            ".dot_only(true)",
            &[
              "Badge::new(\"Unread\").dot_only(true)",
              "Badge::new(\"Unread\").variant(BadgeVariant::Info).dot_only(true)",
              "Badge::new(\"Unread\").variant(BadgeVariant::Success).dot_only(true)",
              "Badge::new(\"Unread\").variant(BadgeVariant::Warning).dot_only(true)",
              "Badge::new(\"Unread\").variant(BadgeVariant::Destructive).dot_only(true)",
              "Badge::new(\"Unread\").size(BadgeSize::Lg).dot_only(true)",
            ],
          ),
        ),
        Section::new(
          "Counts",
          "A number in a small round badge, in the destructive color, at least as wide as it is high. Past its \
           maximum, 99 unless told, it shows the maximum and a plus; at 0 it is nothing unless told.",
          self.frame.demo(
            4,
            counts,
            "Badge::count(8)",
            &[
              "Badge::count(8)",
              "Badge::count(42)",
              "Badge::count(120)",
              "Badge::count(12).max(9)",
              "Badge::count(0).show_zero(true)",
              "Badge::count(4).variant(BadgeVariant::Secondary).size(BadgeSize::Md)",
            ],
          ),
        ),
        Section::new(
          "On a corner",
          "A dot or a count on the top end corner of what it is about, its round end centred 3 in from the corner, \
           with a ring of the background around it. It reaches past the element, which keeps its size: what holds \
           them gives it room or does not clip. A count of 0 leaves the element alone.",
          self.frame.demo(
            5,
            corners,
            ".on(button)",
            &[
              "let bell = || Button::new(\"Notifications\")",
              "    .icon(icons::BELL)",
              "    .icon_only(true)",
              "    .variant(ButtonVariant::Outline);",
              "",
              "Badge::new(\"New notifications\")",
              "    .variant(BadgeVariant::Destructive)",
              "    .dot_only(true)",
              "    .on(bell())",
              "Badge::count(3).label(\"notifications\").on(bell())",
              "Badge::count(120).label(\"unread messages\").on(bell())",
              "Badge::count(0).label(\"notifications\").on(bell())",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a badge is given.",
          props_table(&rows, &["label"], self.frame.room()),
        ),
      ],
    )
  }
}
