mod text_input {
  mod basic_layout;
  mod editing;
  mod empty_value;
  mod focus;
  mod preserves_editing_state_across_render;
  mod renders_caret;
  mod style;
  mod typing_updates_value;
}

mod checkbox {
  mod double_toggle;
  mod keyboard_toggle;
  mod toggles_on_click;
}

mod slider {
  mod avoids_unchanged_updates;
  mod clamps_min_max;
  mod drag_updates_value;
  mod keyboard_update;
  mod renders_thumb;
  mod updates_from_click;
}
