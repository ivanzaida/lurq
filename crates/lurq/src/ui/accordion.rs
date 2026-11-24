//! An accordion: collapsibles one under the other, of which one is open at a time, or any number.

use std::sync::Arc;

use super::{
  collapsible::{CollapsibleState, CollapsibleView, Content, opening},
  icons, names,
  recipe::{Case, Interaction, Name, Recipe, ResolvedRecipe},
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
  /// What is pressed: the header of an item, with its label and chevron.
  Trigger,
  /// What the header says.
  Label,
  /// The chevron at the end of the header: down while the item is closed, up while it is open.
  Chevron,
  /// What opens and closes: as high as what it holds while the item is open, of no height while it is closed.
  Panel,
  /// What the panel holds, in its padding.
  Body,
}

names!(AccordionPart {
  Root => "root",
  Item => "item",
  Trigger => "trigger",
  Label => "label",
  Chevron => "chevron",
  Panel => "panel",
  Body => "body",
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
/// let rich = AccordionItem::new("theme", "Can I bring my own theme?", Column::new())
///   .disabled(true)
///   .id("theme");
/// ```
pub struct AccordionItem<T> {
  value: T,
  label: Arc<str>,
  body: Content,
  disabled: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T> AccordionItem<T> {
  /// An item of `value` whose header says `label`, and that shows `content` while it is open.
  pub fn new(value: T, label: &str, content: impl Into<Element>) -> Self {
    Self::of(value, label, Content::Element(content.into()))
  }

  /// An item of `value` whose header says `label`, and that shows the text `body` while it is open.
  pub fn text(value: T, label: &str, body: &str) -> Self {
    Self::of(value, label, Content::Text(Arc::from(body)))
  }

  fn of(value: T, label: &str, body: Content) -> Self {
    Self {
      value,
      label: Arc::from(label),
      body,
      disabled: false,
      id: None,
      key: None,
    }
  }

  /// Whether the item's header takes no input: it stays as it is, open or closed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The id of the item's header: what is pressed.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the item's header, among what is beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

/// What is told the value of the accordion, once a press has changed it: `Option<T>`, or the whole set `[T]`.
type Handler<V> = Arc<dyn Fn(&V) + Send + Sync>;

/// What a press of the header of the item of a value makes of the items open: the values open after it. Where the
/// accordion is bound, it sets the signal to them.
type Toggle<T> = Arc<dyn Fn(&T) -> Vec<T> + Send + Sync>;

/// What tells the handlers the items open after a press, as the accordion's value: one or none, or the set.
type Tell<T, V> = fn(&[T], &[Handler<V>]);

/// The values open after a press of `value` where one is open at a time: it alone, or none where it was open.
fn one<T: Clone + PartialEq>(open: &[T], value: &T) -> Vec<T> {
  if open.contains(value) {
    Vec::new()
  } else {
    vec![value.clone()]
  }
}

/// The values open after a press of `value` where any number are open: those with it, or without it where it was.
fn many<T: Clone + PartialEq>(open: &[T], value: &T) -> Vec<T> {
  let mut next = open.to_vec();
  match next.iter().position(|held| held == value) {
    Some(index) => {
      next.remove(index);
    }
    None => next.push(value.clone()),
  }
  next
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
/// let faq = Accordion::optional(open, [
///   AccordionItem::text(0, "Is lurq GPU-accelerated?", "Yes, through wgpu."),
///   AccordionItem::text(1, "Does it support dark mode?", "Every theme has a light and a dark appearance."),
/// ])
/// .label("Questions")
/// .on_change(|open: &Option<u8>| println!("{open:?}"));
/// // Any number open: the signal's values. A press is told the whole set open after it.
/// let form = Accordion::multiple(sections, [
///   AccordionItem::text("billing", "Billing", "Card, address and invoices."),
///   AccordionItem::text("shipping", "Shipping", "Where the order goes."),
/// ])
/// .on_change(|open: &[&str]| println!("{open:?}"));
/// // The app holds what is open, and is told what is to be.
/// let held = Accordion::controlled_optional(Some(1), [AccordionItem::text(1, "Billing", "..")], |open| {
///   println!("{open:?}")
/// });
/// # }
/// ```
///
/// Bound to a signal ([`optional`](Self::optional), [`multiple`](Self::multiple)) the accordion sets it, or the
/// app holds what is open and is told what is to be ([`controlled_optional`](Self::controlled_optional),
/// [`controlled_multiple`](Self::controlled_multiple)). [`on_change`](Self::on_change) is told the accordion's
/// value: `&Option<T>` for one at a time, an `Accordion<T>`, and the whole set `&[T]` for any number, an
/// `Accordion<T, [T]>`. `T` is `()` where nothing names it: the theme's recipe is `Accordion`'s, whatever its
/// values. An accordion has no form of which one must be open.
///
/// Each header is a stop of Tab, which Space and Enter press, and Up and Down
/// move the focus from one header to the next. What an item holds opens and
/// closes with its height over the theme's duration, at once where motion is
/// reduced; while it is closed it is not there, and the controls in it are
/// not stops of Tab.
pub struct Accordion<T = (), V: ?Sized = Option<T>> {
  items: Vec<AccordionItem<T>>,
  open: Vec<T>,
  toggle: Toggle<T>,
  tell: Tell<T, V>,
  on_change: Vec<Handler<V>>,
  disabled: bool,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Accordion<T, Option<T>> {
  /// An accordion of which one item is open at a time: the one whose value
  /// the signal has, or none. A press of another opens it and closes the one
  /// that was open; a press of the open one closes it.
  pub fn optional(open: Signal<Option<T>>, items: impl IntoIterator<Item = AccordionItem<T>>) -> Self
  where
    Option<T>: SignalValue,
  {
    let now: Vec<T> = open.get().into_iter().collect();
    Self::of(now, items, Self::tell_one, move |value| {
      let held: Vec<T> = open.get_untracked().into_iter().collect();
      let next = one(&held, value);
      open.set(next.first().cloned());
      next
    })
  }

  /// An accordion of which one item is open at a time, whose state the app holds: the item of `open` is open,
  /// and `on_change` is told, on a press of a header, the value that is to be open: that item's, or none where it
  /// was open. The accordion opens and closes nothing itself.
  pub fn controlled_optional(
    open: Option<T>,
    items: impl IntoIterator<Item = AccordionItem<T>>,
    on_change: impl Fn(&Option<T>) + Send + Sync + 'static,
  ) -> Self {
    let now: Vec<T> = open.into_iter().collect();
    let held = now.clone();
    Self::of(now, items, Self::tell_one, move |value| one(&held, value)).on_change(on_change)
  }

  fn tell_one(next: &[T], handlers: &[Handler<Option<T>>]) {
    let next = next.first().cloned();
    for handler in handlers {
      handler(&next);
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> Accordion<T, [T]> {
  /// An accordion of which any number of items are open: those whose values
  /// the signal has. A press opens an item, or closes it.
  pub fn multiple(open: Signal<Vec<T>>, items: impl IntoIterator<Item = AccordionItem<T>>) -> Self
  where
    Vec<T>: SignalValue,
  {
    let now = open.get();
    Self::of(now, items, Self::tell_many, move |value| {
      let next = many(&open.get_untracked(), value);
      open.set(next.clone());
      next
    })
  }

  /// An accordion of which any number of items are open, whose state the app holds: the items of `open` are open,
  /// and `on_change` is told, on a press of a header, the values that are to be open: those of `open` with that
  /// item's, or without it where it was open.
  pub fn controlled_multiple(
    open: impl IntoIterator<Item = T>,
    items: impl IntoIterator<Item = AccordionItem<T>>,
    on_change: impl Fn(&[T]) + Send + Sync + 'static,
  ) -> Self {
    let now: Vec<T> = open.into_iter().collect();
    let held = now.clone();
    Self::of(now, items, Self::tell_many, move |value| many(&held, value)).on_change(on_change)
  }

  fn tell_many(next: &[T], handlers: &[Handler<[T]>]) {
    for handler in handlers {
      handler(next);
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> Accordion<T, V> {
  fn of(
    open: Vec<T>,
    items: impl IntoIterator<Item = AccordionItem<T>>,
    tell: Tell<T, V>,
    toggle: impl Fn(&T) -> Vec<T> + Send + Sync + 'static,
  ) -> Self {
    Self {
      items: items.into_iter().collect(),
      open,
      toggle: Arc::new(toggle),
      tell,
      on_change: Vec::new(),
      disabled: false,
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      id: None,
      key: None,
    }
  }

  /// Called on a press of a header, after the signal is set where the accordion is bound, with the accordion's
  /// value now: the item open or none (`&Option<T>`), or every item open (`&[T]`). A value the app sets is told to
  /// none of them. A second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(&V) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Whether no header of the accordion takes input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What the accordion is called, to what reads the tree: "Questions". It is a group of that name.
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Puts the accordion and each of its items in a state the theme's recipe names, or takes them out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
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

  /// Draws the header Tab would stop on first (that of the first item that is not disabled) as it is under the
  /// pointer, held down or focused from the keyboard, with nothing pointing at it: for a gallery or a test. The
  /// other headers are drawn as they are.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the root: the items.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the root, among what is beside it.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the accordion in `theme`. Converting an accordion into an
  /// element does this in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let breakpoint = super::current_breakpoint();
    let states = self.states;
    let case = |open: bool, disabled: bool| {
      Case {
        breakpoint,
        states: states.clone(),
        ..Case::new()
      }
      .state(CollapsibleState::Open, open)
      .state(CollapsibleState::Disabled, disabled)
    };
    let entry = if self.disabled {
      None
    } else {
      self.items.iter().position(|item| !item.disabled)
    };
    let handlers: Arc<[Handler<V>]> = self.on_change.into();
    let tell = self.tell;
    let recipe = theme
      .resolve::<Accordion>(&case(false, self.disabled), Some(&self.rules))
      .owned_by(self.id.as_ref());
    let items = self
      .items
      .into_iter()
      .enumerate()
      .map(|(index, item)| {
        let disabled = self.disabled || item.disabled;
        let open = self.open.contains(&item.value);
        let (toggle, handlers, value) = (self.toggle.clone(), handlers.clone(), item.value);
        let press = Arc::new(move |_: bool| {
          tell(&toggle(&value), &handlers);
        });
        CollapsibleView::item(
          item.label,
          item.body,
          open,
          disabled,
          // One chevron, which its part turns as the item opens.
          icons::CHEVRON_DOWN.into(),
          theme
            .resolve::<Accordion>(&case(open, disabled), Some(&self.rules))
            .owned_like(&recipe)
            .item(item.id.as_ref(), AccordionPart::Item),
          AccordionPart::Item.into(),
          press,
          self.forced.filter(|_| entry == Some(index)),
          item.id,
          item.key,
        )
      })
      .collect();
    let view = AccordionView {
      items,
      label: self.label,
      recipe,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Accordion>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> From<Accordion<T, V>> for Element {
  fn from(accordion: Accordion<T, V>) -> Self {
    accordion.build(&current_theme())
  }
}

/// An accordion as a renderer sees it: each item as a collapsible in the
/// accordion's recipe, and the style of its root.
pub struct AccordionView {
  /// The items, each in the case it is in: open or closed, disabled or not.
  pub items: Vec<CollapsibleView>,
  /// What the accordion is called, to what reads the tree.
  pub label: Option<Arc<str>>,
  /// The style of each part of the accordion as a whole, its items closed.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl AccordionView {
  /// The root in the style of its part, with the behavior of the keyboard: each header is a stop of Tab, as a
  /// web accordion's is, and Up and Down move the focus between them. Given a label, it is a group of that name
  /// to what reads the tree.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .focus_group(FocusGroup::vertical())
      // The ring of the focus is drawn around a header, outside it.
      .overflow_visible()
      .part(&self.recipe.part(AccordionPart::Root));
    if let Some(label) = &self.label {
      root = root.describe("a11y_role", "group").describe("a11y_name", label.clone());
    }
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

impl<T: Clone + PartialEq + Send + Sync + 'static, V: ?Sized + 'static> crate::ui::Styled for Accordion<T, V> {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

/// Every accordion has the one recipe and renderer, whatever its items' values are.
impl<T: Send + Sync + 'static, V: ?Sized + 'static> Themed for Accordion<T, V> {
  const NAME: &'static str = "accordion";
  type Renderer = AccordionRenderer;

  fn parts() -> crate::ui::Declared {
    AccordionPart::declared()
  }

  /// The design's accordion (`pencil/lurq-design-system.pen`, Accordion):
  /// items with a line of the border's color under each; a header 14 high
  /// above and under its label, which is in the body's type at medium weight,
  /// with a chevron of 16 in the muted foreground at its end, 12 from the
  /// label; what an item opens is in the body's type in the muted
  /// foreground, at a line height of 1.5, 14 above the line.
  fn recipe() -> Recipe {
    use AccordionPart::{Body, Chevron, Item, Label, Panel, Root, Trigger};
    use PaletteColor::{Foreground, MutedForeground};

    use crate::{animation::Transition, app::theme::MotionDuration, ui::PartTransition};
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
      // The chevron points down, and turns up as its item opens, in the base time.
      .base(Chevron, |s| {
        s.square(16.0)
          .foreground(MutedForeground)
          .transition(PartTransition::new(Transition::transform(), MotionDuration::Base))
      })
      .state(CollapsibleState::Open, Chevron, |s| s.rotate(180.0))
      .base(Body, |s| {
        s.padding_bottom(14.0)
          .typography(TypographyStyle::Body)
          .line_height(1.5)
          .foreground(MutedForeground)
      })
      // A row of the density's height: 14 above and under the label at the default, the steps of a row apart.
      .by_density(super::Selector::new(), Trigger, |s, step| s.padding_y(14.0 + step / 2.0))
      .focus_visible(Trigger, |s| s.focus_ring())
      .state(CollapsibleState::Disabled, Trigger, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      });
    opening(recipe, CollapsibleState::Open.into(), Panel.into(), Body.into())
  }
}
