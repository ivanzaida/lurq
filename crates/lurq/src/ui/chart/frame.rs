//! The frame every chart is drawn in: the header, the plot on a canvas with its grid, its tooltip and its keys, the
//! axis under it and the legend; the props every chart has; and the host that keeps the pointer and the focus of a
//! mounted chart.

use std::{
  hash::{Hash, Hasher},
  marker::PhantomData,
  sync::Arc,
};

use parking_lot::Mutex;

use super::{
  super::Orientation, ChartArea, ChartAxis, ChartLegend, ChartLegendItem, ChartLegendShape, ChartLook, ChartPart,
  ChartPlot, ChartPoint, ChartSeries, ChartSize, ChartState, ChartTooltip, ChartTooltipRow, ValueTextFn,
  default_value_text, series_color, step,
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, MouseEvent},
    theme::{BorderSize, PaletteColor, RadiusSize, ShadowStyle, Theme, TypographyStyle},
  },
  canvas::CanvasHandle,
  components::{Canvas, Column, Row, Stack},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, stack_alignment::StackAlignment, text_style::FontWeight},
  node::{Element, border::Border as Edge, dimension::Dimension, node_kind::TextOverflow, outline::Outline},
  ui::{
    part_style::{FontFamily, Length},
    recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
    skeleton::Skeleton,
    theme::current_theme,
  },
};

type HiddenWrite = Arc<dyn Fn(&[String]) + Send + Sync>;
type Selected = Arc<dyn Fn(ChartPoint) + Send + Sync>;

/// What renders a chart: a theme's in place of the default. Every chart's renderer is one of these.
pub type ChartRenderer = Arc<dyn Fn(ChartView) -> Element + Send + Sync>;

/// The props every chart has, which its builder keeps and its frame is made from. A chart's builder holds one and
/// gives its builder methods with [`chart_props!`](crate::chart_props).
#[derive(Clone, Default)]
pub struct ChartProps {
  pub title: Option<Arc<str>>,
  pub description: Option<Arc<str>>,
  pub label: Option<Arc<str>>,
  pub size: Option<ChartSize>,
  pub height: Option<Length>,
  pub legend: Option<bool>,
  pub hidden: Vec<String>,
  pub write_hidden: Option<HiddenWrite>,
  pub on_hidden_change: Vec<HiddenWrite>,
  pub value_text: Option<ValueTextFn>,
  pub grid: Option<bool>,
  pub value_axis: Option<bool>,
  pub empty_text: Option<Arc<str>>,
  pub loading: bool,
  pub on_select: Vec<Selected>,
  pub states: Vec<Name>,
  pub rules: Recipe,
  pub forced: Option<Interaction>,
  pub id: Option<Arc<str>>,
  pub key: Option<Arc<str>>,
}

impl ChartProps {
  /// The case the chart's recipe is resolved in: its size (`default` unless it was given one) and its states.
  pub fn case(&self, default: ChartSize, empty: bool) -> Case {
    let mut case = Case::new()
      .size(self.size.unwrap_or(default))
      .breakpoint(super::super::current_breakpoint())
      .state(ChartState::Header, self.title.is_some() || self.description.is_some())
      .state(ChartState::Loading, self.loading)
      .state(ChartState::Empty, empty && !self.loading);
    case.states.extend(self.states.iter().copied());
    case
  }

  pub fn value_text(&self) -> ValueTextFn {
    self.value_text.clone().unwrap_or_else(|| Arc::new(default_value_text))
  }

  /// The height of the plot in `recipe`: the props', or the size's in the recipe; for a chart that lays its plot out
  /// from it (a square of that side, a grid of cells).
  pub fn plot_height(&self, recipe: &ResolvedRecipe) -> f32 {
    let length = self.height.or_else(|| recipe.part(ChartPart::Plot).rest().height);
    match length.and_then(|length| length.resolve(recipe.theme())) {
      Some(Dimension::Px(px)) => px,
      _ => 160.0,
    }
  }

  /// Whether `series` is hidden, through the legend.
  pub fn is_hidden(&self, series: &ChartSeries) -> bool {
    self.hidden.iter().any(|name| **name == *series.name())
  }

  /// The frame of a plot of `series` in `recipe`, as these props make it: what a chart's `build` gives its renderer.
  pub fn frame(
    &self,
    recipe: ResolvedRecipe,
    plot: Arc<dyn ChartPlot>,
    series: Vec<ChartSeries>,
    renderer: Option<ChartRenderer>,
  ) -> ChartFrame {
    let look = ChartLook::new(recipe.clone(), &series, self.value_text());
    ChartFrame {
      recipe,
      plot,
      look,
      series,
      props: self.clone(),
      renderer,
      plot_width: None,
      plot_justify: Justify::Start,
      beside: None,
      own_legend: None,
    }
  }
}

/// The builder methods every chart has, on a builder that keeps its [`ChartProps`] in a field named `props`.
#[macro_export]
macro_rules! chart_props {
  () => {
    /// The title over the plot, which is the chart's name to tooling unless it is given a [`label`](Self::label).
    pub fn title(mut self, title: &str) -> Self {
      self.props.title = Some(std::sync::Arc::from(title));
      self
    }

    /// The description under the title: what the values are.
    pub fn description(mut self, description: &str) -> Self {
      self.props.description = Some(std::sync::Arc::from(description));
      self
    }

    /// What the chart is called to tooling, where it has no title or another name than its title. It draws nothing.
    pub fn label(mut self, label: &str) -> Self {
      self.props.label = Some(std::sync::Arc::from(label));
      self
    }

    /// How roomy the frame is: the padding, the room between its rows, the type of its header, the height of the
    /// plot.
    pub fn size(mut self, size: impl Into<$crate::ui::ChartSize>) -> Self {
      self.props.size = Some(size.into());
      self
    }

    /// The height of the plot, over the size's.
    pub fn height(mut self, height: impl Into<$crate::ui::Length>) -> Self {
      self.props.height = Some(height.into());
      self
    }

    /// Whether the chart has its legend under the plot: where it has more than one series unless told.
    pub fn legend(mut self, legend: bool) -> Self {
      self.props.legend = Some(legend);
      self
    }

    /// The series hidden, by their names: those the signal has. A press of a series' item in the legend hides it,
    /// or shows it again.
    pub fn hidden(mut self, hidden: $crate::core::Signal<Vec<String>>) -> Self
    where
      Vec<String>: $crate::core::SignalValue,
    {
      self.props.hidden = hidden.get();
      self.props.write_hidden = Some(std::sync::Arc::new(move |next: &[String]| hidden.set(next.to_vec())));
      self
    }

    /// The series hidden, as the app holds them: `on_change` is told the next after each press of the legend.
    pub fn hidden_controlled(
      mut self,
      hidden: impl IntoIterator<Item = String>,
      on_change: impl Fn(&[String]) + Send + Sync + 'static,
    ) -> Self {
      self.props.hidden = hidden.into_iter().collect();
      self.props.write_hidden = Some(std::sync::Arc::new(on_change));
      self
    }

    /// Called with the series hidden after each press of the legend that changes them. A second handler is called
    /// after the first.
    pub fn on_hidden_change(mut self, handler: impl Fn(&[String]) + Send + Sync + 'static) -> Self {
      self.props.on_hidden_change.push(std::sync::Arc::new(handler));
      self
    }

    /// How each number the chart writes is written: a tick, a tooltip's value, what tooling reads. Whole numbers
    /// whole, others to two places, unless told.
    pub fn value_text(mut self, text: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
      self.props.value_text = Some(std::sync::Arc::new(text));
      self
    }

    /// Whether the plot has a line across it at each tick of its values.
    pub fn grid(mut self, grid: bool) -> Self {
      self.props.grid = Some(grid);
      self
    }

    /// Whether each tick's number is written beside its line.
    pub fn value_axis(mut self, value_axis: bool) -> Self {
      self.props.value_axis = Some(value_axis);
      self
    }

    /// What the plot says where there is no data: "No data" unless told.
    pub fn empty_text(mut self, text: &str) -> Self {
      self.props.empty_text = Some(std::sync::Arc::from(text));
      self
    }

    /// Whether the data is on its way: the plot is a skeleton.
    pub fn loading(mut self, loading: bool) -> Self {
      self.props.loading = loading;
      self
    }

    /// Called with the point a press of it, or Enter or Space on the focused one, selects. A second handler is
    /// called after the first.
    pub fn on_select(mut self, handler: impl Fn($crate::ui::ChartPoint) + Send + Sync + 'static) -> Self {
      self.props.on_select.push(std::sync::Arc::new(handler));
      self
    }

    /// Puts the chart in a state the theme's recipe names, or takes it out.
    pub fn state(mut self, state: impl Into<$crate::ui::Name>, on: bool) -> Self {
      let state = state.into();
      self.props.states.retain(|held| *held != state);
      if on {
        self.props.states.push(state);
      }
      self
    }

    /// Styles one part of this chart over what the theme gives it.
    pub fn style(
      mut self,
      part: impl Into<$crate::ui::Name>,
      style: impl FnOnce($crate::ui::PartStyle) -> $crate::ui::PartStyle,
    ) -> Self {
      self.props.rules = self.props.rules.base(part, style);
      self
    }

    /// Lays rules over the theme's recipe for this chart.
    pub fn rules(mut self, rules: &$crate::ui::Recipe) -> Self {
      self.props.rules = self.props.rules.extend(rules);
      self
    }

    /// Draws the plot as it is focused from the keyboard, with nothing pointing at it: for a gallery or a test.
    pub fn force(mut self, interaction: Option<$crate::ui::Interaction>) -> Self {
      self.props.forced = interaction;
      self
    }

    /// The id of the box; the plot's is the chart's with `-plot`.
    pub fn id(mut self, id: impl Into<std::sync::Arc<str>>) -> Self {
      self.props.id = Some(id.into());
      self
    }

    pub fn key(mut self, key: impl Into<std::sync::Arc<str>>) -> Self {
      self.props.key = Some(key.into());
      self
    }
  };
}

/// A chart's frame, made by its builder in a theme: its resolved recipe, its plot, its series and its props. It builds
/// the chart's view and renders it, still ([`build`](Self::build)) or with the pointer and the focus a host keeps.
pub struct ChartFrame {
  pub recipe: ResolvedRecipe,
  pub plot: Arc<dyn ChartPlot>,
  pub look: ChartLook,
  pub series: Vec<ChartSeries>,
  pub props: ChartProps,
  renderer: Option<ChartRenderer>,
  plot_width: Option<f32>,
  plot_justify: Justify,
  beside: Option<Beside>,
  own_legend: Option<Element>,
}

/// The legend beside the plot: each item's value, and the size of the chart.
struct Beside {
  values: Vec<String>,
  size: ChartSize,
}

/// What a mounted chart keeps from render to render: the point under the pointer, the focused point, the canvas,
/// and how wide the plot was last laid out.
pub struct ChartHostState {
  hovered: Mutex<Option<ChartPoint>>,
  focused: Mutex<Option<ChartPoint>>,
  canvas: ElementRef,
  revision: Signal<u64>,
}

impl ChartHostState {
  fn rebuild(&self) {
    self.revision.update(|revision| *revision = revision.wrapping_add(1));
  }

  /// The point the chart shows as active: the one under the pointer, or the focused one where the plot has the
  /// focus.
  fn active(&self) -> Option<ChartPoint> {
    let hovered = *self.hovered.lock();
    hovered.or_else(|| (*self.focused.lock()).filter(|_| self.canvas.focused()))
  }

  fn hover(&self, point: Option<ChartPoint>) {
    let mut held = self.hovered.lock();
    if *held != point {
      *held = point;
      drop(held);
      self.rebuild();
    }
  }
}

impl ChartFrame {
  /// The plot as wide as `width`, not as the frame: a pie's or a radar's square, its height and its width the same.
  /// It stands at the start of the frame unless told ([`plot_justify`](Self::plot_justify)).
  pub fn plot_width(mut self, width: f32) -> Self {
    self.plot_width = Some(width);
    self
  }

  /// Where a plot narrower than the frame stands across it: at its start, its middle or its end.
  pub fn plot_justify(mut self, justify: Justify) -> Self {
    self.plot_justify = justify;
    self
  }

  /// The legend beside the plot, not under it, one item under another, each with its value at its end (`values`, by
  /// the series' order, the app's words): the board's pie. The plot and the legend stand in [`ChartPart::Body`], the
  /// legend's room and type those of `size`.
  pub fn legend_beside(mut self, values: Vec<String>, size: ChartSize) -> Self {
    self.beside = Some(Beside { values, size });
    self
  }

  /// The chart's own legend in place of its series': a heatmap's scale. It is left out as the series' would be: where
  /// the app turns the legend off, and while the data is on its way.
  pub fn own_legend(mut self, legend: impl Into<Element>) -> Self {
    self.own_legend = Some(legend.into());
    self
  }

  fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.props.forced)
  }

  /// The name of the chart to tooling: its label, or its title.
  fn name(&self) -> Arc<str> {
    self
      .props
      .label
      .clone()
      .or_else(|| self.props.title.clone())
      .unwrap_or_else(|| Arc::from("Chart"))
  }

  /// The chart still: drawn, read by tooling, with nothing that follows the pointer or the keys.
  pub fn build(self) -> Element {
    self.render_with(None)
  }

  fn render_with(mut self, host: Option<Arc<ChartHostState>>) -> Element {
    let plot = self.plot_element(host.as_ref());
    let axis = self.axis_element();
    let legend = self.legend_element();
    let view = ChartView {
      beside: self.beside.is_some(),
      plot_width: self.plot_width,
      title: self.props.title.clone(),
      description: self.props.description.clone(),
      name: self.name(),
      plot,
      axis,
      legend,
      recipe: self.recipe.clone(),
      forced: self.props.forced,
      id: self.props.id.clone(),
      key: self.props.key.clone(),
    };
    match &self.renderer {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The height of the plot: the props', or the size's in the recipe.
  fn plot_height(&self) -> f32 {
    self.props.plot_height(&self.recipe)
  }

  fn plot_element(&self, host: Option<&Arc<ChartHostState>>) -> Element {
    let height = self.plot_height();
    let part = self.part(ChartPart::Plot);
    // A plot of its own width has it in each of its states.
    let wide = self.plot_width.map_or(Dimension::Pct(100.0), Dimension::Px);
    if self.props.loading {
      let block = self.plot_width.map_or(Length::Fill, Length::from);
      return self.placed(
        Column::new()
          .width(wide)
          .describe("a11y_busy", "true")
          .child(Skeleton::block(block, height).build(self.recipe.theme()))
          .into(),
      );
    }
    if self.plot.is_empty() {
      let empty = self.part(ChartPart::Empty);
      return self.placed(
        Row::new()
          .width(wide)
          .height(height)
          .justify(Justify::Center)
          .align_items(Alignment::Center)
          .part(&empty)
          .child(
            empty
              .text(self.props.empty_text.as_deref().unwrap_or("No data"))
              .part(&empty)
              .nowrap(),
          )
          .into(),
      );
    }
    let active = host.and_then(|host| host.active());
    // What the plot is drawn from: a drawing changes only with it, the size and the scale.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    self.plot.signature().hash(&mut hasher);
    self.look.hash_into(&mut hasher);
    active.hash(&mut hasher);
    self.grid().hash(&mut hasher);
    let key = hasher.finish();
    let (plot, look, grid) = (self.plot.clone(), self.look.clone(), self.grid());
    let mut canvas = Canvas::new()
      .paint(key, move |canvas: &CanvasHandle| {
        draw(canvas, &*plot, &look, active, grid)
      })
      .part(&part)
      // As large as the plot's box, whose height is the props' where they give one: over the part's.
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .describe("a11y_role", "img")
      .describe("a11y_name", self.name());
    if let Some(id) = &self.props.id {
      canvas = canvas.id(format!("{id}-plot"));
    }
    let width = self
      .plot_width
      .unwrap_or_else(|| host.map_or(0.0, |host| host.canvas.width()));
    let area = ChartArea { width, height };
    if let Some(host) = host {
      canvas = self.dress(canvas, host);
      if let Some(point) = active {
        let tip = self.plot.tip(point);
        let said = tip
          .rows
          .iter()
          .map(|(series, value)| format!("{}: {}", self.series_name(*series), (self.look.value_text)(*value)))
          .collect::<Vec<_>>()
          .join(", ");
        // A point with no rows (a heatmap's cell with no value) says its title alone.
        let said = if said.is_empty() {
          tip.title.clone()
        } else {
          format!("{}, {said}", tip.title)
        };
        canvas = canvas.describe("a11y_value", said);
      }
    }
    let mut stack = Stack::new().width(wide).height(height).overflow_visible().child(canvas);
    // The plot's own words, over the canvas and letting the pointer through to it.
    let labels = self.plot.labels(area, &self.look);
    if !labels.is_empty() {
      for placed in labels {
        let label = self.part(placed.part);
        stack = stack.child(at(
          placed.x,
          placed.y,
          Row::new()
            .offset_by_size(placed.offset.0, placed.offset.1)
            .child(label.text(&placed.text).part(&label).nowrap())
            .into(),
        ));
      }
    }
    if let Some(point) = active
      && let Some((x, y)) = self.plot.anchor(area, point)
    {
      let tip = self.plot.tip(point);
      let rows = tip.rows.iter().map(|(series, value)| {
        ChartTooltipRow::new(
          self
            .plot
            .tip_color(point, *series, &self.look)
            .map(crate::node::BackgroundColor::Color)
            .unwrap_or_else(|| self.series_color(*series)),
          &self.series_name(*series),
          &(self.look.value_text)(*value),
        )
      });
      let mut tooltip = ChartTooltip::new(&tip.title).rows(rows);
      if let Some(id) = &self.props.id {
        tooltip = tooltip.id(format!("{id}-tooltip"));
      }
      // Beside the cursor at the point's x, at the plot's top: before it where a tooltip of the board's 140 fits.
      let beside = self.plot.tip_place() == super::TipPlace::Beside;
      // Over the point, centred where half the board's tooltip of 140 fits on each side of it in the plot, else
      // kept inside the plot at its ends; beside a legend (a pie's), after the point, over the legend.
      let across = if beside {
        if x >= TIP_WIDTH + TIP_BESIDE { -1.0 } else { 0.0 }
      } else if self.beside.is_some() || x < 70.0 {
        0.0
      } else if width > 0.0 && x + 70.0 > width {
        -1.0
      } else {
        -0.5
      };
      // Over the point where there is room for it in the plot, under it where there is not: about 30 for the
      // title and 18 a row.
      let tall = 30.0 + 18.0 * tip.rows.len() as f32;
      let (x, top, up) = if beside {
        let x = if across < 0.0 { x - TIP_BESIDE } else { x + TIP_BESIDE };
        (x, TIP_TOP, 0.0)
      } else if y - 8.0 >= tall {
        (x, y - 8.0, -1.0)
      } else {
        (x, y + 8.0, 0.0)
      };
      stack = stack.child(at(
        x,
        top,
        Row::new()
          .offset_by_size(across, up)
          .child(tooltip.build(self.recipe.theme()))
          .into(),
      ));
    }
    let gutter = self.gutter(area);
    if gutter <= 0.0 {
      return self.placed(stack.into());
    }
    // The ticks' numbers stand in a room of their own at the plot's start, each at its line's height.
    let label = self.part(ChartPart::ValueLabel);
    let mut numbers = Stack::new().width(gutter).height(height).overflow_visible();
    for tick in self.plot.ticks(area, &self.look) {
      numbers = numbers.child(at(
        0.0,
        tick.position,
        Row::new()
          .width(gutter - GUTTER_GAP)
          .justify(Justify::End)
          .offset_by_size(0.0, -0.5)
          .child(label.text(&tick.text).part(&label).nowrap())
          .into(),
      ));
    }
    Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Start)
      .child(numbers)
      .child(
        Column::new()
          .flex_full(1.0, 1.0, Some(0.0))
          .min_width(Dimension::Px(0.0))
          .child(stack),
      )
      .into()
  }

  /// How wide the room of the ticks' numbers is: the widest of them in the mono type of 11 (an advance of 0.6 of
  /// its size) and the room after it, or none where the chart does not write them.
  fn gutter(&self, area: ChartArea) -> f32 {
    if !self.value_axis() || self.props.loading || self.plot.is_empty() {
      return 0.0;
    }
    let size = self.recipe.part(ChartPart::ValueLabel).rest().text.size.unwrap_or(11.0);
    // A plot with no ticks (a radar writes its rings' numbers itself) has no room for them.
    let Some(widest) = self
      .plot
      .ticks(area, &self.look)
      .iter()
      .map(|tick| tick.text.chars().count())
      .max()
    else {
      return 0.0;
    };
    (widest as f32 * size * 0.6).ceil() + GUTTER_GAP
  }

  /// The plot where it stands across the frame: as it is where it is as wide as the frame or has the legend beside
  /// it, else in a row of the frame's width at its start, its middle or its end.
  fn placed(&self, plot: Element) -> Element {
    if self.plot_width.is_none() || self.beside.is_some() {
      return plot;
    }
    Row::new()
      .width(Dimension::Pct(100.0))
      .justify(self.plot_justify)
      .overflow_visible()
      .child(plot)
      .into()
  }

  /// The plot's behavior on the canvas of a mounted chart: the pointer finds the point it is on, a press selects
  /// it, and the plot is one stop of Tab whose arrows go between points and series.
  fn dress(&self, canvas: Canvas, host: &Arc<ChartHostState>) -> Canvas {
    let (moved, left, pressed, keyed, focus, blur) = (
      host.clone(),
      host.clone(),
      host.clone(),
      host.clone(),
      host.clone(),
      host.clone(),
    );
    let (plot, plot_keys, plot_focus) = (self.plot.clone(), self.plot.clone(), self.plot.clone());
    let (selected, selected_keys) = (self.props.on_select.clone(), self.props.on_select.clone());
    let reference = host.canvas.clone();
    canvas
      .ref_element(host.canvas.clone())
      .tab_index(0)
      .on_mouse_move(move |event: MouseEvent| {
        let Some(canvas) = moved.canvas.as_canvas() else {
          return;
        };
        let size = canvas.size();
        let point = canvas.point_from_window(event.x, event.y).and_then(|(x, y)| {
          plot.point_at(
            ChartArea {
              width: size.width,
              height: size.height,
            },
            x,
            y,
          )
        });
        moved.hover(point);
      })
      .on_mouse_leave(move || left.hover(None))
      .on_click(move |_: MouseEvent| {
        let point = *pressed.hovered.lock();
        if let Some(point) = point {
          *pressed.focused.lock() = Some(point);
          for handler in &selected {
            handler(point);
          }
        }
      })
      .on_focus(move || {
        let mut focused = focus.focused.lock();
        if focused.is_none() {
          let counts = plot_focus.counts();
          *focused = counts
            .iter()
            .position(|count| *count > 0)
            .map(|series| ChartPoint::new(series, 0));
        }
        drop(focused);
        focus.rebuild();
      })
      .on_blur(move || blur.rebuild())
      .on_key_down(move |event: KeyboardEvent| {
        if !reference.focused() || event.alt || event.ctrl || event.meta {
          return;
        }
        let from = (*keyed.focused.lock()).unwrap_or_default();
        match event.key.as_str() {
          "Enter" | " " => {
            event.prevent_default();
            for handler in &selected_keys {
              handler(from);
            }
          }
          key => {
            if let Some(to) = step(&plot_keys.counts(), from, key) {
              event.prevent_default();
              *keyed.focused.lock() = Some(to);
              // The pointer's point gives way to the one the keys moved to.
              *keyed.hovered.lock() = None;
              keyed.rebuild();
            }
          }
        }
      })
      .describe("a11y_roledescription", "chart")
  }

  fn series_name(&self, series: usize) -> String {
    self
      .series
      .get(series)
      .map_or_else(String::new, |series| series.name().to_owned())
  }

  fn series_color(&self, series: usize) -> crate::node::BackgroundColor {
    self
      .series
      .get(series)
      .and_then(|series| series.given_color().cloned())
      .unwrap_or_else(|| series_color(series).into())
  }

  fn grid(&self) -> bool {
    self.props.grid.unwrap_or(false)
  }

  fn value_axis(&self) -> bool {
    self.props.value_axis.unwrap_or(false)
  }

  fn axis_element(&self) -> Option<Element> {
    if self.props.loading || self.plot.is_empty() {
      return None;
    }
    let label = self.part(ChartPart::AxisLabel);
    let mut row = Row::new()
      .part(&self.part(ChartPart::Axis))
      .width(Dimension::Pct(100.0))
      .describe("a11y_hidden", "true");
    // Under the plot, past the room of the ticks' numbers.
    let gutter = self.gutter(ChartArea {
      width: 0.0,
      height: self.plot_height(),
    });
    if gutter > 0.0 {
      row = row.padding_left(gutter);
    }
    let text = |name: &str| {
      label
        .text(name)
        .part(&label)
        .nowrap()
        .text_overflow(TextOverflow::Elipsis)
        .min_width(Dimension::Px(0.0))
    };
    match self.plot.axis() {
      ChartAxis::None => None,
      ChartAxis::Bands { labels, gap } => Some(
        row
          .spacing(gap)
          .with_children(labels.iter().map(|name| {
            Row::new()
              .flex_full(1.0, 1.0, Some(0.0))
              .min_width(Dimension::Px(0.0))
              .justify(Justify::Center)
              .child(text(name))
          }))
          .into(),
      ),
      ChartAxis::Spread(labels) => Some(
        row
          .justify(Justify::SpaceBetween)
          .with_children(labels.iter().map(|name| text(name)))
          .into(),
      ),
    }
  }

  fn legend_element(&mut self) -> Option<Element> {
    if let Some(own) = self.own_legend.take() {
      return (self.props.legend != Some(false) && !self.props.loading).then_some(own);
    }
    if !self.props.legend.unwrap_or(self.series.len() > 1) || self.props.loading {
      return None;
    }
    let values = self
      .beside
      .as_ref()
      .map(|beside| beside.values.as_slice())
      .unwrap_or(&[]);
    let items = self.series.iter().enumerate().map(|(index, series)| {
      let item = ChartLegendItem::new(series.name(), self.series_color(index));
      match values.get(index).filter(|value| !value.is_empty()) {
        Some(value) => item.value(value),
        None => item,
      }
    });
    let mut legend = ChartLegend::new(items).shape(ChartLegendShape::Dot);
    if let Some(beside) = &self.beside {
      legend = legend.orientation(Orientation::Vertical).size(beside.size);
    }
    if let Some(write) = &self.props.write_hidden {
      let write = write.clone();
      legend = legend.hidden_controlled(self.props.hidden.clone(), move |next| write(next));
    }
    for handler in &self.props.on_hidden_change {
      let handler = handler.clone();
      legend = legend.on_hidden_change(move |next| handler(next));
    }
    if let Some(id) = &self.props.id {
      legend = legend.id(format!("{id}-legend"));
    }
    Some(legend.build(self.recipe.theme()))
  }

  /// The chart mounted where it is declared: the pointer finds the point it is on and the tooltip shows it, the
  /// plot is one stop of Tab whose arrows go between points and series, and a press tells `on_select`.
  /// `make` builds the frame in the theme of each render.
  pub fn mount(
    ctx: &mut Ctx,
    key: Option<Arc<str>>,
    make: impl Fn(&Theme) -> ChartFrame + Send + Sync + 'static,
  ) -> Element {
    let props = ChartHostProps { make: Arc::new(make) };
    match key {
      Some(key) => ctx.mount_keyed::<ChartHost>(&key, props),
      None => ctx.mount::<ChartHost>(props),
    }
  }
}

/// The room between the ticks' numbers and the plot.
const GUTTER_GAP: f32 = 8.0;
/// The board's tooltip's width, and how far one beside the cursor is from it and from the plot's top.
const TIP_WIDTH: f32 = 140.0;
const TIP_BESIDE: f32 = 10.0;
const TIP_TOP: f32 = 4.0;

/// `child` at `(x, y)` of the plot, over the canvas and letting the pointer through to it: a line of the plot's
/// size from that place, which the child stands in or out of.
fn at(x: f32, y: f32, child: Element) -> Row {
  Row::new()
    .absolute(x, y, Dimension::Pct(100.0), Dimension::Pct(100.0))
    .align_items(Alignment::Start)
    .overflow_visible()
    .inert(true)
    .child(child)
}

/// Draws `plot` on `canvas`, its grid first: what a chart's canvas is painted with.
fn draw(canvas: &CanvasHandle, plot: &dyn ChartPlot, look: &ChartLook, active: Option<ChartPoint>, grid: bool) {
  let size = canvas.size();
  let area = ChartArea {
    width: size.width,
    height: size.height,
  };
  let scale = canvas.scale_factor().max(1.0);
  let context = canvas.context_2d();
  context.reset();
  context.clear();
  // A line of one pixel is one pixel at any scale, on a whole one.
  let line = (scale.round().max(1.0)) / scale;
  for tick in plot.ticks(area, look) {
    if !tick.baseline && !grid {
      continue;
    }
    let color = look.color(if tick.baseline {
      ChartPart::Baseline
    } else {
      ChartPart::Grid
    });
    let y = snap((tick.position - line).max(0.0).min(area.height - line), scale);
    context.set_fill_style(color);
    context.fill_rect(0.0, y, area.width, line);
  }
  plot.paint(&context, area, look, active);
  canvas.set_items(plot.items(area, look));
}

/// `value` on a whole device pixel at `scale`.
pub fn snap(value: f32, scale: f32) -> f32 {
  (value * scale).round() / scale
}

/// A chart as a renderer sees it: its header's words, its plot, its axis and its legend, built.
pub struct ChartView {
  /// Whether the legend stands beside the plot, not under it.
  pub beside: bool,
  /// The plot's width, where it is not the frame's.
  pub plot_width: Option<f32>,
  pub title: Option<Arc<str>>,
  pub description: Option<Arc<str>>,
  /// Its name to tooling.
  pub name: Arc<str>,
  /// The plot: the canvas with its grid, its tooltip and its keys, or the empty state, or the skeleton.
  pub plot: Element,
  /// The names under the plot, where it has them.
  pub axis: Option<Element>,
  pub legend: Option<Element>,
  pub recipe: ResolvedRecipe,
  forced: Option<Interaction>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl ChartView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part).forced(self.forced)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  /// The box: the card the chart is drawn in, a figure to tooling with its name and its description.
  pub fn root(&self) -> Column {
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(ChartPart::Root))
      .describe("a11y_role", "figure")
      .describe("a11y_name", self.name.clone());
    if let Some(description) = &self.description {
      root = root.describe("a11y_description", description.clone());
    }
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root
  }

  /// The title over the description, each wrapping: `None` where it has neither.
  pub fn header(&self) -> Option<Column> {
    if self.title.is_none() && self.description.is_none() {
      return None;
    }
    let mut header = Column::new()
      .part(&self.part(ChartPart::Header))
      .align_items(Alignment::Stretch);
    if let Some(title) = &self.title {
      let part = self.part(ChartPart::Title);
      header = header.child(part.text(title).part(&part));
    }
    if let Some(description) = &self.description {
      let part = self.part(ChartPart::Description);
      header = header.child(part.text(description).part(&part));
    }
    Some(header)
  }

  /// How the default renderer builds a chart: the header, the plot, the axis and the legend, one under another; or
  /// the plot and the legend beside it in a row, under the header.
  pub fn render(mut self) -> Element {
    let mut root = self.root();
    if let Some(header) = self.header() {
      root = root.child(header);
    }
    let plot = std::mem::replace(&mut self.plot, Row::new().into());
    if self.beside {
      // The plot and the legend beside it, the legend taking the room the plot leaves. They are laid one over the
      // other, the legend moved past the plot, so that the plot's tooltip stands over the legend.
      let gap = self
        .part(ChartPart::Body)
        .rest()
        .gap
        .map(|gap| gap.resolve(&self.theme().spacing(), 0.0))
        .unwrap_or(20.0);
      let mut body = Stack::with(StackAlignment::CenterStart, Vec::<Element>::new())
        .width(Dimension::Pct(100.0))
        .overflow_visible();
      if let Some(legend) = self.legend.take() {
        body = body.child(
          Row::new()
            .width(Dimension::Pct(100.0))
            .padding_left(self.plot_width.unwrap_or(0.0) + gap)
            .child(
              Column::new()
                .flex_full(1.0, 1.0, Some(0.0))
                .min_width(Dimension::Px(0.0))
                .child(legend),
            ),
        );
      }
      return root.child(body.child(plot)).into();
    }
    root = root.child(plot);
    if let Some(axis) = self.axis.take() {
      root = root.child(axis);
    }
    if let Some(legend) = self.legend.take() {
      root = root.child(legend);
    }
    root.into()
  }
}

/// The recipe under every chart's: the board's chart card (`Sk5h8` and its siblings) on the card's fill with an
/// edge of 1, the extra large radius and the small shadow; `Md` padded 24 with 20 between its rows, the title 16/600
/// over the description 13 in the muted foreground, 4 apart, a plot of 160; `Sm` padded 20 with 16 between, a title
/// of 15 over a description of 12, 3 apart, a plot of 140. The axis's names mono 11 in the muted foreground with 12
/// between bands; the grid lines 1 in the border's color at 0.6, the line the values stand on in the border's
/// color; the cursor at the point under the pointer in the input's color; the ring of the focus from the keyboard
/// around the plot. A plot with its legend beside it (a pie's) stands 20 from it, 18 in `Sm`.
pub fn chart_recipe() -> Recipe {
  use ChartPart::{
    Axis, AxisLabel, Baseline, Body, Cursor, Description, Empty, Grid, Header, Plot, Root, Title, ValueLabel,
  };
  use ChartSize::Sm;
  use PaletteColor::{Card, CardForeground, Input, MutedForeground};
  Recipe::new()
    .base(Root, |s| {
      s.width(Length::Fill)
        .background(Card)
        .foreground(CardForeground)
        .radius(RadiusSize::Xl)
        .border(Edge::inside(BorderSize::Sm, PaletteColor::Border))
        .shadow(ShadowStyle::Sm)
        .padding_x(24.0)
        .padding_y(24.0)
        .gap(20.0)
    })
    .size(Sm, Root, |s| s.padding_x(20.0).padding_y(20.0).gap(16.0))
    .base(Header, |s| s.gap(4.0))
    .size(Sm, Header, |s| s.gap(3.0))
    .base(Title, |s| {
      s.typography(TypographyStyle::Body)
        .font_size(16.0)
        .weight(FontWeight::SemiBold)
        .foreground(CardForeground)
    })
    .size(Sm, Title, |s| s.font_size(15.0))
    .base(Description, |s| {
      s.typography(TypographyStyle::Body)
        .font_size(13.0)
        .foreground(MutedForeground)
    })
    .size(Sm, Description, |s| s.font_size(12.0))
    .base(Plot, |s| s.height(160.0))
    .size(Sm, Plot, |s| s.height(140.0))
    .focus_visible(Plot, |s| {
      s.outline(Outline::new(BorderSize::Md, PaletteColor::Ring).offset(2.0))
    })
    .base(Grid, |s| s.background(PaletteColor::Border).opacity(0.6))
    .base(Baseline, |s| s.background(PaletteColor::Border))
    .base(Cursor, |s| s.background(Input))
    .base(Axis, |s| s.gap(12.0))
    .base(AxisLabel, |s| {
      s.font_family(FontFamily::Mono)
        .font_size(11.0)
        .foreground(MutedForeground)
    })
    .base(ValueLabel, |s| {
      s.font_family(FontFamily::Mono)
        .font_size(11.0)
        .foreground(MutedForeground)
    })
    .base(Empty, |s| {
      s.typography(TypographyStyle::Body)
        .font_size(13.0)
        .foreground(MutedForeground)
    })
    .base(Body, |s| s.gap(20.0))
    .size(Sm, Body, |s| s.gap(18.0))
}

struct ChartHostProps {
  make: Arc<dyn Fn(&Theme) -> ChartFrame + Send + Sync>,
}

impl PartialEq for ChartHostProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for ChartHostProps {}

/// What keeps a mounted chart: the point under the pointer and the focused one.
struct ChartHost {
  state: Arc<ChartHostState>,
  _marker: PhantomData<()>,
}

impl Component for ChartHost {
  type Props = ChartHostProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      state: Arc::new(ChartHostState {
        hovered: Mutex::new(None),
        focused: Mutex::new(None),
        canvas: ctx.element_ref(),
        revision: ctx.signal(0_u64),
      }),
      _marker: PhantomData,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let _ = self.state.revision.get();
    let make = ctx.props::<ChartHostProps>().make.clone();
    let frame = make(&current_theme());
    // A point the data no longer has is let go.
    let counts = frame.plot.counts();
    let fits = |point: &Option<ChartPoint>| {
      point.is_some_and(|point| counts.get(point.series).is_some_and(|count| point.index < *count))
    };
    for held in [&self.state.hovered, &self.state.focused] {
      let mut held = held.lock();
      if held.is_some() && !fits(&held) {
        *held = None;
      }
    }
    frame.render_with(Some(self.state.clone()))
  }
}
