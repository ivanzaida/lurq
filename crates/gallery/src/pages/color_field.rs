//! The page of the color field: its playground, the board's field, a color or none, the field in a form field, its
//! states and sizes, and the color swatch it shows.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::{Element, color::Color},
  ui::{ColorField, ColorFieldSize, ColorSwatch, ColorSwatchShape, ColorSwatchSize, Field, Interaction},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The sizes, as the board names them, and how the code names them.
const SIZES: [(ColorFieldSize, &str, &str); 3] = [
  (ColorFieldSize::Sm, "sm", "ColorFieldSize::Sm"),
  (ColorFieldSize::Md, "md", "ColorFieldSize::Md"),
  (ColorFieldSize::Lg, "lg", "ColorFieldSize::Lg"),
];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().overflow_visible().child(shown))
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

/// A field that shows the board's blue and is bound to a signal of its own: for a stage that shows a state.
fn showing() -> ColorField {
  ColorField::new(Signal::new(Color::from_hex("#2563EB")))
}

pub struct ColorFieldPage {
  frame: Frame,
  playground: Playground,
  value: Signal<Color>,
  alpha: Signal<bool>,
  size: Signal<usize>,
  invalid: Signal<bool>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  full_width: Signal<bool>,
  board: Signal<Color>,
  tint: Signal<Option<Color>>,
  brand: Signal<Color>,
  pressed: Signal<usize>,
}

impl ColorFieldPage {
  /// The playground: the field as the panel's props make it, bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (alpha, invalid, read_only, disabled, full_width) = (
      self.alpha.get(),
      self.invalid.get(),
      self.read_only.get(),
      self.disabled.get(),
      self.full_width.get(),
    );
    let (size, size_name, size_code) = SIZES[self.size.get().min(SIZES.len() - 1)];
    let make = {
      let value = self.value.clone();
      move || {
        ColorField::new(value.clone())
          .alpha(alpha)
          .size(size)
          .invalid(invalid)
          .read_only(read_only)
          .disabled(disabled)
          .full_width(full_width)
          .label("Brand color")
      }
    };
    let changed_to = self.playground.recorder();
    let shown = make()
      .force(self.playground.forced())
      .on_change(move |color: Color| changed_to(format!("on_change({})", color.to_hex_upper())));

    let mut code = vec![
      "// value: Signal<Color>".to_owned(),
      "ColorField::new(value.clone())".to_owned(),
    ];
    if alpha {
      code.push("    .alpha(true)".to_owned());
    }
    if size != ColorFieldSize::Md {
      code.push(format!("    .size({size_code})"));
    }
    for (name, set) in [
      ("invalid", invalid),
      ("read_only", read_only),
      ("disabled", disabled),
      ("full_width", full_width),
    ] {
      if set {
        code.push(format!("    .{name}(true)"));
      }
    }
    code.push("    .label(\"Brand color\")".to_owned());

    let tree = vec![
      format!("root        {}", self.playground.measured()),
      "├─ control  an input's box, the hex in the code's type at 13".to_owned(),
      format!("├─ swatch   {}", self.value.get().to_hex_upper()),
      "├─ trigger  24, an eyedropper of 16".to_owned(),
      "└─ panel    a popover of the picker, 232 wide".to_owned(),
    ];
    let sizes: Vec<(&str, usize)> = SIZES.iter().enumerate().map(|(index, size)| (size.1, index)).collect();
    let to_size = self.size.clone();
    let reset = {
      let (value, size) = (self.value.clone(), self.size.clone());
      let flags = [
        self.alpha.clone(),
        self.invalid.clone(),
        self.read_only.clone(),
        self.disabled.clone(),
        self.full_width.clone(),
      ];
      move || {
        value.set(Color::from_hex("#2563EB"));
        size.set(1);
        for flag in &flags {
          flag.set(false);
        }
      }
    };
    let changed = usize::from(alpha)
      + usize::from(size != ColorFieldSize::Md)
      + usize::from(invalid)
      + usize::from(read_only)
      + usize::from(disabled)
      + usize::from(full_width)
      + usize::from(self.value.get() != Color::from_hex("#2563EB"));
    let controls = vec![
      flag("alpha", &self.alpha),
      prop(
        "size",
        "ColorFieldSize",
        segmented_fill(&sizes, &self.size.get(), move |index| to_size.set(index)),
      ),
      divider(),
      flag("invalid", &self.invalid),
      flag("read_only", &self.read_only),
      flag("disabled", &self.disabled),
      flag("full_width", &self.full_width),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Color Field",
        make: Arc::new(move || make().into()),
        shown: Shown::of(shown),
        fill: full_width,
        forceable: true,
        facts: format!("{size_name} · radius 6"),
        event: "on_change",
        code,
        tree,
        controls,
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for ColorFieldPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      value: ctx.signal(Color::from_hex("#2563EB")),
      alpha: ctx.signal(false),
      size: ctx.signal(1),
      invalid: ctx.signal(false),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      full_width: ctx.signal(false),
      board: ctx.signal(Color::from_hex("#2563EB")),
      tint: ctx.signal(None),
      brand: ctx.signal(Color::from_hex("#2563EB")),
      pressed: ctx.signal(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = items().child(item(
      ColorField::new(self.board.clone()).label("Brand color"),
      &format!("the signal has: {}", self.board.get().to_hex_upper()),
    ));

    let optional = items().child(item(
      ColorField::optional(self.tint.clone())
        .placeholder("No tint")
        .alpha(true)
        .label("Tint"),
      &match self.tint.get() {
        Some(color) => format!("the signal has: Some({})", color.to_hex_upper()),
        None => "the signal has: None".to_owned(),
      },
    ));

    let fields = items()
      .child(
        Field::new("Brand color", ColorField::new(self.brand.clone()))
          .description("Buttons and links are drawn in it."),
      )
      .child(
        Field::new("Brand color", ColorField::new(Signal::new(Color::from_hex("#FAFAFA"))))
          .error("Too light to read on a white page."),
      );

    let states = items()
      .child(item(showing(), "at rest"))
      .child(item(showing().force(Some(Interaction::FocusVisible)), "focus"))
      .child(item(showing().invalid(true), "invalid"))
      .child(item(showing().read_only(true), "read only"))
      .child(item(showing().disabled(true), "disabled"));

    let mut sizes = items();
    for (size, name, _) in SIZES {
      sizes = sizes.child(item(showing().size(size), name));
    }

    let pressed = self.pressed.clone();
    let swatches = items()
      .child(item(ColorSwatch::new(Color::from_hex("#2563EB")), "the board's: 24"))
      .child(item(
        ColorSwatch::new(Color::from_hex("#2563EB")).size(ColorSwatchSize::Sm),
        "sm: 20",
      ))
      .child(item(
        ColorSwatch::new(Color::from_hex("#2563EB")).size(ColorSwatchSize::Lg),
        "lg: 32",
      ))
      .child(item(
        ColorSwatch::new(Color::from_hex("#22C55E")).shape(ColorSwatchShape::Round),
        "round",
      ))
      .child(item(ColorSwatch::new(Color::from_hex("#2563EB80")), "alpha 50%"))
      .child(item(
        ColorSwatch::new(Color::from_hex("#3B82F6")).selected(true),
        "selected",
      ))
      .child(item(
        ColorSwatch::new(Color::from_hex("#F59E0B"))
          .label("Amber")
          .on_click(move |_| pressed.update(|count| *count += 1)),
        &format!("pressed {} times", self.pressed.get()),
      ))
      .child(item(
        ColorSwatch::new(Color::from_hex("#A855F7"))
          .on_click(|_| {})
          .force(Some(Interaction::FocusVisible)),
        "focus",
      ))
      .child(item(
        ColorSwatch::new(Color::from_hex("#A855F7"))
          .on_click(|_| {})
          .disabled(true),
        "disabled",
      ));

    let rows = [
      (
        "value",
        "Signal<Color>",
        "",
        "The color: what the box and its swatch show, and what is typed or chosen sets. `optional(Signal<Option<Color>>)` \
         holds none too; `controlled(value, on_change)` is held by the app.",
      ),
      ("alpha", "bool", "false", "The picker has a rail of alpha."),
      (
        "format / formats / swatch / swatches",
        "ColorFormat / Color",
        "",
        "What the picker shows: see Color Picker.",
      ),
      (
        "on_change",
        "impl Fn(Color) / Fn(Option<Color>)",
        "",
        "Called with each change a person makes, typed or chosen; not when the app sets the value.",
      ),
      ("placeholder", "&str", "", "What the box says while it holds no color."),
      (
        "size",
        "ColorFieldSize",
        "Md",
        "An input's: 32, 36, 40 high, with a swatch of 20, 24, 32.",
      ),
      (
        "invalid / warning / valid",
        "bool",
        "false",
        "The color of the box's edge and ring.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "Read and copied, not changed: nothing typed is taken, and the picker does not open.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity on the muted fill; it takes no input.",
      ),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 180.",
      ),
      (
        "placement",
        "Placement",
        "BottomStart",
        "Where the picker opens beside the field.",
      ),
      (
        "label / trigger_label / panel_label",
        "&str",
        "",
        "What the box, the eyedropper and the popover are called, to what reads the tree.",
      ),
      (
        "name",
        "impl Into<Arc<str>>",
        "",
        "The name of its value, the hex, in the form it is in.",
      ),
    ];
    let swatch_rows = [
      (
        "color",
        "impl Into<Color>",
        "",
        "The color, over a checkerboard where it is not opaque.",
      ),
      ("size", "ColorSwatchSize", "Md", "20, 24, 32 square."),
      (
        "shape",
        "ColorSwatchShape",
        "Square",
        "A square with the small radius, or a circle.",
      ),
      ("selected", "bool", "false", "A ring of 2 in the foreground around it."),
      (
        "on_click",
        "impl IntoMouseEventHandler",
        "",
        "Pressed by a click, Enter or Space: it is then a button, a stop of Tab.",
      ),
      (
        "label",
        "&str",
        "",
        "What it is called, to what reads the tree; its hex where it has none.",
      ),
      ("disabled", "bool", "false", "Dimmed, and not pressed."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Color Field"],
        title: "Color Field",
        description: "A color typed in a box, or chosen in a color picker that opens under it; and the swatch that \
                      shows one color.",
        import: Some("use lurq::ui::{ColorField, ColorSwatch};"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of ColorField is a control on the right. The field on the stage is live: type a color and \
           press Enter, or open the picker with the eyedropper, the swatch or Down.",
          playground,
        ),
        Section::new(
          "Color field",
          "The board's field: the swatch of its color, its hex and an eyedropper. What is typed is read on Enter and \
           as the focus leaves, as hex, rgb() or hsl(); what is no color takes back the hex. The eyedropper opens the \
           picker.",
          self.frame.demo(
            0,
            board,
            "ColorField::new(color)",
            &[
              "// color: Signal<Color>",
              "ColorField::new(color.clone()).label(\"Brand color\")",
            ],
          ),
        ),
        Section::new(
          "A color or none",
          "Bound to a Signal<Option<Color>>: emptied, the field holds none and says its placeholder. With alpha the \
           picker has a rail of it, and the hex says it.",
          self.frame.demo(
            1,
            optional,
            "ColorField::optional(tint)",
            &[
              "// tint: Signal<Option<Color>>",
              "ColorField::optional(tint.clone())",
              "    .placeholder(\"No tint\")",
              "    .alpha(true)",
              "    .label(\"Tint\")",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names the box with its label and judges it with its error.",
          self.frame.demo(
            2,
            fields,
            "Field::new(\"Brand color\", field)",
            &[
              "Field::new(\"Brand color\", ColorField::new(brand.clone()))",
              "    .description(\"Buttons and links are drawn in it.\")",
              "",
              "Field::new(\"Brand color\", ColorField::new(brand.clone()))",
              "    .error(\"Too light to read on a white page.\")",
            ],
          ),
        ),
        Section::new(
          "States",
          "The board draws the field at rest; under the pointer, focused, judged, read only and disabled it is an \
           input.",
          self.frame.demo(
            3,
            states,
            ".invalid(true)",
            &[
              "ColorField::new(color.clone())",
              "ColorField::new(color.clone()).force(Some(Interaction::FocusVisible))",
              "ColorField::new(color.clone()).invalid(true)",
              "ColorField::new(color.clone()).read_only(true)",
              "ColorField::new(color.clone()).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Sizes",
          "An input's heights, with a swatch of 20, 24 and 32.",
          self.frame.demo(
            4,
            sizes,
            ".size(ColorFieldSize::Sm)",
            &[
              "ColorField::new(color.clone()).size(ColorFieldSize::Sm)",
              "ColorField::new(color.clone())",
              "ColorField::new(color.clone()).size(ColorFieldSize::Lg)",
            ],
          ),
        ),
        Section::new(
          "Color swatch",
          "One color, as a square of 24 with the small radius and an edge in the border's color, or a circle; over a \
           checkerboard where it is not opaque. Selected, a ring in the foreground; with on_click it is a button.",
          self.frame.demo(
            5,
            swatches,
            "ColorSwatch::new(color)",
            &[
              "ColorSwatch::new(Color::from_hex(\"#2563EB\"))",
              "ColorSwatch::new(blue).size(ColorSwatchSize::Sm)",
              "ColorSwatch::new(blue).size(ColorSwatchSize::Lg)",
              "ColorSwatch::new(green).shape(ColorSwatchShape::Round)",
              "ColorSwatch::new(Color::from_hex(\"#2563EB80\"))",
              "ColorSwatch::new(blue).selected(true)",
              "ColorSwatch::new(amber).label(\"Amber\").on_click(|_| choose(amber))",
              "ColorSwatch::new(purple).on_click(|_| {}).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a field is given.",
          props_table(&rows, &["value"], self.frame.room()),
        ),
        Section::new(
          "Props of the swatch",
          "What a swatch is given.",
          props_table(&swatch_rows, &["color"], self.frame.room()),
        ),
      ],
    )
  }
}
