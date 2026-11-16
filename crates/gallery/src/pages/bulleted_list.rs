//! The page of the list: its playground, the board's list, its numbers, its icons and marks, and a list in an item.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, TypographyStyle},
  },
  components::{Column, Rect, Text},
  core::Signal,
  node::{Element, dimension::Dimension},
  ui::{BulletedList, BulletedListItem, BulletedListMarker, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, field, flag, prop},
};

/// The board's items (`btfDc`).
const ROLES: [&str; 3] = [
  "Semantic color roles",
  "Light and dark palettes",
  "Extra roles for app-specific needs",
];

/// An item longer than the stage is wide, for the playground: it goes on to a second line.
const LONG: &str = "Extra roles for app-specific needs, which a theme names and a recipe reads as it reads the roles lurq \
                    has, in light and in dark";

/// What stands before the items of the playground's list.
#[derive(Clone, Copy, PartialEq, lurq::DevtoolsInspectable)]
enum Mark {
  Bullet,
  Number,
  Check,
  None,
}

impl Mark {
  fn marker(self) -> BulletedListMarker {
    match self {
      Self::Bullet => BulletedListMarker::Bullet,
      Self::Number => BulletedListMarker::Number,
      Self::Check => BulletedListMarker::Icon(icons::CHECK.into()),
      Self::None => BulletedListMarker::None,
    }
  }

  fn code(self) -> Option<&'static str> {
    match self {
      Self::Bullet => None,
      Self::Number => Some("    .marker(BulletedListMarker::Number)"),
      Self::Check => Some("    .marker(icons::CHECK)"),
      Self::None => Some("    .marker(BulletedListMarker::None)"),
    }
  }
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str, width: f32) -> Column {
  Column::new()
    .max_width(FILL)
    .width(width)
    .spacing(10.0)
    .child(shown)
    .child(quiet(caption, 12.0))
}

pub struct BulletedListPage {
  frame: Frame,
  playground: Playground,
  first: Signal<String>,
  mark: Signal<Mark>,
  start: Signal<String>,
  nested: Signal<bool>,
  long: Signal<bool>,
  label: Signal<String>,
}

impl BulletedListPage {
  /// The playground: the list as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (first, mark, nested, long) = (self.first.get(), self.mark.get(), self.nested.get(), self.long.get());
    let (label, start_text) = (self.label.get(), self.start.get());
    let start = start_text.trim().parse::<u64>().unwrap_or(1);
    let numbered = mark == Mark::Number;
    let make = {
      let (first, label) = (first.clone(), label.clone());
      move || -> BulletedList {
        let mut list = BulletedList::new([first.as_str(), ROLES[1]]).marker(mark.marker());
        list = if nested {
          list.item(BulletedListItem::new(ROLES[2]).list(BulletedList::new(["Brand", "Chart series"])))
        } else {
          list.item(ROLES[2])
        };
        if long {
          list = list.item(LONG);
        }
        if numbered && start != 1 {
          list = list.start(start);
        }
        if !label.is_empty() {
          list = list.label(&label);
        }
        list
      }
    };
    let mut code = vec![format!("BulletedList::new([{first:?}, {:?}])", ROLES[1])];
    code.extend(mark.code().map(str::to_owned));
    if nested {
      code.push(format!(
        "    .item(BulletedListItem::new({:?}).list(BulletedList::new([\"Brand\", \"Chart series\"])))",
        ROLES[2]
      ));
    } else {
      code.push(format!("    .item({:?})", ROLES[2]));
    }
    if long {
      code.push("    .item(\"Extra roles for app-specific needs, which a theme names ..\")".to_owned());
    }
    if numbered && start != 1 {
      code.push(format!("    .start({start})"));
    }
    if !label.is_empty() {
      code.push(format!("    .label({label:?})"));
    }
    let mark_line = match mark {
      Mark::Bullet => "│  ├─ mark    one line high · bullet 5 · MutedForeground",
      Mark::Number => "│  ├─ mark    18 wide · the number at its end · MutedForeground",
      Mark::Check => "│  ├─ mark    14 wide · icon 14 · MutedForeground",
      Mark::None => "│  ├─ (no mark)",
    };
    let mut tree = vec![
      format!("root       {}   items 6 apart", self.playground.measured()),
      format!("├─ item    mark and body {} apart", if numbered { 8 } else { 10 }),
      mark_line.to_owned(),
      "│  └─ body    label 14 · Foreground · wraps".to_owned(),
    ];
    if nested {
      tree.push("└─ item    its body holds a list: rings, set in to the text".to_owned());
    }
    let reset = {
      let (first, mark, start) = (self.first.clone(), self.mark.clone(), self.start.clone());
      let (nested, long, label) = (self.nested.clone(), self.long.clone(), self.label.clone());
      Arc::new(move || {
        first.set(ROLES[0].to_owned());
        mark.set(Mark::Bullet);
        start.set("1".to_owned());
        nested.set(false);
        long.set(false);
        label.set(String::new());
      })
    };
    let choose = self.mark.clone();
    let mut controls = vec![
      prop("new", "items", field(&self.first)),
      prop(
        "marker",
        "BulletedListMarker",
        self.playground.select(
          0,
          &[
            ("Bullet", Mark::Bullet),
            ("Number", Mark::Number),
            ("Icon (check)", Mark::Check),
            ("None", Mark::None),
          ],
          &mark,
          move |mark| choose.set(mark),
        ),
      ),
    ];
    if numbered {
      controls.push(prop("start", "u64", field(&self.start)));
    }
    controls.extend([
      flag("item · list", &self.nested),
      flag("a long item", &self.long),
      prop("label", "&str", field(&self.label)),
    ]);
    self.playground.card(
      ctx,
      Spec {
        title: "Bulleted List",
        make: Arc::new({
          let make = make.clone();
          move || make().into()
        }),
        shown: Shown::of(make()),
        fill: true,
        forceable: false,
        facts: match mark {
          Mark::Bullet => "bullets".to_owned(),
          Mark::Number => "numbers".to_owned(),
          Mark::Check => "icons".to_owned(),
          Mark::None => "no marks".to_owned(),
        },
        event: "",
        code,
        tree,
        controls,
        changed: [
          first != ROLES[0],
          mark != Mark::Bullet,
          numbered && start != 1,
          nested,
          long,
          !label.is_empty(),
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }
}

impl Component for BulletedListPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      first: ctx.signal(ROLES[0].to_owned()),
      mark: ctx.signal(Mark::Bullet),
      start: ctx.signal("1".to_owned()),
      nested: ctx.signal(false),
      long: ctx.signal(false),
      label: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = item(BulletedList::new(ROLES), "the board's list", 380.0);
    let numbered = item(
      BulletedList::new(["Add lurq to Cargo.toml", "Write a component", "Run it"]).marker(BulletedListMarker::Number),
      "marker(Number)",
      380.0,
    );
    let lettered = item(
      BulletedList::new(["Tokens", "Recipes", "Renderers"])
        .marker(BulletedListMarker::Number)
        .number_text(|number| format!("{})", (b'a' + (number - 1) as u8 % 26) as char)),
      "number_text",
      380.0,
    );
    let checks = item(
      BulletedList::new(["Signed", "Notarized", "Published"]).marker(icons::CHECK),
      "marker(icons::CHECK)",
      380.0,
    );
    let own = item(
      BulletedList::new([
        BulletedListItem::new("Build passed").media(Rect::new(8.0, 8.0).rounded(4.0).background(PaletteColor::Success)),
        BulletedListItem::new("Two warnings").media(Rect::new(8.0, 8.0).rounded(4.0).background(PaletteColor::Warning)),
        BulletedListItem::new("Deploy held").icon(icons::LOCK),
      ]),
      "media · icon of an item",
      380.0,
    );
    let nested = item(
      BulletedList::new(["Tokens"]).item(BulletedListItem::new("Components").list(
        BulletedList::new(["Button"]).item(BulletedListItem::new("Switch").list(BulletedList::new(["Track", "Thumb"]))),
      )),
      "list in an item",
      380.0,
    );
    let mixed = item(
      BulletedList::new([
        BulletedListItem::new("Install")
          .list(BulletedList::new(["Add the crate", "Enable the features you use"]).marker(BulletedListMarker::Number)),
        BulletedListItem::new("Run").child(
          Text::new("The first window opens with the theme of the system.")
            .variant(TypographyStyle::Prose)
            .color(PaletteColor::MutedForeground)
            .width(Dimension::Pct(100.0)),
        ),
      ]),
      "numbers in bullets · child",
      420.0,
    );
    let rows = [
      (
        "new",
        "impl IntoIterator<Item = impl Into<BulletedListItem>>",
        "",
        "Its items: texts, or BulletedListItems. item and items add more.",
      ),
      (
        "marker",
        "impl Into<BulletedListMarker>",
        "Bullet",
        "What stands before each item: Bullet, Number, Icon(name), or None. The name of an icon is a marker.",
      ),
      ("start", "u64", "1", "The number of the first item of a numbered list."),
      (
        "number_text",
        "Fn(u64) -> String",
        "1.",
        "How a numbered list writes a number: a letter, a bracket, another script.",
      ),
      (
        "label",
        "&str",
        "",
        "What the list is called to tooling, which reads a list of listitems.",
      ),
      (
        "BulletedListItem::new",
        "&str",
        "",
        "What the item says; it wraps at the list's width. An empty one says nothing.",
      ),
      (
        "child / children",
        "impl Into<Element>",
        "",
        "Elements under the item's line, 6 apart: a paragraph, a block of code.",
      ),
      (
        "list",
        "BulletedList",
        "",
        "A list in the item: set in to the item's text, its bullets rings, with the marker it was given.",
      ),
      (
        "icon",
        "impl Into<Name>",
        "",
        "An icon as this item's mark, in place of the list's.",
      ),
      (
        "media",
        "impl Into<Element>",
        "",
        "An element of the app's as this item's mark: a status dot, a picture.",
      ),
    ];
    let playground = self.play(ctx);
    let lists = Column::new().width(FILL).spacing(28.0).child(numbered).child(lettered);
    let marks = Column::new().width(FILL).spacing(28.0).child(checks).child(own);
    let inner = Column::new().width(FILL).spacing(28.0).child(nested).child(mixed);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Bulleted List"],
        title: "Bulleted List",
        description: "A list of lines, each with a mark before it: a bullet, a number, an icon or a mark of the app's \
                      own, with lists in its items. Markdown's lists are drawn with the same numbers. It is read, and \
                      is no stop of Tab.",
        import: Some("use lurq::ui::{BulletedList, BulletedListItem, BulletedListMarker};"),
        status: Some("Beta"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of BulletedList is a control on the right.",
          playground,
        ),
        Section::new(
          "Bulleted List",
          "The board's list: a dot of 5 in the muted color in the middle of each first line, 10 before the text.",
          self.frame.demo(
            0,
            board,
            "BulletedList::new(..)",
            &[
              "BulletedList::new([",
              "  \"Semantic color roles\",",
              "  \"Light and dark palettes\",",
              "  \"Extra roles for app-specific needs\",",
              "])",
            ],
          ),
        ),
        Section::new(
          "Numbers",
          "A numbered list sets its numbers at the end of a width of 18, 8 before the text, and writes them as it \
           is told.",
          self.frame.demo(
            1,
            lists,
            ".marker(BulletedListMarker::Number)",
            &[
              "BulletedList::new([\"Add lurq to Cargo.toml\", \"Write a component\", \"Run it\"])",
              "    .marker(BulletedListMarker::Number)",
              "",
              "BulletedList::new([\"Tokens\", \"Recipes\", \"Renderers\"])",
              "    .marker(BulletedListMarker::Number)",
              "    .number_text(|number| format!(\"{})\", (b'a' + (number - 1) as u8 % 26) as char))",
            ],
          ),
        ),
        Section::new(
          "Icons and marks",
          "An icon of the theme's set for every item, or an icon or an element of the app's own for one: the lines \
           start together after an icon's room.",
          self.frame.demo(
            2,
            marks,
            ".marker(icons::CHECK)",
            &[
              "BulletedList::new([\"Signed\", \"Notarized\", \"Published\"]).marker(icons::CHECK)",
              "",
              "BulletedList::new([",
              "  BulletedListItem::new(\"Build passed\")",
              "    .media(Rect::new(8.0, 8.0).rounded(4.0).background(PaletteColor::Success)),",
              "  BulletedListItem::new(\"Two warnings\")",
              "    .media(Rect::new(8.0, 8.0).rounded(4.0).background(PaletteColor::Warning)),",
              "  BulletedListItem::new(\"Deploy held\").icon(icons::LOCK),",
              "])",
            ],
          ),
        ),
        Section::new(
          "Lists in items",
          "A list in an item starts where the item's text does and has rings for dots; it keeps the marker it was \
           given, and an item holds elements under its line.",
          self.frame.demo(
            3,
            inner,
            ".list(BulletedList::new(..))",
            &[
              "BulletedList::new([\"Tokens\"]).item(",
              "  BulletedListItem::new(\"Components\").list(",
              "    BulletedList::new([\"Button\"])",
              "      .item(BulletedListItem::new(\"Switch\").list(BulletedList::new([\"Track\", \"Thumb\"]))),",
              "  ),",
              ")",
              "",
              "BulletedList::new([",
              "  BulletedListItem::new(\"Install\").list(",
              "    BulletedList::new([\"Add the crate\", \"Enable the features you use\"])",
              "      .marker(BulletedListMarker::Number),",
              "  ),",
              "  BulletedListItem::new(\"Run\")",
              "    .child(Text::new(\"The first window opens ..\").variant(TypographyStyle::Prose)),",
              "])",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a list and its items are given.",
          props_table(&rows, &["new", "BulletedListItem::new"], self.frame.room()),
        ),
      ],
    )
  }
}
