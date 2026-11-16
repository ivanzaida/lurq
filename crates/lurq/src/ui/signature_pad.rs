//! A signature pad: a surface a person signs on with a pointer, a pen or a finger, and the signature as data the app
//! keeps.

use std::{
  hash::Hasher,
  sync::{Arc, Mutex},
};

use super::{
  Button, ButtonSize, ButtonVariant, icons, names,
  part_style::{FontFamily, Length, PartStyle},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, one_line},
  sparkline::{color_of, paint_key, width_of},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{DragEvent, KeyboardEvent, MouseButtonMask, TouchEvent},
    theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme},
  },
  canvas::{ArcDirection, CanvasHandle, Context2D, LineCap, LineJoin},
  components::{Canvas, Column, Row, Stack},
  core::{ElementRef, Signal},
  images::ImageData,
  layout::{Alignment, layout_kind::Justify, stack_alignment::StackAlignment},
  node::{CursorIcon, Element, border::Border, color::Color, dimension::Dimension},
  time::Instant,
};

/// The parts of a signature pad a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SignaturePadPart {
  /// The pad: its box, its edge and the ring of the focus. What takes the focus and the keys.
  Root,
  /// The surface the signature is drawn on and signed on: the whole of the pad.
  Surface,
  /// The signature's line: drawn in its part's background, as wide as its part's width.
  Ink,
  /// The column at the bottom of the pad that holds the line signed on and the words under it: its padding and gap.
  Guide,
  /// The line signed on.
  Baseline,
  /// The words under the line: "Sign above the line".
  Placeholder,
  /// The row of Undo and Clear in the pad's top corner, shown once it is signed.
  Actions,
}

names!(SignaturePadPart {
  Root => "root",
  Surface => "surface",
  Ink => "ink",
  Guide => "guide",
  Baseline => "baseline",
  Placeholder => "placeholder",
  Actions => "actions",
});

/// The states of a signature pad a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SignaturePadState {
  /// Nothing is signed. One of this and [`SignaturePadState::Signed`] is on.
  Empty,
  /// It holds a signature of a stroke or more.
  Signed,
  /// A stroke is being drawn.
  Drawing,
  /// It is signed on: a pointer draws, and its keys and its buttons take strokes back.
  Interactive,
  /// It shows the signature and takes nothing.
  ReadOnly,
  /// What it holds is wrong: a form says so.
  Invalid,
  /// It takes no input.
  Disabled,
}

names!(SignaturePadState {
  Empty => "empty",
  Signed => "signed",
  Drawing => "drawing",
  Interactive => "interactive",
  ReadOnly => "read_only",
  Invalid => "invalid",
  Disabled => "disabled",
});

/// A point of a stroke: where it is, in logical pixels from the top left of the surface the signature was signed on,
/// how hard it was pressed, and when.
#[derive(Clone, Copy, Debug, PartialEq, crate::DevtoolsInspectable)]
pub struct SignaturePoint {
  pub x: f32,
  pub y: f32,
  /// How hard the pen or the finger pressed, from 0 to 1, where the input measures it (a pen, a screen that feels
  /// force); `None` for a mouse.
  pub pressure: Option<f32>,
  /// When, in milliseconds from the first point of its stroke.
  pub time: u32,
}

impl SignaturePoint {
  /// A point at `x`, `y`, pressed with no pressure measured, at the start of its stroke.
  pub fn new(x: f32, y: f32) -> Self {
    Self {
      x,
      y,
      pressure: None,
      time: 0,
    }
  }
}

/// One stroke of a signature: the points of one press, from where it went down to where it was lifted. A stroke of
/// one point is a dot.
#[derive(Clone, Debug, Default, PartialEq, crate::DevtoolsInspectable)]
pub struct SignatureStroke {
  pub points: Vec<SignaturePoint>,
}

impl SignatureStroke {
  /// A stroke through `points`, the first where it went down.
  pub fn new(points: impl IntoIterator<Item = SignaturePoint>) -> Self {
    Self {
      points: points.into_iter().collect(),
    }
  }
}

/// A signature as data: its strokes, in the order they were drawn, and the size of the surface they were drawn on.
/// What an app keeps and gives back: the pad draws it again, scaled to fit the pad it is shown on, and
/// [`to_image`](Self::to_image) makes a picture of it.
///
/// ```
/// use lurq::ui::{Signature, SignaturePoint, SignatureStroke};
///
/// let mut signature = Signature::new(280.0, 124.0);
/// signature.strokes.push(SignatureStroke::new([SignaturePoint::new(20.0, 60.0), SignaturePoint::new(80.0, 40.0)]));
/// assert!(!signature.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, crate::DevtoolsInspectable)]
pub struct Signature {
  /// The width of the surface it was drawn on, in logical pixels: 0 for a signature of no strokes.
  pub width: f32,
  /// The height of the surface it was drawn on.
  pub height: f32,
  pub strokes: Vec<SignatureStroke>,
}

impl Signature {
  /// A signature of no strokes, to be drawn on a surface of `width` by `height`.
  pub fn new(width: f32, height: f32) -> Self {
    Self {
      width,
      height,
      strokes: Vec::new(),
    }
  }

  /// Whether nothing is signed.
  pub fn is_empty(&self) -> bool {
    self.strokes.iter().all(|stroke| stroke.points.is_empty())
  }

  /// How much a signature drawn on its own surface is scaled to fit one of `width` by `height`, from its top left:
  /// the same across and down, so that it keeps its shape.
  pub fn scale_to(&self, width: f32, height: f32) -> f32 {
    if self.width <= 0.0 || self.height <= 0.0 || width <= 0.0 || height <= 0.0 {
      return 1.0;
    }
    (width / self.width).min(height / self.height)
  }

  /// Draws the signature on `draw`, scaled to fit `width` by `height` from its top left: each stroke a line of
  /// `line_width` in `ink`, round at its ends and its joins, smoothed through its points (a curve through the middles
  /// between them); a stroke of one point a dot as wide as the line.
  pub fn paint(&self, draw: &Context2D, width: f32, height: f32, ink: Color, line_width: f32) {
    let scale = self.scale_to(width, height);
    for stroke in &self.strokes {
      let points: Vec<(f32, f32)> = stroke.points.iter().map(|p| (p.x * scale, p.y * scale)).collect();
      trace(draw, &points, ink, line_width);
    }
  }

  /// The signature as a picture: its own size times `scale_factor` in device pixels, its strokes in `ink` and as
  /// wide as `line_width` (in logical pixels, scaled with it), on nothing. Drawn on lurq's CPU canvas, so that it
  /// is the same wherever it is made and needs no window. `None` for a signature of no size, or larger than a canvas
  /// can be. Behind the `canvas` feature, as the pad is.
  pub fn to_image(&self, scale_factor: f32, ink: Color, line_width: f32) -> Option<ImageData> {
    let canvas = CanvasHandle::offscreen(self.width, self.height, scale_factor)?;
    self.paint(&canvas.context_2d(), self.width, self.height, ink, line_width);
    canvas.to_image_data()
  }

  /// The picture of [`to_image`](Self::to_image) as the bytes of a PNG file, 8 bits of red, green, blue and alpha (not
  /// premultiplied), to store or to send. `None` where `to_image` is `None`.
  pub fn to_png(&self, scale_factor: f32, ink: Color, line_width: f32) -> Option<Vec<u8>> {
    let image = self.to_image(scale_factor, ink, line_width)?;
    let mut out = Vec::new();
    {
      let mut encoder = png::Encoder::new(&mut out, image.width(), image.height());
      encoder.set_color(png::ColorType::Rgba);
      encoder.set_depth(png::BitDepth::Eight);
      let mut writer = encoder.write_header().ok()?;
      // The canvas's pixels are given straight (`CanvasHandle::to_image_data`), as a PNG holds them.
      writer.write_image_data(image.data()).ok()?;
    }
    Some(out)
  }
}

/// Draws `points` as one stroke on `draw`: a curve through the middles between them, or a dot of one.
fn trace(draw: &Context2D, points: &[(f32, f32)], ink: Color, line_width: f32) {
  let Some(&(first_x, first_y)) = points.first() else {
    return;
  };
  // A stroke that did not move is a dot.
  let still = points
    .iter()
    .all(|(x, y)| (x - first_x).abs() < 0.5 && (y - first_y).abs() < 0.5);
  if still {
    draw.set_fill_style(ink);
    draw.begin_path();
    let _ = draw.arc(
      first_x,
      first_y,
      line_width / 2.0,
      0.0,
      std::f32::consts::TAU,
      ArcDirection::Clockwise,
    );
    draw.fill();
    return;
  }
  draw.set_stroke_style(ink);
  draw.set_line_width(line_width);
  draw.set_line_cap(LineCap::Round);
  draw.set_line_join(LineJoin::Round);
  draw.begin_path();
  draw.move_to(first_x, first_y);
  if points.len() == 2 {
    draw.line_to(points[1].0, points[1].1);
  } else {
    // Each point bends the line toward it on its way to the middle of it and the next: the line is smooth where
    // the pointer reported a corner between two of its moves.
    for pair in points[1..].windows(2) {
      let ((x, y), (next_x, next_y)) = (pair[0], pair[1]);
      draw.quadratic_curve_to(x, y, (x + next_x) / 2.0, (y + next_y) / 2.0);
    }
    let (last_x, last_y) = points[points.len() - 1];
    draw.line_to(last_x, last_y);
  }
  draw.stroke();
}

/// How far, in logical pixels, a pointer must move from the last point kept for its next point to be kept: the
/// pointer's jitter is not a part of the line.
const LEAST_STEP: f32 = 0.75;

/// What builds a signature pad's nodes: a theme's in place of the default one.
pub type SignaturePadRenderer = Arc<dyn Fn(SignaturePadView) -> Element + Send + Sync>;

type Told = Arc<dyn Fn(&Signature) + Send + Sync>;

/// A signature pad: a surface a person signs on with a mouse, a pen or a finger. The signature on a delivery, the
/// consent on a form.
///
/// ```
/// use lurq::{core::Signal, ui::{Signature, SignaturePad}};
///
/// # fn pads(signature: Signal<Signature>) {
/// // Bound to the app's signal, which every stroke, Undo and Clear set.
/// let pad = SignaturePad::new(signature).label("Signature of the recipient");
/// // The app holds the signature, and is told what a person makes of it.
/// let held = SignaturePad::controlled(Signature::default(), |signature| println!("{} strokes", signature.strokes.len()));
/// # }
/// ```
///
/// A press draws a stroke where the pointer goes until it is let go, smoothed through the points the pointer gives;
/// a pen or a finger draws one too (a finger on the pad draws, and does not scroll what the pad is in). The pad
/// keeps the stroke being drawn; the signature, every stroke made, is the app's ([`Signature`]): points with their
/// pressure where the input measures it and their time, which [`SignaturePad::on_change`] is told after each
/// stroke, after Undo and after Clear. Undo takes back the last stroke and Clear all of them: buttons in the pad's
/// top corner once it is signed, and Backspace (or Ctrl+Z, Cmd+Z on a Mac) and Delete on the pad focused. The pad is
/// one stop of Tab, and its buttons the next two. To tooling it is a picture called by its label, described as signed
/// or not.
///
/// It is drawn on the canvas, and only when what it shows changes. Where it keeps its stroke (converted into an
/// element in a tree, or [`SignaturePad::mount`]) it is signed on; [`SignaturePad::build`] makes the still form,
/// which shows the signature and takes nothing.
pub struct SignaturePad {
  value: Signature,
  bound: Option<Signal<Signature>>,
  on_change: Vec<Told>,
  label: Option<Arc<str>>,
  placeholder: Option<Arc<str>>,
  undo_label: Option<Arc<str>>,
  clear_label: Option<Arc<str>>,
  signed_text: Option<Arc<str>>,
  empty_text: Option<Arc<str>>,
  /// The words it was told to speak in, in place of the app's.
  locale: Option<crate::core::InputWords>,
  read_only: bool,
  disabled: bool,
  invalid: bool,
  full_width: bool,
  width: Option<Length>,
  height: Option<Length>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl SignaturePad {
  fn of(value: Signature, bound: Option<Signal<Signature>>) -> Self {
    Self {
      value,
      bound,
      on_change: Vec::new(),
      label: None,
      placeholder: None,
      undo_label: None,
      clear_label: None,
      signed_text: None,
      empty_text: None,
      locale: None,
      read_only: false,
      disabled: false,
      invalid: false,
      full_width: false,
      width: None,
      height: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// A pad of the signature the signal holds: each stroke, Undo and Clear set it.
  pub fn new(signature: Signal<Signature>) -> Self {
    Self::of(signature.get(), Some(signature))
  }

  /// A pad that shows `signature`, held by the app, and tells `on_change` what a stroke, Undo or Clear make of it:
  /// what is drawn is what the app gives next.
  pub fn controlled(signature: Signature, on_change: impl Fn(&Signature) + Send + Sync + 'static) -> Self {
    Self::of(signature, None).on_change(on_change)
  }

  /// Called with the signature after a stroke is drawn, after Undo and after Clear, once the signal it is bound to
  /// is set. Not while a stroke is drawn, nor when the app sets the signal. A second handler is called after the
  /// first.
  pub fn on_change(mut self, handler: impl Fn(&Signature) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// What it is called to tooling: the locale's "Signature" unless told. A field gives it its label.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// The words under the line: the locale's "Sign above the line" unless told. They stay once it is signed, as the
  /// board draws them.
  pub fn placeholder(mut self, placeholder: &str) -> Self {
    self.placeholder = Some(Arc::from(placeholder));
    self
  }

  /// The name of the button that takes back the last stroke: the locale's "Undo" unless told.
  pub fn undo_label(mut self, label: &str) -> Self {
    self.undo_label = Some(Arc::from(label));
    self
  }

  /// The name of the button that takes back every stroke: the locale's "Clear" unless told.
  pub fn clear_label(mut self, label: &str) -> Self {
    self.clear_label = Some(Arc::from(label));
    self
  }

  /// What the pad says to tooling once it is signed: the locale's "Signed" unless told.
  pub fn signed_text(mut self, text: &str) -> Self {
    self.signed_text = Some(Arc::from(text));
    self
  }

  /// What the pad says to tooling while nothing is signed: the locale's "Not signed" unless told.
  pub fn empty_text(mut self, text: &str) -> Self {
    self.empty_text = Some(Arc::from(text));
    self
  }

  /// The words this pad speaks in, in place of the app's ([`App::set_locale`](crate::app::App::set_locale)): its
  /// name, the words under its line, its buttons' names and what it says it holds. A word told by itself
  /// (`placeholder`, `undo_label` and the others) stays what it was told.
  pub fn locale(mut self, locale: &crate::core::Locale) -> Self {
    self.locale = Some(locale.input.clone());
    self
  }

  /// Whether it shows the signature and takes nothing: no stroke, no buttons, no stop of Tab.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.read_only = read_only;
    self
  }

  /// Whether it takes no input: dimmed, no buttons, and no stop of Tab.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether what it holds is wrong: its edge in the destructive color.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether it is as wide as what holds it, in place of the board's 280.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Its width: the board's 280 unless given, and no wider than what holds it.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// Its height: the board's 124 unless given.
  pub fn height(mut self, height: impl Into<Length>) -> Self {
    self.height = Some(height.into());
    self
  }

  /// Puts the pad in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this pad over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this pad: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the pad as it is focused from the keyboard, with nothing pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the pad: what takes the focus. Its surface is `<id>-surface`, its buttons `<id>-undo` and
  /// `<id>-clear`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells the pad apart from those beside it, and keeps the stroke being drawn with it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the still form of the pad in `theme`: the signature shown, nothing taken. Converting a pad into an
  /// element, or [`mount`](Self::mount), makes the pad that is signed on.
  pub fn build(self, theme: &Theme) -> Element {
    self.core().build(theme, None)
  }

  /// Mounts the pad, which keeps the stroke being drawn: the pad that is signed on.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = SignaturePadProps(self.core());
    match key {
      Some(key) => ctx.mount_keyed::<SignaturePadHost>(&key, props),
      None => ctx.mount::<SignaturePadHost>(props),
    }
  }

  fn core(self) -> Core {
    let (bound, handlers) = (self.bound, self.on_change);
    let current: Arc<dyn Fn() -> Signature + Send + Sync> = match &bound {
      Some(signal) => {
        let signal = signal.clone();
        Arc::new(move || signal.get_untracked())
      }
      None => {
        let value = self.value.clone();
        Arc::new(move || value.clone())
      }
    };
    let change: Told = Arc::new(move |signature: &Signature| {
      if let Some(signal) = &bound {
        signal.set(signature.clone());
      }
      for handler in &handlers {
        handler(signature);
      }
    });
    let mut rules = self.rules;
    if let Some(width) = self.width {
      rules = rules.base(SignaturePadPart::Root, move |s| s.width(width));
    }
    if self.full_width {
      rules = rules.base(SignaturePadPart::Root, |s| s.width(Length::Fill));
    }
    if let Some(height) = self.height {
      rules = rules.base(SignaturePadPart::Root, move |s| s.height(height));
    }
    // The words it was not told: its own locale's, else the app's, read where it is built.
    let words = self.locale.unwrap_or_else(|| super::current_locale().input.clone());
    Core {
      value: self.value,
      current,
      change,
      label: self.label.unwrap_or(words.signature),
      placeholder: self.placeholder.unwrap_or(words.sign_above_the_line),
      undo_label: self.undo_label.unwrap_or(words.undo),
      clear_label: self.clear_label.unwrap_or(words.clear),
      signed_text: self.signed_text.unwrap_or(words.signed),
      empty_text: self.empty_text.unwrap_or(words.not_signed),
      read_only: self.read_only,
      disabled: self.disabled,
      invalid: self.invalid,
      states: self.states,
      rules,
      forced: self.forced,
      id: self.id,
    }
  }
}

/// A pad in a tree is its component, declared: the component that renders the tree mounts it.
impl From<SignaturePad> for Element {
  fn from(pad: SignaturePad) -> Self {
    let key = pad.key.clone();
    let props = SignaturePadProps(pad.core());
    match key {
      Some(key) => Element::component_keyed::<SignaturePadHost>(key, props),
      None => Element::component::<SignaturePadHost>(props),
    }
  }
}

/// A pad with the app's handlers made one.
#[derive(Clone)]
struct Core {
  value: Signature,
  /// The signature as it is now: the signal's, or what the app gave.
  current: Arc<dyn Fn() -> Signature + Send + Sync>,
  change: Told,
  label: Arc<str>,
  placeholder: Arc<str>,
  undo_label: Arc<str>,
  clear_label: Arc<str>,
  signed_text: Arc<str>,
  empty_text: Arc<str>,
  read_only: bool,
  disabled: bool,
  invalid: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
}

/// What a mounted pad keeps: the stroke being drawn, and the elements its pointer is read against.
#[derive(Clone)]
struct Live {
  draft: Arc<Mutex<Option<Draft>>>,
  /// Moved on by every change of the stroke being drawn: what renders the pad again.
  drawn: Signal<u64>,
  surface: ElementRef,
  root: ElementRef,
}

/// The stroke being drawn: its points in the pad's own pixels, when it began, and the finger that draws it (`None`
/// for the mouse).
struct Draft {
  points: Vec<SignaturePoint>,
  began: Instant,
  finger: Option<u64>,
}

impl Live {
  fn redraw(&self) {
    self.drawn.set(self.drawn.get_untracked().wrapping_add(1));
  }

  /// The point at `(x, y)` of the window, from the top left of the surface and kept on it.
  fn local(&self, x: f32, y: f32) -> (f32, f32) {
    let bounds = self.surface.bounds();
    (
      (x - bounds.x).clamp(0.0, bounds.width.max(0.0)),
      (y - bounds.y).clamp(0.0, bounds.height.max(0.0)),
    )
  }

  /// A stroke begins at `(x, y)` of the window, unless one is being drawn already.
  fn begin(&self, x: f32, y: f32, pressure: Option<f32>, finger: Option<u64>) -> bool {
    let mut draft = self.draft.lock().unwrap();
    if draft.is_some() {
      return false;
    }
    let (x, y) = self.local(x, y);
    *draft = Some(Draft {
      points: vec![SignaturePoint {
        x,
        y,
        pressure,
        time: 0,
      }],
      began: Instant::now(),
      finger,
    });
    drop(draft);
    self.redraw();
    true
  }

  /// The stroke of `finger` goes on to `(x, y)` of the window.
  fn extend(&self, x: f32, y: f32, pressure: Option<f32>, finger: Option<u64>) {
    let (x, y) = self.local(x, y);
    let mut held = self.draft.lock().unwrap();
    let Some(draft) = held.as_mut().filter(|draft| draft.finger == finger) else {
      return;
    };
    if let Some(last) = draft.points.last()
      && (last.x - x).hypot(last.y - y) < LEAST_STEP
    {
      return;
    }
    let time = u32::try_from(draft.began.elapsed().as_millis()).unwrap_or(u32::MAX);
    draft.points.push(SignaturePoint { x, y, pressure, time });
    drop(held);
    self.redraw();
  }

  /// The stroke of `finger` is let go: the stroke as the signature has it, in its own pixels, or `None` where no
  /// stroke of that finger is drawn.
  fn end(&self, finger: Option<u64>) -> Option<Vec<SignaturePoint>> {
    let mut held = self.draft.lock().unwrap();
    if held.as_ref().is_none_or(|draft| draft.finger != finger) {
      return None;
    }
    let draft = held.take()?;
    drop(held);
    self.redraw();
    Some(draft.points)
  }

  /// The stroke of `finger` is taken away, by the system: nothing is signed.
  fn cancel(&self, finger: Option<u64>) {
    let mut held = self.draft.lock().unwrap();
    if held.as_ref().is_some_and(|draft| draft.finger == finger) {
      *held = None;
      drop(held);
      self.redraw();
    }
  }
}

impl Core {
  fn build(self, theme: &Theme, live: Option<Live>) -> Element {
    let interactive = !self.read_only && !self.disabled;
    let draft: Vec<SignaturePoint> = live
      .as_ref()
      .and_then(|live| {
        let _ = live.drawn.get();
        live.draft.lock().unwrap().as_ref().map(|draft| draft.points.clone())
      })
      .unwrap_or_default();
    let signed = !self.value.is_empty();
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(SignaturePadState::Empty, !signed)
    .state(SignaturePadState::Signed, signed)
    .state(SignaturePadState::Drawing, !draft.is_empty())
    .state(SignaturePadState::Interactive, interactive)
    .state(SignaturePadState::ReadOnly, self.read_only)
    .state(SignaturePadState::Invalid, self.invalid)
    .state(SignaturePadState::Disabled, self.disabled);
    let recipe = theme.resolve::<SignaturePad>(&case, Some(&self.rules));
    let view = SignaturePadView {
      signature: self.value.clone(),
      draft,
      label: self.label.clone(),
      placeholder: self.placeholder.clone(),
      undo_label: self.undo_label.clone(),
      clear_label: self.clear_label.clone(),
      read_only: self.read_only,
      disabled: self.disabled,
      invalid: self.invalid,
      recipe,
      live: live.filter(|_| interactive),
      core: self,
    };
    match theme.renderer::<SignaturePad>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// A signature pad as a renderer sees it: what it holds and the stroke being drawn, the style of its parts, and its
/// behavior ready to be dressed.
pub struct SignaturePadView {
  /// The signature it holds.
  pub signature: Signature,
  /// The points of the stroke being drawn, in the pad's own pixels: none while no stroke is.
  pub draft: Vec<SignaturePoint>,
  pub label: Arc<str>,
  pub placeholder: Arc<str>,
  pub undo_label: Arc<str>,
  pub clear_label: Arc<str>,
  pub read_only: bool,
  pub disabled: bool,
  pub invalid: bool,
  /// The style of each part of this pad.
  pub recipe: ResolvedRecipe,
  live: Option<Live>,
  core: Core,
}

impl SignaturePadView {
  /// The style of a part, as this pad is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.core.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether it holds a signature of a stroke or more.
  pub fn signed(&self) -> bool {
    !self.signature.is_empty()
  }

  /// Whether a pointer signs on it and its keys and its buttons take strokes back: mounted, and neither read only
  /// nor disabled.
  pub fn interactive(&self) -> bool {
    self.live.is_some()
  }

  fn sub_id(&self, suffix: &str) -> Option<String> {
    self.core.id.as_ref().map(|id| format!("{id}-{suffix}"))
  }

  /// Tells the app the signature `next`.
  fn tell(&self) -> impl Fn(Signature) + Send + Sync + Clone + 'static {
    let change = self.core.change.clone();
    move |next: Signature| change(&next)
  }

  /// What takes back the last stroke, and what takes back every one.
  fn takers(
    &self,
  ) -> (
    impl Fn() + Send + Sync + Clone + 'static,
    impl Fn() + Send + Sync + Clone + 'static,
  ) {
    let (current, tell) = (self.core.current.clone(), self.tell());
    let undo = {
      let (current, tell) = (current.clone(), tell.clone());
      move || {
        let mut next = current();
        while next.strokes.last().is_some_and(|stroke| stroke.points.is_empty()) {
          next.strokes.pop();
        }
        if next.strokes.pop().is_some() {
          tell(next);
        }
      }
    };
    let clear = move || {
      let mut next = current();
      if !next.is_empty() {
        next.strokes.clear();
        tell(next);
      }
    };
    (undo, clear)
  }

  /// The pad's box in the style of its root, with nothing in it yet: what takes the focus and the keys, a picture to
  /// tooling called by its label and described as signed or not.
  pub fn root(&self) -> Stack {
    let said = if self.signed() {
      self.core.signed_text.clone()
    } else {
      self.core.empty_text.clone()
    };
    let mut root = Stack::new()
      .stack_align(StackAlignment::TopStart)
      .part(&self.part(SignaturePadPart::Root))
      .describe("a11y_role", "img")
      .describe("a11y_name", self.label.clone())
      .describe("a11y_description", said);
    if self.read_only {
      root = root.describe("a11y_readonly", "true");
    }
    if self.disabled {
      root = root.describe("a11y_disabled", "true");
    }
    if self.invalid {
      root = root.describe("a11y_invalid", "true");
    }
    if let Some(live) = &self.live {
      let (undo, clear) = self.takers();
      root = root
        .ref_element(live.root.clone())
        .tab_index(0)
        .on_key_down(move |event: KeyboardEvent| {
          if !event.focus_within || event.alt || event.composing {
            return;
          }
          let command = event.ctrl || event.meta;
          match event.key.as_str() {
            "z" | "Z" if command && !event.shift => undo(),
            "Backspace" if !command => undo(),
            "Delete" if !command => clear(),
            _ => return,
          }
          event.prevent_default();
        });
    }
    if let Some(id) = &self.core.id {
      root = root.id(id.clone());
    }
    root
  }

  /// The surface signed on, over the whole pad: a canvas that draws the signature and the stroke being drawn, and
  /// where it is signed on, takes the strokes of a mouse, a pen and a finger.
  pub fn surface(&self) -> Element {
    let theme = self.theme();
    let ink_part = self.part(SignaturePadPart::Ink);
    let (ink, line_width) = (color_of(&ink_part, theme), width_of(&ink_part, theme, 2.0));
    let (signature, draft) = (self.signature.clone(), self.draft.clone());
    // Drawn again only when what it shows changes: the strokes, the stroke being drawn, the ink.
    let key = paint_key(|hash| {
      for stroke in signature
        .strokes
        .iter()
        .chain(std::iter::once(&SignatureStroke::new(draft.iter().copied())))
      {
        hash.write_usize(stroke.points.len());
        for point in &stroke.points {
          hash.write_u32(point.x.to_bits());
          hash.write_u32(point.y.to_bits());
        }
      }
      hash.write_u32(signature.width.to_bits());
      hash.write_u32(signature.height.to_bits());
      hash.write(&[ink.r(), ink.g(), ink.b(), ink.a()]);
      hash.write_u32(line_width.to_bits());
    });
    let mut canvas = Canvas::new()
      .part(&self.part(SignaturePadPart::Surface))
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .paint(key, move |canvas| {
        let (draw, metrics) = (canvas.context_2d(), canvas.metrics());
        draw.reset();
        let (width, height) = (metrics.size.width, metrics.size.height);
        signature.paint(&draw, width, height, ink, line_width);
        let points: Vec<(f32, f32)> = draft.iter().map(|point| (point.x, point.y)).collect();
        trace(&draw, &points, ink, line_width);
      });
    if let Some(live) = self.live.clone() {
      let (current, tell) = (self.core.current.clone(), self.tell());
      // A stroke let go joins the signature: in the signature's own pixels, which are the pad's unless it was signed on
      // a pad of another size.
      let finish = move |live: &Live, points: Vec<SignaturePoint>| {
        let bounds = live.surface.bounds();
        let mut next = current();
        if next.is_empty() {
          next.width = bounds.width;
          next.height = bounds.height;
        }
        let scale = next.scale_to(bounds.width, bounds.height);
        let points = points.into_iter().map(|point| SignaturePoint {
          x: point.x / scale,
          y: point.y / scale,
          ..point
        });
        next.strokes.push(SignatureStroke::new(points));
        tell(next);
      };
      let finish_touch = finish.clone();
      let (on_start, on_move, on_end) = (live.clone(), live.clone(), live.clone());
      let (touch_start, touch_move, touch_end, touch_cancel) = (live.clone(), live.clone(), live.clone(), live.clone());
      canvas = canvas
        .ref_element(live.surface.clone())
        .focuses(&live.root)
        .start_drag_buttons(MouseButtonMask::LEFT)
        .on_drag_start(move |event: DragEvent| {
          on_start.begin(event.x, event.y, None, None);
        })
        .on_drag_move(move |event: DragEvent| on_move.extend(event.x, event.y, None, None))
        .on_drag_end(move |_: DragEvent| {
          if let Some(points) = on_end.end(None) {
            finish(&on_end, points);
          }
        })
        // A finger or a pen on the pad draws, and is taken from what would scroll or click with it.
        .on_touch_start(move |event: TouchEvent| {
          if touch_start.begin(event.x, event.y, event.force, Some(event.id)) {
            event.prevent_default();
          }
        })
        .on_touch_move(move |event: TouchEvent| touch_move.extend(event.x, event.y, event.force, Some(event.id)))
        .on_touch_end(move |event: TouchEvent| {
          if let Some(points) = touch_end.end(Some(event.id)) {
            finish_touch(&touch_end, points);
          }
        })
        .on_touch_cancel(move |event: TouchEvent| touch_cancel.cancel(Some(event.id)));
    }
    if let Some(id) = self.sub_id("surface") {
      canvas = canvas.id(id);
    }
    canvas.into()
  }

  /// The line signed on and the words under it, at the bottom of the pad.
  pub fn guide(&self) -> Element {
    let placeholder = self.part(SignaturePadPart::Placeholder);
    Column::new()
      .part(&self.part(SignaturePadPart::Guide))
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .justify(Justify::End)
      .child(
        Row::new()
          .part(&self.part(SignaturePadPart::Baseline))
          .width(Dimension::Pct(100.0)),
      )
      .child(
        Row::new()
          .width(Dimension::Pct(100.0))
          .min_width(0.0)
          .child(one_line(&placeholder, &self.placeholder)),
      )
      .into()
  }

  /// Undo and Clear, in the pad's top corner: where it is signed on and signed. A press of Clear, or of Undo that
  /// takes back the last stroke, gives the focus to the pad, as the buttons are gone after it.
  pub fn actions(&self) -> Option<Element> {
    let live = self.live.as_ref().filter(|_| self.signed())?;
    let (undo, clear) = self.takers();
    let strokes = self
      .signature
      .strokes
      .iter()
      .filter(|stroke| !stroke.points.is_empty())
      .count();
    let mut undo = Button::new(&self.undo_label)
      .variant(ButtonVariant::Ghost)
      .size(ButtonSize::Xs)
      .icon(icons::UNDO)
      .icon_only(true)
      .on_click(move |_| undo());
    if strokes <= 1 {
      undo = undo.focuses(&live.root);
    }
    let mut clear = Button::new(&self.clear_label)
      .variant(ButtonVariant::Ghost)
      .size(ButtonSize::Xs)
      .icon(icons::TRASH)
      .icon_only(true)
      .focuses(&live.root)
      .on_click(move |_| clear());
    if let Some(id) = self.sub_id("undo") {
      undo = undo.id(id);
    }
    if let Some(id) = self.sub_id("clear") {
      clear = clear.id(id);
    }
    let theme = self.theme();
    Some(
      Row::new()
        .part(&self.part(SignaturePadPart::Actions))
        .width(Dimension::Pct(100.0))
        .justify(Justify::End)
        .align_items(Alignment::Center)
        // The row lies over the surface: only its buttons take the pointer, and the rest of it is signed on.
        .hit_test(crate::node::HitTestBehavior::ContentOnly)
        .child(undo.build(theme))
        .child(clear.build(theme))
        .into(),
    )
  }

  /// How the default renderer builds a pad: the line and its words, the surface over them, and the buttons over that.
  pub fn render(self) -> Element {
    let mut root = self.root().child(self.guide()).child(self.surface());
    if let Some(actions) = self.actions() {
      root = root.child(actions);
    }
    root.into()
  }
}

/// What a mounted pad is given: the pad, built anew by every render of what holds it.
#[derive(Clone)]
struct SignaturePadProps(Core);

impl PartialEq for SignaturePadProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for SignaturePadProps {}

/// The component of a pad: the stroke being drawn.
struct SignaturePadHost {
  live: Live,
}

impl Component for SignaturePadHost {
  type Props = SignaturePadProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      live: Live {
        draft: Arc::default(),
        drawn: ctx.signal(0),
        surface: ElementRef::new(),
        root: ElementRef::new(),
      },
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let core = ctx.props::<SignaturePadProps>().0.clone();
    core.build(&current_theme(), Some(self.live.clone()))
  }
}

impl Themed for SignaturePad {
  const NAME: &'static str = "signature_pad";
  type Renderer = SignaturePadRenderer;

  /// The design's signature pad (`pencil/lurq-design-system.pen`, Signature Pad, in Advanced Inputs): 280 by 124 on
  /// the background, its corners large, an edge of 1 in the input's color inside it. At its bottom, inside 12 and 16,
  /// a line of 1 in the input's color and 6 under it "Sign above the line" in the mono type of 11 in the muted
  /// color; the ink is a line of 2 in the foreground's color, round at its ends. The board draws it signed and no
  /// other state: the ring of the focus is around the pad, Undo and Clear are ghost buttons of the smallest size in
  /// its top corner, 6 in, while it is signed; an invalid pad has its edge in the destructive color, and a disabled
  /// one is dimmed. The pointer is a cross over a pad that is signed on.
  fn recipe() -> Recipe {
    use PaletteColor::{Background, Destructive, Foreground, Input, MutedForeground};
    use SignaturePadPart::{Actions, Baseline, Guide, Ink, Placeholder, Root, Surface};
    use SignaturePadState::{Disabled, Interactive, Invalid};
    Recipe::new()
      .base(Root, |s| {
        fading(
          s.width(280.0)
            .max_width(Length::Fill)
            .height(124.0)
            .background(Background)
            .radius(RadiusSize::Lg)
            .border(Border::inside(BorderSize::Sm, Input)),
        )
      })
      .base(Guide, |s| s.padding_y(12.0).padding_x(16.0).gap(6.0))
      .base(Baseline, |s| s.height(1.0).background(Input))
      .base(Placeholder, |s| {
        s.font_family(FontFamily::Mono)
          .font_size(11.0)
          .foreground(MutedForeground)
      })
      .base(Ink, |s| s.width(2.0).background(Foreground))
      .base(Actions, |s| s.padding_x(6.0).padding_y(6.0).gap(2.0))
      .focus_visible(Root, |s| s.focus_ring())
      .state(Interactive, Surface, |s| s.cursor(CursorIcon::Crosshair))
      .state(Invalid, Root, |s| s.border(Border::inside(BorderSize::Sm, Destructive)))
      .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
  }
}

impl super::FieldControl for SignaturePad {
  /// Called by the field's label unless it has a name of its own, invalid where the field is, and disabled where
  /// the field is.
  fn in_field(mut self, field: &super::InField) -> Element {
    if self.label.is_none() {
      self.label = Some(field.label.clone());
    }
    self.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.disabled |= field.disabled;
    self.into()
  }

  /// Told what the field says, as in `in_field`, and mounted: the same pad as outside a field.
  fn mount_in_field(mut self, field: &super::InField, ctx: &mut Ctx) -> Element {
    if self.label.is_none() {
      self.label = Some(field.label.clone());
    }
    self.invalid |= field.judgment == Some(super::Judgment::Invalid);
    self.disabled |= field.disabled;
    self.mount(ctx)
  }
}
