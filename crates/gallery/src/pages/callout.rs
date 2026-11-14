//! The page of the callout: its playground, the board's callout, the notes
//! of a document with their bar, and a callout in a form.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::Column,
  core::Signal,
  node::Element,
  ui::{AlertVariant, Callout, Field, Input, Link, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, quiet, segmented_fill},
  pages::alert::VARIANTS,
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str, width: f32) -> Column {
  Column::new()
    .width(width)
    .spacing(10.0)
    .child(shown)
    .child(quiet(caption, 12.0))
}

pub struct CalloutPage {
  frame: Frame,
  playground: Playground,
  text: Signal<String>,
  title: Signal<String>,
  variant: Signal<usize>,
  icon: Signal<bool>,
  bar: Signal<bool>,
  key: Signal<String>,
}

impl CalloutPage {
  /// The playground: the callout as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (text, title) = (self.text.get(), self.title.get());
    let (variant, variant_name, variant_code) = VARIANTS[self.variant.get().min(VARIANTS.len() - 1)];
    let (icon, bar) = (self.icon.get(), self.bar.get());
    let make = {
      let (text, title) = (text.clone(), title.clone());
      move || -> Element {
        let mut callout = Callout::new(&text).variant(variant).bar(bar);
        if !title.is_empty() {
          callout = callout.title(&title);
        }
        if !icon {
          callout = callout.icon(None);
        }
        Column::new().width(380.0).child(callout).into()
      }
    };

    let mut code = vec![format!("Callout::new({text:?})")];
    if !title.is_empty() {
      code.push(format!("    .title({title:?})"));
    }
    if variant != AlertVariant::Neutral {
      code.push(format!("    .variant({variant_code})"));
    }
    if !icon {
      code.push("    .icon(None)".to_owned());
    }
    if bar {
      code.push("    .bar(true)".to_owned());
    }

    let mut tree = vec![format!(
      "root          {}   {variant_name}{}",
      self.playground.measured(),
      if bar { " · bar 3" } else { " · radius 6" }
    )];
    if bar && !title.is_empty() {
      tree.push("├─ heading    icon · title".to_owned());
      tree.push("└─ content    text".to_owned());
    } else {
      if icon {
        tree.push("├─ icon       16".to_owned());
      }
      tree.push(format!(
        "└─ content    {}text",
        if title.is_empty() { "" } else { "title · " }
      ));
    }

    let to = (self.text.clone(), self.title.clone(), self.variant.clone());
    let flags = [self.icon.clone(), self.bar.clone()];
    let choose = self.variant.clone();
    let variants: Vec<(&str, usize)> = VARIANTS.iter().enumerate().map(|(index, v)| (v.1, index)).collect();
    self.playground.card(
      ctx,
      Spec {
        title: "Callout",
        make: Arc::new(make.clone()),
        shown: make(),
        fill: false,
        forceable: false,
        facts: format!("{variant_name} · note"),
        event: "",
        code,
        tree,
        controls: vec![
          prop("new", "&str", field(&self.text)),
          prop("title", "&str", field(&self.title)),
          prop(
            "variant",
            "AlertVariant",
            segmented_fill(&variants, &self.variant.get(), move |index| choose.set(index)),
          ),
          divider(),
          flag("icon", &self.icon),
          flag("bar", &self.bar),
        ],
        changed: [
          text != "Theme changes rerender only the components that read the theme.",
          !title.is_empty(),
          variant != AlertVariant::Info,
          !icon,
          bar,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0
            .set("Theme changes rerender only the components that read the theme.".to_owned());
          to.1.set(String::new());
          to.2.set(1);
          flags[0].set(true);
          flags[1].set(false);
        }),
      },
    )
  }
}

impl Component for CalloutPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      text: ctx.signal("Theme changes rerender only the components that read the theme.".to_owned()),
      title: ctx.signal(String::new()),
      variant: ctx.signal(1),
      icon: ctx.signal(true),
      bar: ctx.signal(false),
      key: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = item(
      Callout::new("Theme changes rerender only the components that read the theme.").variant(AlertVariant::Info),
      "callout",
      380.0,
    );
    let notes = Column::new()
      .spacing(12.0)
      .child(
        Callout::new("Blur is skipped when the radius is zero.")
          .title("Note")
          .variant(AlertVariant::Info)
          .bar(true),
      )
      .child(
        Callout::new("Use backdrop blur for panels over busy content.")
          .title("Tip")
          .variant(AlertVariant::Success)
          .icon(icons::LIGHTBULB)
          .bar(true),
      )
      .child(
        Callout::new("Large radii cost more on integrated GPUs.")
          .title("Warning")
          .variant(AlertVariant::Warning)
          .bar(true),
      )
      .child(
        Callout::new("Blurring a video node disables direct scanout.")
          .title("Caution")
          .variant(AlertVariant::Destructive)
          .icon(icons::ALERT_OCTAGON)
          .bar(true),
      );
    let notes = item(notes, "bar · title", 420.0);
    let form = item(
      Column::new()
        .spacing(16.0)
        .child(Field::new("API key", Input::new(self.key.clone()).full_width(true)))
        .child(
          Callout::new("The key is shown once. Keep it where your team keeps its secrets.")
            .title("Before you go on")
            .variant(AlertVariant::Warning)
            .child(Link::new("How keys are stored").external()),
        ),
      "in a form · title · child",
      380.0,
    );
    let rows = [
      (
        "new",
        "&str",
        "",
        "What it says, 13 and 1.5 high. An empty text says nothing.",
      ),
      (
        "title",
        "&str",
        "",
        "What it is, in a word, over its text: \"Note\", \"Tip\". It is what the callout is called.",
      ),
      (
        "child / children",
        "impl Into<Element>",
        "",
        "Elements under the text: a link, a list.",
      ),
      (
        "variant",
        "AlertVariant",
        "Neutral",
        "The fill in the muted form of its color, the icon (and the bar and its title) in its color.",
      ),
      (
        "icon",
        "&str | None",
        "the variant's",
        "An icon of the theme's set in place of the variant's, or none.",
      ),
      (
        "bar",
        "bool",
        "false",
        "A bar of 3 along its start, and the icon before a title in the variant's color: the note of a document.",
      ),
      ("role", "&str", "note", "What it is to tooling that reads the tree."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Callout"],
        title: "Callout",
        description: "A note set apart inside prose or a form: a hint beside a setting, a caution in a page of \
                      documentation. It is read, and is no stop of Tab.",
        import: Some("use lurq::ui::{AlertVariant, Callout};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Callout is a control on the right.",
          playground,
        ),
        Section::new(
          "Callout",
          "The board's callout: the icon of its variant at its start, the text beside it.",
          self.frame.demo(
            0,
            board,
            "Callout::new(..).variant(AlertVariant::Info)",
            &["Callout::new(\"Theme changes rerender only the components that read the theme.\")", "    .variant(AlertVariant::Info)"],
          ),
        ),
        Section::new(
          "Notes of a document",
          "With a bar along its start, a callout is the board's note of a document: the icon before a title in the \
           variant's color, the text under them.",
          self.frame.demo(
            1,
            notes,
            ".title(\"Tip\").bar(true)",
            &[
              "Callout::new(\"Blur is skipped when the radius is zero.\").title(\"Note\").variant(AlertVariant::Info).bar(true)",
              "Callout::new(\"Use backdrop blur for panels over busy content.\")",
              "    .title(\"Tip\")",
              "    .variant(AlertVariant::Success)",
              "    .icon(icons::LIGHTBULB)",
              "    .bar(true)",
              "Callout::new(\"Large radii cost more on integrated GPUs.\").title(\"Warning\").variant(AlertVariant::Warning).bar(true)",
              "Callout::new(\"Blurring a video node disables direct scanout.\")",
              "    .title(\"Caution\")",
              "    .variant(AlertVariant::Destructive)",
              "    .icon(icons::ALERT_OCTAGON)",
              "    .bar(true)",
            ],
          ),
        ),
        Section::new(
          "In a form",
          "A title over the text, and what the app puts under it.",
          self.frame.demo(
            2,
            form,
            ".title(..).child(..)",
            &[
              "Field::new(\"API key\", Input::new(key.clone()).full_width(true))",
              "",
              "Callout::new(\"The key is shown once. Keep it where your team keeps its secrets.\")",
              "    .title(\"Before you go on\")",
              "    .variant(AlertVariant::Warning)",
              "    .child(Link::new(\"How keys are stored\").external())",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a callout is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
