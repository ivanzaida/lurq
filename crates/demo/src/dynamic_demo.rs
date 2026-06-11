use lurq::{
  animation::{Animation, Easing, KeyframesId},
  components::{Column, Image, Rect, Row, Stack},
  layout::{Alignment, StackAlignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, color::Color, dimension::Dimension},
};

use crate::style::{ACCENT, BG, BORDER, PRIMARY, SECONDARY, SURFACE, TEXT, TEXT_MUTED, WARNING, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;

const COLOR_CYCLE_KEYFRAMES: KeyframesId = KeyframesId::new(1);
const GROW_SHRINK_KEYFRAMES: KeyframesId = KeyframesId::new(7);
const PULSE_KEYFRAMES: KeyframesId = KeyframesId::new(11);
const ROCK_KEYFRAMES: KeyframesId = KeyframesId::new(12);
const SLIDE_BOUNCE_KEYFRAMES: KeyframesId = KeyframesId::new(13);
const SPIN_KEYFRAMES: KeyframesId = KeyframesId::new(14);
const SPIN_COLOR_KEYFRAMES: KeyframesId = KeyframesId::new(15);

const ANIMATED_IMAGE_ASSETS: &[(&str, &str)] = &[("GIF", "six-seven.gif"), ("WebP", "animated-webp-supported.webp")];

pub(crate) fn dynamic_keyframes_content() -> Element {
  Column::new()
    .spacing(24.0)
    .child(text("Dynamic Keyframes", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
    .child(section_title("All Keyframes"))
    .child(all_keyframes_grid())
    .padding(CONTENT_PAD)
    .width(FILL_WIDTH)
    .background(BG)
    .into()
}

pub(crate) fn dynamic_images_content() -> Element {
  Column::new()
    .spacing(24.0)
    .child(text("Dynamic Images", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
    .child(section_title("Animated Image Resources"))
    .child(animated_images_grid())
    .child(section_title("Animated Background Images"))
    .child(animated_background_grid())
    .padding(CONTENT_PAD)
    .width(FILL_WIDTH)
    .background(BG)
    .into()
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn all_keyframes_grid() -> Element {
  Column::new()
    .spacing(16.0)
    .child(
      Row::new()
        .spacing(16.0)
        .align_items(Alignment::Stretch)
        .child(keyframe_card("Pulse", pulse_sample()))
        .child(keyframe_card("Color Cycle", color_cycle_sample()))
        .child(keyframe_card("Slide Bounce", slide_bounce_sample()))
        .width(FILL_WIDTH),
    )
    .child(
      Row::new()
        .spacing(16.0)
        .align_items(Alignment::Stretch)
        .child(keyframe_card("Grow / Shrink", grow_shrink_sample()))
        .child(keyframe_card("Spin", spin_sample()))
        .child(keyframe_card("Rock", rock_sample()))
        .child(keyframe_card("Spin + Color", spin_color_sample()))
        .width(FILL_WIDTH),
    )
    .padding(24.0)
    .width(FILL_WIDTH)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn keyframe_card(label: &str, content: impl Into<Element>) -> Element {
  Column::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(
      Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(content)
        .height(118.0)
        .width(FILL_WIDTH),
    )
    .child(text(label, 12.0, FontWeight::Bold, TEXT))
    .child(text("registered keyframes", 10.0, FontWeight::Medium, TEXT_MUTED))
    .padding(16.0)
    .flex(1.0)
    .background("#111827")
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(8.0)
    .into()
}

fn pulse_sample() -> Element {
  Rect::new(74.0, 74.0)
    .background(PRIMARY)
    .rounded(12.0)
    .animation(Animation::new(PULSE_KEYFRAMES).duration_ms(2000).linear().infinite())
    .into()
}

fn color_cycle_sample() -> Element {
  Rect::new(74.0, 74.0)
    .background(PRIMARY)
    .rounded(37.0)
    .animation(
      Animation::new(COLOR_CYCLE_KEYFRAMES)
        .duration_ms(3000)
        .linear()
        .infinite(),
    )
    .into()
}

fn slide_bounce_sample() -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(
      Rect::new(48.0, 48.0)
        .background(ACCENT)
        .rounded(8.0)
        .relative(0.0, 0.0)
        .animation(
          Animation::new(SLIDE_BOUNCE_KEYFRAMES)
            .duration_ms(2500)
            .easing(Easing::EASE_IN_OUT)
            .infinite(),
        ),
    )
    .width(260.0)
    .height(74.0)
    .into()
}

fn grow_shrink_sample() -> Element {
  Rect::new(60.0, 40.0)
    .background(SECONDARY)
    .rounded(8.0)
    .animation(
      Animation::new(GROW_SHRINK_KEYFRAMES)
        .duration_ms(2000)
        .linear()
        .infinite(),
    )
    .into()
}

fn spin_sample() -> Element {
  Rect::new(62.0, 62.0)
    .background(ACCENT)
    .rounded(8.0)
    .animation(Animation::new(SPIN_KEYFRAMES).duration_ms(2000).linear().infinite())
    .into()
}

fn rock_sample() -> Element {
  Rect::new(62.0, 62.0)
    .background(WARNING)
    .rounded(8.0)
    .animation(Animation::new(ROCK_KEYFRAMES).duration_ms(1000).linear().infinite())
    .into()
}

fn spin_color_sample() -> Element {
  Rect::new(62.0, 62.0)
    .background(PRIMARY)
    .rounded(31.0)
    .animation(
      Animation::new(SPIN_COLOR_KEYFRAMES)
        .duration_ms(3000)
        .linear()
        .infinite(),
    )
    .into()
}

fn animated_images_grid() -> Element {
  Row::new()
    .spacing(16.0)
    .align_items(Alignment::Stretch)
    .with_children(
      ANIMATED_IMAGE_ASSETS
        .iter()
        .map(|(label, path)| animated_image_card(label, path)),
    )
    .padding(24.0)
    .width(FILL_WIDTH)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn animated_image_card(label: &str, path: &str) -> Element {
  Column::new()
    .spacing(12.0)
    .child(
      Image::from_resource(path)
        .size(Dimension::Pct(100.0), 210.0)
        .rounded(8.0)
        .clip()
        .border_inside(1.0, Color::from_hex(BORDER)),
    )
    .child(text(label, 13.0, FontWeight::Bold, TEXT))
    .child(text(path, 10.0, FontWeight::Medium, TEXT_MUTED))
    .flex(1.0)
    .into()
}

fn animated_background_grid() -> Element {
  Row::new()
    .spacing(16.0)
    .align_items(Alignment::Stretch)
    .with_children(
      ANIMATED_IMAGE_ASSETS
        .iter()
        .map(|(label, path)| animated_background_card(label, path)),
    )
    .padding(24.0)
    .width(FILL_WIDTH)
    .background(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn animated_background_card(label: &str, path: &str) -> Element {
  Column::new()
    .spacing(12.0)
    .child(
      Stack::new()
        .stack_align(StackAlignment::Center)
        .size(Dimension::Pct(100.0), 220.0)
        .background("#0B1220")
        .background_image(path)
        .child(
          Column::new()
            .spacing(3.0)
            .align_items(Alignment::Center)
            .child(text(label, 13.0, FontWeight::Bold, TEXT))
            .child(text("background image", 10.0, FontWeight::Medium, TEXT_MUTED))
            .padding_horizontal(14.0)
            .padding_vertical(10.0)
            .background("#111827")
            .border_inside(1.0, Color::from_hex(BORDER))
            .rounded(8.0),
        )
        .rounded(8.0)
        .clip()
        .border_inside(1.0, Color::from_hex(BORDER)),
    )
    .child(text(path, 10.0, FontWeight::Medium, TEXT_MUTED))
    .flex(1.0)
    .into()
}
