use crate::{
  app::glyph_engine::GlyphEngine,
  layout::{
    Alignment, Constraints, Offset, Size, StackAlignment,
    layout_kind::{FrameConstraints, LayoutKind, ScrollDirection, ScrollState},
    layout_result::{ChildLayout, LayoutResult},
    quad::{ClipRect, Quad, QuadContent},
    text_style::TextStyle,
  },
  node::{node::Node, padding::Padding},
};

pub(crate) struct LayoutEngine;

impl LayoutEngine {
  pub(crate) fn new() -> Self {
    Self
  }

  pub(crate) fn compute(&self, glyph_engine: &mut GlyphEngine, node: &Node, constraints: Constraints) -> LayoutResult {
    self.layout_node(glyph_engine, node, constraints)
  }

  pub(crate) fn resolve_quads(&self, node: &Node, result: &LayoutResult) -> Vec<Quad> {
    let mut quads = Vec::new();
    self.collect_quads(node, result, 0.0, 0.0, ClipRect::default(), &mut quads);
    quads
  }

  fn collect_quads(
    &self,
    node: &Node,
    result: &LayoutResult,
    abs_x: f32,
    abs_y: f32,
    clip: ClipRect,
    quads: &mut Vec<Quad>,
  ) {
    let is_scroll = matches!(node.kind(), LayoutKind::ScrollModifier { .. });

    if let Some(ref node_ref) = node.node_ref {
      node_ref.update(abs_x, abs_y, result.size.width, result.size.height);
    }

    let has_visual = node.color().is_some() || node.get_border().is_some();
    let content = match node.kind() {
      LayoutKind::Text { content, style } => QuadContent::Text {
        text: content.clone(),
        style: style.clone(),
      },
      _ if has_visual => QuadContent::Rect {
        color: node.color().unwrap_or(crate::node::color::Color::new(0, 0, 0, 0)),
      },
      _ => QuadContent::None,
    };

    match &content {
      QuadContent::None => {}
      _ => {
        quads.push(Quad {
          x: abs_x,
          y: abs_y,
          width: result.size.width,
          height: result.size.height,
          content,
          border_radius: node.get_border_radius(),
          border: node.get_border().cloned(),
          clip,
        });
      }
    }

    let child_clip = if is_scroll {
      ClipRect {
        x: abs_x,
        y: abs_y,
        width: result.size.width,
        height: result.size.height,
        active: true,
      }
    } else {
      clip
    };

    for (child_layout, child_node) in result.children.iter().zip(node.children().iter()) {
      self.collect_quads(
        child_node,
        &child_layout.result,
        abs_x + child_layout.offset.x,
        abs_y + child_layout.offset.y,
        child_clip,
        quads,
      );
    }

    if let LayoutKind::ScrollModifier { state, direction } = node.kind() {
      state.set_viewport_position(abs_x, abs_y);
      let sb_style = node.scrollbar_style();
      state.set_style(sb_style.clone());
      let thumb_color = if state.is_dragging() || state.is_thumb_hovered() {
        sb_style.thumb_hover_color
      } else {
        sb_style.thumb_color
      };

      match direction {
        ScrollDirection::Vertical | ScrollDirection::Both => {
          if let Some(geo) = crate::layout::scrollbar::compute_vertical_scrollbar(
            &sb_style,
            abs_x,
            abs_y,
            result.size.width,
            result.size.height,
            state.content_height(),
            state.scroll_y(),
          ) {
            if sb_style.track_color.a() > 0 {
              quads.push(Quad {
                x: geo.track_x,
                y: geo.track_y,
                width: geo.track_width,
                height: geo.track_height,
                content: QuadContent::Rect {
                  color: sb_style.track_color,
                },
                border_radius: Some(crate::node::border::BorderRadius::all(sb_style.track_radius)),
                border: None,
                clip,
              });
            }
            quads.push(Quad {
              x: geo.thumb_x,
              y: geo.thumb_y,
              width: geo.thumb_width,
              height: geo.thumb_height,
              content: QuadContent::Rect { color: thumb_color },
              border_radius: Some(crate::node::border::BorderRadius::all(sb_style.thumb_radius)),
              border: None,
              clip,
            });
          }
        }
        _ => {}
      }
      match direction {
        ScrollDirection::Horizontal | ScrollDirection::Both => {
          if let Some(geo) = crate::layout::scrollbar::compute_horizontal_scrollbar(
            &sb_style,
            abs_x,
            abs_y,
            result.size.width,
            result.size.height,
            state.content_width(),
            state.scroll_x(),
          ) {
            if sb_style.track_color.a() > 0 {
              quads.push(Quad {
                x: geo.track_x,
                y: geo.track_y,
                width: geo.track_width,
                height: geo.track_height,
                content: QuadContent::Rect {
                  color: sb_style.track_color,
                },
                border_radius: Some(crate::node::border::BorderRadius::all(sb_style.track_radius)),
                border: None,
                clip,
              });
            }
            quads.push(Quad {
              x: geo.thumb_x,
              y: geo.thumb_y,
              width: geo.thumb_width,
              height: geo.thumb_height,
              content: QuadContent::Rect { color: thumb_color },
              border_radius: Some(crate::node::border::BorderRadius::all(sb_style.thumb_radius)),
              border: None,
              clip,
            });
          }
        }
        _ => {}
      }
    }
  }

  fn layout_node(&self, glyph_engine: &mut GlyphEngine, node: &Node, constraints: Constraints) -> LayoutResult {
    // Check cache — skip layout if constraints unchanged and no structural changes
    if let Some(mut cached) = node.layout_cache.get(constraints) {
      if let LayoutKind::ScrollModifier { state, .. } = node.kind() {
        if let Some(child) = cached.children.first_mut() {
          child.offset.x = -state.scroll_x();
          child.offset.y = -state.scroll_y();
        }
        state.update_layout(
          cached.children.first().map(|c| c.result.size.width).unwrap_or(0.0),
          cached.children.first().map(|c| c.result.size.height).unwrap_or(0.0),
          cached.size.width,
          cached.size.height,
        );
      }
      return cached;
    }

    let result = match node.kind() {
      LayoutKind::Leaf => self.layout_leaf(constraints),
      LayoutKind::Text { content, style } => self.layout_text(glyph_engine, content, style, constraints),
      LayoutKind::Row { spacing, align } => self.layout_flex(glyph_engine, node, constraints, *spacing, *align, false),
      LayoutKind::Column { spacing, align } => {
        self.layout_flex(glyph_engine, node, constraints, *spacing, *align, true)
      }
      LayoutKind::Stack { align } => self.layout_stack(glyph_engine, node, constraints, *align),
      LayoutKind::PaddingModifier(padding) => self.layout_padding(glyph_engine, node, constraints, padding),
      LayoutKind::FrameModifier(frame) => self.layout_frame(glyph_engine, node, constraints, frame),
      LayoutKind::OffsetModifier { x, y } => self.layout_offset(glyph_engine, node, constraints, *x, *y),
      LayoutKind::AlignModifier(_) => self.layout_passthrough(glyph_engine, node, constraints),
      LayoutKind::FlexModifier(_) => self.layout_passthrough(glyph_engine, node, constraints),
      LayoutKind::ScrollModifier { state, direction } => {
        self.layout_scroll(glyph_engine, node, constraints, state, *direction)
      }
    };

    node.layout_cache.store(constraints, result.clone());
    result
  }

  fn layout_leaf(&self, constraints: Constraints) -> LayoutResult {
    LayoutResult {
      size: constraints.constrain(Size::default()),
      children: vec![],
    }
  }

  fn layout_text(
    &self,
    glyph_engine: &mut GlyphEngine,
    text: &str,
    style: &TextStyle,
    constraints: Constraints,
  ) -> LayoutResult {
    let max_width = if constraints.max_width.is_finite() {
      constraints.max_width
    } else {
      f32::MAX
    };
    let measured = glyph_engine.measure_text(text, style, max_width);
    let size = constraints.constrain(measured);
    LayoutResult { size, children: vec![] }
  }

  fn layout_flex(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    spacing: f32,
    align: Alignment,
    vertical: bool,
  ) -> LayoutResult {
    let children = node.children();
    if children.is_empty() {
      return LayoutResult {
        size: constraints.constrain(Size::default()),
        children: vec![],
      };
    }

    let total_spacing = spacing * (children.len() as f32 - 1.0).max(0.0);

    let mut flex_total = 0.0_f32;
    let mut non_flex_results: Vec<Option<LayoutResult>> = Vec::with_capacity(children.len());

    for child in children {
      if let LayoutKind::FlexModifier(factor) = child.kind() {
        flex_total += factor;
        non_flex_results.push(None);
      } else {
        let child_constraints = if vertical {
          Constraints {
            min_width: constraints.min_width,
            max_width: constraints.max_width,
            min_height: 0.0,
            max_height: f32::INFINITY,
          }
        } else {
          Constraints {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: constraints.min_height,
            max_height: constraints.max_height,
          }
        };
        non_flex_results.push(Some(self.layout_node(glyph_engine, child, child_constraints)));
      }
    }

    let non_flex_main: f32 = non_flex_results
      .iter()
      .filter_map(|r| r.as_ref())
      .map(|r| if vertical { r.size.height } else { r.size.width })
      .sum();

    let available_main = if vertical {
      (constraints.max_height - total_spacing - non_flex_main).max(0.0)
    } else {
      (constraints.max_width - total_spacing - non_flex_main).max(0.0)
    };

    let mut results: Vec<LayoutResult> = Vec::with_capacity(children.len());
    for (i, child) in children.iter().enumerate() {
      if let Some(existing) = non_flex_results[i].take() {
        results.push(existing);
      } else {
        let flex = match child.kind() {
          LayoutKind::FlexModifier(f) => *f,
          _ => 1.0,
        };
        let flex_size = available_main * (flex / flex_total);
        let child_constraints = if vertical {
          Constraints {
            min_width: constraints.min_width,
            max_width: constraints.max_width,
            min_height: flex_size,
            max_height: flex_size,
          }
        } else {
          Constraints {
            min_width: flex_size,
            max_width: flex_size,
            min_height: constraints.min_height,
            max_height: constraints.max_height,
          }
        };
        results.push(self.layout_node(glyph_engine, child, child_constraints));
      }
    }

    let max_cross: f32 = results
      .iter()
      .map(|r| if vertical { r.size.width } else { r.size.height })
      .fold(0.0_f32, f32::max);

    let total_main: f32 = results
      .iter()
      .map(|r| if vertical { r.size.height } else { r.size.width })
      .sum::<f32>()
      + total_spacing;

    let size = if vertical {
      constraints.constrain(Size::new(max_cross, total_main))
    } else {
      constraints.constrain(Size::new(total_main, max_cross))
    };

    let mut child_layouts = Vec::with_capacity(results.len());
    let mut main_cursor = 0.0_f32;

    for result in results {
      let child_main = if vertical {
        result.size.height
      } else {
        result.size.width
      };
      let child_cross = if vertical {
        result.size.width
      } else {
        result.size.height
      };
      let container_cross = if vertical { size.width } else { size.height };

      let cross_offset = align.cross_offset(container_cross, child_cross);

      let offset = if vertical {
        Offset::new(cross_offset, main_cursor)
      } else {
        Offset::new(main_cursor, cross_offset)
      };

      main_cursor += child_main + spacing;
      child_layouts.push(ChildLayout { offset, result });
    }

    LayoutResult {
      size,
      children: child_layouts,
    }
  }

  fn layout_stack(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    align: StackAlignment,
  ) -> LayoutResult {
    let children = node.children();
    let results: Vec<LayoutResult> = children
      .iter()
      .map(|child| self.layout_node(glyph_engine, child, constraints))
      .collect();

    let max_width = results.iter().map(|r| r.size.width).fold(0.0_f32, f32::max);
    let max_height = results.iter().map(|r| r.size.height).fold(0.0_f32, f32::max);
    let size = constraints.constrain(Size::new(max_width, max_height));

    let child_layouts: Vec<ChildLayout> = results
      .into_iter()
      .zip(children.iter())
      .map(|(result, child)| {
        let child_align = match child.kind() {
          LayoutKind::AlignModifier(a) => a.to_stack_alignment(),
          _ => align,
        };
        let offset = child_align.resolve_offset(size, result.size);
        ChildLayout { offset, result }
      })
      .collect();

    LayoutResult {
      size,
      children: child_layouts,
    }
  }

  fn layout_padding(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    padding: &Padding,
  ) -> LayoutResult {
    let h_pad = padding.get_left().to_px() + padding.get_right().to_px();
    let v_pad = padding.get_top().to_px() + padding.get_bottom().to_px();

    let inner_constraints = Constraints {
      min_width: (constraints.min_width - h_pad).max(0.0),
      max_width: (constraints.max_width - h_pad).max(0.0),
      min_height: (constraints.min_height - v_pad).max(0.0),
      max_height: (constraints.max_height - v_pad).max(0.0),
    };

    let child = &node.children()[0];
    let child_result = self.layout_node(glyph_engine, child, inner_constraints);

    let size = constraints.constrain(Size::new(
      child_result.size.width + h_pad,
      child_result.size.height + v_pad,
    ));

    let offset = Offset::new(padding.get_left().to_px(), padding.get_top().to_px());

    LayoutResult {
      size,
      children: vec![ChildLayout {
        offset,
        result: child_result,
      }],
    }
  }

  fn layout_frame(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    frame: &FrameConstraints,
  ) -> LayoutResult {
    let mut c = constraints;
    if let Some(w) = frame.width {
      c.min_width = w;
      c.max_width = w;
    }
    if let Some(h) = frame.height {
      c.min_height = h;
      c.max_height = h;
    }
    if let Some(v) = frame.min_width {
      c.min_width = c.min_width.max(v);
    }
    if let Some(v) = frame.max_width {
      c.max_width = c.max_width.min(v);
    }
    if let Some(v) = frame.min_height {
      c.min_height = c.min_height.max(v);
    }
    if let Some(v) = frame.max_height {
      c.max_height = c.max_height.min(v);
    }

    let child = &node.children()[0];
    let child_result = self.layout_node(glyph_engine, child, c);
    let size = c.constrain(child_result.size);

    LayoutResult {
      size,
      children: vec![ChildLayout {
        offset: Offset::default(),
        result: child_result,
      }],
    }
  }

  fn layout_offset(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    x: f32,
    y: f32,
  ) -> LayoutResult {
    let child = &node.children()[0];
    let child_result = self.layout_node(glyph_engine, child, constraints);
    let size = child_result.size;

    LayoutResult {
      size,
      children: vec![ChildLayout {
        offset: Offset::new(x, y),
        result: child_result,
      }],
    }
  }

  fn layout_scroll(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    state: &ScrollState,
    direction: ScrollDirection,
  ) -> LayoutResult {
    let child = &node.children()[0];

    let child_constraints = match direction {
      ScrollDirection::Vertical => Constraints {
        min_width: constraints.min_width,
        max_width: constraints.max_width,
        min_height: 0.0,
        max_height: f32::INFINITY,
      },
      ScrollDirection::Horizontal => Constraints {
        min_width: 0.0,
        max_width: f32::INFINITY,
        min_height: constraints.min_height,
        max_height: constraints.max_height,
      },
      ScrollDirection::Both => Constraints {
        min_width: 0.0,
        max_width: f32::INFINITY,
        min_height: 0.0,
        max_height: f32::INFINITY,
      },
    };

    let child_result = self.layout_node(glyph_engine, child, child_constraints);
    let size = constraints.constrain(Size::new(
      child_result.size.width.max(constraints.min_width),
      child_result.size.height.max(constraints.min_height),
    ));

    state.update_layout(
      child_result.size.width,
      child_result.size.height,
      size.width,
      size.height,
    );

    LayoutResult {
      size,
      children: vec![ChildLayout {
        offset: Offset::new(-state.scroll_x(), -state.scroll_y()),
        result: child_result,
      }],
    }
  }

  fn layout_passthrough(&self, glyph_engine: &mut GlyphEngine, node: &Node, constraints: Constraints) -> LayoutResult {
    let child = &node.children()[0];
    let child_result = self.layout_node(glyph_engine, child, constraints);
    let size = child_result.size;

    LayoutResult {
      size,
      children: vec![ChildLayout {
        offset: Offset::default(),
        result: child_result,
      }],
    }
  }
}
