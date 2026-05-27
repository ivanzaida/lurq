use std::path::Path;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::{
  app::{
    component::Component,
    ctx::Ctx,
    events::{KeyboardEvent, MouseButton, MouseEvent, MouseEventKind, ScrollEvent, ScrollPhase},
    glyph_engine::{AtlasPacker, GlyphEngine},
    hit_test::hit_test_tree,
    profiler::{FrameProfile, ProfileScope, RuntimeMemoryProfile},
    render_engine::RenderEngine,
  },
  core::{ElementRect, ElementRef as OwnedElementRef, ElementRefMut as OwnedElementRefMut, IdGenerator, NodeId},
  layout::{
    Constraints, Size,
    layout_engine::LayoutEngine,
    layout_kind::{LayoutKind, ScrollState},
    layout_result::LayoutResult,
    quad::{ClipRect, Quad, QuadContent},
    render_list::{RectCmd, RenderList},
  },
  node::{
    Element, ElementRef, Node,
    border::BorderPlacement,
    color::Color,
    cursor::CursorIcon,
    node_kind::{NodeKind, SliderState},
  },
};

trait AnyRootComponent: Send + Sync {
  fn render(&self, ctx: &mut Ctx) -> Element;
  fn on_mounted(&self);
  fn on_unmounted(&self);
}

struct RootComponentWrapper<C: Component> {
  component: C,
}

impl<C: Component> AnyRootComponent for RootComponentWrapper<C> {
  fn render(&self, ctx: &mut Ctx) -> Element {
    self.component.render(ctx).into()
  }

  fn on_mounted(&self) {
    self.component.on_mounted();
  }

  fn on_unmounted(&self) {
    self.component.on_unmounted();
  }
}

pub struct Runtime {
  id_gen: IdGenerator,
  theme: crate::app::theme::Theme,
  glyph_engine: GlyphEngine,
  layout_engine: LayoutEngine,
  render_engine: Option<Box<dyn RenderEngine>>,
  root: Option<Node>,
  root_component: Option<Box<dyn AnyRootComponent>>,
  root_ctx: Option<Ctx>,
  last_layout: Option<LayoutResult>,
  layout_constraints_override: Option<Constraints>,
  viewport_physical: Size,
  scale_factor: f32,
  scale_override: Option<f32>,
  hover_path: Vec<usize>,
  active_path: Vec<usize>,
  dragging_scroll: Option<ScrollState>,
  dragging_slider: Option<SliderDrag>,
  focused_node: Option<NodeId>,
  focused_event_node: Option<NodeId>,
  focused_path: Option<Vec<usize>>,
  focused_event_path: Option<Vec<usize>>,
  cursor: CursorIcon,
  needs_redraw: bool,
  last_profile: FrameProfile,
  #[cfg(feature = "resources")]
  resource_loader: crate::resources::ResourceLoader,
}

impl Default for Runtime {
  fn default() -> Self {
    Self::new()
  }
}

impl Runtime {
  pub fn new() -> Self {
    Self {
      id_gen: IdGenerator::new(),
      theme: crate::app::theme::Theme::new(),
      glyph_engine: GlyphEngine::new(),
      layout_engine: LayoutEngine::new(),
      render_engine: None,
      root: None,
      root_component: None,
      root_ctx: None,
      last_layout: None,
      layout_constraints_override: None,
      viewport_physical: Size::new(800.0, 600.0),
      scale_factor: 1.0,
      scale_override: None,
      hover_path: Vec::new(),
      active_path: Vec::new(),
      dragging_scroll: None,
      dragging_slider: None,
      focused_node: None,
      focused_event_node: None,
      focused_path: None,
      focused_event_path: None,
      cursor: CursorIcon::Default,
      needs_redraw: false,
      last_profile: FrameProfile::default(),
      #[cfg(feature = "resources")]
      resource_loader: crate::resources::ResourceLoader::new(),
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

  pub fn memory_profile(&self) -> RuntimeMemoryProfile {
    let runtime_struct_bytes = std::mem::size_of::<Self>();
    let root_tree_bytes = self.root.as_ref().map(Node::estimated_memory_bytes).unwrap_or(0);
    let root_context_bytes = self.root_ctx.as_ref().map(Ctx::estimated_memory_bytes).unwrap_or(0);
    let root_component_bytes = self
      .root_component
      .as_ref()
      .map(|_| std::mem::size_of::<Box<dyn AnyRootComponent>>())
      .unwrap_or(0);
    let last_layout_bytes = self
      .last_layout
      .as_ref()
      .map(LayoutResult::estimated_memory_bytes)
      .unwrap_or(0);
    let glyph_engine_bytes = self.glyph_engine.estimated_memory_bytes();
    let render_engine_bytes = self
      .render_engine
      .as_ref()
      .map(|_| std::mem::size_of::<Box<dyn RenderEngine>>())
      .unwrap_or(0);
    let hover_path_bytes = self.hover_path.capacity() * std::mem::size_of::<usize>();
    let active_path_bytes = self.active_path.capacity() * std::mem::size_of::<usize>();
    let dragging_scroll_bytes = self
      .dragging_scroll
      .as_ref()
      .map(|_| std::mem::size_of::<ScrollState>())
      .unwrap_or(0);
    let total_bytes = runtime_struct_bytes
      + root_tree_bytes
      + root_context_bytes
      + root_component_bytes
      + last_layout_bytes
      + glyph_engine_bytes
      + render_engine_bytes
      + hover_path_bytes
      + active_path_bytes
      + dragging_scroll_bytes;

    RuntimeMemoryProfile {
      total_bytes,
      runtime_struct_bytes,
      root_tree_bytes,
      root_context_bytes,
      root_component_bytes,
      last_layout_bytes,
      glyph_engine_bytes,
      render_engine_bytes,
      hover_path_bytes,
      active_path_bytes,
      dragging_scroll_bytes,
    }
  }

  fn viewport_logical(&self) -> Size {
    let s = self.scale_factor();
    Size::new(self.viewport_physical.width / s, self.viewport_physical.height / s)
  }

  pub fn set_render_engine(&mut self, engine: Box<dyn RenderEngine>) {
    self.render_engine = Some(engine);
  }

  pub fn mount_root<C: Component>(&mut self, props: C::Props) {
    if let Some(component) = self.root_component.take() {
      component.on_unmounted();
    }
    if let Some(old) = &mut self.root {
      reset_element_ref_flags_recursive(old);
      old.free_ids(&self.id_gen);
    }
    let mut ctx = Ctx::new_root().with_theme(self.theme.clone());
    ctx.set_root_props(props);
    let component = C::create(&mut ctx);
    let wrapper = RootComponentWrapper { component };
    ctx.begin_render();
    let mut node = wrapper.render(&mut ctx).node;
    ctx.end_render();
    node.assign_ids(&self.id_gen);
    wrapper.on_mounted();
    self.root = Some(node);
    self.root_component = Some(Box::new(wrapper));
    self.root_ctx = Some(ctx);
    self.last_layout = None;
    self.hover_path.clear();
    self.active_path.clear();
    self.clear_focus();
  }

  pub fn rebuild(&mut self) {
    if let (Some(component), Some(ctx)) = (&self.root_component, &mut self.root_ctx) {
      let old_root = self.root.take().map(|mut old| {
        reset_element_ref_flags_recursive(&old);
        old.free_ids(&self.id_gen);
        old
      });
      ctx.begin_render();
      let mut node = component.render(ctx).node;
      ctx.end_render();
      if let Some(old) = &old_root {
        node.preserve_runtime_state_from(old);
      }
      node.assign_ids(&self.id_gen);
      self.root = Some(node);
      self.last_layout = None;
      self.hover_path.clear();
      self.active_path.clear();
      self.refresh_focus_ids();
    }
  }

  pub fn set_root(&mut self, element: impl Into<Element>) {
    if let Some(component) = self.root_component.take() {
      component.on_unmounted();
    }
    if let Some(old) = &mut self.root {
      reset_element_ref_flags_recursive(old);
      old.free_ids(&self.id_gen);
    }
    let mut node = element.into().node;
    node.assign_ids(&self.id_gen);
    self.root = Some(node);
    self.root_component = None;
    self.root_ctx = None;
    self.last_layout = None;
    self.hover_path.clear();
    self.active_path.clear();
    self.clear_focus();
  }

  pub fn root(&self) -> Option<ElementRef<'_>> {
    self.root.as_ref().map(ElementRef::new)
  }

  pub fn find_element(&mut self, predicate: impl for<'a> Fn(ElementRef<'a>) -> bool) -> Option<OwnedElementRef> {
    self.update_layout();

    let root = self.root.as_mut()?;
    let layout = self.last_layout.as_ref()?;
    find_element_recursive(root, layout, 0.0, 0.0, 0.0, 0.0, &predicate)
  }

  pub fn find_element_mut(&mut self, predicate: impl for<'a> Fn(ElementRef<'a>) -> bool) -> Option<OwnedElementRefMut> {
    self.find_element(predicate).map(|element_ref| element_ref.mutable())
  }

  pub fn id_gen(&self) -> &IdGenerator {
    &self.id_gen
  }

  pub fn theme(&self) -> &crate::app::theme::Theme {
    &self.theme
  }

  pub fn resize(&mut self, width: u32, height: u32) {
    self.viewport_physical = Size::new(width as f32, height as f32);
    if let Some(engine) = &mut self.render_engine {
      engine.resize(width, height);
    }
  }

  pub fn pass(&mut self, surface: &(impl HasWindowHandle + HasDisplayHandle)) {
    let scale = self.scale_factor();
    self.update_layout();

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

    let layout_start = ProfileScope::start();
    let result = match &self.last_layout {
      Some(result) => result.clone(),
      None => return,
    };
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
    #[cfg(feature = "image")]
    let mut images = Vec::new();
    #[cfg(feature = "svg")]
    let mut svgs = Vec::new();

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
        #[cfg(feature = "image")]
        QuadContent::Image { data } => {
          images.push(crate::images::ImageCmd {
            x: quad.x * scale,
            y: quad.y * scale,
            width: quad.width * scale,
            height: quad.height * scale,
            image_id: data.id(),
            data: data.data_arc(),
            image_width: data.width(),
            image_height: data.height(),
            clip: scaled_clip,
          });
        }
        #[cfg(feature = "svg")]
        QuadContent::Svg { data } => {
          let w = quad.width * scale;
          let h = quad.height * scale;
          let mesh = crate::svg::tessellate::tessellate(&data, w, h);
          svgs.push(crate::svg::SvgCmd {
            x: quad.x * scale,
            y: quad.y * scale,
            width: w,
            height: h,
            svg_id: data.id(),
            mesh: std::sync::Arc::new(mesh),
            clip: scaled_clip,
          });
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
      #[cfg(feature = "image")]
      images,
      #[cfg(feature = "svg")]
      svgs,
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
      memory: self.memory_profile(),
    };
  }

  pub fn mouse_move(&mut self, x: f32, y: f32) {
    self.dispatch_mouse(x, y, MouseButton::Left, MouseEventKind::Move);
    self.apply_reactive_updates_after_event();
  }

  pub fn mouse_down(&mut self, x: f32, y: f32, button: MouseButton) {
    self.dispatch_mouse(x, y, button, MouseEventKind::Down);
    self.apply_reactive_updates_after_event();
  }

  pub fn mouse_up(&mut self, x: f32, y: f32, button: MouseButton) {
    self.dispatch_mouse(x, y, button, MouseEventKind::Up);
    self.apply_reactive_updates_after_event();
  }

  pub fn click(&mut self, x: f32, y: f32, button: MouseButton) {
    self.dispatch_mouse(x, y, button, MouseEventKind::Click);
    self.apply_reactive_updates_after_event();
  }

  pub fn dblclick(&mut self, x: f32, y: f32, button: MouseButton) {
    self.dispatch_mouse(x, y, button, MouseEventKind::DoubleClick);
    self.apply_reactive_updates_after_event();
  }

  pub fn scroll(&mut self, x: f32, y: f32, delta_x: f32, delta_y: f32, phase: ScrollPhase) {
    self.dispatch_scroll(x, y, delta_x, delta_y, phase);
    self.apply_reactive_updates_after_event();
  }

  pub fn key_down(&mut self, key: String, code: String, shift: bool, ctrl: bool, alt: bool) {
    self.rebuild_if_dirty();
    if self.dispatch_text_input(&key, &code) {
      self.needs_redraw = true;
    }

    let mut evt = KeyboardEvent {
      key,
      code,
      shift,
      ctrl,
      alt,
      target_id: NodeId::UNASSIGNED,
    };
    let root = match &self.root {
      Some(r) => r,
      None => return,
    };
    fire_keyboard_recursive(root, &mut evt);
    self.apply_reactive_updates_after_event();
  }

  pub fn needs_redraw(&self) -> bool {
    self.needs_redraw || self.root.as_ref().is_some_and(has_dirty_element_ref_recursive)
  }

  pub fn cursor(&self) -> CursorIcon {
    self.cursor
  }

  pub fn clear_needs_redraw(&mut self) {
    self.needs_redraw = false;
  }

  fn dispatch_mouse(&mut self, x: f32, y: f32, button: MouseButton, kind: MouseEventKind) {
    let mut evt = MouseEvent {
      x,
      y,
      button,
      kind,
      target_id: NodeId::UNASSIGNED,
    };
    let scale = self.scale_factor();
    let lx = evt.x / scale;
    let ly = evt.y / scale;

    self.update_layout();

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
          self.clear_active_path();
          self.needs_redraw = true;
          return;
        }
        _ => {}
      }
    }

    if let Some(drag) = self.dragging_slider.clone() {
      match evt.kind {
        MouseEventKind::Move => {
          drag.update(lx);
          self.needs_redraw = true;
          return;
        }
        MouseEventKind::Up => {
          drag.update(lx);
          self.dragging_slider = None;
          self.clear_active_path();
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
    let mut pending_focus = None;
    let mut builtin_needs_redraw = false;
    let mut pending_slider_drag = None;

    if matches!(evt.kind, MouseEventKind::Click | MouseEventKind::Down) {
      if matches!(evt.kind, MouseEventKind::Down) {
        if let Some((node, rect)) = hits
          .iter()
          .find(|(node, _)| matches!(node.node_kind(), NodeKind::Slider { .. }))
        {
          if let NodeKind::Slider { state } = node.node_kind() {
            let drag = SliderDrag {
              state: state.clone(),
              x: rect.x,
              width: rect.width,
            };
            drag.update(lx);
            pending_slider_drag = Some(drag);
            pending_focus = Some(FocusTarget {
              input_id: node.node_id(),
              event_id: node.node_id(),
            });
            builtin_needs_redraw = true;
          }
        }
      }
      if let Some(target) = dispatch_builtin_pointer(&hits, lx, matches!(evt.kind, MouseEventKind::Click)) {
        pending_focus = Some(target);
        builtin_needs_redraw = true;
      }
      if hits.is_empty() && matches!(evt.kind, MouseEventKind::Click) {
        if let Some((node, rect)) = find_slider_by_y_recursive(root, result, 0.0, 0.0, ly) {
          if let NodeKind::Slider { state } = node.node_kind() {
            let ratio = if rect.width > 0.0 {
              (lx - rect.x) / rect.width
            } else {
              0.0
            };
            state.set_from_ratio(ratio);
            pending_focus = Some(FocusTarget {
              input_id: node.node_id(),
              event_id: node.node_id(),
            });
            builtin_needs_redraw = true;
          }
        }
      }
    }

    // Check scrollbar thumb hover/press
    for (node, _) in &hits {
      if let LayoutKind::ScrollModifier { state, .. } = node.layout_kind() {
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
      evt.target_id = node.node_id();
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
        let node = unsafe { &*(*old_ptr as *const Node) };
        set_node_hovered(node, false);
        self.needs_redraw = true;
        if let Some(ref handler) = node.events.on_mouse_leave {
          handler();
        }
      }
    }

    for (node, _) in &hits {
      let ptr = *node as *const Node as usize;
      if !self.hover_path.contains(&ptr) {
        set_node_hovered(node, true);
        self.needs_redraw = true;
        if let Some(ref handler) = node.events.on_mouse_enter {
          handler();
        }
      }
    }

    let clear_active_after_dispatch = matches!(evt.kind, MouseEventKind::Up | MouseEventKind::Click);

    for (node, _) in &hits {
      match evt.kind {
        MouseEventKind::Down => {
          set_node_active(node, true);
          self.needs_redraw = true;
        }
        MouseEventKind::Up | MouseEventKind::Click => {
          set_node_active(node, false);
          self.needs_redraw = true;
        }
        _ => {}
      }
    }

    self.hover_path = current_ptrs;
    self.cursor = hits
      .iter()
      .find_map(|(node, _)| node.cursor_icon())
      .unwrap_or(CursorIcon::Default);
    if matches!(evt.kind, MouseEventKind::Down) {
      self.active_path = self.hover_path.clone();
    }
    if builtin_needs_redraw {
      self.needs_redraw = true;
    }
    drop(hits);
    if clear_active_after_dispatch {
      self.clear_active_path();
    }
    if let Some(drag) = pending_slider_drag {
      self.dragging_slider = Some(drag);
    }
    if let Some(target) = pending_focus {
      self.focus_node(target);
    }
  }

  fn dispatch_text_input(&mut self, key: &str, code: &str) -> bool {
    let focused = match self.focused_node {
      Some(id) => id,
      None => return false,
    };
    let root = match &self.root {
      Some(root) => root,
      None => return false,
    };
    let node = self
      .focused_path
      .as_deref()
      .and_then(|path| find_node_by_path(root, path))
      .or_else(|| find_node_by_id(root, focused));
    let node = match node {
      Some(node) => node,
      None => return false,
    };

    let command = code;
    let logical = key;

    match node.node_kind() {
      NodeKind::TextInput { state, .. } => match (logical, command) {
        ("Backspace", _) | (_, "Backspace") => state.backspace(),
        ("Delete", _) | (_, "Delete") => state.delete(),
        ("ArrowLeft", _) | (_, "ArrowLeft") => state.move_left(),
        ("ArrowRight", _) | (_, "ArrowRight") => state.move_right(),
        ("Home", _) | (_, "Home") => state.move_home(),
        ("End", _) | (_, "End") => state.move_end(),
        _ if key.chars().count() == 1 => state.insert(key),
        _ => return false,
      },
      NodeKind::Checkbox { state } => match (logical, command) {
        (" " | "Space" | "Enter", _) | (_, "Space" | "Enter") => state.toggle(),
        _ => return false,
      },
      NodeKind::Slider { state } => match (logical, command) {
        ("ArrowRight" | "ArrowUp", _) | (_, "ArrowRight" | "ArrowUp") => state.nudge(1.0),
        ("ArrowLeft" | "ArrowDown", _) | (_, "ArrowLeft" | "ArrowDown") => state.nudge(-1.0),
        _ => return false,
      },
      _ => return false,
    }

    true
  }

  fn focus_node(&mut self, target: FocusTarget) {
    let Some(root) = self.root.as_ref() else {
      return;
    };
    let Some(input_path) = find_path_by_id(root, target.input_id) else {
      return;
    };
    let event_path = find_path_by_id(root, target.event_id).unwrap_or_else(|| input_path.clone());

    if self.focused_path.as_ref() == Some(&input_path) && self.focused_event_path.as_ref() == Some(&event_path) {
      return;
    }

    let blur = self
      .focused_event_path
      .as_deref()
      .and_then(|path| self.root.as_ref().and_then(|root| find_node_by_path(root, path)))
      .and_then(|node| node.events.on_blur.clone());
    let focus = self
      .root
      .as_ref()
      .and_then(|root| find_node_by_path(root, &event_path))
      .and_then(|node| node.events.on_focus.clone());

    if let Some(node) = self
      .focused_path
      .as_deref()
      .and_then(|path| self.root.as_ref().and_then(|root| find_node_by_path(root, path)))
    {
      set_node_focused(node, false);
      if let NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_focused(false);
      }
    }
    if let Some(node) = self
      .focused_event_path
      .as_deref()
      .and_then(|path| self.root.as_ref().and_then(|root| find_node_by_path(root, path)))
    {
      set_node_focused(node, false);
    }
    if let Some(node) = self.root.as_ref().and_then(|root| find_node_by_path(root, &input_path)) {
      set_node_focused(node, true);
      if let NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_focused(true);
      }
    }
    if let Some(node) = self.root.as_ref().and_then(|root| find_node_by_path(root, &event_path)) {
      set_node_focused(node, true);
    }

    if let Some(handler) = blur {
      handler();
    }
    self.focused_node = Some(target.input_id);
    self.focused_event_node = Some(target.event_id);
    self.focused_path = Some(input_path);
    self.focused_event_path = Some(event_path);
    if let Some(handler) = focus {
      handler();
    }
    self.needs_redraw = true;
  }

  fn dispatch_scroll(&mut self, x: f32, y: f32, delta_x: f32, delta_y: f32, phase: ScrollPhase) {
    let mut evt = ScrollEvent {
      x,
      y,
      delta_x,
      delta_y,
      phase,
      target_id: NodeId::UNASSIGNED,
    };
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
      if let LayoutKind::ScrollModifier { state, .. } = node.layout_kind() {
        state.scroll_by(-evt.delta_x, -evt.delta_y);
        self.needs_redraw = true;
        handled = true;
        break;
      }
    }

    // Fire user handlers
    for (node, _) in &hits {
      evt.target_id = node.node_id();
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

  #[cfg(feature = "resources")]
  pub fn resource_loader(&self) -> &crate::resources::ResourceLoader {
    &self.resource_loader
  }

  #[cfg(feature = "resources")]
  pub fn resource_loader_mut(&mut self) -> &mut crate::resources::ResourceLoader {
    &mut self.resource_loader
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

  pub fn resolve_quads(&self, result: &LayoutResult) -> Vec<Quad> {
    match &self.root {
      Some(root) => self.layout_engine.resolve_quads(root, result),
      None => vec![],
    }
  }

  #[doc(hidden)]
  pub fn set_layout_constraints_override(&mut self, constraints: Option<Constraints>) {
    self.layout_constraints_override = constraints;
    self.last_layout = None;
  }

  #[doc(hidden)]
  pub fn last_layout(&self) -> Option<&LayoutResult> {
    self.last_layout.as_ref()
  }

  fn rebuild_if_dirty(&mut self) {
    if self.root_ctx.as_ref().is_some_and(Ctx::is_dirty) {
      self.rebuild();
    } else if self.root_ctx.as_ref().is_some_and(Ctx::any_dirty) {
      self.refresh_dirty_subtrees();
    }
  }

  fn apply_reactive_updates_after_event(&mut self) {
    if self.root_ctx.as_ref().is_some_and(Ctx::any_dirty) {
      self.needs_redraw = true;
      self.rebuild_if_dirty();
    }
  }

  fn refresh_dirty_subtrees(&mut self) {
    let replacements = match &mut self.root_ctx {
      Some(ctx) => ctx.refresh_dirty_subtrees(),
      None => return,
    };

    if replacements.is_empty() {
      return;
    }

    if let Some(root) = &mut self.root {
      for (slot_id, replacement) in replacements {
        replace_live_component_slot(root, slot_id, replacement, &self.id_gen);
      }
    }

    self.last_layout = None;
    self.hover_path.clear();
    self.active_path.clear();
    self.cursor = CursorIcon::Default;
    self.refresh_focus_ids();
  }

  fn update_layout(&mut self) {
    self.rebuild_if_dirty();
    self.sync_dynamic_content();
    #[cfg(all(feature = "image", feature = "resources"))]
    self.resolve_resource_images();

    if let Some(root) = self.root.as_ref() {
      let constraints = self
        .layout_constraints_override
        .unwrap_or_else(|| Constraints::tight(self.viewport_logical()));
      self.last_layout = Some(self.layout_engine.compute(&mut self.glyph_engine, root, constraints));
    }
  }

  fn sync_dynamic_content(&mut self) {
    if let Some(root) = &mut self.root {
      root.sync_dynamic_content_recursive();
    }
  }

  #[cfg(all(feature = "image", feature = "resources"))]
  fn resolve_resource_images(&mut self) {
    if let Some(root) = &mut self.root {
      Self::resolve_resource_images_recursive(root, &self.resource_loader);
    }
  }

  #[cfg(all(feature = "image", feature = "resources"))]
  fn resolve_resource_images_recursive(
    node: &mut Node,
    loader: &crate::resources::ResourceLoader,
  ) {
    if let NodeKind::ResourceImage { path } = node.node_kind() {
      let key: std::sync::Arc<str> = path.clone();
      if let crate::resources::LoadResourceResult::Loaded(bytes) = loader.load_resource(&key, None) {
        if let Ok(img) = crate::images::ImageData::from_bytes(&bytes) {
          node.intrinsic_size = Some(Size::new(img.width() as f32, img.height() as f32));
          node.node_kind = NodeKind::Image { data: img };
        }
      }
    }
    for child in &mut node.children {
      Self::resolve_resource_images_recursive(child, loader);
    }
  }

  fn clear_active_path(&mut self) {
    for old_ptr in self.active_path.drain(..) {
      let node = unsafe { &*(old_ptr as *const Node) };
      set_node_active(node, false);
      self.needs_redraw = true;
    }
  }

  fn clear_focus(&mut self) {
    if let Some(node) = self
      .focused_path
      .as_deref()
      .and_then(|path| self.root.as_ref().and_then(|root| find_node_by_path(root, path)))
    {
      set_node_focused(node, false);
      if let NodeKind::TextInput { state, .. } = node.node_kind() {
        state.set_focused(false);
      }
    }
    if let Some(node) = self
      .focused_event_path
      .as_deref()
      .and_then(|path| self.root.as_ref().and_then(|root| find_node_by_path(root, path)))
    {
      set_node_focused(node, false);
    }
    self.focused_node = None;
    self.focused_event_node = None;
    self.focused_path = None;
    self.focused_event_path = None;
  }

  fn refresh_focus_ids(&mut self) {
    let Some(root) = self.root.as_ref() else {
      self.clear_focus();
      return;
    };

    self.focused_node = self
      .focused_path
      .as_deref()
      .and_then(|path| find_node_by_path(root, path))
      .map(Node::node_id);
    self.focused_event_node = self
      .focused_event_path
      .as_deref()
      .and_then(|path| find_node_by_path(root, path))
      .map(Node::node_id);

    if self.focused_node.is_none() {
      self.clear_focus();
      return;
    }

    if let Some(node) = self
      .focused_path
      .as_deref()
      .and_then(|path| self.root.as_ref().and_then(|root| find_node_by_path(root, path)))
    {
      set_node_focused(node, true);
    }
    if let Some(node) = self
      .focused_event_path
      .as_deref()
      .and_then(|path| self.root.as_ref().and_then(|root| find_node_by_path(root, path)))
    {
      set_node_focused(node, true);
    }
  }
}

#[derive(Clone)]
struct SliderDrag {
  state: SliderState,
  x: f32,
  width: f32,
}

impl SliderDrag {
  fn update(&self, x: f32) {
    let ratio = if self.width > 0.0 {
      (x - self.x) / self.width
    } else {
      0.0
    };
    self.state.set_from_ratio(ratio);
  }
}

impl Drop for Runtime {
  fn drop(&mut self) {
    if let Some(component) = self.root_component.take() {
      component.on_unmounted();
    }
  }
}

fn find_element_recursive(
  node: &mut Node,
  layout: &LayoutResult,
  abs_x: f32,
  abs_y: f32,
  parent_x: f32,
  parent_y: f32,
  predicate: &impl for<'b> Fn(ElementRef<'b>) -> bool,
) -> Option<OwnedElementRef> {
  let element = ElementRef::new(node);
  let rect = ElementRect {
    x: abs_x,
    y: abs_y,
    relative_x: abs_x - parent_x,
    relative_y: abs_y - parent_y,
    width: layout.size.width,
    height: layout.size.height,
  };

  if predicate(element) {
    let element_ref = node.element_ref_handle();
    element_ref.update(
      rect.x,
      rect.y,
      rect.relative_x,
      rect.relative_y,
      rect.width,
      rect.height,
    );
    return Some(element_ref);
  }

  for (child_layout, child_node) in layout.children.iter().zip(node.children.iter_mut()) {
    if let Some(found) = find_element_recursive(
      child_node,
      &child_layout.result,
      abs_x + child_layout.offset.x,
      abs_y + child_layout.offset.y,
      abs_x,
      abs_y,
      predicate,
    ) {
      return Some(found);
    }
  }

  None
}

fn find_node_by_id(node: &Node, id: NodeId) -> Option<&Node> {
  if node.node_id() == id {
    return Some(node);
  }

  for child in node.children() {
    if let Some(found) = find_node_by_id(child, id) {
      return Some(found);
    }
  }

  None
}

fn find_node_by_path<'a>(node: &'a Node, path: &[usize]) -> Option<&'a Node> {
  let mut current = node;
  for &index in path {
    current = current.children().get(index)?;
  }
  Some(current)
}

fn find_path_by_id(node: &Node, id: NodeId) -> Option<Vec<usize>> {
  fn visit(node: &Node, id: NodeId, path: &mut Vec<usize>) -> bool {
    if node.node_id() == id {
      return true;
    }

    for (index, child) in node.children().iter().enumerate() {
      path.push(index);
      if visit(child, id, path) {
        return true;
      }
      path.pop();
    }

    false
  }

  let mut path = Vec::new();
  visit(node, id, &mut path).then_some(path)
}

#[derive(Clone, Copy)]
struct FocusTarget {
  input_id: NodeId,
  event_id: NodeId,
}

fn set_node_hovered(node: &Node, hovered: bool) {
  node.set_style_hovered(hovered);
  if let Some(ref state) = node.interaction {
    state.set_hovered(hovered);
  }
  if let Some(ref element_ref) = node.element_ref {
    element_ref.set_hovered(hovered);
  }
}

fn set_node_active(node: &Node, active: bool) {
  node.set_style_active(active);
  if let Some(ref state) = node.interaction {
    state.set_active(active);
  }
  if let Some(ref element_ref) = node.element_ref {
    element_ref.set_active(active);
  }
}

fn set_node_focused(node: &Node, focused: bool) {
  node.set_style_focused(focused);
  if let Some(ref state) = node.interaction {
    state.set_focused(focused);
  }
  if let Some(ref element_ref) = node.element_ref {
    element_ref.set_focused(focused);
  }
}

fn reset_element_ref_flags_recursive(node: &Node) {
  node.set_style_hovered(false);
  node.set_style_active(false);
  node.set_style_focused(false);
  if let Some(ref element_ref) = node.element_ref {
    element_ref.set_hovered(false);
    element_ref.set_active(false);
    element_ref.set_focused(false);
  }
  for child in node.children() {
    reset_element_ref_flags_recursive(child);
  }
}

fn replace_live_component_slot(node: &mut Node, slot_id: u64, mut replacement: Node, id_gen: &IdGenerator) -> bool {
  if node.component_slot_id() == Some(slot_id) {
    reset_element_ref_flags_recursive(node);
    replacement.preserve_runtime_state_from(node);
    node.free_ids(id_gen);
    replacement.assign_ids(id_gen);
    *node = replacement;
    return true;
  }

  for child in &mut node.children {
    if replace_live_component_slot(child, slot_id, replacement.clone_for_reuse(), id_gen) {
      return true;
    }
  }

  false
}

fn has_dirty_element_ref_recursive(node: &Node) -> bool {
  node
    .element_ref
    .as_ref()
    .is_some_and(|element_ref| element_ref.has_layout_dirty())
    || node.children().iter().any(has_dirty_element_ref_recursive)
}

fn dispatch_builtin_pointer(
  hits: &[(&Node, crate::app::hit_test::HitRect)],
  x: f32,
  click: bool,
) -> Option<FocusTarget> {
  if !click {
    return None;
  }

  let event_id = hits
    .iter()
    .find(|(node, _)| node.events.on_focus.is_some() || node.events.on_blur.is_some())
    .map(|(node, _)| node.node_id());

  for (node, rect) in hits {
    match node.node_kind() {
      NodeKind::TextInput { .. } => {
        return Some(FocusTarget {
          input_id: node.node_id(),
          event_id: event_id.unwrap_or_else(|| node.node_id()),
        });
      }
      NodeKind::Checkbox { state } => {
        state.toggle();
        return Some(FocusTarget {
          input_id: node.node_id(),
          event_id: event_id.unwrap_or_else(|| node.node_id()),
        });
      }
      NodeKind::Slider { state } => {
        let ratio = if rect.width > 0.0 {
          (x - rect.x) / rect.width
        } else {
          0.0
        };
        state.set_from_ratio(ratio);
        return Some(FocusTarget {
          input_id: node.node_id(),
          event_id: event_id.unwrap_or_else(|| node.node_id()),
        });
      }
      _ => {}
    }
  }

  None
}

fn find_slider_by_y_recursive<'a>(
  node: &'a Node,
  layout: &'a LayoutResult,
  abs_x: f32,
  abs_y: f32,
  y: f32,
) -> Option<(&'a Node, ElementRect)> {
  for (child_layout, child_node) in layout.children.iter().zip(node.children()) {
    if let Some(found) = find_slider_by_y_recursive(
      child_node,
      &child_layout.result,
      abs_x + child_layout.offset.x,
      abs_y + child_layout.offset.y,
      y,
    ) {
      return Some(found);
    }
  }

  let rect = ElementRect {
    x: abs_x,
    y: abs_y,
    relative_x: 0.0,
    relative_y: 0.0,
    width: layout.size.width,
    height: layout.size.height,
  };

  if matches!(node.node_kind(), NodeKind::Slider { .. }) && y >= rect.y && y <= rect.y + rect.height {
    return Some((node, rect));
  }

  None
}

fn fire_keyboard_recursive(node: &Node, evt: &mut KeyboardEvent) {
  evt.target_id = node.node_id();
  if let Some(ref handler) = node.events.on_key_down {
    handler(evt);
  }
  for child in node.children() {
    fire_keyboard_recursive(child, evt);
  }
}
