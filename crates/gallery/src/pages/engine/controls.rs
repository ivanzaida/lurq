//! The page of the engine's own controls: the slider node, the unstyled button, the select of one value or of
//! several, the checkbox node (deprecated for `ui::Checkbox`), and the form they give their values to. Its
//! playground is a slider whose props are the panel's controls; its sections are the demo's Inputs page less its
//! text fields (the Text Input page has those): a form, two checkboxes, three sliders, four selects, and what they
//! all hold.
//!
//! The design system's controls are built on these, and the gallery's Form page shows the form's own `ui`
//! controls; this page shows what is under them.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize, TypographyStyle},
  },
  components::{Button, Column, FormHandle, FormOptions, Row, Select, Slider, TextInput, validators},
  core::Signal,
  images::ImageData,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, SelectPartStyle, SelectStyle, color::Color, padding::Padding},
};

use super::kit::{controls, named};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, mono, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const KINDS: [(&str, usize); 2] = [("Slider::new", 0), ("Slider::new_range", 1)];
const RANGES: [(&str, (i32, i32)); 3] = [("0 to 100", (0, 100)), ("1 to 5", (1, 5)), ("-50 to 50", (-50, 50))];
/// The choices of the range's select: an index into [`RANGES`], which a signal keeps.
const RANGE_CHOICES: [(&str, usize); 3] = [("0 to 100", 0), ("1 to 5", 1), ("-50 to 50", 2)];
const STEPS: [(&str, i32); 3] = [("1", 1), ("5", 5), ("10", 10)];
const ORIENTATIONS: [(&str, usize); 2] = [("Horizontal", 0), ("Vertical", 1)];

/// How long the playground's slider is, and how thick.
const LENGTH: f32 = 280.0;
const THICKNESS: f32 = 24.0;

/// A color of the theme as the slider's and the checkbox's part styles take it: a color, not a role. Read where
/// the control is built, in the theme of what is being built.
fn role(color: PaletteColor) -> Color {
  lurq::ui::current_theme().palette_color(color)
}

/// What is shown for an empty text.
fn shown_text(value: &str) -> &str {
  if value.is_empty() { "<empty>" } else { value }
}

/// What is shown for a list of values.
fn listed(values: &[String]) -> String {
  if values.is_empty() {
    "<none>".to_owned()
  } else {
    values.join(", ")
  }
}

fn on_off(value: bool) -> &'static str {
  if value { "on" } else { "off" }
}

/// A value and what it is, as the demo's pills had them: in a row that wraps.
fn pill(label: &str, value: &str) -> Row {
  Row::new()
    .height(28.0)
    .max_width(FILL)
    .padding_horizontal(10.0)
    .spacing(6.0)
    .align_items(Alignment::Center)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Muted)
    .child(mono(label, 11.0).color(PaletteColor::MutedForeground).nowrap())
    .child(mono(value, 11.0).nowrap())
}

fn pills(all: Vec<(&str, String)>) -> Row {
  let mut row = Row::new().width(FILL).spacing(8.0).wrap();
  for (label, value) in all {
    row = row.child(pill(label, &value));
  }
  row
}

/// A control under its name.
fn labelled(label: &str, control: impl Into<Element>) -> Column {
  Column::new()
    .spacing(6.0)
    .min_width(0.0)
    .child(text(label, 12.0, FontWeight::SemiBold).color(PaletteColor::MutedForeground))
    .child(control)
}

/// Two controls side by side, each taking half the room; under each other where there is too little.
fn pair(first: impl Into<Element>, second: impl Into<Element>) -> Row {
  Row::new()
    .width(FILL)
    .spacing(18.0)
    .wrap()
    .align_items(Alignment::Start)
    .child(Column::new().flex(1.0).min_width(220.0).child(first))
    .child(Column::new().flex(1.0).min_width(220.0).child(second))
}

/// A text field of the engine, named for the form, in the theme's colors.
fn field(value: Signal<String>, name: &str, placeholder: &str) -> TextInput {
  TextInput::new(value)
    .name(name)
    .placeholder(placeholder)
    .single_line()
    .width(FILL)
    .padding_horizontal(12.0)
    .padding_vertical(6.0)
    .background(PaletteColor::Background)
    .border_inside(1.0, PaletteColor::Input)
    .rounded(RadiusSize::Md)
    .cursor(CursorIcon::Text)
    .focused(|style| style.border_inside(2.0, PaletteColor::Ring))
}

/// The engine's button dressed by the app: its fill, its edge, and how it looks pressed, under the pointer and
/// focused.
fn dressed(button: Button, primary: bool) -> Button {
  let (fill, ink, over) = if primary {
    (
      PaletteColor::Primary,
      PaletteColor::PrimaryForeground,
      PaletteColor::Primary,
    )
  } else {
    (PaletteColor::Background, PaletteColor::Foreground, PaletteColor::Muted)
  };
  let button = button
    .height(34.0)
    .padding_horizontal(14.0)
    .background(fill)
    .foreground(ink)
    .rounded(RadiusSize::Md)
    .cursor(CursorIcon::Pointer)
    .focused(|style| style.border_outside(2.0, PaletteColor::Ring));
  if primary {
    button
      .hovered(|style| style.opacity(0.9))
      .active(|style| style.opacity(0.8))
  } else {
    button
      .border_inside(1.0, PaletteColor::Border)
      .hovered(move |style| style.background(over))
      .active(|style| style.background(PaletteColor::Secondary))
  }
}

/// The engine's checkbox in the theme's colors, beside its name and a line under it.
#[allow(deprecated)] // The engine's checkbox node, which `ui::Checkbox` replaces: shown as what is under it.
fn checkbox_row(label: &str, detail: &str, value: Signal<bool>) -> Row {
  Row::new()
    .width(FILL)
    .min_height(42.0)
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(
      lurq::components::Checkbox::new(value)
        .size(20.0, 20.0)
        .border_inside(1.0, PaletteColor::Input)
        .rounded(4.0)
        .cursor(CursorIcon::Pointer)
        .tab_index(0)
        .focused(|style| style.border_inside(2.0, PaletteColor::Ring)),
    )
    .child(
      Column::new()
        .flex(1.0)
        .min_width(0.0)
        .spacing(2.0)
        .child(text(label, 14.0, FontWeight::SemiBold))
        .child(quiet(detail, 12.0)),
    )
}

/// A slider of the demo's: its name and its value over it, the track's fill and shape given on the node.
fn slider_row(label: &str, value: &Signal<i32>, min: i32, max: i32) -> Column {
  Column::new()
    .width(FILL)
    .spacing(8.0)
    .child(
      Row::new()
        .width(FILL)
        .justify(Justify::SpaceBetween)
        .child(text(label, 14.0, FontWeight::SemiBold))
        .child(mono(&value.get().to_string(), 12.0).color(PaletteColor::Primary)),
    )
    .child(
      Slider::new(value.clone())
        .range(min, max)
        .height(18.0)
        .width(FILL)
        .background(PaletteColor::Muted)
        .rounded(9.0)
        .cursor(CursorIcon::Pointer)
        .tab_index(0)
        .focused(|style| style.border_inside(2.0, PaletteColor::Ring)),
    )
}

/// A texture from one color to another along its width, a stripe every other row: what the demo's styled slider
/// paints its track and its thumb with.
fn texture(start: Color, end: Color, width: u32, height: u32) -> ImageData {
  let channel = |from: u8, to: u8, at: f32| (from as f32 + (to as f32 - from as f32) * at).round() as u8;
  let mut pixels = Vec::with_capacity((width * height * 4) as usize);
  for y in 0..height {
    for x in 0..width {
      let at = if width <= 1 { 0.0 } else { x as f32 / (width - 1) as f32 };
      let stripe = if y % 2 == 0 { 0 } else { 12 };
      pixels.push(channel(start.r(), end.r(), at).saturating_add(stripe));
      pixels.push(channel(start.g(), end.g(), at).saturating_add(stripe));
      pixels.push(channel(start.b(), end.b(), at).saturating_add(stripe));
      pixels.push(255);
    }
  }
  ImageData::from_rgba(pixels, width, height)
}

/// The demo's styled slider: a track and a thumb painted with textures of the theme's chart colors, and other
/// ones under the pointer.
fn styled_slider_row(label: &str, value: &Signal<i32>) -> Column {
  let track = texture(role(PaletteColor::Chart1), role(PaletteColor::Chart2), 24, 1);
  let track_hovered = texture(role(PaletteColor::Chart2), role(PaletteColor::Chart3), 24, 1);
  let thumb = texture(role(PaletteColor::Chart4), role(PaletteColor::Chart5), 12, 12);
  let thumb_hovered = texture(role(PaletteColor::Chart5), role(PaletteColor::Chart1), 14, 14);
  Column::new()
    .width(FILL)
    .spacing(8.0)
    .child(
      Row::new()
        .width(FILL)
        .justify(Justify::SpaceBetween)
        .child(text(label, 14.0, FontWeight::SemiBold))
        .child(mono(&value.get().to_string(), 12.0).color(PaletteColor::Primary)),
    )
    .child(
      Slider::new(value.clone())
        .range(0, 100)
        .height(34.0)
        .width(FILL)
        .track(|style| {
          style
            .height(2.0)
            .background(role(PaletteColor::Secondary))
            .background_image(track)
            .background_cover()
            .rounded(1.0)
            .border_center(1.0, PaletteColor::Border)
        })
        .track_hovered(|style| {
          style
            .height(4.0)
            .background_image(track_hovered)
            .background_cover()
            .border_center(1.0, PaletteColor::Primary)
        })
        .thumb(|style| {
          style
            .size(12.0, 12.0)
            .background(role(PaletteColor::Chart4))
            .background_image(thumb)
            .background_cover()
            .rounded(6.0)
            .border_inside(2.0, PaletteColor::Foreground)
        })
        .thumb_hovered(|style| {
          style
            .size(14.0, 14.0)
            .background_image(thumb_hovered)
            .background_cover()
            .rounded(7.0)
            .border_inside(2.0, PaletteColor::Background)
        })
        .cursor(CursorIcon::Pointer)
        .tab_index(0)
        .focused(|style| style.border_inside(2.0, PaletteColor::Ring)),
    )
}

fn sizes() -> [(String, &'static str); 4] {
  [
    ("sm", "Small"),
    ("md", "Medium"),
    ("lg", "Large"),
    ("xl", "Extra Large"),
  ]
  .map(|(value, label)| (value.to_owned(), label))
}

fn fonts() -> [(String, &'static str); 3] {
  [("sans", "Sans"), ("serif", "Serif"), ("mono", "Mono")].map(|(value, label)| (value.to_owned(), label))
}

fn languages() -> [(String, &'static str); 4] {
  [("rust", "Rust"), ("go", "Go"), ("ts", "TypeScript"), ("py", "Python")]
    .map(|(value, label)| (value.to_owned(), label))
}

/// The select's style as the demo's default one: a box on the background with a line round it, the primary
/// color's line under the pointer.
fn plain_style() -> SelectStyle {
  SelectStyle::new()
    .trigger(
      SelectPartStyle::new()
        .background(PaletteColor::Background)
        .border_inside(1.0, PaletteColor::Input)
        .rounded(RadiusSize::Md)
        .padding(Padding::symmetric(10.0, 8.0))
        .min_height(38.0),
    )
    .trigger_hovered(SelectPartStyle::new().border_inside(1.0, PaletteColor::Primary))
    .menu(
      SelectPartStyle::new()
        .background(PaletteColor::Popover)
        .border_inside(1.0, PaletteColor::Border)
        .rounded(RadiusSize::Md),
    )
    .option_hovered(SelectPartStyle::new().background(PaletteColor::Muted))
    .option_selected(SelectPartStyle::new().background(PaletteColor::Secondary))
}

/// The demo's styled select: a filled box, its chevron and its edge in the primary color while it is open or under
/// the pointer, and the chosen option on the primary color.
fn strong_style() -> SelectStyle {
  let theme = lurq::ui::current_theme();
  let mut placeholder = theme.typography_style(TypographyStyle::Body);
  placeholder.font_size = 13.0;
  placeholder.weight = FontWeight::Medium;
  placeholder.color = theme.palette_color(PaletteColor::MutedForeground);
  SelectStyle::new()
    .trigger(
      SelectPartStyle::new()
        .background(PaletteColor::Secondary)
        .border_inside(1.0, PaletteColor::Border)
        .rounded(RadiusSize::Md)
        .padding(Padding::symmetric(10.0, 8.0))
        .text_color(PaletteColor::SecondaryForeground)
        .min_height(40.0),
    )
    .trigger_hovered(SelectPartStyle::new().border_inside(1.0, PaletteColor::Primary))
    .trigger_open(SelectPartStyle::new().border_inside(1.0, PaletteColor::Primary))
    .placeholder_text(placeholder)
    .chevron_color(PaletteColor::Primary)
    .menu(
      SelectPartStyle::new()
        .background(PaletteColor::Secondary)
        .border_inside(1.0, PaletteColor::Border)
        .rounded(RadiusSize::Md),
    )
    .option(SelectPartStyle::new().text_color(PaletteColor::SecondaryForeground))
    .option_hovered(SelectPartStyle::new().background(PaletteColor::Muted))
    .option_selected(
      SelectPartStyle::new()
        .background(PaletteColor::Primary)
        .text_color(PaletteColor::PrimaryForeground),
    )
}

pub struct ControlsPage {
  frame: Frame,
  playground: Playground,
  kind: Signal<usize>,
  /// An index into [`RANGES`].
  range: Signal<usize>,
  step: Signal<i32>,
  orientation: Signal<usize>,
  styled: Signal<bool>,
  value: Signal<i32>,
  values: Signal<(i32, i32)>,
  /// The engine's buttons: how many times each was pressed.
  bare_presses: Signal<i32>,
  dressed_presses: Signal<i32>,
  form: FormHandle,
  submits: Signal<i32>,
  last_submit: Signal<String>,
  /// What the form said when it was submitted with a field it does not take.
  refused: Signal<String>,
  notifications: Signal<bool>,
  beta_access: Signal<bool>,
  volume: Signal<i32>,
  priority: Signal<i32>,
  styled_gain: Signal<i32>,
  size: Signal<String>,
  font: Signal<String>,
  tags: Signal<Vec<String>>,
  langs: Signal<Vec<String>>,
}

impl ControlsPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (kind, range_at, step, orientation, styled) = (
      self.kind.get(),
      self.range.get().min(RANGES.len() - 1),
      self.step.get(),
      self.orientation.get(),
      self.styled.get(),
    );
    let (range_name, (min, max)) = RANGES[range_at];
    let vertical = orientation == 1;
    let range = kind == 1;
    let record = self.playground.recorder();
    let (value, values) = (self.value.clone(), self.values.clone());
    let build = move |record: Option<Arc<dyn Fn(String) + Send + Sync>>| -> Element {
      let mut slider = if range {
        Slider::new_range(values.clone())
      } else {
        Slider::new(value.clone())
      }
      .range(min, max)
      .step(step as f32)
      .cursor(CursorIcon::Pointer)
      .tab_index(0)
      .focused(|style| style.border_inside(2.0, PaletteColor::Ring));
      slider = if vertical {
        slider.vertical().size(THICKNESS, 200.0)
      } else {
        slider.size(LENGTH, THICKNESS).max_width(FILL)
      };
      if styled {
        let thickness =
          |style: lurq::node::SliderPartStyle| if vertical { style.width(6.0) } else { style.height(6.0) };
        slider = slider
          .track(|style| thickness(style).background(role(PaletteColor::Muted)).rounded(3.0))
          .fill(|style| thickness(style).background(role(PaletteColor::Primary)).rounded(3.0))
          .thumb(|style| {
            style
              .size(18.0, 18.0)
              .background(role(PaletteColor::Background))
              .rounded(9.0)
              .border_inside(2.0, PaletteColor::Primary)
          })
          .thumb_hovered(|style| style.size(20.0, 20.0).rounded(10.0))
          .thumb_focused(|style| style.border_outside(3.0, PaletteColor::Ring));
      }
      if let Some(record) = record {
        let (moved, ended) = (record.clone(), record);
        let say = move |low: f32, high: f32| {
          if range {
            format!("({low}, {high})")
          } else {
            format!("({low})")
          }
        };
        slider = slider
          .on_change(move |low, high| moved(format!("on_change{}", say(low, high))))
          .on_change_end(move |low, high| ended(format!("on_change_end{}", say(low, high))));
      }
      slider.into()
    };
    let record: Arc<dyn Fn(String) + Send + Sync> = Arc::new(record);
    let shown = {
      let build = build.clone();
      Shown::built(move |_| build(Some(record)))
    };
    let make = move || build(None);

    let mut code = vec![if range {
      "// values: Signal<(i32, i32)>".to_owned()
    } else {
      "// value: Signal<i32>".to_owned()
    }];
    code.push(if range {
      "Slider::new_range(values.clone())".to_owned()
    } else {
      "Slider::new(value.clone())".to_owned()
    });
    code.push(format!("    .range({min}, {max})"));
    if step != 1 {
      code.push(format!("    .step({step}.0)"));
    }
    if vertical {
      code.push("    .vertical()".to_owned());
      code.push(format!("    .size({THICKNESS}, 200.0)"));
    } else {
      code.push(format!("    .size({LENGTH}, {THICKNESS})"));
    }
    if styled {
      code.push("    .track(|s| s.height(6.0).background(muted).rounded(3.0))".to_owned());
      code.push("    .fill(|s| s.height(6.0).background(primary).rounded(3.0))".to_owned());
      code.push("    .thumb(|s| s.size(18.0, 18.0).background(background).rounded(9.0))".to_owned());
      code.push("    .thumb_hovered(|s| s.size(20.0, 20.0).rounded(10.0))".to_owned());
    }
    code.push("    .on_change(|low, high| ..)".to_owned());
    code.push("    .on_change_end(|low, high| ..)".to_owned());

    let held = if range {
      let (low, high) = self.values.get();
      format!("{low} to {high}")
    } else {
      self.value.get().to_string()
    };
    let tree = vec![
      format!("slider   {}", self.playground.measured()),
      if styled {
        "├─ track   6 thick, the muted color".to_owned()
      } else {
        "├─ track   the theme's engine_slider recipe: Border".to_owned()
      },
      if range {
        "├─ fill    between the two thumbs".to_owned()
      } else {
        "├─ fill    from the least value to the thumb".to_owned()
      },
      if range {
        "└─ thumbs  two: each a stop of Tab, the low one first".to_owned()
      } else {
        "└─ thumb   the theme's engine_slider recipe: Primary".to_owned()
      },
    ];

    let (set_kind, set_range, set_step, set_orientation) = (
      self.kind.clone(),
      self.range.clone(),
      self.step.clone(),
      self.orientation.clone(),
    );
    let reset = {
      let (kind, range, step, orientation, styled, value, values) = (
        self.kind.clone(),
        self.range.clone(),
        self.step.clone(),
        self.orientation.clone(),
        self.styled.clone(),
        self.value.clone(),
        self.values.clone(),
      );
      Arc::new(move || {
        kind.set(0);
        range.set(0);
        step.set(1);
        orientation.set(0);
        styled.set(false);
        value.set(40);
        values.set((20, 80));
      })
    };
    let controls = vec![
      prop(
        "Slider",
        "the value it is bound to",
        self
          .playground
          .select(0, &KINDS, &kind, move |value| set_kind.set(value)),
      ),
      prop(
        "range",
        "(i32, i32)",
        self
          .playground
          .select(1, &RANGE_CHOICES, &range_at, move |value| set_range.set(value)),
      ),
      prop(
        "step",
        "f32",
        self
          .playground
          .select(2, &STEPS, &step, move |value| set_step.set(value)),
      ),
      prop(
        "orientation",
        "SliderOrientation",
        self
          .playground
          .select(3, &ORIENTATIONS, &orientation, move |value| set_orientation.set(value)),
      ),
      divider(),
      flag("track / fill / thumb styles", &self.styled),
    ];
    self.playground.card(
      ctx,
      Spec {
        title: "Slider",
        make: Arc::new(make),
        shown,
        fill: false,
        forceable: false,
        facts: format!("{range_name} · holds {held}"),
        event: "on_change · on_change_end",
        code,
        tree,
        controls,
        changed: [kind != 0, range_at != 0, step != 1, vertical, styled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset,
      },
    )
  }

  /// The engine's button bare, and dressed by the app, each with how many times it was pressed.
  fn buttons(&self) -> Element {
    let (bare, dressed_count) = (self.bare_presses.clone(), self.dressed_presses.clone());
    let with_children = self.dressed_presses.clone();
    controls()
      .align_items(Alignment::Center)
      .child(
        Column::new()
          .spacing(8.0)
          .child(
            Button::new("A bare button")
              .tab_index(0)
              .cursor(CursorIcon::Pointer)
              .on_click(move |_| bare.update(|count| *count += 1)),
          )
          .child(quiet(&format!("pressed {} times", self.bare_presses.get()), 12.0)),
      )
      .child(
        Column::new()
          .spacing(8.0)
          .child(
            Row::new()
              .spacing(10.0)
              .wrap()
              .child(
                dressed(Button::new("Dressed"), true)
                  .tab_index(0)
                  .on_click(move |_| dressed_count.update(|count| *count += 1)),
              )
              .child(
                dressed(Button::empty(), false)
                  .tab_index(0)
                  .spacing(8.0)
                  .child(lurq::ui::Icon::new(lurq::ui::icons::PLUS).size(14.0))
                  .child(text("With children", 14.0, FontWeight::Medium))
                  .on_click(move |_| with_children.update(|count| *count += 1)),
              ),
          )
          .child(quiet(&format!("pressed {} times", self.dressed_presses.get()), 12.0)),
      )
      .into()
  }

  /// The demo's form, with what the form says of its fields: whether each was changed and what is wrong with it.
  fn form(&self, ctx: &mut Ctx) -> Element {
    let form = self.form.clone();
    let (user, email, newsletter) = (form.string("user"), form.string("email"), form.bool("newsletter"));
    let (user_error, email_error) = (form.error("user").get(), form.error("email").get());
    let (user_dirty, email_dirty) = (form.dirty("user").get(), form.dirty("email").get());
    let clear = {
      let (user, email, last) = (user.clone(), email.clone(), self.last_submit.clone());
      move || {
        user.set(String::new());
        email.set(String::new());
        last.set("<none>".to_owned());
      }
    };
    let restore = {
      let (form, last) = (form.clone(), self.last_submit.clone());
      move || {
        form.reset();
        last.set("<none>".to_owned());
      }
    };
    let error_line = |error: Option<Arc<str>>| match error {
      Some(error) => text(&error, 12.0, FontWeight::Normal).color(PaletteColor::Destructive),
      None => quiet(" ", 12.0),
    };
    #[allow(deprecated)] // The engine's checkbox node, which `ui::Checkbox` replaces: the form takes its value.
    let newsletter_box = lurq::components::Checkbox::new(newsletter.clone())
      .name("newsletter")
      .size(20.0, 20.0)
      .border_inside(1.0, PaletteColor::Input)
      .rounded(4.0)
      .cursor(CursorIcon::Pointer)
      .focused(|style| style.border_inside(2.0, PaletteColor::Ring));
    let fields = ctx.form_view(form.clone(), move |_| {
      Column::new()
        .width(FILL)
        .spacing(14.0)
        .child(pair(
          labelled("User", field(user.clone(), "user", "Ada Lovelace"))
            .width(FILL)
            .child(error_line(user_error.clone())),
          labelled("Contact", field(email.clone(), "email", "ada@example.com"))
            .width(FILL)
            .child(error_line(email_error.clone())),
        ))
        .child(
          Row::new()
            .spacing(10.0)
            .align_items(Alignment::Center)
            .child(newsletter_box)
            .child(text("The newsletter", 14.0, FontWeight::Normal)),
        )
        .child(
          Row::new()
            .width(FILL)
            .spacing(10.0)
            .wrap()
            .align_items(Alignment::Center)
            .child(dressed(Button::new("Clear"), false).button().on_click(move |_| clear()))
            .child(
              dressed(Button::new("Reset"), false)
                .button()
                .on_click(move |_| restore()),
            )
            .child(dressed(Button::new("Submit"), true).submit()),
        )
    });
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(fields)
      .child(pills(vec![
        ("submits", self.submits.get().to_string()),
        ("last", self.last_submit.get()),
        ("refused", self.refused.get()),
        ("user", if user_dirty { "changed" } else { "as it was" }.to_owned()),
        ("email", if email_dirty { "changed" } else { "as it was" }.to_owned()),
        ("newsletter", on_off(form.bool("newsletter").get()).to_owned()),
      ]))
      .into()
  }

  /// The demo's two checkboxes and what they hold.
  fn selection(&self) -> Element {
    Column::new()
      .width(FILL)
      .spacing(14.0)
      .child(checkbox_row(
        "Notifications",
        "Account updates",
        self.notifications.clone(),
      ))
      .child(checkbox_row("Beta access", "Early features", self.beta_access.clone()))
      .child(pills(vec![
        ("notifications", on_off(self.notifications.get()).to_owned()),
        ("beta", on_off(self.beta_access.get()).to_owned()),
      ]))
      .into()
  }

  /// The demo's three sliders and what they hold.
  fn sliders(&self) -> Element {
    Column::new()
      .width(FILL)
      .spacing(18.0)
      .child(slider_row("Volume", &self.volume, 0, 100))
      .child(slider_row("Priority", &self.priority, 1, 5))
      .child(styled_slider_row("Styled", &self.styled_gain))
      .child(pills(vec![
        ("volume", self.volume.get().to_string()),
        ("priority", self.priority.get().to_string()),
        ("styled", self.styled_gain.get().to_string()),
      ]))
      .into()
  }

  /// The demo's four selects, of one value and of several, in a plain style and a strong one.
  fn selects(&self) -> Element {
    let select = |value: &Signal<String>, options: Vec<(String, &'static str)>, placeholder: &str, style| {
      Select::new(value.clone())
        .options(options)
        .placeholder(placeholder)
        .width(FILL)
        .style(style)
    };
    let multiple = |value: &Signal<Vec<String>>, placeholder: &str, style| {
      Select::multiple(value.clone())
        .options(languages())
        .placeholder(placeholder)
        .width(FILL)
        .style(style)
    };
    Column::new()
      .width(FILL)
      .spacing(16.0)
      .child(pair(
        labelled(
          "One value, plain",
          select(&self.size, sizes().to_vec(), "Select size", plain_style()),
        )
        .width(FILL),
        labelled(
          "One value, styled",
          select(&self.font, fonts().to_vec(), "Pick a font", strong_style()),
        )
        .width(FILL),
      ))
      .child(pair(
        labelled(
          "Several, plain",
          multiple(&self.tags, "Select languages", plain_style()),
        )
        .width(FILL),
        labelled(
          "Several, styled",
          multiple(&self.langs, "Pick languages", strong_style()),
        )
        .width(FILL),
      ))
      .child(pills(vec![
        ("size", shown_text(&self.size.get()).to_owned()),
        ("font", shown_text(&self.font.get()).to_owned()),
        ("tags", listed(&self.tags.get())),
        ("langs", listed(&self.langs.get())),
      ]))
      .into()
  }

  /// The demo's snapshot of the state: what every control of the page holds, a line each.
  fn snapshot(&self) -> Element {
    let form = &self.form;
    let lines = [
      ("form user", shown_text(&form.string("user").get()).to_owned()),
      ("form email", shown_text(&form.string("email").get()).to_owned()),
      ("form submits", self.submits.get().to_string()),
      ("form last", self.last_submit.get()),
      ("notifications", on_off(self.notifications.get()).to_owned()),
      ("beta", on_off(self.beta_access.get()).to_owned()),
      ("volume", self.volume.get().to_string()),
      ("priority", self.priority.get().to_string()),
      ("styled", self.styled_gain.get().to_string()),
      ("size", shown_text(&self.size.get()).to_owned()),
      ("font", shown_text(&self.font.get()).to_owned()),
      ("tags", listed(&self.tags.get())),
      ("langs", listed(&self.langs.get())),
    ];
    let mut column = Column::new().width(FILL).spacing(8.0);
    for (name, value) in lines {
      column = column.child(named(name, mono(&value, 12.0)));
    }
    column.into()
  }
}

impl Component for ControlsPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let playground = Playground::new(ctx, 4);
    let (value, values) = (ctx.signal(40), ctx.signal((20, 80)));
    let submits = ctx.signal(0);
    let last_submit = ctx.signal("<none>".to_owned());
    let refused = ctx.signal("<none>".to_owned());
    let form = ctx
      .form(
        FormOptions::new()
          .field("user", "Ada Lovelace")
          .field("email", "ada@example.com")
          .field("newsletter", false)
          .validate_string("user", validators::required("A name is needed"))
          .validate_string("email", validators::email("Enter a valid email")),
      )
      .on_submit({
        let (submits, last, refused) = (submits.clone(), last_submit.clone(), refused.clone());
        move |values| {
          submits.update(|count| *count += 1);
          refused.set("<none>".to_owned());
          last.set(format!(
            "{} / {} / {}",
            shown_text(values.get_string("user").unwrap_or_default()),
            shown_text(values.get_string("email").unwrap_or_default()),
            on_off(values.get_bool("newsletter").unwrap_or(false)),
          ));
        }
      })
      .on_invalid({
        let refused = refused.clone();
        move |errors| {
          let first = ["user", "email"]
            .into_iter()
            .find_map(|name| errors.first(name).map(|error| format!("{name}: {error}")));
          refused.set(first.unwrap_or_else(|| "<none>".to_owned()));
        }
      });
    Self {
      frame: Frame::new(ctx),
      playground,
      kind: ctx.signal(0),
      range: ctx.signal(0),
      step: ctx.signal(1),
      orientation: ctx.signal(0),
      styled: ctx.signal(false),
      value,
      values,
      bare_presses: ctx.signal(0),
      dressed_presses: ctx.signal(0),
      form,
      submits,
      last_submit,
      refused,
      notifications: ctx.signal(true),
      beta_access: ctx.signal(false),
      volume: ctx.signal(42),
      priority: ctx.signal(3),
      styled_gain: ctx.signal(68),
      size: ctx.signal("md".to_owned()),
      font: ctx.signal("sans".to_owned()),
      tags: ctx.signal(vec!["rust".to_owned()]),
      langs: ctx.signal(Vec::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "Slider::new / new_range",
        "Signal<i32> / Signal<(i32, i32)>",
        "",
        "Slider: one thumb bound to a value, or two bound to a low and a high one that do not pass each other. \
         new_f32 and new_range_f32 are the same of fractions.",
      ),
      (
        "range / step",
        "i32, i32 / f32",
        "0, 100 / 1",
        "Slider: the least and the most value, and the step the value moves by.",
      ),
      (
        "orientation / vertical",
        "SliderOrientation",
        "Horizontal",
        "Slider: which way the track runs. A vertical one has its least value at the bottom.",
      ),
      (
        "track / fill / thumb",
        "FnOnce(SliderPartStyle) -> SliderPartStyle",
        "",
        "Slider: the style of each part, and of each under the pointer (track_hovered, thumb_hovered) and of the \
         thumb focused from the keyboard (thumb_focused). Without them the theme's engine_slider recipe fills \
         them.",
      ),
      (
        "on_change / on_change_end",
        "Fn(f32, f32)",
        "",
        "Slider: a person moved the value, and the move is over. Told the low and the high value; one value twice.",
      ),
      (
        "Button::new / empty",
        "&str / ()",
        "",
        "Button: a row with the label, or with no child for the app's children. It has no look of its own.",
      ),
      (
        "button / submit",
        "()",
        "button",
        "Button: a button that does what its on_click says, or one that submits the form it is in.",
      ),
      (
        "Select::new / multiple",
        "Signal<T> / Signal<Vec<T>>",
        "",
        "Select: one value, or a set of them, chosen from options of (value, label).",
      ),
      (
        "options / placeholder / style",
        "(T, &str) / &str / SelectStyle",
        "",
        "Select: what can be chosen, what the box says with nothing chosen, and the style of each part.",
      ),
      (
        "Checkbox::new",
        "Signal<bool>",
        "",
        "Checkbox (deprecated for ui::Checkbox): a box bound to a value, with no label.",
      ),
      (
        "ctx.form",
        "FormOptions",
        "",
        "The form: its fields with their defaults and validators, on_submit with the values, on_invalid with the \
         errors.",
      ),
      (
        "ctx.form_view",
        "FormHandle, FnOnce(&mut Ctx)",
        "",
        "What the form holds: a control in it with a name gives its value; Enter in a field and a submit button \
         submit it.",
      ),
      (
        "name",
        "&str",
        "",
        "On a text input, a checkbox, a slider or a select: the field of the form the control gives its value to.",
      ),
    ];
    let playground = self.play(ctx);
    let form = self.form(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Controls"],
        title: "Controls",
        description: "The engine's own controls: a slider, a button with no look of its own, a select of one \
                      value or of several, a checkbox, and the form they give their values to. The design \
                      system's controls are built on them; an app normally wants those.",
        import: Some("use lurq::components::{Button, FormOptions, Select, Slider};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "The engine's slider. Drag the thumb, press the track, or move it with the arrow keys, Page and \
           Home/End; give it two thumbs, another range or step, stand it up, and give its parts styles of their \
           own.",
          playground,
        ),
        Section::new(
          "Button",
          "The engine's button has no look: bare, it is its label, pressed with the pointer and with Enter or \
           Space where it is a stop of Tab. The app dresses it with a fill, an edge and styles for under the \
           pointer, pressed and focused, and may give it children of its own.",
          self.frame.demo(
            0,
            self.buttons(),
            "Button::new(\"Dressed\").background(PaletteColor::Primary)",
            &[
              "Button::new(\"A bare button\").tab_index(0).on_click(|_| ..)",
              "",
              "Button::new(\"Dressed\")",
              "    .height(34.0)",
              "    .padding_horizontal(14.0)",
              "    .background(PaletteColor::Primary)",
              "    .foreground(PaletteColor::PrimaryForeground)",
              "    .rounded(RadiusSize::Md)",
              "    .hovered(|s| s.opacity(0.9))",
              "    .focused(|s| s.border_outside(2.0, PaletteColor::Ring))",
              "",
              "Button::empty().child(Icon::new(icons::PLUS)).child(Text::new(\"With children\"))",
            ],
          ),
        ),
        Section::new(
          "Form",
          "The demo's form: two named fields, a box, and buttons that clear the fields, put them back and submit. \
           Enter in a field submits it too. Empty the user or spoil the email and submit: the form refuses and \
           says why under the field.",
          self.frame.demo(
            1,
            form,
            "ctx.form_view(form.clone(), |_| ..)",
            &[
              "let form = ctx",
              "    .form(",
              "        FormOptions::new()",
              "            .field(\"user\", \"Ada Lovelace\")",
              "            .field(\"email\", \"ada@example.com\")",
              "            .field(\"newsletter\", false)",
              "            .validate_string(\"user\", validators::required(\"A name is needed\"))",
              "            .validate_string(\"email\", validators::email(\"Enter a valid email\")),",
              "    )",
              "    .on_submit(|values| ..)",
              "    .on_invalid(|errors| ..);",
              "",
              "ctx.form_view(form.clone(), |_| {",
              "    Column::new()",
              "        .child(TextInput::new(form.string(\"user\")).name(\"user\"))",
              "        .child(TextInput::new(form.string(\"email\")).name(\"email\"))",
              "        .child(Checkbox::new(form.bool(\"newsletter\")).name(\"newsletter\"))",
              "        .child(Button::new(\"Submit\").submit())",
              "})",
            ],
          ),
        ),
        Section::new(
          "Selection",
          "The engine's checkbox node: a box bound to a value, with no label of its own. It is deprecated for \
           apps, which use ui::Checkbox, the design's checkbox with its label; it is shown as what that is built \
           on, and under #[allow(deprecated)].",
          self.frame.demo(
            2,
            self.selection(),
            "Checkbox::new(notifications.clone())",
            &[
              "#[allow(deprecated)]",
              "Checkbox::new(notifications.clone())",
              "    .size(20.0, 20.0)",
              "    .border_inside(1.0, PaletteColor::Input)",
              "    .rounded(4.0)",
              "    .focused(|s| s.border_inside(2.0, PaletteColor::Ring))",
            ],
          ),
        ),
        Section::new(
          "Sliders",
          "Three sliders: a volume from 0 to 100, a priority from 1 to 5, and one whose track and thumb are \
           painted with pictures, others under the pointer.",
          self.frame.demo(
            3,
            self.sliders(),
            "Slider::new(volume.clone()).range(0, 100)",
            &[
              "Slider::new(volume.clone())",
              "    .range(0, 100)",
              "    .height(18.0)",
              "    .background(PaletteColor::Muted)",
              "    .rounded(9.0)",
              "",
              "Slider::new(gain.clone())",
              "    .track(|s| s.size(220.0, 2.0).background_image(track).background_cover())",
              "    .track_hovered(|s| s.height(4.0).background_image(track_hovered))",
              "    .thumb(|s| s.size(12.0, 12.0).background_image(thumb).rounded(6.0))",
              "    .thumb_hovered(|s| s.size(14.0, 14.0).background_image(thumb_hovered))",
            ],
          ),
        ),
        Section::new(
          "Selects",
          "A select of one value and one of several, each in a plain style and a strong one. A select of several \
           stays open while options are chosen, and says how many are.",
          self.frame.demo(
            4,
            self.selects(),
            "Select::multiple(tags.clone()).options(languages)",
            &[
              "Select::new(size.clone())",
              "    .options([(\"sm\", \"Small\"), (\"md\", \"Medium\"), ..])",
              "    .placeholder(\"Select size\")",
              "    .style(SelectStyle::new().trigger(SelectPartStyle::new().background(..)))",
              "",
              "Select::multiple(tags.clone())",
              "    .options([(\"rust\", \"Rust\"), (\"go\", \"Go\"), ..])",
              "    .placeholder(\"Select languages\")",
            ],
          ),
        ),
        Section::new(
          "What the controls hold",
          "The demo's snapshot of the state: the value of every control on this page, read from its signal.",
          self.frame.demo(
            5,
            self.snapshot(),
            "volume.get()",
            &["// Each line reads the signal its control is bound to.", "volume.get()"],
          ),
        ),
        Section::new(
          "Props",
          "What the engine's slider, button, select and checkbox are given, and what a form is.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
