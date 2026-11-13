//! An accordion: collapsibles one under the other, of which one is open at a time, or any number.

use std::sync::Arc;

use super::{
  collapsible::{Body, CollapsibleState, CollapsibleView, opening},
  icons, names,
  recipe::{Case, Name, Recipe, ResolvedRecipe},
  shared::Chosen,
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::Column,
  core::{Signal, SignalValue},
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, border::Border},
};

/// The parts of an accordion a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccordionPart {
  /// The items, one under the other.
  Root,
  /// One item: its header and its panel, with the line under them.
  Item,
  /// What is pressed: the header of an item, with its label and indicator.
  Trigger,
  /// What the header says.
  Label,
  /// The chevron at the end of the header: down while the item is closed, up while it is open.
  Indicator,
  /// What opens and closes: as high as what it holds while the item is open, of no height while it is closed.
  Panel,
  /// What the panel holds, in its padding.
  Content,
}

names!(AccordionPart {
  Root => "root",
  Item => "item",
  Trigger => "trigger",
  Label => "label",
  Indicator => "indicator",
  Panel => "panel",
  Content => "content",
});

/// The states of an item of an accordion a recipe styles: those of a
/// [`Collapsible`](super::Collapsible), [`CollapsibleState::Open`] and
/// [`CollapsibleState::Disabled`].
pub type AccordionState = CollapsibleState;

/// What builds an accordion's nodes: a theme's in place of the default one.
pub type AccordionRenderer = Arc<dyn Fn(AccordionView) -> Element + Send + Sync>;

/// One item of an accordion: what its header says, what it shows while it is
/// open, and the value it is told apart by.
///
/// ```
/// use lurq::{components::Column, ui::AccordionItem};
///
/// let plain = AccordionItem::text("gpu", "Is lurq GPU-accelerated?", "Yes, through wgpu.");
/// let rich = AccordionItem::new("theme", "Can I bring my own theme?", Column::new()).disabled(true);
/// ```
pub struct AccordionItem<T> {
  value: T,
  label: Arc<str>,
  body: Body,
  disabled: bool,
}

impl<T> AccordionItem<T> {
  /// An item of `value` whose header says `label`, and that shows `content` while it is open.
  pub fn new(value: T, label: &str, content: impl Into<Element>) -> Self {
    Self {
      value,
      label: Arc::from(label),
      body: Body::Element(content.into()),
      disabled: false,
    }
  }

  /// An item of `value` whose header says `label`, and that shows the text `body` while it is open.
  pub fn text(value: T, label: &str, body: &str) -> Self {
    Self {
      value,
      label: Arc::from(label),
      body: Body::Text(Arc::from(body)),
      disabled: false,
    }
  }

  /// Whether the item's header takes no input: it stays as it is, open or closed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }
}

/// An item as the accordion builds it: with whether it is open and what a press of it does.
struct Item {
  label: Arc<str>,
  body: Body,
  disabled: bool,
  open: bool,
  press: Arc<dyn Fn(bool) + Send + Sync>,
}

/// Questions and their answers, the sections of a long form, settings in
/// groups: headers one under the other, each of which opens what is under
/// it.
///
/// ```
/// use lurq::{core::Signal, ui::{Accordion, AccordionItem}};
///
/// # fn faq(open: Signal<Option<u8>>, sections: Signal<Vec<&'static str>>) {
/// // One open at a time: the signal's value, or none.
/// let faq = Accordion::new(open, [
///   AccordionItem::text(0, "Is lurq GPU-accelerated?", "Yes, through wgpu."),
///   AccordionItem::text(1, "Does it support dark mode?", "Every theme has a light and a dark appearance."),
/// ]);
/// // Any number open: the signal's values.
/// let form = Accordion::multiple(sections, [
///   AccordionItem::text("billing", "Billing", "Card, address and invoices."),
///   AccordionItem::text("shipping", "Shipping", "Where the order goes."),
/// ]);
/// # }
/// ```
///
/// Each header is a stop of Tab, which Space and Enter press, and Up and Down
/// move the focus from one header to the next. What an item holds opens and
/// closes with its height over the theme's duration, at once where motion is
/// reduced; while it is closed it is not there, and the controls in it are
/// not stops of Tab.
pub struct Accordion {
  items: Vec<Item>,
  disabled: bool,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Accordion {
  fn of<T: Clone + PartialEq + Send + Sync + 'static>(
    open: Chosen<T>,
    items: impl IntoIterator<Item = AccordionItem<T>>,
  ) -> Self {
    let items = items
      .into_iter()
      .map(|item| {
        let pick = open.picker(item.value.clone());
        Item {
          open: open.has(&item.value),
          press: Arc::new(move |_| pick()),
          label: item.label,
          body: item.body,
          disabled: item.disabled,
        }
      })
      .collect();
    Self {
      items,
      disabled: false,
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// An accordion of which one item is open at a time: the one whose value
  /// the signal has, or none. A press of another opens it and closes the one
  /// that was open; a press of the open one closes it.
  pub fn new<T>(open: Signal<Option<T>>, items: impl IntoIterator<Item = AccordionItem<T>>) -> Self
  where
    T: Clone + PartialEq + Send + Sync + 'static,
    Option<T>: SignalValue,
  {
    let now = open.get();
    let press = move |value: T| {
      open.update(|held| {
        *held = match held.take() {
          Some(open) if open == value => None,
          _ => Some(value),
        }
      })
    };
    Self::of(Chosen::held(now.into_iter().collect(), press), items)
  }

  /// An accordion of which any number of items are open: those whose values
  /// the signal has. A press opens an item, or closes it.
  pub fn multiple<T>(open: Signal<Vec<T>>, items: impl IntoIterator<Item = AccordionItem<T>>) -> Self
  where
    T: Clone + PartialEq + Send + Sync + 'static,
    Vec<T>: SignalValue,
  {
    Self::of(Chosen::many(open), items)
  }

  /// An accordion whose state the app holds: the items of `open` are open,
  /// and `press` is told the value of each one whose header is pressed.
  pub fn controlled<T: Clone + PartialEq + Send + Sync + 'static>(
    open: impl IntoIterator<Item = T>,
    items: impl IntoIterator<Item = AccordionItem<T>>,
    press: impl Fn(T) + Send + Sync + 'static,
  ) -> Self {
    Self::of(Chosen::held(open.into_iter().collect(), press), items)
  }

  /// Whether no header of the accordion takes input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Styles one part of this accordion over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this accordion.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the accordion in `theme`. Converting an accordion into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let breakpoint = super::current_breakpoint();
    let case = |open: bool, disabled: bool| {
      Case::new()
        .breakpoint(breakpoint)
        .state(CollapsibleState::Open, open)
        .state(CollapsibleState::Disabled, disabled)
    };
    let items = self
      .items
      .into_iter()
      .map(|item| {
        let disabled = self.disabled || item.disabled;
        CollapsibleView::item(
          item.label,
          item.body,
          item.open,
          disabled,
          (if item.open {
            icons::CHEVRON_UP
          } else {
            icons::CHEVRON_DOWN
          })
          .into(),
          theme.resolve::<Accordion>(&case(item.open, disabled), Some(&self.rules)),
          AccordionPart::Item.into(),
          item.press,
        )
      })
      .collect();
    let view = AccordionView {
      items,
      recipe: theme.resolve::<Accordion>(&case(false, self.disabled), Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Accordion>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<Accordion> for Element {
  fn from(accordion: Accordion) -> Self {
    accordion.build(&current_theme())
  }
}

/// An accordion as a renderer sees it: each item as a collapsible in the
/// accordion's recipe, and the style of its root.
pub struct AccordionView {
  /// The items, each in the case it is in: open or closed, disabled or not.
  pub items: Vec<CollapsibleView>,
  /// The style of each part of the accordion as a whole, its items closed.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl AccordionView {
  /// The root in the style of its part, with the behavior of the keyboard: each header is a stop of Tab, as a
  /// web accordion's is, and Up and Down move the focus between them.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .focus_group(FocusGroup::vertical())
      // The ring of the focus is drawn around a header, outside it.
      .overflow_visible()
      .part(&self.recipe.part(AccordionPart::Root));
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// How the default renderer builds an accordion: its items one under the other.
  pub fn render(mut self) -> Element {
    let items = std::mem::take(&mut self.items);
    self
      .root()
      .with_children(items.into_iter().map(CollapsibleView::render))
      .into()
  }
}

impl Themed for Accordion {
  const NAME: &'static str = "accordion";
  type Renderer = AccordionRenderer;

  /// The design's accordion (`pencil/lurq-design-system.pen`, Accordion):
  /// items with a line of the border's color under each; a header 14 high
  /// above and under its label, which is in the body's type at medium weight,
  /// with a chevron of 16 in the muted foreground at its end, 12 from the
  /// label; what an item opens is in the body's type in the muted
  /// foreground, at a line height of 1.5, 14 above the line.
  fn recipe() -> Recipe {
    use AccordionPart::{Content, Indicator, Item, Label, Panel, Root, Trigger};
    use PaletteColor::{Foreground, MutedForeground};
    let recipe = Recipe::new()
      .base(Root, |s| s.foreground(Foreground))
      .base(Item, |s| {
        s.underline(Border::inside(BorderSize::Sm, PaletteColor::Border))
      })
      .base(Trigger, |s| {
        super::shared::fading(
          s.padding_y(14.0)
            .gap(12.0)
            .radius(RadiusSize::Sm)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(Label, |s| {
        s.typography(TypographyStyle::Body).weight(FontWeight::Medium)
      })
      .base(Indicator, |s| s.square(16.0).foreground(MutedForeground))
      .base(Content, |s| {
        s.padding_bottom(14.0)
          .typography(TypographyStyle::Body)
          .line_height(1.5)
          .foreground(MutedForeground)
      })
      .focus_visible(Trigger, |s| s.focus_ring())
      .state(CollapsibleState::Disabled, Trigger, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      });
    opening(recipe, CollapsibleState::Open.into(), Panel.into(), Content.into())
  }
}
