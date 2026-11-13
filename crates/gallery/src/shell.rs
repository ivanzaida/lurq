//! The gallery's shell: its own title bar, the header across the top, the
//! side bar with the pages, and the page beside it.

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use lurq::{
  app::{
    ctx::Ctx,
    events::KeyboardEvent,
    theme::{BorderSize, Breakpoint, Density, PaletteColor, RadiusSize, Theme, TypographyStyle},
    window::Appearance,
  },
  components::{
    ChromeTitleBar, Column, Link, Outlet, Popup, Row, ScrollVertical, TextInput, WindowControlColors,
    WindowControlContent, WindowControlKind, WindowControls,
  },
  core::{ElementRef, Signal},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{BackgroundColor, CursorIcon, Element, border::Border, color::Color},
  ui::{Button, ButtonVariant, icons},
};

use crate::{
  kit::{FILL, badge, icon, keys, mono, overline, popover, row_height, segmented, select, text},
  pages::{Group, PAGES, page_at},
};

const TITLE_BAR: f32 = 36.0;
const HEADER: f32 = 56.0;
const SIDEBAR: f32 = 248.0;

/// What the shell keeps between renders: what is searched for, and which of
/// its popups is open.
#[derive(Clone)]
pub struct Shell {
  search: Signal<String>,
  search_field: ElementRef,
  search_focused: Signal<bool>,
  /// Bumped when the search is asked for from the keyboard, so that the shell renders and focuses it.
  find: Signal<u32>,
  find_asked: Arc<AtomicBool>,
  density_anchor: ElementRef,
  density_open: Signal<bool>,
  settings_anchor: ElementRef,
  settings_open: Signal<bool>,
  /// Which of [`MOTION`] is chosen.
  motion: Signal<usize>,
}

/// Whether motion is reduced: as the system says, or as the app does.
const MOTION: [(&str, Option<bool>); 3] = [("System", None), ("Reduced", Some(true)), ("Full", Some(false))];
const TEXT_SCALE: [(&str, f32); 3] = [("100%", 1.0), ("115%", 1.15), ("130%", 1.3)];

impl Shell {
  pub fn new(ctx: &mut Ctx) -> Self {
    Self {
      search: ctx.signal(String::new()),
      search_field: ElementRef::new(),
      search_focused: ctx.signal(false),
      find: ctx.signal(0),
      find_asked: Arc::default(),
      density_anchor: ElementRef::new(),
      density_open: ctx.signal(false),
      settings_anchor: ElementRef::new(),
      settings_open: ctx.signal(false),
      motion: ctx.signal(0),
    }
  }
}

/// The chord that goes to the search, as it is written on the platform.
fn find_chord() -> &'static str {
  if cfg!(target_os = "macos") { "⌘ K" } else { "Ctrl K" }
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
  // Reading the signal is what renders the shell when the search is asked for.
  let _ = state.find.get();
  if state.find_asked.swap(false, Ordering::Relaxed) {
    ctx.focus(&state.search_field);
  }
  let (find, asked) = (state.find.clone(), state.find_asked.clone());
  Column::new()
    .size(FILL, FILL)
    .background(PaletteColor::Background)
    .foreground(PaletteColor::Foreground)
    // Every key pressed in the window comes by here, whatever has the focus.
    .on_key_down(move |event: KeyboardEvent| {
      if (event.ctrl || event.meta) && (event.code == "KeyK" || event.key.eq_ignore_ascii_case("k")) {
        asked.store(true, Ordering::Relaxed);
        find.update(|count| *count += 1);
        event.prevent_default();
      }
    })
    .child(header(ctx, state))
    .child(
      Row::new()
        .width(FILL)
        .flex(1.0)
        .align_items(Alignment::Stretch)
        .child(sidebar(ctx, current, &state.search.get()))
        .child(
          ScrollVertical::new(Outlet::mount(ctx))
            .flex(1.0)
            .border_left(Border::inside(BorderSize::Sm, PaletteColor::Border)),
        ),
    )
    .into()
}

/// The search across the pages: a field with what it is for before it and the keys that go to it after.
fn search_field(theme: &Theme, state: &Shell, roomy: bool) -> Row {
  let body = theme.typography_style(TypographyStyle::Body);
  let hint = lurq::layout::text_style::TextStyle {
    color: theme.palette_color(PaletteColor::MutedForeground),
    ..body.clone()
  };
  let focused = state.search_focused.get();
  let (on, off) = (state.search_focused.clone(), state.search_focused.clone());
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
    .border(Border::inside(
      BorderSize::Sm,
      if focused {
        PaletteColor::Ring
      } else {
        PaletteColor::Input
      },
    ))
    .child(icon("search", 16.0).color(PaletteColor::MutedForeground))
    .child(
      TextInput::styled(state.search.clone(), body)
        .placeholder(if roomy {
          "Search components and props…"
        } else {
          "Search…"
        })
        .placeholder_style(hint)
        .flex(1.0)
        .height(FILL)
        .ref_element(state.search_field.clone())
        .on_focus(move || on.set(true))
        .on_blur(move || off.set(false))
        .tab_index(0),
    );
  // The keys that go to it are said where there is room to say them.
  if roomy {
    field = field.child(keys(find_chord()));
  }
  if focused {
    field = field.outline(theme.focus_ring().outline());
  }
  field
}

/// What the gear opens: what a theme can be told that is not a color.
fn settings(theme: &Theme, state: &Shell) -> Column {
  let setting = |name: &str, about: &str, control: Row| {
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
  let mut brand = Row::new()
    .spacing(10.0)
    .align_items(Alignment::Center)
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
  brand = brand.child(badge(&format!("v{}", lurq::VERSION)));

  let labeled = |label: &str, control: Row| {
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
    .child(labeled("theme", appearance))
    .child(labeled("density", density))
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
    .border_bottom(Border::inside(BorderSize::Sm, PaletteColor::Border))
    .child(brand)
    .child(
      Row::new()
        .flex(1.0)
        .justify(Justify::Center)
        .child(search_field(&theme, state, roomy)),
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

/// The pages in the design system's groups, each group with how many it has.
/// A row of it is 32 high at the default density, and follows the density.
fn sidebar(ctx: &mut Ctx, current: &str, search: &str) -> ScrollVertical {
  let theme = ctx.theme().clone();
  let row_height = row_height(&theme);
  let wanted = search.trim().to_lowercase();
  let mut groups = Column::new()
    .width(FILL)
    .padding_vertical(20.0)
    .padding_horizontal(16.0)
    .spacing(20.0);
  let mut found = 0;
  for group in Group::ALL {
    let pages: Vec<_> = PAGES
      .iter()
      .filter(|page| page.group == group)
      .filter(|page| wanted.is_empty() || page.name.to_lowercase().contains(&wanted) || page.about.contains(&wanted))
      .collect();
    if pages.is_empty() {
      continue;
    }
    found += pages.len();
    let mut list = Column::new().width(FILL).spacing(2.0).child(
      Row::new()
        .width(FILL)
        // A line of 15 and 6 under it, as the design has it, whatever the font's own line is.
        .height(21.0)
        .padding_horizontal(8.0)
        .padding_bottom(6.0)
        .align_items(Alignment::Center)
        .justify(Justify::SpaceBetween)
        .child(overline(group.name()))
        .child(mono(&pages.len().to_string(), 11.0).color(PaletteColor::MutedForeground)),
    );
    for page in pages {
      let on = page.path == current;
      let mut row = Row::new()
        .width(FILL)
        .height(row_height)
        .padding_horizontal(8.0)
        .spacing(8.0)
        .align_items(Alignment::Center)
        .rounded(RadiusSize::Md)
        .cursor(CursorIcon::Pointer);
      if let Some(name) = page.icon {
        row = row.child(icon(name, 16.0).color(PaletteColor::MutedForeground));
      }
      row = row.child(text(
        page.name,
        14.0,
        if on { FontWeight::Medium } else { FontWeight::Normal },
      ));
      row = if on {
        row
          .background(PaletteColor::Highlight)
          .foreground(PaletteColor::HighlightForeground)
      } else {
        row.hovered(|style| {
          style
            .background(PaletteColor::Highlight)
            .foreground(PaletteColor::HighlightForeground)
        })
      };
      list = list.child(Link::build_empty(ctx, page.path).child(row));
    }
    groups = groups.child(list);
  }
  if found == 0 {
    groups = groups.child(
      text("No page for that.", 14.0, FontWeight::Normal)
        .color(PaletteColor::MutedForeground)
        .width(FILL),
    );
  }
  ScrollVertical::new(groups).width(SIDEBAR)
}
