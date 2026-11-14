//! The gallery's shell: its own title bar, the header across the top, the
//! sidebar with the pages, and the page beside it.

use lurq::{
  app::{
    ctx::Ctx,
    events::KeyboardEvent,
    theme::{BorderSize, Breakpoint, Density, PaletteColor, RadiusSize, Theme},
    window::Appearance,
  },
  components::{
    ChromeTitleBar, Column, Outlet, Popup, Row, ScrollVertical, WindowControlColors, WindowControlContent,
    WindowControlKind, WindowControls,
  },
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{BackgroundColor, CursorIcon, Element, border::Border, color::Color},
  ui::{
    Badge, BadgePart, BadgeVariant, Button, ButtonVariant, CommandItem, CommandPalette, FontFamily, Kbd, KeyPlatform,
    SegmentedControl, Sidebar, SidebarGroup, SidebarItem, SidebarTrigger, icons,
  },
};

use crate::{
  kit::{FILL, icon, mono, popover, segmented, select, text},
  pages::{Group, PAGES, page_at},
};

const TITLE_BAR: f32 = 36.0;
const HEADER: f32 = 56.0;

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

/// The title bar of the gallery's window, drawn by the gallery from the
/// theme: the window has no frame of the system's.
pub fn title_bar() -> ChromeTitleBar {
  let glyph = |name: &'static str, size: f32| WindowControlContent::element(move |_| icon(name, size).into());
  let clear = BackgroundColor::Color(Color::new(0, 0, 0, 0));
  let controls = WindowControls::new()
    .button_size(46.0, TITLE_BAR)
    .content(WindowControlKind::Minimize, glyph("minus", 16.0))
    // The square is drawn smaller than the others, so that it weighs as much as they do.
    .content(WindowControlKind::Maximize, glyph("square", 12.0))
    .content(WindowControlKind::Restore, glyph("duplicate", 14.0))
    .content(WindowControlKind::Close, glyph("close", 16.0))
    .foreground(PaletteColor::Foreground)
    .hover_background(PaletteColor::StateHover)
    .active_background(PaletteColor::StatePressed)
    .control_colors(
      WindowControlKind::Close,
      WindowControlColors {
        foreground: PaletteColor::Foreground.into(),
        background: clear,
        hover_background: PaletteColor::Destructive.into(),
        active_background: PaletteColor::DestructiveHover.into(),
      },
    )
    .hover_foreground(WindowControlKind::Close, PaletteColor::DestructiveForeground);
  ChromeTitleBar::new()
    .height(TITLE_BAR)
    .background(PaletteColor::Muted)
    .border_bottom(Border::inside(BorderSize::Sm, PaletteColor::Border))
    .leading(
      Row::new()
        .padding_left(12.0)
        .spacing(8.0)
        .align_items(Alignment::Center)
        .foreground(PaletteColor::Foreground)
        .child(icon("box", 16.0))
        .child(text("lurq gallery", 13.0, FontWeight::Medium)),
    )
    .controls(controls)
}

/// The shell around the page the route is at.
pub fn shell(ctx: &mut Ctx, state: &Shell) -> Element {
  let current = page_at(&ctx.route_path()).path;
  let find = state.search_open.clone();
  // What is behind the cards is the theme's canvas. A card, the header and a control are surfaces on it, so each
  // stands out from what it is on.
  Column::new()
    .size(FILL, FILL)
    .background(PaletteColor::Canvas)
    .foreground(PaletteColor::Foreground)
    // Every key pressed in the window comes by here, whatever has the focus.
    .on_key_down(move |event: KeyboardEvent| {
      if (event.ctrl || event.meta) && (event.code == "KeyK" || event.key.eq_ignore_ascii_case("k")) {
        find.set(true);
        event.prevent_default();
      }
    })
    .child(header(ctx, state))
    .child(search(ctx, state))
    .child(
      Row::new()
        .width(FILL)
        .flex(1.0)
        .align_items(Alignment::Stretch)
        .child(sidebar(ctx, state, current))
        .child(
          // Each page has a scroll of its own: a page opened from another starts at its top, wherever the
          // one before it was scrolled to.
          ScrollVertical::new(Outlet::mount(ctx)).key(current).flex(1.0),
        ),
    )
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
fn search(ctx: &mut Ctx, state: &Shell) -> CommandPalette {
  let navigator = ctx.navigator();
  let mut palette = CommandPalette::new(state.search_open.clone())
    .placeholder("Search components and props…")
    .empty("No page for that.")
    .footer(true);
  for group in Group::ALL {
    let pages = PAGES.iter().filter(|page| page.group == group).map(|page| {
      let mut command = CommandItem::new(page.name)
        .keywords(page.about.split_whitespace())
        .key(page.path);
      if let Some(name) = page.icon {
        command = command.icon(name);
      }
      if let Some(navigator) = navigator.clone() {
        let path = page.path;
        command = command.on_select(move || navigator.push(path));
      }
      command
    });
    palette = palette.group(group.name(), pages);
  }
  palette
}

/// What the gear opens: what a theme can be told that is not a color.
fn settings(theme: &Theme, state: &Shell) -> Column {
  let setting = |name: &str, about: &str, control: SegmentedControl| {
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
      .child(control)
  };
  let motion = MOTION[state.motion.get().min(MOTION.len() - 1)].1;
  let scale = theme.text_scale();
  let scale = TEXT_SCALE
    .iter()
    .map(|(_, value)| *value)
    .min_by(|a, b| (a - scale).abs().total_cmp(&(b - scale).abs()))
    .unwrap_or(1.0);
  popover()
    .width(320.0)
    .padding(16.0)
    .spacing(16.0)
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
  let roomy = lurq::ui::current_breakpoint().is_some_and(|breakpoint| breakpoint >= Breakpoint::Xl);
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
  let controls = Row::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(labeled("theme", appearance.into()))
    .child(labeled("density", density.into()))
    .child(gear)
    .child(
      Popup::new(state.settings_anchor.clone(), settings(&theme, state))
        .open(state.settings_open.clone())
        .placement(lurq::app::ctx::Placement::BottomEnd)
        .offset(0.0, 8.0),
    );
  Row::new()
    .width(FILL)
    .height(HEADER)
    .padding_horizontal(20.0)
    .spacing(16.0)
    .align_items(Alignment::Center)
    .background(PaletteColor::Background)
    .border_bottom(Border::inside(BorderSize::Sm, PaletteColor::Border))
    .child(brand)
    .child(
      Row::new()
        .flex(1.0)
        .justify(Justify::Center)
        .child(search_field(state, roomy)),
    )
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

/// The pages in the design system's groups, each group with how many it has, and the page the route is at
/// selected. A row of it is 32 high at the default density, and follows the density.
fn sidebar(ctx: &mut Ctx, state: &Shell, current: &str) -> Element {
  let groups = Group::ALL.into_iter().filter_map(|group| {
    let pages: Vec<_> = PAGES.iter().filter(|page| page.group == group).collect();
    if pages.is_empty() {
      return None;
    }
    let items = pages.iter().map(|page| {
      let mut item = SidebarItem::new(page.name)
        .key(page.path)
        .href(page.path)
        .selected(page.path == current);
      if let Some(name) = page.icon {
        item = item.icon(name);
      }
      item
    });
    Some(SidebarGroup::new(group.name()).count(pages.len()).items(items))
  });
  Sidebar::new()
    .label("Pages")
    .groups(groups)
    .collapsed(state.collapsed.clone())
    .open(state.sidebar_open.clone())
    .mount(ctx)
}
