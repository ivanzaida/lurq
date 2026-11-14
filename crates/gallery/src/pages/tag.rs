//! The page of the tag and the chip: a playground for each, tags a press
//! takes away, the variants, the sizes and the pill, chips bound to signals,
//! a group of chips, and their states.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Button, ButtonSize, ButtonVariant, Chip, ChipGroup, Choice, Interaction, Tag, TagSize, TagVariant, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

/// Tags side by side, 6 apart as the board lays them out.
fn tags() -> Row {
  Row::new().spacing(6.0).wrap().align_items(Alignment::Center)
}

/// A variant: what it is, what the code calls it, and what the panel calls it.
const VARIANTS: [(TagVariant, &str, &str); 5] = [
  (TagVariant::Neutral, "Neutral", "Neutral"),
  (TagVariant::Info, "Info", "Info"),
  (TagVariant::Success, "Success", "Success"),
  (TagVariant::Warning, "Warning", "Warning"),
  (TagVariant::Destructive, "Destructive", "Destructive"),
];

/// A size: what it is, what the code calls it, and what the panel calls it.
const SIZES: [(TagSize, &str, &str); 3] = [
  (TagSize::Sm, "Sm", "sm"),
  (TagSize::Md, "Md", "md"),
  (TagSize::Lg, "Lg", "lg"),
];
/// The size a tag has unless it says otherwise.
const MD: usize = 1;

/// An icon: what the panel calls it, its name, and what the code calls it.
const ICONS: [(&str, Option<&str>, &str); 4] = [
  ("None", None, ""),
  ("Tag", Some(icons::TAG), "TAG"),
  ("Star", Some(icons::STAR), "STAR"),
  ("User", Some(icons::USER), "USER"),
];

/// The tags the page starts with, which a press of their buttons takes away.
const TOPICS: [&str; 4] = ["design", "desktop", "rust", "gpu"];

/// How many of `changed` are true.
fn count(changed: &[bool]) -> usize {
  changed.iter().filter(|changed| **changed).count()
}

pub struct TagPage {
  frame: Frame,
  playground: Playground,
  label: Signal<String>,
  variant: Signal<usize>,
  size: Signal<usize>,
  icon: Signal<usize>,
  removable: Signal<bool>,
  pill: Signal<bool>,
  disabled: Signal<bool>,
  chip_playground: Playground,
  chip_label: Signal<String>,
  chip_icon: Signal<usize>,
  selected: Signal<bool>,
  chip_disabled: Signal<bool>,
  topics: Signal<Vec<String>>,
  open: Signal<bool>,
  mine: Signal<bool>,
  languages: Signal<Vec<&'static str>>,
  kind: Signal<u8>,
}

impl TagPage {
  /// The tag's playground: the tag as the panel's props make it.
  fn play_tag(&self, ctx: &mut Ctx) -> Element {
    let label = self.label.get();
    let (variant, size, icon) = (self.variant.get(), self.size.get(), self.icon.get());
    let (removable, pill, disabled) = (self.removable.get(), self.pill.get(), self.disabled.get());
    let record = self.playground.recorder();
    let plain = {
      let label = label.clone();
      move || {
        let tag = Tag::new(&label)
          .variant(VARIANTS[variant].0)
          .size(SIZES[size].0)
          .pill(pill)
          .disabled(disabled);
        match ICONS[icon].1 {
          Some(name) => tag.icon(name),
          None => tag,
        }
      }
    };
    let made = plain.clone();
    let make = move || {
      let tag = made();
      if removable { tag.on_remove(|| {}) } else { tag }
    };
    let mut tag = plain().force(self.playground.forced());
    if removable {
      let label = label.clone();
      tag = tag.on_remove(move || record(format!("on_remove() of {label:?}")));
    }

    let mut code = vec![format!("Tag::new({label:?})")];
    if variant != 0 {
      code.push(format!("    .variant(TagVariant::{})", VARIANTS[variant].1));
    }
    if size != MD {
      code.push(format!("    .size(TagSize::{})", SIZES[size].1));
    }
    if ICONS[icon].1.is_some() {
      code.push(format!("    .icon(icons::{})", ICONS[icon].2));
    }
    if pill {
      code.push("    .pill(true)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    if removable {
      code.push("    .on_remove(move || topics.update(|all| all.retain(|t| t != \"rust\")))".to_owned());
    }

    let mut tree = vec![format!("root      {}", self.playground.measured())];
    if ICONS[icon].1.is_some() {
      tree.push("├─ icon      12 × 12 px".to_owned());
    }
    tree.push(format!("{} label     {label:?}", if removable { "├─" } else { "└─" }));
    if removable {
      tree.push(format!(
        "└─ remove    {}   \"Remove {label}\"",
        if pill { "12 × 12 px" } else { "16 × 16 px" }
      ));
    }

    let glyphs: Vec<(&str, usize)> = ICONS
      .iter()
      .enumerate()
      .map(|(index, glyph)| (glyph.0, index))
      .collect();
    let variants: Vec<(&str, usize)> = VARIANTS.iter().enumerate().map(|(index, v)| (v.2, index)).collect();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, s)| (s.2, index)).collect();
    let (pick_icon, pick_variant, pick_size) = (self.icon.clone(), self.variant.clone(), self.size.clone());
    let (to_label, to_variant, to_size, to_icon, to_removable) = (
      self.label.clone(),
      self.variant.clone(),
      self.size.clone(),
      self.icon.clone(),
      self.removable.clone(),
    );
    let flags = [self.pill.clone(), self.disabled.clone()];
    self.playground.card(
      ctx,
      Spec {
        title: "Tag",
        make: Arc::new(move || make().into()),
        shown: tag.into(),
        fill: false,
        forceable: removable,
        facts: if removable {
          "removable".to_owned()
        } else {
          "read only".to_owned()
        },
        event: "on_remove",
        code,
        tree,
        controls: vec![
          prop("label", "&str", field(&self.label)),
          prop(
            "variant",
            "TagVariant",
            self
              .playground
              .select(0, &variants, &variant, move |index| pick_variant.set(index)),
          ),
          prop(
            "size",
            "TagSize",
            segmented_fill(&sizes, &size, move |index| pick_size.set(index)),
          ),
          prop(
            "icon",
            "impl Into<Name>",
            self
              .playground
              .select(1, &glyphs, &icon, move |index| pick_icon.set(index)),
          ),
          divider(),
          flag("on_remove", &self.removable),
          flag("pill", &self.pill),
          flag("disabled", &self.disabled),
        ],
        changed: count(&[variant != 0, size != MD, icon != 0, !removable, pill, disabled]),
        reset: Arc::new(move || {
          to_label.set("rust".to_owned());
          to_variant.set(0);
          to_size.set(MD);
          to_icon.set(0);
          to_removable.set(true);
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }

  /// The chip's playground: the chip as the panel's props make it.
  fn play_chip(&self, ctx: &mut Ctx) -> Element {
    let label = self.chip_label.get();
    let (icon, selected, disabled) = (self.chip_icon.get(), self.selected.get(), self.chip_disabled.get());
    let (record, set) = (self.chip_playground.recorder(), self.selected.clone());
    let plain = {
      let label = label.clone();
      move || {
        let chip = Chip::new(&label).selected(selected).disabled(disabled);
        match ICONS[icon].1 {
          Some(name) => chip.icon(name),
          None => chip,
        }
      }
    };
    let chip = plain().force(self.chip_playground.forced()).on_change(move |next| {
      set.set(next);
      record(format!("on_change({next})"));
    });

    let mut code = vec![format!("Chip::new({label:?})")];
    if ICONS[icon].1.is_some() {
      code.push(format!("    .icon(icons::{})", ICONS[icon].2));
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .bind(open)".to_owned());

    let mut tree = vec![format!(
      "root      {}   {}",
      self.chip_playground.measured(),
      if selected { "selected" } else { "not selected" }
    )];
    if selected {
      tree.push("├─ icon      check, 14 × 14 px".to_owned());
    } else if ICONS[icon].1.is_some() {
      tree.push("├─ icon      14 × 14 px".to_owned());
    }
    tree.push(format!("└─ label     {label:?}"));

    let glyphs: Vec<(&str, usize)> = ICONS
      .iter()
      .enumerate()
      .map(|(index, glyph)| (glyph.0, index))
      .collect();
    let pick = self.chip_icon.clone();
    let (to_label, to_icon) = (self.chip_label.clone(), self.chip_icon.clone());
    let flags = [self.selected.clone(), self.chip_disabled.clone()];
    self.chip_playground.card(
      ctx,
      Spec {
        title: "Chip",
        make: Arc::new(move || plain().into()),
        shown: chip.into(),
        fill: false,
        forceable: true,
        facts: if selected {
          "selected".to_owned()
        } else {
          "not selected".to_owned()
        },
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop("label", "&str", field(&self.chip_label)),
          prop(
            "icon",
            "impl Into<Name>",
            self
              .chip_playground
              .select(0, &glyphs, &icon, move |index| pick.set(index)),
          ),
          divider(),
          flag("selected", &self.selected),
          flag("disabled", &self.chip_disabled),
        ],
        changed: count(&[icon != 0, selected, disabled]),
        reset: Arc::new(move || {
          to_label.set("Open".to_owned());
          to_icon.set(0);
          for flag in &flags {
            flag.set(false);
          }
        }),
      },
    )
  }

  /// The page's tags, each with a button that takes it away; and one that puts them all back.
  fn removable(&self) -> Column {
    let mut row = tags();
    for topic in self.topics.get() {
      let topics = self.topics.clone();
      let gone = topic.clone();
      row = row.child(
        Tag::new(&topic)
          .key(topic.clone())
          .on_remove(move || topics.update(|all| all.retain(|held| *held != gone))),
      );
    }
    let topics = self.topics.clone();
    Column::new().spacing(14.0).child(row).child(
      Row::new().child(
        Button::new("Put them back")
          .variant(ButtonVariant::Outline)
          .size(ButtonSize::Sm)
          .icon(icons::REFRESH)
          .on_click(move |_| topics.set(TOPICS.iter().map(|topic| (*topic).to_owned()).collect())),
      ),
    )
  }
}

impl Component for TagPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      label: ctx.signal("rust".to_owned()),
      variant: ctx.signal(0),
      size: ctx.signal(MD),
      icon: ctx.signal(0),
      removable: ctx.signal(true),
      pill: ctx.signal(false),
      disabled: ctx.signal(false),
      chip_playground: Playground::new(ctx, 1),
      chip_label: ctx.signal("Open".to_owned()),
      chip_icon: ctx.signal(0),
      selected: ctx.signal(false),
      chip_disabled: ctx.signal(false),
      topics: ctx.signal(TOPICS.iter().map(|topic| (*topic).to_owned()).collect()),
      open: ctx.signal(true),
      mine: ctx.signal(false),
      languages: ctx.signal(vec!["rust"]),
      kind: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let variants = items()
      .child(item(Tag::new("design"), "neutral"))
      .child(item(Tag::new("in review").variant(TagVariant::Info), "info"))
      .child(item(Tag::new("passing").variant(TagVariant::Success), "success"))
      .child(item(Tag::new("pending").variant(TagVariant::Warning), "warning"))
      .child(item(
        Tag::new("failing").variant(TagVariant::Destructive),
        "destructive",
      ))
      .child(item(Tag::new("tagged").icon(icons::TAG), "with an icon"));
    let sizes = items()
      .child(item(Tag::new("small").size(TagSize::Sm).on_remove(|| {}), "sm · 20"))
      .child(item(Tag::new("middle").on_remove(|| {}), "md · 24"))
      .child(item(Tag::new("large").size(TagSize::Lg).on_remove(|| {}), "lg · 28"));
    let pills = items()
      .child(item(
        tags()
          .child(Tag::new("React").pill(true).on_remove(|| {}))
          .child(Tag::new("Vue").pill(true).on_remove(|| {})),
        "pill, removable",
      ))
      .child(item(Tag::new("Svelte").pill(true), "pill"))
      .child(item(
        Tag::new("Kind: Rust")
          .pill(true)
          .variant(TagVariant::Info)
          .on_remove(|| {}),
        "pill, info",
      ));
    let bound = |label: &str, on: &Signal<bool>| Chip::new(label).bind(on.clone());
    let says = |on: &Signal<bool>| {
      if on.get() {
        "the signal has: true"
      } else {
        "the signal has: false"
      }
    };
    let chips = items()
      .child(item(bound("Open", &self.open), says(&self.open)))
      .child(item(bound("Assigned to me", &self.mine), says(&self.mine)));
    let languages = self.languages.get().join(", ");
    let kinds = ["All", "Issues", "Pull requests"];
    let group = Column::new()
      .spacing(18.0)
      .child(item(
        ChipGroup::multiple(
          self.languages.clone(),
          [
            Choice::new("rust", "Rust"),
            Choice::new("swift", "Swift"),
            Choice::new("kotlin", "Kotlin"),
            Choice::new("cobol", "Cobol").disabled(true),
          ],
        ),
        &format!("any number: [{languages}]"),
      ))
      .child(item(
        ChipGroup::new(self.kind.clone(), [(0u8, kinds[0]), (1, kinds[1]), (2, kinds[2])]),
        &format!("one: {}", kinds[self.kind.get() as usize]),
      ));
    let states = items()
      .child(item(Chip::new("Swift"), "not selected"))
      .child(item(Chip::new("Rust").selected(true), "selected"))
      .child(item(Chip::new("Hovered").force(Some(Interaction::Hovered)), "hovered"))
      .child(item(
        Chip::new("Focused").force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(Chip::new("Disabled").disabled(true), "disabled"))
      .child(item(
        Tag::new("focused")
          .on_remove(|| {})
          .force(Some(Interaction::FocusVisible)),
        "remove, focus-visible",
      ))
      .child(item(
        Tag::new("locked").on_remove(|| {}).disabled(true),
        "tag, disabled",
      ));
    let tag_rows = [
      (
        "label",
        "&str",
        "",
        "Its text, and what its remove button is called after: \"Remove rust\".",
      ),
      (
        "variant",
        "TagVariant",
        "Neutral",
        "What it says by its color: neutral, info, success, warning, destructive.",
      ),
      ("size", "TagSize", "Md", "20, 24 or 28 high."),
      (
        "icon",
        "impl Into<Name>",
        "",
        "An icon before the text, by its name in the theme's set.",
      ),
      (
        "on_remove",
        "impl Fn()",
        "",
        "Gives it the button that takes it away. A press, Enter, Space, Backspace and Delete press it.",
      ),
      ("pill", "bool", "false", "Round at both ends, with no line around it."),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity; its remove button is not pressed.",
      ),
    ];
    let chip_rows = [
      ("label", "&str", "", "Its text, and what it is called."),
      (
        "selected",
        "bool",
        "false",
        "Whether it is chosen. The chip does not keep it: the app says so.",
      ),
      (
        "on_change",
        "impl Fn(bool)",
        "",
        "Called on a press with what the chip is to be: chosen if it was not.",
      ),
      (
        "bind",
        "Signal<bool>",
        "",
        "Chosen while the signal is true, which a press turns.",
      ),
      (
        "icon",
        "impl Into<Name>",
        "",
        "An icon before the text while it is not chosen. A chosen chip has the check there.",
      ),
      ("disabled", "bool", "false", "Half the opacity; it takes no input."),
    ];
    let tag_playground = self.play_tag(ctx);
    let chip_playground = self.play_chip(ctx);
    let room = self.frame.room();
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Tag"],
        title: "Tag",
        description: "A label on a thing, a keyword or a category, that the app may let a person take away; and \
                      the chip, a tag that is pressed: a filter or a choice that is on or off.",
        import: Some("use lurq::ui::{Button, ButtonSize, ButtonVariant, Chip, ChipGroup, Tag, TagSize, TagVariant};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Tag is a control on the right. With on_remove the tag has its button at the end, and a \
           press of it, or Backspace while it has the focus, is written under Events.",
          tag_playground,
        ),
        Section::new(
          "Removable",
          "The page holds the list. Tab reaches each tag's remove button, and Enter, Space, Backspace or Delete \
           takes that tag away.",
          self.frame.demo(
            0,
            self.removable(),
            "Tag::new(\"rust\").on_remove(..)",
            &[
              "// topics: Signal<Vec<String>>",
              "let mut row = Row::new().spacing(6.0).wrap();",
              "for topic in topics.get() {",
              "    let (topics, gone) = (topics.clone(), topic.clone());",
              "    row = row.child(",
              "        Tag::new(&topic)",
              "            .key(topic.clone())",
              "            .on_remove(move || topics.update(|all| all.retain(|t| *t != gone))),",
              "    );",
              "}",
            ],
          ),
        ),
        Section::new(
          "Variants",
          "A neutral tag is in the fill of what is secondary with a line inside it; the others are in the muted \
           fill of their color, with their text in the color.",
          self.frame.demo(
            1,
            variants,
            ".variant(TagVariant::Info)",
            &[
              "Tag::new(\"design\")",
              "Tag::new(\"in review\").variant(TagVariant::Info)",
              "Tag::new(\"passing\").variant(TagVariant::Success)",
              "Tag::new(\"pending\").variant(TagVariant::Warning)",
              "Tag::new(\"failing\").variant(TagVariant::Destructive)",
              "Tag::new(\"tagged\").icon(icons::TAG)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "20, 24 and 28 high; 24 is the board's tag, 28 a chip's height.",
          self.frame.demo(
            2,
            sizes,
            ".size(TagSize::Lg)",
            &[
              "Tag::new(\"small\").size(TagSize::Sm).on_remove(..)",
              "Tag::new(\"middle\").on_remove(..)",
              "Tag::new(\"large\").size(TagSize::Lg).on_remove(..)",
            ],
          ),
        ),
        Section::new(
          "Pill",
          "Round at both ends and with no line: the token of a box that holds several values, as a multi select \
           does.",
          self.frame.demo(
            3,
            pills,
            ".pill(true)",
            &[
              "Tag::new(\"React\").pill(true).on_remove(..)",
              "Tag::new(\"Svelte\").pill(true)",
              "Tag::new(\"Kind: Rust\").pill(true).variant(TagVariant::Info).on_remove(..)",
            ],
          ),
        ),
        Section::new(
          "Chip playground",
          "Every prop of Chip is a control on the right. The chip on the stage is live: a press turns it.",
          chip_playground,
        ),
        Section::new(
          "Chips",
          "The page holds whether each is chosen, in a signal the chip is bound to. Tab reaches a chip, and Space \
           or Enter turns it.",
          self.frame.demo(
            4,
            chips,
            "Chip::new(\"Open\").bind(open)",
            &[
              "// open, mine: Signal<bool>",
              "Chip::new(\"Open\").bind(open)",
              "Chip::new(\"Assigned to me\").bind(mine)",
            ],
          ),
        ),
        Section::new(
          "Chip group",
          "Chips that are one choice, 6 apart. Tab stops in the group once, the arrow keys move between its chips, \
           and Space or Enter turns the one the focus is on.",
          self.frame.demo(
            5,
            group,
            "ChipGroup::multiple(languages, [..])",
            &[
              "// languages: Signal<Vec<&'static str>>, kind: Signal<u8>",
              "ChipGroup::multiple(languages, [",
              "    Choice::new(\"rust\", \"Rust\"),",
              "    Choice::new(\"swift\", \"Swift\"),",
              "    Choice::new(\"kotlin\", \"Kotlin\"),",
              "    Choice::new(\"cobol\", \"Cobol\").disabled(true),",
              "])",
              "",
              "ChipGroup::new(kind, [(0, \"All\"), (1, \"Issues\"), (2, \"Pull requests\")])",
            ],
          ),
        ),
        Section::new(
          "States",
          "A chip not chosen is outlined, a chosen one filled in the primary color with a check. The ring of the \
           focus is the chip's, and of a tag its remove button's.",
          self.frame.demo(
            6,
            states,
            ".force(Some(Interaction::FocusVisible))",
            &[
              "Chip::new(\"Swift\")",
              "Chip::new(\"Rust\").selected(true)",
              "",
              "// Drawn hovered or focused from the keyboard with nothing pointing at it: for a gallery, a test.",
              "Chip::new(\"Hovered\").force(Some(Interaction::Hovered))",
              "Chip::new(\"Focused\").force(Some(Interaction::FocusVisible))",
              "Chip::new(\"Disabled\").disabled(true)",
              "Tag::new(\"focused\").on_remove(..).force(Some(Interaction::FocusVisible))",
              "Tag::new(\"locked\").on_remove(..).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Tag props",
          "What a tag is given.",
          props_table(&tag_rows, &["label"], room),
        ),
        Section::new(
          "Chip props",
          "What a chip is given.",
          props_table(&chip_rows, &["label"], room),
        ),
      ],
    )
  }
}
