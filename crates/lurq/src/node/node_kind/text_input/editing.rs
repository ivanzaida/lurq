use super::*;

impl TextInputState {
  pub(crate) fn insert(&self, text: &str, keyboard: &KeyboardEvent) -> bool {
    if text.is_empty() {
      return false;
    }
    self.replace_selection_text(text, keyboard);
    true
  }

  pub(crate) fn replace_selection_text(&self, text: &str, keyboard: &KeyboardEvent) -> bool {
    if text.is_empty() && !self.has_selection() {
      return false;
    }
    if !self.fire_input(keyboard, || {
      let (mut value, caret) = self.value_without_selection();
      let caret = clamp_to_char_boundary(&value, caret);
      value.insert_str(caret, text);
      value
    }) {
      return false;
    }

    self.push_undo_snapshot();
    let mut caret = self
      .delete_selection_if_present()
      .unwrap_or_else(|| self.inner.lock().caret);
    self.value.update(|value| {
      caret = clamp_to_char_boundary(value, caret);
      value.insert_str(caret, text);
      caret += text.len();
    });
    let value = self.value();
    let mut inner = self.inner.lock();
    // A controller command must not be mistaken for an external whole-value
    // write when its new caret happens to equal the previous source length.
    inner.last_value = value;
    inner.caret = caret;
    inner.selection_anchor = None;
    drop(inner);
    self.mark_layout_dirty();
    true
  }

  pub(crate) fn insert_newline(&self, keyboard: &KeyboardEvent) -> bool {
    if self.overflow() != TextInputOverflow::Multiline {
      return false;
    }
    self.insert("\n", keyboard)
  }

  pub(crate) fn backspace(&self, keyboard: &KeyboardEvent) -> bool {
    if self.has_selection() {
      if !self.fire_input(keyboard, || self.value_without_selection().0) {
        return true;
      }
      self.push_undo_snapshot();
      self.delete_selection_if_present();
      return true;
    }

    let mut caret = self.inner.lock().caret;
    if caret == 0 {
      return false;
    }
    if !self.fire_input(keyboard, || {
      let mut value = self.value();
      let caret = clamp_to_char_boundary(&value, caret);
      if caret > 0 {
        let previous = previous_char_boundary(&value, caret);
        value.replace_range(previous..caret, "");
      }
      value
    }) {
      return true;
    }

    self.push_undo_snapshot();
    self.value.update(|value| {
      caret = clamp_to_char_boundary(value, caret);
      if caret > 0 {
        let previous = previous_char_boundary(value, caret);
        value.replace_range(previous..caret, "");
        caret = previous;
      }
    });
    self.inner.lock().caret = caret;
    self.mark_layout_dirty();
    true
  }

  pub(crate) fn delete(&self, keyboard: &KeyboardEvent) -> bool {
    if self.has_selection() {
      if !self.fire_input(keyboard, || self.value_without_selection().0) {
        return true;
      }
      self.push_undo_snapshot();
      self.delete_selection_if_present();
      return true;
    }

    let mut caret = self.inner.lock().caret;
    let value = self.value();
    caret = clamp_to_char_boundary(&value, caret);
    if caret >= value.len() {
      return false;
    }
    if !self.fire_input(keyboard, || {
      let mut value = value.clone();
      let next = next_char_boundary(&value, caret);
      value.replace_range(caret..next, "");
      value
    }) {
      return true;
    }

    self.push_undo_snapshot();
    self.value.update(|value| {
      let next = next_char_boundary(value, caret);
      value.replace_range(caret..next, "");
    });
    self.inner.lock().caret = caret;
    self.mark_layout_dirty();
    true
  }

  pub(crate) fn move_left(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock();
    if selecting {
      if inner.selection_anchor.is_none() {
        inner.selection_anchor = Some(inner.caret);
      }
      inner.caret = previous_char_boundary(&value, clamp_to_char_boundary(&value, inner.caret));
    } else if let Some((start, _)) = selection_range_indices(&value, inner.selection_anchor, inner.caret) {
      inner.caret = start;
      inner.selection_anchor = None;
    } else {
      inner.caret = previous_char_boundary(&value, clamp_to_char_boundary(&value, inner.caret));
      inner.selection_anchor = None;
    }
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn move_right(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock();
    if selecting {
      if inner.selection_anchor.is_none() {
        inner.selection_anchor = Some(inner.caret);
      }
      inner.caret = next_char_boundary(&value, clamp_to_char_boundary(&value, inner.caret));
    } else if let Some((_, end)) = selection_range_indices(&value, inner.selection_anchor, inner.caret) {
      inner.caret = end;
      inner.selection_anchor = None;
    } else {
      inner.caret = next_char_boundary(&value, clamp_to_char_boundary(&value, inner.caret));
      inner.selection_anchor = None;
    }
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn move_up(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock();
    let caret = clamp_to_char_boundary(&value, inner.caret);
    let (line_start, _) = line_bounds(&value, caret);
    if line_start == 0 {
      move_inner_to(&mut inner, 0, selecting);
      drop(inner);
      self.mark_layout_dirty();
      return;
    }

    let target_x = caret_x_for_index(&inner.caret_positions, caret);
    let previous_line_end = line_start - 1;
    let (previous_line_start, previous_line_end) = line_bounds(&value, previous_line_end);
    let target = closest_caret_in_range(&inner.caret_positions, previous_line_start, previous_line_end, target_x);
    move_inner_to(&mut inner, target, selecting);
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn move_down(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock();
    let caret = clamp_to_char_boundary(&value, inner.caret);
    let (_, line_end) = line_bounds(&value, caret);
    if line_end >= value.len() {
      move_inner_to(&mut inner, value.len(), selecting);
      drop(inner);
      self.mark_layout_dirty();
      return;
    }

    let target_x = caret_x_for_index(&inner.caret_positions, caret);
    let next_line_start = line_end + 1;
    let (_, next_line_end) = line_bounds(&value, next_line_start);
    let target = closest_caret_in_range(&inner.caret_positions, next_line_start, next_line_end, target_x);
    move_inner_to(&mut inner, target, selecting);
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn move_word_left(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock();
    let target = previous_word_boundary(&value, inner.caret);
    move_inner_to(&mut inner, target, selecting);
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn move_word_right(&self, selecting: bool) {
    let value = self.value();
    let mut inner = self.inner.lock();
    let target = next_word_boundary(&value, inner.caret);
    move_inner_to(&mut inner, target, selecting);
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn move_home(&self, selecting: bool) {
    let mut inner = self.inner.lock();
    if selecting && inner.selection_anchor.is_none() {
      inner.selection_anchor = Some(inner.caret);
    }
    inner.caret = 0;
    if !selecting {
      inner.selection_anchor = None;
    }
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn move_end(&self, selecting: bool) {
    let len = self.value().len();
    let mut inner = self.inner.lock();
    if selecting && inner.selection_anchor.is_none() {
      inner.selection_anchor = Some(inner.caret);
    }
    inner.caret = len;
    if !selecting {
      inner.selection_anchor = None;
    }
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn select_all(&self) {
    let len = self.value().len();
    let mut inner = self.inner.lock();
    inner.selection_anchor = Some(0);
    inner.caret = len;
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn selected_text(&self) -> Option<String> {
    let value = self.value();
    let inner = self.inner.lock();
    let (start, end) = selection_range_indices(&value, inner.selection_anchor, inner.caret)?;
    Some(value[start..end].to_owned())
  }

  pub(crate) fn cut_selection(&self, keyboard: &KeyboardEvent) -> Option<String> {
    let selected = self.selected_text()?;
    if !self.fire_input(keyboard, || self.value_without_selection().0) {
      return Some(selected);
    }
    self.push_undo_snapshot();
    self.delete_selection_if_present();
    Some(selected)
  }

  pub(crate) fn undo(&self, keyboard: &KeyboardEvent) -> bool {
    let current = self.snapshot();
    let Some(snapshot) = self.inner.lock().undo_stack.pop() else {
      return false;
    };
    if !self.fire_input(keyboard, || snapshot.value.clone()) {
      self.push_undo_snapshot_value(snapshot);
      return true;
    }
    self.push_redo_snapshot(current);
    self.restore_snapshot(snapshot);
    self.mark_layout_dirty();
    true
  }

  pub(crate) fn redo(&self, keyboard: &KeyboardEvent) -> bool {
    let current = self.snapshot();
    let Some(snapshot) = self.inner.lock().redo_stack.pop() else {
      return false;
    };
    if !self.fire_input(keyboard, || snapshot.value.clone()) {
      self.push_redo_snapshot(snapshot);
      return true;
    }
    self.push_undo_snapshot_value(current);
    self.restore_snapshot(snapshot);
    self.mark_layout_dirty();
    true
  }

  pub(crate) fn begin_selection_at_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    let mut inner = self.inner.lock();
    inner.caret = caret;
    inner.selection_anchor = Some(caret);
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn update_selection_to_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    self.inner.lock().caret = caret;
    self.mark_layout_dirty();
  }

  pub(crate) fn set_caret_from_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    let mut inner = self.inner.lock();
    inner.caret = caret;
    inner.selection_anchor = None;
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn select_word_at_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    let value = self.value();
    let (start, end) = word_selection_bounds(&value, caret);
    let mut inner = self.inner.lock();
    inner.selection_anchor = Some(start);
    inner.caret = end;
    drop(inner);
    self.mark_layout_dirty();
  }

  pub(crate) fn select_line_at_point(&self, x: f32, y: f32) {
    let caret = self.closest_caret_to_point(x, y);
    let value = self.value();
    let (start, end) = line_bounds(&value, caret);
    let mut inner = self.inner.lock();
    inner.selection_anchor = Some(start);
    inner.caret = end;
    drop(inner);
    self.mark_layout_dirty();
  }
}
