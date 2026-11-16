//! The page of the signature pad: its playground, the board's pad signed, the signature as a picture, its states, and
//! a pad in a field.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{
    Button, ButtonSize, ButtonVariant, Field, Interaction, Picture, Signature, SignaturePad, SignaturePoint,
    SignatureStroke, current_theme,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag},
};

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
    .align_items(Alignment::End)
    .overflow_visible()
}

/// A signature as the board draws one: a wave of three loops over the line, and a rise at its end, on a pad of 280
/// by 124.
fn signed() -> Signature {
  let mut signature = Signature::new(280.0, 124.0);
  let points = (0..=72).map(|step| {
    let t = step as f32 / 72.0;
    let x = 18.0 + 176.0 * t;
    // Three loops that fade into the rise.
    let wave = (t * std::f32::consts::TAU * 3.0).sin() * 16.0 * (1.0 - t * 0.6);
    let rise = if t > 0.8 { (t - 0.8) * 60.0 } else { 0.0 };
    SignaturePoint {
      x,
      y: 62.0 - wave - rise,
      pressure: None,
      time: step * 12,
    }
  });
  signature.strokes.push(SignatureStroke::new(points));
  signature
}

/// What a signal of a signature holds, as the page says it.
fn says(signature: &Signature) -> String {
  let points: usize = signature.strokes.iter().map(|stroke| stroke.points.len()).sum();
  match signature.strokes.len() {
    0 => "the signal has: nothing signed".to_owned(),
    1 => format!("the signal has: 1 stroke of {points} points"),
    strokes => format!("the signal has: {strokes} strokes, {points} points"),
  }
}

pub struct SignaturePadPage {
  frame: Frame,
  playground: Playground,
  signature: Signal<Signature>,
  read_only: Signal<bool>,
  disabled: Signal<bool>,
  invalid: Signal<bool>,
  full_width: Signal<bool>,
  board: Signal<Signature>,
  picture: Signal<bool>,
  consent: Signal<Signature>,
}

impl SignaturePadPage {
  /// The playground: the pad as the panel's props make it, bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (read_only, disabled) = (self.read_only.get(), self.disabled.get());
    let (invalid, full_width) = (self.invalid.get(), self.full_width.get());
    let signature = self.signature.clone();
    let plain = move |signature: Signal<Signature>| {
      SignaturePad::new(signature)
        .label("Signature")
        .read_only(read_only)
        .disabled(disabled)
        .invalid(invalid)
        .full_width(full_width)
    };
    let record = self.playground.recorder();
    let forced = self.playground.forced();
    let shown = {
      let (plain, signature) = (plain.clone(), signature.clone());
      Shown::built(move |ctx| {
        plain(signature)
          .force(forced)
          .on_change(move |signature| {
            record(format!("on_change({} strokes)", signature.strokes.len()));
          })
          .mount(ctx)
      })
    };

    let mut code = vec![
      "SignaturePad::new(signature)".to_owned(),
      "    .label(\"Signature\")".to_owned(),
    ];
    for (on, call) in [
      (read_only, "    .read_only(true)"),
      (disabled, "    .disabled(true)"),
      (invalid, "    .invalid(true)"),
      (full_width, "    .full_width(true)"),
    ] {
      if on {
        code.push(call.to_owned());
      }
    }
    code.push("    .mount(ctx)".to_owned());

    let held = self.signature.get();
    let interactive = !read_only && !disabled;
    let mut tree = vec![
      format!(
        "root         {}   background · edge 1 input · radius lg · an img to tooling",
        self.playground.measured()
      ),
      "├─ guide     padding 12 16 · gap 6 · at the bottom".to_owned(),
      "│  ├─ baseline   1 high · input".to_owned(),
      "│  └─ placeholder   mono 11 · muted foreground".to_owned(),
      format!(
        "{} surface   the whole pad · a canvas · ink 2 foreground{}",
        if !held.is_empty() && interactive {
          "├─"
        } else {
          "└─"
        },
        if interactive { " · a cross for the pointer" } else { "" }
      ),
    ];
    if !held.is_empty() && interactive {
      tree.push("└─ actions   top corner, 6 in · Undo and Clear, ghost xs".to_owned());
    }

    let controls = vec![
      flag("read_only", &self.read_only),
      flag("disabled", &self.disabled),
      flag("invalid", &self.invalid),
      divider(),
      flag("full_width", &self.full_width),
    ];
    let resets = (
      self.signature.clone(),
      [
        self.read_only.clone(),
        self.disabled.clone(),
        self.invalid.clone(),
        self.full_width.clone(),
      ],
    );
    let make_signature = Signal::new(held.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Signature Pad",
        make: Arc::new(move || plain(make_signature.clone()).into()),
        shown,
        fill: full_width,
        forceable: true,
        facts: says(&held).replace("the signal has: ", ""),
        event: "on_change",
        code,
        tree,
        controls,
        changed: [read_only, disabled, invalid, full_width]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          resets.0.set(Signature::default());
          for flag in &resets.1 {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for SignaturePadPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      signature: ctx.signal(Signature::default()),
      read_only: ctx.signal(false),
      disabled: ctx.signal(false),
      invalid: ctx.signal(false),
      full_width: ctx.signal(false),
      board: ctx.signal(signed()),
      picture: ctx.signal(false),
      consent: ctx.signal(Signature::default()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let playground = self.play(ctx);
    let board = items().child(item(
      SignaturePad::new(self.board.clone()).label("Signature").mount(ctx),
      &says(&self.board.get()),
    ));

    // The signature as a picture: made by a press, from what the pad above holds.
    let picture = {
      let made = self.picture.clone();
      let make = Button::new("Make a picture")
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .on_click(move |_| made.set(true));
      let ink = current_theme().palette_color(PaletteColor::Foreground);
      let image = self
        .picture
        .get()
        .then(|| self.board.get().to_image(2.0, ink, 2.0))
        .flatten();
      let caption = match &image {
        Some(image) => {
          let bytes = self.board.get().to_png(2.0, ink, 2.0).map_or(0, |png| png.len());
          format!(
            "to_image(2.0, ink, 2.0): {} × {} px · to_png: {bytes} bytes",
            image.width(),
            image.height()
          )
        }
        None if self.picture.get() => "Nothing signed: no picture.".to_owned(),
        None => "to_image(scale, ink, line_width)".to_owned(),
      };
      let shown: Element = match image {
        Some(image) => Picture::new(image)
          .label("The signature, as a picture")
          .size_px(140.0, 62.0)
          .into(),
        None => quiet("No picture yet.", 12.0).into(),
      };
      items().child(item(
        Row::new()
          .spacing(16.0)
          .align_items(Alignment::Center)
          .child(make)
          .child(shown),
        &caption,
      ))
    };

    let states = items()
      .child(item(
        SignaturePad::controlled(signed(), |_| {}).force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(
        SignaturePad::controlled(signed(), |_| {}).read_only(true),
        "read only",
      ))
      .child(item(
        SignaturePad::controlled(Signature::default(), |_| {}).invalid(true),
        "invalid",
      ))
      .child(item(
        SignaturePad::controlled(signed(), |_| {}).disabled(true),
        "disabled",
      ));

    let consent = self.consent.get();
    let field = items().child(item(
      Column::new().width(320.0).max_width(FILL).child({
        let field = Field::new(
          "Signature of the recipient",
          SignaturePad::new(self.consent.clone()).placeholder("Sign to accept the delivery"),
        );
        if consent.is_empty() {
          field.error("Sign to accept the delivery.")
        } else {
          field.description("Signed: the delivery is accepted.")
        }
      }),
      &says(&consent),
    ));

    let rows = [
      (
        "new",
        "Signal<Signature>",
        "",
        "The signature the signal holds; each stroke, Undo and Clear set it.",
      ),
      (
        "controlled",
        "Signature, impl Fn(&Signature)",
        "",
        "In place of a signal: the app holds the signature and is told what a stroke, Undo or Clear make of it.",
      ),
      (
        "on_change",
        "impl Fn(&Signature)",
        "",
        "Called after a stroke is let go, after Undo and after Clear, once the signal is set. Not while a stroke \
         is drawn.",
      ),
      (
        "placeholder",
        "&str",
        "\"Sign above the line\"",
        "The words under the line. They stay once it is signed.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "Shows the signature and takes nothing: no stroke, no buttons, no stop of Tab.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Dimmed; it takes no input and is no stop of Tab.",
      ),
      ("invalid", "bool", "false", "Its edge in the destructive color."),
      (
        "full_width",
        "bool",
        "false",
        "As wide as what holds it, in place of 280.",
      ),
      ("width, height", "impl Into<Length>", "280, 124", "Its size."),
      (
        "label",
        "&str",
        "\"Signature\"",
        "What it is called to tooling. A field gives it its label.",
      ),
      (
        "undo_label, clear_label",
        "&str",
        "\"Undo\", \"Clear\"",
        "The names of its buttons.",
      ),
      (
        "signed_text, empty_text",
        "&str",
        "\"Signed\", \"Not signed\"",
        "What it says to tooling it holds.",
      ),
    ];
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Signature Pad"],
        title: "Signature Pad",
        description: "A surface a person signs on with a mouse, a pen or a finger: the signature on a delivery, the \
                      consent on a form. The signature is the app's: its strokes, as points with their pressure and \
                      time.",
        import: Some("use lurq::ui::{Signature, SignaturePad};"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of SignaturePad is a control on the right. Sign on the stage: each stroke let go is told, \
           and Undo and Clear in its corner, or Backspace and Delete with the pad focused, take strokes back.",
          playground,
        ),
        Section::new(
          "Signature pad",
          "The board's, signed: the ink 2 wide in the foreground's color, smoothed through the points the pointer \
           gives. The page holds the signature in a signal. The pad is one stop of Tab and its two buttons the next; \
           to a screen reader it is a picture that says whether it is signed.",
          self.frame.demo(
            0,
            board,
            "SignaturePad::new(signature).mount(ctx)",
            &[
              "// signature: Signal<Signature>",
              "SignaturePad::new(signature).label(\"Signature\").mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "As a picture",
          "Signature::to_image draws the signature on lurq's CPU canvas, at its own size times a scale, in the ink \
           and the width given, on nothing: an ImageData to save or to send.",
          self.frame.demo(
            1,
            picture,
            "signature.to_image(2.0, ink, 2.0)",
            &[
              "let ink = theme.palette_color(PaletteColor::Foreground);",
              "if let Some(image) = signature.get().to_image(2.0, ink, 2.0) {",
              "    // image.width(), image.height(), image.data(): RGBA",
              "}",
              "let png: Option<Vec<u8>> = signature.get().to_png(2.0, ink, 2.0);",
            ],
          ),
        ),
        Section::new(
          "States",
          "The board draws the pad signed. The ring of the focus is around the pad; a read-only pad shows the \
           signature and has no buttons, an invalid one has its edge in the destructive color, and a disabled one \
           is dimmed.",
          self.frame.demo(
            2,
            states,
            ".read_only(true)",
            &[
              "SignaturePad::new(signature).force(Some(Interaction::FocusVisible))",
              "SignaturePad::new(signature).read_only(true)",
              "SignaturePad::new(signature).invalid(true)",
              "SignaturePad::new(signature).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names the pad and judges it: until it is signed the field says what is wrong.",
          self.frame.demo(
            3,
            field,
            "Field::new(\"Signature of the recipient\", SignaturePad::new(consent))",
            &[
              "// consent: Signal<Signature>",
              "let field = Field::new(",
              "    \"Signature of the recipient\",",
              "    SignaturePad::new(consent.clone()).placeholder(\"Sign to accept the delivery\"),",
              ");",
              "if consent.get().is_empty() {",
              "    field.error(\"Sign to accept the delivery.\")",
              "} else {",
              "    field.description(\"Signed: the delivery is accepted.\")",
              "}",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a signature pad is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
