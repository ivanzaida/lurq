//! Steps: the stages of a process in order, with one current.

use std::sync::Arc;

use parking_lot::Mutex;

use super::{
  icons, names,
  part_style::Length,
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe, Selector},
  shared::{Orientation, fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{BorderSize, OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::{Justify, ShrinkLimit},
    text_style::FontWeight,
  },
  node::{CursorIcon, Element, FocusGroup, border, dimension::Dimension, node_kind::TextOverflow},
};

/// The least length of the room of a line between two steps in a row.
const MIN_CONNECTOR: f32 = 24.0;
/// The least a title is wide before the steps take another form: a few letters and an ellipsis.
const TITLE_LEAST: f32 = 40.0;

/// The form mounted steps in a row take in the room they have.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Shape {
  /// Each title beside its mark.
  #[default]
  Beside,
  /// Each title under its mark: the room is under what the titles need beside the marks.
  Under,
  /// The steps down the page: the room is under what the marks and their lines need in a row.
  Down,
  /// The board's progress of a phone: a segment for each step and a line that says where the process is. Steps
  /// that are only read, in a room under what the titles need beside the marks.
  Progress,
}

/// A signal's value is something DevTools can inspect: this one has nothing to show.
impl crate::app::component::DevtoolsInspectable for Shape {}

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
  Text,
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
  /// Of the progress form: the row of segments, one for each step.
  Segments,
  /// Of the progress form: the segment of a step, in the color of what is done up to the current step.
  Segment,
  /// Of the progress form: the line under the segments that says where the process is, "Step 2 of 4 · Billing".
  Summary,
}

names!(StepsPart {
  Root => "root",
  Step => "step",
  Marker => "marker",
  Number => "number",
  Icon => "icon",
  Text => "text",
  Title => "title",
  Description => "description",
  Rail => "rail",
  Connector => "connector",
  Line => "line",
  Segments => "segments",
  Segment => "segment",
  Summary => "summary",
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
  Invalid,
  /// A step a press of which makes it current.
  Pressable,
  /// The steps take no input; on a step, that step takes none.
  Disabled,
  /// The steps run in a row.
  Horizontal,
  /// The steps run in a column.
  Vertical,
  /// The steps run in a row with each title under its mark: mounted steps in a room too narrow for the titles
  /// beside the marks.
  Stacked,
  /// The steps are a segment each over a line that says where the process is: mounted steps that are only read,
  /// in a room too narrow for the titles beside the marks.
  Progress,
}

names!(StepsState {
  Done => "done",
  Current => "current",
  Upcoming => "upcoming",
  Invalid => "invalid",
  Pressable => "pressable",
  Disabled => "disabled",
  Horizontal => "horizontal",
  Vertical => "vertical",
  Stacked => "stacked",
  Progress => "progress",
});

/// Which way the steps run: in a row, a line between each step and the
/// next; in a column, a line running down from each step's mark to the next.
/// The state `Horizontal` or `Vertical` of their recipe.
pub type StepsOrientation = Orientation;

/// One stage of a process: a title, and a description, an icon in place of
/// its number, or a mark of what went wrong where it has them.
///
/// ```
/// use lurq::ui::{Step, icons};
///
/// let account = Step::new("Account").description("Your name and email");
/// let billing = Step::new("Billing").icon(icons::USER).invalid(true);
/// let plain: Step = "Finish".into();
/// ```
#[derive(Clone, Debug, Default)]
pub struct Step {
  pub title: Arc<str>,
  pub description: Option<Arc<str>>,
  pub icon: Option<Name>,
  pub invalid: bool,
  pub disabled: bool,
  pub id: Option<Arc<str>>,
  pub key: Option<Arc<str>>,
}

impl Step {
  /// A step called `title`.
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      ..Self::default()
    }
  }

  /// A line under the title that says more.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// An icon of the theme's set in place of the step's number, until the step is done.
  pub fn icon(mut self, name: impl Into<Name>) -> Self {
    self.icon = Some(name.into());
    self
  }

  /// Whether the step has gone wrong: its mark and its title say so, whatever else it is.
  pub fn invalid(mut self, invalid: bool) -> Self {
    self.invalid = invalid;
    self
  }

  /// Whether the step takes no input: it is not pressed, Tab and the arrows pass it by, and it is dimmed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The id of the step's element, in place of `<id>-<index>`. Its mark and the line after it keep the ids
  /// of its index.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of the step's element, for the engine to tell it from the others when steps come and go.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
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
    Self::new(&title)
  }
}

/// Which step is current: a signal a press sets, an index the app holds and is told of a press, or an index the
/// steps only show.
#[derive(Clone)]
enum CurrentStep {
  Signal(Signal<usize>),
  Index(usize),
  Shown(usize),
}

/// What builds the nodes of steps: a theme's in place of the default one.
pub type StepsRenderer = Arc<dyn Fn(StepsView) -> Element + Send + Sync>;

type Change = Arc<dyn Fn(usize) + Send + Sync>;

/// The stages of a process in order, with one current: those before it are
/// done, those after it are to come. A wizard's account, billing and finish.
///
/// ```
/// use lurq::{core::Signal, ui::{Orientation, Step, Steps}};
///
/// # fn steps(at: Signal<usize>, page: usize, go: fn(usize)) {
/// // Shown, not operated: the app moves the current step itself.
/// let progress = Steps::at(page, ["Account", "Workspace", "Finish"]);
/// // A wizard that lets a person go back to a step that is done: a press sets the signal.
/// let wizard = Steps::new(at.clone(), [
///   Step::new("Account").description("Name and email"),
///   Step::new("Billing").description("Card and address"),
///   Step::new("Finish"),
/// ])
/// .orientation(Orientation::Vertical)
/// .on_change(|index| println!("back to step {index}"));
/// // The current step held by the app, which answers a press itself.
/// let held = Steps::controlled(page, ["Account", "Finish"], move |index| go(index));
/// # }
/// ```
///
/// The current step is the app's: a `Signal<usize>` that a press of a step
/// sets ([`Steps::new`]), an index it holds and changes itself when told
/// ([`Steps::controlled`]), or an index the steps only show, where no step
/// is pressed ([`Steps::at`]). An index past the last step has every step
/// done. Steps are pressed only where the app says so, with
/// [`Steps::on_change`], [`Steps::controlled`] or [`Steps::pressable`]: then
/// a step that is done is pressed to go back to it, and a step to come only
/// with [`Steps::ahead`]. Pressed steps are one stop of Tab, at the current
/// step, and the arrow keys of their orientation move between them.
#[derive(Clone)]
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
  on_change: Vec<Change>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  /// The form the room gives steps in a row, where they are mounted.
  shape: Shape,
}

impl Steps {
  /// The steps of a process, of which the one `current` holds is current. A
  /// press of a step, where steps are pressed, sets it.
  pub fn new(current: Signal<usize>, steps: impl IntoIterator<Item = impl Into<Step>>) -> Self {
    Self::with(CurrentStep::Signal(current), steps)
  }

  /// The steps of a process, of which the one at `current` is current, held
  /// by the app: a press of a step tells `on_change` and shows what the app
  /// answers with, the next time it builds the steps.
  pub fn controlled(
    current: usize,
    steps: impl IntoIterator<Item = impl Into<Step>>,
    on_change: impl Fn(usize) + Send + Sync + 'static,
  ) -> Self {
    Self::with(CurrentStep::Index(current), steps).on_change(on_change)
  }

  /// The steps of a process, of which the one at `current` is current, shown
  /// and not operated: the steps of a wizard the app drives. No step is
  /// pressed, whatever [`Steps::pressable`] and [`Steps::on_change`] say, and
  /// the steps are not a stop of Tab.
  pub fn at(current: usize, steps: impl IntoIterator<Item = impl Into<Step>>) -> Self {
    Self::with(CurrentStep::Shown(current), steps)
  }

  fn with(current: CurrentStep, steps: impl IntoIterator<Item = impl Into<Step>>) -> Self {
    Self {
      current,
      steps: steps.into_iter().map(Into::into).collect(),
      orientation: StepsOrientation::default(),
      pressable: false,
      ahead: false,
      disabled: false,
      label: None,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_change: Vec::new(),
      id: None,
      key: None,
      shape: Shape::default(),
    }
  }

  /// Which way the steps run: in a row, or in a column.
  pub fn orientation(mut self, orientation: StepsOrientation) -> Self {
    self.orientation = orientation;
    self
  }

  /// Whether a step that is done is pressed to go back to it. A handler
  /// given with [`Steps::on_change`] says so too; this is for steps bound to
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
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
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

  /// Called when a person makes another step current, with its index from
  /// 0: a press of it, or Enter or Space on it. Not when the app sets the
  /// signal. Makes the steps that are done pressable; where the current step
  /// is a signal, the press sets it first. A second handler is called after
  /// the first.
  pub fn on_change(mut self, handler: impl Fn(usize) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
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

  /// The case of the whole, or of one step: its state, whether it is wrong, pressed or disabled.
  fn case(&self, step: Option<(&Step, StepsState, bool)>) -> Case {
    let vertical = self.orientation == Orientation::Vertical;
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(StepsState::Horizontal, !vertical)
    .state(StepsState::Vertical, vertical)
    .state(StepsState::Stacked, !vertical && self.shape == Shape::Under)
    .state(StepsState::Progress, !vertical && self.shape == Shape::Progress);
    match step {
      // The whole is dimmed once, on its root; a step is dimmed on its own where it alone is disabled.
      None => case.state(StepsState::Disabled, self.disabled),
      Some((step, status, pressable)) => case
        .state(status, true)
        .state(StepsState::Invalid, step.invalid)
        .state(StepsState::Pressable, pressable)
        .state(StepsState::Disabled, step.disabled && !self.disabled),
    }
  }

  /// The steps mounted where they are declared: steps in a row read the room they have, before the frame is
  /// painted, and take the form it has room for. Where the titles have no room beside the marks, cut to a few
  /// letters, steps that are only read are the board's progress of a phone (Mobile/Step Progress): a segment for
  /// each step, in the primary color up to the current one, over a line that says where the process is, "Step 2
  /// of 4 · Billing". Steps a person presses keep every step to press: each title goes under its mark, and where
  /// the marks and the lines between them have no room in a row, the steps run down the page. Steps that are
  /// only built are in a row whatever their room: their titles give way, to nothing if they must, and their marks
  /// and lines never do.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    match self.key.clone() {
      Some(key) => ctx.mount_keyed::<StepsHost>(&key, StepsProps { steps: self }),
      None => ctx.mount::<StepsHost>(StepsProps { steps: self }),
    }
  }

  /// What steps in a row need of width with each title beside its mark, cut to its least, and with each title
  /// under its mark: under the first they stack, and under the second they run down the page.
  fn needs(&self, theme: &Theme) -> (f32, f32) {
    let recipe = theme.resolve::<Steps>(&self.case(None), Some(&self.rules));
    let spacing = theme.spacing();
    let gap = |part: StepsPart| {
      recipe
        .part(part)
        .rest()
        .gap
        .map(|gap| gap.resolve(&spacing, 0.0))
        .unwrap_or(0.0)
    };
    let [_, end, _, start] = recipe.part(StepsPart::Step).rest().paddings();
    let padding = [start, end]
      .into_iter()
      .flatten()
      .map(|padding| padding.resolve(&spacing, 0.0))
      .sum::<f32>();
    let mark = match recipe
      .part(StepsPart::Marker)
      .rest()
      .width
      .and_then(|width| width.resolve(theme))
    {
      Some(Dimension::Px(px)) => px,
      _ => 0.0,
    };
    let count = self.steps.len() as f32;
    let lines = (count - 1.0).max(0.0) * (MIN_CONNECTOR + 2.0 * gap(StepsPart::Root));
    (
      count * (padding + mark + gap(StepsPart::Step) + TITLE_LEAST) + lines,
      count * (padding + mark.max(TITLE_LEAST)) + lines,
    )
  }

  /// Whether a press or a key selects a step: the steps are operated, not only read.
  fn operated(&self) -> bool {
    let shown = matches!(self.current, CurrentStep::Shown(_));
    (self.pressable || !self.on_change.is_empty()) && !self.disabled && !shown
  }

  /// Builds the steps in `theme`, in a row or a column as they were told, whatever their room: their titles give
  /// way in a row too narrow for them, and their marks and lines never do. Converting steps into an element does
  /// this in the theme of what is being rendered; [`mount`](Self::mount) is what measures the room.
  pub fn build(mut self, theme: &Theme) -> Element {
    // Down the page they are the steps of a column, in everything.
    if self.shape == Shape::Down {
      self.orientation = Orientation::Vertical;
    }
    let current = match &self.current {
      CurrentStep::Index(index) | CurrentStep::Shown(index) => *index,
      CurrentStep::Signal(signal) => signal.get(),
    };
    let operated = self.operated();
    let select: Option<Change> = operated.then(|| {
      let handlers = self.on_change.clone();
      let signal = match &self.current {
        CurrentStep::Signal(signal) => Some(signal.clone()),
        CurrentStep::Index(_) | CurrentStep::Shown(_) => None,
      };
      Arc::new(move |index: usize| {
        if let Some(signal) = &signal {
          // Nothing changes where the step is current already.
          if signal.get_untracked() == index {
            return;
          }
          signal.set(index);
        }
        for handler in &handlers {
          handler(index);
        }
      }) as Change
    });
    let steps_of = &self.steps;
    let pressable = |index: usize| {
      select.is_some()
        && index != current
        && (index < current || self.ahead)
        && !steps_of.get(index).is_some_and(|step| step.disabled)
    };
    let recipe = theme
      .resolve::<Steps>(&self.case(None), Some(&self.rules))
      .owned_by(self.id.as_ref());
    let steps: Vec<StepView> = self
      .steps
      .iter()
      .enumerate()
      .map(|(index, step)| {
        let status = status_of(index, current);
        let pressable = pressable(index);
        // A step names its parts after its id, or after the steps' id and its index, as its holder is named.
        let id = step
          .id
          .clone()
          .or_else(|| self.id.as_ref().map(|id| Arc::from(format!("{id}-{index}"))));
        StepView {
          step: step.clone(),
          status,
          pressable,
          recipe: theme
            .resolve::<Steps>(&self.case(Some((step, status, pressable))), Some(&self.rules))
            .owned_like(&recipe)
            .item(id.as_ref(), StepsPart::Step),
        }
      })
      .collect();
    let view = StepsView {
      recipe,
      steps,
      current,
      orientation: self.orientation,
      stacked: self.orientation == Orientation::Horizontal && self.shape == Shape::Under,
      progress: self.orientation == Orientation::Horizontal && self.shape == Shape::Progress,
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

#[derive(Clone)]
struct StepsProps {
  steps: Steps,
}

/// Steps are made anew by every render of what holds them, so props are never the same as before.
impl PartialEq for StepsProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for StepsProps {}

/// Mounted steps: the column that takes the room they are given, which it measures before the frame is painted,
/// around the steps in the form that room has room for.
struct StepsHost {
  anchor: ElementRef,
  shape: Signal<Shape>,
  /// What the steps need of width for each of their two forms in a row, as they were last built, and whether
  /// a person presses them.
  needs: Arc<Mutex<(f32, f32, bool)>>,
}

impl Component for StepsHost {
  type Props = StepsProps;

  fn create(ctx: &mut Ctx) -> Self {
    let (anchor, shape) = (ElementRef::new(), ctx.signal(Shape::default()));
    let needs = Arc::new(Mutex::new((0.0, 0.0, false)));
    // Written only where the room crosses what a form needs: a panel dragged within one form renders nothing.
    let (watch, needed) = (shape.clone(), needs.clone());
    anchor.observe_rect_before_paint(move |rect| {
      let (beside, under, operated) = *needed.lock();
      let now = if rect.width >= beside - 0.5 {
        Shape::Beside
      } else if !operated {
        // Steps that are only read are the board's progress of a phone.
        Shape::Progress
      } else if rect.width >= under - 0.5 {
        Shape::Under
      } else {
        Shape::Down
      };
      if watch.get_untracked() != now {
        watch.set(now);
      }
    });
    Self { anchor, shape, needs }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut steps = ctx.props::<StepsProps>().steps.clone();
    let theme = current_theme();
    // Steps the app runs down the page are so in any room.
    if steps.orientation == Orientation::Horizontal {
      let (beside, under) = steps.needs(&theme);
      *self.needs.lock() = (beside, under, steps.operated());
      steps.shape = self.shape.get();
    } else {
      *self.needs.lock() = (0.0, 0.0, false);
    }
    Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .overflow_visible()
      .ref_element(self.anchor.clone())
      .child(steps.build(&theme))
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
  /// Whether each title is under its mark: steps in a row in a room too narrow for the titles beside the marks.
  pub stacked: bool,
  /// Whether the steps are the progress form: a segment each, over a line that says where the process is.
  pub progress: bool,
  pub disabled: bool,
  pub label: Option<Arc<str>>,
  /// The style of the parts of the whole: its root.
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  select: Option<Change>,
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
    let locale = super::current_locale();
    let words = &locale.navigation;
    let status = match step.status {
      StepsState::Done => &words.step_done,
      StepsState::Current => &words.step_current,
      _ => &words.step_to_come,
    };
    let place = crate::core::fill(
      &words.step_of,
      &[
        ("at", &(index + 1).to_string()),
        ("total", &self.steps.len().to_string()),
      ],
    );
    let mut name = format!("{place}, {}, {status}", step.step.title);
    if step.step.invalid {
      name.push_str(", ");
      name.push_str(&words.step_error);
    }
    name
  }

  fn child_id(&self, index: usize, suffix: &str) -> Option<String> {
    self.id.as_ref().map(|id| format!("{id}-{index}{suffix}"))
  }

  /// The mark of the step at `index`: a cross where it is in error, a check
  /// where it is done, its icon or its number where it is not.
  pub fn marker(&self, index: usize) -> Row {
    let step = &self.steps[index];
    let icon = if step.step.invalid {
      Some(Name::new(icons::CLOSE))
    } else if step.status == StepsState::Done {
      Some(Name::new(icons::CHECK))
    } else {
      step.step.icon.clone()
    };
    // A mark is whole in any room: what is beside it gives way.
    let mut marker = Row::new()
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .flex_shrink(0.0)
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
  pub fn text(&self, index: usize) -> Column {
    let step = &self.steps[index].step;
    let title = self.step_part(index, StepsPart::Title);
    // It gives way where the steps do not fit: the description goes on to more lines, down to the title's width
    // (its least is 0, the title's is its own), and only then is the title cut with an ellipsis.
    let mut body = Column::new()
      .part(&self.step_part(index, StepsPart::Text))
      .flex_shrink(1.0)
      .shrink_limit(ShrinkLimit::ContentFirst)
      .min_width(Dimension::Px(0.0))
      .child({
        let text = title.text(&step.title).nowrap().text_overflow(TextOverflow::Elipsis);
        let row = Row::new().align_items(Alignment::Center).part(&title);
        // Under its mark a title is no wider than its step, and gives way to it.
        if self.stacked {
          row
            .max_width(Dimension::Pct(100.0))
            .child(text.flex_full(0.0, 1.0, None).min_width(Dimension::Px(0.0)))
        } else {
          row.child(text)
        }
      });
    // Under its mark the text is in the middle of the step, as the mark is.
    if self.stacked {
      body = body.align_items(Alignment::Center).max_width(Dimension::Pct(100.0));
    }
    if let Some(description) = &step.description {
      let part = self.step_part(index, StepsPart::Description);
      let fill = Dimension::Pct(100.0);
      body = body.child(
        Column::new()
          .width(fill.clone())
          .min_width(Dimension::Px(0.0))
          .part(&part)
          .child(part.text(description).width(fill)),
      );
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
        // A basis of 24 and a share of the room the steps leave: a line is seen between steps that do not fit,
        // which give way before it.
        Row::new()
          .flex_full(1.0, 0.0, Some(MIN_CONNECTOR))
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
    let horizontal = self.orientation == StepsOrientation::Horizontal;
    // A step in a row is never narrower than its mark with the step's padding around it.
    let least = if horizontal {
      let rest = self.step_part(index, StepsPart::Step).rest();
      let spacing = self.theme().spacing();
      let [_, end, _, start] = rest.paddings();
      let mark = match self
        .step_part(index, StepsPart::Marker)
        .rest()
        .width
        .and_then(|width| width.resolve(self.theme()))
      {
        Some(Dimension::Px(px)) => px,
        _ => 0.0,
      };
      mark
        + [start, end]
          .into_iter()
          .flatten()
          .map(|padding| padding.resolve(&spacing, 0.0))
          .sum::<f32>()
    } else {
      0.0
    };
    let id = step
      .step
      .id
      .as_ref()
      .map(|id| id.to_string())
      .or_else(|| self.child_id(index, ""));
    let key = step.step.key.clone();
    let current = step.status == StepsState::Current;
    let focused = step.pressable || (current && self.interactive());
    macro_rules! dressed {
      ($holder:expr) => {{
        // A step in a row gives way where the steps do not fit, before the lines between them: down to its title
        // while any step's description can still wrap, then the last step first.
        let mut holder = $holder
          .align_items(align)
          .flex_shrink(if horizontal { 1.0 } else { 0.0 })
          .shrink_limit(ShrinkLimit::ContentFirst)
          .min_width(Dimension::Px(least))
          .describe("a11y_name", name)
          .with_children(content);
        if current {
          holder = holder.describe("a11y_current", "step");
        }
        if let Some(id) = id {
          holder = holder.id(id);
        }
        if let Some(key) = key {
          holder = holder.key(key);
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

  /// What the progress form says under its segments: where the process is and what that step is called, "Step 2
  /// of 4 · Billing", and that every step is done once they are.
  pub fn summary(&self) -> String {
    let count = self.steps.len();
    let locale = super::current_locale();
    match self.steps.get(self.current) {
      Some(step) => {
        let place = crate::core::fill(
          &locale.navigation.step_of,
          &[("at", &(self.current + 1).to_string()), ("total", &count.to_string())],
        );
        format!("{place} · {}", step.step.title)
      }
      None => locale.count(&locale.navigation.steps_done, count),
    }
  }

  /// The progress form: a segment for each step, sharing the width, over the line that says where the process
  /// is. To what reads the tree it is a progress bar called by the steps' label, whose value is that line.
  pub fn progress(&self) -> Element {
    let count = self.steps.len();
    let mut segments = Row::new().part(&self.part(StepsPart::Segments));
    for index in 0..count {
      let mut segment = Row::new()
        .part(&self.step_part(index, StepsPart::Segment))
        .flex_full(1.0, 1.0, Some(0.0))
        .min_width(Dimension::Px(0.0));
      if let Some(id) = self.child_id(index, "-segment") {
        segment = segment.id(id);
      }
      segments = segments.child(segment);
    }
    let said = self.summary();
    let summary = self.part(StepsPart::Summary);
    let mut root = Column::new()
      .part(&self.part(StepsPart::Root))
      .align_items(Alignment::Stretch)
      .describe("a11y_role", "progressbar")
      .describe("a11y_valuemin", "0")
      .describe("a11y_valuemax", count.to_string())
      .describe("a11y_valuenow", self.current.min(count).to_string())
      .describe("a11y_valuetext", said.clone())
      .child(segments)
      .child(
        Row::new().min_width(Dimension::Px(0.0)).child(
          summary
            .text(&said)
            .nowrap()
            .text_overflow(crate::components::TextOverflow::Elipsis)
            .flex_full(0.0, 1.0, None)
            .min_width(Dimension::Px(0.0))
            .part(&summary),
        ),
      );
    if let Some(label) = self.label.clone() {
      root = root.describe("a11y_name", label);
    }
    if let Some(id) = self.id.clone() {
      root = root.id(id);
    }
    if let Some(key) = self.key.clone() {
      root = root.key(key);
    }
    root.into()
  }

  /// How the default renderer builds steps: in a row, each step's mark and
  /// text with a line to the next; in a column, each step's mark with the
  /// line down from it, and its text beside them; and the progress form
  /// where mounted steps that are only read have no room for their titles.
  pub fn render(self) -> Element {
    if self.progress {
      return self.progress();
    }
    let count = self.steps.len();
    let mut children: Vec<Element> = Vec::with_capacity(count * 2);
    for index in 0..count {
      let last = index + 1 == count;
      match self.orientation {
        // Each title under its mark: the two in a column in the middle of the step, which gives way with it.
        StepsOrientation::Horizontal if self.stacked => {
          let stack = Column::new()
            .align_items(Alignment::Center)
            .part(&self.step_part(index, StepsPart::Rail))
            .flex_shrink(1.0)
            .min_width(Dimension::Px(0.0))
            .child(self.marker(index))
            .child(self.text(index));
          children.push(self.step(index, [stack.into()]));
          if !last {
            children.push(self.connector(index));
          }
        }
        StepsOrientation::Horizontal => {
          children.push(self.step(index, [self.marker(index).into(), self.text(index).into()]));
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
          children.push(self.step(index, [rail.into(), self.text(index).into()]));
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

impl crate::ui::Styled for Steps {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for Steps {
  const NAME: &'static str = "steps";
  type Renderer = StepsRenderer;

  fn parts() -> crate::ui::Declared {
    StepsPart::declared()
  }

  fn states() -> crate::ui::Declared {
    StepsState::declared()
  }

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
  ///
  /// The progress form is the board's progress of a phone (Mobile/Step
  /// Progress): segments 4 high and round, 4 apart, sharing the width, in the
  /// primary color up to the current step and in the secondary after it (in
  /// the destructive one for a step in error), and 8 under them, in the
  /// body's type at 13 and the muted foreground, where the process is.
  fn recipe() -> Recipe {
    use PaletteColor::*;
    use StepsPart::{
      Connector, Description, Icon, Line, Marker, Number, Rail, Root, Segment, Segments, Step, Summary, Text, Title,
    };
    use StepsState::{Current, Disabled, Done, Horizontal, Invalid, Pressable, Progress, Stacked, Upcoming, Vertical};
    let filled = |s: super::PartStyle| s.background(Primary).foreground(PrimaryForeground);
    Recipe::new()
      .state(Horizontal, Root, |s| s.width(Length::Fill).gap(8.0))
      .base(Step, |s| {
        fading(s.gap(8.0).padding_x(4.0).padding_y(4.0).radius(RadiusSize::Md))
      })
      .base(Marker, |s| s.square(24.0).radius(RadiusSize::Full))
      .base(Number, |s| s.typography(TypographyStyle::Caption).weight(FontWeight::Medium))
      .base(Icon, |s| s.square(14.0))
      .base(Text, |s| s.gap(2.0))
      // As high as the mark, so that the title stands level with its middle and a description goes under both.
      .base(Title, |s| s.height(24.0).typography(TypographyStyle::Label).foreground(Foreground))
      .base(Description, |s| s.typography(TypographyStyle::Caption).foreground(MutedForeground))
      .base(Rail, |s| s.gap(4.0))
      // Under its mark a title is as high as its words: it is not level with the mark there.
      .state(Stacked, Title, |s| s.height(Length::Auto))
      // The progress form: the segments over the line that says where the process is.
      .state(Progress, Root, |s| s.width(Length::Fill).gap(8.0))
      .base(Segments, |s| s.width(Length::Fill).gap(4.0))
      .base(Segment, |s| s.height(4.0).radius(RadiusSize::Full).background(Secondary))
      .state(Done, Segment, |s| s.background(Primary))
      .state(Current, Segment, |s| s.background(Primary))
      .state(Invalid, Segment, |s| s.background(Destructive))
      .base(Summary, |s| {
        s.typography(TypographyStyle::Body)
          .font_size(13.0)
          .foreground(MutedForeground)
      })
      // In a row, as high as a step with its padding, so that the line is level with the middle of the mark.
      .state(Horizontal, Connector, |s| s.height(32.0))
      // In a column, as wide as the mark, and long enough to be seen between two steps without descriptions.
      .state(Vertical, Connector, |s| s.width(24.0).height(12.0))
      .base(Line, |s| s.background(PaletteColor::Border))
      .state(Horizontal, Line, |s| s.height(1.0))
      .state(Vertical, Line, |s| s.width(1.0))
      .state(Done, Marker, filled)
      .state(Current, Marker, filled)
      .state(Upcoming, Marker, |s| {
        s.border(border::Border::inside(BorderSize::Sm, Input)).foreground(MutedForeground)
      })
      .state(Upcoming, Title, |s| s.foreground(MutedForeground))
      .state(Done, Line, |s| s.background(Primary))
      .state(Invalid, Marker, |s| {
        s.background(Destructive)
          .border(border::Border::inside(BorderSize::Sm, Destructive))
          .foreground(DestructiveForeground)
      })
      .state(Invalid, Title, |s| s.foreground(Destructive))
      .state(Pressable, Step, |s| s.cursor(CursorIcon::Pointer))
      .when(Selector::new().state(Pressable).hovered(), Step, |s| s.veil(StateHover))
      .when(Selector::new().state(Pressable).pressed(), Step, |s| s.veil(StatePressed))
      .focus_visible(Step, |s| s.focus_ring())
      .state(Disabled, Root, |s| s.opacity(OpacityLevel::Disabled))
      .state(Disabled, Step, |s| s.opacity(OpacityLevel::Disabled))
  }
}
