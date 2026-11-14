//! An action bar: the toolbar of a selection, over the list it acts on.

use std::sync::Arc;

use super::{
  button::{Button, ButtonPart, ButtonVariant},
  icons, names,
  part_style::{Length, PartAnimation},
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  separator::{Separator, SeparatorPart},
  theme::{Themed, current_theme},
};
use crate::{
  animation::KeyframesId,
  app::{
    events::KeyboardEvent,
    theme::{
      BorderSize, IconSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle,
    },
  },
  components::{Column, Row},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, FocusGroup, HitTestBehavior, border::Border},
};

/// The keyframes a toast comes in and leaves with, which every window has
/// (`ui/toast.rs`): a fade, and a move of 16 from the edge it stands at.
const ENTER_FROM_BELOW: KeyframesId = KeyframesId::new(0xFF11);
const ENTER_FROM_ABOVE: KeyframesId = KeyframesId::new(0xFF12);
const LEAVE_DOWNWARD: KeyframesId = KeyframesId::new(0xFF13);
const LEAVE_UPWARD: KeyframesId = KeyframesId::new(0xFF14);

/// Where the bar stands over what it acts on: the variant of its recipe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ActionBarPlacement {
  /// At the bottom, in the middle; it comes up from the edge and goes back down.
  #[default]
  Bottom,
  /// At the top, in the middle; it comes down from the edge and goes back up.
  Top,
}

names!(ActionBarPlacement {
  Bottom => "bottom",
  Top => "top",
});

/// The parts of an action bar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionBarPart {
  /// The room over what the bar acts on, which it stands in: as large as that, and taking no press itself.
  Root,
  /// The bar: its fill, its edge, its shadow, how it comes in and leaves.
  Bar,
  /// How many are selected: "3 selected".
  Count,
  /// What the bar says of the line between the count, the actions and the clear button: laid over a separator's root.
  Separator,
  /// What the bar says of each action: laid over the theme's recipe for a button's root.
  Action,
  /// What the bar says of an action's icon: laid over a button's icon.
  ActionIcon,
  /// What the bar says of an action's label: laid over a button's label.
  ActionLabel,
  /// What the bar says of an action that destroys what it acts on: laid over a button's root, after `Action`.
  Destructive,
  /// What the bar says of the button that clears the selection: laid over a button's root.
  Clear,
  /// What the bar says of the icon of the clear button: laid over a button's icon.
  ClearIcon,
}

names!(ActionBarPart {
  Root => "root",
  Bar => "bar",
  Count => "count",
  Separator => "separator",
  Action => "action",
  ActionIcon => "action_icon",
  ActionLabel => "action_label",
  Destructive => "destructive",
  Clear => "clear",
  ClearIcon => "clear_icon",
});

/// The states of an action bar a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionBarState {
  /// Nothing is selected: the bar is gone, or leaving.
  Empty,
}

names!(ActionBarState {
  Empty => "empty",
});

/// What builds an action bar's nodes: a theme's in place of the default one.
pub type ActionBarRenderer = Arc<dyn Fn(ActionBarView) -> Element + Send + Sync>;

/// One action of the bar: its button, and whether it destroys what it acts on.
pub struct ActionBarAction {
  pub button: Button,
  pub destructive: bool,
}

/// An action bar: the toolbar of a selection. It stands over the list or the
/// grid whose rows are selected, says how many are, offers what can be done
/// to them, and clears them.
///
/// ```
/// use lurq::{
///   components::Stack,
///   node::Element,
///   ui::{ActionBar, Button, icons},
/// };
///
/// # fn build(list: Element, selected: usize) -> Stack {
/// Stack::new().child(list).child(
///   ActionBar::new(selected)
///     .action(Button::new("Move").icon(icons::FOLDER_IMPORT))
///     .action(Button::new("Duplicate").icon(icons::COPY))
///     .destructive_action(Button::new("Delete").icon(icons::TRASH))
///     .on_clear(|| println!("clear"))
///     .escape_clears(true),
/// )
/// # }
/// ```
///
/// It is as large as the room it is given, and takes no press itself: put it
/// over what it acts on, in a [`Stack`](crate::components::Stack). The bar
/// stands at the bottom of that room (or at its top), comes in while
/// something is selected and leaves as nothing is. It is a toolbar: Tab
/// stops in it once, and Left and Right move between its actions. Escape
/// clears the selection where the app says so ([`escape_clears`](Self::escape_clears)).
pub struct ActionBar {
  count: usize,
  text: Option<Arc<str>>,
  actions: Vec<ActionBarAction>,
  on_clear: Option<Arc<dyn Fn() + Send + Sync>>,
  placement: ActionBarPlacement,
  escape_clears: bool,
  clear_label: Arc<str>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ActionBar {
  /// The bar of a selection of `count` things: shown while it is more than none.
  pub fn new(count: usize) -> Self {
    Self {
      count,
      text: None,
      actions: Vec::new(),
      on_clear: None,
      placement: ActionBarPlacement::default(),
      escape_clears: false,
      clear_label: Arc::from("Clear selection"),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// What the bar says of the selection, in place of "3 selected": "3 files".
  pub fn text(mut self, text: &str) -> Self {
    self.text = Some(Arc::from(text));
    self
  }

  /// An action on what is selected, after the ones the bar has: a button,
  /// which the bar makes a ghost one of its size.
  pub fn action(mut self, button: Button) -> Self {
    self.actions.push(ActionBarAction {
      button,
      destructive: false,
    });
    self
  }

  /// An action that destroys what is selected: drawn in the destructive color.
  pub fn destructive_action(mut self, button: Button) -> Self {
    self.actions.push(ActionBarAction {
      button,
      destructive: true,
    });
    self
  }

  /// What clearing the selection does: the button at the end of the bar,
  /// and Escape where [`escape_clears`](Self::escape_clears). Without it the
  /// bar has no clear button.
  pub fn on_clear(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_clear = Some(Arc::new(handler));
    self
  }

  /// What the clear button is called, to what reads the tree: "Clear selection" by default.
  pub fn clear_label(mut self, label: &str) -> Self {
    self.clear_label = Arc::from(label);
    self
  }

  /// Where the bar stands over what it acts on: at the bottom, or the top.
  pub fn placement(mut self, placement: ActionBarPlacement) -> Self {
    self.placement = placement;
    self
  }

  /// Whether Escape clears the selection while the bar is shown, wherever the
  /// focus is, unless what has the focus took the key itself.
  pub fn escape_clears(mut self, clears: bool) -> Self {
    self.escape_clears = clears;
    self
  }

  /// Styles one part of this bar over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this bar.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the bar. Its clear button has this id with `-clear` after it.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the bar in `theme`. Converting a bar into an element does this
  /// in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = Case::new()
      .variant(self.placement)
      .breakpoint(super::current_breakpoint())
      .state(ActionBarState::Empty, self.count == 0);
    let view = ActionBarView {
      count: self.count,
      text: self
        .text
        .unwrap_or_else(|| Arc::from(format!("{} selected", self.count))),
      actions: self.actions,
      on_clear: self.on_clear,
      placement: self.placement,
      escape_clears: self.escape_clears,
      clear_label: self.clear_label,
      recipe: theme.resolve::<ActionBar>(&case, Some(&self.rules)),
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<ActionBar>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

impl From<ActionBar> for Element {
  fn from(bar: ActionBar) -> Self {
    bar.build(&current_theme())
  }
}

/// An action bar as a renderer sees it: what it says, its actions, the
/// style of its parts, and what clears the selection.
pub struct ActionBarView {
  pub count: usize,
  /// What it says of the selection: "3 selected".
  pub text: Arc<str>,
  pub actions: Vec<ActionBarAction>,
  pub on_clear: Option<Arc<dyn Fn() + Send + Sync>>,
  pub placement: ActionBarPlacement,
  pub escape_clears: bool,
  pub clear_label: Arc<str>,
  /// The style of each part of this bar.
  pub recipe: ResolvedRecipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ActionBarView {
  /// The style of a part, as this bar is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The line between the count, the actions and the clear button.
  pub fn separator(&self) -> Element {
    Separator::vertical()
      .rules(&self.recipe.rules_for(ActionBarPart::Separator, SeparatorPart::Root))
      .build(self.theme())
  }

  /// How many are selected.
  pub fn count(&self) -> Element {
    let part = self.part(ActionBarPart::Count);
    part.text(&self.text).nowrap().part(&part).into()
  }

  /// The action at `index`, a ghost button in the bar's dress.
  pub fn action(&mut self, index: usize) -> Element {
    let action = &self.actions[index];
    let mut rules = self
      .recipe
      .rules_for(ActionBarPart::Action, ButtonPart::Root)
      .extend(&self.recipe.rules_for(ActionBarPart::ActionIcon, ButtonPart::Icon))
      .extend(&self.recipe.rules_for(ActionBarPart::ActionLabel, ButtonPart::Label));
    if action.destructive {
      rules = rules.extend(&self.recipe.rules_for(ActionBarPart::Destructive, ButtonPart::Root));
    }
    action
      .button
      .clone()
      .variant(ButtonVariant::Ghost)
      .rules(&rules)
      .build(self.theme())
  }

  /// The button that clears the selection, if the app said what clearing does.
  pub fn clear(&self) -> Option<Element> {
    let clear = self.on_clear.clone()?;
    let rules = self
      .recipe
      .rules_for(ActionBarPart::Clear, ButtonPart::Root)
      .extend(&self.recipe.rules_for(ActionBarPart::ClearIcon, ButtonPart::Icon));
    let mut button = Button::new(&self.clear_label)
      .icon(icons::CLOSE)
      .icon_only(true)
      .variant(ButtonVariant::Ghost)
      .rules(&rules)
      .on_click(move |_| clear());
    if let Some(id) = &self.id {
      button = button.id(format!("{id}-clear"));
    }
    Some(button.build(self.theme()))
  }

  /// The bar: the count, a line, the actions, a line and the clear button.
  /// It is a toolbar to what reads the tree, called by its count, and one
  /// stop of Tab whose Left and Right move between its buttons.
  pub fn bar(&mut self) -> Row {
    let mut bar = Row::new()
      .part(&self.part(ActionBarPart::Bar))
      .align_items(Alignment::Center)
      .focus_group(FocusGroup::horizontal().one_stop(true))
      .key("action-bar")
      .describe("a11y_role", "toolbar")
      .describe("a11y_name", self.text.clone())
      .describe("a11y_orientation", "horizontal")
      .child(self.count());
    if !self.actions.is_empty() {
      bar = bar.child(self.separator());
    }
    for index in 0..self.actions.len() {
      bar = bar.child(self.action(index));
    }
    if let Some(clear) = self.clear() {
      bar = bar.child(self.separator()).child(clear);
    }
    if let Some(id) = &self.id {
      bar = bar.id(id.clone());
    }
    bar
  }

  /// How the default renderer builds an action bar: the room over what it
  /// acts on, and in it the bar while something is selected. The room keeps
  /// the bar while it leaves.
  pub fn render(mut self) -> Element {
    let justify = match self.placement {
      ActionBarPlacement::Bottom => Justify::End,
      ActionBarPlacement::Top => Justify::Start,
    };
    let mut root = Column::new()
      .part(&self.part(ActionBarPart::Root))
      .presence()
      .justify(justify)
      .align_items(Alignment::Center)
      .overflow_visible()
      .hit_test(HitTestBehavior::ContentOnly);
    if self.escape_clears
      && self.count > 0
      && let Some(clear) = self.on_clear.clone()
    {
      root = root.on_key_down(move |event: KeyboardEvent| {
        let plain = !event.ctrl && !event.alt && !event.meta && !event.shift;
        if plain && event.key == "Escape" && !event.default_prevented() {
          event.prevent_default();
          clear();
        }
      });
    }
    if self.count > 0 {
      root = root.child(self.bar());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }
}

impl Themed for ActionBar {
  const NAME: &'static str = "action_bar";
  type Renderer = ActionBarRenderer;

  /// The design's action bar (`pencil/lurq-design-system.pen`, Action Bar): a
  /// bar 44 high of the popover's color, round at its ends, with an edge of 1
  /// in the border's color and the large shadow, padded 16 before its count
  /// and 6 after its clear button, its parts 4 apart. The count is 13/600; a
  /// line of 20 with 6 on either side sets it apart from the actions, which
  /// are round, 32 high, padded 10, with an icon of 14 and a label of 13, 6
  /// apart; the one that destroys is in the destructive color. The clear
  /// button is a round 32 with a cross of 15. It stands 16 from the bottom of
  /// what it acts on, and comes in with a fade and a move of 16 from the edge.
  fn recipe() -> Recipe {
    use ActionBarPart::*;
    use PaletteColor::{Destructive as Danger, Foreground, Popover, PopoverForeground};
    Recipe::new()
      .base(Root, |s| {
        s.width(Length::Fill)
          .height(Length::Fill)
          .padding_x(16.0)
          .padding_y(16.0)
      })
      .base(Bar, |s| {
        s.height(44.0)
          .padding_start(16.0)
          .padding_end(6.0)
          .gap(4.0)
          .radius(RadiusSize::Full)
          .background(Popover)
          .foreground(PopoverForeground)
          .border(Border::inside(BorderSize::Sm, PaletteColor::Border))
          .shadow(ShadowStyle::Lg)
      })
      .variant(ActionBarPlacement::Bottom, Bar, |s| {
        s.enter(PartAnimation::new(
          ENTER_FROM_BELOW,
          MotionDuration::Base,
          MotionEasing::Enter,
        ))
        .exit(PartAnimation::new(
          LEAVE_DOWNWARD,
          MotionDuration::Fast,
          MotionEasing::Exit,
        ))
      })
      .variant(ActionBarPlacement::Top, Bar, |s| {
        s.enter(PartAnimation::new(
          ENTER_FROM_ABOVE,
          MotionDuration::Base,
          MotionEasing::Enter,
        ))
        .exit(PartAnimation::new(
          LEAVE_UPWARD,
          MotionDuration::Fast,
          MotionEasing::Exit,
        ))
      })
      .base(Count, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
      })
      .base(Separator, |s| s.padding_x(6.0).height(20.0))
      .base(Action, |s| {
        s.height(32.0)
          .padding_x(10.0)
          .gap(6.0)
          .radius(RadiusSize::Full)
          .foreground(Foreground)
      })
      .base(ActionIcon, |s| s.square(IconSize::Sm))
      .base(ActionLabel, |s| s.font_size(13.0).weight(FontWeight::Normal))
      .base(Destructive, |s| s.foreground(Danger))
      .base(Clear, |s| s.square(32.0).padding_x(0.0).radius(RadiusSize::Full))
      .base(ClearIcon, |s| s.square(15.0))
  }
}
