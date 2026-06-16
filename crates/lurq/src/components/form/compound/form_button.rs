use std::{fmt, sync::Arc};

use crate::{
  app::{ctx::Ctx, events::MouseEvent, theme::BorderSize},
  components::{Button, Text},
  layout::{Alignment, layout_kind::Justify},
  node::{CursorIcon, Element, Style, border::Border},
};

type ClickCallback = Arc<dyn Fn(&MouseEvent) + Send + Sync>;

#[derive(Clone, crate::DevtoolsInspectable)]
pub struct FormPrimaryButtonProps {
  pub label: Arc<str>,
  pub submit: bool,
  #[devtools_ignore]
  pub on_click: Option<ClickCallback>,
}

#[derive(Clone, crate::DevtoolsInspectable)]
pub struct FormSecondaryButtonProps {
  pub label: Arc<str>,
  pub submit: bool,
  #[devtools_ignore]
  pub on_click: Option<ClickCallback>,
}

impl FormPrimaryButtonProps {
  pub fn new(label: impl Into<Arc<str>>) -> Self {
    Self {
      label: label.into(),
      submit: true,
      on_click: None,
    }
  }

  pub fn submit(mut self) -> Self {
    self.submit = true;
    self
  }

  pub fn button(mut self) -> Self {
    self.submit = false;
    self
  }

  pub fn on_click(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.on_click = Some(Arc::new(f));
    self
  }
}

impl FormSecondaryButtonProps {
  pub fn new(label: impl Into<Arc<str>>) -> Self {
    Self {
      label: label.into(),
      submit: false,
      on_click: None,
    }
  }

  pub fn submit(mut self) -> Self {
    self.submit = true;
    self
  }

  pub fn button(mut self) -> Self {
    self.submit = false;
    self
  }

  pub fn on_click(mut self, f: impl Fn(&MouseEvent) + Send + Sync + 'static) -> Self {
    self.on_click = Some(Arc::new(f));
    self
  }
}

impl fmt::Debug for FormPrimaryButtonProps {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    debug_button_props(f, "FormPrimaryButtonProps", &self.label, self.submit, &self.on_click)
  }
}

impl fmt::Debug for FormSecondaryButtonProps {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    debug_button_props(f, "FormSecondaryButtonProps", &self.label, self.submit, &self.on_click)
  }
}

impl PartialEq for FormPrimaryButtonProps {
  fn eq(&self, other: &Self) -> bool {
    self.label == other.label && self.submit == other.submit && same_callback(&self.on_click, &other.on_click)
  }
}

impl PartialEq for FormSecondaryButtonProps {
  fn eq(&self, other: &Self) -> bool {
    self.label == other.label && self.submit == other.submit && same_callback(&self.on_click, &other.on_click)
  }
}

pub struct FormPrimaryButton;

impl crate::app::component::Component for FormPrimaryButton {
  type Props = FormPrimaryButtonProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let style = ctx.theme().form().button.primary.clone();
    let palette = ctx.theme().palette().clone();
    let typography = ctx.theme().typography().clone();
    render_form_button(&props.label, props.submit, props.on_click, style, &typography, &palette)
  }
}

pub struct FormSecondaryButton;

impl crate::app::component::Component for FormSecondaryButton {
  type Props = FormSecondaryButtonProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let style = ctx.theme().form().button.secondary.clone();
    let palette = ctx.theme().palette().clone();
    let typography = ctx.theme().typography().clone();
    render_form_button(&props.label, props.submit, props.on_click, style, &typography, &palette)
  }
}

fn render_form_button(
  label: &str,
  submit: bool,
  on_click: Option<ClickCallback>,
  style: crate::app::theme::FormButtonRole,
  typography: &crate::app::theme::ThemeTypography,
  palette: &crate::app::theme::ThemePalette,
) -> Button {
  let mut button = Button::empty()
    .width(style.width)
    .height(style.height)
    .padding_custom(style.padding)
    .background(style.background)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .cursor(CursorIcon::Pointer)
    .child(Text::styled(label, style.text.resolve(typography, palette)))
    .border(Border::inside(BorderSize::Sm, style.border))
    .rounded(style.radius);

  if submit {
    button = button.submit();
  } else {
    button = button.button();
  }
  button = button.hovered_style(state_style(style.background_hover, style.border_hover));
  button = button.active_style(state_style(style.background_active, style.border_active));
  if let Some(on_click) = on_click {
    button = button.on_click(move |event| on_click(&event));
  }

  button
}

fn state_style(background: crate::app::theme::PaletteColor, border: crate::app::theme::PaletteColor) -> Style {
  let mut style = Style::new();
  style = style.background(background);
  style = style.border(Border::inside(BorderSize::Sm, border));
  style
}

fn debug_button_props(
  f: &mut fmt::Formatter<'_>,
  name: &str,
  label: &Arc<str>,
  submit: bool,
  on_click: &Option<ClickCallback>,
) -> fmt::Result {
  f.debug_struct(name)
    .field("label", label)
    .field("submit", &submit)
    .field("on_click", &on_click.as_ref().map(|_| "<callback>"))
    .finish()
}

fn same_callback(left: &Option<ClickCallback>, right: &Option<ClickCallback>) -> bool {
  match (left, right) {
    (Some(left), Some(right)) => Arc::ptr_eq(left, right),
    (None, None) => true,
    _ => false,
  }
}
