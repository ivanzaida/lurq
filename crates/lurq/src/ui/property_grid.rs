//! A property grid: an inspector's names with their editors, in categories that open and close.

use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

use super::{
  Checkbox, ColorPicker, ColorSwatch, ColorSwatchSize, Input, InputPart, NumberFormat, NumberInput, NumberInputLayout,
  Popover, Select, Slider, icons, names,
  opening_rows::{OpeningRows, RowMotion},
  recipe::{Case, Interaction, Name, Recipe, ResolvedPart, ResolvedRecipe},
  shared::{fading, glyph, pressable},
  theme::{Themed, current_theme},
};
use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{DragEvent, KeyboardEvent, MouseButtonMask},
    theme::{OpacityLevel, PaletteColor, RadiusSize, Theme, TypographyStyle},
  },
  components::{self, Column, Row, SelectOption, Stack, VirtualizedList},
  core::{ElementRef, Signal},
  layout::{
    Alignment,
    layout_kind::{Justify, ScrollState},
    text_style::FontWeight,
  },
  node::{
    CursorIcon, Element, FocusGroup, FocusRows, border::Border as Edge, color::Color, dimension::Dimension,
    node_kind::TextOverflow,
  },
};

/// The parts of a property grid a recipe styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropertyGridPart {
  /// The grid: its header, its categories and their rows.
  Root,
  /// The row at the top with the title and what the app puts at its end.
  Header,
  /// The header's title: "PROPERTIES".
  Title,
  /// A category's row: what opens and closes it.
  Category,
  /// What says a category opens: down while it is open, turned to the end while it is closed.
  Chevron,
  /// What opens and closes under a category's row: as high as its rows while it is open, of no height while it is
  /// closed, with a transition between the two.
  Panel,
  /// The rows in a category's panel: they fade in as it opens and out as it closes.
  Body,
  /// A category's title.
  CategoryTitle,
  /// A property's row: its name and its editor.
  Row,
  /// A property's name.
  Label,
  /// The mark of a property whose value is not its default.
  Modified,
  /// What holds a property's editors.
  Value,
  /// The box of an editor, laid over the box of the control it is: its height, its edge, its type.
  Editor,
  /// The letter of a value of two (W, H), in its editor.
  Axis,
  /// The letter X of a position.
  AxisX,
  /// The letter Y of a position.
  AxisY,
  /// What puts a property's default back.
  Reset,
  /// The chevrons at the end of a number's box that step it; the eyedropper at the end of a color's.
  Stepper,
  /// A color's swatch, outside its box.
  Swatch,
  /// What is dragged to share the width between the names and the editors.
  Divider,
}

names!(PropertyGridPart {
  Root => "root",
  Header => "header",
  Title => "title",
  Category => "category",
  Chevron => "chevron",
  Panel => "panel",
  Body => "body",
  CategoryTitle => "category_title",
  Row => "row",
  Label => "label",
  Modified => "modified",
  Value => "value",
  Editor => "editor",
  Axis => "axis",
  AxisX => "axis_x",
  AxisY => "axis_y",
  Reset => "reset",
  Stepper => "stepper",
  Swatch => "swatch",
  Divider => "divider",
});

/// The states of a property grid a recipe styles, besides what the pointer and the keyboard do to it. A row's and
/// a category's own are resolved on their parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropertyGridState {
  /// Nothing in it takes input.
  Disabled,
  /// It is as wide as what holds it.
  FullWidth,
  /// A category that is open.
  Open,
  /// A property whose value is not its default.
  Modified,
  /// A property that is read and not changed.
  ReadOnly,
  /// A property, or a category, that takes no input.
  Unavailable,
}

names!(PropertyGridState {
  Disabled => "disabled",
  FullWidth => "full_width",
  Open => "open",
  Modified => "modified",
  ReadOnly => "read_only",
  Unavailable => "unavailable",
});

/// What builds a property grid's nodes: a theme's in place of the default one.
pub type PropertyGridRenderer = Arc<dyn Fn(PropertyGridView) -> Element + Send + Sync>;

/// The width of the names unless the app or a drag says otherwise: the board's.
pub const PROPERTY_NAME_WIDTH: f32 = 84.0;
/// The least width of the names and of the editors as the divider is dragged.
const LEAST: f32 = 48.0;

type Notify = Arc<dyn Fn() + Send + Sync>;

/// What an editor is built with: the rules of the grid's editor part laid over the control's box, its name, and
/// what tells the grid it changed.
#[derive(Clone)]
pub struct EditorContext {
  pub theme: Theme,
  /// The grid's `Editor` part, as rules for each control's box.
  editor: ResolvedPart,
  axis: [ResolvedPart; 3],
  pub label: Arc<str>,
  pub read_only: bool,
  pub disabled: bool,
  /// Whether a number has its stepper at the end of its box, or a color its eyedropper.
  pub stepper: bool,
  notify: Notify,
  /// The texts a person edited and has not committed yet, kept by the mounted grid.
  edited: Option<(Arc<str>, Edited)>,
  /// What a color row keeps from render to render where the grid is mounted: whether its picker is open, and the
  /// hex being typed.
  color: Option<ColorKeep>,
  /// The grid's stepper, swatch and eyedropper parts.
  stepper_part: ResolvedPart,
  swatch_part: ResolvedPart,
}

/// What a color row keeps.
#[derive(Clone)]
struct ColorKeep {
  open: Signal<bool>,
  draft: Signal<String>,
}

/// The ids of the texts edited since they were last told.
type Edited = Arc<Mutex<std::collections::HashSet<Arc<str>>>>;

impl EditorContext {
  /// The editor part's rules laid over the part `inner` of a control, its padding left to the control: for a box
  /// that holds something of its own at its start (a color field's swatch).
  pub fn rules_for_unpadded(&self, inner: impl Into<Name>) -> Recipe {
    let mut styles = self.editor.styles().clone();
    styles.rest.padding_x = None;
    styles.rest.padding_start = None;
    styles.rest.padding_end = None;
    let inner = inner.into();
    Recipe::new()
      .base(inner, |_| styles.rest.clone())
      .hovered(inner, |_| styles.hovered.clone())
      .focus_visible(inner, |_| styles.focus_visible.clone())
  }

  /// The editor part's rules laid over the part `inner` of a control.
  pub fn rules_for(&self, inner: impl Into<Name>) -> Recipe {
    let styles = self.editor.styles().clone();
    let inner = inner.into();
    Recipe::new()
      .base(inner, |_| styles.rest.clone())
      .hovered(inner, |_| styles.hovered.clone())
      .focus_visible(inner, |_| styles.focus_visible.clone())
  }

  /// The rules of the grid's `Swatch` part laid over a color swatch's root.
  fn swatch_rules(&self) -> Recipe {
    let styles = self.swatch_part.styles().clone();
    Recipe::new().base(super::ColorSwatchPart::Root, |_| styles.rest.clone())
  }

  /// How far in from an editor's edge its content starts: its padding.
  fn inset(&self) -> f32 {
    let rest = self.editor.rest();
    rest
      .padding_end
      .or(rest.padding_x.clone())
      .map(|padding: crate::node::SpacingValue| padding.resolve(&self.theme.spacing(), 0.0))
      .unwrap_or(6.0)
  }

  /// A length in pixels, where it is one.
  fn px(&self, length: Option<super::part_style::Length>) -> Option<f32> {
    match length?.resolve(&self.theme)? {
      Dimension::Px(px) => Some(px),
      _ => None,
    }
  }

  /// Tells the grid that a person changed the value: after it is set.
  pub fn changed(&self) {
    (self.notify)();
  }

  /// A text committed: told, and no longer edited.
  fn committed(&self) {
    if let Some((id, edited)) = &self.edited {
      edited.lock().unwrap().remove(id);
    }
    (self.notify)();
  }
}

type BuildEditor = Arc<dyn Fn(&EditorContext) -> Element + Send + Sync>;

/// One property of a grid: its name and the editor of its value, of the type `V` it holds. A category takes any
/// property, of any type.
pub struct Property<V> {
  name: Arc<str>,
  editor: BuildEditor,
  get: Option<Arc<dyn Fn() -> V + Send + Sync>>,
  set: Option<Arc<dyn Fn(V) + Send + Sync>>,
  default: Option<V>,
  value_text: Option<Arc<dyn Fn() -> String + Send + Sync>>,
  description: Option<Arc<str>>,
  read_only: bool,
  disabled: bool,
  stepper: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl<V: Clone + PartialEq + Send + Sync + crate::core::SignalValue + 'static> Property<V> {
  fn bound(
    name: &str,
    value: &Signal<V>,
    text: impl Fn(&V) -> String + Send + Sync + 'static,
    editor: impl Fn(&EditorContext) -> Element + Send + Sync + 'static,
  ) -> Self {
    let (got, set, shown) = (value.clone(), value.clone(), value.clone());
    Self {
      name: Arc::from(name),
      editor: Arc::new(editor),
      get: Some(Arc::new(move || got.get_untracked())),
      set: Some(Arc::new(move |to| set.set(to))),
      default: None,
      value_text: Some(Arc::new(move || text(&shown.get_untracked()))),
      description: None,
      read_only: false,
      disabled: false,
      stepper: false,
      id: None,
      key: None,
    }
  }

  /// The value the property's reset puts back: while the value is another, the name is marked and the reset is
  /// shown.
  pub fn default(mut self, value: V) -> Self {
    self.default = Some(value);
    self
  }
}

impl<V> Property<V> {
  /// What the property is, said to tooling with its row and its editor.
  pub fn description(mut self, description: &str) -> Self {
    self.description = Some(Arc::from(description));
    self
  }

  /// Whether its value is read, selected and copied, and not changed.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.read_only = read_only;
    self
  }

  /// Whether it takes no input, dimmed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// What the grid's `on_change` is told of it, in place of its name; the id of its row's element under the
  /// grid's id.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of its row's element.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }
}

/// A number as an inspector writes it: no decimals where it has none.
fn number_text(value: f64) -> String {
  if value.fract() == 0.0 && value.abs() < 1e15 {
    format!("{}", value as i64)
  } else {
    format!("{}", (value * 1000.0).round() / 1000.0)
  }
}

impl Property<String> {
  /// A text typed in an input.
  pub fn text(name: &str, value: Signal<String>) -> Self {
    let bound = value.clone();
    Self::bound(
      name,
      &value,
      |text| text.clone(),
      move |cx| {
        // A text is told as it is committed, after it is set: Enter, or the focus leaving it once it was edited.
        let submitted = cx.clone();
        let mut input = Input::new(bound.clone())
          .label(&cx.label)
          .read_only(cx.read_only)
          .disabled(cx.disabled)
          .full_width(true)
          .rules(&cx.rules_for(InputPart::Control))
          .on_submit(move |_| submitted.committed());
        if let Some((id, edited)) = cx.edited.clone() {
          let (left, marked) = (cx.clone(), edited.clone());
          let marked_id = id.clone();
          input = input.on_input(move |event: &crate::app::events::TextInputEvent| {
            if !event.default_prevented() {
              marked.lock().unwrap().insert(marked_id.clone());
            }
          });
          input.on_leave.push(Arc::new(move || {
            if edited.lock().unwrap().contains(&id) {
              left.committed();
            }
          }));
        }
        input.into()
      },
    )
  }
}

impl Property<f64> {
  /// Whether the number has its buttons, one over the other at the end of its box: the board's rotation. Without
  /// them it is stepped with Up and Down.
  pub fn stepper(mut self, stepper: bool) -> Self {
    self.stepper = stepper;
    self
  }

  /// A number typed in a number input, and stepped with Up and Down.
  pub fn number(name: &str, value: Signal<f64>) -> Self {
    Self::number_with(name, value, "")
  }

  /// A number with a unit written after it: `°`, `%`, `px`.
  pub fn number_with(name: &str, value: Signal<f64>, suffix: &str) -> Self {
    let (bound, suffix): (_, Arc<str>) = (value.clone(), Arc::from(suffix));
    let unit = suffix.clone();
    Self::bound(
      name,
      &value,
      move |number| format!("{}{unit}", number_text(*number)),
      move |cx| {
        let changed = cx.clone();
        let mut field = NumberInput::new(bound.clone())
          .layout(NumberInputLayout::Bare)
          .format(NumberFormat::new().suffix(&suffix))
          .label(&cx.label)
          .read_only(cx.read_only)
          .disabled(cx.disabled)
          .full_width(true)
          .rules(&cx.rules_for(super::NumberInputPart::Control))
          .on_change(move |_| changed.changed());
        if !cx.stepper {
          return field.into();
        }
        // The board's stepper: one control of chevrons at the end of the box, its upper half a step up and its
        // lower half a step down. The keys step the number in the box; the stepper takes no focus.
        let side = cx.px(cx.stepper_part.rest().width).unwrap_or(12.0);
        let inset = cx.inset();
        field = field.style(super::NumberInputPart::Control, move |s: super::PartStyle| {
          s.padding_end(inset + side + 4.0)
        });
        let able = !cx.read_only && !cx.disabled;
        let half = |up: bool| {
          let (value, told) = (bound.clone(), cx.clone());
          let mut half = components::Button::empty()
            .width(side)
            .height(side / 2.0)
            .focusable(false)
            .disabled(!able)
            .describe("a11y_role", "button")
            .describe("a11y_name", {
              let locale = super::current_locale();
              if up {
                locale.fields.increase.clone()
              } else {
                locale.fields.decrease.clone()
              }
            });
          if able {
            half = half.on_click(move |_| {
              value.update(|number| *number += if up { 1.0 } else { -1.0 });
              told.changed();
            });
          }
          half
        };
        let stepper = Stack::new()
          .child(glyph(&cx.stepper_part, Name::from(icons::CHEVRONS_VERTICAL), false))
          .child(Column::new().child(half(true)).child(half(false)));
        Stack::new()
          .width(Dimension::Pct(100.0))
          .stack_align(crate::layout::StackAlignment::CenterEnd)
          .child(field)
          .child(Row::new().padding_right(inset).child(stepper))
          .into()
      },
    )
  }
}

impl Property<f32> {
  /// A number between `min` and `max` on a slider, and typed beside it: an opacity, a volume.
  pub fn slider(name: &str, value: Signal<f32>, min: f32, max: f32, suffix: &str) -> Self {
    let (bound, suffix): (_, Arc<str>) = (value.clone(), Arc::from(suffix));
    let unit = suffix.clone();
    let typed = Signal::new(f64::from(value.get_untracked()));
    Self::bound(
      name,
      &value,
      move |number| format!("{}{unit}", number_text(f64::from(*number))),
      move |cx| {
        // The number typed beside the slider follows the value, and sets it.
        if typed.get_untracked() != f64::from(bound.get_untracked()) {
          typed.set(f64::from(bound.get_untracked()));
        }
        let (slid, typed_changed, to) = (cx.clone(), cx.clone(), bound.clone());
        let slider = Slider::new(bound.clone())
          .min(min)
          .max(max)
          .label(&cx.label)
          .disabled(cx.disabled || cx.read_only)
          .on_change(move |_| slid.changed());
        let field = NumberInput::new(typed.clone())
          .layout(NumberInputLayout::Bare)
          .min(f64::from(min))
          .max(f64::from(max))
          .format(NumberFormat::new().suffix(&suffix))
          .label(&cx.label)
          .read_only(cx.read_only)
          .disabled(cx.disabled)
          .rules(&cx.rules_for(super::NumberInputPart::Control))
          .style(super::NumberInputPart::Root, |s: super::PartStyle| s.width(56.0))
          .on_change(move |number| {
            if let Some(number) = number {
              to.set(number as f32);
              typed_changed.changed();
            }
          });
        Row::new()
          .width(Dimension::Pct(100.0))
          .align_items(Alignment::Center)
          .spacing(8.0)
          .child(Row::new().flex(1.0).child(slider))
          .child(field)
          .into()
      },
    )
  }
}

impl Property<bool> {
  /// A setting that is on or off: a checkbox.
  pub fn toggle(name: &str, value: Signal<bool>) -> Self {
    let bound = value.clone();
    Self::bound(
      name,
      &value,
      |on| if *on { "on" } else { "off" }.to_owned(),
      move |cx| {
        let changed = cx.clone();
        Checkbox::new(&cx.label)
          .label_hidden(true)
          .bind(bound.clone())
          .disabled(cx.disabled || cx.read_only)
          .on_change(move |_| changed.changed())
          .into()
      },
    )
  }
}

impl Property<Color> {
  /// A color: its swatch, a press of which opens the picker, and its hex typed in a box beside it, as the board's
  /// inspector draws it. With [`eyedropper`](Property::eyedropper) the box has the picker's eyedropper at its end.
  pub fn color(name: &str, value: Signal<Color>) -> Self {
    let bound = value.clone();
    Self::bound(
      name,
      &value,
      |color| color.to_hex_upper(),
      move |cx| color_row(cx, &bound),
    )
  }

  /// Whether the hex's box has the eyedropper at its end, which opens the picker too.
  pub fn eyedropper(mut self, eyedropper: bool) -> Self {
    self.stepper = eyedropper;
    self
  }
}

/// A hex as a person types it, `#RGB`, `#RRGGBB` or `#RRGGBBAA` with or without its `#`: the color it says.
fn parse_hex(text: &str) -> Option<Color> {
  let digits = text.trim().trim_start_matches('#');
  let valid = matches!(digits.len(), 3 | 6 | 8) && digits.chars().all(|c| c.is_ascii_hexdigit());
  valid.then(|| Color::from_hex(&format!("#{digits}")))
}

/// The board's color row: a swatch of 24 outside the box, the hex in the box.
fn color_row(cx: &EditorContext, value: &Signal<Color>) -> Element {
  let color = value.get_untracked();
  let hex = color.to_hex_upper();
  let keep = cx.color.clone().unwrap_or_else(|| ColorKeep {
    open: Signal::new(false),
    draft: Signal::new(hex.clone()),
  });
  let edited = cx
    .edited
    .as_ref()
    .is_some_and(|(id, edited)| edited.lock().unwrap().contains(id));
  // What the box shows follows the color, but for a hex being typed.
  if !edited && keep.draft.get_untracked() != hex {
    keep.draft.set(hex.clone());
  }
  let able = !cx.read_only && !cx.disabled;
  let anchor = ElementRef::new();
  let commit = {
    let (draft, value, told) = (keep.draft.clone(), value.clone(), cx.clone());
    move || {
      if let Some(id_edited) = &told.edited {
        id_edited.1.lock().unwrap().remove(&id_edited.0);
      }
      match parse_hex(&draft.get_untracked()) {
        Some(color) if color != value.get_untracked() => {
          value.set(color);
          told.changed();
        }
        Some(_) => {}
        // What is not a color is put back.
        None => draft.set(value.get_untracked().to_hex_upper()),
      }
    }
  };
  let submitted = commit.clone();
  let mut input = Input::new(keep.draft.clone())
    .label(&cx.label)
    .read_only(cx.read_only)
    .disabled(cx.disabled)
    .full_width(true)
    .rules(&cx.rules_for(InputPart::Control))
    .on_submit(move |_| submitted());
  if let Some((id, edited)) = cx.edited.clone() {
    input = input.on_input(move |event: &crate::app::events::TextInputEvent| {
      if !event.default_prevented() {
        edited.lock().unwrap().insert(id.clone());
      }
    });
    input.on_leave.push(Arc::new(commit));
  }
  let toggle = |open: &Signal<bool>| {
    let open = open.clone();
    move |_: crate::app::events::MouseEvent| open.update(|open| *open = !*open)
  };
  let mut swatch = components::Button::empty()
    .focusable(false)
    .disabled(!able)
    .ref_element(anchor.clone())
    .describe(
      "a11y_name",
      crate::core::fill(&super::current_locale().fields.picker, &[("name", &cx.label)]),
    )
    .describe("a11y_haspopup", "dialog")
    .child(
      ColorSwatch::new(color)
        .size(ColorSwatchSize::Md)
        .rules(&cx.swatch_rules())
        .build(&cx.theme),
    );
  if able && cx.color.is_some() {
    swatch = swatch.on_click(toggle(&keep.open));
  }
  let mut field: Element = input.into();
  if cx.stepper {
    // The eyedropper at the end of the box, which opens the picker as the swatch does.
    let mut dropper = components::Button::empty()
      .focusable(false)
      .disabled(!able)
      .describe(
        "a11y_name",
        crate::core::fill(&super::current_locale().fields.picker, &[("name", &cx.label)]),
      )
      .child(glyph(&cx.stepper_part, Name::from(icons::EYEDROPPER), false));
    if able && cx.color.is_some() {
      dropper = dropper.on_click(toggle(&keep.open));
    }
    field = Stack::new()
      .width(Dimension::Pct(100.0))
      .stack_align(crate::layout::StackAlignment::CenterEnd)
      .child(field)
      .child(Row::new().padding_right(cx.inset()).child(dropper))
      .into();
  }
  let mut row = Row::new()
    .width(Dimension::Pct(100.0))
    .align_items(Alignment::Center)
    .spacing(6.0)
    .child(swatch)
    .child(Row::new().flex(1.0).min_width(Dimension::Px(0.0)).child(field));
  if cx.color.is_some() {
    let (value, told) = (value.clone(), cx.clone());
    let picker = ColorPicker::controlled(color, move |color| {
      if color != value.get_untracked() {
        value.set(color);
        told.changed();
      }
    })
    .bare(true);
    row = row.child(
      Popover::new(keep.open.clone(), anchor)
        .label(&crate::core::fill(
          &super::current_locale().fields.picker,
          &[("name", &cx.label)],
        ))
        .child(picker.build(&cx.theme)),
    );
  }
  row.into()
}

impl Property<(f64, f64)> {
  /// A position: X and Y, each typed in its own box, X in the destructive color and Y in the success color as the
  /// axes are drawn.
  pub fn xy(name: &str, x: Signal<f64>, y: Signal<f64>) -> Self {
    Self::two(name, ("X", x), ("Y", y), true)
  }

  /// A value of two numbers, each typed in its own box with its letter: a width and a height (`("W", w)`, `("H",
  /// h)`).
  pub fn pair(name: &str, first: (&str, Signal<f64>), second: (&str, Signal<f64>)) -> Self {
    Self::two(name, first, second, false)
  }

  fn two(name: &str, first: (&str, Signal<f64>), second: (&str, Signal<f64>), axes: bool) -> Self {
    let (letters, values): ([Arc<str>; 2], [Signal<f64>; 2]) =
      ([Arc::from(first.0), Arc::from(second.0)], [first.1, second.1]);
    let (got, set, shown) = (values.clone(), values.clone(), values.clone());
    let shown_letters = letters.clone();
    let editor = move |cx: &EditorContext| -> Element {
      let mut row = Row::new()
        .width(Dimension::Pct(100.0))
        .align_items(Alignment::Center)
        .spacing(6.0);
      for (index, (letter, value)) in letters.iter().zip(values.iter()).enumerate() {
        let axis = match (axes, index) {
          (true, 0) => &cx.axis[1],
          (true, _) => &cx.axis[2],
          _ => &cx.axis[0],
        };
        let changed = cx.clone();
        let rest = cx.editor.rest();
        let inset = rest
          .padding_start
          .or(rest.padding_x.clone())
          .map(|padding: crate::node::SpacingValue| padding.resolve(&cx.theme.spacing(), 0.0))
          .unwrap_or(6.0);
        let field = NumberInput::new(value.clone())
          .layout(NumberInputLayout::Bare)
          .label(&format!("{} {letter}", cx.label))
          .read_only(cx.read_only)
          .disabled(cx.disabled)
          .full_width(true)
          .rules(&cx.rules_for(super::NumberInputPart::Control))
          // Room at the start of the box for the letter laid over it.
          .style(super::NumberInputPart::Control, move |s: super::PartStyle| s.padding_start(inset + 14.0))
          .on_change(move |_| changed.changed());
        let high = rest
          .height
          .and_then(|height| match height.resolve(&cx.theme) {
            Some(Dimension::Px(px)) => Some(px),
            _ => None,
          })
          .unwrap_or(24.0);
        let letter = Row::new()
          .inert(true)
          .height(high)
          .padding_left(inset)
          .align_items(Alignment::Center)
          .part(axis)
          .child(axis.text(letter).nowrap());
        row = row.child(Stack::new().flex(1.0).child(field).child(letter));
      }
      row.into()
    };
    Self {
      name: Arc::from(name),
      editor: Arc::new(editor),
      get: Some(Arc::new(move || (got[0].get_untracked(), got[1].get_untracked()))),
      set: Some(Arc::new(move |(first, second)| {
        set[0].set(first);
        set[1].set(second);
      })),
      default: None,
      value_text: Some(Arc::new(move || {
        format!(
          "{} {}, {} {}",
          shown_letters[0],
          number_text(shown[0].get_untracked()),
          shown_letters[1],
          number_text(shown[1].get_untracked())
        )
      })),
      description: None,
      read_only: false,
      disabled: false,
      stepper: false,
      id: None,
      key: None,
    }
  }
}

impl<T: Clone + PartialEq + Send + Sync + crate::core::SignalValue + 'static> Property<T> {
  /// One of `options`, chosen in a select: a blend mode, an alignment.
  pub fn choice(name: &str, value: Signal<T>, options: impl IntoIterator<Item = impl Into<SelectOption<T>>>) -> Self {
    let options: Arc<Vec<SelectOption<T>>> = Arc::new(options.into_iter().map(Into::into).collect());
    let (bound, said) = (value.clone(), options.clone());
    let text = move |chosen: &T| {
      said
        .iter()
        .find(|option: &&SelectOption<T>| option.value() == chosen)
        .map(|option: &SelectOption<T>| option.label().to_string())
        .unwrap_or_default()
    };
    Self::bound(name, &value, text, move |cx| {
      let changed = cx.clone();
      Select::<T>::new(bound.clone(), options.iter().cloned())
        .label(&cx.label)
        .read_only(cx.read_only)
        .disabled(cx.disabled)
        .full_width(true)
        .rules(&cx.rules_for(super::SelectPart::Trigger))
        .on_change(move |_: &T| changed.changed())
        .into()
    })
  }
}

impl Property<()> {
  /// Any element as the editor: what the grid has no editor for. It has no default and tells the grid nothing.
  pub fn custom(name: &str, editor: impl Fn() -> Element + Send + Sync + 'static) -> Self {
    Self {
      name: Arc::from(name),
      editor: Arc::new(move |_| editor()),
      get: None,
      set: None,
      default: None,
      value_text: None,
      description: None,
      read_only: false,
      disabled: false,
      stepper: false,
      id: None,
      key: None,
    }
  }
}

/// A property of any type, as a category holds it.
#[derive(Clone)]
pub struct PropertyRow {
  name: Arc<str>,
  editor: BuildEditor,
  stepper: bool,
  modified: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
  reset: Option<Arc<dyn Fn() + Send + Sync>>,
  value_text: Option<Arc<dyn Fn() -> String + Send + Sync>>,
  description: Option<Arc<str>>,
  read_only: bool,
  disabled: bool,
  id: Arc<str>,
  key: Option<Arc<str>>,
}

impl<V: PartialEq + Clone + Send + Sync + 'static> From<Property<V>> for PropertyRow {
  fn from(property: Property<V>) -> Self {
    let (modified, reset) = match (property.default, property.get, property.set) {
      (Some(default), Some(get), Some(set)) => {
        let (known, back) = (default.clone(), default);
        (
          Some(Arc::new(move || get() != known) as Arc<dyn Fn() -> bool + Send + Sync>),
          Some(Arc::new(move || set(back.clone())) as Arc<dyn Fn() + Send + Sync>),
        )
      }
      _ => (None, None),
    };
    Self {
      id: property.id.unwrap_or_else(|| property.name.clone()),
      name: property.name,
      editor: property.editor,
      stepper: property.stepper,
      modified,
      reset,
      value_text: property.value_text,
      description: property.description,
      read_only: property.read_only,
      disabled: property.disabled,
      key: property.key,
    }
  }
}

/// A category of a property grid: a title over the properties it holds, open or closed.
#[derive(Clone)]
pub struct PropertyCategory {
  title: Arc<str>,
  properties: Vec<PropertyRow>,
  default_open: bool,
  disabled: bool,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl PropertyCategory {
  /// A category called `title`, open.
  pub fn new(title: &str) -> Self {
    Self {
      title: Arc::from(title),
      properties: Vec::new(),
      default_open: true,
      disabled: false,
      id: None,
      key: None,
    }
  }

  /// Adds a property after those it holds.
  pub fn property(mut self, property: impl Into<PropertyRow>) -> Self {
    self.properties.push(property.into());
    self
  }

  /// Adds properties after those it holds.
  pub fn properties(mut self, properties: impl IntoIterator<Item = impl Into<PropertyRow>>) -> Self {
    self.properties.extend(properties.into_iter().map(Into::into));
    self
  }

  /// Whether it is open as the grid is first shown: it is unless told. The mounted grid keeps it after.
  pub fn default_open(mut self, open: bool) -> Self {
    self.default_open = open;
    self
  }

  /// Whether nothing in it takes input.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// The id of its row's element under the grid's id.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  /// The key of its row's element, and what the mounted grid keeps whether it is open by: its title unless told.
  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  fn known_as(&self) -> Arc<str> {
    self.key.clone().unwrap_or_else(|| self.title.clone())
  }
}

type ChangeHandler = Arc<dyn Fn(&str) + Send + Sync>;

/// What a mounted grid keeps: which categories are open, and the width of the names.
#[derive(Clone)]
struct GridHost {
  open: Signal<HashMap<Arc<str>, bool>>,
  name_width: Signal<Option<f32>>,
  edited: Edited,
  /// What each color row keeps, by its property's id.
  colors: Arc<Mutex<HashMap<Arc<str>, ColorKeep>>>,
  /// The row the focus is to go back to at the next render: set by Escape in its editor, taken by the render.
  refocus: Arc<Mutex<Option<Arc<str>>>>,
  /// Moved on to wake the render that takes `refocus`.
  wake: Signal<u64>,
  /// The row the focus goes to in this render, and the ref it carries.
  focus: Option<(Arc<str>, ElementRef)>,
  /// The categories in motion, where the grid's rows are in a virtual list.
  motion: Arc<OpeningRows>,
}

impl GridHost {
  /// Opens or closes the category known as `known`, a person's doing: the open is kept, and its motion starts.
  fn set_open(&self, known: &Arc<str>, to: bool) {
    self.open.update(|open| {
      open.insert(known.clone(), to);
    });
    self.motion.started(known, to);
  }
}

/// A property grid: an inspector's names with the editors of their values, in categories that open and close. The
/// position and the size of what is selected on a canvas, its fill, its opacity.
///
/// ```
/// use lurq::{core::Signal, ui::{Property, PropertyCategory, PropertyGrid}};
///
/// let (x, y, rotation, visible) = (Signal::new(120.0), Signal::new(64.0), Signal::new(0.0), Signal::new(true));
/// let grid = PropertyGrid::new()
///   .title("Properties")
///   .category(
///     PropertyCategory::new("Transform")
///       .property(Property::xy("Position", x, y))
///       .property(Property::number_with("Rotation", rotation, "°").default(0.0)),
///   )
///   .category(PropertyCategory::new("Appearance").property(Property::toggle("Visible", visible)))
///   .on_change(|id| println!("{id} changed"));
/// ```
///
/// Each value is the app's signal, edited at once by its editor: an input, a number input, a checkbox, a select,
/// a color field, a slider. A property with a [`default`](Property::default) whose value is another has its name
/// marked and a control that puts the default back. To the keys it is a tree: Up and Down go between the rows,
/// Left and Right close and open a category (and go from a property to its category), Tab or Enter go into the
/// editor, Escape goes back to the row. The divider between the names and the editors is dragged, or moved with
/// Left and Right while it has the focus. Opening and closing and the divider are what the grid mounted with
/// [`PropertyGrid::mount`] keeps: converted into an element on its own, its categories are as they are first
/// shown and the divider moves nothing.
#[derive(Clone)]
pub struct PropertyGrid {
  title: Option<Arc<str>>,
  trailing: Option<Arc<dyn Fn() -> Element + Send + Sync>>,
  categories: Vec<PropertyCategory>,
  name_width: f32,
  height: Option<super::part_style::Length>,
  label: Option<Arc<str>>,
  read_only: bool,
  disabled: bool,
  full_width: bool,
  states: Vec<Name>,
  rules: Recipe,
  forced: Option<Interaction>,
  on_change: Vec<ChangeHandler>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
  host: Option<GridHost>,
}

impl Default for PropertyGrid {
  fn default() -> Self {
    Self::new()
  }
}

impl PropertyGrid {
  /// A grid with no category yet.
  pub fn new() -> Self {
    Self {
      title: None,
      trailing: None,
      categories: Vec::new(),
      name_width: PROPERTY_NAME_WIDTH,
      height: None,
      label: None,
      read_only: false,
      disabled: false,
      full_width: false,
      states: Vec::new(),
      rules: Recipe::new(),
      forced: None,
      on_change: Vec::new(),
      id: None,
      key: None,
      host: None,
    }
  }

  /// The header over the categories, with this title: "Properties", which the design writes in capitals.
  pub fn title(mut self, title: &str) -> Self {
    self.title = Some(Arc::from(title));
    self
  }

  /// What stands at the end of the header: its buttons.
  pub fn trailing(mut self, trailing: impl Fn() -> Element + Send + Sync + 'static) -> Self {
    self.trailing = Some(Arc::new(trailing));
    self
  }

  /// Adds a category after those it has.
  pub fn category(mut self, category: PropertyCategory) -> Self {
    self.categories.push(category);
    self
  }

  /// Adds categories after those it has.
  pub fn categories(mut self, categories: impl IntoIterator<Item = PropertyCategory>) -> Self {
    self.categories.extend(categories);
    self
  }

  /// The width of the names, in place of the board's 84: where the mounted grid's divider starts.
  pub fn name_width(mut self, width: f32) -> Self {
    self.name_width = width;
    self
  }

  /// The height of the grid, its rows scrolling in it: a long inspector's. Mounted, it builds only the rows in view
  /// and those near them, the keys reaching every row.
  pub fn height(mut self, height: impl Into<super::part_style::Length>) -> Self {
    self.height = Some(height.into());
    self
  }

  /// What the grid is called to what reads the tree: its title, else "Properties".
  pub fn label(mut self, label: &str) -> Self {
    self.label = Some(Arc::from(label));
    self
  }

  /// Whether every value is read and not changed.
  pub fn read_only(mut self, read_only: bool) -> Self {
    self.read_only = read_only;
    self
  }

  /// Whether nothing in it takes input, dimmed.
  pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
  }

  /// Whether it is as wide as what holds it, in place of the board's 300: an inspector's column.
  pub fn full_width(mut self, full_width: bool) -> Self {
    self.full_width = full_width;
    self
  }

  /// Called with the id of a property a person changed (its `id`, else its name), after its value is set: at once
  /// for a box, a choice, a color, a slider and a number, as each edit is made for a text, and as its reset is
  /// pressed. Not when the app sets a value. A second handler is called after the first.
  pub fn on_change(mut self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.on_change.push(Arc::new(handler));
    self
  }

  /// Puts the grid in a state the theme's recipe names, or takes it out.
  pub fn state(mut self, state: impl Into<Name>, on: bool) -> Self {
    let state = state.into();
    self.states.retain(|held| *held != state);
    if on {
      self.states.push(state);
    }
    self
  }

  /// Styles one part of this grid over what the theme gives it.
  pub fn style(mut self, part: impl Into<Name>, style: impl FnOnce(super::PartStyle) -> super::PartStyle) -> Self {
    self.rules = self.rules.base(part, style);
    self
  }

  /// Lays rules over the theme's recipe for this grid.
  pub fn rules(mut self, rules: &Recipe) -> Self {
    self.rules = self.rules.extend(rules);
    self
  }

  /// Draws the row the focus would enter at as it is under the pointer or focused from the keyboard, with nothing
  /// pointing at it: for a gallery or a test.
  pub fn force(mut self, interaction: Option<Interaction>) -> Self {
    self.forced = interaction;
    self
  }

  /// The id of the grid. A category's row is `<id>-<category's id or title>`, a property's
  /// `<id>-<property's id or name>`, its editor's holder `<id>-<property>-editor`, its reset `<id>-<property>-reset`,
  /// the divider `<id>-divider`.
  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// The grid mounted where it is declared: it keeps which categories are open and where the divider was dragged.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    let key = self.key.clone();
    let props = GridProps { grid: Arc::new(self) };
    match key {
      Some(key) => ctx.mount_keyed::<GridMount>(&key, props),
      None => ctx.mount::<GridMount>(props),
    }
  }

  fn is_open(&self, category: &PropertyCategory, opened: &HashMap<Arc<str>, bool>) -> bool {
    opened
      .get(&category.known_as())
      .copied()
      .unwrap_or(category.default_open)
  }

  /// Builds the grid in `theme`. Converting a grid into an element does this in the theme of what is being
  /// rendered.
  pub fn build(self, theme: &Theme) -> Element {
    let view = self.view(theme);
    match theme.renderer::<PropertyGrid>() {
      Some(renderer) => renderer(view),
      None => view.render(),
    }
  }

  /// The grid as a renderer sees it, in `theme`.
  fn view(self, theme: &Theme) -> PropertyGridView {
    let case = Case {
      variant: None,
      size: None,
      density: None,
      breakpoint: super::current_breakpoint(),
      states: self.states.clone(),
    }
    .state(PropertyGridState::Disabled, self.disabled)
    .state(PropertyGridState::FullWidth, self.full_width);
    let recipe = theme.resolve::<PropertyGrid>(&case, Some(&self.rules));
    let resolve = |states: &[(PropertyGridState, bool)]| {
      let case = states
        .iter()
        .fold(case.clone(), |case, (state, on)| case.state(*state, *on));
      theme.resolve::<PropertyGrid>(&case, Some(&self.rules))
    };
    let name_width = self
      .host
      .as_ref()
      .and_then(|host| host.name_width.get())
      .unwrap_or(self.name_width);
    // The render that shows the categories follows what opens and closes them.
    let opened = self.host.as_ref().map(|host| host.open.get()).unwrap_or_default();
    let mut categories = Vec::new();
    for category in &self.categories {
      let open = self.is_open(category, &opened);
      let unavailable = category.disabled;
      let rows = category
        .properties
        .iter()
        .map(|property| {
          let modified = property.modified.as_ref().is_some_and(|modified| modified());
          let recipe = resolve(&[
            (PropertyGridState::Modified, modified),
            (PropertyGridState::ReadOnly, property.read_only || self.read_only),
            (PropertyGridState::Unavailable, property.disabled || unavailable),
          ]);
          PropertyView {
            property: property.clone(),
            modified,
            recipe,
          }
        })
        .collect();
      categories.push(CategoryView {
        recipe: resolve(&[
          (PropertyGridState::Open, open),
          (PropertyGridState::Unavailable, unavailable),
        ]),
        category: category.clone(),
        open,
        rows,
      });
    }
    let mut recipe = recipe;
    if let Some(height) = self.height {
      recipe = theme.resolve::<PropertyGrid>(
        &case,
        Some(&self.rules.clone().base(PropertyGridPart::Root, |s| s.height(height))),
      );
    }
    let view = PropertyGridView {
      label: self
        .label
        .clone()
        .or_else(|| self.title.clone())
        .unwrap_or_else(|| super::current_locale().fields.properties.clone()),
      title: self.title.clone(),
      trailing: self.trailing.clone(),
      name_width,
      read_only: self.read_only,
      disabled: self.disabled,
      recipe,
      categories,
      forced: self.forced,
      on_change: self.on_change.clone(),
      host: self.host.clone(),
      id: self.id.clone(),
      key: self.key.clone(),
    };
    view
  }
}

impl From<PropertyGrid> for Element {
  fn from(grid: PropertyGrid) -> Self {
    grid.build(&current_theme())
  }
}

/// A category as a renderer sees it.
pub struct CategoryView {
  pub category: PropertyCategory,
  pub open: bool,
  /// The style of its parts, in its state.
  pub recipe: ResolvedRecipe,
  pub rows: Vec<PropertyView>,
}

/// A property as a renderer sees it.
pub struct PropertyView {
  pub property: PropertyRow,
  /// Whether its value is not its default.
  pub modified: bool,
  /// The style of its parts, in its state.
  pub recipe: ResolvedRecipe,
}

impl PropertyRow {
  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn id(&self) -> &str {
    &self.id
  }

  pub fn description(&self) -> Option<&str> {
    self.description.as_deref()
  }

  /// Its value as tooling reads it: "X 120, Y 64".
  pub fn value_text(&self) -> Option<String> {
    self.value_text.as_ref().map(|text| text())
  }
}

/// A property grid as a renderer sees it: what it was given, the style of its parts, and its rows ready to be
/// dressed.
pub struct PropertyGridView {
  /// What it is called to what reads the tree.
  pub label: Arc<str>,
  pub title: Option<Arc<str>>,
  /// The width of the names.
  pub name_width: f32,
  pub read_only: bool,
  pub disabled: bool,
  /// The style of each part of the grid as a whole.
  pub recipe: ResolvedRecipe,
  pub categories: Vec<CategoryView>,
  trailing: Option<Arc<dyn Fn() -> Element + Send + Sync>>,
  forced: Option<Interaction>,
  on_change: Vec<ChangeHandler>,
  host: Option<GridHost>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl PropertyGridView {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  pub fn theme(&self) -> &Theme {
    self.recipe.theme()
  }

  fn child_id(&self, suffix: &str) -> Option<Arc<str>> {
    self.id.as_ref().map(|id| Arc::from(format!("{id}-{suffix}")))
  }

  fn tell(&self) -> impl Fn(&str) + Send + Sync + Clone + 'static {
    let handlers = self.on_change.clone();
    move |id: &str| {
      for handler in &handlers {
        handler(id);
      }
    }
  }

  /// The header: the title, and what the app puts at its end.
  pub fn header(&self) -> Option<Row> {
    let title = self.title.as_ref()?;
    let part = self.part(PropertyGridPart::Title);
    // The design writes the title in capitals.
    let shown = title.to_uppercase();
    let mut header = Row::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .part(&self.part(PropertyGridPart::Header))
      .child(
        Row::new()
          .flex(1.0)
          .min_width(Dimension::Px(0.0))
          .part(&part)
          .child(part.text(&shown).nowrap().text_overflow(TextOverflow::Elipsis)),
      );
    if let Some(trailing) = &self.trailing {
      header = header.child(trailing());
    }
    Some(header)
  }

  /// A category's row: its chevron and its title. A press, or Left and Right, close and open it.
  pub fn category_row(&self, index: usize, entry: bool) -> Option<components::Button> {
    let view = self.categories.get(index)?;
    let part = |name: PropertyGridPart| {
      let part = view.recipe.part(name);
      if entry { part.forced(self.forced) } else { part }
    };
    let unavailable = view.category.disabled || self.disabled;
    let title = part(PropertyGridPart::CategoryTitle);
    // One chevron, down, turned to the end while the category is closed: it turns as the category opens and
    // closes, over the opening's duration, at once where motion is reduced.
    let theme = self.theme();
    let chevron = Row::new()
      .transform(crate::node::transform::Transform2D::rotate_deg(if view.open {
        0.0
      } else {
        -90.0
      }))
      .transition(
        crate::animation::Transition::transform()
          .duration(theme.duration_value(crate::app::theme::MotionDuration::Base))
          .easing(theme.easing_value(crate::app::theme::MotionEasing::Standard)),
      )
      .child(glyph(
        &part(PropertyGridPart::Chevron),
        Name::from(icons::CHEVRON_DOWN),
        false,
      ));
    let mut row = pressable(&part(PropertyGridPart::Category), unavailable)
      .justify(Justify::Start)
      .align_items(Alignment::Center)
      .describe("a11y_role", "row")
      .describe("a11y_name", view.category.title.clone())
      .describe("a11y_level", "1")
      .describe("a11y_expanded", if view.open { "true" } else { "false" })
      .focus_level(1)
      .focus_entry(entry)
      .focus_label(view.category.title.clone())
      .key(format!("category-{}", view.category.known_as()))
      .child(chevron)
      .child(
        Row::new().flex(1.0).min_width(Dimension::Px(0.0)).part(&title).child(
          title
            .text(&view.category.title)
            .nowrap()
            .text_overflow(TextOverflow::Elipsis),
        ),
      );
    if let Some(host) = &self.host {
      let (host, known) = (host.clone(), view.category.known_as());
      let set = move |to: bool| host.set_open(&known, to);
      let (pressed, now) = (set.clone(), view.open);
      row = row
        .focus_expanded(view.open, move |to| set(to))
        .on_click(move |_| pressed(!now));
    }
    if let Some(id) = self.child_id(view.category.id.as_deref().unwrap_or(&view.category.title)) {
      row = row.id(id);
    }
    Some(row)
  }

  /// The row of the property `row` of the category `category`: its name, marked where its value is not its default,
  /// its editor, and its reset.
  pub fn property_row(&self, category: usize, row: usize, entry: bool) -> Option<components::Button> {
    let category_view = self.categories.get(category)?;
    let view = category_view.rows.get(row)?;
    let property = &view.property;
    let part = |name: PropertyGridPart| {
      let part = view.recipe.part(name);
      if entry { part.forced(self.forced) } else { part }
    };
    let read_only = property.read_only || self.read_only;
    let disabled = property.disabled || category_view.category.disabled || self.disabled;
    let tell = self.tell();
    let id = property.id.clone();
    let notify: Notify = {
      let (tell, id) = (tell.clone(), id.clone());
      Arc::new(move || tell(&id))
    };
    let cx = EditorContext {
      theme: self.theme().clone(),
      editor: view.recipe.part(PropertyGridPart::Editor),
      axis: [
        view.recipe.part(PropertyGridPart::Axis),
        view.recipe.part(PropertyGridPart::AxisX),
        view.recipe.part(PropertyGridPart::AxisY),
      ],
      label: property.name.clone(),
      read_only,
      disabled,
      stepper: property.stepper,
      notify,
      edited: self
        .host
        .as_ref()
        .map(|host| (property.id.clone(), host.edited.clone())),
      color: self.host.as_ref().map(|host| {
        host
          .colors
          .lock()
          .unwrap()
          .entry(property.id.clone())
          .or_insert_with(|| ColorKeep {
            open: Signal::new(false),
            draft: Signal::new(String::new()),
          })
          .clone()
      }),
      stepper_part: view.recipe.part(PropertyGridPart::Stepper),
      swatch_part: view.recipe.part(PropertyGridPart::Swatch),
    };
    let row_key: Arc<str> = Arc::from(format!("property-{}", property.key.as_deref().unwrap_or(&property.id)));
    let editor_ref = ElementRef::new();
    // A column that stretches its editor: an editor whose own outer row has no width (a color field's) is given
    // the room.
    let mut editor = Column::new()
      .width(Dimension::Pct(100.0))
      .justify(Justify::Center)
      .align_items(Alignment::Stretch)
      .ref_element(editor_ref.clone());
    if let Some(host) = &self.host {
      // Escape in an editor gives the focus back to its row, where nothing in the editor took the key.
      let (back, refocus, wake) = (row_key.clone(), host.refocus.clone(), host.wake.clone());
      editor = editor.on_key_down(move |event: KeyboardEvent| {
        if event.key == "Escape" && event.focus_within && !event.default_prevented() {
          event.prevent_default();
          *refocus.lock().unwrap() = Some(back.clone());
          wake.update(|wake| *wake += 1);
        }
      });
    }
    let editor = editor.child((property.editor)(&cx));
    let editor = match self.child_id(&format!("{id}-editor")) {
      Some(element_id) => editor.id(element_id),
      None => editor,
    };
    let label_part = part(PropertyGridPart::Label);
    let mut label = Row::new()
      .width(self.name_width)
      .min_width(Dimension::Px(0.0))
      .align_items(Alignment::Center)
      .spacing(6.0)
      .child(
        Row::new()
          .flex_full(0.0, 1.0, None)
          .min_width(Dimension::Px(0.0))
          .part(&label_part)
          .child(
            label_part
              .text(&property.name)
              .nowrap()
              .text_overflow(TextOverflow::Elipsis),
          ),
      );
    if view.modified {
      label = label.child(Row::new().part(&part(PropertyGridPart::Modified)));
    }
    let mut row_button = pressable(&part(PropertyGridPart::Row), disabled)
      .justify(Justify::Start)
      .align_items(Alignment::Center)
      // Tab goes from the row into its editor; a press of the name, or Enter, gives the editor the focus.
      .stops_inside(true)
      .focuses(&editor_ref)
      .describe("a11y_role", "row")
      .describe("a11y_name", property.name.clone())
      .describe("a11y_level", "2")
      .focus_level(2)
      .focus_entry(entry)
      .focus_label(property.name.clone())
      .key(row_key.as_ref())
      .child(label)
      .child(
        Row::new()
          .flex(1.0)
          .min_width(Dimension::Px(0.0))
          .part(&part(PropertyGridPart::Value))
          // The editors are stops of Tab, not rows the arrows go between.
          .focus_group(FocusGroup::none())
          .child(editor),
      );
    if let Some(value) = property.value_text() {
      row_button = row_button.describe("a11y_value", value);
    }
    if let Some(description) = &property.description {
      row_button = row_button.describe("a11y_description", description.clone());
    }
    if read_only {
      row_button = row_button.describe("a11y_readonly", "true");
    }
    if let (true, Some(reset)) = (view.modified, &property.reset) {
      let reset_part = part(PropertyGridPart::Reset);
      let (reset, tell, id) = (reset.clone(), tell.clone(), id.clone());
      let able = !read_only && !disabled;
      let mut control = pressable(&reset_part, !able)
        .describe("a11y_role", "button")
        .describe(
          "a11y_name",
          crate::core::fill(&super::current_locale().fields.reset, &[("name", &property.name)]),
        )
        .child(glyph(&reset_part, Name::from(icons::ROTATE_LEFT), false));
      if able {
        control = control.on_click(move |_| {
          reset();
          tell(&id);
        });
      }
      if let Some(element_id) = self.child_id(&format!("{}-reset", property.id)) {
        control = control.id(element_id);
      }
      // A stop of Tab after the editor, and not a row the arrows go to.
      row_button = row_button.child(Row::new().focus_group(FocusGroup::none()).child(control));
    }
    if let Some(element_id) = self.child_id(&property.id) {
      row_button = row_button.id(element_id);
    }
    if let Some((to, element)) = self.host.as_ref().and_then(|host| host.focus.as_ref())
      && *to == row_key
    {
      row_button = row_button.ref_element(element.clone());
    }
    Some(row_button)
  }

  /// The divider between the names and the editors, over the rows: dragged, or moved with Left and Right while
  /// it has the focus. It moves nothing where the grid is not mounted.
  pub fn divider(&self) -> Option<Element> {
    let host = self.host.clone()?;
    if self.disabled {
      return None;
    }
    let part = self.part(PropertyGridPart::Divider);
    let row_part = self.part(PropertyGridPart::Row).rest();
    let start = row_part
      .padding_start
      .or(row_part.padding_x.clone())
      .map(|padding: crate::node::SpacingValue| padding.resolve(&self.theme().spacing(), 0.0))
      .unwrap_or(0.0);
    let gap = row_part
      .gap
      .map(|gap| gap.resolve(&self.theme().spacing(), 0.0))
      .unwrap_or(0.0);
    let side = part
      .rest()
      .width
      .and_then(|width| match width.resolve(self.theme()) {
        Some(Dimension::Px(px)) => Some(px),
        _ => None,
      })
      .unwrap_or(8.0);
    let width = self.name_width;
    let dragged = Arc::new(Mutex::new(width));
    let (from, moved, keyed) = (dragged.clone(), host.name_width.clone(), host.name_width.clone());
    let clamp = |width: f32| width.max(LEAST);
    let mut divider = components::Button::empty()
      .tab_index(0)
      .part(&part)
      // As high as the rows, whatever gives the grid its height: a share of a height that is the rows' own is
      // nothing, and a divider of no height is under no pointer.
      .absolute(
        start + width + gap / 2.0 - side / 2.0,
        0.0,
        Dimension::Px(side),
        Dimension::Pct(100.0),
      )
      .describe("a11y_role", "separator")
      .describe("a11y_name", super::current_locale().fields.names_width.clone())
      .describe("a11y_orientation", "vertical")
      .describe("a11y_valuenow", format!("{}", width.round()))
      .start_drag_buttons(MouseButtonMask::LEFT)
      .on_drag_start(move |_: DragEvent| *from.lock().unwrap() = width)
      .on_drag_move(move |event: DragEvent| {
        let start = *dragged.lock().unwrap();
        moved.set(Some(clamp(start + event.total_delta_x)));
      })
      .on_key_down(move |event: KeyboardEvent| {
        if !event.focus_within || event.ctrl || event.alt || event.meta {
          return;
        }
        let step = if event.shift { 40.0 } else { 8.0 };
        let by = match event.key.as_str() {
          "ArrowLeft" => -step,
          "ArrowRight" => step,
          _ => return,
        };
        event.prevent_default();
        keyed.set(Some(clamp(width + by)));
      });
    if let Some(id) = self.child_id("divider") {
      divider = divider.id(id);
    }
    Some(divider.into())
  }

  /// The row `shown` stands for, in a virtual list: it says its place to the focus group's rows.
  fn shown_row(&self, shown: Shown, place: usize) -> Option<Element> {
    let row = match shown {
      Shown::Category(index) => self.category_row(index, place == 0)?,
      Shown::Property(category, row) => self.property_row(category, row, false)?,
    };
    Some(row.focus_row(place).into())
  }

  /// The root: its header over `body`, the rows with the divider over them.
  fn root(&self, rows: Element) -> Element {
    let body: Element = match self.divider() {
      Some(divider) => Stack::new()
        .width(Dimension::Pct(100.0))
        .flex(1.0)
        .child(rows)
        .child(divider)
        .into(),
      None => rows,
    };
    let mut root = Column::new()
      .align_items(Alignment::Stretch)
      .part(&self.part(PropertyGridPart::Root))
      .describe("a11y_role", "treegrid")
      .describe("a11y_name", self.label.clone());
    if let Some(header) = self.header() {
      root = root.child(header);
    }
    root = root.child(body);
    if let Some(id) = &self.id {
      root = root.id(id.clone());
    }
    if let Some(key) = &self.key {
      root = root.key(key.clone());
    }
    root.into()
  }

  /// How the default renderer builds a grid: the header, then each category's row and, while it is open, the rows
  /// of its properties, in a nested group of the focus; the divider over them.
  pub fn render(self) -> Element {
    let mut rows = Column::new()
      .width(Dimension::Pct(100.0))
      .align_items(Alignment::Stretch)
      .focus_group(FocusGroup::vertical().nested(true).one_stop(true).typeahead(true));
    let mut first = true;
    for index in 0..self.categories.len() {
      if let Some(row) = self.category_row(index, first) {
        rows = rows.child(row);
        first = false;
      }
      rows = rows.child(self.panel(index));
    }
    self.root(rows.into())
  }

  /// What opens and closes under the category `index`'s row, as an accordion's item does: its height is the
  /// recipe's, of none while the category is closed and its rows' while it is open, with a transition between the
  /// two. Its rows stay while it closes over them, only painted, and are gone with its body's exit.
  pub fn panel(&self, index: usize) -> Column {
    let view = &self.categories[index];
    let mut panel = Column::new()
      .align_items(Alignment::Stretch)
      .clip()
      .presence()
      // Keyed as its category's row is: the same panel from render to render, whose height runs.
      .key(format!("panel-{}", view.category.known_as()))
      .part(&view.recipe.part(PropertyGridPart::Panel));
    if let Some(id) = self.child_id(&format!(
      "{}-panel",
      view.category.id.as_deref().unwrap_or(&view.category.title)
    )) {
      panel = panel.id(id);
    }
    if view.open {
      let mut body = Column::new()
        .key("rows")
        .align_items(Alignment::Stretch)
        .part(&view.recipe.part(PropertyGridPart::Body));
      for row in 0..view.rows.len() {
        if let Some(row) = self.property_row(index, row, false) {
          body = body.child(row);
        }
      }
      panel = panel.child(body);
    }
    panel
  }
}

/// A row a grid shows: a category's, or a property's of an open category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shown {
  Category(usize),
  Property(usize, usize),
}

/// The focus ring inside a row, over its line: the tree view's.
fn ring_inside() -> crate::node::outline::Outline {
  crate::node::outline::Outline::new(crate::app::theme::BorderSize::Md, PaletteColor::Ring).offset(-2.0)
}

impl crate::ui::Styled for PropertyGrid {
  type Component = Self;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl Themed for PropertyGrid {
  const NAME: &'static str = "property_grid";
  type Renderer = PropertyGridRenderer;

  fn parts() -> crate::ui::Declared {
    PropertyGridPart::declared()
  }

  fn states() -> crate::ui::Declared {
    PropertyGridState::declared()
  }

  /// The design's property grid (`pencil/lurq-design-system.pen`, Property Grid): 300 wide on `Background`, an edge
  /// of 1 in `Border` and the large radius. Its header is 30 high on `Muted` with a line under it, its title in 11
  /// semibold capitals spaced 0.6 in `MutedForeground`, 10 in. A category's row is 28 high on `Muted` with a line
  /// under it, padded 8 across, its chevron of 14 in `MutedForeground` 6 before its title in 12 semibold. A
  /// property's row is 32 high with a line under it, padded 12 at its start and 8 at its end, its name 84 wide in
  /// 12 in `MutedForeground` and its editors 8 after it, 6 apart. An editor is 24 high on `Background` with an edge
  /// of 1 in `Input`, the small radius, padded 6 across, its text in 12 mono; the letter of a value of two is 11
  /// semibold mono, X in `Destructive`, Y in `Success`, others in `MutedForeground`. A property whose value is not
  /// its default has its name in 12 semibold in `Foreground`, a dot of 5 in `Info` 6 after it, and the reset of 13
  /// in `MutedForeground` at the row's end. The board draws no row under the pointer or focused: a row under the
  /// pointer is on `Muted`, a focused one has the focus ring inside it; a read-only or disabled one is dimmed.
  fn recipe() -> Recipe {
    use PaletteColor::{
      Background, Destructive, Foreground, Info, Input as InputColor, Muted, MutedForeground, Success,
    };
    use PropertyGridPart::*;
    let under = |s: super::PartStyle| s.underline(Edge::inside(1.0, PaletteColor::Border));
    let mono = |s: super::PartStyle, size: f32| s.typography(TypographyStyle::Mono).font_size(size);
    let row_case = super::recipe::Selector::new();
    let recipe = Recipe::new()
      .base(Root, |s| {
        s.width(300.0)
          .max_width(super::Length::Fill)
          .background(Background)
          .border(Edge::inside(1.0, PaletteColor::Border))
          .radius(RadiusSize::Lg)
      })
      .state(PropertyGridState::FullWidth, Root, |s| {
        s.width(super::part_style::Length::Fill)
      })
      .base(Header, |s| {
        under(s.height(30.0).padding_start(10.0).padding_end(4.0).background(Muted))
      })
      .base(Title, |s| {
        s.typography(TypographyStyle::Caption)
          .font_size(11.0)
          .weight(FontWeight::SemiBold)
          .letter_spacing(0.6)
          .foreground(MutedForeground)
      })
      .base(Category, |s| {
        fading(under(
          s.height(28.0)
            .padding_x(8.0)
            .gap(6.0)
            .background(Muted)
            .cursor(CursorIcon::Pointer),
        ))
      })
      .row_density(row_case.clone(), Category, 28.0)
      .base(Chevron, |s| s.square(14.0).foreground(MutedForeground))
      .base(CategoryTitle, |s| {
        s.typography(TypographyStyle::Caption)
          .font_size(12.0)
          .weight(FontWeight::SemiBold)
          .foreground(Foreground)
      })
      .base(Row, |s| {
        fading(under(s.height(32.0).padding_start(12.0).padding_end(8.0).gap(8.0)))
      })
      .row_density(row_case, Row, 32.0)
      .base(Label, |s| {
        s.typography(TypographyStyle::Caption)
          .font_size(12.0)
          .foreground(MutedForeground)
      })
      .base(Modified, |s| s.square(5.0).radius(RadiusSize::Full).background(Info))
      .base(Value, |s| s.gap(6.0))
      .base(Editor, |s| {
        mono(s, 12.0)
          .height(24.0)
          .padding_x(6.0)
          .radius(RadiusSize::Sm)
          .background(Background)
          .border(Edge::inside(1.0, InputColor))
      })
      .base(Axis, |s| {
        mono(s, 11.0).weight(FontWeight::SemiBold).foreground(MutedForeground)
      })
      .base(AxisX, |s| {
        mono(s, 11.0).weight(FontWeight::SemiBold).foreground(Destructive)
      })
      .base(AxisY, |s| {
        mono(s, 11.0).weight(FontWeight::SemiBold).foreground(Success)
      })
      .base(Reset, |s| {
        fading(
          s.square(13.0)
            .radius(RadiusSize::Sm)
            .foreground(MutedForeground)
            .cursor(CursorIcon::Pointer),
        )
      })
      .base(Divider, |s| s.width(8.0).cursor(CursorIcon::ColResize))
      .base(Stepper, |s| {
        s.square(12.0).foreground(MutedForeground).cursor(CursorIcon::Pointer)
      })
      .hovered(Stepper, |s| s.foreground(Foreground))
      .base(Swatch, |s| {
        s.radius(RadiusSize::Sm).border(Edge::inside(1.0, PaletteColor::Border))
      })
      .hovered(Category, |s| s.foreground(Foreground))
      .hovered(Row, |s| s.background(Muted))
      .focus_visible(Category, |s| s.outline(ring_inside()))
      .focus_visible(Row, |s| s.outline(ring_inside()))
      .hovered(Reset, |s| s.foreground(Foreground))
      .focus_visible(Reset, |s| s.focus_ring())
      .focus_visible(Divider, |s| s.background(PaletteColor::Ring))
      .state(PropertyGridState::Modified, Label, |s| {
        s.weight(FontWeight::SemiBold).foreground(Foreground)
      })
      .state(PropertyGridState::Unavailable, Row, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .state(PropertyGridState::Unavailable, Category, |s| {
        s.opacity(OpacityLevel::Disabled).cursor(CursorIcon::Default)
      })
      .state(PropertyGridState::Disabled, Root, |s| s.opacity(OpacityLevel::Disabled));
    // A category opens and closes as an accordion's item does: its panel's height over the theme's base duration,
    // its rows fading in and out.
    super::collapsible::opening(recipe, PropertyGridState::Open.into(), Panel.into(), Body.into())
  }
}

/// A grid to mount.
#[derive(Clone)]
struct GridProps {
  grid: Arc<PropertyGrid>,
}

impl PartialEq for GridProps {
  fn eq(&self, _: &Self) -> bool {
    false
  }
}

impl crate::app::component::DevtoolsInspectable for GridProps {}

/// What keeps a mounted grid's open categories and its names' width.
struct GridMount {
  host: GridHost,
  /// The rows a virtual list steps over, and what its row components draw.
  rows: Arc<GridRows>,
  shown: ShownRows,
  generation: std::sync::atomic::AtomicU64,
  /// The row the focus went back to last, and the ref it carries: kept on it in every render after, since the
  /// render that asks for the focus need not be the last of its pass.
  focused: Arc<Mutex<Option<(Arc<str>, ElementRef)>>>,
}

impl Component for GridMount {
  type Props = GridProps;

  fn create(ctx: &mut Ctx) -> Self {
    let wake: Signal<u64> = ctx.signal(0);
    // The categories' rows open and close as a tree's do: the motion is the one they share.
    let motion = OpeningRows::new(ctx, &wake);
    Self {
      host: GridHost {
        open: ctx.signal(HashMap::new()),
        name_width: ctx.signal(None),
        edited: Arc::default(),
        colors: Arc::default(),
        refocus: Arc::default(),
        wake,
        focus: None,
        motion,
      },
      focused: Arc::default(),
      rows: Arc::new(GridRows {
        data: Mutex::default(),
        reveal_key: ctx.signal(None),
        reveal_request: ctx.signal(0),
        scroll: ScrollState::new(),
      }),
      shown: Arc::default(),
      generation: std::sync::atomic::AtomicU64::new(0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let mut grid = (*ctx.props::<GridProps>().grid).clone();
    let mut host = self.host.clone();
    // Read so that a wake renders the grid.
    let _ = self.host.wake.get();
    if let Some(row) = self.host.refocus.lock().unwrap().take() {
      let element = ElementRef::new();
      ctx.focus(&element);
      *self.focused.lock().unwrap() = Some((row, element));
    }
    host.focus = self.focused.lock().unwrap().clone();
    grid.host = Some(host);
    let theme = current_theme();
    // A grid with no height has nothing to scroll: every row is built. One a theme draws is the theme's.
    if grid.height.is_none() || theme.renderer::<PropertyGrid>().is_some() {
      let element: Element = grid.into();
      return element;
    }
    let view = grid.view(&theme);
    let motion = self.host.motion.clone();
    let moving = motion.read(&theme);
    // The rows shown, and the rows of a closing category, kept while they close.
    let mut shown: Vec<(Shown, bool)> = Vec::new();
    for (index, category) in view.categories.iter().enumerate() {
      shown.push((Shown::Category(index), false));
      let known = category.category.known_as();
      let kept = !category.open && moving.keeps(&known);
      if category.open || kept {
        shown.extend((0..category.rows.len()).map(|row| (Shown::Property(index, row), kept)));
      }
    }
    let key = |row: &Shown| match *row {
      Shown::Category(index) => format!("category-{}", view.categories[index].category.known_as()),
      Shown::Property(category, row) => {
        let property = &view.categories[category].rows[row].property;
        format!("property-{}", property.key.as_deref().unwrap_or(&property.id))
      }
    };
    let keys: Vec<String> = shown.iter().map(|(row, _)| key(row)).collect();
    let height = |row: &Shown| {
      let part = match row {
        Shown::Category(_) => PropertyGridPart::Category,
        Shown::Property(..) => PropertyGridPart::Row,
      };
      match view.part(part).rest().height.and_then(|height| height.resolve(&theme)) {
        Some(Dimension::Px(px)) => px,
        _ => 32.0,
      }
    };
    let heights: Vec<f32> = shown.iter().map(|(row, _)| height(row)).collect();
    // Each property row's height and how it runs to it, as its category opens or closes.
    let motions: Vec<Option<RowMotion>> = shown
      .iter()
      .zip(&heights)
      .map(|((row, kept), height)| {
        let Shown::Property(category, place) = *row else {
          return None;
        };
        let known = view.categories[category].category.known_as();
        Some(moving.row(&known, place, view.categories[category].rows.len(), *height, *kept))
      })
      .collect();
    let open = self.host.clone();
    *self.rows.data.lock().unwrap() = RowsData {
      keys: keys.clone(),
      labels: shown
        .iter()
        .map(|(row, _)| match *row {
          Shown::Category(index) => view.categories[index].category.title.clone(),
          Shown::Property(category, row) => view.categories[category].rows[row].property.name.clone(),
        })
        .collect(),
      levels: shown
        .iter()
        .map(|(row, _)| if matches!(row, Shown::Category(_)) { 1 } else { 2 })
        .collect(),
      expanded: shown
        .iter()
        .map(|(row, _)| match *row {
          Shown::Category(index) => Some(view.categories[index].open),
          Shown::Property(..) => None,
        })
        .collect(),
      // The rows of a closing category are not reached: they are going.
      enabled: shown
        .iter()
        .map(|(row, kept)| match *row {
          Shown::Category(index) => !view.categories[index].category.disabled && !view.disabled,
          Shown::Property(category, row) => {
            !kept
              && !view.categories[category].rows[row].property.disabled
              && !view.categories[category].category.disabled
              && !view.disabled
          }
        })
        .collect(),
      toggles: shown
        .iter()
        .map(|(row, _)| match *row {
          Shown::Category(index) => {
            let (host, known) = (open.clone(), view.categories[index].category.known_as());
            Some(Arc::new(move |to: bool| host.set_open(&known, to)) as Arc<dyn Fn(bool) + Send + Sync>)
          }
          Shown::Property(..) => None,
        })
        .collect(),
      step: heights.iter().copied().fold(32.0, f32::min),
    };
    motion.rendered(&moving);
    let generation = self.generation.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
    let view = Arc::new(view);
    let places: Vec<Shown> = shown.iter().map(|(row, _)| *row).collect();
    *self.shown.lock().unwrap() = Some((view.clone(), places));
    let items: Arc<Vec<GridItem>> = Arc::new(
      keys
        .into_iter()
        .zip(heights)
        .zip(motions)
        .enumerate()
        .map(|(place, ((key, height), motion))| GridItem {
          place,
          key,
          height,
          motion,
          generation,
        })
        .collect(),
    );
    let shared = self.shown.clone();
    let list = VirtualizedList::new_shared(ctx, items)
      .with_scroll_state(self.rows.scroll.clone())
      .reveal_key(self.rows.reveal_key.get())
      .reveal_request(self.rows.reveal_request.get())
      .estimate_height(|item: &GridItem| item.motion.as_ref().map_or(item.height, |motion| motion.estimate))
      // The bar has a lane of its own beside the rows: no editor, reset or mark is under it, at rest or shown.
      .scrollbar(crate::layout::scrollbar::ScrollBarStyle {
        placement: crate::layout::scrollbar::ScrollBarPlacement::Reserved,
        ..crate::layout::scrollbar::ScrollBarStyle::default()
      })
      .width(Dimension::Pct(100.0))
      .flex(1.0)
      .mount_keyed::<GridRowHost, _, _, _>(
        |item: &GridItem| item.key.clone(),
        move |item: &GridItem| GridRowProps {
          place: item.place,
          generation: item.generation,
          motion: item.motion.clone(),
          shown: shared.clone(),
        },
      );
    let rows: Arc<dyn FocusRows> = self.rows.clone();
    let rows = Column::new()
      .width(Dimension::Pct(100.0))
      .flex(1.0)
      .align_items(Alignment::Stretch)
      .focus_group(
        FocusGroup::vertical()
          .nested(true)
          .one_stop(true)
          .typeahead(true)
          .wrap(false),
      )
      .focus_rows(rows)
      .child(list);
    view.root(rows.into())
  }
}

/// What a grid in a virtual list built last, which its row components draw.
type ShownRows = Arc<Mutex<Option<(Arc<PropertyGridView>, Vec<Shown>)>>>;

/// The rows of a grid in a virtual list as the engine's focus group steps over them.
struct GridRows {
  data: Mutex<RowsData>,
  reveal_key: Signal<Option<String>>,
  reveal_request: Signal<u64>,
  scroll: ScrollState,
}

#[derive(Default)]
struct RowsData {
  keys: Vec<String>,
  labels: Vec<Arc<str>>,
  levels: Vec<u32>,
  expanded: Vec<Option<bool>>,
  enabled: Vec<bool>,
  toggles: Vec<Option<Arc<dyn Fn(bool) + Send + Sync>>>,
  /// The height of the lowest row: what a page of rows is counted in.
  step: f32,
}

impl FocusRows for GridRows {
  fn count(&self) -> usize {
    self.data.lock().unwrap().keys.len()
  }

  fn label(&self, row: usize) -> Option<String> {
    self.data.lock().unwrap().labels.get(row).map(|label| label.to_string())
  }

  fn enabled(&self, row: usize) -> bool {
    self.data.lock().unwrap().enabled.get(row).copied().unwrap_or(false)
  }

  fn level(&self, row: usize) -> u32 {
    self.data.lock().unwrap().levels.get(row).copied().unwrap_or(1)
  }

  fn expanded(&self, row: usize) -> Option<bool> {
    self.data.lock().unwrap().expanded.get(row).copied().flatten()
  }

  fn set_expanded(&self, row: usize, open: bool) {
    let toggle = self.data.lock().unwrap().toggles.get(row).cloned().flatten();
    if let Some(toggle) = toggle {
      toggle(open);
    }
  }

  fn page(&self) -> usize {
    let step = self.data.lock().unwrap().step.max(1.0);
    ((self.scroll.viewport_height() / step).floor() as usize).max(1)
  }

  fn reveal(&self, row: usize) {
    let Some(key) = self.data.lock().unwrap().keys.get(row).cloned() else {
      return;
    };
    self.reveal_key.set(Some(key));
    self.reveal_request.update(|request| *request = request.wrapping_add(1));
  }
}

/// A row of the virtual list: its place among the rows shown, from which render, and how a property's row moves.
#[derive(Clone, PartialEq)]
struct GridItem {
  place: usize,
  key: String,
  height: f32,
  motion: Option<RowMotion>,
  generation: u64,
}

struct GridRowProps {
  place: usize,
  generation: u64,
  motion: Option<RowMotion>,
  shown: ShownRows,
}

impl PartialEq for GridRowProps {
  fn eq(&self, other: &Self) -> bool {
    self.place == other.place
      && self.generation == other.generation
      && self.motion == other.motion
      && Arc::ptr_eq(&self.shown, &other.shown)
  }
}

impl crate::app::component::DevtoolsInspectable for GridRowProps {}

/// One row of a grid in a virtual list.
struct GridRowHost;

impl Component for GridRowHost {
  type Props = GridRowProps;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<GridRowProps>();
    let (view, row) = {
      let shown = props.shown.lock().unwrap();
      match shown.as_ref() {
        Some((view, rows)) => (Some(view.clone()), rows.get(props.place).copied()),
        None => (None, None),
      }
    };
    let built = match (view, row) {
      (Some(view), Some(row)) => view.shown_row(row, props.place).unwrap_or_else(|| Column::new().into()),
      _ => Column::new().into(),
    };
    let Some(motion) = &props.motion else {
      return built;
    };
    // A property's row in a frame of its height, which runs to the height its category's motion gives it.
    motion.frame(built)
  }
}
