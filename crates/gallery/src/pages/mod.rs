//! The pages of the gallery, in the groups and the order of the design
//! system. A page is here when what it shows exists: a component gets its
//! page when it lands.

use lurq::{app::ctx::Ctx, node::Element};

mod button;
mod checkbox;
mod family;
mod family_play;
mod icons;
mod overview;
mod radio;
  },
  Page {
mod slider;
mod switch;
mod tokens;

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
];

/// The page at `path`: the overview for a path that is no page's.
pub fn page_at(path: &str) -> &'static Page {
  PAGES.iter().find(|page| page.path == path).unwrap_or(&PAGES[0])
}
