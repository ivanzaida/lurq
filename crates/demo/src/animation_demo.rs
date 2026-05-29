use lurq::{
  animation::{AnimatableProperty, AnimatableValue, Animation, Easing, Keyframes, Transition},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{
    CursorIcon, Element,
    color::Color,
    dimension::Dimension,
    transform::{Decomposed, Transform2D},
  },
};

use crate::style::{ACCENT, BG, BORDER, PRIMARY, SECONDARY, SURFACE, TEXT, TEXT_MUTED, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;

pub(crate) fn register_keyframes(tree: &mut lurq::app::Tree) {
  tree.register_keyframes(
    Keyframes::new("pulse")
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
      })
      .frame(0.5, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(0.3));
      })
      .frame(1.0, |f| {
        f.set(AnimatableProperty::Opacity, AnimatableValue::Float(1.0));
      }),
  );

  tree.register_keyframes(
    Keyframes::new("color-cycle")
      .frame(0.0, |f| {
        f.set(
          AnimatableProperty::BackgroundColor,
          AnimatableValue::Color(Color::from_hex("#3b82f6")),
        );
      })
      .frame(0.33, |f| {
        f.set(
          AnimatableProperty::BackgroundColor,
          AnimatableValue::Color(Color::from_hex("#8b5cf6")),
        );
      })
      .frame(0.66, |f| {
        f.set(
          AnimatableProperty::BackgroundColor,
          AnimatableValue::Color(Color::from_hex("#06b6d4")),
        );
      })
      .frame(1.0, |f| {
        f.set(
          AnimatableProperty::BackgroundColor,
          AnimatableValue::Color(Color::from_hex("#3b82f6")),
        );
      }),
  );

  tree.register_keyframes(
    Keyframes::new("slide-bounce")
      .frame(0.0, |f| {
        f.set(AnimatableProperty::OffsetX, AnimatableValue::Float(0.0));
      })
      .frame(0.5, |f| {
        f.set(AnimatableProperty::OffsetX, AnimatableValue::Float(200.0));
        f.easing(Easing::EASE_OUT);
      })
      .frame(1.0, |f| {
        f.set(AnimatableProperty::OffsetX, AnimatableValue::Float(0.0));
        f.easing(Easing::EASE_IN);
      }),
  );

  tree.register_keyframes(
    Keyframes::new("spin")
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Transform, Decomposed::IDENTITY.with_rotate(0.0));
      })
      .frame(1.0, |f| {
        f.set(
          AnimatableProperty::Transform,
          Decomposed::IDENTITY.with_rotate(std::f32::consts::TAU),
        );
      }),
  );

  tree.register_keyframes(
    Keyframes::new("rock")
      .frame(0.0, |f| {
        f.set(
          AnimatableProperty::Transform,
          Decomposed::IDENTITY.with_rotate_deg(-15.0),
        );
      })
      .frame(0.5, |f| {
        f.set(
          AnimatableProperty::Transform,
          Decomposed::IDENTITY.with_rotate_deg(15.0),
        );
      })
      .frame(1.0, |f| {
        f.set(
          AnimatableProperty::Transform,
          Decomposed::IDENTITY.with_rotate_deg(-15.0),
        );
      }),
  );

  tree.register_keyframes(
    Keyframes::new("spin-color")
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Transform, Decomposed::IDENTITY.with_rotate(0.0));
        f.set(AnimatableProperty::BackgroundColor, Color::from_hex("#3b82f6"));
      })
      .frame(0.33, |f| {
        f.set(
          AnimatableProperty::Transform,
          Decomposed::IDENTITY.with_rotate(std::f32::consts::TAU / 3.0),
        );
        f.set(AnimatableProperty::BackgroundColor, Color::from_hex("#8b5cf6"));
      })
      .frame(0.66, |f| {
        f.set(
          AnimatableProperty::Transform,
          Decomposed::IDENTITY.with_rotate(std::f32::consts::TAU * 2.0 / 3.0),
        );
        f.set(AnimatableProperty::BackgroundColor, Color::from_hex("#06b6d4"));
      })
      .frame(1.0, |f| {
        f.set(
          AnimatableProperty::Transform,
          Decomposed::IDENTITY.with_rotate(std::f32::consts::TAU),
        );
        f.set(AnimatableProperty::BackgroundColor, Color::from_hex("#3b82f6"));
      }),
  );

  tree.register_keyframes(
    Keyframes::new("grow-shrink")
      .frame(0.0, |f| {
        f.set(AnimatableProperty::Width, AnimatableValue::Float(60.0));
        f.set(AnimatableProperty::Height, AnimatableValue::Float(40.0));
      })
      .frame(0.5, |f| {
        f.set(AnimatableProperty::Width, AnimatableValue::Float(200.0));
        f.set(AnimatableProperty::Height, AnimatableValue::Float(60.0));
      })
      .frame(1.0, |f| {
        f.set(AnimatableProperty::Width, AnimatableValue::Float(60.0));
        f.set(AnimatableProperty::Height, AnimatableValue::Float(40.0));
      }),
  );
}

pub(crate) fn animation_content() -> Element {
  lurq::components::Column::new()
    .spacing(24.0)
    .child(text("Animation", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
    .child(section_title("Hover Transitions"))
    .child(text(
      "Move your mouse over the boxes to see smooth property transitions.",
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
    .child(hover_transitions())
    .child(section_title("Easing Functions"))
    .child(text(
      "All boxes transition on hover with different easing curves.",
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
    .child(easing_comparison())
    .child(section_title("Keyframe Animations"))
    .child(text(
      "Continuous animations using registered keyframe sequences.",
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
    .child(keyframe_demos())
    .child(section_title("Opacity"))
    .child(text(
      "Static opacity levels applied to identical elements.",
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
    .child(opacity_showcase())
    .child(section_title("Combined"))
    .child(text(
      "Transitions and keyframe animations working together on the same node.",
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
    .child(combined_demo())
    .child(section_title("Transforms"))
    .child(text(
      "GPU-accelerated 2D transforms: rotate, scale, skew. Paint-only — no layout impact.",
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
    .child(transform_demos())
    .pad(CONTENT_PAD)
    .width(FILL_WIDTH)
    .fill(BG)
    .into()
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

// --- Hover Transitions ---

fn hover_transitions() -> Element {
  lurq::components::Column::new()
    .spacing(16.0)
    .child(
      lurq::components::Row::new()
        .spacing(16.0)
        .child(transition_box(
          "Color",
          "#3b82f6",
          |s| s.fill("#ef4444"),
          Transition::background_color().duration_ms(400),
        ))
        .child(transition_box(
          "Border",
          "#1e293b",
          |s| s.border_inside(3.0, Color::from_hex("#22c55e")),
          Transition::all().duration_ms(300),
        ))
        .child(transition_box(
          "Radius",
          "#8b5cf6",
          |s| s.rounded(32.0),
          Transition::all().duration_ms(500).easing(Easing::EASE_OUT),
        ))
        .child(transition_box(
          "Opacity",
          "#06b6d4",
          |s| s.background(Color::new(6, 182, 212, 80)),
          Transition::background_color()
            .duration_ms(600)
            .easing(Easing::EASE_IN_OUT),
        ))
        .child(width_box())
        .child(all_props_box()),
    )
    .pad(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn transition_box(
  label: &str,
  base_color: &str,
  hovered_style: impl FnOnce(lurq::node::Style) -> lurq::node::Style,
  transition: Transition,
) -> Element {
  lurq::components::Column::new()
    .spacing(8.0)
    .align_items(Alignment::Center)
    .child(
      lurq::components::Rect::new(80.0, 60.0)
        .fill(base_color)
        .rounded(8.0)
        .border_inside(2.0, Color::new(255, 255, 255, 30))
        .transition(transition)
        .hovered(hovered_style)
        .cursor(CursorIcon::Pointer),
    )
    .child(text(label, 11.0, FontWeight::Medium, TEXT_MUTED))
    .into()
}

fn width_box() -> Element {
  lurq::components::Column::new()
    .spacing(8.0)
    .align_items(Alignment::Center)
    .child(
      lurq::components::Rect::new(80.0, 60.0)
        .fill("#f59e0b")
        .rounded(8.0)
        .border_inside(2.0, Color::new(255, 255, 255, 30))
        .transition(Transition::all().duration_ms(400).easing(Easing::EASE_IN_OUT))
        .hovered(|s| s.size(130.0, 60.0))
        .cursor(CursorIcon::Pointer),
    )
    .child(text("Width", 11.0, FontWeight::Medium, TEXT_MUTED))
    .into()
}

fn all_props_box() -> Element {
  lurq::components::Column::new()
    .spacing(8.0)
    .align_items(Alignment::Center)
    .child(
      lurq::components::Rect::new(80.0, 60.0)
        .fill("#3b82f6")
        .rounded(4.0)
        .border_inside(2.0, Color::new(255, 255, 255, 30))
        .transition(
          Transition::background_color()
            .duration_ms(400)
            .easing(Easing::EASE_IN_OUT),
        )
        .transition(Transition::all().duration_ms(500).easing(Easing::EASE_OUT))
        .hovered(|s| {
          s.fill("#ef4444")
            .rounded(28.0)
            .border_inside(3.0, Color::from_hex("#22c55e"))
            .size(130.0, 70.0)
        })
        .cursor(CursorIcon::Pointer),
    )
    .child(text("All Props", 11.0, FontWeight::Medium, TEXT_MUTED))
    .into()
}

// --- Easing Comparison ---

fn easing_comparison() -> Element {
  lurq::components::Column::new()
    .spacing(12.0)
    .child(easing_row("Linear", Easing::Linear))
    .child(easing_row("Ease", Easing::EASE))
    .child(easing_row("Ease-In", Easing::EASE_IN))
    .child(easing_row("Ease-Out", Easing::EASE_OUT))
    .child(easing_row("Ease-In-Out", Easing::EASE_IN_OUT))
    .pad(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn easing_row(label: &str, easing: Easing) -> Element {
  lurq::components::Row::new()
    .spacing(16.0)
    .align_items(Alignment::Center)
    .child(text(label, 11.0, FontWeight::Medium, TEXT_MUTED).width(90.0))
    .child(
      lurq::components::Rect::new(120.0, 32.0)
        .fill(PRIMARY)
        .rounded(6.0)
        .transition(Transition::background_color().duration_ms(800).easing(easing))
        .transition(Transition::all().duration_ms(800).easing(easing))
        .hovered(|s| s.fill("#22c55e").size(240.0, 32.0))
        .cursor(CursorIcon::Pointer),
    )
    .width(FILL_WIDTH)
    .height(40.0)
    .into()
}

// --- Keyframe Animations ---

fn keyframe_demos() -> Element {
  lurq::components::Row::new()
    .spacing(24.0)
    .align_items(Alignment::Start)
    .child(keyframe_card(
      "Pulse",
      lurq::components::Rect::new(60.0, 60.0)
        .fill(PRIMARY)
        .rounded(8.0)
        .animation(Animation::new("pulse").duration_ms(2000).linear().infinite()),
    ))
    .child(keyframe_card(
      "Color Cycle",
      lurq::components::Rect::new(60.0, 60.0)
        .fill(PRIMARY)
        .rounded(30.0)
        .animation(Animation::new("color-cycle").duration_ms(3000).linear().infinite()),
    ))
    .child(keyframe_card(
      "Slide",
      lurq::components::Rect::new(40.0, 40.0)
        .fill(ACCENT)
        .rounded(6.0)
        .relative(0.0, 0.0)
        .animation(
          Animation::new("slide-bounce")
            .duration_ms(2500)
            .easing(Easing::EASE_IN_OUT)
            .infinite(),
        ),
    ))
    .child(keyframe_card(
      "Grow/Shrink",
      lurq::components::Rect::new(60.0, 40.0)
        .fill(SECONDARY)
        .rounded(6.0)
        .animation(Animation::new("grow-shrink").duration_ms(2000).linear().infinite()),
    ))
    .pad(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn keyframe_card(label: &str, content: impl Into<Element>) -> Element {
  lurq::components::Column::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(content)
        .size(260.0, 80.0),
    )
    .child(text(label, 11.0, FontWeight::Medium, TEXT_MUTED))
    .into()
}

// --- Opacity Showcase ---

fn opacity_showcase() -> Element {
  lurq::components::Row::new()
    .spacing(12.0)
    .align_items(Alignment::End)
    .child(opacity_sample("100%", 1.0))
    .child(opacity_sample("75%", 0.75))
    .child(opacity_sample("50%", 0.5))
    .child(opacity_sample("25%", 0.25))
    .child(opacity_sample("10%", 0.1))
    .pad(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn opacity_sample(label: &str, value: f32) -> Element {
  lurq::components::Column::new()
    .spacing(6.0)
    .align_items(Alignment::Center)
    .child(
      lurq::components::Rect::new(60.0, 60.0)
        .fill(PRIMARY)
        .rounded(8.0)
        .opacity(value),
    )
    .child(text(label, 11.0, FontWeight::Medium, TEXT_MUTED))
    .into()
}

// --- Combined ---

fn combined_demo() -> Element {
  lurq::components::Row::new()
    .spacing(16.0)
    .child(
      lurq::components::Column::new()
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(
          lurq::components::Rect::new(100.0, 60.0)
            .fill(PRIMARY)
            .rounded(8.0)
            .animation(Animation::new("pulse").duration_ms(3000).linear().infinite())
            .transition(Transition::background_color().duration_ms(300))
            .hovered(|s| s.fill("#ef4444"))
            .cursor(CursorIcon::Pointer),
        )
        .child(text("Pulse + Hover Color", 10.0, FontWeight::Medium, TEXT_MUTED)),
    )
    .child(
      lurq::components::Column::new()
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(
          lurq::components::Rect::new(100.0, 60.0)
            .fill(SECONDARY)
            .rounded(8.0)
            .animation(Animation::new("color-cycle").duration_ms(4000).linear().infinite())
            .transition(Transition::all().duration_ms(400).easing(Easing::EASE_OUT))
            .hovered(|s| s.rounded(30.0))
            .cursor(CursorIcon::Pointer),
        )
        .child(text("Cycle + Hover Radius", 10.0, FontWeight::Medium, TEXT_MUTED)),
    )
    .child(
      lurq::components::Column::new()
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(
          lurq::components::Rect::new(100.0, 60.0)
            .fill(ACCENT)
            .rounded(8.0)
            .transition(Transition::background_color().duration_ms(500))
            .transition(Transition::all().duration_ms(500).easing(Easing::EASE_IN_OUT))
            .hovered(|s| s.fill("#f59e0b").rounded(30.0).size(120.0, 70.0))
            .cursor(CursorIcon::Pointer),
        )
        .child(text("Multi-property", 10.0, FontWeight::Medium, TEXT_MUTED)),
    )
    .pad(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

// --- Transforms ---

fn transform_demos() -> Element {
  lurq::components::Row::new()
    .spacing(24.0)
    .align_items(Alignment::Start)
    .child(transform_card(
      "Static Rotate",
      lurq::components::Rect::new(60.0, 60.0)
        .fill(PRIMARY)
        .rounded(8.0)
        .transform(Transform2D::rotate_deg(45.0)),
    ))
    .child(transform_card(
      "Static Scale",
      lurq::components::Rect::new(60.0, 60.0)
        .fill(SECONDARY)
        .rounded(8.0)
        .transform(Transform2D::scale(1.4, 0.7)),
    ))
    .child(transform_card(
      "Spin",
      lurq::components::Rect::new(50.0, 50.0)
        .fill(ACCENT)
        .rounded(6.0)
        .animation(Animation::new("spin").duration_ms(2000).linear().infinite()),
    ))
    .child(transform_card(
      "Rock",
      lurq::components::Rect::new(50.0, 50.0)
        .fill("#f59e0b")
        .rounded(6.0)
        .animation(Animation::new("rock").duration_ms(1000).linear().infinite()),
    ))
    .child(transform_card(
      "Spin + Cycle",
      lurq::components::Rect::new(50.0, 50.0)
        .fill(PRIMARY)
        .rounded(25.0)
        .animation(Animation::new("spin-color").duration_ms(3000).linear().infinite()),
    ))
    .pad(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn transform_card(label: &str, content: impl Into<Element>) -> Element {
  lurq::components::Column::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(content)
        .size(120.0, 100.0),
    )
    .child(text(label, 11.0, FontWeight::Medium, TEXT_MUTED))
    .into()
}
