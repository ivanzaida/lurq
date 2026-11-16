//! The pages of the gallery, in the groups and the order of the design
//! system. A page is here when what it shows exists: a component gets its
//! page when it lands.

use lurq::{app::ctx::Ctx, node::Element};

mod accordion;
mod action_bar;
mod ai_label;
mod alert;
mod area_chart;
mod audio_player;
mod audio_queue;
mod avatar;
mod back_to_top;
mod badge;
mod bar_list;
mod gauge;
mod heatmap;
mod meter;
mod sparkline;
mod stat_card;
mod banner;
mod bar_chart;
mod blockquote;
mod bulleted_list;
mod breadcrumb;
mod button;
mod calendar;
mod callout;
mod card;
mod checkbox;
mod checkbox_card;
mod code_block;
mod color_field;
mod color_picker;
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
mod empty_state;
mod event_calendar;
mod expandable_tile;
mod family;
mod family_play;
mod fieldset;
mod file_upload;
mod find_bar;
mod floating_panel;
mod hover_card;
mod icons;
mod info_label;
mod input;
mod input_group;
mod input_otp;
mod kbd;
mod lightbox;
mod line_chart;
mod list;
mod listbox;
mod markdown;
mod media_time;
mod mentions;
mod media_speed_menu;
mod media_volume;
mod menu;
mod menubar;
mod message_box;
mod month_picker;
mod multi_select;
mod navbar;
mod navigation_menu;
mod number_input;
mod outcome;
mod overview;
mod pagination;
mod password_input;
mod picture;
mod persona;
mod pie_chart;
mod play_button;
mod popconfirm;
mod popover;
mod progress;
mod progress_dialog;
mod property_grid;
mod radar_chart;
mod radio;
mod radio_card;
mod rating;
mod scatter_chart;
mod scroll_area;
mod search_field;
mod seek_bar;
mod select;
mod selection_marquee;
mod separator;
mod sheet;
mod shortcut_recorder;
mod sidebar;
mod skeleton;
mod slider;
mod sortable_list;
mod spinner;
mod splitter;
mod status_bar;
mod steps;
mod swatch_picker;
mod switch;
mod table;
mod table_of_contents;
mod tabs;
mod tag;
mod teaching_tip;
mod textarea;
mod time_field;
mod time_picker;
mod timeline;
mod toast;
mod toggletip;
mod token_input;
mod tokens;
mod toolbar;
mod tooltip;
mod tree_select;
mod tree_view;
mod video_player;
mod waveform;
mod window_chrome;
mod window_tabs;
mod year_picker;

/// A group of the side bar.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Group {
  GettingStarted,
  Actions,
  Inputs,
  Navigation,
  Surfaces,
  Data,
  Charts,
  Media,
  Desktop,
}

impl Group {
  /// The groups in the order of the side bar.
  pub const ALL: [Group; 9] = [
    Self::GettingStarted,
    Self::Actions,
    Self::Inputs,
    Self::Navigation,
    Self::Surfaces,
    Self::Data,
    Self::Charts,
    Self::Media,
    Self::Desktop,
  ];

  pub fn name(self) -> &'static str {
    match self {
      Self::GettingStarted => "Getting started",
      Self::Actions => "Actions",
      Self::Inputs => "Inputs",
      Self::Navigation => "Navigation",
      Self::Surfaces => "Surfaces & overlays",
      Self::Data => "Data",
      Self::Charts => "Charts",
      Self::Media => "Media",
      Self::Desktop => "Desktop",
    }
  }
}

/// A page of the gallery.
pub struct Page {
  pub path: &'static str,
  pub name: &'static str,
  /// The icon before its name in the side bar.
  pub icon: Option<&'static str>,
  /// What the page is about, in lower case: the search finds it by these words too.
  pub about: &'static str,
  pub group: Group,
  pub mount: fn(&mut Ctx) -> Element,
}

/// Every page there is, in the order of the side bar within its group.
pub const PAGES: &[Page] = &[
  Page {
    path: "/",
    name: "Overview",
    icon: Some("home"),
    about: "start layers recipes renderers",
    group: Group::GettingStarted,
    mount: |ctx| ctx.mount::<overview::Overview>(()),
  },
  Page {
    path: "/tokens",
    name: "Tokens",
    icon: Some("palette"),
    about: "palette color typography font radius shadow spacing size density duration motion opacity blur focus ring",
    group: Group::GettingStarted,
    mount: |ctx| ctx.mount::<tokens::Tokens>(()),
  },
  Page {
    path: "/icons",
    name: "Icons",
    icon: Some("box"),
    about: "icon font glyph size",
    group: Group::GettingStarted,
    mount: |ctx| ctx.mount::<icons::Icons>(()),
  },
  Page {
    path: "/button",
    name: "Button",
    icon: Some("hand-pointer"),
    about: "variant size shape icon icon_only loading disabled full_width on_click pill primary secondary outline ghost destructive label trailing_icon description style rules force",
    group: Group::Actions,
    mount: |ctx| ctx.mount::<button::ButtonPage>(()),
  },
  Page {
    path: "/link",
    name: "Link",
    icon: Some("external-link"),
    about: "link text underline external navigate label on_change icon icon_only size disabled child children variant new multiple controlled choices full_width on_click",
    group: Group::Actions,
    mount: |ctx| ctx.mount::<family::LinkPage>(()),
  },
  Page {
    path: "/toggle",
    name: "Toggle",
    icon: Some("star"),
    about: "toggle on off pressed state bold label on_change icon icon_only size disabled child children variant new multiple controlled choices full_width on_click external",
    group: Group::Actions,
    mount: |ctx| ctx.mount::<family::TogglePage>(()),
  },
  Page {
    path: "/toggle-group",
    name: "Toggle Group",
    icon: Some("bold"),
    about: "toggle group single multiple choice alignment label on_change icon icon_only size disabled child children variant new controlled choices full_width on_click external",
    group: Group::Actions,
    mount: |ctx| ctx.mount::<family::ToggleGroupPage>(()),
  },
  Page {
    path: "/segmented-control",
    name: "Segmented Control",
    icon: Some("rows-2"),
    about: "segmented control segment track choice tabs label on_change icon icon_only size disabled child children variant new multiple controlled choices full_width on_click external",
    group: Group::Actions,
    mount: |ctx| ctx.mount::<family::SegmentedControlPage>(()),
  },
  Page {
    path: "/button-group",
    name: "Button Group",
    icon: Some("duplicate"),
    about: "button group split joined separator label on_change icon icon_only size disabled child children variant new multiple controlled choices full_width on_click external",
    group: Group::Actions,
    mount: |ctx| ctx.mount::<family::ButtonGroupPage>(()),
  },
  Page {
    path: "/input",
    name: "Input",
    icon: Some("text-cursor"),
    about: "input field text entry label description error hint required optional placeholder icon invalid read only form value size warning valid read_only disabled full_width on_input name control horizontal fluid fluid field label inside floating label",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<input::InputPage>(()),
  },
  Page {
    path: "/textarea",
    name: "Textarea",
    icon: Some("pencil"),
    about: "textarea multiline text rows placeholder invalid disabled scroll bio note value size warning valid read_only on_change label",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<textarea::TextareaPage>(()),
  },
  Page {
    path: "/checkbox",
    name: "Checkbox",
    icon: Some("check"),
    about: "checkbox check box tick indeterminate select all form option label checked bind on_change invalid label_hidden disabled group orientation",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<checkbox::CheckboxPage>(()),
  },
  Page {
    path: "/radio",
    name: "Radio",
    icon: Some("dot-circle"),
    about: "radio group choice option selected one of exclusive value choices controlled orientation horizontal vertical invalid disabled label on_change label_hidden",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<radio::RadioPage>(()),
  },
  Page {
    path: "/switch",
    name: "Switch",
    icon: Some("bolt"),
    about: "switch on off setting track thumb toggle dark mode label bind on_change invalid label_hidden disabled",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<switch::SwitchPage>(()),
  },
  Page {
    path: "/slider",
    name: "Slider",
    icon: Some("sliders"),
    about: "slider range value min max step track thumb volume drag orientation length label disabled",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<slider::SliderPage>(()),
  },
  Page {
    path: "/popover",
    name: "Popover",
    icon: Some("message"),
    about: "popover panel overlay popup anchor placement dismiss escape outside title description open trigger child width offset collision",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<popover::PopoverPage>(()),
  },
  Page {
    path: "/menu",
    name: "Menu",
    icon: Some("menu"),
    about: "menu dropdown item command shortcut separator label group checked destructive disabled popup anchor split enu new open placement inline tem icon radio submenu on_select",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<menu::MenuPage>(()),
  },
  Page {
    path: "/dialog",
    name: "Dialog",
    icon: Some("window"),
    about: "dialog modal veil scrim overlay confirm title description footer close escape scroll open child size width scrollable close_button dismiss_on_veil dismiss_on_escape inline",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<dialog::DialogPage>(()),
  },
  Page {
    path: "/select",
    name: "Select",
    icon: Some("chevrons-vertical"),
    about: "select dropdown choose option list menu placeholder chevron value time zone options size invalid disabled full_width warning valid label name",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<select::SelectPage>(()),
  },
  Page {
    path: "/checkbox-card",
    name: "Checkbox Card",
    icon: Some("check-circle"),
    about: "checkbox card choice tile option add-on plan price icon description checked bind group title on_change invalid disabled full_width cards controlled orientation horizontal",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<checkbox_card::CheckboxCardPage>(()),
  },
  Page {
    path: "/radio-card",
    name: "Radio Card",
    icon: Some("circle"),
    about: "radio card choice tile option plan price icon description selected group exclusive title on_change invalid disabled full_width cards controlled orientation horizontal",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<radio_card::RadioCardPage>(()),
  },
  Page {
    path: "/fieldset",
    name: "Fieldset",
    icon: Some("rows-4"),
    about: "fieldset legend group fields disabled form error summary errors alert attention link focus description field child children full_width ield rror new focuses",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<fieldset::FieldsetPage>(()),
  },
  Page {
    path: "/input-otp",
    name: "Input OTP",
    icon: Some("shield"),
    about: "input otp one time code verification pin two factor 2fa cells digits paste autofill mask value length groups kind uppercase on_complete size invalid warning valid read_only disabled label name",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<input_otp::InputOtpPage>(()),
  },
  Page {
    path: "/password-input",
    name: "Password Input",
    icon: Some("lock"),
    about: "password input masked secret reveal show hide eye strength meter weak strong new password value placeholder bind_reveal on_reveal strength_message new_password size invalid warning valid read_only disabled full_width",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<password_input::PasswordInputPage>(()),
  },
  Page {
    path: "/search-field",
    name: "Search Field",
    icon: Some("search"),
    about: "search field query filter find clear escape enter submit shortcut key hint searchbox value placeholder on_submit on_input size invalid warning valid read_only disabled full_width",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<search_field::SearchFieldPage>(()),
  },
  Page {
    path: "/input-group",
    name: "Input Group",
    icon: Some("link"),
    about: "input group addon prefix suffix before after https url domain unit button select joined size invalid warning valid disabled full_width",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<input_group::InputGroupPage>(()),
  },
  Page {
    path: "/number-input",
    name: "Number Input",
    icon: Some("plus"),
    about: "number input numeric stepper spin button quantity price min max step precision format prefix suffix value layout size placeholder invalid warning valid read_only disabled label on_change",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<number_input::NumberInputPage>(()),
  },
  Page {
    path: "/combobox",
    name: "Combobox",
    icon: Some("chevron-down"),
    about: "combobox autocomplete typeahead search filter select option list type custom value fetch loading no results options placeholder on_input allow_custom query size invalid warning valid disabled full_width label name",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<combobox::ComboboxPage>(()),
  },
  Page {
    path: "/multi-select",
    name: "Multi Select",
    icon: Some("list"),
    about: "multi select multiple choose several tags tokens chips checkbox list filter max shown clear value options placeholder max_shown clearable on_input size invalid warning valid disabled full_width label name",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<multi_select::MultiSelectPage>(()),
  },
  Page {
    path: "/token-input",
    name: "Token Input",
    icon: Some("users"),
    about: "token input tags chips tag list recipients variables template tokens remove backspace oken nput separators draft unique commit_on_blur on_change on_query query find_tokens dress_token new variant removable atomic placeholder rows size invalid warning valid read_only disabled full_width on_input controller label",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<token_input::TokenInputPage>(()),
  },
  Page {
    path: "/mentions",
    name: "Mentions",
    icon: Some("mail"),
    about: "mentions mention at people trigger suggestions autocomplete comment prompt slash command hashtag value controller ention on_mention tokens rows placeholder size invalid warning valid read_only disabled full_width label",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<mentions::MentionsPage>(()),
  },
  Page {
    path: "/info-label",
    name: "Info Label",
    icon: Some("info"),
    about: "info label field hint help tooltip mark explain explanation i icon more information placement arrow force_open nfo abel",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<info_label::InfoLabelPage>(()),
  },
  Page {
    path: "/calendar",
    name: "Calendar",
    icon: Some("calendar-days"),
    about: "calendar date day month week range picker grid today weekday locale first day week numbers value memory months min max disabled_days first_day week_numbers outside_days disabled on_change controlled label previous_label next_label force",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<calendar::CalendarPage>(()),
  },
  Page {
    path: "/date-picker",
    name: "Date Picker",
    icon: Some("calendar"),
    about: "date picker datepicker calendar day field typed parse format popover due date birthday value memory placeholder size locale min max disabled_days clearable week_numbers open month invalid warning valid read_only disabled label on_change controlled on_input on_submit on_open_change on_dismiss placement offset collision inline trigger_label clear_label panel_label previous_label next_label name",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<date_picker::DatePickerPage>(()),
  },
  Page {
    path: "/date-range-picker",
    name: "Date Range Picker",
    icon: Some("calendar-range"),
    about: "date range picker period from to start end calendar presets last 7 days two months stay report value memory preset format parse placeholder min max disabled_days clearable size judgments read_only disabled label on_change controlled presets on_input on_submit on_open_change on_dismiss placement offset collision inline trigger_label clear_label panel_label months",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<date_range_picker::DateRangePickerPage>(()),
  },
  Page {
    path: "/rating",
    name: "Rating",
    icon: Some("star"),
    about: "rating stars score review feedback quality half precision clearable read only at optional controlled max size icon empty_icon label value_text invalid disabled on_change slider",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<rating::RatingPage>(()),
  },
  Page {
    path: "/shortcut-recorder",
    name: "Shortcut Recorder",
    icon: Some("keyboard"),
    about: "shortcut recorder hotkey keybinding key combination accelerator record capture press keys kbd conflict taken validate require_modifier placeholder recording_text clear_label modifier_text platform optional controlled on_change size invalid warning valid read_only disabled full_width name label",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<shortcut_recorder::ShortcutRecorderPage>(()),
  },
  Page {
    path: "/color-picker",
    name: "Color Picker",
    icon: Some("palette"),
    about: "color picker colour hue saturation brightness alpha opacity hex rgb hsl swatches eyedropper area rail value controlled on_change on_change_end alpha format formats swatch swatches bare label disabled saturation_label brightness_label hue_label alpha_label format_label",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<color_picker::ColorPickerPage>(()),
  },
  Page {
    path: "/color-field",
    name: "Color Field",
    icon: Some("eyedropper"),
    about: "color field colour input hex swatch eyedropper picker popover typed parse rgb hsl value optional controlled on_change placeholder size invalid warning valid read_only disabled full_width alpha format formats swatches placement label trigger_label panel_label name color swatch selected shape round on_click",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<color_field::ColorFieldPage>(()),
  },
  Page {
    path: "/swatch-picker",
    name: "Swatch Picker",
    icon: Some("palette"),
    about: "swatch picker color colour palette swatches grid choose label tag highlight hex value optional controlled on_change columns size framed label invalid disabled swatch",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<swatch_picker::SwatchPickerPage>(()),
  },
  Page {
    path: "/file-upload",
    name: "File Upload",
    icon: Some("upload"),
    about: "file upload files choose browse dialog drop list progress uploading done failed retry remove accept multiple max_size max_files zone on_files on_reject on_remove on_retry on_change controlled at UploadFile DroppedFile",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<file_upload::FileUploadPage>(()),
  },
  Page {
    path: "/drop-zone",
    name: "Drop Zone",
    icon: Some("inbox"),
    about: "drop zone files drag drop dropped browse dialog accept multiple max_size max_files on_files on_reject drag_over rejected title description icon drop_text reject_text mount",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<drop_zone::DropZonePage>(()),
  },
  Page {
    path: "/date-field",
    name: "Date Field",
    icon: Some("calendar"),
    about: "date field segments day month year typed spin button keyboard birthday new optional controlled controlled_optional min max today locale icon size invalid warning valid read_only disabled full_width label name memory on_change",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<date_field::DateFieldPage>(()),
  },
  Page {
    path: "/time-field",
    name: "Time Field",
    icon: Some("timer"),
    about: "time field segments hour minute second am pm clock twelve hours typed spin button new optional controlled controlled_optional step seconds hour_cycle min max locale icon size invalid read_only disabled label name memory on_change",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<time_field::TimeFieldPage>(()),
  },
  Page {
    path: "/time-picker",
    name: "Time Picker",
    icon: Some("calendar-clock"),
    about: "time picker hours minutes columns panel popover meeting slot new optional controlled controlled_optional step hour_cycle min max placeholder locale size invalid read_only disabled label name memory open placement offset collision inline trigger_label panel_label on_change on_input on_submit on_open_change on_dismiss",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<time_picker::TimePickerPage>(()),
  },
  Page {
    path: "/month-picker",
    name: "Month Picker",
    icon: Some("calendar-days"),
    about: "month picker months year grid report expiry new optional controlled controlled_optional min max today locale previous_label next_label memory label invalid disabled on_change",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<month_picker::MonthPickerPage>(()),
  },
  Page {
    path: "/year-picker",
    name: "Year Picker",
    icon: Some("calendar"),
    about: "year picker years decade grid birth new optional controlled controlled_optional min max today locale previous_label next_label memory label invalid disabled on_change",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<year_picker::YearPickerPage>(()),
  },
  Page {
    path: "/tree-select",
    name: "Tree Select",
    icon: Some("list-tree"),
    about: "tree select treeselect cascader hierarchy nested options folder file category dropdown combobox panel branches leaves path new optional multiple controlled controlled_optional controlled_multiple on_change open branches branches_controlled on_branch_change leaves_only path separator_text max_shown placeholder size clearable clear_label empty_text invalid warning valid read_only disabled full_width label panel_label placement offset collision inline on_open_change on_dismiss mount TreeItem",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<tree_select::TreeSelectPage>(()),
  },
  Page {
    path: "/tooltip",
    name: "Tooltip",
    icon: Some("info"),
    about: "tooltip hint hover focus delay shortcut arrow side placement overlay label describe text mount open disabled",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<tooltip::TooltipPage>(()),
  },
  Page {
    path: "/toast",
    name: "Toast",
    icon: Some("bell"),
    about: "toast snackbar notification message transient success error warning info action undo timer corner stack dismiss title description variant icon close_button on_close label destructive duration persistent mount",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<toast::ToastPage>(()),
  },
  Page {
    path: "/card",
    name: "Card",
    icon: Some("square"),
    about: "card surface panel tile header title description action content footer outline elevated filled pressed interactive settings stat child children variant size width on_click disabled",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<card::CardPage>(()),
  },
  Page {
    path: "/sheet",
    name: "Sheet",
    icon: Some("layers"),
    about: "sheet side panel edge slide start end top bottom drawer grabber veil scrim modal dialog detent drag open size title description label child footer close_button dismiss_on_veil dismiss_on_escape detents inline",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<sheet::SheetPage>(()),
  },
  Page {
    path: "/drawer",
    name: "Drawer",
    icon: Some("panel-right-close"),
    about: "drawer navigation sidebar side panel push aside region open close width overlay veil label child height dismiss_on_veil dismiss_on_escape",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<drawer::DrawerPage>(()),
  },
  Page {
    path: "/message-box",
    name: "Message Box",
    icon: Some("alert-octagon"),
    about: "message box alert dialog alertdialog confirm question warning error information answer default cancel destructive checkbox ask again detail open kind icon title width inline",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<message_box::MessageBoxPage>(()),
  },
  Page {
    path: "/popconfirm",
    name: "Popconfirm",
    icon: Some("help-circle"),
    about: "popconfirm confirm popover question yes no delete destructive anchor inline open trigger title description icon placement offset",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<popconfirm::PopconfirmPage>(()),
  },
  Page {
    path: "/progress-dialog",
    name: "Progress Dialog",
    icon: Some("hourglass"),
    about: "progress dialog copy export install work bar percent count status cancel pause details indeterminate open icon title message value max format detail action on_cancel width inline",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<progress_dialog::ProgressDialogPage>(()),
  },
  Page {
    path: "/hover-card",
    name: "Hover Card",
    icon: Some("eye"),
    about: "hover card preview profile person link file details popover delay pointer rest focus tab arrow mount title description media fact child placement open_delay close_delay open disabled width",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<hover_card::HoverCardPage>(()),
  },
  Page {
    path: "/toggletip",
    name: "Toggletip",
    icon: Some("cursor"),
    about: "toggletip info help question button press tooltip status screen reader touch link learn more label mount content child icon placement arrow open disabled width",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<toggletip::ToggletipPage>(()),
  },
  Page {
    path: "/teaching-tip",
    name: "Teaching Tip",
    icon: Some("lightbulb"),
    about: "teaching tip coach mark onboarding tour step next back guide callout feature discovery arrow anchor open mount title description media child primary secondary on_close close_button variant placement takes_focus width",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<teaching_tip::TeachingTipPage>(()),
  },
  Page {
    path: "/ai-label",
    name: "AI Label",
    icon: Some("sparkles"),
    about: "ai label artificial intelligence generated model assistant sparkles mark badge suggestion draft text icon size abel",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<ai_label::AiLabelPage>(()),
  },
  Page {
    path: "/back-to-top",
    name: "Back To Top",
    icon: Some("arrow-up"),
    about: "back to top scroll up button page long list threshold floating return arrow smooth reduced motion on_click force_shown ack",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<back_to_top::BackToTopPage>(()),
  },
  Page {
    path: "/command-palette",
    name: "Command Palette",
    icon: Some("terminal"),
    about: "command palette search find run go to quick open launcher spotlight ctrl k filter group shortcut keywords ommand alette new placeholder item on_input query loading empty footer inline tem label icon detail disabled on_select",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<command_palette::CommandPalettePage>(()),
  },
  Page {
    path: "/context-menu",
    name: "Context Menu",
    icon: Some("more-vertical"),
    about: "context menu right click secondary press long press shift f10 menu key edit bar cut copy paste overflow more ontext enu new target open item separator label disabled",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<context_menu::ContextMenuPage>(()),
  },
  Page {
    path: "/tabs",
    name: "Tabs",
    icon: Some("folder"),
    about: "tabs tab list panel section view switch line pill vertical count overflow scroll more menu value variant orientation disabled manual label",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<tabs::TabsPage>(()),
  },
  Page {
    path: "/accordion",
    name: "Accordion",
    icon: Some("rows-3"),
    about: "accordion collapsible disclosure expand collapse section faq question answer panel chevron details open items controlled disabled style rules",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<accordion::AccordionPage>(()),
  },
  Page {
    path: "/breadcrumb",
    name: "Breadcrumb",
    icon: Some("chevron-right"),
    about: "breadcrumb trail path way steps link current page separator chevron slash ellipsis collapsed max items navigation step readcrumb tep value icon on_select max_items menu disabled",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<breadcrumb::BreadcrumbPage>(()),
  },
  Page {
    path: "/pagination",
    name: "Pagination",
    icon: Some("skip-forward"),
    about: "pagination pages page next previous ellipsis compact rows per page table footer results items siblings page_size summary on_change disabled",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<pagination::PaginationPage>(()),
  },
  Page {
    path: "/steps",
    name: "Steps",
    icon: Some("list-numbered"),
    about: "steps stepper wizard progress stage current done to come error vertical horizontal orientation on_select pressable ahead disabled label",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<steps::StepsPage>(()),
  },
  Page {
    path: "/table-of-contents",
    name: "Table of Contents",
    icon: Some("list-tree"),
    about: "table of contents toc on this page outline sections headings anchors scroll spy scrollspy reading current section jump navigation article docs sticky new entry entries level container reading_line current on_change on_select title label mount disabled",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<table_of_contents::TableOfContentsPage>(()),
  },
  Page {
    path: "/tree-view",
    name: "Tree View",
    icon: Some("list-tree"),
    about: "tree view treeview file tree explorer folder file nested hierarchy expand collapse outline lazy guides multiple select items expanded selected disabled label on_select on_activate on_expand style rules",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<tree_view::TreeViewPage>(()),
  },
  Page {
    path: "/menubar",
    name: "Menubar",
    icon: Some("more-horizontal"),
    about: "menubar menu bar file edit view help title bar app menus mnemonic alt keyboard shortcut submenu enubar new open title_disabled disabled",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<menubar::MenubarPage>(()),
  },
  Page {
    path: "/toolbar",
    name: "Toolbar",
    icon: Some("wrench"),
    about: "toolbar tools bar row buttons toggles group separator spacer overflow more menu command bar ribbon table formatting new child mount variant size label oolbar tem shortcut stays as_given in_menu roup",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<toolbar::ToolbarPage>(()),
  },
  Page {
    path: "/action-bar",
    name: "Action Bar",
    icon: Some("archive"),
    about: "action bar selection selected bulk actions count clear escape floating bottom top contextual new destructive_action on_clear placement escape_clears",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<action_bar::ActionBarPage>(()),
  },
  Page {
    path: "/sidebar",
    name: "Sidebar",
    icon: Some("panel-left"),
    about: "sidebar side bar navigation rail collapsed collapse expand trigger groups items nested tree selected \
            current page badge count shortcut header footer workspace account drawer overlay narrow",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<sidebar::SidebarPage>(()),
  },
  Page {
    path: "/navbar",
    name: "Navbar",
    icon: Some("dashboard"),
    about: "navbar nav bar header top bar app bar site navigation links brand logo search avatar account menu \
            current page sticky narrow mobile hamburger drawer trailing menu_label media",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<navbar::NavbarPage>(()),
  },
  Page {
    path: "/navigation-menu",
    name: "Navigation Menu",
    icon: Some("share-nodes"),
    about: "navigation menu mega menu nav links panel dropdown hover delay columns featured card site header \
            disclosure",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<navigation_menu::NavigationMenuPage>(()),
  },
  Page {
    path: "/badge",
    name: "Badge",
    icon: Some("inbox"),
    about: "badge status label count counter dot indicator unread notification variant success warning info destructive outline size shape icon dot_only adge max show_zero",
    group: Group::Data,
    mount: |ctx| ctx.mount::<badge::BadgePage>(()),
  },
  Page {
    path: "/tag",
    name: "Tag",
    icon: Some("tag"),
    about: "tag chip pill label keyword category remove filter choice selected chip group variant size icon on_remove disabled on_change bind",
    group: Group::Data,
    mount: |ctx| ctx.mount::<tag::TagPage>(()),
  },
  Page {
    path: "/avatar",
    name: "Avatar",
    icon: Some("user"),
    about: "avatar initials picture image photo profile person user presence online away busy offline group stack overlap more count name icon size shape vatar roup max label",
    group: Group::Data,
    mount: |ctx| ctx.mount::<avatar::AvatarPage>(()),
  },
  Page {
    path: "/persona",
    name: "Persona",
    icon: Some("users"),
    about: "persona person people user member avatar name description line presence online away busy offline size list menu header ersona",
    group: Group::Data,
    mount: |ctx| ctx.mount::<persona::PersonaPage>(()),
  },
  Page {
    path: "/separator",
    name: "Separator",
    icon: Some("minus"),
    about: "separator divider line rule horizontal vertical label or toolbar section length decorative",
    group: Group::Data,
    mount: |ctx| ctx.mount::<separator::SeparatorPage>(()),
  },
  Page {
    path: "/scroll-area",
    name: "Scroll Area",
    icon: Some("rows-3"),
    about: "scroll area scrollbar scroll bar thumb track lane gutter overflow wheel drag region keys orientation             vertical horizontal both scrollbars auto always hover scrolling never label scroll_state width height             max_height croll rea",
    group: Group::Data,
    mount: |ctx| ctx.mount::<scroll_area::ScrollAreaPage>(()),
  },
  Page {
    path: "/spinner",
    name: "Spinner",
    icon: Some("spinner"),
    about: "spinner loader loading busy wait ring dots bars indeterminate status overlay veil inline kind size variant label label_hidden decorative",
    group: Group::Data,
    mount: |ctx| ctx.mount::<spinner::SpinnerPage>(()),
  },
  Page {
    path: "/skeleton",
    name: "Skeleton",
    icon: Some("image"),
    about: "skeleton placeholder loading line circle block paragraph pulse shimmer media row card size width height lines animation keleton status label content",
    group: Group::Data,
    mount: |ctx| ctx.mount::<skeleton::SkeletonPage>(()),
  },
  Page {
    path: "/progress",
    name: "Progress",
    icon: Some("battery"),
    about: "progress bar ring indeterminate loading upload value max percent label top bar meter variant size new show_value format top_bar width",
    group: Group::Data,
    mount: |ctx| ctx.mount::<progress::ProgressPage>(()),
  },
  Page {
    path: "/meter",
    name: "Meter",
    icon: Some("signal"),
    about: "meter segmented disk space battery quota level zone good warning bad value min max low high optimum variant segments label description value_text",
    group: Group::Data,
    mount: |ctx| ctx.mount::<meter::MeterPage>(()),
  },
  Page {
    path: "/gauge",
    name: "Gauge",
    icon: Some("gauge"),
    about: "gauge dial half ring arc speedometer cpu load temperature value min max low high optimum color label value_text bound_text",
    group: Group::Data,
    mount: |ctx| ctx.mount::<gauge::GaugePage>(()),
  },
  Page {
    path: "/alert",
    name: "Alert",
    icon: Some("warning"),
    about: "alert message notice error warning success info destructive severity status callout inline dismiss close action new description variant icon child children on_close role",
    group: Group::Data,
    mount: |ctx| ctx.mount::<alert::AlertPage>(()),
  },
  Page {
    path: "/banner",
    name: "Banner",
    icon: Some("megaphone"),
    about: "banner announcement bar top window page notice update offline sticky dismiss close link centered new variant icon action on_close open role",
    group: Group::Data,
    mount: |ctx| ctx.mount::<banner::BannerPage>(()),
  },
  Page {
    path: "/callout",
    name: "Callout",
    icon: Some("quote"),
    about: "callout note tip warning caution admonition aside hint prose document bar form new title child children variant icon role",
    group: Group::Data,
    mount: |ctx| ctx.mount::<callout::CalloutPage>(()),
  },
  Page {
    path: "/kbd",
    name: "Kbd",
    icon: Some("keyboard"),
    about: "kbd key keyboard shortcut hotkey chord sequence accelerator mac command glyph inline code mono new platform variant size nline ode",
    group: Group::Data,
    mount: |ctx| ctx.mount::<kbd::KbdPage>(()),
  },
  Page {
    path: "/empty-state",
    name: "Empty State",
    icon: Some("folder-open"),
    about: "empty state blank no results nothing zero data placeholder illustration call to action title description icon media size bordered height",
    group: Group::Data,
    mount: |ctx| ctx.mount::<empty_state::EmptyStatePage>(()),
  },
  Page {
    path: "/result",
    name: "Result",
    icon: Some("sparkles"),
    about: "result outcome success error warning info not found 404 done finished page dialog variant title description icon media extra action bordered width",
    group: Group::Data,
    mount: |ctx| ctx.mount::<outcome::OutcomePage>(()),
  },
  Page {
    path: "/list",
    name: "List",
    icon: Some("list-media"),
    about: "list item row settings person avatar icon title description subtitle value meta time chevron switch divider inset contained header density touch mobile ist tem new on_click href selected disabled items dividers action style rules",
    group: Group::Data,
    mount: |ctx| ctx.mount::<list::ListPage>(()),
  },
  Page {
    path: "/listbox",
    name: "Listbox",
    icon: Some("list-check"),
    about: "listbox list box option choose chosen select single multiple many range shift ctrl group check keys page typeahead virtual thousands value options optional multiple controlled controlled_optional controlled_multiple follow_focus on_change label invalid disabled full_width density style rules mount",
    group: Group::Data,
    mount: |ctx| ctx.mount::<listbox::ListboxPage>(()),
  },
  Page {
    path: "/data-list",
    name: "Data List",
    icon: Some("list-pairs"),
    about: "data list description list term definition value key details properties metadata dl text item vertical label style rules",
    group: Group::Data,
    mount: |ctx| ctx.mount::<data_list::DataListPage>(()),
  },
  Page {
    path: "/event-calendar",
    name: "Event Calendar",
    icon: Some("calendar-days"),
    about: "event calendar events schedule scheduler agenda week month day planner meeting appointment today more weekdays weekends timetable new controlled event events period shown weekends header max_shown first_day locale today on_change on_select on_more more_text today_label previous_label next_label mount disabled variant",
    group: Group::Data,
    mount: |ctx| ctx.mount::<event_calendar::EventCalendarPage>(()),
  },
  Page {
    path: "/code-block",
    name: "Code Block",
    icon: Some("file-code"),
    about: "code block snippet source syntax highlight rust language title file copy clipboard line numbers lines highlight_lines wrap scroll max_height min_height mono pre ode lock new language title label line_numbers first_line wrap copy_button copy_label copied_text on_copy selectable mount",
    group: Group::Data,
    mount: |ctx| ctx.mount::<code_block::CodeBlockPage>(()),
  },
  Page {
    path: "/blockquote",
    name: "Blockquote",
    icon: Some("quote"),
    about: "blockquote quote citation passage prose markdown said line italic lockquote new child children label",
    group: Group::Data,
    mount: |ctx| ctx.mount::<blockquote::BlockquotePage>(()),
  },
  Page {
    path: "/bulleted-list",
    name: "Bulleted List",
    icon: Some("list"),
    about: "bulleted list bullet bullets numbered ordered unordered number numbers ol ul li item items nested mark marker icon check media prose markdown typography new item items marker start number_text label list icon media child children",
    group: Group::Data,
    mount: |ctx| ctx.mount::<bulleted_list::BulletedListPage>(()),
  },
  Page {
    path: "/expandable-tile",
    name: "Expandable Tile",
    icon: Some("rows-2"),
    about: "expandable tile expand collapse open close more details disclosure card fold chevron press tile new text child children open bind on_open_change icon open_icon disabled full_width label",
    group: Group::Data,
    mount: |ctx| ctx.mount::<expandable_tile::ExpandableTilePage>(()),
  },
  Page {
    path: "/table",
    name: "Table",
    icon: Some("table"),
    about: "table data rows columns sort sortable select selection checkbox header sticky scroll virtualized thousands density empty loading skeleton pagination footer grid row_key selected checkboxes height max_height count total on_row_click on_row_activate mount label disabled style rules",
    group: Group::Data,
    mount: |ctx| ctx.mount::<table::TablePage>(()),
  },
  Page {
    path: "/timeline",
    name: "Timeline",
    icon: Some("history"),
    about: "timeline history activity feed log events release deploy tracking order steps dot line rail mark avatar new item items label description time icon media variant current upcoming pending child children action on_select disabled style rules orientation horizontal size align alternate time_column value on_load_more loading more_label loading_text mount room panel splitter",
    group: Group::Data,
    mount: |ctx| ctx.mount::<timeline::TimelinePage>(()),
  },
  Page {
    path: "/bar-chart",
    name: "Bar Chart",
    icon: Some("chart-bar"),
    about: "bar chart bars columns chart graph plot series categories values stacked grouped side by side current negative zero grid value_axis legend tooltip hidden value_text on_select empty_text loading size height canvas ChartSeries ChartPoint ChartLegend ChartTooltip",
    group: Group::Charts,
    mount: |ctx| ctx.mount::<bar_chart::BarChartPage>(()),
  },
  Page {
    path: "/line-chart",
    name: "Line Chart",
    icon: Some("chart-line"),
    about: "line chart lines graph plot series categories values trend over time smooth curve straight gap missing ten thousand cursor tooltip legend hidden grid value_axis axis_labels value_text on_select empty_text loading size height canvas ChartSeries ChartPoint LineCurve",
    group: Group::Charts,
    mount: |ctx| ctx.mount::<line_chart::LineChartPage>(()),
  },
  Page {
    path: "/area-chart",
    name: "Area Chart",
    icon: Some("chart-area"),
    about: "area chart areas fill under line stacked whole parts memory series categories smooth curve cursor tooltip legend hidden grid value_axis axis_labels value_text on_select empty_text loading size height canvas ChartSeries LineCurve",
    group: Group::Charts,
    mount: |ctx| ctx.mount::<area_chart::AreaChartPage>(()),
  },
  Page {
    path: "/scatter-chart",
    name: "Scatter Chart",
    icon: Some("chart-scatter"),
    about: "scatter chart points dots x y two axes correlation trend regression outlier series nearest point tooltip legend hidden grid value_axis x_text value_text on_select empty_text loading size height canvas ChartSeries points",
    group: Group::Charts,
    mount: |ctx| ctx.mount::<scatter_chart::ScatterChartPage>(()),
  },
  Page {
    path: "/pie-chart",
    name: "Pie Chart",
    icon: Some("chart-pie"),
    about: "pie chart donut ring slices share whole parts percent legend beside total hole names series colors legend_value share_text total_text tooltip hidden value_text on_select empty_text loading size height canvas ChartSeries ChartPoint PieLegendValue",
    group: Group::Charts,
    mount: |ctx| ctx.mount::<pie_chart::PieChartPage>(()),
  },
  Page {
    path: "/radar-chart",
    name: "Radar Chart",
    icon: Some("chart-radar"),
    about: "radar chart spider web star polar axes rings spokes polygon profile compare series values names value_axis legend tooltip hidden value_text on_select empty_text loading size height canvas ChartSeries ChartPoint",
    group: Group::Charts,
    mount: |ctx| ctx.mount::<radar_chart::RadarChartPage>(()),
  },
  Page {
    path: "/heatmap",
    name: "Heatmap",
    icon: Some("grid"),
    about: "heatmap heat map grid cells calendar contributions commits activity scale steps less more range rows columns column_axis cell_size missing value tooltip value_text on_select empty_text loading size height canvas ChartSeries ChartPoint",
    group: Group::Charts,
    mount: |ctx| ctx.mount::<heatmap::HeatmapPage>(()),
  },
  Page {
    path: "/data-grid",
    name: "Data Grid",
    icon: Some("table"),
    about: "data grid datagrid spreadsheet cells cell focus edit editing in place editor text number choice checkbox on_edit read_only resizable resize pinned pin columns sort selected keyboard arrows tab enter escape f2 virtualized thousands fifty columns mount CellEditor CellValue CellEdit",
    group: Group::Data,
    mount: |ctx| ctx.mount::<data_grid::DataGridPage>(()),
  },
  Page {
    path: "/markdown",
    name: "Markdown",
    icon: Some("file-text"),
    about: "markdown document prose rich text headings lists ordered nested bullets tasks task list checkbox table columns alignment rule image caption figure admonition note tip important warning caution callout details summary footnotes footnote reference kbd keys inline code struck strikethrough link selectable fit_content on_task_change highlights rendered_text ThemeMarkdown",
    group: Group::Data,
    mount: |ctx| ctx.mount::<markdown::MarkdownPage>(()),
  },
  Page {
    path: "/bar-list",
    name: "Bar List",
    icon: Some("chart-rows"),
    about: "bar list chart bars ranking top slowest largest horizontal bar value scale name track new item items title description label max color value_text empty_text framed on_select key disabled style rules",
    group: Group::Charts,
    mount: |ctx| ctx.mount::<bar_list::BarListPage>(()),
  },
  Page {
    path: "/sparkline",
    name: "Sparkline",
    icon: Some("pulse"),
    about: "sparkline trend line series small chart history values min max color width height label summary_text empty_text canvas",
    group: Group::Charts,
    mount: |ctx| ctx.mount::<sparkline::SparklinePage>(()),
  },
  Page {
    path: "/stat-card",
    name: "Stat Card",
    icon: Some("trending-up"),
    about: "stat card kpi metric number dashboard change delta trend badge label value description media sparkline variant size on_click",
    group: Group::Charts,
    mount: |ctx| ctx.mount::<stat_card::StatCardPage>(()),
  },
  Page {
    path: "/seek-bar",
    name: "Seek Bar",
    icon: Some("play"),
    about: "seek bar seekbar scrubber timeline progress position duration buffered loaded chapters thumb drag step on_media picture video preview hover tooltip time on_change on_change_end on_hover size disabled mount",
    group: Group::Media,
    mount: |ctx| ctx.mount::<seek_bar::SeekBarPage>(()),
  },
  Page {
    path: "/media-time",
    name: "Media Time",
    icon: Some("timer"),
    about: "media time elapsed total duration remaining position hours not loaded live badge mono on_media size on_click player clock",
    group: Group::Media,
    mount: |ctx| ctx.mount::<media_time::MediaTimePage>(()),
  },
  Page {
    path: "/waveform",
    name: "Waveform",
    icon: Some("equalizer"),
    about: "waveform audio voice message peaks bars loudness played unheard seek progress controlled at on_change on_change_end step disabled",
    group: Group::Media,
    mount: |ctx| ctx.mount::<waveform::WaveformPage>(()),
  },
  Page {
    path: "/picture",
    name: "Picture",
    icon: Some("image"),
    about: "picture image img photo figure caption alt thumbnail cover contain fill fit anchor aspect ratio skeleton loading progress error retry badge bar actions download menu select selected gallery bare overlay Picture PictureFit PictureSize PictureShape from_file from_bytes from_resource empty label width height size on_click on_download on_retry on_change mount",
    group: Group::Media,
    mount: |ctx| ctx.mount::<picture::PicturePage>(()),
  },
  Page {
    path: "/lightbox",
    name: "Lightbox",
    icon: Some("zoom-in"),
    about: "lightbox viewer gallery picture photo image zoom pan wheel pinch drag fit counter filmstrip thumbnails previous next arrows download close escape overlay modal inline Lightbox index zoom zoom_max wrap filmstrip counter on_index_change on_zoom_change on_download on_dismiss",
    group: Group::Media,
    mount: |ctx| ctx.mount::<lightbox::LightboxPage>(()),
  },
  Page {
    path: "/video-player",
    name: "Video Player",
    icon: Some("play-circle"),
    about: "video player media movie clip picture poster seek bar play pause volume mute captions subtitles speed rate quality settings picture in picture fullscreen live buffering waiting ended error retry overlay feed controls keys VideoPlayer VideoPlayerSize playing position duration buffered on_playing_change on_position_change on_position_change_end on_volume_change on_muted_change on_rate_change on_captions_change on_quality_change on_fullscreen_change on_picture_in_picture_change on_retry mount",
    group: Group::Media,
    mount: |ctx| ctx.mount::<video_player::VideoPlayerPage>(()),
  },
  Page {
    path: "/splitter",
    name: "Splitter",
    icon: Some("resize-horizontal"),
    about: "splitter split pane panes resizable resize divider handle grip drag sidebar editor terminal layout collapse horizontal vertical sizes on_resize on_resize_end step large_step framed disabled plitter ane size share min max collapsible label",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<splitter::SplitterPage>(()),
  },
  Page {
    path: "/property-grid",
    name: "Property Grid",
    icon: Some("sliders"),
    about: "property grid inspector properties category categories editor name value default reset modified changed xy pair slider toggle choice color text number custom read_only disabled on_change name_width divider title trailing",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<property_grid::PropertyGridPage>(()),
  },
  Page {
    path: "/document-tabs",
    name: "Document Tabs",
    icon: Some("file-code"),
    about: "document tabs editor files open documents tab bar close dirty unsaved pinned pin preview reorder drag move overflow more menu context menu new tab add middle click ctrl+w new optional controlled controlled_optional tab tabs on_change on_close on_reorder on_add on_keep context_menu label close_label add_label more_label disabled mount DocumentTab",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<document_tabs::DocumentTabsPage>(()),
  },
  Page {
    path: "/status-bar",
    name: "Status Bar",
    icon: Some("pulse"),
    about: "status bar statusbar window bottom branch errors warnings progress indexing line column encoding language notifications items leading trailing variant tone priority drops_first tooltip on_select icon icon_only label mount StatusItem StatusRegion",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<status_bar::StatusBarPage>(()),
  },
  Page {
    path: "/window-chrome",
    name: "Window Chrome",
    icon: Some("window"),
    about: "window chrome title bar titlebar frame controls minimize maximize restore close traffic lights windows macos gnome kde linux platform WindowPlatform WindowControls minimize_label maximize_label restore_label close_label minimizable maximizable on_close title icon label leading trailing tabs menubar content overlay inactive maximized drag snap system menu decorations custom",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<window_chrome::WindowChromePage>(()),
  },
  Page {
    path: "/window-tabs",
    name: "Window Tabs",
    icon: Some("window"),
    about: "window tabs tab strip title bar titlebar browser editor files open close add new tab plus unsaved dot reorder drag detach tear off move ctrl tab ctrl w shortcuts keyboard WindowTab WindowTabs on_change on_close on_add on_reorder on_detach close_button add_label close_label disabled label mount indow abs",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<window_tabs::WindowTabsPage>(()),
  },
  Page {
    path: "/command-bar",
    name: "Command Bar",
    icon: Some("layers"),
    about: "command bar commandbar ribbon tabs groups group label clipboard format insert view paste cut copy large small stack toggle on secondary more chevron overflow menu toolbar editor file manager window CommandBarTab CommandBarCommand on_change on_select controlled disabled label more_label",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<command_bar::CommandBarPage>(()),
  },
  Page {
    path: "/find-bar",
    name: "Find Bar",
    icon: Some("search"),
    about: "find bar findbar search find in page replace replace all match case whole word regex regular expression count matches next previous f3 enter escape ctrl+f query on_query on_next on_previous on_replace on_replace_all on_dismiss return_focus open count_text",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<find_bar::FindBarPage>(()),
  },
  Page {
    path: "/floating-panel",
    name: "Floating Panel",
    icon: Some("layers"),
    about: "floating panel tool window palette inspector layers drag move resize collapse close dock title bar grip header PanelHeader FloatingPanels front position size open media button menu on_dismiss on_position_change on_size_change on_open_change default_position default_size min_size resizable collapsible close_button dismiss_on_escape label",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<floating_panel::FloatingPanelPage>(()),
  },
  Page {
    path: "/dock-layout",
    name: "Dock Layout",
    icon: Some("panels"),
    about: "dock layout docking panel panels split splits tabs tool window editor ide workspace drag drop rearrange float floating hide hidden strip edge maximize restore close documents area empty arrangement store restore persist layout DockLayout DockPanel DockState DockNode DockPlace DockSide state to_text parse show move_panel on_change on_detach on_close empty count dirty close_button floatable hideable movable min_size icon f6 shift+f10",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<dock_layout::DockLayoutPage>(()),
  },
  Page {
    path: "/column-view",
    name: "Column View",
    icon: Some("columns"),
    about: "column view columns miller finder file browser hierarchy folders path drill down side by side lazy spinner new controlled path items on_change on_activate disabled label loading_label mount TreeItem ColumnView",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<column_view::ColumnViewPage>(()),
  },
  Page {
    path: "/sortable-list",
    name: "Sortable List",
    icon: Some("grip-vertical"),
    about: "sortable list reorder drag drop rows order move handle grip ghost insertion line count selection keyboard alt arrows SortableList SortableItem on_reorder on_reorder_many move_rows selected selected_controlled on_selected_change handle handle_only disabled label mount",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<sortable_list::SortableListPage>(()),
  },
  Page {
    path: "/selection-marquee",
    name: "Selection Marquee",
    icon: Some("marquee"),
    about: "selection marquee rubber band lasso rectangle drag select tiles grid files canvas shift ctrl add toggle escape scroll autoscroll SelectionMarquee Marquee MarqueeMode on_start on_change on_change_end on_cancel scroll rect disabled label touches contains select mount",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<selection_marquee::SelectionMarqueePage>(()),
  },
  Page {
    path: "/play-button",
    name: "Play Button",
    icon: Some("play"),
    about: "play button pause replay media player video audio start stop round disc overlay picture loading spinner playing on_change size variant primary overlay play_label pause_label replay_label disabled",
    group: Group::Media,
    mount: |ctx| ctx.mount::<play_button::PlayButtonPage>(()),
  },
  Page {
    path: "/volume",
    name: "Volume",
    icon: Some("volume"),
    about: "volume mute unmute sound loud quiet slider media player video audio muted collapsed on_media picture on_change on_change_end on_muted_change controlled muted_controlled size length label mute_label unmute_label mount",
    group: Group::Media,
    mount: |ctx| ctx.mount::<media_volume::MediaVolumePage>(()),
  },
  Page {
    path: "/speed-menu",
    name: "Speed Menu",
    icon: Some("gauge"),
    about: "speed menu playback rate speed media player video audio normal faster slower 0.5x 1.5x 2x check on_change on_dismiss on_open_change rates heading_text label placement offset inline MEDIA_RATES media_rate_text",
    group: Group::Media,
    mount: |ctx| ctx.mount::<media_speed_menu::MediaSpeedMenuPage>(()),
  },
  Page {
    path: "/audio-player",
    name: "Audio Player",
    icon: Some("music"),
    about: "audio player sound music track song podcast voice message mini player docked card compact row waveform seek bar play pause previous next shuffle repeat one all volume mute speed rate playback menu artwork title loading error try again unheard dot AudioPlayer AudioPlayerVariant AudioRepeat on_playing_change on_change on_change_end",
    group: Group::Media,
    mount: |ctx| ctx.mount::<audio_player::AudioPlayerPage>(()),
  },
  Page {
    path: "/audio-queue",
    name: "Audio Queue",
    icon: Some("list"),
    about: "audio queue playlist up next tracks rows list current now playing equalizer number artwork length more unavailable arrows keyboard AudioQueue AudioQueueItem on_change on_more",
    group: Group::Media,
    mount: |ctx| ctx.mount::<audio_queue::AudioQueuePage>(()),
  },
];

/// The page at `path`: the overview for a path that is no page's.
pub fn page_at(path: &str) -> &'static Page {
  PAGES.iter().find(|page| page.path == path).unwrap_or(&PAGES[0])
}
