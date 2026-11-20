//! The design system's components, and what a theme styles them with.
//!
//! A component here is built like any other in lurq, by a builder that goes
//! where an element goes: `Button::new("Save").variant(ButtonVariant::Primary)`.
//! What it looks like is the theme's to say, in three layers:
//!
//! - **Tokens**: the palette and the scales of `lurq::app::theme`.
//! - **Recipes** ([`Recipe`]): for each component, the style of each of its parts ([`PartStyle`]) in each case: always,
//!   for a variant, for a size, in a state. The design's recipes are the default; a theme lays rules over one, replaces
//!   one whole, or starts with none ([`ThemeDefinition::bare`](crate::app::theme::ThemeDefinition::bare)).
//! - **Renderers**: what builds a component's nodes from its recipe. A theme may give a component, or one part of it,
//!   another.
//!
//! A component being built takes the theme of the component being rendered
//! around it ([`current_theme`]), so it needs no context passed to it, and
//! what renders it renders again when the theme changes.

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
mod banner;
#[cfg(feature = "canvas")]
mod bar_chart;
mod bar_list;
mod blockquote;
mod breadcrumb;
pub(crate) mod bulleted_list;
mod button;
mod button_group;
mod calendar;
mod callout;
mod card;
mod carousel;
mod cascader;
#[cfg(feature = "canvas")]
mod chart;
mod checkbox;
mod chip;
mod choice_card;
mod code_block;
mod code_tokens;
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
mod data_list;
mod date_field;
mod date_picker;
mod date_range_picker;
mod dialog;
mod dock_layout;
mod document_tabs;
mod drawer;
mod drop_zone;
pub(crate) mod edit_menu;
mod editable;
mod empty_state;
pub(crate) mod engine_look;
mod event_calendar;
mod expandable_tile;
mod fab;
mod field;
mod fieldset;
mod file_upload;
mod find_bar;
mod floating_panel;
mod gallery;
#[cfg(feature = "canvas")]
mod gauge;
#[cfg(feature = "canvas")]
mod heatmap;
mod highlight;
mod hover_card;
mod icon;
pub mod icons;
mod info_label;
mod input;
mod input_group;
mod input_otp;
mod kbd;
mod lightbox;
#[cfg(feature = "canvas")]
mod line_chart;
mod link;
mod link_preview;
mod list_item;
mod listbox;
mod media_captions;
mod media_speed_menu;
mod media_events;
mod media_time;
mod media_volume;
mod mentions;
mod menu;
mod menubar;
mod message_box;
mod meter;
mod month_picker;
mod multi_select;
mod navbar;
mod navigation_menu;
mod node_edits;
mod notification;
mod notification_center;
mod number_input;
mod opening_rows;
mod outcome;
mod pagination;
mod part_ids;
mod part_style;
mod password_input;
mod path_field;
mod persona;
mod picture;
#[cfg(feature = "canvas")]
mod pie_chart;
mod play_button;
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
mod recipe;
#[cfg(feature = "markdown")]
mod rich_text_editor;
#[cfg(feature = "canvas")]
mod scatter_chart;
mod scroll_area;
mod search_field;
mod seek_bar;
mod segmented;
mod select;
mod select_trigger;
mod selection;
mod selection_marquee;
mod separator;
mod settings_card;
mod shared;
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
mod stat_card;
mod status_bar;
mod status_label;
mod steps;
mod swatch_picker;
mod switch;
mod table;
mod table_of_contents;
mod tabs;
mod tag;
mod teaching_tip;
mod text_box;
mod textarea;
pub(crate) mod theme;
mod time_field;
mod time_picker;
mod timeline;
mod tip;
mod toast;
mod toggle;
mod toggle_group;
mod toggletip;
mod token_input;
mod token_list;
mod toolbar;
mod tooltip;
mod transfer;
mod tray_popover;
mod tree_select;
mod tree_view;
mod video_player;
mod watermark;
mod waveform;
mod window_chrome;
mod window_tabs;
mod year_picker;

pub use accordion::{Accordion, AccordionItem, AccordionPart, AccordionRenderer, AccordionState, AccordionView};
pub use action_bar::{
  ActionBar, ActionBarAction, ActionBarPart, ActionBarRenderer, ActionBarSide, ActionBarState, ActionBarView,
};
pub use action_sheet::{ActionSheet, ActionSheetPart, ActionSheetRenderer, ActionSheetState, ActionSheetView};
pub use ai_label::{AiLabel, AiLabelPart, AiLabelRenderer, AiLabelSize, AiLabelState, AiLabelView};
pub use alert::{Alert, AlertPart, AlertRenderer, AlertState, AlertVariant, AlertView};
pub use angle_slider::{
  AngleSlider, AngleSliderPart, AngleSliderRenderer, AngleSliderState, AngleSliderView, AngleText,
};
pub use audio_player::{
  AUDIO_MINI_NARROW, AUDIO_MINI_TIME_FROM, AUDIO_MINI_VOLUME_FROM, AudioControls, AudioPlayer, AudioPlayerPart,
  AudioPlayerRenderer, AudioPlayerState, AudioPlayerVariant, AudioPlayerView, AudioRepeat, VOICE_RATES,
  audio_rate_text,
};
pub use audio_queue::{
  AudioQueue, AudioQueueItem, AudioQueuePart, AudioQueueRenderer, AudioQueueState, AudioQueueView,
};
pub use avatar::{
  Avatar, AvatarGroup, AvatarGroupPart, AvatarGroupRenderer, AvatarGroupSize, AvatarGroupView, AvatarPart,
  AvatarPresence, AvatarRenderer, AvatarShape, AvatarSize, AvatarState, AvatarView, Presence,
};
pub use back_to_top::{
  BACK_TO_TOP_THRESHOLD, BackToTop, BackToTopPart, BackToTopRenderer, BackToTopState, BackToTopView,
};
pub use badge::{Badge, BadgePart, BadgeRenderer, BadgeShape, BadgeSize, BadgeState, BadgeVariant, BadgeView};
pub use banner::{Banner, BannerPart, BannerRenderer, BannerState, BannerView};
#[cfg(feature = "canvas")]
pub use bar_chart::{BarChart, BarChartPart, BarChartState};
pub use bar_list::{BarList, BarListItem, BarListPart, BarListRenderer, BarListRow, BarListState, BarListView};
pub use blockquote::{Blockquote, BlockquotePart, BlockquoteRenderer, BlockquoteState, BlockquoteView};
pub use breadcrumb::{
  Breadcrumb, BreadcrumbPart, BreadcrumbPartRenderer, BreadcrumbRenderer, BreadcrumbSeparator, BreadcrumbState,
  BreadcrumbStep, BreadcrumbVariant, BreadcrumbView,
};
pub use bulleted_list::{
  BulletedList, BulletedListItem, BulletedListMarker, BulletedListPart, BulletedListRenderer, BulletedListState,
  BulletedListView,
};
pub use button::{
  Button, ButtonPart, ButtonPartRenderer, ButtonRenderer, ButtonShape, ButtonSize, ButtonState, ButtonVariant,
  ButtonView,
};
pub use button_group::{
  ButtonGroup, ButtonGroupPart, ButtonGroupRenderer, ButtonGroupSize, ButtonGroupState, ButtonGroupVariant,
};
pub use calendar::{Calendar, CalendarChoice, CalendarPart, CalendarRenderer, CalendarState, CalendarView, DateMemory};
pub use callout::{Callout, CalloutPart, CalloutRenderer, CalloutState, CalloutView};
pub use card::{Card, CardPart, CardRenderer, CardSize, CardState, CardVariant, CardView};
pub use carousel::{Carousel, CarouselPart, CarouselRenderer, CarouselState, CarouselView};
pub use cascader::{Cascader, CascaderPart, CascaderRenderer, CascaderSize, CascaderState, CascaderView};
#[cfg(feature = "canvas")]
pub use chart::ChartLabel;
#[cfg(feature = "canvas")]
pub use chart::{
  BandScale, BarEnd, ChartArea, ChartAxis, ChartFrame, ChartLegend, ChartLegendItem, ChartLegendPart,
  ChartLegendRenderer, ChartLegendShape, ChartLegendState, ChartLegendView, ChartLook, ChartMotion, ChartPart,
  ChartPlot, ChartPoint, ChartProps, ChartRenderer, ChartSeries, ChartSize, ChartState, ChartTick, ChartTip,
  ChartTooltip, ChartTooltipPart, ChartTooltipRenderer, ChartTooltipRow, ChartTooltipView, ChartView, LinearScale,
  TipPlace, chart_motion, chart_recipe, default_value_text, nearest, nearest_sorted, series_color, snap as chart_snap,
  step as chart_step,
};
pub use checkbox::{
  Checkbox, CheckboxGroup, CheckboxGroupPart, CheckboxGroupRenderer, CheckboxGroupState, CheckboxGroupView,
  CheckboxPart, CheckboxRenderer, CheckboxState, CheckboxView,
};
pub use chip::{
  Chip, ChipGroup, ChipGroupPart, ChipGroupRenderer, ChipGroupState, ChipPart, ChipRenderer, ChipState, ChipView,
};
pub use choice_card::{
  CheckboxCard, CheckboxCardGroup, ChoiceCardGroupPart, ChoiceCardGroupRenderer, ChoiceCardGroupState,
  ChoiceCardGroupView, ChoiceCardKind, ChoiceCardPart, ChoiceCardRenderer, ChoiceCardState, ChoiceCardView, RadioCard,
  RadioCardGroup,
};
pub use code_block::{
  CODE_BLOCK_COPIED_FOR, CODE_BLOCK_ROW_HEIGHT, CODE_BLOCK_WINDOWED_LINES, CodeBlock, CodeBlockPart,
  CodeBlockRenderer, CodeBlockState, CodeBlockView,
};
pub use collapsible::{Collapsible, CollapsiblePart, CollapsibleRenderer, CollapsibleState, CollapsibleView};
pub use color_field::{
  ColorField, ColorFieldPart, ColorFieldRenderer, ColorFieldSize, ColorFieldState, ColorFieldView, FieldColor,
};
pub use color_picker::{
  ColorFormat, ColorPicker, ColorPickerPart, ColorPickerRenderer, ColorPickerState, ColorPickerView,
};
pub use color_swatch::{
  ColorSwatch, ColorSwatchPart, ColorSwatchRenderer, ColorSwatchShape, ColorSwatchSize, ColorSwatchState,
  ColorSwatchView,
};
pub use column_view::{
  ColumnView, ColumnViewColumn, ColumnViewItem, ColumnViewPart, ColumnViewRenderer, ColumnViewState, ColumnViewView,
};
pub use combobox::{Combobox, ComboboxPart, ComboboxRenderer, ComboboxSize, ComboboxState, ComboboxView};
pub use command_bar::{
  CommandBar, CommandBarCommand, CommandBarCommandPart, CommandBarCommandRenderer, CommandBarCommandState,
  CommandBarCommandView, CommandBarPart, CommandBarRenderer, CommandBarState, CommandBarTab, CommandBarView,
};
pub use command_palette::{
  CommandFilter, CommandItem, CommandPalette, CommandPalettePart, CommandPaletteRenderer, CommandPaletteState,
  CommandPaletteView, matches_every_word,
};
pub use context_menu::{ContextMenu, ContextMenuPart, ContextMenuRenderer, ContextMenuState, ContextMenuView};
pub use data_grid::DataGrid;
pub use data_list::{
  DataList, DataListItem, DataListOrientation, DataListPart, DataListRenderer, DataListState, DataListView,
};
pub use date_field::{
  DateField, DateFieldPart, DateFieldRenderer, DateFieldSize, DateFieldState, DateFieldView, FieldValue,
  SegmentFieldView, SegmentShown,
};
pub use date_picker::{
  DatePicker, DatePickerPart, DatePickerRenderer, DatePickerSize, DatePickerState, DatePickerView, Picked,
  PickerRenderer, PickerView,
};
pub use date_range_picker::{DateRangePicker, DateRangePickerRenderer, DateRangePickerView};
pub use dialog::{Dialog, DialogPart, DialogRenderer, DialogSize, DialogState, DialogView};
pub use dock_layout::{
  DockCloseEvent, DockDocuments, DockFloat, DockGroup, DockHidden, DockLayout, DockLayoutPart, DockLayoutRenderer,
  DockLayoutState, DockLayoutView, DockNode, DockPanel, DockPlace, DockSide, DockSplit, DockState, DockStateParseError,
};
pub use document_tabs::{
  DocumentTab, DocumentTabEntry, DocumentTabs, DocumentTabsPart, DocumentTabsRenderer, DocumentTabsState,
  DocumentTabsView,
};
pub use drawer::{Drawer, DrawerPart, DrawerRenderer, DrawerState, DrawerView};
pub use drop_zone::{
  DropZone, DropZonePart, DropZoneRenderer, DropZoneState, DropZoneView, RejectReason, RejectedFile,
};
pub use editable::{Editable, EditablePart, EditableRenderer, EditableState, EditableView};
pub use empty_state::{
  EmptyState, EmptyStatePart, EmptyStateRenderer, EmptyStateSize, EmptyStateState, EmptyStateView,
};
pub use event_calendar::{
  CalendarEvent, CalendarEventVariant, EventCalendar, EventCalendarForm, EventCalendarPart, EventCalendarPeriod,
  EventCalendarRenderer, EventCalendarState, EventCalendarView, ShownEvent,
};
pub use expandable_tile::{
  ExpandableTile, ExpandableTilePart, ExpandableTileRenderer, ExpandableTileState, ExpandableTileView,
};
pub use fab::{Fab, FabPart, FabRenderer, FabState};
pub use field::{Field, FieldControl, FieldPart, FieldRenderer, FieldState, FieldView, InField};
pub use fieldset::{
  ErrorSummary, ErrorSummaryPart, ErrorSummaryRenderer, ErrorSummaryState, ErrorSummaryView, FieldError, Fieldset,
  FieldsetPart, FieldsetRenderer, FieldsetState, FieldsetVariant, FieldsetView,
};
pub use file_upload::{
  FileUpload, FileUploadPart, FileUploadRenderer, FileUploadState, FileUploadView, UploadFile, UploadState,
};
pub use find_bar::{FindBar, FindBarPart, FindBarRenderer, FindBarState, FindBarView};
pub use floating_panel::{
  FloatingPanel, FloatingPanelPart, FloatingPanelRenderer, FloatingPanelState, FloatingPanelView, FloatingPanels,
  PanelHeader, PanelHeaderPart, PanelHeaderRenderer, PanelHeaderState, PanelHeaderView, PanelPosition, PanelSize,
};
pub use gallery::{
  GALLERY_LEAST_PICTURE, Gallery, GalleryKey, GalleryPart, GalleryRenderer, GalleryState, GalleryView,
};
#[cfg(feature = "canvas")]
pub use gauge::{Gauge, GaugePart, GaugeRenderer, GaugeView};
#[cfg(feature = "canvas")]
pub use heatmap::{Heatmap, HeatmapPart};
#[cfg(feature = "markdown")]
pub(crate) use highlight::fill_of;
pub use highlight::{FindError, FindOptions, Highlight, HighlightPart, HighlightRenderer, HighlightView, find_matches};
pub use hover_card::{HoverCard, HoverCardPart, HoverCardRenderer, HoverCardState, HoverCardView};
pub use icon::{Icon, SPIN, SPIN_PERIOD, SpinMotion};
pub use info_label::{InfoLabel, InfoLabelPart, InfoLabelRenderer, InfoLabelState, InfoLabelView};
pub use input::{Input, InputPart, InputRenderer, InputSize, InputState, InputView};
pub use input_group::{
  InputAddon, InputGroup, InputGroupPart, InputGroupRenderer, InputGroupSize, InputGroupState, InputGroupView,
};
pub(crate) use input_otp::keyframes as input_otp_keyframes;
pub use input_otp::{
  INPUT_OTP_CARET_BLINK, INPUT_OTP_CARET_PERIOD, InputOtp, InputOtpKind, InputOtpPart, InputOtpRenderer, InputOtpSize,
  InputOtpState, InputOtpView,
};
pub use kbd::{
  InlineCode, InlineCodePart, InlineCodeRenderer, InlineCodeView, Kbd, KbdKey, KbdPart, KbdRenderer, KbdSize, KbdState,
  KbdVariant, KbdView, KeyPlatform,
};
pub use lightbox::{Lightbox, LightboxPart, LightboxRenderer, LightboxState, LightboxVariant, LightboxView};
#[cfg(feature = "canvas")]
pub use line_chart::{AreaChart, LineChart, LineChartPart, LineChartState, LineCurve, line_values_looked_at};
pub use link::{Link, LinkPart, LinkRenderer, LinkState, LinkView};
pub use link_preview::{
  LINK_PREVIEW_PICTURE_RATIO, LinkImage, LinkPreview, LinkPreviewPart, LinkPreviewRenderer, LinkPreviewState,
  LinkPreviewVariant, LinkPreviewView,
};
pub use list_item::{
  List, ListItem, ListItemPart, ListItemRenderer, ListItemState, ListItemView, ListPart, ListRenderer, ListState,
  ListView,
};
pub use listbox::{Listbox, ListboxOption, ListboxPart, ListboxRenderer, ListboxState, ListboxView};
pub use media_captions::{MediaCaptions, MediaCue};
pub use media_speed_menu::{
  MediaSpeedMenu, MediaSpeedMenuPart, MediaSpeedMenuRenderer, MediaSpeedMenuState, MediaSpeedMenuView,
};
pub use media_time::{
  MEDIA_RATES, MediaTime, MediaTimePart, MediaTimeRenderer, MediaTimeSize, MediaTimeState, MediaTimeView,
  media_rate_text, media_time_text,
};
pub use media_volume::{
  MediaVolume, MediaVolumePart, MediaVolumeRenderer, MediaVolumeSize, MediaVolumeState, MediaVolumeView,
};
pub use mentions::{
  MENTIONS_LIMIT, Mention, MentionSource, Mentions, MentionsController, MentionsPart, MentionsRenderer, MentionsSize,
  MentionsState, MentionsView,
};
pub use menu::{
  Menu, MenuEntry, MenuItem, MenuItemPart, MenuItemRenderer, MenuItemState, MenuItemView, MenuPart, MenuRenderer,
  MenuState, MenuView,
};
pub use menubar::{Menubar, MenubarMenu, MenubarPart, MenubarRenderer, MenubarState, MenubarView};
pub use message_box::{
  MessageBox, MessageBoxAnswer, MessageBoxPart, MessageBoxRenderer, MessageBoxRole, MessageBoxState, MessageBoxVariant,
  MessageBoxView,
};
pub use meter::{Meter, MeterPart, MeterRenderer, MeterSegment, MeterState, MeterVariant, MeterView};
pub use month_picker::{
  CellShown, MonthPicker, MonthPickerPart, MonthPickerRenderer, MonthPickerState, MonthPickerView, PeriodsView,
};
pub use multi_select::{
  MultiSelect, MultiSelectPart, MultiSelectRenderer, MultiSelectSize, MultiSelectState, MultiSelectView,
};
pub use navbar::{Navbar, NavbarBrand, NavbarPart, NavbarRenderer, NavbarState, NavbarView};
pub use navigation_menu::{
  NavItem, NavLink, NavPanel, NavigationMenu, NavigationMenuPart, NavigationMenuRenderer, NavigationMenuState,
  NavigationMenuVariant, NavigationMenuView,
};
pub use notification::{
  Notification, NotificationPart, NotificationRenderer, NotificationState, NotificationVariant, NotificationView,
};
pub use notification_center::{
  NotificationCenter, NotificationCenterPart, NotificationCenterRenderer, NotificationCenterState,
  NotificationCenterView,
};
pub use number_input::{
  NumberFormat, NumberInput, NumberInputLayout, NumberInputPart, NumberInputRenderer, NumberInputSize,
  NumberInputState, NumberInputView,
};
pub use outcome::{Outcome, OutcomePart, OutcomeRenderer, OutcomeState, OutcomeVariant, OutcomeView};
pub use pagination::{PageItem, Pagination, PaginationPart, PaginationRenderer, PaginationState, PaginationView};
pub use part_style::{
  FontFamily, Length, Opacity, PartAnimation, PartStyle, PartTransition, TextPatch, TransitionDuration,
  TransitionEasing,
};
pub use password_input::{
  PasswordInput, PasswordInputPart, PasswordInputRenderer, PasswordInputSize, PasswordInputState, PasswordInputView,
  PasswordStrength,
};
pub use path_field::{
  PathField, PathFieldPart, PathFieldRenderer, PathFieldSize, PathFieldState, PathFieldView, PathKind,
};
pub use persona::{Persona, PersonaPart, PersonaRenderer, PersonaSize, PersonaState, PersonaView};
pub use crate::core::PictureError;
pub use picture::{
  PICTURE_LARGE_FROM, PICTURE_SMALL_UP_TO, PICTURE_SMALLEST_UP_TO, Picture, PictureBar, PictureFit, PictureForm,
  PictureLoad, PicturePart, PicturePlaceholder, PictureRenderer, PictureShape, PictureSize, PictureSmall, PictureState,
  PictureVariant, PictureView,
};
#[cfg(feature = "canvas")]
pub use pie_chart::{PieChart, PieChartPart, PieChartState, PieLegendValue, ShareTextFn, default_share_text};
pub use play_button::{
  PlayButton, PlayButtonPart, PlayButtonRenderer, PlayButtonSize, PlayButtonState, PlayButtonVariant, PlayButtonView,
};
pub use popconfirm::{Popconfirm, PopconfirmPart, PopconfirmRenderer, PopconfirmState, PopconfirmView};
pub use popover::{Popover, PopoverPart, PopoverRenderer, PopoverView};
pub(crate) use progress::keyframes as progress_keyframes;
pub use progress::{
  PROGRESS_LEAD, PROGRESS_PERIOD, PROGRESS_SEGMENT, Progress, ProgressFormat, ProgressPart, ProgressRenderer,
  ProgressSize, ProgressState, ProgressVariant, ProgressView, RingProgress, RingProgressPart, RingProgressRenderer,
  RingProgressView,
};
pub use progress_dialog::{
  ProgressDialog, ProgressDialogPart, ProgressDialogRenderer, ProgressDialogState, ProgressDialogView,
};
pub use property_grid::{
  CategoryView, EditorContext, PROPERTY_NAME_WIDTH, Property, PropertyCategory, PropertyGrid, PropertyGridPart,
  PropertyGridRenderer, PropertyGridState, PropertyGridView, PropertyRow, PropertyView,
};
pub use qr_code::{QrCode, QrCodePart, QrCodeRenderer, QrCodeState, QrCodeView, QrLevel, QrMatrix, QrRect};
#[cfg(feature = "canvas")]
pub use radar_chart::{RadarChart, RadarChartPart};
pub use radio::{
  Radio, RadioGroup, RadioGroupPart, RadioGroupRenderer, RadioGroupState, RadioGroupView, RadioPart, RadioRenderer,
  RadioSelection, RadioState, RadioView,
};
pub use rating::{
  Rating, RatingPart, RatingPrecision, RatingRenderer, RatingSize, RatingState, RatingValue, RatingValueText,
  RatingView,
};
pub use recipe::{
  Case, Interaction, Name, PartStyles, ROW_STEPS, Recipe, ResolvedPart, ResolvedRecipe, Rule, Selector,
};
#[cfg(feature = "markdown")]
pub use rich_text_editor::{
  RICH_TEXT_MAX_ROWS, RICH_TEXT_MIN_ROWS, RichTextController, RichTextEditor, RichTextEditorPart,
  RichTextEditorRenderer, RichTextEditorState, RichTextEditorView, RichTextFormat, RichTextTool,
};
#[cfg(feature = "canvas")]
pub use scatter_chart::{ScatterChart, ScatterChartPart, ScatterChartState};
pub use scroll_area::{
  ScrollArea, ScrollAreaOrientation, ScrollAreaPart, ScrollAreaRenderer, ScrollAreaScrollbars, ScrollAreaState,
  ScrollAreaView,
};
pub use search_field::{
  SearchField, SearchFieldPart, SearchFieldRenderer, SearchFieldSize, SearchFieldState, SearchFieldView,
};
pub use seek_bar::{SeekBar, SeekBarPart, SeekBarRenderer, SeekBarSize, SeekBarState, SeekBarView, SeekPreview};
pub use segmented::{
  SegmentedControl, SegmentedControlPart, SegmentedControlRenderer, SegmentedControlSize, SegmentedControlState,
};
pub use select::{AnySelect, Select, SelectPart, SelectRenderer, SelectSize, SelectState, SelectView};
pub use selection_marquee::{
  Marquee, MarqueeMode, SelectionMarquee, SelectionMarqueePart, SelectionMarqueeRenderer, SelectionMarqueeState,
  SelectionMarqueeView,
};
pub use separator::{Separator, SeparatorOrientation, SeparatorPart, SeparatorRenderer, SeparatorState, SeparatorView};
pub use settings_card::{SettingsCard, SettingsCardPart, SettingsCardRenderer, SettingsCardState, SettingsCardView};
pub use shared::{Choice, FADE_IN, FADE_OUT, IconChoice, Orientation};
pub(crate) use sheet::keyframes as sheet_keyframes;
pub use sheet::{
  SHEET_IN_FROM_BOTTOM, SHEET_IN_FROM_END, SHEET_IN_FROM_START, SHEET_IN_FROM_TOP, SHEET_OUT_TO_BOTTOM,
  SHEET_OUT_TO_END, SHEET_OUT_TO_START, SHEET_OUT_TO_TOP, Sheet, SheetPart, SheetRenderer, SheetSide, SheetSize,
  SheetState, SheetView,
};
pub use shortcut_recorder::{
  ShortcutCheck, ShortcutRecorder, ShortcutRecorderPart, ShortcutRecorderRenderer, ShortcutRecorderSize,
  ShortcutRecorderState, ShortcutRecorderView, ShortcutValue,
};
pub use sidebar::{
  Sidebar, SidebarGroup, SidebarHeading, SidebarHeadingVariant, SidebarItem, SidebarPart, SidebarRenderer,
  SidebarState, SidebarTrigger, SidebarView,
};
#[cfg(feature = "canvas")]
pub use signature_pad::{
  Signature, SignaturePad, SignaturePadPart, SignaturePadRenderer, SignaturePadState, SignaturePadView, SignaturePoint,
  SignatureStroke,
};
pub(crate) use skeleton::keyframes as skeleton_keyframes;
pub use skeleton::{
  SKELETON_PULSE, SKELETON_PULSE_PERIOD, SKELETON_SHIMMER, SKELETON_SHIMMER_PERIOD, Skeleton, SkeletonAnimation,
  SkeletonPart, SkeletonRenderer, SkeletonShape, SkeletonView,
};
pub use slider::{
  RangeSlider, Slider, SliderNumber, SliderOrientation, SliderPart, SliderRenderer, SliderState, SliderView,
};
pub use sortable_list::{
  SORTABLE_LIST_TILT, SortableItem, SortableList, SortableListPart, SortableListRenderer, SortableListState,
  SortableListView, SortableRow, move_rows,
};
#[cfg(feature = "canvas")]
pub use sparkline::{Sparkline, SparklinePart, SparklineRenderer, SparklineView};
pub(crate) use spinner::keyframes as spinner_keyframes;
pub use spinner::{
  LoadingOverlay, LoadingOverlayPart, LoadingOverlayRenderer, LoadingOverlayState, LoadingOverlayView, PageLoader,
  PageLoaderPart, PageLoaderRenderer, PageLoaderState, PageLoaderView, SPINNER_BARS, SPINNER_BARS_PERIOD, SPINNER_DOTS,
  SPINNER_DOTS_PERIOD, Spinner, SpinnerKind, SpinnerPart, SpinnerRenderer, SpinnerSize, SpinnerState, SpinnerVariant,
  SpinnerView,
};
pub use splitter::{
  Splitter, SplitterOrientation, SplitterPane, SplitterPart, SplitterRenderer, SplitterState, SplitterView,
};
pub use stat_card::{
  StatCard, StatCardPart, StatCardRenderer, StatCardSize, StatCardState, StatCardVariant, StatCardView,
};
pub use status_bar::{
  StatusBar, StatusBarPart, StatusBarRenderer, StatusBarState, StatusBarVariant, StatusBarView, StatusEntry,
  StatusItem, StatusItemVariant, StatusRegion,
};
pub use status_label::{
  StatusLabel, StatusLabelPart, StatusLabelRenderer, StatusLabelSize, StatusLabelState, StatusLabelVariant,
  StatusLabelView,
};
pub use steps::{Step, StepView, Steps, StepsOrientation, StepsPart, StepsRenderer, StepsState, StepsView};
pub use swatch_picker::{
  Swatch, SwatchItem, SwatchPicker, SwatchPickerPart, SwatchPickerRenderer, SwatchPickerState, SwatchPickerView,
};
pub use switch::{Switch, SwitchPart, SwitchRenderer, SwitchState, SwitchView};
pub use table::{
  BodyWindow, CellEdit, CellEditor, CellKey, CellRange, CellValue, ColumnAlign, ColumnFilter, ColumnLayout,
  ColumnMenuLabels, ColumnPin, ColumnPlace, ExportFormat, ExportLabels, ExportScope, FilterLabels, Geometry,
  GridBarLabels, GridCell, GridExport, GridPage, GridQuery, GridRequest, GroupKey, SearchMatches, SortDirection,
  SourceLabels, SourceRow, SummaryKind, SummaryLabels, Table, TableCell, TableColumn, TableCount, TableFilters,
  TableGroup, TableHeader, TablePart, TableRenderer, TableRow, TableSort, TableState, TableSummary, TableTextError,
  TableView, TextMatch, in_tracks, to_csv, to_delimited, to_tsv,
};
pub use table_of_contents::{
  TableOfContents, TableOfContentsEntry, TableOfContentsPart, TableOfContentsRenderer, TableOfContentsState,
  TableOfContentsView,
};
pub use tabs::{Tab, TabEntry, Tabs, TabsOrientation, TabsPart, TabsRenderer, TabsState, TabsVariant, TabsView};
pub use tag::{Tag, TagPart, TagRenderer, TagShape, TagSize, TagState, TagVariant, TagView};
pub use teaching_tip::{
  TeachingTip, TeachingTipPart, TeachingTipRenderer, TeachingTipState, TeachingTipVariant, TeachingTipView, Tour,
  TourStep,
};
pub use text_box::{Judgment, TextBoxSize, TextBoxState};
pub use textarea::{
  TEXTAREA_MAX_ROWS, TEXTAREA_MIN_ROWS, Textarea, TextareaPart, TextareaRenderer, TextareaSize, TextareaState,
  TextareaView,
};
pub use theme::{
  Declared, Styled, Themed, UiDefinition, current_breakpoint, current_date_locale, current_locale, current_pointer,
  current_theme, in_theme, touch_form,
};
pub use time_field::{TimeField, TimeFieldRenderer, TimeFieldView};
pub use time_picker::{
  ColumnKind, TimePicker, TimePickerPart, TimePickerRenderer, TimePickerSize, TimePickerState, TimePickerView,
};
pub use timeline::{
  Timeline, TimelineAlign, TimelineItem, TimelineItemView, TimelineMore, TimelinePart, TimelineRenderer, TimelineSize,
  TimelineState, TimelineVariant, TimelineView,
};
pub(crate) use toast::keyframes as toast_keyframes;
pub use toast::{
  TOAST_DURATION, TOASTS_SHOWN, Toast, ToastAction, ToastId, ToastPart, ToastPosition, ToastRenderer, ToastState,
  ToastVariant, ToastView, Toasts,
};
pub use toggle::{Toggle, TogglePart, ToggleRenderer, ToggleSize, ToggleState, ToggleView};
pub use toggle_group::{
  ToggleGroup, ToggleGroupOrientation, ToggleGroupPart, ToggleGroupRenderer, ToggleGroupSize, ToggleGroupState,
};
pub use toggletip::{Toggletip, ToggletipPart, ToggletipRenderer, ToggletipState, ToggletipView};
pub use token_input::{
  MENTION, Token, TokenInput, TokenInputPart, TokenInputRenderer, TokenInputSize, TokenInputState, TokenInputView,
};
pub use toolbar::{
  Toolbar, ToolbarCommand, ToolbarGroup, ToolbarItem, ToolbarPart, ToolbarRenderer, ToolbarSize, ToolbarSlot,
  ToolbarState, ToolbarVariant, ToolbarView,
};
pub use tooltip::{
  TOOLTIP_DELAY, TOOLTIP_SKIP_DELAY, Tooltip, TooltipPart, TooltipRenderer, TooltipSide, TooltipState, TooltipView,
};
pub use transfer::{Transfer, TransferList, TransferPart, TransferRenderer, TransferState, TransferView};
pub use tray_popover::{
  TrayAction, TrayItem, TrayPopover, TrayPopoverPart, TrayPopoverRenderer, TrayPopoverState, TrayPopoverView,
  TrayStatusVariant, TrayToggle,
};
pub use tree_select::{
  TreeSelect, TreeSelectPart, TreeSelectRenderer, TreeSelectSize, TreeSelectState, TreeSelectView,
};
pub use tree_view::{
  ManySelected, OneSelected, RowSelection, TreeItem, TreeRow, TreeView, TreeViewPart, TreeViewRenderer, TreeViewState,
  TreeViewView,
};
pub use video_player::{
  VIDEO_PLAYER_COMPACT_BELOW, VIDEO_PLAYER_HIDE_AFTER, VIDEO_PLAYER_MINIMAL_BELOW, VIDEO_PLAYER_SEEK_STEP,
  VIDEO_PLAYER_VOLUME_FOLDS_BELOW, VIDEO_PLAYER_VOLUME_STEP, VideoFullscreen, VideoPlayer, VideoPlayerBar,
  VideoPlayerMenu, VideoPlayerPart, VideoPlayerRenderer, VideoPlayerSize, VideoPlayerState, VideoPlayerTouch,
  VideoPlayerView,
};
pub use watermark::{
  WATERMARK_ANGLE, WATERMARK_GAP, WATERMARK_OFFSET, WATERMARK_STAGGER, Watermark, WatermarkPart, WatermarkRenderer,
  WatermarkState, WatermarkView,
};
pub use waveform::{Waveform, WaveformPart, WaveformRenderer, WaveformState, WaveformView};
pub use window_chrome::{
  WindowChrome, WindowChromeIcon, WindowChromePart, WindowChromeRenderer, WindowChromeState, WindowChromeView,
  WindowControl, WindowControls, WindowPlatform,
};
pub use window_tabs::{
  WindowTab, WindowTabEntry, WindowTabs, WindowTabsPart, WindowTabsRenderer, WindowTabsState, WindowTabsView,
};
pub use year_picker::{YearPicker, YearPickerRenderer, YearPickerView};

/// Gives an enum of names its names: `as_str`, and a conversion into
/// [`Name`], so that a variant, a size, a state or a part of a component is
/// accepted where a name is.
macro_rules! names {
  ($type:ident { $($variant:ident => $name:literal),* $(,)? }) => {
    impl $type {
      pub fn as_str(self) -> &'static str {
        match self {
          $( Self::$variant => $name, )*
        }
      }

      /// Every one of them, in order.
      pub fn all() -> &'static [$type] {
        &[ $( Self::$variant, )* ]
      }

      /// Every one of them as a name and as the Rust that writes it: what a component gives as its
      /// [`Themed::parts`]($crate::ui::Themed::parts) or [`states`]($crate::ui::Themed::states).
      pub fn declared() -> $crate::ui::Declared {
        $crate::ui::Declared::of(
          stringify!($type),
          vec![ $( ($crate::ui::Name::new($name), stringify!($variant)), )* ],
        )
      }
    }

    impl From<$type> for $crate::ui::Name {
      fn from(name: $type) -> Self {
        $crate::ui::Name::new(name.as_str())
      }
    }

    // A name of the design system goes in a signal with DevTools on, as any value an app keeps does: a
    // playground's variant, a setting's size.
    impl $crate::app::component::DevtoolsInspectable for $type {
      fn inspect(&self, formatter: &mut $crate::app::component::DevtoolsFormatter<'_>) {
        formatter.value_debug(&self.as_str());
      }
    }
  };
}
pub(crate) use names;

/// Values of the design system that are no name of a recipe go in a signal with DevTools on too, shown as they are
/// debugged: a shape, an orientation, a position.
macro_rules! inspectable {
  ($($type:ty),* $(,)?) => {
    $(
      impl $crate::app::component::DevtoolsInspectable for $type {
        fn inspect(&self, formatter: &mut $crate::app::component::DevtoolsFormatter<'_>) {
          formatter.value_debug(self);
        }
      }
    )*
  };
}

inspectable!(
  avatar::AvatarShape,
  badge::BadgeShape,
  breadcrumb::BreadcrumbSeparator,
  button::ButtonShape,
  calendar::CalendarChoice,
  event_calendar::EventCalendarForm,
  choice_card::ChoiceCardKind,
  color_picker::ColorFormat,
  color_swatch::ColorSwatchShape,
  input_otp::InputOtpKind,
  kbd::KeyPlatform,
  message_box::MessageBoxRole,
  pagination::PageItem,
  picture::PictureFit,
  picture::PictureShape,
  selection_marquee::Marquee,
  part_style::Length,
  part_style::Opacity,
  part_style::FontFamily,
  part_style::TransitionDuration,
  part_style::TransitionEasing,
  recipe::Interaction,
  skeleton::SkeletonAnimation,
  spinner::SpinnerKind,
  table::SortDirection,
  table::ColumnAlign,
  toast::ToastPosition,
);
