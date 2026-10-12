use super::*;

#[derive(Clone)]
pub(crate) struct CheckboxState {
  value: Signal<bool>,
  inner: Arc<Mutex<CheckboxInner>>,
}

struct CheckboxInner {
  style: CheckboxStyle,
  /// Whether the app set the box style; without one the box takes the
  /// theme's fill.
  box_styled: bool,
  checked_style: Option<CheckboxStyle>,
  hovered_style: Option<CheckboxStyle>,
  checked_hovered_style: Option<CheckboxStyle>,
  focused_style: Option<CheckboxStyle>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CheckboxLayoutSignature {
  width: Option<f32>,
  height: Option<f32>,
  checked_width: Option<f32>,
  checked_height: Option<f32>,
  hovered_width: Option<f32>,
  hovered_height: Option<f32>,
  checked_hovered_width: Option<f32>,
  checked_hovered_height: Option<f32>,
}

impl CheckboxState {
  pub(crate) fn new(value: Signal<bool>) -> Self {
    Self {
      value,
      inner: Arc::new(Mutex::new(CheckboxInner {
        style: CheckboxStyle::new(),
        box_styled: false,
        checked_style: None,
        hovered_style: None,
        checked_hovered_style: None,
        focused_style: None,
      })),
    }
  }

  pub(crate) fn is_checked(&self) -> bool {
    self.value.get_untracked()
  }

  pub(crate) fn toggle(&self) {
    self.value.update(|checked| *checked = !*checked);
  }

  pub(crate) fn set_checked(&self, checked: bool) {
    if self.is_checked() != checked {
      self.value.set(checked);
    }
  }

  pub(crate) fn set_style(&self, style: CheckboxStyle) {
    let mut inner = self.inner.lock().unwrap();
    inner.style = style;
    inner.box_styled = true;
  }

  /// Whether the app set the box style, or the checked box style when
  /// `checked`, which decides whether an unset fill falls back to the theme.
  pub(crate) fn part_styled(&self, checked: bool) -> bool {
    let inner = self.inner.lock().unwrap();
    if checked {
      inner.checked_style.is_some()
    } else {
      inner.box_styled
    }
  }

  pub(crate) fn set_checked_style(&self, style: CheckboxStyle) {
    self.inner.lock().unwrap().checked_style = Some(style);
  }

  pub(crate) fn set_hovered_style(&self, style: CheckboxStyle) {
    self.inner.lock().unwrap().hovered_style = Some(style);
  }

  pub(crate) fn set_checked_hovered_style(&self, style: CheckboxStyle) {
    self.inner.lock().unwrap().checked_hovered_style = Some(style);
  }

  pub(crate) fn set_focused_style(&self, style: CheckboxStyle) {
    self.inner.lock().unwrap().focused_style = Some(style);
  }

  /// Layers: base, checked, focused (paint only; its size is ignored so focus
  /// never moves layout), hovered, checked and hovered.
  pub(crate) fn style(&self, checked: bool, hovered: bool, focused: bool) -> CheckboxStyle {
    let inner = self.inner.lock().unwrap();
    let mut style = inner.style.clone();
    if checked && let Some(checked_style) = &inner.checked_style {
      style.merge_from(checked_style);
    }
    if focused && let Some(focused_style) = &inner.focused_style {
      let (width, height) = (style.width, style.height);
      style.merge_from(focused_style);
      style.width = width;
      style.height = height;
    }
    if hovered && let Some(hovered_style) = &inner.hovered_style {
      style.merge_from(hovered_style);
    }
    if checked
      && hovered
      && let Some(checked_hovered_style) = &inner.checked_hovered_style
    {
      style.merge_from(checked_hovered_style);
    }
    style
  }

  pub(crate) fn preferred_size(&self, default_width: f32, default_height: f32) -> (f32, f32) {
    let inner = self.inner.lock().unwrap();
    let width = std::iter::once(inner.style.width)
      .chain(inner.checked_style.as_ref().map(|style| style.width))
      .chain(inner.hovered_style.as_ref().map(|style| style.width))
      .chain(inner.checked_hovered_style.as_ref().map(|style| style.width))
      .flatten()
      .fold(default_width, f32::max);
    let height = std::iter::once(inner.style.height)
      .chain(inner.checked_style.as_ref().map(|style| style.height))
      .chain(inner.hovered_style.as_ref().map(|style| style.height))
      .chain(inner.checked_hovered_style.as_ref().map(|style| style.height))
      .flatten()
      .fold(default_height, f32::max);
    (width, height)
  }

  pub(crate) fn layout_signature(&self) -> CheckboxLayoutSignature {
    let inner = self.inner.lock().unwrap();
    CheckboxLayoutSignature {
      width: inner.style.width,
      height: inner.style.height,
      checked_width: inner.checked_style.as_ref().and_then(|style| style.width),
      checked_height: inner.checked_style.as_ref().and_then(|style| style.height),
      hovered_width: inner.hovered_style.as_ref().and_then(|style| style.width),
      hovered_height: inner.hovered_style.as_ref().and_then(|style| style.height),
      checked_hovered_width: inner.checked_hovered_style.as_ref().and_then(|style| style.width),
      checked_hovered_height: inner.checked_hovered_style.as_ref().and_then(|style| style.height),
    }
  }

  #[cfg(all(feature = "image", feature = "resources"))]
  pub(crate) fn resolve_resource_images(
    &self,
    mut resolve: impl FnMut(&Arc<str>) -> Option<crate::images::ImageData>,
  ) -> bool {
    fn resolve_style(
      style: &mut CheckboxStyle,
      resolve: &mut impl FnMut(&Arc<str>) -> Option<crate::images::ImageData>,
    ) -> bool {
      let Some(key) = style.indicator_resource_image.clone() else {
        return false;
      };
      let Some(img) = resolve(&key) else {
        return false;
      };
      if style.indicator_image.as_ref().map(crate::images::ImageData::id) != Some(img.id()) {
        style.indicator_image = Some(img);
        return true;
      }
      false
    }

    let mut inner = self.inner.lock().unwrap();
    let mut changed = resolve_style(&mut inner.style, &mut resolve);
    if let Some(style) = &mut inner.checked_style {
      changed |= resolve_style(style, &mut resolve);
    }
    if let Some(style) = &mut inner.hovered_style {
      changed |= resolve_style(style, &mut resolve);
    }
    if let Some(style) = &mut inner.checked_hovered_style {
      changed |= resolve_style(style, &mut resolve);
    }
    if let Some(style) = &mut inner.focused_style {
      changed |= resolve_style(style, &mut resolve);
    }
    changed
  }
}
