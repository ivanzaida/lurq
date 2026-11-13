//! Steps: the stages of a process in order, with one current.

use std::sync::Arc;

use super::{
  icons, names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, FocusGroup, border},
};

/// The parts of steps a recipe styles. Each step's parts are styled in the
/// states of that step: done, current or to come, in error, pressable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StepsPart {
  /// The steps and the lines between them.
  Root,
  /// One step: its mark and its text. What is pressed and focused where steps are pressed.
  Step,
  /// The round mark of a step: its number, a check once it is done, or its icon.
  Marker,
  /// The number in the mark.
  Number,
  /// The icon in the mark: the check of a step done, the cross of one in error, or the step's own.
  Icon,
  /// The title and the description, one under the other.
  Body,
  /// What the step is called.
  Title,
  /// A line under the title that says more.
  Description,
  /// In a column, the mark with the line that runs down from it to the next step.
  Rail,
  /// What holds the line after a step: as high as the step's mark in a row, as wide as it in a column.
  Connector,
  /// The line after a step, to the next: drawn up to the current step in the color of what is done.
  Line,
}

names!(StepsPart {
  Root => "root",
  Step => "step",
  Marker => "marker",
  Number => "number",
  Icon => "icon",
  Body => "body",
  Title => "title",
  Description => "description",
  Rail => "rail",
  Connector => "connector",
  Line => "line",
});

/// The states of a step a recipe styles, besides what the pointer and the
/// keyboard do to it. A step is in exactly one of `Done`, `Current` and
/// `Upcoming`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StepsState {
  /// A step before the current one.
  Done,
  /// The step the process is at.
  Current,
  /// A step after the current one.
  Upcoming,
  /// A step the app says has gone wrong.
  Error,
  /// A step a press of which selects it.
  Pressable,
  /// The steps take no input.
  Disabled,
}

names!(StepsState {
  Done => "done",
  Current => "current",
  Upcoming => "upcoming",
  Error => "error",
  Pressable => "pressable",
  Disabled => "disabled",
});

/// Which way the steps run: the variant of their recipe.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StepsOrientation {
  /// In a row, a line between each step and the next.
  #[default]
  Horizontal,
  /// In a column, a line running down from each step's mark to the next.
  Vertical,
}

names!(StepsOrientation {
  Horizontal => "horizontal",
  Vertical => "vertical",
});

/// One stage of a process: a title, and a description, an icon in place of
/// its number, or an error where it has them.
///
/// ```
/// use lurq::ui::{Step, icons};
///
/// let account = Step::new("Account").description("Your name and email");
/// let billing = Step::new("Billing").icon(icons::USER).error(true);
/// let plain: Step = "Finish".into();
/// ```
#[derive(Clone, Debug)]
pub struct Step {
  pub title: Arc<str>,
  pub description: Option<Arc<str>>,
  pub icon: Option<Name>,
  pub error: bool,
}

impl Step {
  /// A step called `title`.
  pub fn new(title: impl AsRef<str>) -> Self {
    Self {
      title: Arc::from(title.as_ref()),
      description: None,
      icon: None,
      error: false,
    }
  }

  /// A line under the title that says more.
  pub fn description(mut self, description: impl AsRef<str>) -> Self {
    self.description = Some(Arc::from(description.as_ref()));
    self
  }

  /// An icon of the theme's set in place of the step's number, until the step is done.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether the step has gone wrong: its mark and its title say so, whatever else it is.
  pub fn error(mut self, error: bool) -> Self {
    self.error = error;
    self
  }
}

impl From<&str> for Step {
  fn from(title: &str) -> Self {
    Self::new(title)
  }
}

impl From<String> for Step {
  fn from(title: String) -> Self {
    Self::new(title)
  }
}

/// Which step is current: an index the app holds, or a signal a press sets.
#[derive(Clone)]
pub enum CurrentStep {
  Index(usize),
  Signal(Signal<usize>),
}

impl From<usize> for CurrentStep {
  fn from(index: usize) -> Self {
    Self::Index(index)
  }
}

impl From<Signal<usize>> for CurrentStep {
  fn from(signal: Signal<usize>) -> Self {
    Self::Signal(signal)
  }
}

impl From<&Signal<usize>> for CurrentStep {
  fn from(signal: &Signal<usize>) -> Self {
    Self::Signal(signal.clone())
  }
}

/// What builds the nodes of steps: a theme's in place of the default one.
pub type StepsRenderer = Arc<dyn Fn(StepsView) -> Element + Send + Sync>;

type Select = Arc<dyn Fn(usize) + Send + Sync>;

/// The stages of a process in order, with one current: those before it are
/// done, those after it are to come. A wizard's account, billing and finish.
///
/// ```
/// use lurq::{core::Signal, ui::{Step, Steps, StepsOrientation}};
///
/// # fn steps(at: Signal<usize>) {
/// // Read, not operated: the app moves the current step itself.
/// let progress = Steps::new(1, ["Account", "Workspace", "Finish"]);
/// // A wizard that lets a person go back to a step that is done: a press sets the signal.
/// let wizard = Steps::new(&at, [
///   Step::new("Account").description("Name and email"),
///   Step::new("Billing").description("Card and address"),
///   Step::new("Finish"),
/// ])
/// .orientation(StepsOrientation::Vertical)
/// .on_select(|index| println!("back to step {index}"));
/// # }
/// ```
///
/// The current step is the app's: an index, or a `Signal<usize>` that a
/// press of a step sets. An index past the last step has every step done.
/// Steps are pressed only where the app says so, with [`Steps::on_select`]
/// or [`Steps::pressable`]: then a step that is done is pressed to go back
/// to it, and a step to come only with [`Steps::ahead`]. Pressed steps are
/// one stop of Tab, at the current step, and the arrow keys of their
/// orientation move between them.
pub struct Steps {
  current: CurrentStep,
  steps: Vec<Step>,
  orientation: StepsOrientation,
  pressable: bool,
  ahead: bool,
  disabled: bool,
  label: Option<Arc<str>>,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_select: Vec<Select>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl Steps {
  /// The steps of a process, of which the one at `current` is current.
  pub fn new(current: impl Into<CurrentStep>, steps: impl IntoIterator<Item = impl Into<Step>>) -> Self {
    Self {
      current: current.into(),
      steps: steps.into_iter().map(Into::into).collect(),
      orientation: StepsOrientation::default(),
      pressable: false,
      ahead: false,
      disabled: false,
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_select: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// Which way the steps run: in a row, or in a column.
  pub fn orientation(mut self, orientation: StepsOrientation) -> Self {
    self.orientation = orientation;
    self
  }

  /// Whether a step that is done is pressed to go back to it. A handler
  /// given with [`Steps::on_select`] says so too; this is for steps bound to
  /// a signal that need no handler.
  pub fn pressable(mut self, pressable: bool) -> Self {
    self.pressable = pressable;
    self
  }

  /// Whether a step to come is pressed too, where steps are pressed: a
  /// process whose steps can be taken in any order.
  pub fn ahead(mut self, ahead: bool) -> Self {
    self.ahead = ahead;
    self
  }

  /// Whether the steps take no input: none is pressed, and they are dimmed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What the steps are called to what reads the tree: "Checkout".
  pub fn label(mut self, label: impl AsRef<str>) -> Self {
    self.label = Some(Arc::from(label.as_ref()));
    self
  }

  /// Puts the steps in a state the theme's recipe names, or takes them out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Called on a press of a step with its index, from 0. Makes the steps
  /// that are done pressable; where the current step is a signal, the press
  /// sets it first.
  pub fn on_select(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
    self.on_select.push(Arc::new(handler));
    self
  }

  /// Styles one part of these steps over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for these steps.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the step the focus would enter at as it is under the pointer,
  /// held down or focused from the keyboard, with nothing pointing at it: for
  /// a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the whole. Each step has the id `<id>-<index>`, its mark
  /// `<id>-<index>-marker`, and the line after it `<id>-<index>-line`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The case of the whole, or of one step: its state, whether it is in error and whether it is pressed.
  fn case(&self, step: Option<(&Step, StepsState, bool)>) -> Case {
    let mut case = Case {
      variant: Some(self.orientation.into()),
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(StepsState::Disabled, self.disabled);
    if let Some((step, status, pressable)) = step {
      case = case
        .state(status, true)
        .state(StepsState::Error, step.error)
        .state(StepsState::Pressable, pressable);
    }
    case
  }

  /// Builds the steps in `theme`. Converting steps into an element does this
  /// in the theme of what is being rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let current = match &self.current {
      CurrentStep::Index(index) => *index,
      CurrentStep::Signal(signal) => signal.get(),
    };
    let select: Option<Select> = ((self.pressable || !self.on_select.is_empty()) && !self.disabled).then(|| {
      let handlers = self.on_select.clone();
      let signal = match &self.current {
        CurrentStep::Signal(signal) => Some(signal.clone()),
        CurrentStep::Index(_) => None,
      };
      Arc::new(move |index: usize| {
        if let Some(signal) = &signal
          && signal.get() != index
        {
          signal.set(index);
        }
        for handler in &handlers {
          handler(index);
        }
      }) as Select
    });
    let pressable = |index: usize| select.is_some() && index != current && (index < current || self.ahead);
    let steps: Vec<StepView> = self
      .steps
      .iter()
      .enumerate()
      .map(|(index, step)| {
        let status = status_of(index, current);
        let pressable = pressable(index);
        StepView {
          step: step.clone(),
          status,
          pressable,
          recipe: theme.resolve::<Steps>(&self.case(Some((step, status, pressable))), Some(&self.rules)),
        }
      })
      .collect();
    let view = StepsView {
      recipe: theme.resolve::<Steps>(&self.case(None), Some(&self.rules)),
      steps,
      current,
      orientation: self.orientation,
      disabled: self.disabled,
      label: self.label,
      forced: self.forced,
      select,
      id: self.id,
      key: self.key,
    };
    match theme.renderer::<Steps>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }
}

fn status_of(index: usize, current: usize) -> StepsState {
  match index.cmp(&current) {
    std::cmp::Ordering::Less => StepsState::Done,
    std::cmp::Ordering::Equal => StepsState::Current,
    std::cmp::Ordering::Greater => StepsState::Upcoming,
  }
}

impl From<Steps> for Element {
  fn from(steps: Steps) -> Self {
    steps.build(&current_theme())
  }
}

/// One step as a renderer sees it: what the app gave it, what it is, and the
/// style of its parts in its states.
#[derive(Clone)]
pub struct StepView {
  pub step: Step,
  /// `Done`, `Current` or `Upcoming`.
  pub status: StepsState,
  /// Whether a press of it selects it.
  pub pressable: bool,
  /// The style of each part of this step.
  pub recipe: ResolvedRecipe,
}

/// Steps as a renderer sees them: what they were given, the style of their
/// parts step by step, and their behavior ready to be dressed.
pub struct StepsView {
  pub steps: Vec<StepView>,
  /// The index of the current step: past the last where every step is done.
  pub current: usize,
  pub orientation: StepsOrientation,
  pub disabled: bool,
  pub label: Option<Arc<str>>,
  /// The style of the parts of the whole: its root.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  select: Option<Select>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl StepsView {
  /// The style of a part of the whole, as it is to be drawn.
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// The style of a part of the step at `index`, in that step's states.
  pub fn step_part(&self, index: usize, part: impl Into<Name>) -> ResolvedPart {
    let forced = (Some(index) == self.entry()).then_some(self.forced).flatten();
    self.steps[index].recipe.part(part).forced(forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// Whether any step is pressed: the steps are then operated, not only read.
  pub fn interactive(&self) -> bool {
    self.select.is_some() && self.steps.iter().any(|step| step.pressable)
  }

  /// The step Tab enters the steps at: the current one, or where none is,
  /// the last that is pressed. `None` where no step is pressed.
  pub fn entry(&self) -> Option<usize> {
    if !self.interactive() {
      return None;
    }
    if self.current < self.steps.len() {
      return Some(self.current);
    }
    self.steps.iter().rposition(|step| step.pressable)
  }

  /// What the step at `index` is called to what reads the tree: its place,
  /// its title and its state. "Step 2 of 4, Billing, current".
  pub fn step_name(&self, index: usize) -> String {
    let step = &self.steps[index];
    let status = match step.status {
      StepsState::Done => "done",
      StepsState::Current => "current",
      _ => "to come",
    };
    let error = if step.step.error { ", error" } else { "" };
    format!(
      "Step {} of {}, {}, {status}{error}",
      index + 1,
      self.steps.len(),
      step.step.title
    )
  }

  fn child_id(&self, index: usize, suffix: &str) -> Option<String> {
    self.id.as_ref().map(|id| format!("{id}-{index}{suffix}"))
  }

  /// The mark of the step at `index`: a cross where it is in error, a check
  /// where it is done, its icon or its number where it is not.
  pub fn marker(&self, index: usize) -> Row {
    let step = &self.steps[index];
    let icon = if step.step.error {
      Some(Name::new(icons::CLOSE))
    } else if step.status == StepsState::Done {
      Some(Name::new(icons::CHECK))
    } else {
      step.step.icon.clone()
    };
    let mut marker = Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .part(&self.step_part(index, StepsPart::Marker));
    marker = match icon {
      Some(name) => marker.child(glyph(&self.step_part(index, StepsPart::Icon), name, false)),
      None => {
        let number = self.step_part(index, StepsPart::Number);
        marker.child(number.text(&(index + 1).to_string()).nowrap())
      }
    };
    if let Some(id) = self.child_id(index, "-marker") {
      marker = marker.id(id);
    }
    marker
  }

  /// The title of the step at `index`, with its description under it.
  pub fn body(&self, index: usize) -> Column {
    let step = &self.steps[index].step;
    let title = self.step_part(index, StepsPart::Title);
    let mut body = Column::new().part(&self.step_part(index, StepsPart::Body)).child(
      Row::new()
        .align_items(Alignment::Center)
        .part(&title)
        .child(title.text(&step.title).nowrap()),
    );
    if let Some(description) = &step.description {
      let part = self.step_part(index, StepsPart::Description);
      body = body.child(Row::new().part(&part).child(part.text(description)));
    }
    body
  }

  /// The line after the step at `index`, in what holds it: across the row,
  /// or down the column from the step's mark.
  pub fn connector(&self, index: usize) -> Element {
    let line = self.step_part(index, StepsPart::Line);
    let connector = self.step_part(index, StepsPart::Connector);
    let id = self.child_id(index, "-line");
    match self.orientation {
      StepsOrientation::Horizontal => {
        let mut line = Row::new().flex(1.0).part(&line);
        if let Some(id) = id {
          line = line.id(id);
        }
        Row::new()
          .flex(1.0)
          .align_items(Alignment::Center)
          .part(&connector)
          .child(line)
          .into()
      }
      StepsOrientation::Vertical => {
        let mut line = Column::new().flex(1.0).part(&line);
        if let Some(id) = id {
          line = line.id(id);
        }
        Column::new()
          .flex(1.0)
          .align_items(Alignment::Center)
          .part(&connector)
          .child(line)
          .into()
      }
    }
  }

  /// The step at `index` with `content` in it, and its behavior: where it is
  /// pressed, the element that is pressed and focused; where the steps are
  /// operated, the current step is focused too, as where Tab enters them.
  /// To what reads the tree it is an item of the list, called by
  /// [`StepsView::step_name`].
  pub fn step(&self, index: usize, content: impl IntoIterator<Item = Element>) -> Element {
    let step = &self.steps[index];
    let part = self.step_part(index, StepsPart::Step);
    let align = match self.orientation {
      StepsOrientation::Horizontal => Alignment::Start,
      // The rail is as high as the body, so that its line runs down to the next step.
      StepsOrientation::Vertical => Alignment::Stretch,
    };
    let name = self.step_name(index);
    let id = self.child_id(index, "");
    let current = step.status == StepsState::Current;
    let focused = step.pressable || (current && self.interactive());
    macro_rules! dressed {
      ($holder:expr) => {{
        let mut holder = $holder
          .align_items(align)
          .describe("a11y_name", name)
          .with_children(content);
        if current {
          holder = holder.describe("a11y_current", "step");
        }
        if let Some(id) = id {
          holder = holder.id(id);
        }
        holder.into()
      }};
    }
    if focused {
      let mut button = pressable(&part, false)
        .justify(Justify::Start)
        .describe("a11y_role", if step.pressable { "button" } else { "listitem" })
        .focus_entry(Some(index) == self.entry());
      if step.pressable
        && let Some(select) = self.select.clone()
      {
        button = button.on_click(move |_| select(index));
      }
      dressed!(button)
    } else {
      dressed!(Row::new().part(&part).describe("a11y_role", "listitem"))
    }
  }

  /// How the default renderer builds steps: in a row, each step's mark and
  /// text with a line to the next; in a column, each step's mark with the
  /// line down from it, and its text beside them.
  pub fn render(self) -> Element {
    let count = self.steps.len();
    let mut children: Vec<Element> = Vec::with_capacity(count * 2);
    for index in 0..count {
      let last = index + 1 == count;
      match self.orientation {
        StepsOrientation::Horizontal => {
          children.push(self.step(index, [self.marker(index).into(), self.body(index).into()]));
          if !last {
            children.push(self.connector(index));
          }
        }
        StepsOrientation::Vertical => {
          let mut rail = Column::new()
            .align_items(Alignment::Center)
            .part(&self.step_part(index, StepsPart::Rail))
            .child(self.marker(index));
          if !last {
            rail = rail.child(self.connector(index));
          }
          children.push(self.step(index, [rail.into(), self.body(index).into()]));
        }
      }
    }
    let keys = match self.orientation {
      StepsOrientation::Horizontal => FocusGroup::horizontal(),
      StepsOrientation::Vertical => FocusGroup::vertical(),
    }
    .one_stop(true);
    let root = self.part(StepsPart::Root);
    macro_rules! whole {
      ($holder:expr) => {{
        // A ring around a step at either end is drawn outside the whole.
        let mut holder = $holder
          .part(&root)
          .overflow_visible()
          .describe("a11y_role", "list")
          .with_children(children);
        if self.interactive() {
          holder = holder.focus_group(keys);
        }
        if let Some(label) = self.label.clone() {
          holder = holder.describe("a11y_name", label);
        }
        if let Some(id) = self.id.clone() {
          holder = holder.id(id);
        }
        if let Some(key) = self.key.clone() {
          holder = holder.key(key);
        }
        holder.into()
      }};
    }
    match self.orientation {
      StepsOrientation::Horizontal => whole!(Row::new().align_items(Alignment::Start)),
      StepsOrientation::Vertical => whole!(Column::new()),
    }
  }
}

impl Themed for Steps {
  const NAME: &'static str = "steps";
  type Renderer = StepsRenderer;

  /// The design's steps (`pencil/lurq-design-system.pen`, Steps): a mark of
  /// 24, round, in the primary color with a check of 14 once a step is done
  /// and with its number (12, medium) while it is current, and with an edge
  /// in the color of an input and the number muted while it is to come; the
  /// title is 8 beside the mark in the label's type, muted while the step is
  /// to come. A line of 1 runs 12 from one step to the next, in the primary
  /// color up to the current step and in the border's after it. A step is
  /// padded 4 all round, for the ring of its focus and the veil of the
  /// pointer where it is pressed, and the whole is 8 apart from its lines,
  /// so the line is 12 from the mark and the title as the board has it.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use StepsOrientation::{Horizontal, Vertical};
    use StepsPart::{Body, Connector, Description, Icon, Line, Marker, Number, Rail, Root, Step, Title};
    use StepsState::{Current, Disabled, Done, Error, Pressable, Upcoming};
    let filled = |s: super::PartStyle| s.background(Primary).foreground(PrimaryForeground);
    Recipe::new()
      .variant(Horizontal, Root, |s| s.width(Length::Fill).gap(8.0))
      .base(Step, |s| {
        fading(s.gap(8.0).padding_x(4.0).padding_y(4.0).radius(RadiusSize::Md))
      })
      .base(Marker, |s| s.square(24.0).radius(RadiusSize::Full))
      .base(Number, |s| s.typography(TypographyStyle::Caption).weight(FontWeight::Medium))
      .base(Icon, |s| s.square(14.0))
      .base(Body, |s| s.gap(2.0))
      // As high as the mark, so that the title stands level with its middle and a description goes under both.
      .base(Title, |s| s.height(24.0).typography(TypographyStyle::Label).foreground(Foreground))
      .base(Description, |s| s.typography(TypographyStyle::Caption).foreground(MutedForeground))
      .base(Rail, |s| s.gap(4.0))
      // In a row, as high as a step with its padding, so that the line is level with the middle of the mark.
      .variant(Horizontal, Connector, |s| s.height(32.0))
      // In a column, as wide as the mark, and long enough to be seen between two steps without descriptions.
      .variant(Vertical, Connector, |s| s.width(24.0).height(12.0))
      .base(Line, |s| s.background(PaletteColor::Border))
      .variant(Horizontal, Line, |s| s.height(1.0))
      .variant(Vertical, Line, |s| s.width(1.0))
      .state(Done, Marker, filled)
      .state(Current, Marker, filled)
      .state(Upcoming, Marker, |s| {
        s.border(border::Border::inside(BorderSize::Sm, Input)).foreground(MutedForeground)
      })
      .state(Upcoming, Title, |s| s.foreground(MutedForeground))
      .state(Done, Line, |s| s.background(Primary))
      .state(Error, Marker, |s| {
        s.background(Destructive)
          .border(border::Border::inside(BorderSize::Sm, Destructive))
          .foreground(DestructiveForeground)
      })
      .state(Error, Title, |s| s.foreground(Destructive))
      .state(Pressable, Step, |s| s.cursor(CursorIcon::Pointer))
      .when(Selector::new().state(Pressable).hovered(), Step, |s| s.veil(StateHover))
      .when(Selector::new().state(Pressable).pressed(), Step, |s| s.veil(StatePressed))
      .focus_visible(Step, |s| s.focus_ring())
      .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
  }
}
