//! The page of the engine's components: a type that keeps state between renders and is given props by what mounts
//! it. Its playground mounts a card whose props are its controls; its sections are the demo's Components page:
//! three cards of one component with other props, and a keyed list whose items keep their state as they move.

use std::sync::{
  Arc,
  atomic::{AtomicU32, Ordering},
};

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    events::MouseEvent,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Rect, Row},
  core::{Ref, Signal},
  layout::{Alignment, text_style::FontWeight},
  node::{Element, border::Border},
  ui::{Button, ButtonSize, ButtonVariant, Switch},
};

use super::kit::{samples, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, mono, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, field, prop},
};

/// What a card says it is: its color is the theme's for it.
#[derive(Clone, Copy, PartialEq, Eq, lurq::DevtoolsInspectable)]
enum Tone {
  Info,
  Warning,
  Error,
}

impl Tone {
  fn color(self) -> PaletteColor {
    match self {
      Self::Info => PaletteColor::Info,
      Self::Warning => PaletteColor::Warning,
      Self::Error => PaletteColor::Destructive,
    }
  }

  fn name(self) -> &'static str {
    match self {
      Self::Info => "Info",
      Self::Warning => "Warning",
      Self::Error => "Error",
    }
  }
}

const TONES: [Tone; 3] = [Tone::Info, Tone::Warning, Tone::Error];
/// The choices of the playground's select: an index into [`TONES`], which a signal keeps.
const TONE_CHOICES: [(&str, usize); 3] = [("Info", 0), ("Warning", 1), ("Error", 2)];

/// The props of a card: what the parent gives it each time it renders.
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct InfoCardProps {
  title: String,
  body: String,
  tone: Tone,
}

/// A card that shows its props, and counts its own renders: a parent that mounts it with equal props does not
/// render it again.
struct InfoCard {
  renders: Ref<u32>,
}

impl Component for InfoCard {
  type Props = InfoCardProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      renders: ctx.create_ref(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.update(|count| *count += 1);
    let props = ctx.props::<InfoCardProps>();
    let accent = props.tone.color();
    Column::new()
      .width(240.0)
      .max_width(FILL)
      .spacing(8.0)
      .padding_horizontal(16.0)
      .padding_vertical(12.0)
      .rounded(RadiusSize::Lg)
      .background(PaletteColor::Card)
      .border(Border::inside(2.0, accent.clone()))
      .child(text(&props.title, 16.0, FontWeight::Bold).color(accent).width(FILL))
      .child(Rect::new(FILL, 1.0).background(PaletteColor::Border))
      .child(text(&props.body, 13.0, FontWeight::Normal).width(FILL))
      .child(quiet(&format!("rendered {} times", self.renders.get()), 11.0))
  }
}

/// An item of the keyed list: its key, and what it is called.
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct ListItem {
  key: String,
  label: String,
}

impl ListItem {
  fn new(key: &str) -> Self {
    Self {
      key: key.to_owned(),
      label: format!("Item {}", key.to_ascii_uppercase()),
    }
  }
}

/// How many rows of the list were ever made: each takes the next number when it is created, and keeps it.
static MADE: AtomicU32 = AtomicU32::new(0);

/// A row of the list with state of its own: the number it was made with, and how many times it was pressed.
struct KeyedRow {
  made: u32,
  presses: Signal<i32>,
}

impl Component for KeyedRow {
  type Props = ListItem;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      made: MADE.fetch_add(1, Ordering::Relaxed) + 1,
      presses: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let item = ctx.props::<ListItem>();
    let presses = self.presses.clone();
    Row::new()
      .width(FILL)
      .min_height(44.0)
      .padding_horizontal(12.0)
      .padding_vertical(6.0)
      .spacing(12.0)
      .align_items(Alignment::Center)
      .rounded(RadiusSize::Md)
      .background(PaletteColor::Background)
      .border(crate::kit::line())
      .child(
        mono(&format!("key \"{}\"", item.key), 12.0)
          .color(PaletteColor::MutedForeground)
          .nowrap(),
      )
      .child(
        Column::new()
          .flex(1.0)
          .min_width(0.0)
          .child(text(&item.label, 14.0, FontWeight::Medium).width(FILL))
          .child(quiet(
            &format!("made #{}, pressed {} times", self.made, self.presses.get()),
            11.0,
          )),
      )
      .child(
        Button::new("+1")
          .variant(ButtonVariant::Outline)
          .size(ButtonSize::Sm)
          .on_click(move |_: MouseEvent| presses.update(|count| *count += 1)),
      )
  }
}

pub struct ComponentsPage {
  frame: Frame,
  playground: Playground,
  title: Signal<String>,
  body: Signal<String>,
  /// An index into [`TONES`].
  tone: Signal<usize>,
  items: Signal<Vec<ListItem>>,
  /// The letter the next item added is keyed with, as a number.
  next: Signal<u32>,
  keyed: Signal<bool>,
}

impl ComponentsPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (title, body, tone_at) = (self.title.get(), self.body.get(), self.tone.get());
    let tone = TONES[tone_at.min(TONES.len() - 1)];
    let props = InfoCardProps {
      title: title.clone(),
      body: body.clone(),
      tone,
    };
    {
      let props = props.clone();
      self
        .playground
        .window_shows(move |ctx| ctx.mount::<InfoCard>(props.clone()));
    }
    let code = vec![
      "ctx.mount::<InfoCard>(InfoCardProps {".to_owned(),
      format!("    title: {title:?}.to_owned(),"),
      format!("    body: {body:?}.to_owned(),"),
      format!("    tone: Tone::{},", tone.name()),
      "})".to_owned(),
      String::new(),
      "// impl Component for InfoCard: in render,".to_owned(),
      "let props = ctx.props::<InfoCardProps>();".to_owned(),
    ];
    let tree = vec![
      format!("InfoCard   {}", self.playground.measured()),
      "└─ column".to_owned(),
      format!("   ├─ text   {title:?}"),
      "   ├─ rect   a line".to_owned(),
      format!("   ├─ text   {body:?}"),
      "   └─ text   its renders".to_owned(),
    ];
    let set_tone = self.tone.clone();
    let reset = {
      let (title, body, tone) = (self.title.clone(), self.body.clone(), self.tone.clone());
      Arc::new(move || {
        title.set("Info".to_owned());
        body.set("Some info content".to_owned());
        tone.set(0);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Component",
        // The card is mounted: the window of its own mounts it there (`window_shows`), and this is not called.
        make: Arc::new(|| Row::new().into()),
        shown: Shown::built(move |ctx| ctx.mount::<InfoCard>(props)),
        fill: false,
        forceable: false,
        facts: format!("tone {}", tone.name()),
        event: "",
        code,
        tree,
        controls: vec![
          prop("title", "String", field(&self.title)),
          prop("body", "String", field(&self.body)),
          prop(
            "tone",
            "Tone",
            self
              .playground
              .select(0, &TONE_CHOICES, &tone_at, move |value| set_tone.set(value)),
          ),
          divider(),
        ],
        changed: [title != "Info", body != "Some info content", tone_at != 0]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }

  /// One component three times, with other props.
  fn cards(ctx: &mut Ctx) -> Row {
    let card = |ctx: &mut Ctx, tone: Tone, body: &str| {
      ctx.mount::<InfoCard>(InfoCardProps {
        title: tone.name().to_owned(),
        body: body.to_owned(),
        tone,
      })
    };
    samples()
      .child(card(ctx, Tone::Info, "Some info content"))
      .child(card(ctx, Tone::Warning, "Be careful."))
      .child(card(ctx, Tone::Error, "Something went wrong."))
  }

  /// The list, its buttons, and the switch between keys and places.
  fn list(&self, ctx: &mut Ctx) -> Column {
    let keyed = self.keyed.get();
    let items = self.items.get();
    let rows: Vec<Element> = if keyed {
      ctx.for_each(
        items.clone(),
        |item| item.key.clone(),
        |ctx, item| ctx.mount::<KeyedRow>(item),
      )
    } else {
      // By its place: the component in the first slot stays the first, whichever item it is given.
      items.iter().map(|item| ctx.mount::<KeyedRow>(item.clone())).collect()
    };
    let add = {
      let (items, next) = (self.items.clone(), self.next.clone());
      move |_: MouseEvent| {
        let letter = char::from_u32(next.get()).unwrap_or('?');
        items.update(|items| items.push(ListItem::new(&letter.to_string())));
        next.set(next.get() + 1);
      }
    };
    let shuffle = {
      let items = self.items.clone();
      move |_: MouseEvent| {
        items.update(|items| {
          if items.len() > 1 {
            items.rotate_left(1);
          }
        })
      }
    };
    let remove = {
      let items = self.items.clone();
      move |_: MouseEvent| {
        items.update(|items| {
          items.pop();
        })
      }
    };
    let set_keyed = self.keyed.clone();
    let small = |label: &str| Button::new(label).variant(ButtonVariant::Outline).size(ButtonSize::Sm);
    let mut list = well().padding(12.0).spacing(8.0).with_children(rows);
    if items.is_empty() {
      list = list.child(quiet("No items: add one.", 12.0));
    }
    Column::new()
      .width(FILL)
      .spacing(12.0)
      .child(
        Row::new()
          .width(FILL)
          .spacing(8.0)
          .wrap()
          .align_items(Alignment::Center)
          .child(small("Add item").on_click(add))
          .child(small("Shuffle").on_click(shuffle))
          .child(small("Remove last").disabled(items.is_empty()).on_click(remove)),
      )
      .child(
        Switch::new("Keyed with for_each")
          .on(keyed)
          .on_change(move |on| set_keyed.set(on)),
      )
      .child(list)
      .child(quiet(
        if keyed {
          "Shuffling preserves component state: each row keeps its number and its presses."
        } else {
          "By place: a row's state stays where it is, and another item's label comes to it."
        },
        12.0,
      ))
  }
}

impl Component for ComponentsPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      title: ctx.signal("Info".to_owned()),
      body: ctx.signal("Some info content".to_owned()),
      tone: ctx.signal(0),
      items: ctx.signal(vec![ListItem::new("a"), ListItem::new("b"), ListItem::new("c")]),
      next: ctx.signal(u32::from('d')),
      keyed: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "type Props",
        "Send + PartialEq",
        "()",
        "What the parent gives the component each time it mounts it. Equal props render it no more.",
      ),
      (
        "create",
        "fn(&mut Ctx) -> Self",
        "",
        "Called once, when the component is mounted: where its signals, memos and timers are made.",
      ),
      (
        "render",
        "fn(&self, &mut Ctx)",
        "",
        "Builds what the component shows, from its state and ctx.props::<Props>().",
      ),
      (
        "on_mounted / on_unmounted",
        "fn(&self)",
        "",
        "Called when the component comes into the tree and when it leaves it.",
      ),
      (
        "ctx.mount",
        "C::Props -> Element",
        "",
        "Mounts a component where the element stands. A component mounted at the same place in the next render \
         is the same one, with its state.",
      ),
      (
        "ctx.mount_keyed",
        "&str, C::Props",
        "",
        "Mounts a component by a key: the same key in the next render is the same component, wherever it is.",
      ),
      (
        "ctx.for_each",
        "items, key, mount",
        "",
        "Mounts a component for each item by the item's key: as the items move, their components move with them.",
      ),
    ];
    let playground = self.play(ctx);
    let cards = Self::cards(ctx);
    let list = self.list(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Components"],
        title: "Components",
        description: "A component is a type that keeps its state between renders and is given props by what \
                      mounts it. Its render builds what it shows; it is rendered again when its state or its \
                      props change.",
        import: Some("use lurq::app::{component::Component, ctx::Ctx};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A card mounted with the props beside it. It counts its own renders: a change of a prop renders it \
           again, and a render of the page with the same props does not.",
          playground,
        ),
        Section::new(
          "Component with props",
          "One component, three times, each with props of its own: a title, a text and a tone.",
          self.frame.demo(
            0,
            cards,
            "ctx.mount::<InfoCard>(props)",
            &[
              "#[derive(Clone, PartialEq)]",
              "struct InfoCardProps { title: String, body: String, tone: Tone }",
              "",
              "impl Component for InfoCard {",
              "    type Props = InfoCardProps;",
              "    fn create(_ctx: &mut Ctx) -> Self { Self }",
              "    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {",
              "        let props = ctx.props::<InfoCardProps>();",
              "        Column::new().child(Text::new(&props.title)).child(Text::new(&props.body))",
              "    }",
              "}",
              "",
              "ctx.mount::<InfoCard>(InfoCardProps { title: \"Warning\".into(), body: \"Be careful.\".into(), \
               tone: Tone::Warning })",
            ],
          ),
        ),
        Section::new(
          "Keyed list (for_each)",
          "Each row is a component with state of its own: the number it was made with, and its presses. Press a \
           row's +1, then Shuffle, which moves the first item to the end: each row keeps its state. Turn the keys \
           off and shuffle again: the state stays in its place, and the labels move.",
          self.frame.demo(
            1,
            list,
            "ctx.for_each(items, |item| item.key.clone(), ..)",
            &[
              "let rows = ctx.for_each(",
              "    items.get(),",
              "    |item| item.key.clone(),",
              "    |ctx, item| ctx.mount::<KeyedRow>(item),",
              ");",
              "Column::new().with_children(rows)",
              "",
              "// Shuffle",
              "items.update(|items| items.rotate_left(1));",
              "// Remove last",
              "items.update(|items| { items.pop(); });",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a component is, and how a parent mounts one.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
