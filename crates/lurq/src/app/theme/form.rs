use crate::{
  layout::text_style::{FontWeight, TextStyle},
  node::{
    BackgroundColor, CheckboxStyle, SliderPartStyle, TextColor,
    border::{Border, ThemedBorderRadius},
    color::Color,
    dimension::Dimension,
    padding::Padding,
    spacing_value::SpacingValue,
  },
};

#[derive(Clone)]
pub struct FormTheme {
  pub field: FormFieldStyle,
  pub input: FormInputStyle,
  pub checkbox: FormCheckboxStyle,
  pub slider: FormSliderStyle,
  pub primary_button: FormButtonStyle,
  pub secondary_button: FormButtonStyle,
}

#[derive(Clone, PartialEq)]
pub struct FormFieldStyle {
  pub spacing: SpacingValue,
  pub label: TextStyle,
  pub hint: TextStyle,
  pub error: TextStyle,
}

#[derive(Clone, PartialEq)]
pub struct FormInputStyle {
  pub height: Dimension,
  pub padding: Padding,
  pub radius: ThemedBorderRadius,
  pub background: BackgroundColor,
  pub border: Option<Border>,
  pub focused_background: Option<BackgroundColor>,
  pub focused_border: Option<Border>,
  pub error_background: Option<BackgroundColor>,
  pub error_border: Option<Border>,
  pub disabled_background: Option<BackgroundColor>,
  pub disabled_border: Option<Border>,
  pub text: TextStyle,
  pub placeholder: TextStyle,
  pub caret_color: TextColor,
}

#[derive(Clone)]
pub struct FormCheckboxStyle {
  pub box_style: CheckboxStyle,
  pub checked_box_style: CheckboxStyle,
  pub box_hovered_style: CheckboxStyle,
  pub checked_box_hovered_style: CheckboxStyle,
}

#[derive(Clone)]
pub struct FormSliderStyle {
  pub track: SliderPartStyle,
  pub track_hovered: SliderPartStyle,
  pub thumb: SliderPartStyle,
  pub thumb_hovered: SliderPartStyle,
}

#[derive(Clone, PartialEq)]
pub struct FormButtonStyle {
  pub width: Dimension,
  pub height: Dimension,
  pub padding: Padding,
  pub radius: ThemedBorderRadius,
  pub background: BackgroundColor,
  pub border: Option<Border>,
  pub hovered_background: Option<BackgroundColor>,
  pub hovered_border: Option<Border>,
  pub active_background: Option<BackgroundColor>,
  pub active_border: Option<Border>,
  pub text: TextStyle,
}

impl Default for FormTheme {
  fn default() -> Self {
    Self {
      field: FormFieldStyle::default(),
      input: FormInputStyle::default(),
      checkbox: FormCheckboxStyle::default(),
      slider: FormSliderStyle::default(),
      primary_button: FormButtonStyle::primary(),
      secondary_button: FormButtonStyle::secondary(),
    }
  }
}

impl Default for FormFieldStyle {
  fn default() -> Self {
    let base = TextStyle::default();
    Self {
      spacing: SpacingValue::from(4.0),
      label: TextStyle {
        font_size: 13.0,
        weight: FontWeight::Medium,
        color: Color::new(33, 37, 41, 255),
        ..base.clone()
      },
      hint: TextStyle {
        font_size: 12.0,
        color: Color::new(108, 117, 125, 255),
        ..base.clone()
      },
      error: TextStyle {
        font_size: 12.0,
        color: Color::new(220, 53, 69, 255),
        ..base
      },
    }
  }
}

impl Default for FormInputStyle {
  fn default() -> Self {
    let text = TextStyle {
      color: Color::new(33, 37, 41, 255),
      ..TextStyle::default()
    };
    Self {
      height: Dimension::Px(36.0),
      padding: Padding::symmetric(10.0, 8.0),
      radius: ThemedBorderRadius::all(4.0),
      background: BackgroundColor::from(Color::new(255, 255, 255, 255)),
      border: Some(Border::inside(1.0, Color::new(206, 212, 218, 255))),
      focused_background: None,
      focused_border: Some(Border::inside(1.0, Color::new(13, 110, 253, 255))),
      error_background: None,
      error_border: Some(Border::inside(1.0, Color::new(220, 53, 69, 255))),
      disabled_background: Some(BackgroundColor::from(Color::new(233, 236, 239, 255))),
      disabled_border: None,
      placeholder: TextStyle {
        color: Color::new(108, 117, 125, 255),
        ..text.clone()
      },
      text,
      caret_color: TextColor::from(Color::new(13, 110, 253, 255)),
    }
  }
}

impl Default for FormCheckboxStyle {
  fn default() -> Self {
    Self {
      box_style: CheckboxStyle::new()
        .size(16.0, 16.0)
        .background(Color::new(255, 255, 255, 255))
        .rounded(4.0)
        .border_inside(1.0, Color::new(206, 212, 218, 255)),
      checked_box_style: CheckboxStyle::new()
        .background(Color::new(13, 110, 253, 255))
        .border_inside(1.0, Color::new(13, 110, 253, 255))
        .indicator_size(8.0, 8.0),
      box_hovered_style: CheckboxStyle::new().border_inside(1.0, Color::new(13, 110, 253, 255)),
      checked_box_hovered_style: CheckboxStyle::new().background(Color::new(11, 94, 215, 255)),
    }
  }
}

impl Default for FormSliderStyle {
  fn default() -> Self {
    Self {
      track: SliderPartStyle::new()
        .height(4.0)
        .background(Color::new(222, 226, 230, 255))
        .rounded(999.0),
      track_hovered: SliderPartStyle::new().background(Color::new(206, 212, 218, 255)),
      thumb: SliderPartStyle::new()
        .size(16.0, 16.0)
        .background(Color::new(13, 110, 253, 255))
        .rounded(999.0),
      thumb_hovered: SliderPartStyle::new().background(Color::new(11, 94, 215, 255)),
    }
  }
}

impl FormButtonStyle {
  pub fn primary() -> Self {
    Self {
      width: Dimension::Pct(100.0),
      height: Dimension::Px(36.0),
      padding: Padding::symmetric(14.0, 8.0),
      radius: ThemedBorderRadius::all(4.0),
      background: BackgroundColor::from(Color::new(13, 110, 253, 255)),
      border: Some(Border::inside(1.0, Color::new(13, 110, 253, 255))),
      hovered_background: Some(BackgroundColor::from(Color::new(11, 94, 215, 255))),
      hovered_border: Some(Border::inside(1.0, Color::new(10, 88, 202, 255))),
      active_background: Some(BackgroundColor::from(Color::new(10, 88, 202, 255))),
      active_border: Some(Border::inside(1.0, Color::new(10, 83, 190, 255))),
      text: TextStyle {
        font_size: 13.0,
        weight: FontWeight::Medium,
        color: Color::new(255, 255, 255, 255),
        ..TextStyle::default()
      },
    }
  }

  pub fn secondary() -> Self {
    Self {
      width: Dimension::Pct(100.0),
      height: Dimension::Px(36.0),
      padding: Padding::symmetric(14.0, 8.0),
      radius: ThemedBorderRadius::all(4.0),
      background: BackgroundColor::from(Color::new(255, 255, 255, 255)),
      border: Some(Border::inside(1.0, Color::new(108, 117, 125, 255))),
      hovered_background: Some(BackgroundColor::from(Color::new(108, 117, 125, 255))),
      hovered_border: Some(Border::inside(1.0, Color::new(108, 117, 125, 255))),
      active_background: Some(BackgroundColor::from(Color::new(90, 98, 104, 255))),
      active_border: Some(Border::inside(1.0, Color::new(86, 94, 100, 255))),
      text: TextStyle {
        font_size: 13.0,
        weight: FontWeight::Medium,
        color: Color::new(33, 37, 41, 255),
        ..TextStyle::default()
      },
    }
  }
}
