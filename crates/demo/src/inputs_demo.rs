use lurq::{
  app::{component::Component, ctx::Ctx},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, color::Color, dimension::Dimension},
};

use crate::style::{DemoTheme, ThemePalette, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;
const PANEL_RADIUS: f32 = 6.0;

pub(crate) struct InputsDemo {
  name: Signal<String>,
  email: Signal<String>,
  scroll_sample: Signal<String>,
  notes: Signal<String>,
  limited_notes: Signal<String>,
  notifications: Signal<bool>,
  beta_access: Signal<bool>,
  volume: Signal<f32>,
  priority: Signal<f32>,
}

impl Component for InputsDemo {
  type Props = DemoTheme;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      name: ctx.signal("Mira Chen".to_owned()),
      email: ctx.signal(String::new()),
      scroll_sample: ctx.signal("A long single-line value that should scroll inside a narrow input".to_owned()),
      notes: ctx.signal("Line one\nLine two".to_owned()),
      limited_notes: ctx.signal("First row\nSecond row\nThird row\nFourth row\nFifth row".to_owned()),
      notifications: ctx.signal(true),
      beta_access: ctx.signal(false),
      volume: ctx.signal(42.0),
      priority: ctx.signal(3.0),
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
      .child(sliders_card(self.volume.clone(), self.priority.clone(), palette))
      .child(summary_card(
        self.name.clone(),
        self.email.clone(),
        self.scroll_sample.clone(),
        self.notes.clone(),
        self.limited_notes.clone(),
        self.notifications.clone(),
        self.beta_access.clone(),
        self.volume.clone(),
        self.priority.clone(),
        palette,
      ))
      .padding(CONTENT_PAD)
      .width(FILL_WIDTH)
      .fill(palette.bg)
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

fn text_fields_card(
  name: Signal<String>,
  email: Signal<String>,
  scroll_sample: Signal<String>,
  notes: Signal<String>,
  limited_notes: Signal<String>,
  palette: ThemePalette,
) -> Element {
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
    .height(38.0)
    .width(FILL_WIDTH)
    .padding_horizontal(12.0)
    .padding_vertical(6.0)
    .fill("#ffffff")
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
    .fill("#ffffff")
    .border_inside(1.0, Color::from_hex(palette.border))
    .rounded(PANEL_RADIUS)
    .cursor(CursorIcon::Text)
    .focused(move |style| style.border_inside(2.0, Color::from_hex(palette.primary)))
    .into()
}

fn multiline_text_input(value: Signal<String>, placeholder: &str, palette: ThemePalette) -> Element {
  lurq::components::TextInput::new(value)
    .placeholder(placeholder)
    .textarea()
    .width(FILL_WIDTH)
    .padding_horizontal(12.0)
    .padding_vertical(8.0)
    .fill("#ffffff")
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
    .fill("#ffffff")
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
        .fill("#ffffff")
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

fn sliders_card(volume: Signal<f32>, priority: Signal<f32>, palette: ThemePalette) -> Element {
  card_frame(palette)
    .spacing(18.0)
    .child(slider_row("Volume", volume.clone(), 0.0, 100.0, palette))
    .child(slider_row("Priority", priority.clone(), 1.0, 5.0, palette))
    .child(
      lurq::components::Row::new()
        .spacing(12.0)
        .child(value_pill("volume", &format!("{:.0}", volume.get()), palette))
        .child(value_pill("priority", &format!("{:.1}", priority.get()), palette))
        .width(FILL_WIDTH),
    )
    .padding(24.0)
    .into()
}

fn slider_row(label: &str, value: Signal<f32>, min: f32, max: f32, palette: ThemePalette) -> Element {
  let current = value.get();
  lurq::components::Column::new()
    .spacing(8.0)
    .child(
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .child(text(label, 14.0, FontWeight::Bold, palette.text))
        .child(lurq::components::Spacer::new().flex(1.0))
        .child(text(&format!("{current:.1}"), 12.0, FontWeight::Medium, palette.accent)),
    )
    .child(
      lurq::components::Slider::new(value)
        .range(min, max)
        .height(18.0)
        .width(FILL_WIDTH)
        .fill("#cbd5e1")
        .rounded(9.0)
        .cursor(CursorIcon::Pointer)
        .focused(move |style| style.border_inside(2.0, Color::from_hex(palette.primary))),
    )
    .width(FILL_WIDTH)
    .into()
}

fn summary_card(
  name: Signal<String>,
  email: Signal<String>,
  scroll_sample: Signal<String>,
  notes: Signal<String>,
  limited_notes: Signal<String>,
  notifications: Signal<bool>,
  beta_access: Signal<bool>,
  volume: Signal<f32>,
  priority: Signal<f32>,
  palette: ThemePalette,
) -> Element {
  let scroll_preview = preview_text(&scroll_sample.get());
  let notes_preview = preview_text(&notes.get());
  let limited_notes_preview = preview_text(&limited_notes.get());

  card_frame(palette)
    .spacing(8.0)
    .child(text("State Snapshot", 16.0, FontWeight::Bold, palette.text))
    .child(summary_row("name", display_text(&name.get()), palette))
    .child(summary_row("email", display_text(&email.get()), palette))
    .child(summary_row("scroll", &scroll_preview, palette))
    .child(summary_row("notes", &notes_preview, palette))
    .child(summary_row("rows", &limited_notes_preview, palette))
    .child(summary_row("notifications", bool_label(notifications.get()), palette))
    .child(summary_row("beta", bool_label(beta_access.get()), palette))
    .child(summary_row("volume", &format!("{:.0}", volume.get()), palette))
    .child(summary_row("priority", &format!("{:.1}", priority.get()), palette))
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
    .fill(palette.bg)
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

fn bool_label(value: bool) -> &'static str {
  if value { "on" } else { "off" }
}
