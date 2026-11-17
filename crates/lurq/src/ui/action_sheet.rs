//! An action sheet: a phone's choice among a few actions, risen from the bottom of the screen over a scrim.

use std::sync::Arc;

use super::{
  MenuItem, names,
  part_style::PartAnimation,
  recipe::{Case, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{FADE_IN, FADE_OUT, fading, pressable},
  sheet::{SHEET_IN_FROM_BOTTOM, SHEET_OUT_TO_BOTTOM},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    events::MouseEvent,
    theme::{BorderSize, MotionDuration, MotionEasing, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Modal, Root, Row, Stack},
  core::Signal,
  layout::{
    Alignment,
    layout_kind::Justify,
    text_style::{FontWeight, TextAlign},
  },
  node::{CursorIcon, Element, FocusGroup, border::Border, dimension::Dimension},
};

/// The parts of an action sheet a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionSheetPart {
  /// What covers the screen under the sheet.
  Scrim,
  /// The sheet: its options, and Cancel under them.
  Root,
  /// The card of the title and the options.
  Options,
  /// What the sheet is about, over the options.
  Title,
  /// An option.
  Option,
  /// The words of an option.
  Label,
  /// The card of Cancel.
  Cancel,
  /// The word of Cancel.
  CancelLabel,
}

names!(ActionSheetPart {
  Scrim => "scrim",
  Root => "root",
  Options => "options",
  Title => "title",
  Option => "option",
  Label => "label",
  Cancel => "cancel",
  CancelLabel => "cancel_label",
});

/// The states of an action sheet's option a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActionSheetState {
  /// The option destroys something: its words are in the destructive role.
  Destructive,
  /// The option takes nothing.
  Disabled,
}

names!(ActionSheetState {
  Destructive => "destructive",
  Disabled => "disabled",
});

/// What builds an action sheet's nodes: a theme's in place of the default one.
pub type ActionSheetRenderer = Arc<dyn Fn(ActionSheetView) -> Element + Send + Sync>;

type Action = Arc<dyn Fn() + Send + Sync>;

/// An action sheet: on a phone, the actions on something, risen from the bottom of the screen over a scrim, and
/// Cancel under them.
///
/// ```
/// use lurq::{core::Signal, ui::{ActionSheet, MenuItem}};
///
/// let open = Signal::new(false);
/// let sheet = ActionSheet::new(open.clone(), [
///   MenuItem::new("Save to Photos").on_select(|| {}),
///   MenuItem::new("Copy"),
///   MenuItem::new("Delete photo").destructive(true),
/// ])
/// .title("Lake at dusk");
/// ```
///
/// It is open while its signal says so. A press of an option does what it does and closes the sheet; so do
/// Cancel, a press on the scrim and Escape. The options are [`MenuItem`]s: their words, whether they destroy
/// something and whether they are disabled are shown, and what they are told to do when chosen is done; an icon,
/// a shortcut and a submenu are not shown.
#[derive(Clone)]
pub struct ActionSheet {
  open: Signal<bool>,
  items: Vec<MenuItem>,
  title: Option<Arc<str>>,
  cancel_label: Arc<str>,
  on_dismiss: Vec<Action>,
  inline: bool,
  /// How far the screen's lower edge the system draws over reaches: what the sheet keeps clear of.
  below: f32,
  states: Vec<Name>,
  rules: Recipe,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ActionSheet {
  /// A sheet of `items`, open while `open` says so.
  pub fn new(open: Signal<bool>, items: impl IntoIterator<Item = MenuItem>) -> Self {
    Self {
      open,
      items: items.into_iter().collect(),
      title: None,
      cancel_label: super::current_locale().common.cancel.clone(),
      on_dismiss: Vec::new(),
      inline: false,
      below: 0.0,
      states: Vec::new(),
      rules: Recipe::new(),
      id: None,
      key: None,
    }
  }

  /// One more option, after the others.
  pub fn item(mut self, item: MenuItem) -> Self {
    self.items.push(item);
    self
  }

  /// What the sheet is about, over its options, in the muted role.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// What Cancel says: the locale's word ("Cancel" in English) unless told.
  pub fn cancel_label(mut self, label: &str) -> Self {
    self.cancel_label = Arc::from(label);
    self
  }

  /// Called when it is closed without an option: Cancel, the scrim, Escape.
  pub fn on_dismiss(mut self, handler: impl Fn() + Send + Sync + 'static) -> Self {
    self.on_dismiss.push(Arc::new(handler));
    self
  }

  /// The sheet where it is declared, open or not, without the scrim and not over the screen: for a gallery, a
  /// screenshot, a test.
  pub fn inline(mut self, inline: bool) -> Self {
    self.inline = inline;
    self
  }

  /// Turns a state a recipe's rules name on or off.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// The style of one part, over the theme's.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Rules over the theme's.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// The id of the sheet; its options are `<id>-option-<n>` and Cancel `<id>-cancel`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the sheet in `theme`. Converting it into an element does this in the theme of what is rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let case = |state: Option<ActionSheetState>| {
      let case = Case {
        variant: None,
        size: None,
        density: None,
        breakpoint: super::current_breakpoint(),
        states: self.states.clone(),
      };
      match state {
        Some(state) => case.state(state, true),
        None => case,
      }
    };
    let view = ActionSheetView {
      recipe: theme.resolve::<ActionSheet>(&case(None), Some(&self.rules)),
      destructive: theme.resolve::<ActionSheet>(&case(Some(ActionSheetState::Destructive)), Some(&self.rules)),
      disabled: theme.resolve::<ActionSheet>(&case(Some(ActionSheetState::Disabled)), Some(&self.rules)),
      sheet: self,
    };
    match theme.renderer::<ActionSheet>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

/// A sheet in a tree is its component, declared: what reads how far the screen's lower edge the system draws
/// over reaches, which the sheet keeps clear of. A sheet drawn in place needs none of it.
impl From<ActionSheet> for Element {
  fn from(sheet: ActionSheet) -> Self {
    if sheet.inline {
      return sheet.build(&current_theme());
    }
    match sheet.key.clone() {
      Some(key) => Element::component_keyed::<ActionSheetHost>(key, ActionSheetProps(sheet)),
      None => Element::component::<ActionSheetHost>(ActionSheetProps(sheet)),
    }
  }
}

/// An action sheet to mount.
#[derive(Clone)]
struct ActionSheetProps(ActionSheet);

/// Its items and its handlers are made anew by every render of what holds it.
impl PartialEq for ActionSheetProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for ActionSheetProps {}

/// The component of an action sheet, which keeps nothing: it asks the window for its safe area.
struct ActionSheetHost;

impl crate::app::component::Component for ActionSheetHost {
  type Props = ActionSheetProps;

  fn create(_: &mut crate::app::ctx::Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut crate::app::ctx::Ctx) -> impl Into<Element> {
    let mut sheet = ctx.props::<ActionSheetProps>().0.clone();
    sheet.below = ctx.safe_area().bottom;
    sheet.build(&current_theme())
  }
}

/// An action sheet as a renderer sees it: what it was given, and the style of its parts.
pub struct ActionSheetView {
  /// The style of each part.
  pub recipe: ResolvedRecipe,
  /// The style of each part of an option that destroys something, and of one that is disabled.
  destructive: ResolvedRecipe,
  disabled: ResolvedRecipe,
  sheet: ActionSheet,
}

impl ActionSheetView {
  /// The style of a part.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  fn id(&self, part: &str) -> Option<String> {
    self.sheet.id.as_ref().map(|id| format!("{id}-{part}"))
  }

  /// What closes the sheet without an option, and tells the app.
  pub fn dismisser(&self) -> impl Fn() + Send + Sync + Clone + 'static {
    let (open, told) = (self.sheet.open.clone(), self.sheet.on_dismiss.clone());
    move || {
      open.set(false);
      told.iter().for_each(|tell| tell());
    }
  }

  /// The card of the title and the options. The arrow keys move among the options.
  pub fn options(&self) -> Element {
    let full = Dimension::Pct(100.0);
    let mut options = Column::new()
      .part(&self.part(ActionSheetPart::Options))
      .width(full)
      .clip()
      .focus_group(FocusGroup::vertical());
    if let Some(title) = &self.sheet.title {
      let part = self.part(ActionSheetPart::Title);
      options = options.child(
        Row::new()
          .part(&part)
          .width(full)
          .justify(Justify::Center)
          .child(part.text(title).text_align(TextAlign::Center).part(&part)),
      );
    }
    for (at, item) in self.sheet.items.iter().enumerate() {
      let recipe = if item.is_disabled() {
        &self.disabled
      } else if item.is_destructive() {
        &self.destructive
      } else {
        &self.recipe
      };
      let (part, label) = (
        recipe.part(ActionSheetPart::Option),
        recipe.part(ActionSheetPart::Label),
      );
      let (handlers, open) = (item.select_handlers(), self.sheet.open.clone());
      let mut option = pressable(&part, item.is_disabled())
        .width(full)
        .align_items(Alignment::Center)
        .describe("a11y_role", "button")
        .describe("a11y_name", item.label_of().clone())
        .child(label.text(item.label_of()).part(&label))
        .on_click(move |event: MouseEvent| {
          event.stop_propagation();
          open.set(false);
          handlers.iter().for_each(|handler| handler());
        });
      if let Some(id) = self.id(&format!("option-{at}")) {
        option = option.id(id);
      }
      options = options.child(option);
    }
    options.into()
  }

  /// Cancel, in a card of its own under the options.
  pub fn cancel(&self) -> Element {
    let (part, label) = (
      self.part(ActionSheetPart::Cancel),
      self.part(ActionSheetPart::CancelLabel),
    );
    let close = self.dismisser();
    let mut cancel = pressable(&part, false)
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .describe("a11y_role", "button")
      .describe("a11y_name", self.sheet.cancel_label.clone())
      .child(label.text(&self.sheet.cancel_label).part(&label))
      .on_click(move |event: MouseEvent| {
        event.stop_propagation();
        close();
      });
    if let Some(id) = self.id("cancel") {
      cancel = cancel.id(id);
    }
    cancel.into()
  }

  /// How the default renderer builds an action sheet: over the screen while it is open, the options and Cancel at
  /// its bottom on the scrim, which a press closes.
  pub fn render(self) -> Element {
    let full = Dimension::Pct(100.0);
    let mut sheet = Column::new()
      .part(&self.part(ActionSheetPart::Root))
      .width(full)
      .describe("a11y_role", "dialog")
      // A press in the sheet is not one on the scrim.
      .on_click(|event: MouseEvent| event.stop_propagation())
      .child(self.options())
      .child(self.cancel());
    if let Some(id) = &self.sheet.id {
      sheet = sheet.id(id.clone());
    }
    if let Some(title) = &self.sheet.title {
      sheet = sheet.describe("a11y_name", title.clone());
    }
    if self.sheet.inline {
      return sheet.into();
    }
    // Clear of what the system draws over the screen's lower edge, its own 8 over that.
    let sheet = Column::new().width(full).padding_bottom(self.sheet.below).child(sheet);
    let close = self.dismisser();
    let scrim = Stack::new().size(full, full).part(&self.part(ActionSheetPart::Scrim));
    let holder = Column::new()
      .size(full, full)
      .align_items(Alignment::Center)
      .justify(Justify::End)
      .on_click(move |_: MouseEvent| close())
      .child(sheet);
    let mut stack = Stack::new().size(full, full).child(scrim).child(holder);
    if let Some(key) = &self.sheet.key {
      stack = stack.key(key.clone());
    }
    let mut modal = Modal::new(stack)
      .open(self.sheet.open.clone())
      .target(Root)
      .dismiss_on_escape(true)
      .takes_focus(true);
    for handler in &self.sheet.on_dismiss {
      let handler = handler.clone();
      modal = modal.on_dismiss(move || handler());
    }
    modal.into()
  }
}

impl crate::ui::Styled for ActionSheet {
  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for ActionSheet {
  const NAME: &'static str = "action_sheet";
  type Renderer = ActionSheetRenderer;

  fn parts() -> crate::ui::Declared {
    ActionSheetPart::declared()
  }

  fn states() -> crate::ui::Declared {
    ActionSheetState::declared()
  }

  /// The design's action sheet (`pencil/lurq-design-system.pen`, Mobile/Action Sheet): on the scrim, 8 in from the
  /// screen's sides and bottom, a card in `Card` with a line of `Border` and the large corners: the title 12 over
  /// and under, 16 in, 13 in `MutedForeground`; options of 56, a line over each, their words 17 in the middle in
  /// `Foreground`, or in `Destructive`; 8 under it, Cancel alike in a card of its own, 17 semibold. It rises from the
  /// bottom and the scrim fades in.
  fn recipe() -> Recipe {
    use ActionSheetPart::{Cancel, CancelLabel, Label, Options, Root, Title};
    use PaletteColor::{Border as Line, Card, Destructive, Foreground, MutedForeground, Scrim as Shade, StateHover};
    let card = |s: super::PartStyle| {
      s.background(Card)
        .radius(RadiusSize::Xl)
        .border(Border::inside(BorderSize::Sm, Line))
    };
    let entering = |keyframes| PartAnimation::new(keyframes, MotionDuration::Slow, MotionEasing::Enter);
    let leaving = |keyframes| PartAnimation::new(keyframes, MotionDuration::Base, MotionEasing::Exit);
    Recipe::new()
      .base(ActionSheetPart::Scrim, |s| {
        s.background(Shade).enter(entering(FADE_IN)).exit(leaving(FADE_OUT))
      })
      .base(Root, |s| {
        s.gap(8.0)
          .padding_x(8.0)
          .padding_bottom(8.0)
          .enter(entering(SHEET_IN_FROM_BOTTOM))
          .exit(leaving(SHEET_OUT_TO_BOTTOM))
      })
      .base(Options, card)
      .base(Title, |s| {
        s.padding_x(16.0)
          .padding_y(12.0)
          .typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      .base(ActionSheetPart::Option, |s| {
        fading(
          s.height(56.0)
            .top_line(Border::inside(BorderSize::Sm, Line))
            .cursor(CursorIcon::Pointer),
        )
      })
      .hovered(ActionSheetPart::Option, |s| s.veil(StateHover))
      .focus_visible(ActionSheetPart::Option, |s| s.focus_ring())
      .base(Label, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(17.0)
          .foreground(Foreground)
      })
      .state(ActionSheetState::Destructive, Label, |s| s.foreground(Destructive))
      .state(ActionSheetState::Disabled, ActionSheetPart::Option, |s| {
        s.opacity(0.5).cursor(CursorIcon::Default)
      })
      .base(Cancel, |s| fading(card(s).height(56.0).cursor(CursorIcon::Pointer)))
      .hovered(Cancel, |s| s.veil(StateHover))
      .focus_visible(Cancel, |s| s.focus_ring())
      .base(CancelLabel, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(17.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
      })
  }
}
