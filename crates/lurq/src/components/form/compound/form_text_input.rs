use std::sync::Arc;

use crate::{
  app::ctx::Ctx,
  components::{Control, FormControlField, FormFieldProps, TextInput},
  node::{Element, Style, dimension::Dimension},
};

#[derive(Clone, Debug, PartialEq, crate::DevtoolsInspectable)]
pub struct FormTextInputProps {
  pub control: Control<String>,
  pub label: Option<Arc<str>>,
  pub hint: Option<Arc<str>>,
  pub placeholder: Option<Arc<str>>,
  #[devtools_ignore]
  pub height: Option<Dimension>,
  pub multiline: bool,
}

impl FormTextInputProps {
  pub fn new(control: Control<String>) -> Self {
    Self {
      control,
      label: None,
      hint: None,
      placeholder: None,
      height: None,
      multiline: false,
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

  pub fn placeholder(mut self, placeholder: impl Into<Arc<str>>) -> Self {
    self.placeholder = Some(placeholder.into());
    self
  }

  pub fn height(mut self, height: impl Into<Dimension>) -> Self {
    self.height = Some(height.into());
    self
  }

  pub fn multiline(mut self) -> Self {
    self.multiline = true;
    self
  }
}

pub struct FormTextInput;

impl crate::app::component::Component for FormTextInput {
  type Props = FormTextInputProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let control = ctx.form_control(&props.control);
    let value = control.value();
    let input_style = ctx.theme().form().input.clone();
    let has_error = control.should_show_error();
    let disabled = control.is_disabled();

    let background = if disabled {
      input_style.disabled_background.unwrap_or(input_style.background)
    } else if has_error {
      input_style.error_background.unwrap_or(input_style.background)
    } else {
      input_style.background
    };
    let border = if disabled {
      input_style.disabled_border.or(input_style.border)
    } else if has_error {
      input_style.error_border.or(input_style.border)
    } else {
      input_style.border
    };
    let height = props.height.unwrap_or(input_style.height);

    let clear_error_form = props.control.form();
    let clear_error_name = Arc::<str>::from(props.control.name());
    ctx.watch(&value, move |_| {
      clear_error_form.clear_error(&clear_error_name);
    });

    let mut input = TextInput::styled(value, input_style.text.clone())
      .name(control.name())
      .width(Dimension::Pct(100.0))
      .height(height)
      .padding_custom(input_style.padding)
      .background(background)
      .placeholder_style(input_style.placeholder.clone())
      .caret_color(input_style.caret_color);

    if let Some(placeholder) = props.placeholder.as_deref() {
      input = input.placeholder(placeholder);
    }
    if let Some(border) = border {
      input = input.border(border);
    }
    if let Some(radius) = input_style.radius.as_border_radius() {
      input = input.corner_radius_custom(radius);
    }
    if !has_error && !disabled && (input_style.focused_background.is_some() || input_style.focused_border.is_some()) {
      let mut focused = Style::new();
      if let Some(background) = input_style.focused_background {
        focused = focused.background(background);
      }
      if let Some(border) = input_style.focused_border {
        focused = focused.border(border);
      }
      input = input.focused_style(focused);
    }
    input = if props.multiline {
      input.multiline()
    } else {
      input.single_line()
    };

    ctx.mount_with::<FormControlField<String>>(
      FormFieldProps::new(props.control)
        .maybe_label(props.label)
        .maybe_hint(props.hint),
      vec![input.into()],
    )
  }
}
