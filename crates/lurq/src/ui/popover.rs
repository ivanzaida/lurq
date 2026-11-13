//! A popover: a panel beside the element that opens it, over the window, holding anything.

use std::sync::Arc;

use super::{
  names,
  part_style::{Length, PartStyle},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    ctx::{CollisionStrategy, Placement},
    theme::{BorderSize, PaletteColor, RadiusSize, ShadowStyle, SpacingSize, Theme, TypographyStyle},
  },
  components::{Column, Popup},
  core::{ElementRef, Signal},
  layout::{Alignment, text_style::FontWeight},
  node::{Element, SpacingValue, border::Border},
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
/// let (anchor, open) = (ElementRef::new(), Signal::new(false));
/// let toggle = open.clone();
/// let trigger = Row::new()
///   .ref_element(anchor.clone())
///   .child(Button::new("Dimensions").on_click(move |_| toggle.update(|open| *open = !*open)));
/// let popover = Popover::new(anchor, open)
///   .title("Dimensions")
///   .description("Set the size of the layer.")
///   .placement(Placement::BottomStart)
///   .child(Field::new("Width", Input::new(Signal::new("320px".to_owned())).full_width(true)).full_width(true));
/// let both = Row::new().child(trigger).child(popover);
/// ```
///
/// A popover does not keep whether it is open: the app's signal does. A
/// press outside it or Escape closes it, which sets the signal to `false`.
/// The element that opens it is the app's too: the anchor is what it is
/// shown beside, and a press of the anchor is not a press outside it.
pub struct Popover {
  anchor: ElementRef,
  open: Signal<bool>,
  title: Option<Arc<str>>,
  description: Option<Arc<str>>,
  children: Vec<Element>,
  placement: Placement,
  offset: SpacingValue,
  collision: CollisionStrategy,
  width: Option<Length>,
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
      title: None,
      description: None,
      children: Vec::new(),
      placement: Placement::BottomStart,
      offset: SpacingSize::Xs.into(),
      collision: CollisionStrategy::FlipThenClamp,
      width: None,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
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
    Popup::new(self.anchor, panel)
      .open(self.open)
      .placement(self.placement)
      .offset(x, y)
      .collision(self.collision)
      .into()
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
      .describe("a11y_role", "dialog");
    if let Some(title) = &self.title {
      panel = panel.describe("a11y_name", title.clone());
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
  /// foreground, 4 apart; what the body holds is 14 apart.
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
