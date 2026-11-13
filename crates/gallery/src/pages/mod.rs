//! The pages of the gallery, in the groups and the order of the design
//! system. A page is here when what it shows exists: a component gets its
//! page when it lands.

use lurq::{app::ctx::Ctx, node::Element};

mod accordion;
mod breadcrumb;
mod button;
mod card;
mod checkbox;
mod dialog;
mod family;
mod family_play;
mod icons;
mod input;
mod menu;
mod overview;
mod pagination;
mod popover;
mod radio;
mod select;
mod separator;
mod slider;
mod steps;
mod switch;
mod tabs;
mod textarea;
mod toast;
mod tokens;
mod tooltip;

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
    path: "/separator",
    name: "Separator",
    icon: None,
    about: "separator divider line rule horizontal vertical label or toolbar section",
    group: Group::Data,
    mount: |ctx| ctx.mount::<separator::SeparatorPage>(()),
  },
];

/// The page at `path`: the overview for a path that is no page's.
pub fn page_at(path: &str) -> &'static Page {
  PAGES.iter().find(|page| page.path == path).unwrap_or(&PAGES[0])
}
