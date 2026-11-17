//! The gallery's shell: the header across the top, the sidebar with the pages, and beside it the page of the tab
//! that is shown.

use lurq::{
  app::{
    ctx::Ctx,
    events::{EdgePanEvent, KeyboardEvent, ScreenEdge},
    theme::{BorderSize, Breakpoint, Density, PaletteColor, RadiusSize, Theme, TypographyStyle},
    window::Appearance,
  },
  components::{Column, Popup, Row},
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border, color::Color},
  ui::{
    Badge, BadgePart, BadgeVariant, Button, ButtonVariant, CommandItem, CommandPalette, FontFamily, Kbd, KeyPlatform,
    Length, Recipe, Selector, Sidebar, SidebarGroup, SidebarItem, SidebarPart, SidebarState, SidebarTrigger, icons,
  },
};

use crate::{
  kit::{FILL, icon, mono, popover, segmented, select, text},
  pages::{Group, PAGES, page_at},
  tabs::OpenTabs,
};

const HEADER: f32 = 56.0;
const SIDEBAR: f32 = 248.0;

/// What the shell keeps between renders: whether the search is open, and which of its popups is.
#[derive(Clone)]
pub struct Shell {
  /// Whether the search across the pages, a command palette, is open.
  search_open: Signal<bool>,
  density_anchor: ElementRef,
  density_open: Signal<bool>,
  settings_anchor: ElementRef,
  settings_open: Signal<bool>,
  /// Which of [`MOTION`] is chosen.
  motion: Signal<usize>,
  /// Whether the sidebar is its rail, and whether it is open over the page on a narrow window.
  collapsed: Signal<bool>,
  sidebar_open: Signal<bool>,
}

/// Whether motion is reduced: as the system says, or as the app does.
const MOTION: [(&str, Option<bool>); 3] = [("System", None), ("Reduced", Some(true)), ("Full", Some(false))];
const TEXT_SCALE: [(&str, f32); 3] = [("100%", 1.0), ("115%", 1.15), ("130%", 1.3)];

impl Shell {
  pub fn new(ctx: &mut Ctx) -> Self {
    Self {
      search_open: ctx.signal(false),
      density_anchor: ElementRef::new(),
      density_open: ctx.signal(false),
      settings_anchor: ElementRef::new(),
      settings_open: ctx.signal(false),
      motion: ctx.signal(0),
      collapsed: ctx.signal(false),
      sidebar_open: ctx.signal(false),
    }
  }
}

/// The shell around the page of the tab that is shown.
pub fn shell(ctx: &mut Ctx, state: &Shell, open: &OpenTabs) -> Element {
  let router = open.router();
  let current = page_at(&router.path().get()).path;
  let find = state.search_open.clone();
  let new_tab = open.clone();
  // On a phone on its side the side bar and the page keep clear of the notch. The bottom is left to the pages:
  // what scrolls goes under the home indicator, and a page with a bar of its own pads by the safe area itself.
  let safe = ctx.safe_area();
  // As narrow as a phone the side bar lies over the page, and a finger coming in from the left edge opens it, as
  // its button does: the finger is then the side bar's, and does not also go back in the history.
  let narrow = !lurq::ui::current_breakpoint().is_some_and(|breakpoint| breakpoint >= Breakpoint::Md);
  let side = state.sidebar_open.clone();
  // What is behind the cards is the theme's canvas. A card, the header and a control are surfaces on it, so each
  // stands out from what it is on.
  Column::new()
    .size(FILL, FILL)
    .background(PaletteColor::Canvas)
    .foreground(PaletteColor::Foreground)
    .on_edge_pan(move |event: EdgePanEvent| {
      if !narrow || event.edge != ScreenEdge::Left || event.default_prevented() {
        return;
      }
      event.prevent_default();
      if !side.get_untracked() {
        side.set(true);
      }
    })
    .child(header(ctx, state))
    .child(search(state, open))
    .child(
      Row::new()
        .width(FILL)
        .flex(1.0)
        .padding_left(safe.left)
        .padding_right(safe.right)
        .align_items(Alignment::Stretch)
        .child(sidebar(ctx, state, open, current))
        .child(Column::new().flex(1.0).min_width(0.0).child(open.outlet(ctx))),
    )
    // Every key pressed in the window comes by here, whatever has the focus, after everything on the page has
    // heard it: a key comes to the elements in the order of the tree, and this one is the last of it. A key
    // something on the page took for its own (Ctrl+K in an editor is its link) is not the window's.
    .child(Row::new().on_key_down(move |event: KeyboardEvent| {
      if event.default_prevented() || !(event.ctrl || event.meta) || event.alt {
        return;
      }
      if event.code == "KeyK" || event.key.eq_ignore_ascii_case("k") {
        find.set(true);
        event.prevent_default();
      } else if !event.shift && (event.code == "KeyT" || event.key.eq_ignore_ascii_case("t")) {
        // A new tab, as a browser opens one; the strip's own keys close it and go between the tabs.
        new_tab.open("/");
        event.prevent_default();
      }
    }))
    .into()
}

/// What opens the search across the pages: a box that looks like the field it was, with what it is for in it and
/// the keys that open it after. A press, Enter or Space opens it.
fn search_field(state: &Shell, roomy: bool) -> Row {
  let open = state.search_open.clone();
  let pressed = state.search_open.clone();
  let mut field = Row::new()
    .width(FILL)
    .max_width(360.0)
    .height(36.0)
    .padding_left(12.0)
    .padding_right(8.0)
    .spacing(8.0)
    .align_items(Alignment::Center)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Background)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Input))
    .hovered(|style| style.border(Border::inside(BorderSize::Sm, PaletteColor::Ring)))
    .cursor(CursorIcon::Pointer)
    .tab_index(0)
    .describe("a11y_role", "button")
    .describe("a11y_name", "Search")
    .on_click(move |_| open.set(true))
    .on_key_down(move |event: KeyboardEvent| {
      if event.focus_within && (event.key == "Enter" || event.key == " ") {
        event.prevent_default();
        pressed.set(true);
      }
    })
    .child(icon("search", 16.0).color(PaletteColor::MutedForeground))
    .child(
      Row::new().flex(1.0).child(
        text(
          if roomy {
            "Search components and props…"
          } else {
            "Search…"
          },
          14.0,
          FontWeight::Normal,
        )
        .color(PaletteColor::MutedForeground),
      ),
    );
  // The keys that open it are said where there is room to say them.
  if roomy {
    field = field.child(Kbd::shortcut("Mod+K").platform(KeyPlatform::current()));
  }
  field
}

/// The search across the pages: a command palette of every page, in the groups of the side bar, found by its name
/// and by what it is about. Running one goes to it.
fn search(state: &Shell, open: &OpenTabs) -> CommandPalette {
  let mut palette = CommandPalette::new(state.search_open.clone())
    .placeholder("Search components and props…")
    .empty_text("No page for that.")
    .key_hints(true);
  for group in Group::ALL {
    let pages = PAGES.iter().filter(|page| page.group == group).map(|page| {
      let mut command = CommandItem::new(page.name)
        .keywords(page.about.split_whitespace())
        .key(page.path);
      if let Some(name) = page.icon {
        command = command.icon(name);
      }
      // In the tab that is shown when it is run, which may be another than the one the search was built in.
      let (path, open) = (page.path, open.clone());
      command.on_select(move || {
        open.focus_next_page();
        open.router().push(path);
      })
    });
    palette = palette.group(group.name(), pages);
  }
  palette
}

/// What the gear opens: what a theme can be told that is not a color; and, in a window as narrow as a phone,
/// the appearance and the density too, which the header has no room for there.
fn settings(theme: &Theme, state: &Shell, narrow: bool) -> Column {
  // A setting of the window: its name, what it does, and its control (a segmented control of any value).
  fn setting(name: &str, about: &str, control: impl Into<Element>) -> Column {
    Column::new()
      .width(FILL)
      .spacing(8.0)
      .child(
        Column::new()
          .width(FILL)
          .spacing(2.0)
          .child(text(name, 13.0, FontWeight::Medium))
          .child(
            text(about, 12.0, FontWeight::Normal)
              .color(PaletteColor::MutedForeground)
              .width(FILL),
          ),
      )
      .child(control.into())
  }
  let motion = MOTION[state.motion.get().min(MOTION.len() - 1)].1;
  let scale = theme.text_scale();
  let scale = TEXT_SCALE
    .iter()
    .map(|(_, value)| *value)
    .min_by(|a, b| (a - scale).abs().total_cmp(&(b - scale).abs()))
    .unwrap_or(1.0);
  let mut panel = popover()
    .width(if narrow { 288.0 } else { 320.0 })
    .padding(16.0)
    .spacing(16.0);
  if narrow {
    let looks = [
      ("System", None),
      ("Light", Some(Appearance::Light)),
      ("Dark", Some(Appearance::Dark)),
    ];
    let densities: Vec<(&str, Density)> = Density::all()
      .iter()
      .map(|density| (density_name(*density), *density))
      .collect();
    panel = panel
      .child(setting(
        "Theme",
        "The appearance of every page: the system's, or one of the two.",
        segmented(&looks, &theme.preferred_appearance(), {
          let theme = theme.clone();
          move |look| theme.set_preferred_appearance(look)
        })
        .full_width(true),
      ))
      .child(setting(
        "Density",
        "How high a row and a control are.",
        segmented(&densities, &theme.density(), {
          let theme = theme.clone();
          move |density| theme.set_density(density)
        })
        .full_width(true),
      ));
  }
  panel
    .child(setting(
      "Motion",
      "Reduced, a change of place or size ends at once; a fade still runs.",
      segmented(&MOTION, &motion, {
        let (theme, chosen) = (theme.clone(), state.motion.clone());
        move |reduced| {
          chosen.set(MOTION.iter().position(|(_, value)| *value == reduced).unwrap_or(0));
          theme.set_reduced_motion(reduced);
        }
      }),
    ))
    .child(setting(
      "Text size",
      "Every typography style of the theme, scaled.",
      segmented(&TEXT_SCALE, &scale, {
        let theme = theme.clone();
        move |scale| theme.set_text_scale(scale)
      }),
    ))
    .child(setting(
      "Bold text",
      "Every style a step heavier, as a system's bold text setting asks.",
      segmented(&[("Off", false), ("On", true)], &theme.bold_text(), {
        let theme = theme.clone();
        move |bold| theme.set_bold_text(bold)
      }),
    ))
}

/// Brand, search across the pages, and what applies to every demo at once.
fn header(ctx: &mut Ctx, state: &Shell) -> Row {
  let theme = ctx.theme().clone();
  // Narrower than a desktop, the header keeps its controls and drops what names them.
  let breakpoint = lurq::ui::current_breakpoint();
  let roomy = breakpoint.is_some_and(|breakpoint| breakpoint >= Breakpoint::Xl);
  // As narrow as a phone, it keeps the side bar's button, the brand, the search and the gear: the appearance
  // and the density are in what the gear opens.
  let narrow = !breakpoint.is_some_and(|breakpoint| breakpoint >= Breakpoint::Md);
  let looks = [
    ("System", None),
    ("Light", Some(Appearance::Light)),
    ("Dark", Some(Appearance::Dark)),
  ];
  // What collapses the sidebar to its rail, or opens it over the page on a narrow window.
  let trigger = SidebarTrigger::new(state.collapsed.clone()).open(state.sidebar_open.clone());
  let mut brand = Row::new()
    .spacing(10.0)
    .align_items(Alignment::Center)
    .child(trigger)
    .child(
      Row::new()
        .size(28.0, 28.0)
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .rounded(RadiusSize::Md)
        .background(PaletteColor::Primary)
        .foreground(PaletteColor::PrimaryForeground)
        .child(icon("box", 16.0)),
    )
    .child(text("lurq", 15.0, FontWeight::SemiBold));
  if roomy {
    brand = brand.child(text("Design System", 14.0, FontWeight::Normal).color(PaletteColor::MutedForeground));
  }
  brand = brand.child(
    Badge::new(&format!("v{}", lurq::VERSION))
      .variant(BadgeVariant::Secondary)
      .style(BadgePart::Label, |s| s.font_family(FontFamily::Mono).font_size(11.0)),
  );

  let labeled = |label: &str, control: Element| {
    let mut row = Row::new().spacing(12.0).align_items(Alignment::Center);
    if roomy {
      row = row.child(mono(label, 12.0).color(PaletteColor::MutedForeground));
    }
    row.child(control)
  };
  let appearance = segmented(&looks, &theme.preferred_appearance(), {
    let theme = theme.clone();
    move |look| theme.set_preferred_appearance(look)
  });
  let densities: Vec<(&str, Density)> = Density::all()
    .iter()
    .map(|density| (density_name(*density), *density))
    .collect();
  let density = select(
    &state.density_anchor,
    &state.density_open,
    &densities,
    &theme.density(),
    {
      let theme = theme.clone();
      move |density| theme.set_density(density)
    },
  )
  .width(150.0);
  // The popup hangs from what the button is in: a component takes no reference to itself.
  let gear = Row::new().ref_element(state.settings_anchor.clone()).child(
    Button::new("Settings")
      .variant(ButtonVariant::Ghost)
      .icon(icons::SETTINGS)
      .icon_only(true)
      .on_click({
        let open = state.settings_open.clone();
        move |_| open.update(|open| *open = !*open)
      }),
  );
  let mut controls = Row::new()
    .spacing(if narrow { 4.0 } else { 12.0 })
    .align_items(Alignment::Center);
  if narrow {
    // The search is a button there: its field has no room beside the brand.
    controls = controls.child(
      Button::new("Search")
        .variant(ButtonVariant::Ghost)
        .icon(icons::SEARCH)
        .icon_only(true)
        .on_click({
          let open = state.search_open.clone();
          move |_| open.set(true)
        }),
    );
  } else {
    controls = controls
      .child(labeled("theme", appearance.into()))
      .child(labeled("density", density.into()));
  }
  let controls = controls.child(gear).child(
    Popup::new(state.settings_anchor.clone(), settings(&theme, state, narrow))
      .open(state.settings_open.clone())
      .placement(lurq::app::ctx::Placement::BottomEnd)
      .offset(0.0, 8.0),
  );
  let middle = if narrow {
    Row::new().flex(1.0)
  } else {
    Row::new()
      .flex(1.0)
      .justify(Justify::Center)
      .child(search_field(state, roomy))
  };
  // On a phone the header reaches under the status bar and the notch, and keeps what is in it clear of them.
  let safe = ctx.safe_area();
  let edge = if narrow { 12.0 } else { 20.0 };
  Row::new()
    .width(FILL)
    .height(HEADER + safe.top)
    .padding_top(safe.top)
    .padding_left(edge + safe.left)
    .padding_right(edge + safe.right)
    .spacing(if narrow { 8.0 } else { 16.0 })
    .align_items(Alignment::Center)
    .background(PaletteColor::Background)
    .border_bottom(Border::inside(BorderSize::Sm, PaletteColor::Border))
    .child(brand)
    .child(middle)
    .child(controls)
}

/// A density as the header names it.
fn density_name(density: Density) -> &'static str {
  match density {
    Density::Compact => "Compact",
    Density::Default => "Default",
    Density::Comfortable => "Comfortable",
    Density::Touch => "Touch",
    Density::Extra(name) => name.as_str(),
  }
}

/// The pages in the design system's groups, each group with how many it has, and the page the tab that is shown is
/// at selected: a press goes there in that tab, the middle button or a press with Ctrl opens it in another. A row of it
/// is 32 high at the default density, and follows the density.
fn sidebar(ctx: &mut Ctx, state: &Shell, open: &OpenTabs, current: &str) -> Element {
  let groups = Group::ALL.into_iter().filter_map(|group| {
    let pages: Vec<_> = PAGES.iter().filter(|page| page.group == group).collect();
    if pages.is_empty() {
      return None;
    }
    let items = pages.iter().map(|page| {
      let (path, aside) = (page.path, open.clone());
      let mut item = SidebarItem::new(page.name)
        .key(page.path)
        .href(page.path)
        .selected(page.path == current)
        // The middle button, or a press with Ctrl: in a tab of its own.
        .on_open_aside(move || aside.open(path));
      if let Some(name) = page.icon {
        item = item.icon(name);
      }
      item
    });
    Some(SidebarGroup::new(group.name()).count(pages.len()).items(items))
  });
  // The gallery's side bar is its own, as it was drawn by hand: on the canvas of its window, the page that is open
  // a card on it, 248 wide, padded 20 down and 16 across with 20 between its groups, a group's label in the mono
  // family over 6, and its rows' names in 14. The component's own look is the board's sidebar, which stands beside
  // a page of another color. The rail and the sidebar over the page keep the component's room.
  let look = Recipe::new()
    .base(SidebarPart::Root, |s| {
      s.width(SIDEBAR).padding_y(20.0).background(PaletteColor::Canvas)
    })
    .state(SidebarState::Collapsed, SidebarPart::Root, |s| s.width(52.0).padding_y(8.0))
    .state(SidebarState::Overlay, SidebarPart::Root, |s| {
      s.width(Length::Fill).background(Color::new(0, 0, 0, 0))
    })
    .base(SidebarPart::Body, |s| s.padding_x(16.0).gap(20.0))
    .state(SidebarState::Collapsed, SidebarPart::Body, |s| s.padding_x(8.0).gap(4.0))
    // A line of 15 and 6 under it, as the design has it, whatever the font's own line is.
    .base(SidebarPart::GroupLabel, |s| {
      s.height(21.0).padding_top(0.0).padding_bottom(6.0)
    })
    .base(SidebarPart::GroupText, |s| {
      s.typography(TypographyStyle::Mono)
        .font_size(11.0)
        .weight(FontWeight::Normal)
        .letter_spacing(1.0)
    })
    .base(SidebarPart::ItemLabel, |s| s.font_size(14.0))
    // The open page's icon stays muted: the card says which it is.
    .state(SidebarState::Selected, SidebarPart::ItemIcon, |s| s.foreground(PaletteColor::MutedForeground))
    .hovered(SidebarPart::Item, |s| {
      s.background(PaletteColor::Highlight)
        .foreground(PaletteColor::HighlightForeground)
    })
    .state(SidebarState::Selected, SidebarPart::Item, |s| {
      s.background(PaletteColor::Card)
        .foreground(PaletteColor::CardForeground)
    })
    .when(
      Selector::new().state(SidebarState::Selected).hovered(),
      SidebarPart::Item,
      |s| s.background(PaletteColor::Card),
    )
    .when(
      Selector::new().state(SidebarState::Selected).pressed(),
      SidebarPart::Item,
      |s| s.background(PaletteColor::Card),
    );
  Sidebar::new()
    .label("Pages")
    .rules(&look)
    .groups(groups)
    .collapsed(state.collapsed.get())
    .open(state.sidebar_open.clone())
    .mount(ctx)
}
