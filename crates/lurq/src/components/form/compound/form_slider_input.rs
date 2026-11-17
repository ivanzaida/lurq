use std::sync::Arc;

use super::form_field::form_field;
use crate::{
  app::ctx::Ctx,
  components::Control,
  core::Signal,
  node::Element,
  ui::{self, Length, SliderPart},
};

#[derive(Clone, Debug, PartialEq, crate::DevtoolsInspectable)]
pub struct FormSliderInputProps {
  pub control: Control<f64>,
  pub label: Option<Arc<str>>,
  pub hint: Option<Arc<str>>,
  pub min: i32,
  pub max: i32,
}

impl FormSliderInputProps {
  pub fn new(control: Control<f64>) -> Self {
    Self {
      control,
      label: None,
      hint: None,
      min: 0,
      max: 100,
    }
  }

  pub fn label(mut self, label: impl Into<Arc<str>>) -> Self {
    self.label = Some(label.into());
    self
  }

  pub fn hint(mut self, hint: impl Into<Arc<str>>) -> Self {
    self.hint = Some(hint.into());
    self
  }

  pub fn range(mut self, min: i32, max: i32) -> Self {
    self.min = min;
    self.max = max;
    self
  }
}

/// A slider of a form: the design system's [`ui::Slider`] in a [`ui::Field`], bound to a number control in whole
/// steps from the least of its range to the greatest. Its label is above it and its hint under it, or the form's
/// error in the hint's place; the form judges the field when the slider loses the focus. The slider is as wide as
/// the field, which is as wide as what holds it. It is styled by the theme's recipes for `ui::Slider` and
/// `ui::Field`.
pub struct FormSliderInput {
  value: Signal<i32>,
}

impl crate::app::component::Component for FormSliderInput {
  type Props = FormSliderInputProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let field_value = props.control.field().value();
    let slider_value = ctx.signal(clamp_slider_value(field_value.get_untracked(), props.min, props.max));

    let field_from_slider = field_value.clone();
    ctx.watch(&slider_value, move |value| {
      let next = *value as f64;
      if field_from_slider.get_untracked() != next {
        field_from_slider.set(next);
      }
    });

    let slider_from_field = slider_value.clone();
    ctx.watch(&field_value, move |value| {
      let next = clamp_slider_value(*value, props.min, props.max);
      if slider_from_field.get_untracked() != next {
        slider_from_field.set(next);
      }
    });

    Self { value: slider_value }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let control = ctx.form_control(&props.control);
    let blur_control = control.clone();
    // A range given the wrong way round is the range between the two, as the value is kept in it.
    let (min, max) = (props.min.min(props.max), props.min.max(props.max));

    let slider = ui::Slider::new(self.value.clone())
      .min(min)
      .max(max)
      .name(control.name())
      .style(SliderPart::Root, |s| s.width(Length::Fill))
      .on_leave(move || {
        blur_control.mark_touched();
        blur_control.validate();
      });

    form_field(&control, props.label.as_deref(), props.hint.as_deref(), slider)
  }
}

fn clamp_slider_value(value: f64, min: i32, max: i32) -> i32 {
  let (min, max) = if min <= max { (min, max) } else { (max, min) };
  value.round().clamp(min as f64, max as f64) as i32
}
