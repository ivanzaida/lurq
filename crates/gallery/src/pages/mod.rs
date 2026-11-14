//! The pages of the gallery, in the groups and the order of the design
//! system. A page is here when what it shows exists: a component gets its
//! page when it lands.

use lurq::{app::ctx::Ctx, node::Element};

mod accordion;
mod alert;
mod avatar;
mod badge;
mod banner;
mod breadcrumb;
mod button;
mod callout;
mod card;
mod checkbox;
mod checkbox_card;
mod combobox;
mod command_palette;
mod context_menu;
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
mod mentions;
mod menu;
mod message_box;
mod multi_select;
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
mod skeleton;
mod slider;
mod spinner;
mod steps;
mod switch;
mod tabs;
mod tag;
mod teaching_tip;
mod textarea;
mod toast;
mod toggletip;
mod token_input;
mod tokens;
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
    icon: None,
    about: "variant size shape icon icon_only loading disabled full_width on_click pill primary secondary outline ghost destructive",
    group: Group::Actions,
    mount: |ctx| ctx.mount::<button::ButtonPage>(()),
  },
  Page {
    path: "/link",
    name: "Link",
    icon: None,
    about: "link text underline external navigate",
    group: Group::Actions,
    mount: |ctx| ctx.mount::<family::LinkPage>(()),
  },
  Page {
    path: "/toggle",
    name: "Toggle",
    icon: None,
    about: "toggle on off pressed state bold",
    group: Group::Actions,
    mount: |ctx| ctx.mount::<family::TogglePage>(()),
  },
  Page {
    path: "/toggle-group",
    name: "Toggle Group",
    icon: None,
    about: "toggle group single multiple choice alignment",
    group: Group::Actions,
    mount: |ctx| ctx.mount::<family::ToggleGroupPage>(()),
  },
  Page {
    path: "/segmented-control",
    name: "Segmented Control",
    icon: None,
    about: "segmented control segment track choice tabs",
    group: Group::Actions,
    mount: |ctx| ctx.mount::<family::SegmentedControlPage>(()),
  },
  Page {
    path: "/button-group",
    name: "Button Group",
    icon: None,
    about: "button group split joined separator",
    group: Group::Actions,
    mount: |ctx| ctx.mount::<family::ButtonGroupPage>(()),
  },
  Page {
    path: "/input",
    name: "Input",
    icon: None,
    about: "input field text entry label description error hint required optional placeholder icon invalid read only form",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<input::InputPage>(()),
  },
  Page {
    path: "/textarea",
    name: "Textarea",
    icon: None,
    about: "textarea multiline text rows placeholder invalid disabled scroll bio note",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<textarea::TextareaPage>(()),
  },
  Page {
    path: "/checkbox",
    name: "Checkbox",
    icon: None,
    about: "checkbox check box tick indeterminate select all form option",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<checkbox::CheckboxPage>(()),
  },
  Page {
    path: "/radio",
    name: "Radio",
    icon: None,
    about: "radio group choice option chosen one of exclusive",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<radio::RadioPage>(()),
  },
  Page {
    path: "/switch",
    name: "Switch",
    icon: None,
    about: "switch on off setting track thumb toggle dark mode",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<switch::SwitchPage>(()),
  },
  Page {
    path: "/slider",
    name: "Slider",
    icon: None,
    about: "slider range value min max step track thumb volume drag",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<slider::SliderPage>(()),
  },
  Page {
    path: "/popover",
    name: "Popover",
    icon: None,
    about: "popover panel overlay popup anchor placement dismiss escape outside title description",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<popover::PopoverPage>(()),
  },
  Page {
    path: "/menu",
    name: "Menu",
    icon: None,
    about: "menu dropdown item command shortcut separator label group checked destructive disabled popup anchor split",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<menu::MenuPage>(()),
  },
  Page {
    path: "/dialog",
    name: "Dialog",
    icon: None,
    about: "dialog modal veil scrim overlay confirm title description footer close escape scroll",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<dialog::DialogPage>(()),
  },
  Page {
    path: "/select",
    name: "Select",
    icon: None,
    about: "select dropdown choose option list menu placeholder chevron value time zone",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<select::SelectPage>(()),
  },
  Page {
    path: "/checkbox-card",
    name: "Checkbox Card",
    icon: None,
    about: "checkbox card choice tile option add-on plan price icon description checked group",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<checkbox_card::CheckboxCardPage>(()),
  },
  Page {
    path: "/radio-card",
    name: "Radio Card",
    icon: None,
    about: "radio card choice tile option plan price icon description chosen group exclusive",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<radio_card::RadioCardPage>(()),
  },
  Page {
    path: "/fieldset",
    name: "Fieldset",
    icon: None,
    about: "fieldset legend group fields disabled form error summary errors alert attention link focus",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<fieldset::FieldsetPage>(()),
  },
  Page {
    path: "/input-otp",
    name: "Input OTP",
    icon: None,
    about: "input otp one time code verification pin two factor 2fa cells digits paste autofill mask",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<input_otp::InputOtpPage>(()),
  },
  Page {
    path: "/password-input",
    name: "Password Input",
    icon: None,
    about: "password input masked secret reveal show hide eye strength meter weak strong new password",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<password_input::PasswordInputPage>(()),
  },
  Page {
    path: "/search-field",
    name: "Search Field",
    icon: None,
    about: "search field query filter find clear escape enter submit shortcut key hint searchbox",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<search_field::SearchFieldPage>(()),
  },
  Page {
    path: "/input-group",
    name: "Input Group",
    icon: None,
    about: "input group addon prefix suffix before after https url domain unit button select joined",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<input_group::InputGroupPage>(()),
  },
  Page {
    path: "/number-input",
    name: "Number Input",
    icon: None,
    about: "number input numeric stepper spin button quantity price min max step precision format prefix suffix",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<number_input::NumberInputPage>(()),
  },
  Page {
    path: "/combobox",
    name: "Combobox",
    icon: None,
    about: "combobox autocomplete typeahead search filter select option list type custom value fetch loading no results",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<combobox::ComboboxPage>(()),
  },
  Page {
    path: "/multi-select",
    name: "Multi Select",
    icon: None,
    about: "multi select multiple choose several tags tokens chips checkbox list filter max shown clear",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<multi_select::MultiSelectPage>(()),
  },
  Page {
    path: "/token-input",
    name: "Token Input",
    icon: None,
    about: "token input tags chips tag list recipients variables template tokens remove backspace",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<token_input::TokenInputPage>(()),
  },
  Page {
    path: "/mentions",
    name: "Mentions",
    icon: None,
    about: "mentions mention at people trigger suggestions autocomplete comment prompt slash command hashtag",
    group: Group::Inputs,
    mount: |ctx| ctx.mount::<mentions::MentionsPage>(()),
  },
  Page {
    path: "/tooltip",
    name: "Tooltip",
    icon: None,
    about: "tooltip hint hover focus delay shortcut arrow side placement overlay label describe",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<tooltip::TooltipPage>(()),
  },
  Page {
    path: "/toast",
    name: "Toast",
    icon: None,
    about: "toast snackbar notification message transient success error warning info action undo timer corner stack dismiss",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<toast::ToastPage>(()),
  },
  Page {
    path: "/card",
    name: "Card",
    icon: None,
    about: "card surface panel tile header title description action content footer outline elevated filled pressed interactive settings stat",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<card::CardPage>(()),
  },
  Page {
    path: "/sheet",
    name: "Sheet",
    icon: None,
    about: "sheet side panel edge slide start end top bottom drawer grabber veil scrim modal dialog detent drag",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<sheet::SheetPage>(()),
  },
  Page {
    path: "/drawer",
    name: "Drawer",
    icon: None,
    about: "drawer navigation sidebar side panel push aside region open close width overlay veil",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<drawer::DrawerPage>(()),
  },
  Page {
    path: "/message-box",
    name: "Message Box",
    icon: None,
    about: "message box alert dialog alertdialog confirm question warning error information answer default cancel destructive checkbox ask again detail",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<message_box::MessageBoxPage>(()),
  },
  Page {
    path: "/popconfirm",
    name: "Popconfirm",
    icon: None,
    about: "popconfirm confirm popover question yes no delete destructive anchor inline",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<popconfirm::PopconfirmPage>(()),
  },
  Page {
    path: "/progress-dialog",
    name: "Progress Dialog",
    icon: None,
    about: "progress dialog copy export install work bar percent count status cancel pause details indeterminate",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<progress_dialog::ProgressDialogPage>(()),
  },
  Page {
    path: "/hover-card",
    name: "Hover Card",
    icon: None,
    about: "hover card preview profile person link file details popover delay pointer rest focus tab arrow",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<hover_card::HoverCardPage>(()),
  },
  Page {
    path: "/toggletip",
    name: "Toggletip",
    icon: None,
    about: "toggletip info help question button press tooltip status screen reader touch link learn more",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<toggletip::ToggletipPage>(()),
  },
  Page {
    path: "/teaching-tip",
    name: "Teaching Tip",
    icon: None,
    about: "teaching tip coach mark onboarding tour step next back guide callout feature discovery arrow",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<teaching_tip::TeachingTipPage>(()),
  },
  Page {
    path: "/command-palette",
    name: "Command Palette",
    icon: None,
    about: "command palette search find run go to quick open launcher spotlight ctrl k filter group shortcut keywords",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<command_palette::CommandPalettePage>(()),
  },
  Page {
    path: "/context-menu",
    name: "Context Menu",
    icon: None,
    about: "context menu right click secondary press long press shift f10 menu key edit bar cut copy paste overflow more",
    group: Group::Surfaces,
    mount: |ctx| ctx.mount::<context_menu::ContextMenuPage>(()),
  },
  Page {
    path: "/tabs",
    name: "Tabs",
    icon: None,
    about: "tabs tab list panel section view switch line pill vertical count overflow scroll more menu",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<tabs::TabsPage>(()),
  },
  Page {
    path: "/accordion",
    name: "Accordion",
    icon: None,
    about: "accordion collapsible disclosure expand collapse section faq question answer panel chevron details",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<accordion::AccordionPage>(()),
  },
  Page {
    path: "/breadcrumb",
    name: "Breadcrumb",
    icon: None,
    about: "breadcrumb trail path way steps link current page separator chevron slash ellipsis collapsed max items navigation",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<breadcrumb::BreadcrumbPage>(()),
  },
  Page {
    path: "/pagination",
    name: "Pagination",
    icon: None,
    about: "pagination pages page next previous ellipsis compact rows per page table footer results",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<pagination::PaginationPage>(()),
  },
  Page {
    path: "/steps",
    name: "Steps",
    icon: None,
    about: "steps stepper wizard progress stage current done to come error vertical horizontal",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<steps::StepsPage>(()),
  },
  Page {
    path: "/tree-view",
    name: "Tree View",
    icon: None,
    about: "tree view treeview file tree explorer folder file nested hierarchy expand collapse outline lazy guides multiple select",
    group: Group::Navigation,
    mount: |ctx| ctx.mount::<tree_view::TreeViewPage>(()),
  },
  Page {
    path: "/badge",
    name: "Badge",
    icon: None,
    about: "badge status label count counter dot indicator unread notification variant success warning info destructive outline",
    group: Group::Data,
    mount: |ctx| ctx.mount::<badge::BadgePage>(()),
  },
  Page {
    path: "/tag",
    name: "Tag",
    icon: None,
    about: "tag chip pill label keyword category remove filter choice selected chip group",
    group: Group::Data,
    mount: |ctx| ctx.mount::<tag::TagPage>(()),
  },
  Page {
    path: "/avatar",
    name: "Avatar",
    icon: None,
    about: "avatar initials picture image photo profile person user presence online away busy offline group stack overlap more count",
    group: Group::Data,
    mount: |ctx| ctx.mount::<avatar::AvatarPage>(()),
  },
  Page {
    path: "/separator",
    name: "Separator",
    icon: None,
    about: "separator divider line rule horizontal vertical label or toolbar section",
    group: Group::Data,
    mount: |ctx| ctx.mount::<separator::SeparatorPage>(()),
  },
  Page {
    path: "/spinner",
    name: "Spinner",
    icon: None,
    about: "spinner loader loading busy wait ring dots bars indeterminate status overlay veil inline",
    group: Group::Data,
    mount: |ctx| ctx.mount::<spinner::SpinnerPage>(()),
  },
  Page {
    path: "/skeleton",
    name: "Skeleton",
    icon: None,
    about: "skeleton placeholder loading line circle block paragraph pulse shimmer media row card",
    group: Group::Data,
    mount: |ctx| ctx.mount::<skeleton::SkeletonPage>(()),
  },
  Page {
    path: "/progress",
    name: "Progress",
    icon: None,
    about: "progress bar ring indeterminate loading upload value max percent label top bar meter variant size",
    group: Group::Data,
    mount: |ctx| ctx.mount::<progress::ProgressPage>(()),
  },
  Page {
    path: "/alert",
    name: "Alert",
    icon: None,
    about: "alert message notice error warning success info destructive severity status callout inline dismiss close action",
    group: Group::Data,
    mount: |ctx| ctx.mount::<alert::AlertPage>(()),
  },
  Page {
    path: "/banner",
    name: "Banner",
    icon: None,
    about: "banner announcement bar top window page notice update offline sticky dismiss close link centered",
    group: Group::Data,
    mount: |ctx| ctx.mount::<banner::BannerPage>(()),
  },
  Page {
    path: "/callout",
    name: "Callout",
    icon: None,
    about: "callout note tip warning caution admonition aside hint prose document bar form",
    group: Group::Data,
    mount: |ctx| ctx.mount::<callout::CalloutPage>(()),
  },
  Page {
    path: "/kbd",
    name: "Kbd",
    icon: None,
    about: "kbd key keyboard shortcut hotkey chord sequence accelerator mac command glyph inline code mono",
    group: Group::Data,
    mount: |ctx| ctx.mount::<kbd::KbdPage>(()),
  },
  Page {
    path: "/empty-state",
    name: "Empty State",
    icon: None,
    about: "empty state blank no results nothing zero data placeholder illustration call to action",
    group: Group::Data,
    mount: |ctx| ctx.mount::<empty_state::EmptyStatePage>(()),
  },
  Page {
    path: "/result",
    name: "Result",
    icon: None,
    about: "result outcome success error warning info not found 404 done finished page dialog",
    group: Group::Data,
    mount: |ctx| ctx.mount::<outcome::OutcomePage>(()),
  },
];

/// The page at `path`: the overview for a path that is no page's.
pub fn page_at(path: &str) -> &'static Page {
  PAGES.iter().find(|page| page.path == path).unwrap_or(&PAGES[0])
}
