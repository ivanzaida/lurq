//! The pages of the gallery, in the groups and the order of the design
//! system. A page is here when what it shows exists: a component gets its
//! page when it lands.

use lurq::{app::ctx::Ctx, node::Element};

mod accordion;
mod action_bar;
mod alert;
mod avatar;
mod badge;
mod banner;
mod breadcrumb;
mod button;
mod calendar;
mod callout;
mod card;
mod checkbox;
mod checkbox_card;
mod combobox;
mod command_palette;
mod context_menu;
mod data_list;
mod date_picker;
mod date_range_picker;
mod dialog;
mod drawer;
mod empty_state;
mod family;
mod family_play;
mod fieldset;
mod hover_card;
mod icons;
mod input;
mod input_group;
mod input_otp;
mod kbd;
mod list;
mod listbox;
mod mentions;
mod menu;
mod menubar;
mod message_box;
mod multi_select;
mod navbar;
mod navigation_menu;
mod number_input;
mod outcome;
mod overview;
mod pagination;
mod password_input;
mod popconfirm;
mod popover;
mod progress;
mod progress_dialog;
mod radio;
mod radio_card;
mod search_field;
mod select;
mod separator;
mod sheet;
mod sidebar;
mod skeleton;
mod slider;
mod spinner;
mod splitter;
mod steps;
mod switch;
mod table;
mod tabs;
mod tag;
mod teaching_tip;
mod textarea;
mod toast;
mod toggletip;
mod token_input;
mod tokens;
mod toolbar;
mod tooltip;
mod tree_view;

/// A group of the side bar.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Group {
  GettingStarted,
  Actions,
  Inputs,
  Navigation,
  Surfaces,
  Data,
  Desktop,
}

impl Group {
  /// The groups in the order of the side bar.
  pub const ALL: [Group; 7] = [
    Self::GettingStarted,
    Self::Actions,
    Self::Inputs,
    Self::Navigation,
    Self::Surfaces,
    Self::Data,
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
    about: "input field text entry label description error hint required optional placeholder icon invalid read only form value size warning valid read_only disabled full_width on_input name control horizontal",
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
    about: "checkbox check box tick indeterminate select all form option label checked on_change invalid label_hidden disabled",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<checkbox::CheckboxPage>(()),
  },
  Page {
    path: "/radio",
    name: "Radio",
    icon: Some("dot-circle"),
    about: "radio group choice option chosen one of exclusive value choices controlled horizontal invalid disabled label on_choose label_hidden",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<radio::RadioPage>(()),
  },
  Page {
    path: "/switch",
    name: "Switch",
    icon: Some("bolt"),
    about: "switch on off setting track thumb toggle dark mode label on_change label_hidden disabled",
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
    about: "checkbox card choice tile option add-on plan price icon description checked group title on_change invalid disabled full_width values cards controlled horizontal",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<checkbox_card::CheckboxCardPage>(()),
  },
  Page {
    path: "/radio-card",
    name: "Radio Card",
    icon: Some("circle"),
    about: "radio card choice tile option plan price icon description chosen group exclusive title on_choose invalid disabled full_width value cards controlled horizontal",
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
    path: "/calendar",
    name: "Calendar",
    icon: Some("calendar-days"),
    about: "calendar date day month week range picker grid today weekday locale first day week numbers value memory months min max disabled_days first_day week_numbers outside_days disabled on_change on_range_change",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<calendar::CalendarPage>(()),
  },
  Page {
    path: "/date-picker",
    name: "Date Picker",
    icon: Some("calendar"),
    about: "date picker datepicker calendar day field typed parse format popover due date birthday value memory placeholder size locale min max disabled_days clearable week_numbers open month invalid warning valid read_only disabled label on_change",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<date_picker::DatePickerPage>(()),
  },
  Page {
    path: "/date-range-picker",
    name: "Date Range Picker",
    icon: Some("calendar-range"),
    about: "date range picker period from to start end calendar presets last 7 days two months stay report value memory preset format parse placeholder min max disabled_days clearable size judgments read_only disabled label on_change",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<date_range_picker::DateRangePickerPage>(()),
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
    about: "toast snackbar notification message transient success error warning info action undo timer corner stack dismiss title description variant icon icon_hidden closable on_close duration persistent",
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
            current page sticky narrow mobile hamburger drawer",
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
    path: "/separator",
    name: "Separator",
    icon: Some("minus"),
    about: "separator divider line rule horizontal vertical label or toolbar section length decorative",
    group: Group::Data,
    mount: |ctx| ctx.mount::<separator::SeparatorPage>(()),
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
    icon: None,
    about: "list item row settings person avatar icon title description subtitle value meta time chevron switch divider inset contained header density touch mobile ist tem new on_click href selected disabled items dividers action style rules",
    group: Group::Data,
    mount: |ctx| ctx.mount::<list::ListPage>(()),
  },
  Page {
    path: "/listbox",
    name: "Listbox",
    icon: None,
    about: "listbox list box option choose chosen select single multiple many range shift ctrl group check keys page typeahead virtual thousands value options follow_focus on_change label invalid disabled full_width density style rules mount",
    group: Group::Data,
    mount: |ctx| ctx.mount::<listbox::ListboxPage>(()),
  },
  Page {
    path: "/data-list",
    name: "Data List",
    icon: None,
    about: "data list description list term definition value key details properties metadata dl text item vertical label style rules",
    group: Group::Data,
    mount: |ctx| ctx.mount::<data_list::DataListPage>(()),
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
    path: "/splitter",
    name: "Splitter",
    icon: Some("resize-horizontal"),
    about: "splitter split pane panes resizable resize divider handle grip drag sidebar editor terminal layout collapse horizontal vertical sizes on_resize on_resize_end step large_step framed disabled plitter ane size share min max collapsible label",
    group: Group::Desktop,
    mount: |ctx| ctx.mount::<splitter::SplitterPage>(()),
  },
];

/// The page at `path`: the overview for a path that is no page's.
pub fn page_at(path: &str) -> &'static Page {
  PAGES.iter().find(|page| page.path == path).unwrap_or(&PAGES[0])
}
