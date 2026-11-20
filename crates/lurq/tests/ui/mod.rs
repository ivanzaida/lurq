//! The design system's components and what a theme styles them with.

mod accordion;
mod action_bar;
mod action_sheet;
mod ai_label;
mod alert;
mod angle_slider;
mod audio_player;
mod audio_queue;
mod avatar;
mod back_to_top;
mod badge;
#[cfg(feature = "canvas")]
mod bar_chart;
mod bar_list;
mod blockquote;
mod breadcrumb;
mod bulleted_list;
mod button;
mod calendar;
mod calendar_panel_keys;
mod card;
mod carousel;
mod cascader;
#[cfg(feature = "canvas")]
mod chart_motion;
mod checkbox;
mod choice_card;
mod choice_details;
mod code_block;
mod code_tokenizer;
mod collapsible;
mod color_field;
mod color_picker;
mod color_swatch;
mod column_view;
mod combobox;
mod command_bar;
mod command_palette;
mod context_menu;
mod data_grid;
mod data_grid_bar;
mod data_grid_editing;
mod data_grid_scale;
mod data_grid_tree;
mod data_list;
mod date_field;
mod date_picker;
mod date_range_picker;
mod dialog;
mod dialog_field_details;
mod dock_layout;
mod dock_state;
mod document_tabs;
mod drawer;
mod drop_zone;
mod editable;
mod empty_state;
mod event_calendar;
mod expandable_tile;
mod family;
mod field_mount;
mod fieldset;
mod file_upload;
mod find_bar;
mod floating_panel;
mod fluid_field;
mod follows_room;
mod forced_on_one_item;
// The form controls of `components`, which need the engine's forms.
#[cfg(feature = "form")]
mod form_controls;
// Pictures decoded in the test: the feature that has `lurq::images`.
#[cfg(feature = "raster")]
mod gallery;
#[cfg(feature = "canvas")]
mod gauge;
#[cfg(feature = "canvas")]
mod heatmap;
mod highlight;
mod hover_card;
mod icon;
mod info_label;
mod input;
mod input_kinds;
mod input_otp;
mod kbd;
// Pictures decoded in the test: the feature that has `lurq::images`.
#[cfg(feature = "raster")]
mod lightbox;
#[cfg(feature = "canvas")]
mod line_chart;
mod link_preview;
mod list_item;
mod listbox;
mod locale;
mod media_captions;
#[cfg(feature = "media")]
mod media_events;
mod media_speed_menu;
mod media_time;
mod media_volume;
mod menu;
mod menu_content;
mod menu_item_detail;
mod menu_keys;
mod menubar;
mod message_box;
mod meter;
mod month_picker;
mod multi_select;
mod narrow_room;
mod navbar;
mod navigation_menu;
mod notification;
mod notification_center;
mod number_input;
mod outcome;
mod pagination;
mod part_foreground;
mod part_ids;
mod path_field;
mod persona;
mod picture;
#[cfg(feature = "canvas")]
mod pie_chart;
mod play_button;
mod pointer_only;
mod popconfirm;
mod popover;
mod progress;
mod progress_dialog;
mod property_grid;
mod qr_code;
#[cfg(feature = "canvas")]
mod radar_chart;
mod radio;
mod rating;
mod reopened_layers;
mod rows_and_forms;
#[cfg(feature = "markdown")]
mod rich_text_editor;
#[cfg(feature = "canvas")]
mod scatter_chart;
mod scroll_area;
mod seek_bar;
mod select;
mod selection_marquee;
mod separator;
mod settings_card;
mod sheet;
mod shortcut_recorder;
mod sidebar;
#[cfg(feature = "canvas")]
mod signature_pad;
mod skeleton;
mod slider;
mod sortable_list;
#[cfg(feature = "canvas")]
mod sparkline;
mod spinner;
mod splitter;
mod stage;
mod stat_card;
mod status_bar;
mod status_label;
mod steps;
mod styled;
mod swatch_picker;
mod switch;
mod table;
mod table_of_contents;
mod tabs;
mod tag;
mod teaching_tip;
mod text_editing;
mod textarea;
mod theming;
mod time_field;
mod time_picker;
mod timeline;
mod toast;
mod toggletip;
mod token_input;
mod toolbar;
mod tooltip;
mod tooltip_text;
mod transfer;
mod tray_popover;
mod tree_select;
mod tree_view;
mod video_player;
mod watermark;
mod waveform;
mod window_chrome;
mod window_tabs;
mod window_tabs_hosted;
mod window_tabs_list;
