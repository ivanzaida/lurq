use std::path::Path;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, MouseEvent, MouseEventKind, ScrollEvent, ScrollPhase},
    glyph_engine::{AtlasPacker, GlyphEngine},
    hit_test::hit_test_tree,
    profiler::{FrameProfile, ProfileScope},
    render_engine::RenderEngine,
  },
  layout::{
    Constraints, Size,
    layout_engine::LayoutEngine,
    layout_kind::{LayoutKind, ScrollState},
    layout_result::LayoutResult,
    quad::{ClipRect, Quad, QuadContent},
    render_list::{RectCmd, RenderList},
  },
  node::{border::BorderPlacement, color::Color, node::Node},
};

trait AnyRootComponent: Send + Sync {
  fn render(&self, ctx: &mut Ctx) -> Node;
}

struct RootComponentWrapper<C: Component> {
  component: C,
}

impl<C: Component> AnyRootComponent for RootComponentWrapper<C> {
  fn render(&self, ctx: &mut Ctx) -> Node {
    self.component.render(ctx)
  }
}

pub struct Runtime {
  glyph_engine: GlyphEngine,
  layout_engine: LayoutEngine,
  render_engine: Option<Box<dyn RenderEngine>>,
  root: Option<Node>,
  root_component: Option<Box<dyn AnyRootComponent>>,
  root_ctx: Option<Ctx>,
  last_layout: Option<LayoutResult>,
  viewport_physical: Size,
  scale_factor: f32,
  scale_override: Option<f32>,
  hover_path: Vec<usize>,
  dragging_scroll: Option<ScrollState>,
  needs_redraw: bool,
  last_profile: FrameProfile,
}

impl Runtime {
  pub fn new() -> Self {
    Self {
      glyph_engine: GlyphEngine::new(),
      layout_engine: LayoutEngine::new(),
      render_engine: None,
      root: None,
      root_component: None,
      root_ctx: None,
      last_layout: None,
      viewport_physical: Size::new(800.0, 600.0),
      scale_factor: 1.0,
      scale_override: None,
      hover_path: Vec::new(),
      dragging_scroll: None,
      needs_redraw: false,
      last_profile: FrameProfile::default(),
    }
  }

  pub fn scale_factor(&self) -> f32 {
    self.scale_override.unwrap_or(self.scale_factor)
  }

  pub fn set_scale_override(&mut self, scale: Option<f32>) {
    self.scale_override = scale;
    self.glyph_engine.clear_cache();
  }

  pub fn set_scale_factor(&mut self, scale: f32) {
    self.scale_factor = scale;
    self.glyph_engine.clear_cache();
  }

  pub fn last_profile(&self) -> &FrameProfile {
    &self.last_profile
  }

  fn viewport_logical(&self) -> Size {
    let s = self.scale_factor();
    Size::new(self.viewport_physical.width / s, self.viewport_physical.height / s)
  }

  pub fn set_render_engine(&mut self, engine: Box<dyn RenderEngine>) {
    self.render_engine = Some(engine);
  }

  pub fn mount_root<C: Component>(&mut self, props: C::Props) {
    let mut ctx = Ctx::new_root();
    let component = C::create(&mut ctx, props);
    let wrapper = RootComponentWrapper { component };
    ctx.begin_render();
    let node = wrapper.render(&mut ctx);
    self.root = Some(node);
    self.root_component = Some(Box::new(wrapper));
    self.root_ctx = Some(ctx);
    self.last_layout = None;
    self.hover_path.clear();
  }

  pub fn rebuild(&mut self) {
    if let (Some(component), Some(ctx)) = (&self.root_component, &mut self.root_ctx) {
      ctx.begin_render();
      let node = component.render(ctx);
      self.root = Some(node);
      self.last_layout = None;
      self.hover_path.clear();
    }
  }

  pub fn set_root(&mut self, node: Node) {
    self.root = Some(node);
    self.root_component = None;
    self.root_ctx = None;
    self.last_layout = None;
    self.hover_path.clear();
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    self.viewport_physical = Size::new(width as f32, height as f32);
    if let Some(engine) = &mut self.render_engine {
      engine.resize(width, height);
    }
  }

  pub fn pass(&mut self, surface: &(impl HasWindowHandle + HasDisplayHandle)) {
    let scale = self.scale_factor();
    let logical = self.viewport_logical();

    let root = match &self.root {
      Some(r) => r,
      None => return,
    };
    let render_engine = match &mut self.render_engine {
      Some(r) => r,
      None => return,
    };

    let window = surface.window_handle().unwrap();
    let display = surface.display_handle().unwrap();
    let frame_start = ProfileScope::start();

    self.glyph_engine.reset_stats();

    let constraints = Constraints::tight(logical);
    let layout_start = ProfileScope::start();
    let result = self.layout_engine.compute(&mut self.glyph_engine, root, constraints);
    let layout_dur = layout_start.elapsed();

    let quad_start = ProfileScope::start();
    let quads = self.layout_engine.resolve_quads(root, &result);
    let quad_dur = quad_start.elapsed();
    let quad_count = quads.len();

    self.last_layout = Some(result);

    let glyph_start = ProfileScope::start();
    let mut rects = Vec::new();
    let mut atlas_packer = AtlasPacker::new();
    let mut glyphs = Vec::new();

    for quad in &quads {
      let scaled_clip = if quad.clip.active {
        ClipRect {
          x: quad.clip.x * scale,
          y: quad.clip.y * scale,
          width: quad.clip.width * scale,
          height: quad.clip.height * scale,
          active: true,
        }
      } else {
        ClipRect::default()
      };

      match &quad.content {
        QuadContent::Rect { color } => {
          let radii = quad
            .border_radius
            .map(|r| {
              [
                r.top_left * scale,
                r.top_right * scale,
                r.bottom_right * scale,
                r.bottom_left * scale,
              ]
            })
            .unwrap_or([0.0; 4]);

          let (mut x, mut y, mut w, mut h) = (quad.x * scale, quad.y * scale, quad.width * scale, quad.height * scale);
          let (stroke, stroke_color) = if let Some(ref b) = quad.border {
            let bw = b.width;
            let sw = [bw.top * scale, bw.right * scale, bw.bottom * scale, bw.left * scale];
            match b.placement {
              BorderPlacement::Outside => {
                x -= sw[3];
                y -= sw[0];
                w += sw[1] + sw[3];
                h += sw[0] + sw[2];
              }
              BorderPlacement::Center => {
                x -= sw[3] * 0.5;
                y -= sw[0] * 0.5;
                w += (sw[1] + sw[3]) * 0.5;
                h += (sw[0] + sw[2]) * 0.5;
              }
              BorderPlacement::Inside => {}
            }
            (sw, b.color)
          } else {
            ([0.0; 4], Color::new(0, 0, 0, 0))
          };

          rects.push(RectCmd {
            x,
            y,
            width: w,
            height: h,
            color: *color,
            radii,
            stroke,
            stroke_color,
            clip: scaled_clip,
          });
        }
        QuadContent::Text { text, style } => {
          let mut scaled_style = style.clone();
          scaled_style.font_size *= scale;
          let max_width = if quad.width > 0.0 { quad.width * scale } else { f32::MAX };
          let mut glyph_cmds = self.glyph_engine.rasterize_text(
            text,
            &scaled_style,
            max_width,
            quad.x * scale,
            quad.y * scale,
            &mut atlas_packer,
          );
          for g in &mut glyph_cmds {
            g.clip = scaled_clip;
          }
          glyphs.extend(glyph_cmds);
        }
        QuadContent::None => {}
      }
    }

    let glyph_dur = glyph_start.elapsed();
    let rect_count = rects.len();
    let glyph_count = glyphs.len();

    let list = RenderList {
      rects,
      glyphs,
      atlas: atlas_packer.to_atlas(),
    };

    let gpu_start = ProfileScope::start();
    render_engine.render(&list, window, display);
    let gpu_dur = gpu_start.elapsed();

    self.last_profile = FrameProfile {
      layout: layout_dur,
      quad_resolve: quad_dur,
      glyph_rasterize: glyph_dur,
      gpu_submit: gpu_dur,
      total: frame_start.elapsed(),
      quad_count,
      rect_count,
      glyph_count,
      glyph_cache_hits: self.glyph_engine.glyph_hits,
      glyph_cache_misses: self.glyph_engine.glyph_misses,
      text_measure_cache_hits: self.glyph_engine.measure_hits,
      text_measure_cache_misses: self.glyph_engine.measure_misses,
    };
  }

  pub fn propagate_event(&mut self, event: crate::app::events::Event) {
    match event {
      crate::app::events::Event::Mouse(e) => self.propagate_mouse_event(e),
      crate::app::events::Event::Keyboard(e) => self.propagate_keyboard_event(e),
      crate::app::events::Event::Scroll(e) => self.propagate_scroll_event(e),
    }
  }

  pub fn needs_redraw(&self) -> bool {
    self.needs_redraw
  }

  pub fn clear_needs_redraw(&mut self) {
    self.needs_redraw = false;
  }

  pub fn propagate_mouse_event(&mut self, evt: MouseEvent) {
    let scale = self.scale_factor();
    let lx = evt.x / scale;
    let ly = evt.y / scale;

    // Handle active scrollbar drag
    if let Some(ref drag_state) = self.dragging_scroll.clone() {
      match evt.kind {
        MouseEventKind::Move => {
          drag_state.drag_to(ly, &drag_state.style());
          self.needs_redraw = true;
          return;
        }
        MouseEventKind::Up => {
          drag_state.end_drag();
          self.dragging_scroll = None;
          self.needs_redraw = true;
          return;
        }
        _ => {}
      }
    }

    let root = match &self.root {
      Some(r) => r,
      None => return,
    };
    let result = match &self.last_layout {
      Some(r) => r,
      None => return,
    };

    let mut hits = Vec::new();
    hit_test_tree(root, result, 0.0, 0.0, lx, ly, &mut hits);

    // Check scrollbar thumb hover/press
    for (node, _) in &hits {
      if let LayoutKind::ScrollModifier { state, .. } = node.kind() {
        let sb_style = node.scrollbar_style();
        let (tx, ty, tw, th) = state.thumb_rect(&sb_style);
        let on_thumb = lx >= tx && lx <= tx + tw && ly >= ty && ly <= ty + th;

        if on_thumb != state.is_thumb_hovered() {
          state.set_thumb_hovered(on_thumb);
          self.needs_redraw = true;
        }

        if on_thumb && matches!(evt.kind, MouseEventKind::Down) {
          state.begin_drag(ly);
          self.dragging_scroll = Some(state.clone());
          self.needs_redraw = true;
          return;
        }
      }
    }

    // Normal event dispatch
    for (node, _rect) in &hits {
      match evt.kind {
        MouseEventKind::Click => {
          if let Some(ref handler) = node.events.on_click {
            handler(&evt);
          }
        }
        MouseEventKind::DoubleClick => {
          if let Some(ref handler) = node.events.on_dblclick {
            handler(&evt);
          }
        }
        MouseEventKind::Down => {
          if let Some(ref handler) = node.events.on_mouse_down {
            handler(&evt);
          }
        }
        MouseEventKind::Up => {
          if let Some(ref handler) = node.events.on_mouse_up {
            handler(&evt);
          }
        }
        MouseEventKind::Move => {
          if let Some(ref handler) = node.events.on_mouse_move {
            handler(&evt);
          }
        }
      }
    }

    let current_ptrs: Vec<usize> = hits.iter().map(|(n, _)| *n as *const Node as usize).collect();

    for old_ptr in &self.hover_path {
      if !current_ptrs.contains(old_ptr) {
        let node_ref = unsafe { &*(*old_ptr as *const Node) };
        if let Some(ref state) = node_ref.interaction {
          state.set_hovered(false);
          self.needs_redraw = true;
        }
        if let Some(ref handler) = node_ref.events.on_mouse_leave {
          handler();
        }
      }
    }

    for (node, _) in &hits {
      let ptr = *node as *const Node as usize;
      if !self.hover_path.contains(&ptr) {
        if let Some(ref state) = node.interaction {
          state.set_hovered(true);
          self.needs_redraw = true;
        }
        if let Some(ref handler) = node.events.on_mouse_enter {
          handler();
        }
      }
    }

    // Update active state
    for (node, _) in &hits {
      if let Some(ref state) = node.interaction {
        match evt.kind {
          MouseEventKind::Down => {
            state.set_active(true);
            self.needs_redraw = true;
          }
          MouseEventKind::Up | MouseEventKind::Click => {
            if state.is_active() {
              state.set_active(false);
              self.needs_redraw = true;
            }
          }
          _ => {}
        }
      }
    }

    self.hover_path = current_ptrs;
  }

  pub fn propagate_keyboard_event(&mut self, evt: KeyboardEvent) {
    let root = match &self.root {
      Some(r) => r,
      None => return,
    };

    fire_keyboard_recursive(root, &evt);
  }

  pub fn propagate_scroll_event(&mut self, evt: ScrollEvent) {
    let root = match &self.root {
      Some(r) => r,
      None => return,
    };
    let result = match &self.last_layout {
      Some(r) => r,
      None => return,
    };

    let scale = self.scale_factor();
    let lx = evt.x / scale;
    let ly = evt.y / scale;

    let mut hits = Vec::new();
    hit_test_tree(root, result, 0.0, 0.0, lx, ly, &mut hits);

    // Auto-scroll the first scroll container hit
    let mut handled = false;
    for (node, _) in &hits {
      if let LayoutKind::ScrollModifier { state, .. } = node.kind() {
        state.scroll_by(-evt.delta_x, -evt.delta_y);
        self.needs_redraw = true;
        handled = true;
        break;
      }
    }

    // Fire user handlers
    for (node, _) in &hits {
      match evt.phase {
        ScrollPhase::Start => {
          if let Some(ref handler) = node.events.on_scroll_start {
            handler(&evt);
          }
        }
        ScrollPhase::Scroll => {
          if let Some(ref handler) = node.events.on_scroll {
            handler(&evt);
          }
        }
        ScrollPhase::End => {
          if let Some(ref handler) = node.events.on_scroll_end {
            handler(&evt);
          }
        }
      }
    }

    let _ = handled;
  }

  pub fn load_font(&mut self, data: Vec<u8>) {
    self.glyph_engine.load_font(data);
  }

  pub fn load_font_file(&mut self, path: &Path) {
    self.glyph_engine.load_font_file(path);
  }

  pub fn load_fonts_dir(&mut self, path: &Path) {
    self.glyph_engine.load_fonts_dir(path);
  }

  pub fn register_font(&mut self, name: &str, family: &str) {
    self.glyph_engine.register_font(name, family);
  }

  pub fn compute_layout(&mut self, constraints: Constraints) -> Option<LayoutResult> {
    let root = self.root.as_ref()?;
    Some(self.layout_engine.compute(&mut self.glyph_engine, root, constraints))
  }

  pub fn resolve_quads(&self, result: &LayoutResult) -> Vec<Quad> {
    match &self.root {
      Some(root) => self.layout_engine.resolve_quads(root, result),
      None => vec![],
    }
  }
}

fn fire_keyboard_recursive(node: &Node, evt: &KeyboardEvent) {
  if let Some(ref handler) = node.events.on_key_down {
    handler(evt);
  }
  for child in node.children() {
    fire_keyboard_recursive(child, evt);
  }
}
