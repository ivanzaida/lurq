use std::sync::Arc;

use crate::{
  app::{ctx::Ctx, theme::BorderSize},
  components::{Control, FormControlField, FormFieldProps, TextInput},
  node::{Element, Style, border::Border, dimension::Dimension},
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
    let palette = ctx.theme().palette().clone();
    let typography = ctx.theme().typography().clone();
    let has_error = control.should_show_error();
    let disabled = control.is_disabled();

    let background = if disabled {
      crate::app::theme::PaletteColor::SurfacePanel
    } else if has_error {
      input_style.background_error
    } else {
      input_style.background
    };
    let border = if disabled {
      input_style.border
    } else if has_error {
      input_style.border_error
    } else {
      input_style.border
    };
    let height = props.height.unwrap_or(input_style.height);

    let clear_error_form = props.control.form();
    let clear_error_name = Arc::<str>::from(props.control.name());
    ctx.watch(&value, move |_| {
      clear_error_form.clear_error(&clear_error_name);
    });

    let mut placeholder_style = input_style.placeholder.resolve(&typography, &palette);
    placeholder_style.color = placeholder_style.color.with_opacity(0.4);

    let mut input = TextInput::styled(value, input_style.text.resolve(&typography, &palette))
      .name(control.name())
      .width(Dimension::Pct(100.0))
      .height(height)
      .padding_custom(input_style.padding)
      .background(background)
      .placeholder_style(placeholder_style)
      .caret_color(input_style.caret)
      .border(Border::inside(BorderSize::Sm, border))
      .rounded(input_style.radius);

    if let Some(placeholder) = props.placeholder.as_deref() {
      input = input.placeholder(placeholder);
    }
    if !has_error && !disabled {
      let mut focused = Style::new();
      focused = focused.border(Border::inside(BorderSize::Sm, input_style.border_focus));
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
