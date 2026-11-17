//! The page of the property grid: its playground, the board's inspector, every kind of property, a changed value
//! and its reset, and rows that are read only or disabled.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row, SelectOption},
  core::Signal,
  layout::Alignment,
  node::{Element, color::Color},
  ui::{Button, ButtonSize, ButtonVariant, Property, PropertyCategory, PropertyGrid, icons},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, field, flag, prop},
};

/// The widths of the names the playground offers.
const WIDTHS: [(&str, f32); 3] = [("84", 84.0), ("110", 110.0), ("140", 140.0)];

/// The values of the board's inspector, which every grid of the page edits.
#[derive(Clone)]
struct Values {
  x: Signal<f64>,
  y: Signal<f64>,
  w: Signal<f64>,
  h: Signal<f64>,
  rotation: Signal<f64>,
  fill: Signal<Color>,
  opacity: Signal<f32>,
  radius: Signal<f64>,
  blend: Signal<u8>,
  visible: Signal<bool>,
  name: Signal<String>,
  shadow: Signal<bool>,
}

impl Values {
  fn new(ctx: &mut Ctx) -> Self {
    Self {
      x: ctx.signal(120.0),
      y: ctx.signal(64.0),
      w: ctx.signal(320.0),
      h: ctx.signal(200.0),
      rotation: ctx.signal(0.0),
      fill: ctx.signal(Color::from_hex("#2563EB")),
      opacity: ctx.signal(80.0),
      radius: ctx.signal(12.0),
      blend: ctx.signal(0),
      visible: ctx.signal(true),
      name: ctx.signal("Card".to_owned()),
      shadow: ctx.signal(false),
    }
  }

  /// The board's categories, the radius changed from its default of 8.
  fn categories(&self) -> Vec<PropertyCategory> {
    vec![
      PropertyCategory::new("Transform")
        .property(Property::xy("Position", self.x.clone(), self.y.clone()))
        .property(Property::pair("Size", ("W", self.w.clone()), ("H", self.h.clone())))
        .property(
          Property::number_with("Rotation", self.rotation.clone(), "°")
            .stepper(true)
            .default(0.0),
        ),
      PropertyCategory::new("Appearance")
        .property(Property::color("Fill", self.fill.clone()))
        .property(Property::slider("Opacity", self.opacity.clone(), 0.0, 100.0, "%"))
        .property(Property::number("Radius", self.radius.clone()).default(8.0))
        .property(Property::choice(
          "Blend",
          self.blend.clone(),
          [
            SelectOption::new(0u8, "Normal"),
            SelectOption::new(1u8, "Multiply"),
            SelectOption::new(2u8, "Screen"),
          ],
        ))
        .property(Property::toggle("Visible", self.visible.clone())),
      PropertyCategory::new("Effects")
        .default_open(false)
        .property(Property::toggle("Shadow", self.shadow.clone()).default(false))
        .property(Property::text("Name", self.name.clone())),
    ]
  }
}

/// The buttons at the end of the board's header: more, and close.
fn header_buttons() -> Element {
  let button = |icon: &'static str, label: &str| {
    Button::new(label)
      .icon(icon)
      .icon_only(true)
      .variant(ButtonVariant::Ghost)
      .size(ButtonSize::Xs)
  };
  Row::new()
    .spacing(2.0)
    .child(button(icons::MORE_HORIZONTAL, "More"))
    .child(button(icons::CLOSE, "Close"))
    .into()
}

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).child(shown))
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

pub struct PropertyGridPage {
  frame: Frame,
  playground: Playground,
  values: Values,
  title: Signal<String>,
  width: Signal<usize>,
  header: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  /// The values of the long inspector: 500 numbers.
  many: Vec<Signal<f64>>,
}

impl PropertyGridPage {
  /// The playground: the board's inspector as the panel's props make it, mounted, its changes told in the Events tab.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let title = self.title.get();
    let (width_name, width) = WIDTHS[self.width.get().min(WIDTHS.len() - 1)];
    let flags = [
      ("header", self.header.get()),
      ("read_only", self.read_only.get()),
      ("disabled", self.disabled.get()),
      ("full_width", self.full_width.get()),
    ];
    let values = self.values.clone();
    let make = {
      let title = title.clone();
      let set: Vec<bool> = flags.iter().map(|(_, set)| *set).collect();
      move || {
        let mut grid = PropertyGrid::new()
          .categories(values.categories())
          .name_width(width)
          .read_only(set[1])
          .disabled(set[2])
          .full_width(set[3]);
        if set[0] {
          grid = grid.title(&title).trailing(header_buttons);
        }
        grid
      }
    };
    let told = self.playground.recorder();
    let grid = self.playground.styled(
      make()
        .force(self.playground.forced())
        .on_change(move |id| told(format!("on_change({id:?})"))),
    );

    let mut code = vec!["PropertyGrid::new()".to_owned()];
    if flags[0].1 {
      code.push(format!("    .title({title:?})"));
      code.push("    .trailing(header_buttons)".to_owned());
    }
    code.extend([
      "    .category(".to_owned(),
      "        PropertyCategory::new(\"Transform\")".to_owned(),
      "            .property(Property::xy(\"Position\", x, y))".to_owned(),
      "            .property(Property::pair(\"Size\", (\"W\", w), (\"H\", h)))".to_owned(),
      "            .property(Property::number_with(\"Rotation\", rotation, \"°\").stepper(true).default(0.0)),"
        .to_owned(),
      "    )".to_owned(),
      "    .category(PropertyCategory::new(\"Appearance\") /* Fill, Opacity, Radius, Blend, Visible */)".to_owned(),
      "    .category(PropertyCategory::new(\"Effects\").default_open(false) /* .. */)".to_owned(),
    ]);
    if width != 84.0 {
      code.push(format!("    .name_width({width:.0}.0)"));
    }
    for (name, set) in &flags[1..] {
      if *set {
        code.push(format!("    .{name}(true)"));
      }
    }
    code.push("    .on_change(|id| ..)".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let tree = vec![
      format!("root          {}", self.playground.measured()),
      format!(
        "├─ header    {}",
        if flags[0].1 {
          format!("{:?}", title.to_uppercase())
        } else {
          "none".to_owned()
        }
      ),
      "├─ Transform  Position (X, Y) · Size (W, H) · Rotation".to_owned(),
      "├─ Appearance Fill · Opacity · Radius (changed) · Blend · Visible".to_owned(),
      "└─ Effects    closed".to_owned(),
      format!("names {width_name} wide; editors 24 high"),
    ];
    let widths: Vec<(&str, usize)> = WIDTHS.iter().enumerate().map(|(index, item)| (item.0, index)).collect();
    let to_width = self.width.clone();
    let reset = {
      let (title, width, header, read_only, disabled, full_width) = (
        self.title.clone(),
        self.width.clone(),
        self.header.clone(),
        self.read_only.clone(),
        self.disabled.clone(),
        self.full_width.clone(),
      );
      move || {
        title.set("Properties".to_owned());
        width.set(0);
        header.set(true);
        read_only.set(false);
        disabled.set(false);
        full_width.set(false);
      }
    };
    let changed = usize::from(title != "Properties")
      + usize::from(width != 84.0)
      + usize::from(!flags[0].1)
      + flags[1..].iter().filter(|(_, set)| *set).count();
    let on = flags[3].1;
    self.playground.card(
      ctx,
      Spec {
        title: "Property Grid",
        make: Arc::new(move || make().into()),
        shown: Shown::built(move |ctx| grid.mount(ctx)),
        fill: on,
        forceable: true,
        facts: format!("names {width_name}"),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop("title", "&str", field(&self.title)),
          prop(
            "name_width",
            "f32",
            self
              .playground
              .select(0, &widths, &self.width.get(), move |index| to_width.set(index)),
          ),
          divider(),
          flag("header", &self.header),
          flag("read_only", &self.read_only),
          flag("disabled", &self.disabled),
          flag("full_width", &self.full_width),
        ],
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for PropertyGridPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      values: Values::new(ctx),
      title: ctx.signal("Properties".to_owned()),
      width: ctx.signal(0),
      header: ctx.signal(true),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      many: (0..500).map(|index| ctx.signal(index as f64)).collect(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let values = &self.values;
    let board = items().child(item(
      PropertyGrid::new()
        .title("Properties")
        .trailing(header_buttons)
        .categories(values.categories())
        .mount(ctx),
      "the board's inspector: its values are the page's, as the playground's",
    ));
    let kinds = items().child(item(
      PropertyGrid::new()
        .category(
          PropertyCategory::new("Every kind")
            .property(Property::text("Text", values.name.clone()))
            .property(Property::number_with("Number", values.rotation.clone(), "°").stepper(true))
            .property(Property::slider("Slider", values.opacity.clone(), 0.0, 100.0, "%"))
            .property(Property::toggle("Toggle", values.visible.clone()))
            .property(Property::choice(
              "Choice",
              values.blend.clone(),
              [SelectOption::new(0u8, "Normal"), SelectOption::new(1u8, "Multiply")],
            ))
            .property(Property::color("Color", values.fill.clone()))
            .property(Property::xy("XY", values.x.clone(), values.y.clone()))
            .property(Property::pair("Pair", ("W", values.w.clone()), ("H", values.h.clone())))
            .property(Property::custom("Custom", || {
              Button::new("Edit…")
                .variant(ButtonVariant::Outline)
                .size(ButtonSize::Xs)
                .into()
            })),
        )
        .mount(ctx),
      "text · number · slider · toggle · choice · color · xy · pair · custom",
    ));
    let states = items()
      .child(item(
        PropertyGrid::new()
          .category(
            PropertyCategory::new("Limits")
              .property(Property::number("Read only", values.radius.clone()).read_only(true))
              .property(Property::number("Disabled", values.radius.clone()).disabled(true))
              .property(Property::number("Changed", values.radius.clone()).default(8.0))
              .property(Property::number(
                "The spacing between the columns of the layout",
                values.w.clone(),
              )),
          )
          .mount(ctx),
        "read only · disabled · changed, with its reset · a long name, cut",
      ))
      .child(item(
        PropertyGrid::new()
          .title("Properties")
          .disabled(true)
          .categories(values.categories()),
        "the grid disabled",
      ));

    let mut long = PropertyGrid::new().title("Inspector").height(360.0);
    for category in 0..20 {
      let mut group = PropertyCategory::new(&format!("Component {}", category + 1)).default_open(category < 3);
      for index in 0..25 {
        let at = category * 25 + index;
        group = group.property(Property::number(&format!("Field {}", at + 1), self.many[at].clone()));
      }
      long = long.category(group);
    }
    let long = items().child(item(
      long.mount(ctx),
      "500 properties in 20 categories: only the rows near the view are built",
    ));

    let rows = [
      (
        "title",
        "&str",
        "",
        "The header over the categories, its title in capitals.",
      ),
      (
        "trailing",
        "impl Fn() -> Element",
        "",
        "What stands at the end of the header: its buttons.",
      ),
      (
        "category / categories",
        "PropertyCategory",
        "",
        "Categories, each a title over its properties.",
      ),
      (
        "name_width",
        "f32",
        "84",
        "The width of the names: where the divider starts.",
      ),
      (
        "read_only / disabled",
        "bool",
        "false",
        "Every value read and not changed; or nothing takes input, dimmed.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 300: an inspector's column.",
      ),
      (
        "height",
        "impl Into<Length>",
        "",
        "Its rows scroll in it; mounted, only the rows near the view are built.",
      ),
      ("label", "&str", "its title", "What it is called to tooling."),
      (
        "on_change",
        "impl Fn(&str)",
        "",
        "Told the id of a property a person changed, after it is set.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Keeps which categories are open, the divider, and Escape back to the row.",
      ),
      (
        "Property::text / number / number_with",
        "&str, Signal<..>",
        "",
        "A text in an input; a number, with its unit.",
      ),
      (
        "Property::slider",
        "&str, Signal<f32>, min, max, unit",
        "",
        "A slider with the number typed beside it.",
      ),
      (
        "Property::toggle / choice",
        "&str, Signal<..>",
        "",
        "A checkbox; a select.",
      ),
      (
        "Property::color",
        "&str, Signal<Color>",
        "",
        "Its swatch, which opens the picker, and its hex in a box; eyedropper(true) adds the eyedropper.",
      ),
      (
        "stepper",
        "bool",
        "false",
        "A number's chevrons at the end of its box, whose halves step it.",
      ),
      (
        "Property::xy / pair",
        "&str, Signal<f64> × 2",
        "",
        "Two numbers with their letters: X and Y, W and H.",
      ),
      (
        "Property::custom",
        "&str, impl Fn() -> Element",
        "",
        "Any element as the editor.",
      ),
      (
        "default",
        "V",
        "",
        "What the reset puts back; the name is marked while the value is another.",
      ),
      (
        "description / read_only / disabled / id / key",
        "",
        "",
        "Of the property.",
      ),
      (
        "PropertyCategory::default_open",
        "bool",
        "true",
        "Whether it is open as it is first shown.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Desktop", "Property Grid"],
        title: "Property Grid",
        description: "An inspector: the names of what is selected and the editors of their values, in categories \
                      that open and close. Every value is edited at once, and a changed one can be put back.",
        import: Some("use lurq::ui::{Property, PropertyCategory, PropertyGrid};"),
        status: Some("New"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of PropertyGrid is a control on the right. The grid on the stage is mounted: its values are the \
           page's, a change is told in the Events tab, and Up, Down, Left, Right, Tab, Enter and Escape go as in a \
           tree with editors in it.",
          playground,
        ),
        Section::new(
          "Property Grid",
          "The board's inspector: Transform and Appearance open, Effects closed, the radius changed from its default \
           with its mark and its reset. Drag the line between the names and the editors to share the width.",
          self.frame.demo(
            0,
            board,
            "PropertyGrid::new().title(\"Properties\").categories(..)",
            &[
              "PropertyGrid::new()",
              "    .title(\"Properties\")",
              "    .trailing(header_buttons)",
              "    .category(PropertyCategory::new(\"Transform\").property(Property::xy(\"Position\", x, y)) /* .. */)",
              "    .category(PropertyCategory::new(\"Appearance\") /* .. */)",
              "    .category(PropertyCategory::new(\"Effects\").default_open(false) /* .. */)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Kinds of property",
          "Each kind is one of ui's controls, its box laid over by the grid's editor part: 24 high, its text in mono.",
          self.frame.demo(
            1,
            kinds,
            "Property::text(\"Text\", name)",
            &[
              "Property::text(\"Text\", name)",
              "Property::number_with(\"Number\", rotation, \"°\").stepper(true)",
              "Property::slider(\"Slider\", opacity, 0.0, 100.0, \"%\")",
              "Property::toggle(\"Toggle\", visible)",
              "Property::choice(\"Choice\", blend, [SelectOption::new(0u8, \"Normal\"), ..])",
              "Property::color(\"Color\", fill)",
              "Property::xy(\"XY\", x, y)",
              "Property::pair(\"Pair\", (\"W\", w), (\"H\", h))",
              "Property::custom(\"Custom\", || Button::new(\"Edit…\").into())",
            ],
          ),
        ),
        Section::new(
          "States",
          "A read-only value is read and not changed; a disabled one is dimmed; a value not its default has its name \
           marked and a reset at the end of its row. A name longer than its column ends in an ellipsis.",
          self.frame.demo(
            2,
            states,
            ".default(8.0)",
            &[
              "Property::number(\"Read only\", radius).read_only(true)",
              "Property::number(\"Disabled\", radius).disabled(true)",
              "Property::number(\"Changed\", radius).default(8.0)",
              "PropertyGrid::new().disabled(true)",
            ],
          ),
        ),
        Section::new(
          "A long inspector",
          "A game engine's inspector holds hundreds of properties. Given a height, the grid scrolls, and mounted it \
           builds only the rows near its view: Up, Down, Home, End, PageUp and PageDown reach every row, which is \
           built as the focus comes to it.",
          self.frame.demo(
            3,
            long,
            "PropertyGrid::new().height(360.0)",
            &[
              "let mut grid = PropertyGrid::new().title(\"Inspector\").height(360.0);",
              "for (name, fields) in components {",
              "    grid = grid.category(PropertyCategory::new(name).properties(fields));",
              "}",
              "grid.mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a property grid is given.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
