use std::sync::Arc;

use crate::{
  core::{ElementRef, Signal, SignalValue},
  layout::{Alignment, layout_kind::Justify, text_style::TextStyle},
  node::{
    Element, Node, SelectStyle,
    node_kind::{SelectChangeCallback, TextOverflow},
  },
};

enum Binding<T>
where
  T: SignalValue,
  Vec<T>: SignalValue,
{
  Single(Signal<T>),
  Multiple(Signal<Vec<T>>),
}

/// A native, generic dropdown select. Single-select binds a `Signal<T>`;
/// multi-select binds a `Signal<Vec<T>>`. Options pair a value with a label;
/// the selected value(s) are derived by comparing the bound signal against the
/// option values each render, so the control stays in sync reactively.
pub struct Select<T>
where
  T: SignalValue,
  Vec<T>: SignalValue,
{
  node: Node,
  binding: Binding<T>,
  options: Vec<(T, Arc<str>)>,
  placeholder: Option<Arc<str>>,
  style: SelectStyle,
  trigger: Option<Arc<dyn Fn(SelectTriggerState) -> Element + Send + Sync>>,
}

#[derive(Clone)]
pub struct SelectTriggerState {
  pub label: Option<Arc<str>>,
  pub placeholder: Option<Arc<str>>,
  pub selected_labels: Vec<Arc<str>>,
  pub selected_count: usize,
  pub multiple: bool,
}

impl<T> Select<T>
where
  T: Clone + PartialEq + Send + Sync + 'static + SignalValue,
  Vec<T>: SignalValue,
{
  pub fn new(value: Signal<T>) -> Self {
    Self {
      node: Node::select().focusable(true),
      binding: Binding::Single(value),
      options: Vec::new(),
      placeholder: None,
      style: SelectStyle::new(),
      trigger: None,
    }
  }

  pub fn multiple(value: Signal<Vec<T>>) -> Self {
    Self {
      node: Node::select().focusable(true),
      binding: Binding::Multiple(value),
      options: Vec::new(),
      placeholder: None,
      style: SelectStyle::new(),
      trigger: None,
    }
  }

  pub fn options(mut self, options: impl IntoIterator<Item = (T, impl Into<Arc<str>>)>) -> Self {
    self.options = options
      .into_iter()
      .map(|(value, label)| (value, label.into()))
      .collect();
    self
  }

  pub fn placeholder(mut self, placeholder: impl Into<Arc<str>>) -> Self {
    self.placeholder = Some(placeholder.into());
    self
  }

  pub fn style(mut self, style: SelectStyle) -> Self {
    self.style = style;
    self
  }

  pub fn style_with(mut self, f: impl FnOnce(SelectStyle) -> SelectStyle) -> Self {
    self.style = f(SelectStyle::new());
    self
  }

  pub fn trigger<R>(mut self, f: impl Fn(SelectTriggerState) -> R + Send + Sync + 'static) -> Self
  where
    R: Into<Element>,
  {
    self.trigger = Some(Arc::new(move |state| f(state).into()));
    self
  }

  pub fn width(mut self, width: impl Into<crate::node::dimension::Dimension>) -> Self {
    self.node = self.node.width(width);
    self
  }

  pub fn height(mut self, height: impl Into<crate::node::dimension::Dimension>) -> Self {
    self.node = self.node.height(height);
    self
  }

  pub fn tab_index(mut self, tab_index: i32) -> Self {
    self.node = self.node.tab_index(tab_index);
    self
  }

  pub fn ref_element(mut self, element_ref: impl Into<ElementRef>) -> Self {
    self.node = self.node.ref_element(element_ref);
    self
  }

  #[cfg(feature = "form")]
  pub fn name(mut self, name: impl Into<Arc<str>>) -> Self {
    self.node = self.node.name(name);
    self
  }

  fn finalize(self) -> Node {
    let labels: Vec<Arc<str>> = self.options.iter().map(|(_, label)| label.clone()).collect();
    let values: Vec<T> = self.options.into_iter().map(|(value, _)| value).collect();

    let (selected, multiple, on_change) = match self.binding {
      Binding::Single(signal) => {
        let current = signal.get();
        let selected: Vec<usize> = values.iter().position(|value| *value == current).into_iter().collect();
        let option_values = values.clone();
        let on_change: SelectChangeCallback = Arc::new(move |index| {
          if let Some(value) = option_values.get(index) {
            signal.set(value.clone());
          }
        });
        (selected, false, on_change)
      }
      Binding::Multiple(signal) => {
        let current = signal.get();
        let selected: Vec<usize> = values
          .iter()
          .enumerate()
          .filter(|(_, value)| current.contains(value))
          .map(|(index, _)| index)
          .collect();
        let option_values = values.clone();
        let on_change: SelectChangeCallback = Arc::new(move |index| {
          if let Some(value) = option_values.get(index) {
            signal.update(|current| {
              if let Some(position) = current.iter().position(|existing| existing == value) {
                current.remove(position);
              } else {
                current.push(value.clone());
              }
            });
          }
        });
        (selected, true, on_change)
      }
    };
    let selected_labels: Vec<Arc<str>> = selected
      .iter()
      .filter_map(|index| labels.get(*index).cloned())
      .collect();
    let label = match selected_labels.as_slice() {
      [] => self.placeholder.clone(),
      [label] => Some(label.clone()),
      many => Some(Arc::from(format!("{} selected", many.len()))),
    };
    let trigger_state = SelectTriggerState {
      label,
      placeholder: self.placeholder.clone(),
      selected_count: selected.len(),
      selected_labels,
      multiple,
    };
    let trigger = self
      .trigger
      .map(|render| render(trigger_state.clone()).node)
      .unwrap_or_else(|| default_trigger(trigger_state, &self.style));

    self
      .node
      .with_tag_name(Arc::from("Select"))
      .with_children([trigger])
      .select_labels(labels)
      .select_selected(selected)
      .select_multiple(multiple)
      .select_placeholder(self.placeholder)
      .select_style(self.style)
      .select_on_change(on_change)
  }
}

fn default_trigger(state: SelectTriggerState, style: &SelectStyle) -> Node {
  use crate::node::dimension::Dimension;

  let trigger = style.resolved_trigger(false, false, false);
  let is_placeholder = state.selected_count == 0;
  let text = state.label.unwrap_or_default();
  let text_node = match (is_placeholder, style.placeholder_text.as_ref(), trigger.text.as_ref()) {
    (true, Some(text_style), _) => Node::text_styled(&text, text_style.clone()),
    (_, _, Some(text_style)) => Node::text_styled(&text, text_style.clone()),
    _ => Node::text(&text),
  }
  .text_wrap(false)
  .text_overflow(TextOverflow::Elipsis)
  .min_width(0.0)
  .flex(1.0);

  let mut chevron_style = trigger.text.unwrap_or_else(TextStyle::default);
  chevron_style.font_size = style.chevron_size;
  if let Some(color) = style.chevron_color {
    chevron_style.color = color;
  }
  let chevron = Node::text_styled("\u{25BE}", chevron_style)
    .text_wrap(false)
    .width(Dimension::Px(style.chevron_size + 4.0));

  Node::row(8.0, Alignment::Center, vec![text_node, chevron])
    .justify(Justify::Start)
    .width(Dimension::Pct(100.0))
}

impl<T> From<Select<T>> for Element
where
  T: Clone + PartialEq + Send + Sync + 'static + SignalValue,
  Vec<T>: SignalValue,
{
  fn from(select: Select<T>) -> Self {
    Element::from_node(select.finalize())
  }
}
