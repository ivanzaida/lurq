//! The page of the persona: its playground, the board's people, the sizes,
//! an avatar of the app's own, and a name longer than its room.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{Avatar, AvatarPresence, AvatarShape, Persona, PersonaSize, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, card, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

const PRESENCES: [(&str, Option<AvatarPresence>); 5] = [
  ("none", None),
  ("online", Some(AvatarPresence::Online)),
  ("away", Some(AvatarPresence::Away)),
  ("busy", Some(AvatarPresence::Busy)),
  ("offline", Some(AvatarPresence::Offline)),
];

const SIZES: [(&str, PersonaSize); 5] = [
  ("xs", PersonaSize::Xs),
  ("sm", PersonaSize::Sm),
  ("md", PersonaSize::Md),
  ("lg", PersonaSize::Lg),
  ("xl", PersonaSize::Xl),
];

/// The names of a list with the index of each: what a control of the panel chooses between.
fn indexed<T>(list: &[(&'static str, T)]) -> Vec<(&'static str, usize)> {
  list
    .iter()
    .enumerate()
    .map(|(index, (name, _))| (*name, index))
    .collect()
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .overflow_visible()
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(32.0)
    .wrap()
    .align_items(Alignment::End)
    .overflow_visible()
}

/// The props of the playground's persona.
#[derive(Clone)]
struct Props {
  name: String,
  description: String,
  presence: usize,
  size: usize,
  bot: bool,
}

impl Props {
  fn persona(&self) -> Persona {
    let mut persona = Persona::new(&self.name)
      .presence(PRESENCES[self.presence].1)
      .size(SIZES[self.size].1);
    if !self.description.is_empty() {
      persona = persona.description(&self.description);
    }
    if self.bot {
      persona = persona.avatar(Avatar::new(&self.name).icon(icons::TERMINAL).shape(AvatarShape::Square));
    }
    persona
  }

  fn code(&self) -> Vec<String> {
    let mut code = vec![format!("Persona::new({:?})", self.name)];
    if !self.description.is_empty() {
      code.push(format!("    .description({:?})", self.description));
    }
    if let Some(presence) = PRESENCES[self.presence].1 {
      code.push(format!("    .presence(AvatarPresence::{presence:?})"));
    }
    if SIZES[self.size].1 != PersonaSize::Md {
      code.push(format!("    .size(PersonaSize::{:?})", SIZES[self.size].1));
    }
    if self.bot {
      code.push(format!(
        "    .avatar(Avatar::new({:?}).icon(icons::TERMINAL).shape(AvatarShape::Square))",
        self.name
      ));
    }
    code
  }

  fn tree(&self, measured: String) -> Vec<String> {
    let mut tree = vec![
      format!("root          {measured}   group {:?}", self.name),
      format!(
        "├─ avatar     {}{}",
        if self.bot {
          "an icon, square"
        } else {
          "the initials of the name"
        },
        PRESENCES[self.presence]
          .1
          .map_or(String::new(), |presence| format!(", {presence:?}").to_lowercase())
      ),
      "└─ text".to_owned(),
    ];
    if self.description.is_empty() {
      tree.push(format!("   └─ name    {:?}", self.name));
    } else {
      tree.push(format!("   ├─ name    {:?}", self.name));
      tree.push(format!("   └─ description {:?}", self.description));
    }
    tree
  }
}

pub struct PersonaPage {
  frame: Frame,
  playground: Playground,
  name: Signal<String>,
  description: Signal<String>,
  /// The indices of the presence and the size in their lists.
  presence: Signal<usize>,
  size: Signal<usize>,
  bot: Signal<bool>,
}

impl PersonaPage {
  fn props(&self) -> Props {
    Props {
      name: self.name.get(),
      description: self.description.get(),
      presence: self.presence.get(),
      size: self.size.get(),
      bot: self.bot.get(),
    }
  }

  /// The playground: the persona as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let (presence, size) = (self.presence.clone(), self.size.clone());
    let controls = vec![
      prop("name", "&str", field(&self.name)),
      prop("description", "&str", field(&self.description)),
      prop(
        "presence",
        "Option<AvatarPresence>",
        segmented_fill(&indexed(&PRESENCES), &self.presence.get(), move |chosen| {
          presence.set(chosen)
        }),
      ),
      prop(
        "size",
        "PersonaSize",
        segmented_fill(&indexed(&SIZES), &self.size.get(), move |chosen| size.set(chosen)),
      ),
      divider(),
      flag("an avatar of its own", &self.bot),
    ];
    let changed = [
      props.name != "Ivan Zaida",
      props.description != "Available · Kyiv",
      props.presence != 1,
      props.size != 2,
      props.bot,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    let reset = {
      let (name, description) = (self.name.clone(), self.description.clone());
      let (presence, size, bot) = (self.presence.clone(), self.size.clone(), self.bot.clone());
      Arc::new(move || {
        name.set("Ivan Zaida".to_owned());
        description.set("Available · Kyiv".to_owned());
        presence.set(1);
        size.set(2);
        bot.set(false);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Persona",
        make: {
          let props = props.clone();
          Arc::new(move || props.persona().into())
        },
        shown: Shown::of(self.playground.styled(props.persona())),
        fill: false,
        forceable: false,
        facts: format!("avatar {}", SIZES[props.size].0),
        event: "",
        code: props.code(),
        tree: props.tree(self.playground.measured()),
        controls,
        changed,
        reset,
      },
    )
  }
}

impl Component for PersonaPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      name: ctx.signal("Ivan Zaida".to_owned()),
      description: ctx.signal("Available · Kyiv".to_owned()),
      presence: ctx.signal(1),
      size: ctx.signal(2),
      bot: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let people = items()
      .child(item(
        Persona::new("Ivan Zaida")
          .description("Available · Kyiv")
          .presence(AvatarPresence::Online),
        "persona",
      ))
      .child(item(
        Persona::new("Ivy Chen")
          .description("Away · back at 15:00")
          .presence(AvatarPresence::Away),
        "persona · away",
      ))
      .child(item(
        Persona::new("Mark Rossi")
          .description("In a call")
          .presence(AvatarPresence::Busy),
        "busy",
      ))
      .child(item(
        Persona::new("Lena Quist")
          .description("Last seen yesterday")
          .presence(AvatarPresence::Offline),
        "offline",
      ))
      .child(item(Persona::new("Anna Kovac"), "a name alone"));
    let sizes = SIZES.iter().fold(items(), |row, (name, size)| {
      row.child(item(
        Persona::new("Ivy Chen")
          .description("Designer")
          .presence(AvatarPresence::Online)
          .size(*size),
        name,
      ))
    });
    let own = items()
      .child(item(
        Persona::new("Build bot").description("Runs on every push").avatar(
          Avatar::new("Build bot")
            .icon(icons::TERMINAL)
            .shape(AvatarShape::Square),
        ),
        "an icon, square",
      ))
      .child(item(
        Persona::new("Design systems")
          .description("12 members")
          .avatar(Avatar::new("Design systems").shape(AvatarShape::Square)),
        "a team",
      ));
    let narrow = card()
      .width(220.0)
      .max_width(FILL)
      .padding(12.0)
      .spacing(12.0)
      .overflow_visible()
      .child(
        Persona::new("Maximiliana Alexandrovna von Hohenzollern")
          .description("Away until the end of the next sprint")
          .presence(AvatarPresence::Away),
      )
      .child(
        Persona::new("Ivan Zaida")
          .description("Available · Kyiv")
          .presence(AvatarPresence::Online),
      );
    let rows = [
      (
        "name",
        "&str",
        "",
        "Who it is: the name beside the avatar, the avatar's initials, and what tooling calls the persona.",
      ),
      (
        "description",
        "&str",
        "",
        "The line under the name, in the muted color: where the person is, what they do, when they are back.",
      ),
      (
        "presence",
        "impl Into<Option<AvatarPresence>>",
        "None",
        "The dot at the corner of the avatar: Online, Away, Busy or Offline.",
      ),
      (
        "presence_label",
        "&str",
        "",
        "What the dot says to tooling in place of the presence's own word.",
      ),
      (
        "avatar",
        "Avatar",
        "the name's",
        "An avatar of the app's own, with a picture, an icon or a shape. The persona gives it its size and its \
         presence.",
      ),
      (
        "size",
        "impl Into<Name>",
        "Md",
        "A PersonaSize, the avatar's: 20, 24, 32, 40 or 56, with the text to match.",
      ),
      (
        "style, rules, state",
        "",
        "",
        "Over the theme's recipe: PersonaPart::Root, Avatar (its face), Text, Name, Description.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Persona"],
        title: "Persona",
        description: "A person in a list, a menu or a header: their avatar with the dot that says whether they are \
                      there, their name, and a line under it.",
        import: Some("use lurq::ui::{AvatarPresence, Persona};"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Persona is a control on the right. A persona is read and not operated: it takes no press \
           and no focus.",
          playground,
        ),
        Section::new(
          "People",
          "The avatar of the name, 10 before the name in 14 and medium, with a line 1 under it in 12 of the muted \
           color. The dot takes the color of the presence.",
          self.frame.demo(
            0,
            people,
            ".presence(AvatarPresence::Online)",
            &[
              "Persona::new(\"Ivan Zaida\")",
              "    .description(\"Available · Kyiv\")",
              "    .presence(AvatarPresence::Online)",
              "Persona::new(\"Ivy Chen\")",
              "    .description(\"Away · back at 15:00\")",
              "    .presence(AvatarPresence::Away)",
              "Persona::new(\"Anna Kovac\")",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "The sizes are the avatar's, from 20 to 56, and the text follows: a name of 12 to 18 over a line of 11 \
           to 14.",
          self.frame.demo(
            1,
            sizes,
            ".size(PersonaSize::Lg)",
            &[
              "Persona::new(\"Ivy Chen\")",
              "    .description(\"Designer\")",
              "    .presence(AvatarPresence::Online)",
              "    .size(PersonaSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "An avatar of its own",
          "What is not a person, or has a picture, gives the persona its avatar; the persona gives the avatar its \
           size and its presence.",
          self.frame.demo(
            2,
            own,
            ".avatar(Avatar::new(..).icon(..))",
            &[
              "Persona::new(\"Build bot\")",
              "    .description(\"Runs on every push\")",
              "    .avatar(Avatar::new(\"Build bot\").icon(icons::TERMINAL).shape(AvatarShape::Square))",
            ],
          ),
        ),
        Section::new(
          "In a narrow room",
          "A name or a line longer than the room ends in an ellipsis, each on its one line. The avatar keeps its \
           size.",
          self.frame.demo(
            3,
            narrow,
            "Persona::new(a long name)",
            &[
              "// In a card 220 wide.",
              "Persona::new(\"Maximiliana Alexandrovna von Hohenzollern\")",
              "    .description(\"Away until the end of the next sprint\")",
              "    .presence(AvatarPresence::Away)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a persona is given.",
          props_table(&rows, &["name"], self.frame.room()),
        ),
      ],
    )
  }
}
