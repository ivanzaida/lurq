//! A volume control: a button that mutes, and a slider of how loud.

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use super::{
  Button, ButtonPart, ButtonVariant, Slider, SliderPart, icons, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, MouseEvent},
    theme::Theme,
  },
  components::Row,
  core::Signal,
  layout::Alignment,
  node::{Element, Node, NodeUpdate, Outline, border::Border, color::Color},
};

/// How large a volume control is: a button of 32 with an icon of 16, or of 36 with 20 in a video's bar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MediaVolumeSize {
  #[default]
  Md,
  Lg,
}

names!(MediaVolumeSize {
  Md => "md",
  Lg => "lg",
});

/// The parts of a volume control a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MediaVolumePart {
  /// The button and the slider, side by side.
  Root,
  /// The ghost button that mutes: its rules are laid over a [`Button`]'s root.
  Button,
  /// The glyph of the button: how loud, or crossed out. Laid over the button's icon.
  Icon,
  /// The slider of how loud: laid over a [`Slider`]'s root.
  Slider,
  /// The rail of the slider, laid over its track.
  Track,
  /// The part of the rail up to the volume, laid over the slider's fill.
  Fill,
  /// What is dragged, laid over the slider's thumb.
  Thumb,
}

names!(MediaVolumePart {
  Root => "root",
  Button => "button",
  Icon => "icon",
  Slider => "slider",
  Track => "track",
  Fill => "fill",
  Thumb => "thumb",
});

/// The states of a volume control a recipe styles, besides what the pointer and the keyboard do to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MediaVolumeState {
  /// The sound is off: the glyph is crossed and the slider stands at nothing.
  Muted,
  /// It is the button alone until the pointer or the focus is in it.
  Collapsed,
  /// It stands over a picture: white, the same in a light and a dark theme.
  OnMedia,
  /// It takes no input.
  Disabled,
}

names!(MediaVolumeState {
  Muted => "muted",
  Collapsed => "collapsed",
  OnMedia => "on_media",
  Disabled => "disabled",
});

/// What builds a volume control's nodes: a theme's in place of the default one.
pub type MediaVolumeRenderer = Arc<dyn Fn(MediaVolumeView) -> Element + Send + Sync>;

type Handler<T> = Arc<dyn Fn(T) + Send + Sync>;

/// A volume control.
///
/// ```
/// use lurq::{core::Signal, ui::MediaVolume};
///
/// let (volume, muted) = (Signal::new(0.6_f32), Signal::new(false));
/// let control = MediaVolume::new(volume).muted(muted).on_change(|volume| println!("{volume}"));
/// // The app holds both and is told what a person makes them.
/// let held = MediaVolume::controlled(0.6, |volume| println!("{volume}")).muted_controlled(false, |muted| println!("{muted}"));
/// ```
///
/// The volume runs from 0 to 1. Muting keeps it, so that the sound comes back as loud as it was: muted, the slider
/// stands at nothing and the app's volume is not touched. Dragging the slider up from nothing while muted turns the
/// sound back on, and tells [`MediaVolume::on_muted_change`] so as well as the volume. The glyph is the loud one
/// from a half up, the low one under it, and the crossed one while muted or at nothing.
///
/// Collapsed it is the button alone, and the slider comes while the pointer is over it or the focus is in it. What
/// that needs between renders is kept by [`MediaVolume::mount`]. A control given to a tree as it is
/// (`.child(MediaVolume::new(..))`) is the same component, declared: what renders the tree mounts it.
/// [`MediaVolume::build`] is a still picture of it, for a test.
#[derive(Clone)]
pub struct MediaVolume {
  /// The app's volume, where it is bound to a signal.
  volume: Option<Signal<f32>>,
  /// The app's volume, where it holds it.
  controlled: Option<f64>,
  muted: bool,
  on_change: Vec<Handler<f64>>,
  on_change_end: Vec<Handler<f64>>,
  on_muted_change: Vec<Handler<bool>>,
  collapsed: bool,
  on_media: bool,
  size: Name,
  length: Option<f32>,
  label: Arc<str>,
  mute_label: Arc<str>,
  unmute_label: Arc<str>,
  disabled: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  /// What a mounted control keeps of the pointer and the focus.
  kept: Option<Kept>,
}

impl MediaVolume {
  /// A volume control bound to `volume`, from 0 to 1. What a person does sets the signal.
  pub fn new(volume: Signal<f32>) -> Self {
    Self {
      volume: Some(volume),
      controlled: None,
      muted: false,
      on_change: Vec::new(),
      on_change_end: Vec::new(),
      on_muted_change: Vec::new(),
      collapsed: false,
      on_media: false,
      size: MediaVolumeSize::Md.into(),
      length: None,
      label: Arc::from("Volume"),
      mute_label: Arc::from("Mute"),
      unmute_label: Arc::from("Unmute"),
      disabled: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
      kept: None,
    }
  }

  /// A volume control that shows `volume`, held by the app, and tells `on_change` what a person makes it.
  pub fn controlled(volume: impl Into<f64>, on_change: impl Fn(f64) + Send + Sync + 'static) -> Self {
    let mut control = Self::new(Signal::new(0.0)).on_change(on_change);
    control.volume = None;
    control.controlled = Some(volume.into());
    control
  }

  /// Muted while `muted` is true, which a press of the button turns, and which dragging the slider up from nothing
  /// turns off.
  pub fn muted(mut self, muted: Signal<bool>) -> Self {
    self.muted = muted.get();
    self.on_muted_change.insert(0, Arc::new(move |next| muted.set(next)));
    self
  }

  /// Muted while `muted` is true, held by the app, which is told what a press makes it.
  pub fn muted_controlled(mut self, muted: bool, on_muted_change: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.muted = muted;
    self.on_muted_change(on_muted_change)
  }

  /// Called with the volume each time a person moves it. Bound, after the signal is set. A second handler is called
  /// after the first.
  pub fn on_change(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Called with the volume when a drag is let go or a key's step is made. A second handler is called after the
  /// first.
  pub fn on_change_end(mut self, handler: impl Fn(f64) + Send + Sync + 'static) -> Self {
    self.on_change_end.push(Arc::new(handler));
    self
  }

  /// Called with whether it is to be muted: on a press of the button, and with `false` when the slider is dragged up
  /// from nothing while muted. A second handler is called after the first.
  pub fn on_muted_change(mut self, handler: impl Fn(bool) + Send + Sync + 'static) -> Self {
    self.on_muted_change.push(Arc::new(handler));
    self
  }

  /// Whether it is the button alone, the slider coming under the pointer and with the focus: where a bar has no
  /// room for the slider. The slider comes and goes only where the control is [mounted](Self::mount).
  pub fn collapsed(mut self, collapsed: bool) -> Self {
    self.collapsed = collapsed;
    self
  }

  /// Whether it stands over a picture: white, the same in a light and a dark theme.
  pub fn on_media(mut self, on_media: bool) -> Self {
    self.on_media = on_media;
    self
  }

  /// Its size, a [`MediaVolumeSize`] or one the theme's recipe names.
  pub fn size(mut self, size: impl Into<Name>) -> Self {
    self.size = size.into();
    self
  }

  /// How long the slider is, in pixels, in place of the recipe's 72.
  pub fn length(mut self, length: f32) -> Self {
    self.length = Some(length);
    self
  }

  /// What the slider is called, to what reads the tree: "Volume" unless told.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Arc::from(label);
    self
  }

  /// What the button is called while a press would mute: "Mute" unless told.
  pub fn mute_label(mut self, label: &str) -> Self {
    self.mute_label = Arc::from(label);
    self
  }

  /// What the button is called while a press would bring the sound back: "Unmute" unless told.
  pub fn unmute_label(mut self, label: &str) -> Self {
    self.unmute_label = Arc::from(label);
    self
  }

  /// Whether it takes no input: the button and the slider are dimmed, and Tab passes them by.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Puts the control in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this control over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this control: what the component it is in says of it in this place.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws its button as it is under the pointer, held down or focused from the keyboard, with nothing pointing
  /// at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the button.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// What tells it apart from those beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Puts the control in the tree of `ctx`'s component, in the theme of what is being rendered: as
  /// [`build`](Self::build) does, and it keeps whether the pointer is over it, whether the focus is in it and
  /// whether its slider is being dragged, so that a collapsed control shows its slider while any of them is so.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let mut element = ctx.mount::<MediaVolumeHost>(MediaVolumeProps { control: self });
    if let Some(key) = key {
      element = element.key(key);
    }
    element
  }

  /// The control with what a component that holds it keeps for it between renders, in place of the control's
  /// own mount: built with it, a collapsed control shows its slider as a mounted one does.
  pub(super) fn kept(mut self, kept: Kept) -> Self {
    self.kept = Some(kept);
    self
  }

  /// Builds a still picture of the volume control in `theme`, for a test or a screenshot: collapsed, it is the
  /// button alone. An app gives the control to its tree, or mounts it ([`MediaVolume::mount`]).
  pub fn build(self, theme: &Theme) -> Element {
    self.build_with(theme, &mut |button| button)
  }

  /// Builds the control with its button given to `dress` first: what holds the control hangs a tooltip of its own
  /// from the button alone, where one around the whole control would show over the slider too.
  pub(super) fn build_with(self, theme: &Theme, dress: &mut dyn FnMut(Element) -> Element) -> Element {
    let volume = match (&self.volume, self.controlled) {
      (_, Some(held)) => held,
      (Some(volume), None) => f64::from(volume.get()),
      (None, None) => 0.0,
    };
    let case = Case {
      variant: None,
      size: Some(self.size),
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(MediaVolumeState::Muted, self.muted)
    .state(MediaVolumeState::Collapsed, self.collapsed)
    .state(MediaVolumeState::OnMedia, self.on_media)
    .state(MediaVolumeState::Disabled, self.disabled);
    let revealed = !self.collapsed || (!self.disabled && self.kept.as_ref().is_some_and(Kept::asks));
    let view = MediaVolumeView {
      volume,
      muted: self.muted,
      collapsed: self.collapsed,
      revealed,
      on_media: self.on_media,
      size: self.size,
      length: self.length,
      label: self.label,
      mute_label: self.mute_label,
      unmute_label: self.unmute_label,
      disabled: self.disabled,
      recipe: theme.resolve::<MediaVolume>(&case, Some(&self.rules)),
      bound: self.volume.filter(|_| self.controlled.is_none()),
      on_change: self.on_change,
      on_change_end: self.on_change_end,
      on_muted_change: self.on_muted_change,
      forced: self.forced,
      id: self.id,
      key: self.key,
      kept: self.kept,
    };
    match theme.renderer::<MediaVolume>() {
      Some(renderer) => renderer(view),
      None => view.render_with(dress),
    }
  }
}

/// What a mounted control keeps between renders: whether the pointer is over it, whether the focus is on its button
/// or its slider, and whether its slider is being dragged.
#[derive(Clone)]
pub(super) struct Kept {
  hovered: Signal<bool>,
  focused: Signal<bool>,
  dragging: Signal<bool>,
  /// Whether the focus came with a press of the pointer: it is over the control then, and the focus it leaves
  /// behind asks for nothing once it has gone. A key pressed in the control makes the focus the keyboard's.
  pointed: Signal<bool>,
  /// Whether the pointer is held down in the control: what it changes then is a drag. A slider the app holds and
  /// has not answered yet tells a change again after its end, which is no drag.
  pressed: Arc<AtomicBool>,
}

impl Kept {
  /// What a component that holds a control keeps for it, in place of the control's own mount: a player, which
  /// lays its bar out by whether the slider is out.
  pub(super) fn new(ctx: &mut Ctx) -> Self {
    Self {
      hovered: ctx.signal(false),
      focused: ctx.signal(false),
      dragging: ctx.signal(false),
      pointed: ctx.signal(false),
      pressed: Arc::default(),
    }
  }

  /// Whether any of them asks for the slider: the pointer over the control, its slider dragged, or the focus the
  /// keyboard brought to it. Read in a render, it renders again when one changes.
  pub(super) fn asks(&self) -> bool {
    self.hovered.get() || self.dragging.get() || (self.focused.get() && !self.pointed.get())
  }
}

/// Sets `signal` to `value` where it holds something else.
fn put(signal: &Signal<bool>, value: bool) {
  if signal.get_untracked() != value {
    signal.set(value);
  }
}

/// Has each stop of Tab in `node` say in `kept` whether it has the focus, and whether the pointer brought it: a
/// press focuses what is under the pointer, which is over the control then.
fn watch_focus(node: &mut Node, kept: &Kept) {
  if node.tab_index_value().is_some_and(|index| index >= 0) && node.is_focusable() {
    let (gained, lost) = (kept.clone(), kept.focused.clone());
    NodeUpdate::on_focus(node, move || {
      put(&gained.pointed, gained.hovered.get_untracked());
      put(&gained.focused, true);
    });
    NodeUpdate::on_blur(node, move || put(&lost, false));
  }
  for child in &mut node.children {
    watch_focus(child, kept);
  }
}

/// A volume control as a renderer sees it: what it was given, the style of its parts, and its behavior ready to be
/// dressed.
pub struct MediaVolumeView {
  /// The app's volume, from 0 to 1: kept while muted.
  pub volume: f64,
  pub muted: bool,
  pub collapsed: bool,
  /// Whether the slider is shown: always, unless it is collapsed and nothing asks for it.
  pub revealed: bool,
  pub on_media: bool,
  pub size: Name,
  /// The length of the slider it was given; `None` for the recipe's.
  pub length: Option<f32>,
  pub label: Arc<str>,
  pub mute_label: Arc<str>,
  pub unmute_label: Arc<str>,
  pub disabled: bool,
  /// The style of each part of this control.
  pub recipe: ResolvedRecipe,
  bound: Option<Signal<f32>>,
  on_change: Vec<Handler<f64>>,
  on_change_end: Vec<Handler<f64>>,
  on_muted_change: Vec<Handler<bool>>,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  kept: Option<Kept>,
}

impl MediaVolumeView {
  /// The style of a part, as this control is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether there is no sound: muted, or at nothing.
  pub fn silent(&self) -> bool {
    self.muted || self.volume <= 0.0
  }

  /// Where the slider stands: at nothing while muted, at the volume otherwise.
  pub fn shown_volume(&self) -> f64 {
    if self.muted { 0.0 } else { self.volume.clamp(0.0, 1.0) }
  }

  /// The glyph's name: crossed while silent, low under a half, loud from a half up.
  pub fn icon_name(&self) -> &'static str {
    if self.silent() {
      icons::VOLUME_OFF
    } else if self.volume < 0.5 {
      icons::VOLUME_LOW
    } else {
      icons::VOLUME
    }
  }

  /// The ghost button that mutes, dressed by this control's recipe over the theme's for a button. It is called by
  /// what a press would do, and a press tells every handler of `on_muted_change` the other state.
  pub fn button(&self) -> Button {
    let rules = self
      .recipe
      .rules_for(MediaVolumePart::Button, ButtonPart::Root)
      .extend(&self.recipe.rules_for(MediaVolumePart::Icon, ButtonPart::Icon));
    let label = if self.muted {
      &self.unmute_label
    } else {
      &self.mute_label
    };
    let mut button = Button::new(label)
      .icon(self.icon_name())
      .icon_only(true)
      .variant(ButtonVariant::Ghost)
      .disabled(self.disabled)
      .force(self.forced)
      .rules(&rules);
    if !self.on_muted_change.is_empty() {
      let (handlers, next) = (self.on_muted_change.clone(), !self.muted);
      button = button.on_click(move |_| {
        for handler in &handlers {
          handler(next);
        }
      });
    }
    if let Some(id) = &self.id {
      button = button.id(id.clone());
    }
    button
  }

  /// The slider of how loud, dressed by this control's recipe over the theme's for a slider. It shows
  /// [`MediaVolumeView::shown_volume`]; what a person does sets the app's signal where it is bound, tells every
  /// handler of `on_change`, and, dragged up from nothing while muted, tells `on_muted_change` that the sound is
  /// back. A mounted control keeps it shown while it is being dragged.
  pub fn slider(&self) -> Slider {
    let rules = self
      .recipe
      .rules_for(MediaVolumePart::Slider, SliderPart::Root)
      .extend(&self.recipe.rules_for(MediaVolumePart::Track, SliderPart::Track))
      .extend(&self.recipe.rules_for(MediaVolumePart::Fill, SliderPart::Fill))
      .extend(&self.recipe.rules_for(MediaVolumePart::Thumb, SliderPart::Thumb));
    let (bound, changed, muting) = (self.bound.clone(), self.on_change.clone(), self.on_muted_change.clone());
    // Told once in a gesture: the handlers of a drag hear many values before the app renders the control unmuted.
    let unmute = self.muted.then(|| Arc::new(AtomicBool::new(false)));
    let dragging = self.kept.as_ref().map(|kept| kept.dragging.clone());
    let held = self.kept.clone();
    let mut slider = Slider::controlled(self.shown_volume() as f32, move |to| {
      if let Some(held) = &held
        && held.pressed.load(Ordering::Relaxed)
      {
        put(&held.dragging, true);
      }
      if let Some(bound) = &bound {
        bound.set(to as f32);
      }
      for handler in &changed {
        handler(to);
      }
      if to > 0.0
        && let Some(told) = &unmute
        && !told.swap(true, Ordering::Relaxed)
      {
        for handler in &muting {
          handler(false);
        }
      }
    })
    .max(1.0)
    .step(0.05)
    .label(&self.label)
    .disabled(self.disabled)
    .rules(&rules);
    let ended = self.on_change_end.clone();
    let let_go = self.kept.as_ref().map(|kept| kept.pressed.clone());
    slider = slider.on_change_end(move |to| {
      if let Some(let_go) = &let_go {
        let_go.store(false, Ordering::Relaxed);
      }
      if let Some(dragging) = &dragging {
        put(dragging, false);
      }
      for handler in &ended {
        handler(to);
      }
    });
    if let Some(length) = self.length {
      slider = slider.length(length);
    }
    slider
  }

  /// How the default renderer builds a volume control: the button, and the slider beside it where it is shown. A
  /// mounted control hears the pointer come and go over the whole of it, and the focus come to and leave its button
  /// and its slider.
  pub fn render(self) -> Element {
    self.render_with(&mut |button| button)
  }

  /// [`render`](Self::render), with the button given to `dress` before it is put in the control.
  fn render_with(self, dress: &mut dyn FnMut(Element) -> Element) -> Element {
    let theme = self.theme().clone();
    let mut root = Row::new()
      .align_items(Alignment::Center)
      .part(&self.part(MediaVolumePart::Root))
      .child(dress(self.button().build(&theme)));
    if self.revealed {
      root = root.child(self.slider().build(&theme));
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    let Some(kept) = &self.kept else {
      return root.into();
    };
    let (enter, leave, keyed) = (kept.hovered.clone(), kept.hovered.clone(), kept.pointed.clone());
    let (down, up, gone) = (kept.pressed.clone(), kept.pressed.clone(), kept.clone());
    let mut element: Element = root
      .on_mouse_enter(move || put(&enter, true))
      // While the slider is dragged the runtime gives it every move of the pointer and tells no one else, so the
      // pointer is not said to leave during a drag: when it is, nothing is held any more. A slider the app holds
      // and does not answer is let go without its end being told, and would stay out.
      .on_mouse_leave(move || {
        gone.pressed.store(false, Ordering::Relaxed);
        put(&gone.dragging, false);
        put(&leave, false);
      })
      .on_mouse_down(move |_: MouseEvent| down.store(true, Ordering::Relaxed))
      .on_mouse_up(move |_: MouseEvent| up.store(false, Ordering::Relaxed))
      // A key pressed in it: the focus is the keyboard's from here on, whatever brought it.
      .on_key_down(move |event: KeyboardEvent| {
        if event.focus_within {
          put(&keyed, false);
        }
      })
      .into();
    watch_focus(&mut element.node, kept);
    element
  }
}

impl Themed for MediaVolume {
  const NAME: &'static str = "media_volume";
  type Renderer = MediaVolumeRenderer;

  /// The design's volume control (`pencil/lurq-design-system.pen`, `Media/Volume`, `hRRVr`): a ghost button of 32
  /// with a glyph of 16 and a slider 72 long, 4 apart; in a video's bar the button is 36 with a glyph of 20. Over a
  /// picture (the video player's bar) it is white, the same in a light and a dark theme: the glyph white, the button
  /// white at 12% under the pointer and at 20% held down, the ring of the focus white; the slider's fill and thumb
  /// white, the thumb 12 with no edge, the rail white at 20%.
  fn recipe() -> Recipe {
    use MediaVolumePart::{Button, Fill, Icon, Root, Slider, Thumb, Track};
    use MediaVolumeState::OnMedia;
    let (white, clear) = (Color::from_hex("#FFFFFF"), Color::from_hex("#00000000"));
    let on_media = || Selector::new().state(OnMedia);
    Recipe::new()
      .base(Root, |s| s.gap(4.0))
      .base(Button, |s| s.square(32.0))
      .base(Icon, |s| s.square(16.0))
      .base(Slider, |s| s.width(72.0))
      .size(MediaVolumeSize::Lg, Button, |s| s.square(36.0))
      .size(MediaVolumeSize::Lg, Icon, |s| s.square(20.0))
      .state(OnMedia, Button, |s| s.foreground(white))
      .when(on_media().hovered(), Button, |s| {
        s.background(Color::from_hex("#FFFFFF1F")).foreground(white)
      })
      // The button's own pressed veil would darken white at 20% to grey: the board has white at 20% alone.
      .when(on_media().pressed(), Button, |s| {
        s.background(Color::from_hex("#FFFFFF33")).veil(clear).foreground(white)
      })
      .when(on_media().focus_visible(), Button, |s| {
        s.outline(Outline::new(2.0, white).offset(2.0))
      })
      .state(OnMedia, Track, |s| s.background(Color::from_hex("#FFFFFF33")))
      .state(OnMedia, Fill, |s| s.background(white))
      .state(OnMedia, Thumb, |s| {
        s.square(12.0).background(white).border(Border::inside(0.0, clear))
      })
      .when(on_media().focus_visible(), Thumb, |s| {
        s.outline(Outline::new(2.0, white).offset(2.0))
      })
  }
}

/// A control in a tree is its component, declared: the component that renders the tree mounts it.
impl crate::app::component::IntoComponent for MediaVolume {
  type Component = MediaVolumeHost;

  fn into_props(self) -> MediaVolumeProps {
    MediaVolumeProps { control: self }
  }

  fn component_key(&self) -> Option<Arc<str>> {
    self.key.clone()
  }
}

/// A volume control to mount.
#[doc(hidden)]
#[derive(Clone)]
pub struct MediaVolumeProps {
  control: MediaVolume,
}

/// The control is made anew by every render of what holds it, so props are never the same as before.
impl PartialEq for MediaVolumeProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for MediaVolumeProps {}

/// A mounted volume control: it keeps whether the pointer is over it, whether the focus is in it, and whether its
/// slider is being dragged.
#[doc(hidden)]
pub struct MediaVolumeHost {
  kept: Kept,
}

impl Component for MediaVolumeHost {
  type Props = MediaVolumeProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self { kept: Kept::new(ctx) }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let MediaVolumeProps { mut control } = ctx.props::<MediaVolumeProps>().clone();
    control.kept = Some(self.kept.clone());
    control.build(&current_theme())
  }
}
