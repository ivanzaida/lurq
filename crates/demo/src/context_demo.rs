use lurq::{
  app::{component::Component, ctx::Ctx},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, color::Color, dimension::Dimension},
};

use crate::style::{BG, BORDER, DemoTheme, PRIMARY, TEXT, TEXT_MUTED, ThemePalette, text};

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

#[derive(Clone, Copy, PartialEq, Eq)]
enum LocaleMode {
  EnUs,
  EnGb,
}

impl LocaleMode {
  fn toggle(self) -> Self {
    match self {
      Self::EnUs => Self::EnGb,
      Self::EnGb => Self::EnUs,
    }
  }

  fn context(self) -> LocaleContext {
    match self {
      Self::EnUs => LocaleContext {
        locale: "en-US",
        greeting: "Hello!",
        date_format: "MM/DD/YYYY",
      },
      Self::EnGb => LocaleContext {
        locale: "en-GB",
        greeting: "Hello there!",
        date_format: "DD/MM/YYYY",
      },
    }
  }
}

#[derive(Clone, lurq::DevtoolsInspectable)]
pub(crate) struct ContextDemoProps {
  pub(crate) theme: Signal<DemoTheme>,
}

impl PartialEq for ContextDemoProps {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

pub(crate) struct ContextDemo {
  theme: Signal<DemoTheme>,
  locale: Signal<LocaleMode>,
}

impl Component for ContextDemo {
  type Props = ContextDemoProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      theme: ctx.props::<ContextDemoProps>().theme.clone(),
      locale: ctx.signal(LocaleMode::EnUs),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let theme = self.theme.get();
    let palette = theme.palette();

    lurq::components::Column::new()
      .spacing(24.0)
      .child(text("Context & Themes", 28.0, FontWeight::Bold, palette.text).width(FILL_WIDTH))
      .child(section_title("Theme Switching", palette))
      .child(theme_card(theme, self.theme.clone(), palette))
      .child(section_title("Context Provide/Consume", palette))
      .child(ctx.mount::<LocaleProvider>(LocaleProviderProps {
        locale: self.locale.clone(),
        theme,
      }))
      .padding(CONTENT_PAD)
      .width(FILL_WIDTH)
      .fill(palette.bg)
  }
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct LocaleProviderProps {
  locale: Signal<LocaleMode>,
  theme: DemoTheme,
}

impl PartialEq for LocaleProviderProps {
  fn eq(&self, other: &Self) -> bool {
    self.theme == other.theme
  }
}

struct LocaleProvider;

impl Component for LocaleProvider {
  type Props = LocaleProviderProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<LocaleProviderProps>().clone();
    let locale_mode = props.locale.get();
    let locale = locale_mode.context();
    let palette = props.theme.palette();
    ctx.provide(locale.clone());

    card_frame(palette)
      .spacing(12.0)
      .child(
        lurq::components::Row::new()
          .spacing(12.0)
          .align_items(Alignment::Center)
          .child(text(
            &format!("Provider: locale = \"{}\"", locale.locale),
            14.0,
            FontWeight::Normal,
            palette.text,
          ))
          .child(locale_toggle(props.locale, palette))
          .width(FILL_WIDTH),
      )
      .child(ctx.mount::<LocaleConsumerA>(props.theme))
      .child(ctx.mount::<LocaleConsumerB>(props.theme))
      .padding(24.0)
  }
}

struct LocaleConsumerA;

impl Component for LocaleConsumerA {
  type Props = DemoTheme;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let palette = ctx.props::<DemoTheme>().palette();
    let locale = ctx.use_context::<LocaleContext>().unwrap_or(LocaleContext {
      locale: "unknown",
      greeting: "Unavailable",
      date_format: "Unavailable",
    });

    context_child(
      "Child A",
      &format!("locale: \"{}\" - Greeting: \"{}\"", locale.locale, locale.greeting),
      palette,
    )
  }
}

struct LocaleConsumerB;

impl Component for LocaleConsumerB {
  type Props = DemoTheme;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let palette = ctx.props::<DemoTheme>().palette();
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
      palette,
    )
  }
}

fn section_title(label: &str, palette: ThemePalette) -> Element {
  text(label, 18.0, FontWeight::Bold, palette.text)
    .width(FILL_WIDTH)
    .into()
}

fn card_frame(palette: ThemePalette) -> lurq::components::Column {
  lurq::components::Column::new()
    .width(FILL_WIDTH)
    .fill(palette.surface)
    .border_inside(1.0, Color::from_hex(palette.border))
    .rounded(CARD_RADIUS)
}

fn theme_card(theme: DemoTheme, theme_signal: Signal<DemoTheme>, palette: ThemePalette) -> Element {
  card_frame(palette)
    .spacing(16.0)
    .child(
      lurq::components::Row::new()
        .spacing(12.0)
        .align_items(Alignment::Center)
        .child(text(
          &format!("Current app theme: {}", theme.label()),
          14.0,
          FontWeight::Normal,
          palette.text,
        ))
        .child(theme_toggle(theme_signal, palette))
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
    .padding(24.0)
    .into()
}

fn theme_toggle(theme: Signal<DemoTheme>, palette: ThemePalette) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text("Toggle Theme", 12.0, FontWeight::Normal, "#ffffff"))
    .size(120.0, 32.0)
    .fill(palette.primary)
    .rounded(PANEL_RADIUS)
    .cursor(CursorIcon::Pointer)
    .hovered(move |style| style.fill(palette.primary_hover))
    .active(move |style| style.fill(palette.primary_active))
    .on_click(move |_| theme.set(theme.get().toggle()))
    .into()
}

fn locale_toggle(locale: Signal<LocaleMode>, palette: ThemePalette) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text("Switch Locale", 12.0, FontWeight::Normal, "#ffffff"))
    .size(120.0, 32.0)
    .fill(palette.primary)
    .rounded(PANEL_RADIUS)
    .cursor(CursorIcon::Pointer)
    .hovered(move |style| style.fill(palette.primary_hover))
    .active(move |style| style.fill(palette.primary_active))
    .on_click(move |_| locale.set(locale.get().toggle()))
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
    .padding(16.0)
    .flex(1.0)
    .fill(background)
    .border_inside(1.0, Color::from_hex(border_color))
    .rounded(CARD_RADIUS)
    .into()
}

fn context_child(name: &str, value: &str, palette: ThemePalette) -> Element {
  lurq::components::Column::new()
    .spacing(2.0)
    .child(text(name, 12.0, FontWeight::Bold, palette.accent))
    .child(text(value, 12.0, FontWeight::Normal, palette.text).nowrap())
    .padding_horizontal(12.0)
    .padding_vertical(8.0)
    .width(FILL_WIDTH)
    .fill(palette.bg)
    .border_inside(1.0, Color::from_hex(palette.border))
    .rounded(PANEL_RADIUS)
    .into()
}
