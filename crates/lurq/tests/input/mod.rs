mod text {
  mod selectable;
}

mod select {
  mod interaction;
}

mod text_input {
  mod basic_layout;
  mod editing;
  mod empty_value;
  mod focus;
  mod overflow_anchor;
  mod preserves_editing_state_across_render;
  mod renders_caret;
  mod soft_wrap_selection;
  mod style;
  mod typing_updates_value;
}

mod checkbox {
  mod double_toggle;
  mod keyboard_toggle;
  mod renders_style;
  mod toggles_on_click;
}

#[cfg(feature = "form")]
mod forms {
  mod async_submit;
  mod compound;
  mod control;
  mod enter_submits;
  mod form_data;
  mod logical_wrapper;
  mod prefill;
  mod rerender;
  mod state;
  mod submit_button;
  mod tab_navigation;
  mod validation;
}

mod slider {
  mod avoids_unchanged_updates;
  mod clamps_min_max;
  mod drag_updates_value;
  mod keyboard_update;
  mod renders_thumb;
  mod suppresses_parent_click;
  mod updates_from_click;
}
