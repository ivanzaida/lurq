//! The page of the color picker: its playground, the board's picker with its swatches, a color with alpha and the
//! fields of RGB and HSL, a picker held by the app, and its states.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::{Element, color::Color},
  ui::{ColorFormat, ColorPicker, Interaction, current_theme},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The board's swatches: the theme's five colors of charts and its foreground.
fn board_swatches() -> Vec<Color> {
  let theme = current_theme();
  [
    PaletteColor::Chart1,
    PaletteColor::Chart2,
    PaletteColor::Chart3,
    PaletteColor::Chart4,
    PaletteColor::Chart5,
    PaletteColor::Foreground,
  ]
  .into_iter()
  .map(|role| theme.palette_color(role))
  .collect()
}

/// The formats the playground offers, and how the code names them.
const FORMATS: [(ColorFormat, &str, &str); 3] = [
  (ColorFormat::Hex, "hex", "ColorFormat::Hex"),
  (ColorFormat::Rgb, "rgb", "ColorFormat::Rgb"),
  (ColorFormat::Hsl, "hsl", "ColorFormat::Hsl"),
];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
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

pub struct ColorPickerPage {
  frame: Frame,
  playground: Playground,
  value: Signal<Color>,
  alpha: Signal<bool>,
  format: Signal<usize>,
  switch: Signal<bool>,
  swatches: Signal<bool>,
  disabled: Signal<bool>,
  board: Signal<Color>,
  veil: Signal<Color>,
  held: Signal<Color>,
}

impl ColorPickerPage {
  /// The playground: the picker as the panel's props make it, bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (alpha, switch, swatches, disabled) = (
      self.alpha.get(),
      self.switch.get(),
      self.swatches.get(),
      self.disabled.get(),
    );
    let (format, format_name, format_code) = FORMATS[self.format.get().min(FORMATS.len() - 1)];
    let make = {
      let value = self.value.clone();
      move || {
        let mut picker = ColorPicker::new(value.clone())
          .alpha(alpha)
          .format(format)
          .disabled(disabled)
          .label("Accent color");
        if switch {
          picker = picker.formats(FORMATS.map(|format| format.0));
        }
        if swatches {
          picker = picker.swatches(board_swatches());
        }
        picker
      }
    };
    let (changed_to, ended_at) = (self.playground.recorder(), self.playground.recorder());
    let shown = make()
      .force(self.playground.forced())
      .on_change(move |color| changed_to(format!("on_change({})", color.to_hex_upper())))
      .on_change_end(move |color| ended_at(format!("on_change_end({})", color.to_hex_upper())));

    let mut code = vec![
      "// value: Signal<Color>".to_owned(),
      "ColorPicker::new(value.clone())".to_owned(),
    ];
    if alpha {
      code.push("    .alpha(true)".to_owned());
    }
    if format != ColorFormat::Hex {
      code.push(format!("    .format({format_code})"));
    }
    if switch {
      code.push("    .formats([ColorFormat::Hex, ColorFormat::Rgb, ColorFormat::Hsl])".to_owned());
    }
    if swatches {
      code.push("    .swatches(chart_colors)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .label(\"Accent color\")".to_owned());
    code.push("    .on_change_end(|color| save(color))".to_owned());

    let hsv = self.value.get().to_hsv();
    let mut tree = vec![
      format!("root        {}   12 in, 12 apart", self.playground.measured()),
      format!(
        "├─ area     208 × 120   saturation {:.0}%, brightness {:.0}%",
        hsv.s * 100.0,
        hsv.v * 100.0
      ),
      format!("├─ hue      208 × 10    {:.0}°", hsv.h),
    ];
    if alpha {
      tree.push(format!("├─ alpha    208 × 10    {:.0}%", hsv.a * 100.0));
    }
    if swatches {
      tree.push("├─ swatches 6 × 20, 8 apart".to_owned());
    }
    tree.push(format!(
      "└─ fields   {format_name}   {}",
      self.value.get().to_hex_upper()
    ));

    let formats: Vec<(&str, usize)> = FORMATS
      .iter()
      .enumerate()
      .map(|(index, format)| (format.1, index))
      .collect();
    let to_format = self.format.clone();
    let reset = {
      let (value, format) = (self.value.clone(), self.format.clone());
      let flags = [self.alpha.clone(), self.switch.clone(), self.disabled.clone()];
      let swatches = self.swatches.clone();
      move || {
        value.set(Color::from_hex("#2563EB"));
        format.set(0);
        swatches.set(true);
        for flag in &flags {
          flag.set(false);
        }
      }
    };
    let changed = usize::from(alpha)
      + usize::from(format != ColorFormat::Hex)
      + usize::from(switch)
      + usize::from(!swatches)
      + usize::from(disabled)
      + usize::from(self.value.get() != Color::from_hex("#2563EB"));
    let controls = vec![
      flag("alpha", &self.alpha),
      prop(
        "format",
        "ColorFormat",
        segmented_fill(&formats, &self.format.get(), move |index| to_format.set(index)),
      ),
      flag("formats (a switch)", &self.switch),
      flag("swatches", &self.swatches),
      divider(),
      flag("disabled", &self.disabled),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Color Picker",
        make: Arc::new(move || make().into()),
        shown: Shown::of(self.playground.styled(shown)),
        fill: false,
        forceable: true,
        facts: "232 wide · radius 8".to_owned(),
        event: "on_change / on_change_end",
        code,
        tree,
        controls,
        changed,
        reset: Arc::new(reset),
      },
    )
  }
}

impl Component for ColorPickerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      value: ctx.signal(Color::from_hex("#2563EB")),
      alpha: ctx.signal(false),
      format: ctx.signal(0),
      switch: ctx.signal(false),
      swatches: ctx.signal(true),
      disabled: ctx.signal(false),
      board: ctx.signal(Color::from_hex("#2563EB")),
      veil: ctx.signal(Color::from_hex("#2563EB80")),
      held: ctx.signal(Color::from_hex("#22C55E")),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = items().child(item(
      ColorPicker::new(self.board.clone())
        .swatches(board_swatches())
        .label("Accent color"),
      &format!("the signal has: {}", self.board.get().to_hex_upper()),
    ));

    let veil = items().child(item(
      ColorPicker::new(self.veil.clone())
        .alpha(true)
        .format(ColorFormat::Rgb)
        .formats([ColorFormat::Hex, ColorFormat::Rgb, ColorFormat::Hsl])
        .label("Overlay color"),
      &format!("the signal has: {}", self.veil.get().to_hex_upper()),
    ));

    let held = self.held.clone();
    let controlled = items().child(item(
      ColorPicker::controlled(self.held.get(), move |color| held.set(color))
        .swatches(board_swatches())
        .label("Series color"),
      &format!("the app holds: {}", self.held.get().to_hex_upper()),
    ));

    let still = || ColorPicker::new(Signal::new(Color::from_hex("#2563EB")));
    let states = items()
      .child(item(still(), "at rest"))
      .child(item(
        still().force(Some(Interaction::FocusVisible)),
        "focused from the keyboard",
      ))
      .child(item(still().disabled(true), "disabled"));

    let rows = [
      (
        "value",
        "Signal<Color>",
        "",
        "The color: what the picker shows, and what a person's change sets. `controlled(color, on_change)` is the \
         same held by the app.",
      ),
      (
        "alpha",
        "bool",
        "false",
        "A rail of alpha over a checkerboard, and the fields say the alpha.",
      ),
      (
        "format",
        "ColorFormat",
        "Hex",
        "The fields: one of hex (the board's), or one for each channel of RGB or HSL.",
      ),
      (
        "formats",
        "impl IntoIterator<Item = ColorFormat>",
        "",
        "The fields a button before them switches between.",
      ),
      (
        "eyedropper",
        "bool",
        "true",
        "The button at the end of the fields that picks a color from the screen: shown where the platform can pick.",
      ),
      (
        "swatch / swatches",
        "impl Into<Color>",
        "",
        "Circles of 20 under the rails; a press of one makes the color it.",
      ),
      (
        "on_change",
        "impl Fn(Color)",
        "",
        "Called with each change a person makes, as a thumb moves; not when the app sets the color.",
      ),
      (
        "on_change_end",
        "impl Fn(Color)",
        "",
        "Called once a change is over (a drag let go, a key's step, a swatch, a field committed), where the color \
         is another than it was.",
      ),
      (
        "bare",
        "bool",
        "false",
        "Without its panel: what a popover that holds it draws.",
      ),
      (
        "label",
        "&str",
        "",
        "What the picker is called, to what reads the tree.",
      ),
      (
        "saturation_label / brightness_label / hue_label / alpha_label / format_label",
        "&str",
        "",
        "The words the sliders and the switch say to tooling.",
      ),
      ("disabled", "bool", "false", "Dimmed; nothing in it takes input."),
      (
        "style / rules",
        "ColorPickerPart",
        "",
        "Root, Area, AreaThumb, Hue, Alpha, Thumb, Checker, CheckerCell, Swatches, Swatch, Fields, Input, Format, \
         FieldLabel.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Color Picker"],
        title: "Color Picker",
        description: "A color chosen in an area of saturation and brightness, on a rail of hue and one of alpha, from \
                      swatches, or typed as hex, RGB or HSL.",
        import: Some("use lurq::ui::{ColorFormat, ColorPicker};"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of ColorPicker is a control on the right. The picker on the stage is live: drag in the area \
           and on the rails, press a swatch, use the arrows with the focus in the area.",
          playground,
        ),
        Section::new(
          "Color picker",
          "The board's picker: the area of saturation across and brightness up, the rail of hue, the theme's colors \
           of charts as swatches and the hex. A press or a drag sets the color; with the focus in the area the arrows \
           step it by 1%, ten with Shift.",
          self.frame.demo(
            0,
            board,
            "ColorPicker::new(color).swatches(chart_colors)",
            &[
              "// color: Signal<Color>",
              "ColorPicker::new(color.clone())",
              "    .swatches(chart_colors)",
              "    .label(\"Accent color\")",
            ],
          ),
        ),
        Section::new(
          "Alpha and channels",
          "A rail of alpha over a checkerboard, and the fields of RGB with the alpha; the button before them \
           switches to hex and to HSL.",
          self.frame.demo(
            1,
            veil,
            ".alpha(true).format(ColorFormat::Rgb)",
            &[
              "ColorPicker::new(overlay.clone())",
              "    .alpha(true)",
              "    .format(ColorFormat::Rgb)",
              "    .formats([ColorFormat::Hex, ColorFormat::Rgb, ColorFormat::Hsl])",
              "    .label(\"Overlay color\")",
            ],
          ),
        ),
        Section::new(
          "Held by the app",
          "The app holds the color and is told each change; the picker shows what it answers with.",
          self.frame.demo(
            2,
            controlled,
            "ColorPicker::controlled(color, on_change)",
            &[
              "ColorPicker::controlled(series, move |color| set_series(color))",
              "    .swatches(chart_colors)",
              "    .label(\"Series color\")",
            ],
          ),
        ),
        Section::new(
          "States",
          "The ring of the focus is around the area or the rail that has it. A disabled picker is dimmed and takes \
           nothing.",
          self.frame.demo(
            3,
            states,
            ".disabled(true)",
            &[
              "ColorPicker::new(color.clone())",
              "",
              "// Drawn as it is focused, with nothing focusing it: for a gallery, a test.",
              "ColorPicker::new(color.clone()).force(Some(Interaction::FocusVisible))",
              "",
              "ColorPicker::new(color.clone()).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a picker is given.",
          props_table(&rows, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
