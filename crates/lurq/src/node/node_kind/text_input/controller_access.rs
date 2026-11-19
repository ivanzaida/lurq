use std::sync::Weak;

use super::*;

pub(crate) struct WeakTextInputState {
  value: Signal<String>,
  inner: Weak<Mutex<TextInputInner>>,
  on_input: Weak<Mutex<Vec<TextInputCallback>>>,
  layout_dirty: Weak<AtomicBool>,
  layout_subscribers: Weak<crate::core::layout_dirty::LayoutSubscribers>,
}

impl WeakTextInputState {
  pub(crate) fn upgrade(&self) -> Option<TextInputState> {
    Some(TextInputState {
      value: self.value.clone(),
      inner: self.inner.upgrade()?,
      on_input: self.on_input.upgrade()?,
      layout_dirty: self.layout_dirty.upgrade()?,
      layout_subscribers: self.layout_subscribers.upgrade()?,
    })
  }
}

impl TextInputState {
  pub(crate) fn downgrade(&self) -> WeakTextInputState {
    WeakTextInputState {
      value: self.value.clone(),
      inner: Arc::downgrade(&self.inner),
      on_input: Arc::downgrade(&self.on_input),
      layout_dirty: Arc::downgrade(&self.layout_dirty),
      layout_subscribers: Arc::downgrade(&self.layout_subscribers),
    }
  }
  pub(crate) fn controller_selection(&self) -> (usize, Option<usize>) {
    self.sync_external_value();
    let inner = self.inner.lock();
    (inner.caret, inner.selection_anchor)
  }

  /// Where the caret before `byte` stood when the input was last laid out, from the start of its text's box
  /// (inside its padding) with its scrolling taken off, and the height of the text's band there: `(x, y,
  /// height)`. A byte past what was laid out is placed at the end of it.
  pub(crate) fn controller_point(&self, byte: usize) -> (f32, f32, f32) {
    let inner = self.inner.lock();
    let byte = byte.min(
      inner
        .caret_positions
        .iter()
        .map(|position| position.index)
        .max()
        .unwrap_or(0),
    );
    let x = caret_x_for_index(&inner.caret_positions, byte) - inner.scroll_x;
    let y = caret_y_for_index(&inner.caret_positions, byte).unwrap_or(inner.caret_y) - inner.scroll_y();
    let (top, height) = inner.text_band.unwrap_or((0.0, inner.caret_height));
    (x, y + top, height)
  }

  /// Has the input painted anew, for a change of what is drawn over it that is not its own to know of.
  pub(crate) fn repaint(&self) {
    self.mark_layout_dirty_keeping_scroll();
  }

  pub(crate) fn controller_select(&self, anchor: usize, caret: usize) -> bool {
    self.sync_external_value();
    let value = self.value();
    if !value.is_char_boundary(anchor) || !value.is_char_boundary(caret) {
      return false;
    }
    let mut inner = self.inner.lock();
    inner.caret = caret;
    inner.token_affinity = None;
    inner.selection_anchor = (anchor != caret).then_some(anchor);
    drop(inner);
    self.mark_layout_dirty();
    true
  }
}
