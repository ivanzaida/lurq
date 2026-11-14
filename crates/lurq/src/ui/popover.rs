//! A popover: a panel beside the element that opens it, over the window, holding anything.

use std::sync::Arc;

use super::{
  names,
  part_style::{Length, PartAnimation, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{FADE_IN, FADE_OUT},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::{CollisionStrategy, Placement},
    theme::{
      BorderSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, ShadowStyle, SpacingSize, Theme,
      TypographyStyle,
    },
  },
  components::{Column, Popup, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, text_style::FontWeight},
  node::{Element, NodeUpdate, SpacingValue, border::Border},
};

/// The parts of a popover a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PopoverPart {
  /// The panel: its fill, its edge, its shadow, and its header over its body.
  Root,
  /// The title over the description.
  Header,
  /// What the popover is about, in a line.
  Title,
  /// What it is for, under the title.
  Description,
  /// What the popover holds, one thing under another.
  Body,
}

names!(PopoverPart {
  Root => "root",
  Header => "header",
  Title => "title",
  Description => "description",
  Body => "body",
});

/// What builds a popover's panel: a theme's in place of the default one.
/// Where the panel is shown, and when, stays the popover's.
pub type PopoverRenderer = Arc<dyn Fn(PopoverView) -> Element + Send + Sync>;

/// A popover: a panel shown beside the element that opens it, over the
/// window, holding anything: the size of a layer, a few settings, a form.
///
/// ```
/// use lurq::{
///   components::{Placement, Row},
///   core::{ElementRef, Signal},
///   ui::{Button, Field, Input, Popover},
/// };
///
/// // What the app keeps from one render to the next: the anchor, and whether it is open.
/// let (anchor, open) = (ElementRef::new(), Signal::new(false));
///
/// // The button opens and closes it, and it is shown under the button.
/// let popover = Popover::new(anchor.clone(), open.clone())
///   .trigger(Button::new("Dimensions"))
///   .title("Dimensions")
///   .description("Set the size of the layer.")
///   .placement(Placement::BottomStart)
///   .child(Field::new("Width", Input::new(Signal::new("320px".to_owned())).full_width(true)).full_width(true));
///
/// // Beside an element of the app's, which opens it in its own way.
/// let (anchor, open) = (ElementRef::new(), Signal::new(false));
/// let row = Row::new()
///   .child(Row::new().ref_element(anchor.clone()))
///   .child(Popover::new(anchor, open).title("Details"));
/// ```
///
/// A popover does not keep whether it is open: the app's signal does. A
/// press outside it or Escape closes it, which sets the signal to `false`.
/// Its trigger ([`Popover::trigger`]) turns the signal; without one the app
/// opens it, and the anchor is any element of its own. A press of the
/// anchor is not a press outside it. The anchor is kept from one render to
/// the next (a field of the component, `ctx.element_ref()`): one made anew in
/// each render is not the one the focus goes back to as it closes.
///
/// As it opens the focus goes to the first thing in it that takes the focus,
/// and as it closes with the focus inside, back to the anchor. Where there is
/// no room for it on its side it goes to the other, and it is kept in the
/// window.
pub struct Popover {
  anchor: ElementRef,
  open: Signal<bool>,
  trigger: Option<Element>,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  children: Vec<Element>,
  placement: Placement,
  offset: SpacingValue,
  collision: CollisionStrategy,
  width: Option<Length>,
  alert: bool,
  name: Option<Arc<str>>,
  described: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Popover {
  /// A popover shown beside `anchor` while `open` is `true`.
  pub fn new(anchor: impl Into<ElementRef>, open: Signal<bool>) -> Self {
    Self {
      anchor: anchor.into(),
      open,
      trigger: None,
      title: None,
      description: None,
      children: Vec::new(),
      placement: Placement::BottomStart,
      offset: SpacingSize::Xs.into(),
      collision: CollisionStrategy::FlipThenClamp,
      width: None,
      alert: false,
      name: None,
      described: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// What opens and closes the popover: a button. It is put before the
  /// popover, is its anchor, and a press of it turns the signal. While
  /// another popup is open a press of it closes that one and opens this one.
  /// It says to what reads the tree whether the popover is open.
  pub fn trigger(mut self, trigger: impl Into<Element>) -> Self {
    self.trigger = Some(trigger.into());
    self
  }

  /// What the popover is about, at the top of it.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// What it is for, under the title.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// Adds something the popover holds, under what it holds already.
  pub fn child(mut self, child: impl Into<Element>) -> Self {
    self.children.push(child.into());
    self
  }

  /// Adds several things the popover holds.
  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
    self.children.extend(children.into_iter().map(Into::into));
    self
  }

  /// Which side of the anchor the popover is on, and which edge of it it
  /// lines up with. Under the anchor at its start by default.
  pub fn placement(mut self, placement: Placement) -> Self {
    self.placement = placement;
    self
  }

  /// How far the popover is from its anchor: the theme's smallest spacing by default.
  pub fn offset(mut self, offset: impl Into<SpacingValue>) -> Self {
    self.offset = offset.into();
    self
  }

  /// What the popover does where there is no room for it on its side: goes
  /// to the other side, then is moved into the window, by default.
  pub fn collision(mut self, collision: CollisionStrategy) -> Self {
    self.collision = collision;
    self
  }

  /// How wide the panel is, in place of what the theme gives it.
  pub fn width(mut self, width: impl Into<Length>) -> Self {
    self.width = Some(width.into());
    self
  }

  /// Whether the popover asks something that has to be answered: an
  /// `alertdialog`, and not a `dialog`, to tooling that reads the tree.
  pub fn alert(mut self, alert: bool) -> Self {
    self.alert = alert;
    self
  }

  /// What the popover is called to tooling that reads the tree, for one
  /// whose title is drawn by what it holds and not given to [`Popover::title`].
  pub fn named(mut self, name: &str) -> Self {
    self.name = Some(Arc::from(name));
    self
  }

  /// What the popover is described by to tooling that reads the tree.
  pub fn described(mut self, description: &str) -> Self {
    self.described = Some(Arc::from(description));
    self
  }

  /// Puts the popover in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this popover over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this popover.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the panel.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the popover in `theme`. Converting a popover into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    };
    // A width given here is under what `style` and `rules` say of the root.
    let mut rules = Recipe::new();
    if let Some(width) = self.width {
      rules = rules.base(PopoverPart::Root, |s| s.width(width));
    }
    let rules = rules.extend(&self.rules);
    let view = PopoverView {
      title: self.title,
      description: self.description,
      children: self.children,
      recipe: theme.resolve::<Popover>(&case, Some(&rules)),
      alert: self.alert,
      name: self.name,
      described: self.described,
      id: self.id,
      key: self.key,
    };
    let panel = match theme.renderer::<Popover>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    };
    // The offset is away from the anchor: along the side the popover is on.
    let offset = self.offset.resolve(&theme.spacing(), 0.0);
    let (x, y) = match self.placement {
      Placement::LeftStart
      | Placement::Left
      | Placement::LeftEnd
      | Placement::RightStart
      | Placement::Right
      | Placement::RightEnd => (offset, 0.0),
      _ => (0.0, offset),
    };
    let popup = Popup::new(self.anchor.clone(), panel)
      .open(self.open.clone())
      .placement(self.placement)
      .offset(x, y)
      .collision(self.collision)
      .takes_focus(true);
    let Some(mut trigger) = self.trigger else {
      return popup.into();
    };
    // The trigger's own node is the anchor, so the engine knows it for the popup's trigger: the first thing
    // a press reaches that takes the focus.
    let node = &mut *trigger.node;
    let (open, expanded) = (self.open.clone(), self.open.get());
    NodeUpdate::ref_element(node, self.anchor);
    NodeUpdate::on_click(node, move |_: crate::app::events::MouseEvent| {
      open.update(|open| *open = !*open)
    });
    NodeUpdate::describe(node, Arc::from("a11y_haspopup"), Arc::from("dialog"));
    NodeUpdate::describe(
      node,
      Arc::from("a11y_expanded"),
      Arc::from(if expanded { "true" } else { "false" }),
    );
    Row::new().child(trigger).child(popup).into()
  }
}

impl From<Popover> for Element {
  fn from(popover: Popover) -> Self {
    popover.build(&current_theme())
  }
}

/// A popover's panel as a renderer sees it: what it was given and the style of its parts.
pub struct PopoverView {
  pub title: Option<Arc<str>>,
  pub description: Option<Arc<str>>,
  /// What the popover holds.
  pub children: Vec<Element>,
  /// The style of each part of this popover.
  pub recipe: ResolvedRecipe,
  alert: bool,
  name: Option<Arc<str>>,
  described: Option<Arc<str>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl PopoverView {
  /// The style of a part, as this popover is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// A text in the style of a part, in a node that has the part's color.
  fn text(&self, part: PopoverPart, content: &str) -> Column {
    let part = self.part(part);
    Column::new().part(&part).child(part.text(content))
  }

  /// The title over the description, where the popover has either.
  pub fn header(&self) -> Option<Column> {
    if self.title.is_none() && self.description.is_none() {
      return None;
    }
    let mut header = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(PopoverPart::Header));
    if let Some(title) = &self.title {
      header = header.child(self.text(PopoverPart::Title, title));
    }
    if let Some(description) = &self.description {
      header = header.child(self.text(PopoverPart::Description, description));
    }
    Some(header)
  }

  /// The panel with nothing in it yet: what a renderer puts the header and
  /// the body in. It is a dialog to what reads the tree, called by its title.
  pub fn panel(&self) -> Column {
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(PopoverPart::Root))
      .describe("a11y_role", if self.alert { "alertdialog" } else { "dialog" });
    if let Some(name) = self.name.as_ref().or(self.title.as_ref()) {
      panel = panel.describe("a11y_name", name.clone());
    }
    if let Some(description) = &self.described {
      panel = panel.describe("a11y_description", description.clone());
    }
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    if let Some(key) = &self.key {
      panel = panel.key(key.clone());
    }
    panel
  }

  /// How the default renderer builds a popover: the header over the body, in the panel.
  pub fn render(mut self) -> Element {
    let mut panel = self.panel();
    if let Some(header) = self.header() {
      panel = panel.child(header);
    }
    let children = std::mem::take(&mut self.children);
    if !children.is_empty() {
      let mut body = Column::new()
        .align_items(Alignment::Stretch)
        .part(&self.part(PopoverPart::Body));
      for child in children {
        body = body.child(child);
      }
      panel = panel.child(body);
    }
    panel.into()
  }
}

impl Themed for Popover {
  const NAME: &'static str = "popover";
  type Renderer = PopoverRenderer;

  /// The design's popover (`pencil/lurq-design-system.pen`, Popover): a panel
  /// 292 wide in the popover's colors, with an edge of 1 in the border's
  /// color, the large radius and the medium shadow, 16 of padding and 14
  /// between its header and its body. The header has the title in the body's
  /// type at medium weight over the description in 13 and the muted
  /// foreground, 4 apart; what the body holds is 14 apart. It fades in over
  /// the theme's base duration in the entering curve, and out over the fast
  /// one in the leaving curve (Foundations, Motion: popovers take the base
  /// duration, and exits are quicker than entrances).
  fn recipe() -> Recipe {
    use PaletteColor::{Border as Edge, MutedForeground, Popover as Panel, PopoverForeground};
    use PopoverPart::{Body, Description, Header, Root, Title};
    Recipe::new()
      .base(Root, |s| {
        s.width(292.0)
          .padding_x(16.0)
          .padding_y(16.0)
          .gap(14.0)
          .radius(RadiusSize::Lg)
          .background(Panel)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, Edge))
          .shadow(ShadowStyle::Md)
          .enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
          .exit(PartAnimation::new(FADE_OUT, MotionDuration::Fast, MotionEasing::Exit))
      })
      .base(Header, |s| s.gap(4.0))
      .base(Title, |s| {
        s.typography(TypographyStyle::Body).weight(FontWeight::Medium)
      })
      .base(Description, |s| {
        s.typography(TypographyStyle::Description)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .base(Body, |s| s.gap(14.0))
  }
}
