use std::sync::Arc;

use crate::{
  app::ctx::Ctx,
  components::{Control, FormControlField, FormFieldProps, Slider},
  core::Signal,
  node::{Element, dimension::Dimension},
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
    let slider_style = ctx.theme().form().slider.clone();
    let palette = ctx.theme().palette().clone();
    let blur_control = control.clone();

    let slider = Slider::new(self.value.clone())
      .name(control.name())
      .width(Dimension::Pct(100.0))
      .range(props.min, props.max)
      .track_style(slider_style.track_style(&palette))
      .track_hovered_style(slider_style.track_hovered_style(&palette))
      .thumb_style(slider_style.thumb_style(&palette))
      .thumb_hovered_style(slider_style.thumb_hovered_style(&palette))
      .on_blur(move || {
        blur_control.mark_touched();
        blur_control.validate();
      });

    ctx.mount_with::<FormControlField<f64>>(
      FormFieldProps::new(props.control)
        .maybe_label(props.label)
        .maybe_hint(props.hint),
      vec![slider.into()],
    )
  }
}

fn clamp_slider_value(value: f64, min: i32, max: i32) -> i32 {
  let (min, max) = if min <= max { (min, max) } else { (max, min) };
  value.round().clamp(min as f64, max as f64) as i32
}
