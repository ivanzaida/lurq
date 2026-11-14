//! A banner: a message across the full width of what holds it, at the top of
//! a window or a page, that the app shows and takes away.

use std::sync::Arc;

use super::{
  AlertVariant, Link, LinkPart,
  alert::{AlertIcon, close_control, close_style, shown_icon, wrapping},
  icons, names,
  part_style::{Length, PartAnimation},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{FADE_IN, FADE_OUT, glyph},
  theme::{Themed, current_breakpoint, current_theme},
};
use crate::{
  app::theme::{IconSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{self, Column, Row},
  layout::{
    Alignment,
    layout_kind::{Justify, Sticky},
    text_style::FontWeight,
  },
  node::{Element, dimension::Dimension},
};

/// The parts of a banner a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BannerPart {
  /// What stands in the flow of the page as wide as the room it is given,
  /// shown or not: what sticks to the top where the page scrolls.
  Root,
  /// The bar: its fill, its corners, the room inside it, and how it comes in
  /// and leaves (`enter`, `exit`).
  Panel,
  /// The icon of its variant, at the start.
  Icon,
  /// What it says.
  Text,
  /// What holds the action, after the text.
  Action,
  /// A link given as the action, in the variant's color: what the board's
  /// "Update now" is.
  Link,
  /// What closes it, at the end.
  Close,
}

names!(BannerPart {
  Root => "root",
  Panel => "panel",
  Icon => "icon",
  Text => "text",
  Action => "action",
  Link => "link",
  Close => "close",
});

/// The states of a banner a recipe styles, besides what the pointer and the
/// keyboard do to its close control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BannerState {
  /// What is in it stands in its middle, the text as wide as it is.
  Centered,
  /// It sticks to the top of what scrolls it.
  Sticky,
}

names!(BannerState {
  Centered => "centered",
  Sticky => "sticky",
});

/// What a banner's action is: an element of the app's, or a link it gives the
/// variant's color.
enum BannerAction {
  Element(Element),
  Link(Link),
}

/// What builds a banner's nodes: a theme's in place of the default one.
pub type BannerRenderer = Arc<dyn Fn(BannerView) -> Element + Send + Sync>;

/// A banner: a message across the full width of what holds it, one line
/// where it fits, at the top of a window or a page. A new version, a
/// connection lost, a trial that ends.
///
/// ```
/// use lurq::ui::{AlertVariant, Banner, Link};
///
/// let update = Banner::new("lurq 0.61 is available with faster text shaping.")
///   .variant(AlertVariant::Info)
///   .link(Link::new("Update now").on_click(|_| println!("update")))
///   .on_close(|| println!("dismissed"));
/// ```
///
/// The app shows it and takes it away: [`Banner::open`] with what the app
/// keeps, false once its close control is pressed. It stays in the tree
/// either way, so it fades in as it is shown and out as it is taken away,
/// and the page under it moves up when it is gone. Where the page scrolls,
/// [`Banner::sticky`] keeps it at the top.
pub struct Banner {
  text: Arc<str>,
  variant: AlertVariant,
  icon: Option<AlertIcon>,
  action: Option<BannerAction>,
  open: bool,
  centered: bool,
  sticky: bool,
  role: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_close: Vec<Arc<dyn Fn() + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Banner {
  /// A banner that says `text`.
  pub fn new(text: &str) -> Self {
    Self {
      text: Arc::from(text),
      variant: AlertVariant::Neutral,
      icon: None,
      action: None,
      open: true,
      centered: false,
      sticky: false,
      role: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_close: Vec::new(),
      id: None,
      key: None,
    }
  }

  pub fn variant(mut self, variant: AlertVariant) -> Self {
    self.variant = variant;
    self
  }

  /// An icon of the theme's set in place of the variant's, or none: `icon(None)`.
  pub fn icon(mut self, icon: impl Into<AlertIcon>) -> Self {
    self.icon = Some(icon.into());
    self
  }

  /// What can be done about it, after the text: a button, or any element.
  pub fn action(mut self, action: impl Into<Element>) -> Self {
    self.action = Some(BannerAction::Element(action.into()));
    self
  }

  /// A link as the action, after the text, in the variant's color and
  /// semibold: "Update now".
  pub fn link(mut self, link: Link) -> Self {
    self.action = Some(BannerAction::Link(link));
    self
  }

  /// Whether it is shown. It is unless it says; taken away it fades out.
  pub fn open(mut self, open: bool) -> Self {
    self.open = open;
    self
  }

  /// Whether what is in it stands in its middle, as a bar at the top of a
  /// window has it, rather than from its start.
  pub fn centered(mut self, centered: bool) -> Self {
    self.centered = centered;
    self
  }

  /// Whether it sticks to the top of what scrolls the page it is on.
  pub fn sticky(mut self, sticky: bool) -> Self {
    self.sticky = sticky;
    self
  }

  /// What it is to tooling that reads the tree, in place of its variant's.
  pub fn role(mut self, role: &str) -> Self {
    self.role = Some(Arc::from(role));
    self
  }

  /// Puts the banner in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a press of its close control, which it has once it is given
  /// one. The app takes the banner away: `open(false)`.
  pub fn on_close(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_close.push(Arc::new(handler));
    self
  }

  /// Styles one part of this banner over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this banner.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the close control as it is under the pointer, held down or
  /// focused from the keyboard, with nothing pointing at it: for a gallery
  /// or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the bar.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of what stands in the flow of the page.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the banner in `theme`. Converting a banner into an element does
  /// this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case {
      variant: Some(self.variant.into()),
      size: None,
      density: None,
      breakpoint: current_breakpoint(),
      states: self.states.clone(),
    }
    .state(BannerState::Centered, self.centered)
    .state(BannerState::Sticky, self.sticky);
    // The board's banner announces: a neutral and an informative one have the megaphone.
    let default_icon = match self.variant {
      AlertVariant::Neutral | AlertVariant::Info => icons::MEGAPHONE,
      variant => variant.icon(),
    };
    let view = BannerView {
      text: self.text,
      variant: self.variant,
      icon: shown_icon(self.icon, default_icon),
      action: self.action,
      open: self.open,
      centered: self.centered,
      sticky: self.sticky,
      role: self.role.unwrap_or_else(|| Arc::from(self.variant.role())),
      closable: !self.on_close.is_empty(),
      recipe: theme.resolve::<Banner>(&case, Some(&self.rules)),
      forced: self.forced,
      on_close: self.on_close,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Banner>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Banner> for Element {
  fn from(banner: Banner) -> Self {
    banner.build(&current_theme())
  }
}

/// A banner as a renderer sees it: what it was given, the style of its
/// parts, and its behavior ready to be dressed.
pub struct BannerView {
  pub text: Arc<str>,
  pub variant: AlertVariant,
  /// The icon it shows, by its name in the theme's set: the variant's, the one it was given, or none.
  pub icon: Option<Name>,
  action: Option<BannerAction>,
  /// Whether it is shown.
  pub open: bool,
  pub centered: bool,
  pub sticky: bool,
  /// What it is to tooling that reads the tree: `alert` or `status`, or what the app said.
  pub role: Arc<str>,
  /// Whether it has a close control: it was given a handler for it.
  pub closable: bool,
  /// The style of each part of this banner.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  on_close: Vec<Arc<dyn Fn() + Send + Sync>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl BannerView {
  /// The style of a part, as this banner is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The icon in the style of its part, if the banner has one.
  pub fn icon(&self) -> Option<Element> {
    self.icon.map(|icon| glyph(&self.part(BannerPart::Icon), icon, false))
  }

  /// What it says: as wide as the room the rest leaves it, going on to the
  /// next line where it does not fit in one; centered, as wide as it is and
  /// no wider than that room.
  pub fn text(&self) -> Column {
    let part = self.part(BannerPart::Text);
    if self.centered {
      Column::new()
        .flex_full(0.0, 1.0, None)
        .min_width(Dimension::Px(0.0))
        .part(&part)
        .child(part.text(&self.text))
    } else {
      wrapping(&part, &self.text).flex(1.0).min_width(Dimension::Px(0.0))
    }
  }

  /// The action, in the style of its part: a link given as one in the style
  /// of the `Link` part over the theme's for a link.
  pub fn action(&mut self) -> Option<Row> {
    let action = match self.action.take()? {
      BannerAction::Element(element) => element,
      BannerAction::Link(link) => {
        let style = self.part(BannerPart::Link).rest();
        let (color, text) = (style.foreground.clone(), style.text.clone());
        link
          .style(LinkPart::Root, move |mut s| {
            s.foreground = color;
            s
          })
          .style(LinkPart::Label, move |mut s| {
            s.text = text;
            s
          })
          .into()
      }
    };
    Some(
      Row::new()
        .align_items(Alignment::Center)
        .part(&self.part(BannerPart::Action))
        .child(action),
    )
  }

  /// What closes it: a press of it tells every handler of `on_close`.
  pub fn close(&self) -> components::Button {
    close_control(&self.part(BannerPart::Close), &self.on_close)
  }

  /// The bar: its icon, its text, its action and its close control side by
  /// side. It is what its role says to what reads the tree, called by its
  /// text. It has a key, so that what holds it keeps it while it fades out.
  pub fn panel(&mut self) -> Row {
    let mut panel = Row::new()
      .key("banner")
      .align_items(Alignment::Center)
      .justify(if self.centered { Justify::Center } else { Justify::Start })
      .part(&self.part(BannerPart::Panel))
      .describe("a11y_role", self.role.clone())
      .describe("a11y_name", self.text.clone());
    if let Some(icon) = self.icon() {
      panel = panel.child(icon);
    }
    panel = panel.child(self.text());
    if let Some(action) = self.action() {
      panel = panel.child(action);
    }
    if self.closable {
      panel = panel.child(self.close());
    }
    if let Some(id) = &self.id {
      panel = panel.id(id.clone());
    }
    panel
  }

  /// How the default renderer builds a banner: what stands in the flow of
  /// the page, as wide as the room it is given, and in it the bar while it
  /// is shown. What stands keeps the bar while it leaves.
  pub fn render(mut self) -> Element {
    let mut root = Column::new().presence().part(&self.part(BannerPart::Root));
    if self.sticky {
      root = root.sticky(Sticky::top(0.0));
    }
    if self.open {
      root = root.child(self.panel());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl Themed for Banner {
  const NAME: &'static str = "banner";
  type Renderer = BannerRenderer;

  /// The design's banner (`pencil/lurq-design-system.pen`, Banner): a bar as
  /// wide as the room it is given in the muted form of the variant's color
  /// (the muted fill for a neutral one), with corners of `radius-md`, padded
  /// 10 and 16. In it, 12 apart and centered on a line, the icon of 16 in the
  /// variant's color, the text of 13 in the foreground's, a link in the
  /// variant's color, semibold, and the close control of 16. It fades in
  /// over the theme's base duration as it is shown and out over its fast
  /// one as it is taken away.
  fn recipe() -> Recipe {
    use BannerPart as P;
    let mut recipe = Recipe::new()
      .base(P::Root, |s| s.width(Length::Fill))
      .base(P::Panel, |s| {
        s.width(Length::Fill)
          .background(PaletteColor::Muted)
          .foreground(PaletteColor::Foreground)
          .radius(RadiusSize::Md)
          .padding_x(16.0)
          .padding_y(10.0)
          .gap(12.0)
          .enter(PartAnimation::new(FADE_IN, MotionDuration::Base, MotionEasing::Enter))
          .exit(PartAnimation::new(FADE_OUT, MotionDuration::Fast, MotionEasing::Exit))
      })
      .base(P::Icon, |s| {
        s.square(Length::Icon(IconSize::Md))
          .foreground(PaletteColor::Foreground)
      })
      .base(P::Text, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(PaletteColor::Foreground)
      })
      .base(P::Action, |s| s.gap(8.0))
      .base(P::Link, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::SemiBold)
          .foreground(PaletteColor::Foreground)
      });
    recipe = close_style(recipe, P::Close).base(P::Close, |s| s.foreground(PaletteColor::Foreground));
    for variant in AlertVariant::all().iter().copied() {
      recipe = recipe
        .variant(variant, P::Panel, |s| s.background(variant.tint()))
        .variant(variant, P::Icon, |s| s.foreground(variant.accent()))
        .variant(variant, P::Link, |s| s.foreground(variant.accent()));
    }
    recipe
  }
}
