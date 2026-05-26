use crate::{
  app::glyph_engine::GlyphEngine,
  layout::{
    Alignment, Constraints, Offset, Size, StackAlignment,
    layout_kind::{
      FlexParams, FlexWrap, FrameConstraints, Justify, LayoutKind, Overflow, ScrollDirection, ScrollState,
    },
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
    Self::invalidate_scroll_ancestors(node);
    let result = self.layout_node(glyph_engine, node, constraints);
    node.clear_guards();
    result
  }

  fn invalidate_scroll_ancestors(node: &Node) -> bool {
    let mut child_dirty = false;
    for child in node.children() {
      if Self::invalidate_scroll_ancestors(child) {
        child_dirty = true;
      }
    }
    if let LayoutKind::ScrollModifier { state, .. } = node.kind() {
      if state.take_scroll_dirty() {
        node.layout_cache.invalidate();
        return true;
      }
    }
    if child_dirty {
      node.layout_cache.invalidate();
    }
    child_dirty
  }

  pub(crate) fn needs_visual_update(node: &Node) -> bool {
    node.any_visual_dirty()
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
      LayoutKind::Text { style } => QuadContent::Text {
        text: node.text_content().unwrap_or("").to_owned(),
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

    let child_clip = if is_scroll || node.overflow == Overflow::Hidden {
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
    if node.text_content.is_changed() {
      node.layout_cache.invalidate();
    }

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
      LayoutKind::Leaf => self.layout_leaf(node, constraints),
      LayoutKind::Text { style } => {
        let content = node.text_content().unwrap_or("");
        self.layout_text(glyph_engine, content, style, constraints)
      }
      LayoutKind::Row {
        spacing,
        align,
        justify,
        wrap,
      } => self.layout_flex(
        glyph_engine,
        node,
        constraints,
        *spacing,
        *align,
        *justify,
        *wrap,
        false,
      ),
      LayoutKind::Column {
        spacing,
        align,
        justify,
        wrap,
      } => self.layout_flex(glyph_engine, node, constraints, *spacing, *align, *justify, *wrap, true),
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

  fn layout_leaf(&self, node: &Node, constraints: Constraints) -> LayoutResult {
    let preferred = node.intrinsic_size.unwrap_or_default();
    LayoutResult {
      size: constraints.constrain(preferred),
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
    justify: Justify,
    wrap: FlexWrap,
    vertical: bool,
  ) -> LayoutResult {
    let children = node.children();
    if children.is_empty() {
      return LayoutResult {
        size: constraints.constrain(Size::default()),
        children: vec![],
      };
    }

    if wrap == FlexWrap::Wrap {
      return self.layout_flex_wrap(glyph_engine, node, constraints, spacing, align, justify, vertical);
    }

    let total_spacing = spacing * (children.len() as f32 - 1.0).max(0.0);
    let max_main = if vertical {
      constraints.max_height
    } else {
      constraints.max_width
    };

    let mut grow_total = 0.0_f32;
    let mut shrink_total = 0.0_f32;
    let mut non_flex_results: Vec<Option<LayoutResult>> = Vec::with_capacity(children.len());
    let mut flex_params_list: Vec<FlexParams> = Vec::with_capacity(children.len());

    for child in children {
      if let LayoutKind::FlexModifier(params) = child.kind() {
        grow_total += params.grow;
        shrink_total += params.shrink;
        flex_params_list.push(*params);
        if params.grow == 0.0 && params.basis.is_none() {
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
        } else {
          non_flex_results.push(None);
        }
      } else {
        flex_params_list.push(FlexParams {
          grow: 0.0,
          shrink: 0.0,
          basis: None,
        });
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

    let remaining = max_main - total_spacing - non_flex_main;

    let mut results: Vec<LayoutResult> = Vec::with_capacity(children.len());
    for (i, child) in children.iter().enumerate() {
      if let Some(existing) = non_flex_results[i].take() {
        results.push(existing);
      } else {
        let params = &flex_params_list[i];
        let basis_size = params.basis.unwrap_or(0.0);
        let flex_size = if remaining > 0.0 && grow_total > 0.0 {
          basis_size + remaining.max(0.0) * (params.grow / grow_total)
        } else {
          basis_size
        };
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

    if shrink_total > 0.0 {
      let total_children_main: f32 = results
        .iter()
        .map(|r| if vertical { r.size.height } else { r.size.width })
        .sum();
      let overflow = total_children_main + total_spacing - max_main;
      if overflow > 0.0 {
        let mut remaining_overflow = overflow;
        let mut remaining_shrink = shrink_total;
        let mut frozen = vec![false; children.len()];

        loop {
          let mut any_clamped = false;
          for i in 0..children.len() {
            if frozen[i] {
              continue;
            }
            let params = &flex_params_list[i];
            if params.shrink <= 0.0 {
              continue;
            }
            let child_main = if vertical {
              results[i].size.height
            } else {
              results[i].size.width
            };
            let shrink_amount = remaining_overflow * (params.shrink / remaining_shrink);
            let min_main = children[i].min_main_size(vertical);
            let new_main = (child_main - shrink_amount).max(min_main);
            if new_main > child_main - shrink_amount {
              frozen[i] = true;
              let actual_shrink = child_main - new_main;
              remaining_overflow -= actual_shrink;
              remaining_shrink -= params.shrink;
              any_clamped = true;
              if vertical {
                results[i].size.height = new_main;
              } else {
                results[i].size.width = new_main;
              }
            }
          }
          if !any_clamped {
            break;
          }
          if remaining_shrink <= 0.0 {
            break;
          }
        }

        for i in 0..children.len() {
          if frozen[i] {
            continue;
          }
          let params = &flex_params_list[i];
          if params.shrink <= 0.0 {
            continue;
          }
          let child_main = if vertical {
            results[i].size.height
          } else {
            results[i].size.width
          };
          let shrink_amount = remaining_overflow * (params.shrink / remaining_shrink);
          let new_main = (child_main - shrink_amount).max(0.0);
          if vertical {
            results[i].size.height = new_main;
          } else {
            results[i].size.width = new_main;
          }
        }
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

    let container_cross = if vertical { size.width } else { size.height };

    if matches!(align, Alignment::Stretch) {
      for (i, child) in children.iter().enumerate() {
        let r = &results[i];
        let child_cross = if vertical { r.size.width } else { r.size.height };
        if child_cross < container_cross {
          let stretch_constraints = if vertical {
            Constraints {
              min_width: container_cross,
              max_width: container_cross,
              min_height: r.size.height,
              max_height: r.size.height,
            }
          } else {
            Constraints {
              min_width: r.size.width,
              max_width: r.size.width,
              min_height: container_cross,
              max_height: container_cross,
            }
          };
          results[i] = self.layout_node(glyph_engine, child, stretch_constraints);
        }
      }
    }

    let child_layouts = self.position_flex_line(&results, &size, spacing, align, justify, vertical);

    LayoutResult {
      size,
      children: child_layouts,
    }
  }

  fn position_flex_line(
    &self,
    results: &[LayoutResult],
    container_size: &Size,
    spacing: f32,
    align: Alignment,
    justify: Justify,
    vertical: bool,
  ) -> Vec<ChildLayout> {
    let container_main = if vertical {
      container_size.height
    } else {
      container_size.width
    };
    let container_cross = if vertical {
      container_size.width
    } else {
      container_size.height
    };
    let children_main: f32 = results
      .iter()
      .map(|r| if vertical { r.size.height } else { r.size.width })
      .sum();
    let free_space = (container_main - children_main).max(0.0);
    let n = results.len() as f32;

    let (leading, gap) = match justify {
      Justify::Start => (0.0, spacing),
      Justify::End => (free_space - spacing * (n - 1.0), spacing),
      Justify::Center => ((free_space - spacing * (n - 1.0)) / 2.0, spacing),
      Justify::SpaceBetween => {
        if n > 1.0 {
          (0.0, free_space / (n - 1.0))
        } else {
          (0.0, 0.0)
        }
      }
      Justify::SpaceAround => {
        let g = free_space / n;
        (g / 2.0, g)
      }
      Justify::SpaceEvenly => {
        let g = free_space / (n + 1.0);
        (g, g)
      }
    };

    let mut child_layouts = Vec::with_capacity(results.len());
    let mut main_cursor = leading;

    for (i, result) in results.iter().enumerate() {
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
      let cross_offset = align.cross_offset(container_cross, child_cross);

      let offset = if vertical {
        Offset::new(cross_offset, main_cursor)
      } else {
        Offset::new(main_cursor, cross_offset)
      };

      main_cursor += child_main + if i < (n as usize - 1) { gap } else { 0.0 };
      child_layouts.push(ChildLayout {
        offset,
        result: result.clone(),
      });
    }

    child_layouts
  }

  fn layout_flex_wrap(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    spacing: f32,
    align: Alignment,
    justify: Justify,
    vertical: bool,
  ) -> LayoutResult {
    let children = node.children();
    let max_main = if vertical {
      constraints.max_height
    } else {
      constraints.max_width
    };

    let child_results: Vec<LayoutResult> = children
      .iter()
      .map(|child| {
        let c = if vertical {
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
        self.layout_node(glyph_engine, child, c)
      })
      .collect();

    let mut lines: Vec<Vec<usize>> = vec![vec![]];
    let mut line_main = 0.0_f32;

    for (i, r) in child_results.iter().enumerate() {
      let child_main = if vertical { r.size.height } else { r.size.width };
      let needed = if lines.last().unwrap().is_empty() {
        child_main
      } else {
        spacing + child_main
      };

      if !lines.last().unwrap().is_empty() && line_main + needed > max_main {
        lines.push(vec![i]);
        line_main = child_main;
      } else {
        lines.last_mut().unwrap().push(i);
        line_main += needed;
      }
    }

    let mut all_layouts = vec![
      ChildLayout {
        offset: Offset::default(),
        result: LayoutResult {
          size: Size::default(),
          children: vec![]
        },
      };
      children.len()
    ];
    let mut cross_cursor = 0.0_f32;
    let mut max_main_used = 0.0_f32;

    for line_indices in &lines {
      let line_results: Vec<LayoutResult> = line_indices.iter().map(|&i| child_results[i].clone()).collect();
      let line_cross: f32 = line_results
        .iter()
        .map(|r| if vertical { r.size.width } else { r.size.height })
        .fold(0.0_f32, f32::max);

      let line_main_total: f32 = line_results
        .iter()
        .map(|r| if vertical { r.size.height } else { r.size.width })
        .sum::<f32>()
        + spacing * (line_results.len() as f32 - 1.0).max(0.0);

      max_main_used = max_main_used.max(line_main_total);

      let line_size = if vertical {
        Size::new(line_cross, max_main.min(constraints.max_height))
      } else {
        Size::new(max_main.min(constraints.max_width), line_cross)
      };

      let positioned = self.position_flex_line(&line_results, &line_size, spacing, align, justify, vertical);

      for (j, &idx) in line_indices.iter().enumerate() {
        let mut layout = positioned[j].clone();
        if vertical {
          layout.offset.x += cross_cursor;
        } else {
          layout.offset.y += cross_cursor;
        }
        all_layouts[idx] = layout;
      }

      cross_cursor += line_cross + spacing;
    }

    let total_cross = (cross_cursor - spacing).max(0.0);
    let size = if vertical {
      constraints.constrain(Size::new(total_cross, max_main_used))
    } else {
      constraints.constrain(Size::new(max_main_used, total_cross))
    };

    LayoutResult {
      size,
      children: all_layouts,
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
    let parent_w = constraints.max_width;
    let parent_h = constraints.max_height;
    let left = padding.get_left().resolve(parent_w);
    let right = padding.get_right().resolve(parent_w);
    let top = padding.get_top().resolve(parent_h);
    let bottom = padding.get_bottom().resolve(parent_h);
    let h_pad = left + right;
    let v_pad = top + bottom;

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

    let offset = Offset::new(left, top);

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

    c.min_width = c.min_width.min(c.max_width);
    c.min_height = c.min_height.min(c.max_height);

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
