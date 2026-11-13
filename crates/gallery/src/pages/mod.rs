//! The pages of the gallery, in the groups and the order of the design
//! system. A page is here when what it shows exists: a component gets its
//! page when it lands.

use lurq::{app::ctx::Ctx, node::Element};

mod icons;
mod overview;
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
    icon: Some("sparkles"),
    about: "icon font glyph size",
    group: Group::GettingStarted,
    mount: |ctx| ctx.mount::<icons::Icons>(()),
  },
];

/// The page at `path`: the overview for a path that is no page's.
pub fn page_at(path: &str) -> &'static Page {
  PAGES.iter().find(|page| page.path == path).unwrap_or(&PAGES[0])
}
