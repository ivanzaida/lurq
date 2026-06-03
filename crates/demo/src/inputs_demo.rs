use lurq::{
  app::{component::Component, ctx::Ctx},
  core::Signal,
  images::ImageData,
  layout::{
    Alignment,
    layout_kind::Justify,
    text_style::{FontWeight, TextStyle},
  },
  node::{CursorIcon, Element, color::Color, dimension::Dimension},
};

use crate::style::{BORDER, DemoTheme, ThemePalette, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;
const PANEL_RADIUS: f32 = 6.0;

pub(crate) struct InputsDemo {
  name: Signal<String>,
  email: Signal<String>,
  hash: Signal<String>,
  scroll_sample: Signal<String>,
  notes: Signal<String>,
  limited_notes: Signal<String>,
  notifications: Signal<bool>,
  beta_access: Signal<bool>,
  volume: Signal<i32>,
  priority: Signal<i32>,
  styled_gain: Signal<i32>,
}

impl Component for InputsDemo {
  type Props = DemoTheme;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      name: ctx.signal("Lol Kek".to_owned()),
      email: ctx.signal(String::new()),
      hash: ctx.signal(String::new()),
      scroll_sample: ctx.signal("A long single-line value that should scroll inside a narrow input".to_owned()),
      notes: ctx.signal("Line one\nLine two".to_owned()),
      limited_notes: ctx.signal("First row\nSecond row\nThird row\nFourth row\nFifth row".to_owned()),
      notifications: ctx.signal(true),
      beta_access: ctx.signal(false),
      volume: ctx.signal(42),
      priority: ctx.signal(3),
      styled_gain: ctx.signal(68),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let palette = ctx.props::<DemoTheme>().palette();

    lurq::components::Column::new()
      .spacing(24.0)
      .child(text("Inputs", 28.0, FontWeight::Bold, palette.text).width(FILL_WIDTH))
      .child(section_title("Text Fields", palette))
      .child(text_fields_card(
        self.name.clone(),
        self.email.clone(),
        self.hash.clone(),
        self.scroll_sample.clone(),
        self.notes.clone(),
        self.limited_notes.clone(),
        palette,
      ))
      .child(section_title("Selection", palette))
      .child(selection_card(
        self.notifications.clone(),
        self.beta_access.clone(),
        palette,
      ))
      .child(section_title("Sliders", palette))
      .child(sliders_card(
        self.volume.clone(),
        self.priority.clone(),
        self.styled_gain.clone(),
        palette,
      ))
      .child(summary_card(
        self.name.clone(),
        self.email.clone(),
        self.hash.clone(),
        self.scroll_sample.clone(),
        self.notes.clone(),
        self.limited_notes.clone(),
        self.notifications.clone(),
        self.beta_access.clone(),
        self.volume.clone(),
        self.priority.clone(),
        self.styled_gain.clone(),
        palette,
      ))
      .padding(CONTENT_PAD)
      .width(FILL_WIDTH)
      .background(palette.bg)
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
    .background(palette.surface)
    .border_inside(1.0, Color::from_hex(palette.border))
    .rounded(CARD_RADIUS)
}

fn text_fields_card(
  name: Signal<String>,
  email: Signal<String>,
  hash: Signal<String>,
  scroll_sample: Signal<String>,
  notes: Signal<String>,
  limited_notes: Signal<String>,
  palette: ThemePalette,
) -> Element {
  let hash_preview = preview_text(&hash.get());
  let scroll_preview = preview_text(&scroll_sample.get());
  let notes_preview = preview_text(&notes.get());
  let limited_notes_preview = preview_text(&limited_notes.get());

  card_frame(palette)
    .spacing(16.0)
    .child(
      lurq::components::Row::new()
        .spacing(18.0)
        .child(field_stack("Name", text_input(name.clone(), "Display name", palette)).flex(1.0))
        .child(field_stack("Email", text_input(email.clone(), "name@example.com", palette)).flex(1.0))
        .width(FILL_WIDTH),
    )
    .child(field_stack("Styled hash", styled_hash_input(hash.clone())).width(FILL_WIDTH))
    .child(
      lurq::components::Row::new()
        .spacing(18.0)
        .align_items(Alignment::Start)
        .child(
          field_stack(
            "Scroll",
            scroll_text_input(scroll_sample.clone(), "Single line", palette),
          )
          .width(220.0),
        )
        .child(field_stack("Multiline", multiline_text_input(notes.clone(), "Notes", palette)).flex(1.0))
        .width(FILL_WIDTH),
    )
    .child(
      field_stack(
        "Rows 3-4",
        rows_text_input(limited_notes.clone(), "Capped rows", palette),
      )
      .width(FILL_WIDTH),
    )
    .child(
      lurq::components::Row::new()
        .spacing(12.0)
        .child(value_pill("name", display_text(&name.get()), palette))
        .child(value_pill("email", display_text(&email.get()), palette))
        .child(value_pill("hash", &hash_preview, palette))
        .child(value_pill("scroll", &scroll_preview, palette))
        .child(value_pill("notes", &notes_preview, palette))
        .child(value_pill("rows", &limited_notes_preview, palette))
        .width(FILL_WIDTH),
    )
    .padding(24.0)
    .into()
}

fn text_input(value: Signal<String>, placeholder: &str, palette: ThemePalette) -> Element {
  lurq::components::TextInput::new(value)
    .placeholder(placeholder)
    .single_line()
    .width(FILL_WIDTH)
    .padding_horizontal(12.0)
    .padding_vertical(6.0)
    .background("#ffffff")
    .border_inside(1.0, Color::from_hex(palette.border))
    .rounded(PANEL_RADIUS)
    .cursor(CursorIcon::Text)
    .focused(move |style| style.border_inside(2.0, Color::from_hex(palette.primary)))
    .into()
}

fn scroll_text_input(value: Signal<String>, placeholder: &str, palette: ThemePalette) -> Element {
  lurq::components::TextInput::new(value)
    .placeholder(placeholder)
    .single_line()
    .height(38.0)
    .width(FILL_WIDTH)
    .padding_horizontal(12.0)
    .padding_vertical(6.0)
    .background("#ffffff")
    .border_inside(1.0, Color::from_hex(palette.border))
    .rounded(PANEL_RADIUS)
    .cursor(CursorIcon::Text)
    .focused(move |style| style.border_inside(2.0, Color::from_hex(palette.primary)))
    .into()
}

fn styled_hash_input(value: Signal<String>) -> Element {
  let value_style = TextStyle {
    font_size: 13.0,
    weight: FontWeight::Medium,
    color: Color::from_hex("#e5e7eb"),
    caret_color: Some(Color::from_hex("#38bdf8").into()),
    ..TextStyle::default()
  };
  let placeholder_style = TextStyle {
    font_size: 13.0,
    weight: FontWeight::Medium,
    color: Color::from_hex("#94a3b8"),
    ..TextStyle::default()
  };

  lurq::components::TextInput::styled(value, value_style)
    .width(Dimension::Pct(100.0))
    .height(40.0)
    .padding_horizontal(10.0)
    .rounded(5.0)
    .background("#101215")
    .border_inside(1.0, Color::from_hex(BORDER))
    .placeholder("a3f1b2c4d5e691cc...")
    .placeholder_style(placeholder_style)
    .single_line()
    .cursor(CursorIcon::Text)
    .into()
}

fn multiline_text_input(value: Signal<String>, placeholder: &str, palette: ThemePalette) -> Element {
  lurq::components::TextInput::new(value)
    .placeholder(placeholder)
    .textarea()
    .width(FILL_WIDTH)
    .padding_horizontal(12.0)
    .padding_vertical(8.0)
    .background("#ffffff")
    .border_inside(1.0, Color::from_hex(palette.border))
    .rounded(PANEL_RADIUS)
    .cursor(CursorIcon::Text)
    .focused(move |style| style.border_inside(2.0, Color::from_hex(palette.primary)))
    .into()
}

fn rows_text_input(value: Signal<String>, placeholder: &str, palette: ThemePalette) -> Element {
  lurq::components::TextInput::new(value)
    .placeholder(placeholder)
    .rows(3, 4)
    .width(FILL_WIDTH)
    .padding_horizontal(12.0)
    .padding_vertical(8.0)
    .background("#ffffff")
    .border_inside(1.0, Color::from_hex(palette.border))
    .rounded(PANEL_RADIUS)
    .cursor(CursorIcon::Text)
    .focused(move |style| style.border_inside(2.0, Color::from_hex(palette.primary)))
    .into()
}

fn field_stack(label: &str, input: Element) -> lurq::components::Column {
  lurq::components::Column::new()
    .spacing(6.0)
    .child(text(label, 12.0, FontWeight::Bold, "#94a3b8"))
    .child(input)
}

fn selection_card(notifications: Signal<bool>, beta_access: Signal<bool>, palette: ThemePalette) -> Element {
  card_frame(palette)
    .spacing(14.0)
    .child(checkbox_row(
      "Notifications",
      "Account updates",
      notifications.clone(),
      palette,
    ))
    .child(checkbox_row(
      "Beta access",
      "Early features",
      beta_access.clone(),
      palette,
    ))
    .child(
      lurq::components::Row::new()
        .spacing(12.0)
        .child(value_pill("notifications", bool_label(notifications.get()), palette))
        .child(value_pill("beta", bool_label(beta_access.get()), palette))
        .width(FILL_WIDTH),
    )
    .padding(24.0)
    .into()
}

fn checkbox_row(label: &str, detail: &str, value: Signal<bool>, palette: ThemePalette) -> Element {
  lurq::components::Row::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(
      lurq::components::Checkbox::new(value)
        .size(20.0, 20.0)
        .background("#ffffff")
        .border_inside(1.0, Color::from_hex(palette.border))
        .rounded(4.0)
        .cursor(CursorIcon::Pointer)
        .focused(move |style| style.border_inside(2.0, Color::from_hex(palette.primary))),
    )
    .child(
      lurq::components::Column::new()
        .spacing(2.0)
        .child(text(label, 14.0, FontWeight::Bold, palette.text))
        .child(text(detail, 12.0, FontWeight::Normal, palette.text_muted)),
    )
    .height(42.0)
    .width(FILL_WIDTH)
    .into()
}

fn sliders_card(
  volume: Signal<i32>,
  priority: Signal<i32>,
  styled_gain: Signal<i32>,
  palette: ThemePalette,
) -> Element {
  card_frame(palette)
    .spacing(18.0)
    .child(slider_row("Volume", volume.clone(), 0, 100, palette))
    .child(slider_row("Priority", priority.clone(), 1, 5, palette))
    .child(styled_slider_row("Styled", styled_gain.clone(), palette))
    .child(
      lurq::components::Row::new()
        .spacing(12.0)
        .child(value_pill("volume", &volume.get().to_string(), palette))
        .child(value_pill("priority", &priority.get().to_string(), palette))
        .child(value_pill("styled", &styled_gain.get().to_string(), palette))
        .width(FILL_WIDTH),
    )
    .padding(24.0)
    .into()
}

fn slider_row(label: &str, value: Signal<i32>, min: i32, max: i32, palette: ThemePalette) -> Element {
  let current = value.get();
  lurq::components::Column::new()
    .spacing(8.0)
    .child(
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .child(text(label, 14.0, FontWeight::Bold, palette.text))
        .child(lurq::components::Spacer::new().flex(1.0))
        .child(text(&current.to_string(), 12.0, FontWeight::Medium, palette.accent)),
    )
    .child(
      lurq::components::Slider::new(value)
        .range(min, max)
        .height(18.0)
        .width(FILL_WIDTH)
        .background("#cbd5e1")
        .rounded(9.0)
        .cursor(CursorIcon::Pointer)
        .focused(move |style| style.border_inside(2.0, Color::from_hex(palette.primary))),
    )
    .width(FILL_WIDTH)
    .into()
}

fn styled_slider_row(label: &str, value: Signal<i32>, palette: ThemePalette) -> Element {
  let current = value.get();
  let track_image = slider_texture(palette.primary, palette.accent, 24, 1);
  let track_hover_image = slider_texture(palette.primary_hover, "#22c55e", 24, 1);
  let thumb_image = slider_texture("#f97316", "#facc15", 12, 12);
  let thumb_hover_image = slider_texture("#fb7185", "#38bdf8", 14, 14);

  lurq::components::Column::new()
    .spacing(8.0)
    .child(
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .child(text(label, 14.0, FontWeight::Bold, palette.text))
        .child(lurq::components::Spacer::new().flex(1.0))
        .child(text(&current.to_string(), 12.0, FontWeight::Medium, palette.accent)),
    )
    .child(
      lurq::components::Slider::new(value)
        .range(0, 100)
        .height(34.0)
        .width(FILL_WIDTH)
        .track(|style| {
          style
            .size(220.0, 2.0)
            .background(palette.surface_dark)
            .background_image(track_image)
            .background_cover()
            .rounded(1.0)
            .border_center(1.0, Color::from_hex(palette.border))
        })
        .track_hovered(|style| {
          style
            .height(4.0)
            .background_image(track_hover_image)
            .background_cover()
            .border_center(1.0, Color::from_hex(palette.primary_hover))
        })
        .thumb(|style| {
          style
            .size(12.0, 12.0)
            .background("#f97316")
            .background_image(thumb_image)
            .background_cover()
            .rounded(6.0)
            .border_inside(2.0, Color::from_hex("#0f172a"))
        })
        .thumb_hovered(|style| {
          style
            .size(14.0, 14.0)
            .background_image(thumb_hover_image)
            .background_cover()
            .rounded(7.0)
            .border_inside(2.0, Color::from_hex("#f8fafc"))
        })
        .cursor(CursorIcon::Pointer)
        .focused(move |style| style.border_inside(2.0, Color::from_hex(palette.primary))),
    )
    .width(FILL_WIDTH)
    .into()
}

fn summary_card(
  name: Signal<String>,
  email: Signal<String>,
  hash: Signal<String>,
  scroll_sample: Signal<String>,
  notes: Signal<String>,
  limited_notes: Signal<String>,
  notifications: Signal<bool>,
  beta_access: Signal<bool>,
  volume: Signal<i32>,
  priority: Signal<i32>,
  styled_gain: Signal<i32>,
  palette: ThemePalette,
) -> Element {
  let hash_preview = preview_text(&hash.get());
  let scroll_preview = preview_text(&scroll_sample.get());
  let notes_preview = preview_text(&notes.get());
  let limited_notes_preview = preview_text(&limited_notes.get());

  card_frame(palette)
    .spacing(8.0)
    .child(text("State Snapshot", 16.0, FontWeight::Bold, palette.text))
    .child(summary_row("name", display_text(&name.get()), palette))
    .child(summary_row("email", display_text(&email.get()), palette))
    .child(summary_row("hash", &hash_preview, palette))
    .child(summary_row("scroll", &scroll_preview, palette))
    .child(summary_row("notes", &notes_preview, palette))
    .child(summary_row("rows", &limited_notes_preview, palette))
    .child(summary_row("notifications", bool_label(notifications.get()), palette))
    .child(summary_row("beta", bool_label(beta_access.get()), palette))
    .child(summary_row("volume", &volume.get().to_string(), palette))
    .child(summary_row("priority", &priority.get().to_string(), palette))
    .child(summary_row("styled", &styled_gain.get().to_string(), palette))
    .padding(20.0)
    .into()
}

fn summary_row(label: &str, value: &str, palette: ThemePalette) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .child(text(label, 12.0, FontWeight::Medium, palette.text_muted).width(110.0))
    .child(text(value, 12.0, FontWeight::Normal, palette.text).nowrap())
    .width(FILL_WIDTH)
    .into()
}

fn value_pill(label: &str, value: &str, palette: ThemePalette) -> Element {
  lurq::components::Row::new()
    .spacing(6.0)
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 11.0, FontWeight::Medium, palette.text_muted))
    .child(text(value, 11.0, FontWeight::Bold, palette.text).nowrap())
    .height(28.0)
    .padding_horizontal(10.0)
    .padding_vertical(0.0)
    .background(palette.bg)
    .border_inside(1.0, Color::from_hex(palette.border))
    .rounded(PANEL_RADIUS)
    .into()
}

fn display_text(value: &str) -> &str {
  if value.is_empty() { "<empty>" } else { value }
}

fn preview_text(value: &str) -> String {
  let normalized = value.replace(['\r', '\n'], " / ");
  const MAX_CHARS: usize = 42;
  if normalized.chars().count() <= MAX_CHARS {
    return normalized;
  }

  let mut preview = normalized.chars().take(MAX_CHARS).collect::<String>();
  preview.push_str("...");
  preview
}

fn slider_texture(start: &str, end: &str, width: u32, height: u32) -> ImageData {
  let start = Color::from_hex(start);
  let end = Color::from_hex(end);
  let mut pixels = Vec::with_capacity((width * height * 4) as usize);

  for y in 0..height {
    for x in 0..width {
      let t = if width <= 1 { 0.0 } else { x as f32 / (width - 1) as f32 };
      let stripe = if y % 2 == 0 { 0 } else { 12 };
      pixels.push(lerp_channel(start.r(), end.r(), t).saturating_add(stripe));
      pixels.push(lerp_channel(start.g(), end.g(), t).saturating_add(stripe));
      pixels.push(lerp_channel(start.b(), end.b(), t).saturating_add(stripe));
      pixels.push(255);
    }
  }

  ImageData::from_rgba(pixels, width, height)
}

fn lerp_channel(start: u8, end: u8, t: f32) -> u8 {
  (start as f32 + (end as f32 - start as f32) * t).round() as u8
}

fn bool_label(value: bool) -> &'static str {
  if value { "on" } else { "off" }
}
