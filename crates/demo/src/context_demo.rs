use lurq::{
  app::{component::Component, ctx::Ctx},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, color::Color, dimension::Dimension},
};

use crate::style::{ACCENT, BG, BORDER, PRIMARY, SURFACE, TEXT, TEXT_MUTED, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;
const PANEL_RADIUS: f32 = 6.0;

#[derive(Clone)]
struct LocaleContext {
  locale: &'static str,
  greeting: &'static str,
  date_format: &'static str,
}

pub(crate) struct ContextDemo {
  dark_theme: Signal<bool>,
}

impl Component for ContextDemo {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      dark_theme: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let is_dark = self.dark_theme.get();
    let theme_signal = self.dark_theme.clone();

    lurq::components::Column::new()
      .spacing(24.0)
      .child(text("Context & Themes", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
      .child(section_title("Theme Switching"))
      .child(theme_card(is_dark, theme_signal))
      .child(section_title("Context Provide/Consume"))
      .child(ctx.mount::<LocaleProvider>(()))
      .pad(CONTENT_PAD)
      .width(FILL_WIDTH)
      .fill(BG)
  }
}

struct LocaleProvider;

impl Component for LocaleProvider {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.provide(LocaleContext {
      locale: "en-US",
      greeting: "Hello!",
      date_format: "MM/DD/YYYY",
    });

    card_frame()
      .spacing(12.0)
      .child(text("Provider: locale = \"en-US\"", 14.0, FontWeight::Normal, TEXT))
      .child(ctx.mount::<LocaleConsumerA>(()))
      .child(ctx.mount::<LocaleConsumerB>(()))
      .pad(24.0)
  }
}

struct LocaleConsumerA;

impl Component for LocaleConsumerA {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let locale = ctx.use_context::<LocaleContext>().unwrap_or(LocaleContext {
      locale: "unknown",
      greeting: "Unavailable",
      date_format: "Unavailable",
    });

    context_child(
      "Child A",
      &format!("locale: \"{}\" - Greeting: \"{}\"", locale.locale, locale.greeting),
    )
  }
}

struct LocaleConsumerB;

impl Component for LocaleConsumerB {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let locale = ctx.use_context::<LocaleContext>().unwrap_or(LocaleContext {
      locale: "unknown",
      greeting: "Unavailable",
      date_format: "Unavailable",
    });

    context_child(
      "Child B",
      &format!(
        "locale: \"{}\" - Date format: \"{}\"",
        locale.locale, locale.date_format
      ),
    )
  }
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn card_frame() -> lurq::components::Column {
  lurq::components::Column::new()
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
}

fn theme_card(is_dark: bool, dark_theme: Signal<bool>) -> Element {
  card_frame()
    .spacing(16.0)
    .child(
      lurq::components::Row::new()
        .spacing(12.0)
        .align_items(Alignment::Center)
        .child(text(
          if is_dark {
            "Current theme: Dark"
          } else {
            "Current theme: Light"
          },
          14.0,
          FontWeight::Normal,
          TEXT,
        ))
        .child(theme_toggle(dark_theme))
        .width(FILL_WIDTH),
    )
    .child(
      lurq::components::Row::new()
        .spacing(20.0)
        .child(theme_preview(
          "Dark Theme",
          "bg: #0F172A  surface: #1E293B",
          BG,
          TEXT,
          TEXT_MUTED,
          PRIMARY,
          BORDER,
        ))
        .child(theme_preview(
          "Light Theme",
          "bg: #F8FAFC  surface: #FFFFFF",
          "#f8fafc",
          BG,
          "#94a3b8",
          "#2563eb",
          "#e2e8f0",
        ))
        .width(FILL_WIDTH),
    )
    .pad(24.0)
    .into()
}

fn theme_toggle(dark_theme: Signal<bool>) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text("Toggle Theme", 12.0, FontWeight::Normal, "#ffffff"))
    .size(120.0, 32.0)
    .fill(PRIMARY)
    .rounded(PANEL_RADIUS)
    .cursor(CursorIcon::Pointer)
    .hovered(|style| style.fill("#60a5fa"))
    .active(|style| style.fill("#2563eb"))
    .on_click(move |_| dark_theme.set(!dark_theme.get()))
    .into()
}

fn theme_preview(
  title: &str,
  body: &str,
  background: &str,
  title_color: &str,
  body_color: &str,
  button_color: &str,
  border_color: &str,
) -> Element {
  lurq::components::Column::new()
    .spacing(6.0)
    .child(text(title, 16.0, FontWeight::Bold, title_color))
    .child(text(body, 11.0, FontWeight::Normal, body_color).nowrap())
    .child(
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(text("Primary", 11.0, FontWeight::Normal, "#ffffff"))
        .size(100.0, 28.0)
        .fill(button_color)
        .rounded(4.0),
    )
    .height(150.0)
    .pad(16.0)
    .flex(1.0)
    .fill(background)
    .border_inside(1.0, Color::from_hex(border_color))
    .rounded(CARD_RADIUS)
    .into()
}

fn context_child(name: &str, value: &str) -> Element {
  lurq::components::Column::new()
    .spacing(2.0)
    .child(text(name, 12.0, FontWeight::Bold, ACCENT))
    .child(text(value, 12.0, FontWeight::Normal, TEXT).nowrap())
    .pad_xy(12.0, 8.0)
    .width(FILL_WIDTH)
    .fill(BG)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(PANEL_RADIUS)
    .into()
}
