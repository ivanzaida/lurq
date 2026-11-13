//! The gallery's shell: the header across the top, the side bar with the
//! pages, and the page beside it.

use lurq::{
  app::{
    ctx::Ctx,
    theme::{BorderSize, PaletteColor, RadiusSize, TypographyStyle},
    window::Appearance,
  },
  components::{Column, Link, Outlet, Row, ScrollVertical, Text, TextInput},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, border::Border},
};

use crate::{
  kit::{FILL, badge, muted, segmented, styled},
  pages::{Group, PAGES, page_at},
};

const HEADER: f32 = 56.0;
const SIDEBAR: f32 = 248.0;

/// The shell around the page the route is at.
pub fn shell(ctx: &mut Ctx, search: &Signal<String>) -> Element {
  let current = page_at(&ctx.route_path()).path;
  Column::new()
    .size(FILL, FILL)
    .background(PaletteColor::Background)
    .foreground(PaletteColor::Foreground)
    .child(header(ctx, search))
    .child(
      Row::new()
        .width(FILL)
        .flex(1.0)
        .align_items(Alignment::Stretch)
        .child(sidebar(ctx, current, &search.get()))
        .child(ScrollVertical::new(Outlet::mount(ctx)).flex(1.0)),
    )
    .into()
}

/// Brand, search across the pages, and what applies to every demo at once.
fn header(ctx: &mut Ctx, search: &Signal<String>) -> Row {
  let theme = ctx.theme().clone();
  let looks = [
    ("System", None),
    ("Light", Some(Appearance::Light)),
    ("Dark", Some(Appearance::Dark)),
  ];
  let brand = Row::new()
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
        .child(styled("l", TypographyStyle::Label, |style| {
          style.weight = FontWeight::Bold
        })),
    )
    .child(styled("lurq", TypographyStyle::Body, |style| {
      style.weight = FontWeight::SemiBold
    }))
    .child(muted("Design System", TypographyStyle::Body))
    .child(badge(&format!("v{}", lurq::VERSION)));
  let body = theme.typography_style(TypographyStyle::Body);
  let hint = lurq::layout::text_style::TextStyle {
    color: theme.palette_color(PaletteColor::MutedForeground),
    ..body.clone()
  };
  let find = TextInput::styled(search.clone(), body)
    .placeholder("Search pages…")
    .placeholder_style(hint)
    .size(360.0, 36.0)
    .padding_horizontal(12.0)
    .rounded(RadiusSize::Md)
    .background(PaletteColor::Background)
    .border(Border::inside(BorderSize::Sm, PaletteColor::Input))
    .tab_index(0);
  let appearance = Row::new()
    .spacing(10.0)
    .align_items(Alignment::Center)
    .child(muted("theme", TypographyStyle::Mono))
    .child(segmented(&looks, &theme.preferred_appearance(), {
      let theme = theme.clone();
      move |look| theme.set_preferred_appearance(look)
    }));
  Row::new()
    .width(FILL)
    .height(HEADER)
    .padding_horizontal(20.0)
    .align_items(Alignment::Center)
    .justify(Justify::SpaceBetween)
    .border_bottom(Border::inside(BorderSize::Sm, PaletteColor::Border))
    .child(brand)
    .child(find)
    .child(appearance)
}

/// The pages in the design system's groups, each group with how many it has.
fn sidebar(ctx: &mut Ctx, current: &str, search: &str) -> ScrollVertical {
  let wanted = search.trim().to_lowercase();
  let mut groups = Column::new().width(FILL).padding(16.0).spacing(20.0);
  let mut found = 0;
  for group in Group::ALL {
    let pages: Vec<_> = PAGES
      .iter()
      .filter(|page| page.group == group)
      .filter(|page| wanted.is_empty() || page.name.to_lowercase().contains(&wanted))
      .collect();
    if pages.is_empty() {
      continue;
    }
    found += pages.len();
    let mut list = Column::new().width(FILL).spacing(2.0).child(
      Row::new()
        .width(FILL)
        .height(28.0)
        .padding_horizontal(8.0)
        .align_items(Alignment::Center)
        .justify(Justify::SpaceBetween)
        .foreground(PaletteColor::MutedForeground)
        .child(styled(
          &group.name().to_uppercase(),
          TypographyStyle::Caption,
          |style| {
            style.letter_spacing = 1.2;
            style.weight = FontWeight::Medium;
          },
        ))
        .child(Text::new(&pages.len().to_string()).variant(TypographyStyle::Caption)),
    );
    for page in pages {
      let on = page.path == current;
      let mut row = Row::new()
        .width(FILL)
        .height(32.0)
        .padding_horizontal(8.0)
        .align_items(Alignment::Center)
        .rounded(RadiusSize::Md)
        .cursor(CursorIcon::Pointer)
        .child(styled(page.name, TypographyStyle::Body, |style| {
          if on {
            style.weight = FontWeight::Medium;
          }
        }));
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
    groups = groups.child(muted("No page with that name.", TypographyStyle::Body).width(FILL));
  }
  ScrollVertical::new(groups)
    .width(SIDEBAR)
    .border_right(Border::inside(BorderSize::Sm, PaletteColor::Border))
}
