//! The page of mentions: its playground, a comment that mentions people and teams, commands after a slash, its
//! states, and a field that holds one.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Avatar, Field, Interaction, Mention, Mentions, MentionsController, TokenInputSize, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// How wide the board draws mentions.
const WIDTH: f32 = 280.0;

/// The sizes, how the code names them, and the type of each.
const SIZES: [(TokenInputSize, &str, &str, f32); 3] = [
  (TokenInputSize::Sm, "sm", "TokenInputSize::Sm", 13.0),
  (TokenInputSize::Md, "md", "TokenInputSize::Md", 14.0),
  (TokenInputSize::Lg, "lg", "TokenInputSize::Lg", 15.0),
];

/// The people the board offers, and a few more.
const PEOPLE: [&str; 6] = [
  "Ivan Zaida",
  "Ivy Chen",
  "Ivo Kovac",
  "Mara Lind",
  "Noor Haddad",
  "Tomas Berg",
];

fn people() -> Vec<Mention> {
  PEOPLE
    .into_iter()
    .map(|name| Mention::new(name).avatar(Avatar::new(name)))
    .collect()
}

/// The teams a `#` offers: what was typed, made a team, and the teams that start with it.
fn teams(typed: &str) -> Vec<Mention> {
  ["design", "engine", "docs", "release"]
    .into_iter()
    .filter(|team| team.starts_with(&typed.to_lowercase()))
    .map(|team| Mention::new(team).icon(icons::USERS))
    .collect()
}

/// The commands a `/` offers.
fn commands() -> Vec<Mention> {
  [
    ("summarize", icons::FILE_TEXT),
    ("translate", icons::MESSAGE),
    ("explain", icons::LIGHTBULB),
  ]
  .into_iter()
  .map(|(name, icon)| Mention::new(name).icon(icon))
  .collect()
}

/// A field as wide as the board draws one, with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .width(WIDTH)
    .spacing(9.0)
    .child(shown)
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::Start)
}

pub struct MentionsPage {
  frame: Frame,
  playground: Playground,
  played: MentionsController,
  value: Signal<String>,
  placeholder: Signal<String>,
  label: Signal<String>,
  size: Signal<usize>,
  at: Signal<bool>,
  hash: Signal<bool>,
  invalid: Signal<bool>,
  warning: Signal<bool>,
  valid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  comment: Signal<String>,
  commented: MentionsController,
  prompt: Signal<String>,
  prompted: MentionsController,
  note: Signal<String>,
  noted: MentionsController,
}

impl MentionsPage {
  /// The playground: the field as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let placeholder = self.placeholder.get();
    let name = self.label.get();
    let (size, size_name, size_code, font_size) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let (at, hash) = (self.at.get(), self.hash.get());
    let flags = [
      ("invalid", self.invalid.get()),
      ("warning", self.warning.get()),
      ("valid", self.valid.get()),
      ("read_only", self.read_only.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
    ];
    let value = self.value.clone();
    let make = {
      let (placeholder, name) = (placeholder.clone(), name.clone());
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move |value: Signal<String>, controller: &MentionsController| {
        let mut mentions = Mentions::new(value, controller)
          .placeholder(&placeholder)
          .size(size)
          .invalid(set[0])
          .warning(set[1])
          .valid(set[2])
          .read_only(set[3])
          .disabled(set[4])
          .full_width(set[5]);
        if at {
          mentions = mentions.trigger('@', people());
        }
        if hash {
          mentions = mentions.trigger('#', teams);
        }
        if !name.is_empty() {
          mentions = mentions.label(&name);
        }
        mentions
      }
    };
    let record = self.playground.recorder();
    let shown = make(value.clone(), &self.played)
      .force(self.playground.forced())
      .on_mention(move |text| record(format!("on_mention({text:?})")));

    let mut code = vec![
      "// value: Signal<String>; mentions: MentionsController, kept with it".to_owned(),
      "Mentions::new(value.clone(), &mentions)".to_owned(),
    ];
    if at {
      code.push("    .trigger('@', people) // Vec<Mention>".to_owned());
    }
    if hash {
      code.push("    .trigger('#', |typed: &str| teams(typed))".to_owned());
    }
    code.push(format!("    .placeholder({placeholder:?})"));
    if !name.is_empty() {
      code.push(format!("    .label({name:?})"));
    }
    if size != TokenInputSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    for (flag, set) in flags {
      if set {
        code.push(format!("    .{flag}(true)"));
      }
    }

    let mentioned = self.played.mentioned();
    let held = value.get();
    let tree = vec![
      format!("root      {}", self.playground.measured()),
      format!("├─ text    {} characters, {font_size} px", held.chars().count()),
      format!("├─ tokens  {} mentioned", mentioned.len()),
      "└─ list    under the trigger, while it offers".to_owned(),
    ];

    let (to_value, to_placeholder, to_label, to_size) = (
      self.value.clone(),
      self.placeholder.clone(),
      self.label.clone(),
      self.size.clone(),
    );
    let (to_at, to_hash) = (self.at.clone(), self.hash.clone());
    let flag_signals = [
      self.invalid.clone(),
      self.warning.clone(),
      self.valid.clone(),
      self.read_only.clone(),
      self.disabled.clone(),
      self.full_width.clone(),
    ];
    let choose_size = self.size.clone();
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    let make = Arc::new(make);
    self.playground.card(
      ctx,
      Spec {
        title: "Mentions",
        make: Arc::new(move || {
          Column::new()
            .width(360.0)
            .child(make(Signal::new(String::new()), &MentionsController::new()))
            .into()
        }),
        shown: Column::new().width(FILL).child(shown).into(),
        fill: true,
        forceable: true,
        facts: format!("{size_name} · {} mentions", mentioned.len()),
        event: "on_mention",
        code,
        tree,
        controls: vec![
          prop("placeholder", "&str", field(&self.placeholder)),
          prop("label", "&str", field(&self.label)),
          prop(
            "size",
            "TokenInputSize",
            segmented_fill(&sizes, &self.size.get(), move |index| choose_size.set(index)),
          ),
          flag("trigger('@', people)", &self.at),
          flag("trigger('#', teams)", &self.hash),
          divider(),
          flag("invalid", &self.invalid),
          flag("warning", &self.warning),
          flag("valid", &self.valid),
          flag("read_only", &self.read_only),
          flag("disabled", &self.disabled),
          flag("full_width", &self.full_width),
        ],
        changed: [
          !held.is_empty(),
          placeholder != "Write a comment",
          !name.is_empty(),
          size != TokenInputSize::Md,
          !at,
          !hash,
        ]
        .into_iter()
        .chain(flags.iter().map(|(_, set)| *set))
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_value.set(String::new());
          to_placeholder.set("Write a comment".to_owned());
          to_label.set(String::new());
          to_size.set(1);
          to_at.set(true);
          to_hash.set(true);
          for flag in &flag_signals {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for MentionsPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let commented = MentionsController::new();
    commented.mention("@Ivan Zaida");
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      played: MentionsController::new(),
      value: ctx.signal(String::new()),
      placeholder: ctx.signal("Write a comment".to_owned()),
      label: ctx.signal(String::new()),
      size: ctx.signal(1),
      at: ctx.signal(true),
      hash: ctx.signal(true),
      invalid: ctx.signal(false),
      warning: ctx.signal(false),
      valid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      comment: ctx.signal("Thanks for the review @Ivan Zaida ".to_owned()),
      commented,
      prompt: ctx.signal(String::new()),
      prompted: MentionsController::new(),
      note: ctx.signal(String::new()),
      noted: MentionsController::new(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mentioned: Vec<String> = self.commented.mentioned().iter().map(|text| text.to_string()).collect();
    let comment = items().child(item(
      Mentions::new(self.comment.clone(), &self.commented)
        .trigger('@', people())
        .trigger('#', teams)
        .placeholder("Write a comment"),
      &format!("mentioned: {}", mentioned.join(", ")),
    ));
    let prompt = items().child(item(
      Mentions::new(self.prompt.clone(), &self.prompted)
        .trigger('/', commands())
        .rows(1, 6)
        .placeholder("Ask, or / for a command"),
      "a command after a slash",
    ));
    let filled = |text: &str| {
      let controller = MentionsController::new();
      controller.mention("@Ivy Chen");
      (Signal::new(text.to_owned()), controller)
    };
    let state = |caption: &str, change: &dyn Fn(Mentions) -> Mentions| {
      let (value, controller) = filled("Ping @Ivy Chen about the release.");
      item(
        change(Mentions::new(value, &controller).trigger('@', people())),
        caption,
      )
    };
    let states = items()
      .child(state("default", &|mentions| mentions))
      .child(state("hover", &|mentions| mentions.force(Some(Interaction::Hovered))))
      .child(state("focus", &|mentions| {
        mentions.force(Some(Interaction::FocusVisible))
      }))
      .child(state("invalid", &|mentions| mentions.invalid(true)))
      .child(state("read only", &|mentions| mentions.read_only(true)))
      .child(state("disabled", &|mentions| mentions.disabled(true)));
    let note = self.note.get();
    let in_field = {
      let field = Field::new(
        "Hand-off note",
        Mentions::new(self.note.clone(), &self.noted)
          .trigger('@', people())
          .placeholder("Who takes it over? @ to mention")
          .full_width(true),
      )
      .full_width(true);
      if self.noted.mentioned().iter().any(|text| note.contains(text.as_ref())) {
        field.description("Whoever is mentioned is told.").valid(true)
      } else {
        field.error("Mention who takes it over.")
      }
    };
    let in_field = items().child(Column::new().width(360.0).child(in_field));
    let props = [
      (
        "value",
        "Signal<String>",
        "",
        "The text, mentions and all, each as the text of its offer.",
      ),
      (
        "controller",
        "&MentionsController",
        "",
        "What the field keeps from one render to the next, kept by the app with the value.",
      ),
      (
        "trigger",
        "(char, Vec<Mention> or Fn(&str) -> Vec<Mention>)",
        "",
        "What a character at the start of a word offers: a list narrowed to what is typed, or what a function finds.",
      ),
      (
        "Mention",
        "label, text, avatar, icon",
        "",
        "An offer: its label in the list, its avatar or icon, and the text it puts in the field.",
      ),
      (
        "on_mention",
        "impl Fn(&str)",
        "",
        "Called with the text of an offer as it is chosen.",
      ),
      (
        "tokens",
        "impl Fn(&str) -> Vec<Token>",
        "",
        "Other tokens of the app's in the text, besides the mentions.",
      ),
      (
        "rows",
        "(usize, usize)",
        "2, 8",
        "The rows it shows at least and grows to before it scrolls.",
      ),
      ("placeholder", "&str", "", "What it shows while it holds nothing."),
      (
        "size",
        "TokenInputSize",
        "Md",
        "The type of its text and mentions: 13, 14 or 15.",
      ),
      (
        "invalid",
        "bool",
        "false",
        "What it holds is wrong: the destructive color.",
      ),
      ("warning", "bool", "false", "Allowed with a caveat: the warning color."),
      ("valid", "bool", "false", "Checked and right: the success color."),
      (
        "read_only",
        "bool",
        "false",
        "Read and copied, not changed: a trigger opens nothing.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Dimmed; it takes no input and Tab passes it by.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 280.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called, to what reads the tree. A field gives it its own label.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Mentions"],
        title: "Mentions",
        description: "Text in which a trigger, @, # or /, opens a list of what can be mentioned under it. What is \
                      chosen stands in the text as a token: one thing to the caret, and the app's text for it in the \
                      value.",
        import: Some("use lurq::ui::{Mention, Mentions, MentionsController};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Mentions is a control on the right. The field on the stage is live: type @ and a few letters, \
           move with Up and Down, choose with Enter or a click, put the list away with Escape.",
          playground,
        ),
        Section::new(
          "Mentions",
          "The page keeps the text in a signal and the field's controller beside it. @ offers people, narrowed to \
           what is typed; # offers what a function finds for it. The comment starts with a mention the controller \
           was told of.",
          self.frame.demo(
            0,
            comment,
            "Mentions::new(comment.clone(), &mentions).trigger('@', people)",
            &[
              "// comment: Signal<String>; mentions: MentionsController, kept with it",
              "let people = vec![",
              "    Mention::new(\"Ivan Zaida\").avatar(Avatar::new(\"Ivan Zaida\")),",
              "    Mention::new(\"Ivy Chen\").avatar(Avatar::new(\"Ivy Chen\")),",
              "];",
              "mentions.mention(\"@Ivan Zaida\"); // a mention the loaded text already has",
              "Mentions::new(comment.clone(), &mentions)",
              "    .trigger('@', people)",
              "    .trigger('#', |typed: &str| teams(typed))",
              "    .placeholder(\"Write a comment\")",
            ],
          ),
        ),
        Section::new(
          "Commands",
          "Any character can be a trigger, and an offer can have an icon in place of an avatar: a prompt box with \
           commands after a slash.",
          self.frame.demo(
            1,
            prompt,
            ".trigger('/', commands)",
            &[
              "let commands = vec![",
              "    Mention::new(\"summarize\").icon(icons::FILE_TEXT),",
              "    Mention::new(\"translate\").icon(icons::MESSAGE),",
              "];",
              "Mentions::new(prompt.clone(), &prompted)",
              "    .trigger('/', commands)",
              "    .rows(1, 6)",
              "    .placeholder(\"Ask, or / for a command\")",
            ],
          ),
        ),
        Section::new(
          "States",
          "The box is a token input's on rows, and its edge says its state. A read-only one opens no list.",
          self.frame.demo(
            2,
            states,
            ".invalid(true)",
            &[
              "Mentions::new(value.clone(), &mentions).trigger('@', people)",
              "    .force(Some(Interaction::Hovered))",
              "    .force(Some(Interaction::FocusVisible))",
              "    .invalid(true)",
              "    .read_only(true)",
              "    .disabled(true)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names and judges the field as it does any input: here it is invalid until someone is mentioned.",
          self.frame.demo(
            3,
            in_field,
            "Field::new(\"Hand-off note\", mentions)",
            &[
              "let mentioned = noted.mentioned().iter().any(|text| note.get().contains(text.as_ref()));",
              "let field = Field::new(",
              "    \"Hand-off note\",",
              "    Mentions::new(note.clone(), &noted).trigger('@', people).full_width(true),",
              ")",
              ".full_width(true);",
              "if mentioned { field.description(\"Whoever is mentioned is told.\").valid(true) }",
              "else { field.error(\"Mention who takes it over.\") }",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a mentions field is given.",
          props_table(&props, &["value", "controller"], self.frame.room()),
        ),
      ],
    )
  }
}
