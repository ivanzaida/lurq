#[cfg(feature = "perf_profile")]
use std::time::{Duration, Instant};
use std::{
  cell::{Cell, RefCell},
  sync::Arc,
};

use crate::{
  app::{
    ctx::{ModalSpec, OverlaySpec},
    glyph_engine::GlyphEngine,
    theme::{CaretMode, ThemeBorderSizes, ThemeCaret, ThemePalette, ThemeRadii, ThemeSpacing, ThemeTypography},
  },
  core::{ElementRect, ElementRef},
  layout::{
    Alignment, Constraints, Offset, Size, StackAlignment,
    layout_kind::{
      FlexParams, FlexWrap, FrameConstraints, Justify, LayoutKind, Overflow, Position, ScrollAxis, ScrollDirection,
      ScrollState,
    },
    layout_result::{ChildLayout, LayoutResult},
    quad::{ClipRect, Quad, QuadContent},
    scrollbar::{ScrollBarPlacement, ScrollBarStyle, ScrollBarVisibility},
    text_style::TextStyle,
  },
  node::{
    CheckboxStyle, TextTransformMode,
    border::{BorderPlacement, BorderRadius, ResolvedBorder, ResolvedBorders},
    color::Color,
    dimension::Dimension,
    node::Node,
    node_kind::{NodeKind, SelectState, SliderPartRect, TextInputOverflow, TextInputState, TextOverflow},
    padding::Padding,
    slider_style::SliderPartStyle,
    transform::Transform2D,
  },
};

const DEFAULT_CHECKBOX_WIDTH: f32 = 18.0;
const DEFAULT_CHECKBOX_HEIGHT: f32 = 18.0;
const DEFAULT_SLIDER_WIDTH: f32 = 120.0;
const DEFAULT_SLIDER_HEIGHT: f32 = 20.0;
const DEFAULT_SLIDER_THUMB_MIN_SIZE: f32 = 12.0;
const DEFAULT_TEXT_INPUT_WIDTH: f32 = 120.0;
const DEFAULT_SELECT_WIDTH: f32 = 160.0;
const DEFAULT_SELECT_HEIGHT: f32 = 32.0;
#[cfg(any(feature = "image", all(feature = "svg", feature = "resources")))]
const DEFAULT_RESOURCE_WIDTH: f32 = 0.0;
#[cfg(any(feature = "image", all(feature = "svg", feature = "resources")))]
const DEFAULT_RESOURCE_HEIGHT: f32 = 0.0;
const DEFAULT_QUAD_OPACITY: f32 = 1.0;
#[cfg(feature = "perf_profile")]
const SLOW_LAYOUT_NODE_THRESHOLD: Duration = Duration::from_millis(4);

#[cfg(feature = "perf_profile")]
fn layout_kind_profile_name(kind: &LayoutKind) -> &'static str {
  match kind {
    LayoutKind::Leaf => "leaf",
    LayoutKind::Row { .. } => "row",
    LayoutKind::Column { .. } => "column",
    LayoutKind::Stack { .. } => "stack",
    LayoutKind::LogicalModifier => "logical_modifier",
    LayoutKind::ScrollModifier { .. } => "scroll_modifier",
  }
}

const DEFAULT_CONTROL_SURFACE_COLOR: Color = Color::new(255, 255, 255, 255);
const DEFAULT_TRANSPARENT_COLOR: Color = Color::new(0, 0, 0, 0);
const DEFAULT_CHECKBOX_CHECKED_COLOR: Color = Color::new(34, 197, 94, 255);
const DEFAULT_SLIDER_TRACK_COLOR: Color = Color::new(203, 213, 225, 255);
const DEFAULT_SLIDER_THUMB_COLOR: Color = Color::new(71, 85, 105, 255);
const DEFAULT_TEXT_SELECTION_COLOR: Color = Color::new(191, 219, 254, 255);
const DEFAULT_CARET_COLOR: Color = Color::new(15, 23, 42, 255);
fn text_input_display_style<'a>(
  state: &crate::node::node_kind::TextInputState,
  style: &'a TextStyle,
  placeholder_style: Option<&'a TextStyle>,
) -> &'a TextStyle {
  if state.is_showing_placeholder() {
    placeholder_style.unwrap_or(style)
  } else {
    style
  }
}

fn text_input_vertical_offset(state: &TextInputState, height: f32) -> f32 {
  if state.overflow() == TextInputOverflow::Scroll {
    ((height - state.caret_height()).max(0.0)) * 0.5
  } else {
    0.0
  }
}

fn bounded_text_width(width: f32) -> bool {
  width.is_finite() && width < f32::MAX
}

fn constraints_are_tight(constraints: Constraints) -> bool {
  constraints.min_width == constraints.max_width && constraints.min_height == constraints.max_height
}

#[derive(Clone, Copy, Default)]
pub(crate) struct ResolvedPadding {
  pub(crate) left: f32,
  pub(crate) top: f32,
  pub(crate) right: f32,
  pub(crate) bottom: f32,
}

#[derive(Clone)]
struct ChildLayoutOverride {
  constraints: Constraints,
  result: LayoutResult,
}

#[derive(Default)]
pub(crate) struct OverlayLayoutIndex {
  pub(crate) elements: Vec<ElementLayoutRecord>,
  pub(crate) overlays: Vec<OverlayLayoutRecord>,
}

#[derive(Clone)]
pub(crate) struct ElementLayoutRecord {
  pub(crate) element_ref: ElementRef,
  pub(crate) rect: ElementRect,
}

pub(crate) enum OverlayLayoutRecord {
  SelectMenu {
    reuse_key: Option<Arc<str>>,
    state: SelectState,
    bounds: ElementRect,
  },
  Overlay {
    reuse_key: Option<Arc<str>>,
    spec: OverlaySpec,
  },
  Modal {
    reuse_key: Option<Arc<str>>,
    spec: ModalSpec,
    parent: ElementRect,
  },
}

impl Clone for OverlayLayoutIndex {
  fn clone(&self) -> Self {
    Self {
      elements: self.elements.clone(),
      overlays: self.overlays.clone(),
    }
  }
}

impl Clone for OverlayLayoutRecord {
  fn clone(&self) -> Self {
    match self {
      Self::SelectMenu {
        reuse_key,
        state,
        bounds,
      } => Self::SelectMenu {
        reuse_key: reuse_key.clone(),
        state: state.clone(),
        bounds: *bounds,
      },
      Self::Overlay { reuse_key, spec } => Self::Overlay {
        reuse_key: reuse_key.clone(),
        spec: spec.clone_for_reuse(),
      },
      Self::Modal {
        reuse_key,
        spec,
        parent,
      } => Self::Modal {
        reuse_key: reuse_key.clone(),
        spec: spec.clone_for_reuse(),
        parent: *parent,
      },
    }
  }
}

pub(crate) struct LayoutEngine {
  last_recalculated: Cell<bool>,
  text_input_caret_visible: Cell<bool>,
  palette: RefCell<ThemePalette>,
  border_sizes: RefCell<ThemeBorderSizes>,
  spacing: RefCell<ThemeSpacing>,
  radii: RefCell<ThemeRadii>,
  caret: RefCell<ThemeCaret>,
  scrollbar: RefCell<ScrollBarStyle>,
  typography: RefCell<ThemeTypography>,
}

#[cfg(feature = "image")]
struct BackgroundImagePlacement {
  x: f32,
  y: f32,
  width: f32,
  height: f32,
  uv_min: [f32; 2],
  uv_max: [f32; 2],
}

#[cfg(feature = "image")]
fn background_image_placement(
  size_mode: crate::node::BackgroundSize,
  box_width: f32,
  box_height: f32,
  image_width: f32,
  image_height: f32,
) -> BackgroundImagePlacement {
  let full = BackgroundImagePlacement {
    x: 0.0,
    y: 0.0,
    width: box_width,
    height: box_height,
    uv_min: [0.0, 0.0],
    uv_max: [1.0, 1.0],
  };

  if box_width <= 0.0 || box_height <= 0.0 || image_width <= 0.0 || image_height <= 0.0 {
    return full;
  }

  match size_mode {
    crate::node::BackgroundSize::Stretch => full,
    crate::node::BackgroundSize::Contain => {
      let scale = (box_width / image_width).min(box_height / image_height);
      let width = image_width * scale;
      let height = image_height * scale;
      BackgroundImagePlacement {
        x: (box_width - width) * 0.5,
        y: (box_height - height) * 0.5,
        width,
        height,
        uv_min: [0.0, 0.0],
        uv_max: [1.0, 1.0],
      }
    }
    crate::node::BackgroundSize::Cover => {
      let box_aspect = box_width / box_height;
      let image_aspect = image_width / image_height;
      let mut uv_min = [0.0, 0.0];
      let mut uv_max = [1.0, 1.0];

      if image_aspect > box_aspect {
        let visible_u = box_aspect / image_aspect;
        uv_min[0] = (1.0 - visible_u) * 0.5;
        uv_max[0] = 1.0 - uv_min[0];
      } else if image_aspect < box_aspect {
        let visible_v = image_aspect / box_aspect;
        uv_min[1] = (1.0 - visible_v) * 0.5;
        uv_max[1] = 1.0 - uv_min[1];
      }

      BackgroundImagePlacement { uv_min, uv_max, ..full }
    }
  }
}

fn push_slider_part_quads(
  quads: &mut Vec<Quad>,
  mut rect: SliderPartRect,
  style: &SliderPartStyle,
  color: Color,
  border_radius: Option<BorderRadius>,
  border: Option<ResolvedBorders>,
  opacity: f32,
  transform: Transform2D,
  clip: ClipRect,
) {
  if transform.is_identity() {
    rect.x = rect.x.round();
    rect.y = rect.y.round();
  }

  #[cfg(not(feature = "image"))]
  let _ = style;
  #[cfg(feature = "image")]
  let has_image = style.background_image.is_some();
  #[cfg(not(feature = "image"))]
  let has_image = false;

  let (rect_x, rect_y, rect_transform, rect_transform_origin) = transformed_quad_frame(rect.x, rect.y, transform);
  quads.push(Quad {
    x: rect_x,
    y: rect_y,
    width: rect.width,
    height: rect.height,
    opacity,
    transform: rect_transform,
    transform_origin: rect_transform_origin,
    content: QuadContent::Rect { color, gradient: None },
    border_radius,
    border: if has_image { None } else { border },
    clip,
  });

  #[cfg(feature = "image")]
  if let Some(ref bg_image) = style.background_image {
    let placement = background_image_placement(
      style.background_size,
      rect.width,
      rect.height,
      bg_image.width() as f32,
      bg_image.height() as f32,
    );
    let image_x = rect.x + placement.x;
    let image_y = rect.y + placement.y;
    let (image_x, image_y, image_transform, image_transform_origin) =
      transformed_quad_frame(image_x, image_y, transform);
    quads.push(Quad {
      x: image_x,
      y: image_y,
      width: placement.width,
      height: placement.height,
      opacity,
      transform: image_transform,
      transform_origin: image_transform_origin,
      content: QuadContent::Image {
        data: bg_image.clone(),
        uv_min: placement.uv_min,
        uv_max: placement.uv_max,
      },
      border_radius,
      border: None,
      clip,
    });
  }

  if has_image && border.is_some() {
    let (rect_x, rect_y, rect_transform, rect_transform_origin) = transformed_quad_frame(rect.x, rect.y, transform);
    quads.push(Quad {
      x: rect_x,
      y: rect_y,
      width: rect.width,
      height: rect.height,
      opacity,
      transform: rect_transform,
      transform_origin: rect_transform_origin,
      content: QuadContent::Rect {
        color: DEFAULT_TRANSPARENT_COLOR,
        gradient: None,
      },
      border_radius,
      border,
      clip,
    });
  }
}

fn push_checkbox_quads(
  quads: &mut Vec<Quad>,
  rect: SliderPartRect,
  style: &CheckboxStyle,
  color: Color,
  border_radius: Option<BorderRadius>,
  border: Option<ResolvedBorders>,
  checked: bool,
  opacity: f32,
  transform: Transform2D,
  clip: ClipRect,
) {
  #[cfg(not(feature = "image"))]
  let _ = (style, checked);

  let (rect_x, rect_y, rect_transform, rect_transform_origin) = transformed_quad_frame(rect.x, rect.y, transform);
  quads.push(Quad {
    x: rect_x,
    y: rect_y,
    width: rect.width,
    height: rect.height,
    opacity,
    transform: rect_transform,
    transform_origin: rect_transform_origin,
    content: QuadContent::Rect { color, gradient: None },
    border_radius,
    border,
    clip,
  });

  #[cfg(feature = "image")]
  if checked && let Some(ref indicator_image) = style.indicator_image {
    let indicator_width = style
      .indicator_width
      .unwrap_or(rect.width * 0.65)
      .min(rect.width)
      .max(0.0);
    let indicator_height = style
      .indicator_height
      .unwrap_or(rect.height * 0.65)
      .min(rect.height)
      .max(0.0);
    let indicator_x = rect.x + (rect.width - indicator_width) * 0.5;
    let indicator_y = rect.y + (rect.height - indicator_height) * 0.5;
    let placement = background_image_placement(
      style.indicator_size,
      indicator_width,
      indicator_height,
      indicator_image.width() as f32,
      indicator_image.height() as f32,
    );
    let image_x = indicator_x + placement.x;
    let image_y = indicator_y + placement.y;
    let (image_x, image_y, image_transform, image_transform_origin) =
      transformed_quad_frame(image_x, image_y, transform);
    quads.push(Quad {
      x: image_x,
      y: image_y,
      width: placement.width,
      height: placement.height,
      opacity,
      transform: image_transform,
      transform_origin: image_transform_origin,
      content: QuadContent::Image {
        data: indicator_image.clone(),
        uv_min: placement.uv_min,
        uv_max: placement.uv_max,
      },
      border_radius: None,
      border: None,
      clip,
    });
  }
}

fn quad_transform(transform: Transform2D) -> Transform2D {
  transform.linear_part()
}

fn transformed_quad_frame(x: f32, y: f32, transform: Transform2D) -> (f32, f32, Transform2D, Option<[f32; 2]>) {
  if transform.is_identity() {
    return (x, y, Transform2D::IDENTITY, None);
  }

  let (x, y) = transform.transform_point(x, y);
  let linear = quad_transform(transform);
  let origin = if linear.is_identity() { None } else { Some([0.0, 0.0]) };
  (x, y, linear, origin)
}

impl LayoutEngine {
  pub(crate) fn new() -> Self {
    Self {
      last_recalculated: Cell::new(false),
      text_input_caret_visible: Cell::new(true),
      palette: RefCell::new(ThemePalette::default()),
      border_sizes: RefCell::new(ThemeBorderSizes::default()),
      spacing: RefCell::new(ThemeSpacing::default()),
      radii: RefCell::new(ThemeRadii::default()),
      caret: RefCell::new(ThemeCaret::default()),
      scrollbar: RefCell::new(ScrollBarStyle::default()),
      typography: RefCell::new(ThemeTypography::default()),
    }
  }

  pub(crate) fn compute(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    palette: ThemePalette,
    border_sizes: ThemeBorderSizes,
    spacing: ThemeSpacing,
    radii: ThemeRadii,
    caret: ThemeCaret,
    scrollbar: ScrollBarStyle,
    typography: ThemeTypography,
    force_dirty: bool,
  ) -> LayoutResult {
    self.compute_inner(
      glyph_engine,
      node,
      constraints,
      palette,
      border_sizes,
      spacing,
      radii,
      caret,
      scrollbar,
      typography,
      force_dirty,
    )
  }

  #[allow(clippy::too_many_arguments)]
  pub(crate) fn compute_with_overlay_index(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    palette: ThemePalette,
    border_sizes: ThemeBorderSizes,
    spacing: ThemeSpacing,
    radii: ThemeRadii,
    caret: ThemeCaret,
    scrollbar: ScrollBarStyle,
    typography: ThemeTypography,
    force_dirty: bool,
  ) -> (LayoutResult, OverlayLayoutIndex) {
    let result = self.compute_inner(
      glyph_engine,
      node,
      constraints,
      palette,
      border_sizes,
      spacing,
      radii,
      caret,
      scrollbar,
      typography,
      force_dirty,
    );
    let index = Self::collect_overlay_index(node, &result);
    (result, index)
  }

  #[allow(clippy::too_many_arguments)]
  fn compute_inner(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    palette: ThemePalette,
    border_sizes: ThemeBorderSizes,
    spacing: ThemeSpacing,
    radii: ThemeRadii,
    caret: ThemeCaret,
    scrollbar: ScrollBarStyle,
    typography: ThemeTypography,
    force_dirty: bool,
  ) -> LayoutResult {
    self.last_recalculated.set(false);
    *self.palette.borrow_mut() = palette;
    *self.border_sizes.borrow_mut() = border_sizes;
    *self.spacing.borrow_mut() = spacing;
    *self.radii.borrow_mut() = radii;
    *self.caret.borrow_mut() = caret;
    *self.scrollbar.borrow_mut() = scrollbar;
    *self.typography.borrow_mut() = typography;
    Self::mark_layout_dirty(node, force_dirty);
    let result = self.layout_node(glyph_engine, node, constraints);
    node.clear_guards();
    result
  }

  fn collect_overlay_index(node: &Node, layout: &LayoutResult) -> OverlayLayoutIndex {
    let mut index = OverlayLayoutIndex::default();
    Self::collect_overlay_index_recursive(node, layout, 0.0, 0.0, 0.0, 0.0, &mut index);
    index
  }

  fn collect_overlay_index_recursive(
    node: &Node,
    layout: &LayoutResult,
    abs_x: f32,
    abs_y: f32,
    parent_x: f32,
    parent_y: f32,
    index: &mut OverlayLayoutIndex,
  ) {
    let rect = ElementRect {
      x: abs_x,
      y: abs_y,
      relative_x: abs_x - parent_x,
      relative_y: abs_y - parent_y,
      width: layout.size.width,
      height: layout.size.height,
    };

    if let Some(element_ref) = node.element_ref.as_ref() {
      index.elements.push(ElementLayoutRecord {
        element_ref: element_ref.clone(),
        rect,
      });
    }

    if let NodeKind::Select { state } = node.node_kind()
      && state.is_open()
    {
      index.overlays.push(OverlayLayoutRecord::SelectMenu {
        reuse_key: node
          .node_id()
          .is_assigned()
          .then(|| Arc::<str>::from(format!("select:{}", node.node_id().value()))),
        state: state.clone(),
        bounds: rect,
      });
    }

    if let Some(spec) = node.overlay_declaration() {
      index.overlays.push(OverlayLayoutRecord::Overlay {
        reuse_key: spec
          .node
          .component_key()
          .map(|key| Arc::<str>::from(format!("overlay:{key}"))),
        spec: spec.clone_for_reuse(),
      });
    }

    for (child_layout, child) in layout.children.iter().zip(node.children()) {
      if let Some(spec) = child.modal_declaration() {
        index.overlays.push(OverlayLayoutRecord::Modal {
          reuse_key: spec
            .node
            .component_key()
            .map(|key| Arc::<str>::from(format!("modal:{key}"))),
          spec: spec.clone_for_reuse(),
          parent: rect,
        });
      }

      Self::collect_overlay_index_recursive(
        child,
        &child_layout.result,
        abs_x + child_layout.offset.x,
        abs_y + child_layout.offset.y,
        abs_x,
        abs_y,
        index,
      );
    }
  }

  #[cfg_attr(not(feature = "perf_profile"), allow(dead_code))]
  pub(crate) fn last_recalculated(&self) -> bool {
    self.last_recalculated.get()
  }

  pub(crate) fn set_text_input_caret_visible(&self, visible: bool) {
    self.text_input_caret_visible.set(visible);
  }

  fn should_render_text_input_caret(&self, node: &Node) -> bool {
    match node.caret_mode_value().unwrap_or_else(|| self.caret.borrow().mode()) {
      CaretMode::Persistent => true,
      CaretMode::Blinking => self.text_input_caret_visible.get(),
    }
  }

  fn mark_layout_dirty(node: &Node, force_dirty: bool) -> bool {
    // A node without a cached result has either never been laid out or had its
    // cache invalidated by a layout-affecting change (e.g. a reconciled subtree
    // patched in via a component slot replacement). It must be laid out, and its
    // ancestors must recompute to reposition it, so propagate dirtiness upward.
    let text_input_dirty = match node.node_kind() {
      NodeKind::TextInput { state, .. } => state.take_layout_dirty(),
      NodeKind::Select { state } => state.take_layout_dirty(),
      _ => false,
    };
    let mut local_dirty =
      force_dirty || node.text_content.is_changed() || text_input_dirty || !node.layout_cache.has_cached_result();

    // A cached result that contradicts the node's own fixed frame is stale no
    // matter how it got there (observed in production: retained-tree diffs can
    // transplant a cache across a frame change, leaving a clean-flagged node
    // laid at an old size — e.g. a virtualized list's spacer holding a
    // previous window's height, which blanks the viewport). Only sizes the
    // cached constraints would have permitted count as conflicts, so nodes
    // legitimately clamped by their parent are left alone.
    if !local_dirty && let Some((cached_constraints, cached_size)) = node.layout_cache.cached_entry() {
      let width_conflicts = matches!(
        node.frame.width,
        Some(Dimension::Px(px))
          if (cached_size.width - px).abs() > 0.5
            && px <= cached_constraints.max_width + 0.5
            && px >= cached_constraints.min_width - 0.5
      );
      let height_conflicts = matches!(
        node.frame.height,
        Some(Dimension::Px(px))
          if (cached_size.height - px).abs() > 0.5
            && px <= cached_constraints.max_height + 0.5
            && px >= cached_constraints.min_height - 0.5
      );
      if width_conflicts || height_conflicts {
        local_dirty = true;
      }
    }

    let scroll_offset_dirty = matches!(
      node.layout_kind(),
      LayoutKind::ScrollModifier { state, .. } if state.take_scroll_dirty()
    );

    if node
      .element_ref
      .as_ref()
      .is_some_and(|element_ref| element_ref.take_layout_dirty())
    {
      local_dirty = true;
    }

    if node.take_style_layout_dirty() {
      local_dirty = true;
    }

    let mut child_dirty = false;
    for child in node.children() {
      child_dirty |= Self::mark_layout_dirty(child, force_dirty);
    }

    if local_dirty {
      node.layout_cache.mark_local_dirty();
    }
    if child_dirty || scroll_offset_dirty {
      node.layout_cache.mark_descendant_dirty();
    }

    local_dirty || child_dirty || scroll_offset_dirty
  }

  pub(crate) fn resolve_quads(&self, node: &Node, result: &LayoutResult) -> Vec<Quad> {
    self.resolve_quads_with_viewport(node, result, ClipRect::default())
  }

  pub(crate) fn resolve_quads_with_viewport(
    &self,
    node: &Node,
    result: &LayoutResult,
    viewport: ClipRect,
  ) -> Vec<Quad> {
    let mut quads = Vec::new();
    self.resolve_quads_with_viewport_into(node, result, viewport, &mut quads);
    quads
  }

  pub(crate) fn resolve_quads_with_viewport_into(
    &self,
    node: &Node,
    result: &LayoutResult,
    viewport: ClipRect,
    quads: &mut Vec<Quad>,
  ) {
    let root_offset = node.offset_position().unwrap_or_default();
    self.collect_quads(
      node,
      result,
      root_offset.x,
      root_offset.y,
      0.0,
      0.0,
      Transform2D::IDENTITY,
      viewport,
      viewport,
      true,
      quads,
    );
  }

  fn collect_quads(
    &self,
    node: &Node,
    result: &LayoutResult,
    abs_x: f32,
    abs_y: f32,
    parent_x: f32,
    parent_y: f32,
    inherited_transform: Transform2D,
    clip: ClipRect,
    cull_clip: ClipRect,
    culling_enabled: bool,
    quads: &mut Vec<Quad>,
  ) {
    if node_is_plain_logical_wrapper(node) {
      if let Some(ref element_ref) = node.element_ref {
        element_ref.update(
          abs_x,
          abs_y,
          abs_x - parent_x,
          abs_y - parent_y,
          result.size.width,
          result.size.height,
        );
      }

      let (child_clip, child_cull_clip, child_culling_enabled) =
        if node.overflow == Overflow::Hidden && inherited_transform.is_identity() {
          let overflow_clip = intersect_clip(
            clip,
            ClipRect {
              x: abs_x,
              y: abs_y,
              width: result.size.width,
              height: result.size.height,
              active: true,
              border_radius: None,
            },
          );
          let child_cull_clip = if culling_enabled {
            intersect_clip(cull_clip, overflow_clip)
          } else {
            ClipRect::default()
          };
          (overflow_clip, child_cull_clip, culling_enabled)
        } else {
          (clip, cull_clip, culling_enabled)
        };

      for (child_layout, child_node) in result.children.iter().zip(node.children().iter()) {
        let child_abs_x = abs_x + child_layout.offset.x;
        let child_abs_y = abs_y + child_layout.offset.y;
        if child_culling_enabled
          && clipped_subtree_is_hidden(
            child_node,
            &child_layout.result,
            child_abs_x,
            child_abs_y,
            inherited_transform,
            child_cull_clip,
          )
        {
          continue;
        }

        self.collect_quads(
          child_node,
          &child_layout.result,
          child_abs_x,
          child_abs_y,
          abs_x,
          abs_y,
          inherited_transform,
          child_clip,
          child_cull_clip,
          child_culling_enabled,
          quads,
        );
      }
      return;
    }

    if let Some(ref element_ref) = node.element_ref {
      element_ref.update(
        abs_x,
        abs_y,
        abs_x - parent_x,
        abs_y - parent_y,
        result.size.width,
        result.size.height,
      );
    }

    let background_color = node.resolved_color(&self.palette.borrow());
    let background_gradient = node
      .resolved_gradient()
      .and_then(|gradient| crate::layout::render_list::RenderGradient::resolve(&gradient, &self.palette.borrow()));
    let resolved_border = node.get_resolved_border(&self.palette.borrow(), &self.border_sizes.borrow());
    let has_visual = background_color.is_some() || background_gradient.is_some() || resolved_border.is_some();
    let defer_border_to_overlay = resolved_border.is_some() && has_visual && !node.children().is_empty();
    let content = match node.node_kind() {
      NodeKind::Text {
        state,
        style,
        transform_mode,
      } => {
        let resolved_style = style.resolve(&self.typography.borrow(), &self.palette.borrow());
        let vertical_align = resolved_style.vertical_align;
        QuadContent::Text {
          text: state
            .display_text()
            .unwrap_or_else(|| node.text_content().unwrap_or_default().to_owned()),
          style: resolved_style,
          wrap: state.render_wrap(),
          vertical_align,
          center_using_ink_bounds: false,
          transform_mode: *transform_mode,
        }
      }
      #[cfg(feature = "markdown")]
      NodeKind::RichText {
        spans, transform_mode, ..
      } => QuadContent::RichText {
        spans: spans.clone(),
        wrap: node.text_wrap && node.text_overflow == TextOverflow::Clip,
        vertical_align: crate::layout::text_style::VerticalAlign::Center,
        transform_mode: *transform_mode,
      },
      NodeKind::TextInput {
        state,
        style,
        placeholder_style,
      } => {
        let display_style = text_input_display_style(state, style, placeholder_style.as_ref()).clone();
        // Single-line inputs center their glyph ink in the box; multi-line
        // inputs flow from the top and scroll.
        let vertical_align = if state.overflow() == crate::node::node_kind::TextInputOverflow::Multiline {
          crate::layout::text_style::VerticalAlign::Top
        } else {
          display_style.vertical_align
        };
        QuadContent::Text {
          text: state.rendered_text_for_layout(),
          style: display_style,
          wrap: state.overflow() == crate::node::node_kind::TextInputOverflow::Multiline,
          vertical_align,
          center_using_ink_bounds: vertical_align == crate::layout::text_style::VerticalAlign::Center,
          transform_mode: TextTransformMode::Bitmap,
        }
      }
      NodeKind::Checkbox { .. } => QuadContent::None,
      #[cfg(feature = "image")]
      NodeKind::Image { data } => QuadContent::Image {
        data: data.clone(),
        uv_min: [0.0, 0.0],
        uv_max: [1.0, 1.0],
      },
      #[cfg(feature = "image")]
      NodeKind::Video { data, fit } => {
        let placement = background_image_placement(
          *fit,
          result.size.width,
          result.size.height,
          data.width() as f32,
          data.height() as f32,
        );
        QuadContent::Video {
          data: data.clone(),
          uv_min: placement.uv_min,
          uv_max: placement.uv_max,
        }
      }
      #[cfg(feature = "image")]
      NodeKind::ResourceImage { .. } => QuadContent::None,
      #[cfg(feature = "svg")]
      NodeKind::Svg { data } => QuadContent::Svg { data: data.clone() },
      #[cfg(all(feature = "svg", feature = "resources"))]
      NodeKind::ResourceSvg { .. } => QuadContent::None,
      NodeKind::Slider { .. } => QuadContent::None,
      _ if has_visual => QuadContent::Rect {
        color: background_color.unwrap_or(DEFAULT_TRANSPARENT_COLOR),
        gradient: background_gradient.clone(),
      },
      _ => QuadContent::None,
    };

    let opacity = node.opacity;
    let local_transform = node.effective_transform();
    let local_transform_origin_abs = [abs_x + result.size.width * 0.5, abs_y + result.size.height * 0.5];
    let local_affine = if local_transform.is_identity() {
      Transform2D::IDENTITY
    } else {
      local_transform.around_origin(local_transform_origin_abs)
    };
    let transform = if inherited_transform.is_identity() {
      local_affine
    } else if local_affine.is_identity() {
      inherited_transform
    } else if local_transform.is_identity() {
      inherited_transform
    } else {
      inherited_transform.then(&local_affine)
    };

    match &content {
      QuadContent::None => {}
      _ => {
        if matches!(content, QuadContent::Text { .. } | QuadContent::RichText { .. }) && has_visual {
          let (visual_x, visual_y, visual_transform, visual_transform_origin) =
            transformed_quad_frame(abs_x, abs_y, transform);
          quads.push(Quad {
            x: visual_x,
            y: visual_y,
            width: result.size.width,
            height: result.size.height,
            opacity,
            transform: visual_transform,
            transform_origin: visual_transform_origin,
            content: QuadContent::Rect {
              color: background_color.unwrap_or(DEFAULT_TRANSPARENT_COLOR),
              gradient: background_gradient,
            },
            border_radius: node.get_border_radius(&self.radii.borrow()),
            border: if defer_border_to_overlay { None } else { resolved_border },
            clip,
          });
        }

        if let NodeKind::Text { state, style, .. } = node.node_kind()
          && state.selectable()
        {
          let style = style.resolve(&self.typography.borrow(), &self.palette.borrow());
          let palette = self.palette.borrow();
          let selection_color = node
            .selection_color_value()
            .and_then(|color| color.resolve(&palette))
            .unwrap_or(DEFAULT_TEXT_SELECTION_COLOR);
          let selection_height = (style.font_size * style.line_height).min(result.size.height).max(1.0);
          let selection_clip = clip;
          for selection in state.selection_ranges(node.text_content().unwrap_or_default()) {
            let selection_x = abs_x + selection.x;
            let selection_y = abs_y + selection.y;
            let (selection_x, selection_y, selection_transform, selection_transform_origin) =
              transformed_quad_frame(selection_x, selection_y, transform);
            quads.push(Quad {
              x: selection_x,
              y: selection_y,
              width: selection.width,
              height: selection_height,
              opacity,
              transform: selection_transform,
              transform_origin: selection_transform_origin,
              content: QuadContent::Rect {
                color: selection_color,
                gradient: None,
              },
              border_radius: None,
              border: None,
              clip: selection_clip,
            });
          }
        }
        #[cfg(feature = "markdown")]
        if let NodeKind::RichText { state, spans, .. } = node.node_kind()
          && state.selectable()
        {
          let palette = self.palette.borrow();
          let selection_color = node
            .selection_color_value()
            .and_then(|color| color.resolve(&palette))
            .unwrap_or(DEFAULT_TEXT_SELECTION_COLOR);
          let style = spans.first().map(|span| &span.style);
          let selection_height = style
            .map(|style| style.font_size * style.line_height)
            .unwrap_or(16.0)
            .min(result.size.height)
            .max(1.0);
          let selection_clip = clip;
          for selection in state.selection_ranges(node.text_content().unwrap_or_default()) {
            let selection_x = abs_x + selection.x;
            let selection_y = abs_y + selection.y;
            let (selection_x, selection_y, selection_transform, selection_transform_origin) =
              transformed_quad_frame(selection_x, selection_y, transform);
            quads.push(Quad {
              x: selection_x,
              y: selection_y,
              width: selection.width,
              height: selection_height,
              opacity,
              transform: selection_transform,
              transform_origin: selection_transform_origin,
              content: QuadContent::Rect {
                color: selection_color,
                gradient: None,
              },
              border_radius: None,
              border: None,
              clip: selection_clip,
            });
          }
        }

        if let NodeKind::TextInput { state, .. } = node.node_kind()
          && state.is_focused()
        {
          let palette = self.palette.borrow();
          let selection_color = node
            .selection_color_value()
            .and_then(|color| color.resolve(&palette))
            .unwrap_or(DEFAULT_TEXT_SELECTION_COLOR);
          let padding = self.resolved_padding_for_size(node, result.size);
          let content_width = (result.size.width - padding.left - padding.right).max(0.0);
          let content_height = (result.size.height - padding.top - padding.bottom).max(0.0);
          let selection_height = state.caret_height().min(content_height).max(1.0);
          let vertical_offset = padding.top + text_input_vertical_offset(state, content_height);
          let selection_clip = intersect_clip(
            clip,
            ClipRect {
              x: abs_x + padding.left,
              y: abs_y + padding.top,
              width: content_width,
              height: content_height,
              active: true,
              border_radius: None,
            },
          );
          for selection in state.selection_ranges() {
            let selection_x = abs_x + padding.left + selection.x;
            let selection_y = abs_y + vertical_offset + selection.y;
            let (selection_x, selection_y, selection_transform, selection_transform_origin) =
              transformed_quad_frame(selection_x, selection_y, transform);
            quads.push(Quad {
              x: selection_x,
              y: selection_y,
              width: selection.width,
              height: selection_height,
              opacity,
              transform: selection_transform,
              transform_origin: selection_transform_origin,
              content: QuadContent::Rect {
                color: selection_color,
                gradient: None,
              },
              border_radius: None,
              border: None,
              clip: selection_clip,
            });
          }
        }

        let (content_x, content_y, content_width, content_height, content_clip) = match node.node_kind() {
          NodeKind::TextInput { state, .. } => {
            let padding = self.resolved_padding_for_size(node, result.size);
            let content_width = (result.size.width - padding.left - padding.right).max(0.0);
            let content_height = (result.size.height - padding.top - padding.bottom).max(0.0);
            let scroll_x = state.scroll_x();
            let scroll_y = state.scroll_y();
            // The glyph run is vertically aligned by the render path (via the
            // quad's `vertical_align`), so the content box starts at the top of
            // the padding box here — no metric centering offset is baked in.
            if matches!(
              state.overflow(),
              TextInputOverflow::Scroll | TextInputOverflow::Multiline
            ) {
              (
                abs_x + padding.left - scroll_x,
                abs_y + padding.top - scroll_y,
                content_width + scroll_x,
                content_height + scroll_y,
                intersect_clip(
                  clip,
                  ClipRect {
                    x: abs_x + padding.left,
                    y: abs_y + padding.top,
                    width: content_width,
                    height: content_height,
                    active: true,
                    border_radius: None,
                  },
                ),
              )
            } else {
              (
                abs_x + padding.left,
                abs_y + padding.top,
                content_width,
                content_height,
                clip,
              )
            }
          }
          #[cfg(feature = "image")]
          NodeKind::Video { data, fit } => {
            let placement = background_image_placement(
              *fit,
              result.size.width,
              result.size.height,
              data.width() as f32,
              data.height() as f32,
            );
            (
              abs_x + placement.x,
              abs_y + placement.y,
              placement.width,
              placement.height,
              clip,
            )
          }
          _ => (abs_x, abs_y, result.size.width, result.size.height, clip),
        };

        let content_uses_separate_visual_rect =
          matches!(content, QuadContent::Text { .. } | QuadContent::RichText { .. }) && has_visual;
        let (content_x, content_y, content_transform, content_transform_origin) =
          transformed_quad_frame(content_x, content_y, transform);
        quads.push(Quad {
          x: content_x,
          y: content_y,
          width: content_width,
          height: content_height,
          opacity,
          transform: content_transform,
          transform_origin: content_transform_origin,
          content,
          border_radius: if content_uses_separate_visual_rect {
            None
          } else {
            node.get_border_radius(&self.radii.borrow())
          },
          border: if content_uses_separate_visual_rect {
            None
          } else if defer_border_to_overlay {
            None
          } else {
            resolved_border
          },
          clip: content_clip,
        });
      }
    }

    #[cfg(feature = "image")]
    if let Some(ref bg_image) = *node.background_image {
      let placement = background_image_placement(
        node.background_size,
        result.size.width,
        result.size.height,
        bg_image.width() as f32,
        bg_image.height() as f32,
      );
      let image_x = abs_x + placement.x;
      let image_y = abs_y + placement.y;
      let (image_x, image_y, image_transform, image_transform_origin) =
        transformed_quad_frame(image_x, image_y, transform);
      quads.push(Quad {
        x: image_x,
        y: image_y,
        width: placement.width,
        height: placement.height,
        opacity,
        transform: image_transform,
        transform_origin: image_transform_origin,
        content: QuadContent::Image {
          data: bg_image.clone(),
          uv_min: placement.uv_min,
          uv_max: placement.uv_max,
        },
        border_radius: node.get_border_radius(&self.radii.borrow()),
        border: None,
        clip,
      });
    }

    match node.node_kind() {
      NodeKind::TextInput { state, style, .. } if state.is_focused() && self.should_render_text_input_caret(node) => {
        let padding = self.resolved_padding_for_size(node, result.size);
        let content_width = (result.size.width - padding.left - padding.right).max(0.0);
        let content_height = (result.size.height - padding.top - padding.bottom).max(0.0);
        let caret_height = state.caret_height().min(content_height).max(1.0);
        let vertical_offset = padding.top + text_input_vertical_offset(state, content_height);
        let caret_x = abs_x + padding.left + state.caret_x();
        let caret_y = abs_y + vertical_offset + state.caret_y();
        let palette = self.palette.borrow();
        let caret_color = node
          .caret_color_value()
          .and_then(|color| color.resolve(&palette))
          .or_else(|| style.caret_color.as_ref().and_then(|color| color.resolve(&palette)))
          .unwrap_or(DEFAULT_CARET_COLOR);
        let (caret_x, caret_y, caret_transform, caret_transform_origin) =
          transformed_quad_frame(caret_x, caret_y, transform);
        quads.push(Quad {
          x: caret_x,
          y: caret_y,
          width: 1.0,
          height: caret_height,
          opacity,
          transform: caret_transform,
          transform_origin: caret_transform_origin,
          content: QuadContent::Rect {
            color: caret_color,
            gradient: None,
          },
          border_radius: None,
          border: None,
          clip: intersect_clip(
            clip,
            ClipRect {
              x: abs_x + padding.left,
              y: abs_y + padding.top,
              width: content_width,
              height: content_height,
              active: true,
              border_radius: None,
            },
          ),
        });
      }
      NodeKind::Checkbox { state } => {
        let checked = state.is_checked();
        let hovered = node.is_style_hovered();
        let style = state.style(checked, hovered);
        let width = style.width.unwrap_or(result.size.width).min(result.size.width).max(0.0);
        let height = style
          .height
          .unwrap_or(result.size.height)
          .min(result.size.height)
          .max(0.0);
        let rect = SliderPartRect {
          x: abs_x + (result.size.width - width) * 0.5,
          y: abs_y + (result.size.height - height) * 0.5,
          width,
          height,
        };
        let color = if checked {
          style.color.unwrap_or(DEFAULT_CHECKBOX_CHECKED_COLOR)
        } else {
          style
            .color
            .or_else(|| node.resolved_color(&self.palette.borrow()))
            .unwrap_or(DEFAULT_CONTROL_SURFACE_COLOR)
        };
        push_checkbox_quads(
          quads,
          rect,
          &style,
          color,
          style
            .border_radius
            .map(|radius| radius.resolve(&self.radii.borrow()))
            .or_else(|| node.get_border_radius(&self.radii.borrow())),
          style
            .border
            .as_ref()
            .and_then(|border| border.resolve_with_sizes(&self.palette.borrow(), &self.border_sizes.borrow()))
            .or_else(|| node.get_resolved_border(&self.palette.borrow(), &self.border_sizes.borrow())),
          checked,
          opacity,
          transform,
          clip,
        );
      }
      NodeKind::Slider { state } => {
        let hovered = node.is_style_hovered() || state.is_hovered() || state.is_dragging();
        let track_style = state.track_style(hovered);
        let fill_style = state.fill_style(hovered);
        let thumb_style = state.thumb_style(hovered);
        let (track_rect, thumb_rect) = state.part_rects(
          abs_x,
          abs_y,
          result.size.width,
          result.size.height,
          hovered,
          DEFAULT_SLIDER_THUMB_MIN_SIZE,
        );
        let track_color = track_style
          .color
          .or_else(|| node.resolved_color(&self.palette.borrow()))
          .unwrap_or(DEFAULT_SLIDER_TRACK_COLOR);
        let track_radius = track_style
          .border_radius
          .map(|radius| radius.resolve(&self.radii.borrow()))
          .or_else(|| node.get_border_radius(&self.radii.borrow()));
        let track_border = track_style
          .border
          .as_ref()
          .and_then(|border| border.resolve_with_sizes(&self.palette.borrow(), &self.border_sizes.borrow()))
          .or_else(|| node.get_resolved_border(&self.palette.borrow(), &self.border_sizes.borrow()));
        push_slider_part_quads(
          quads,
          track_rect,
          &track_style,
          track_color,
          track_radius,
          track_border,
          opacity,
          transform,
          clip,
        );
        if let Some(fill_style) = fill_style {
          let fill_height = fill_style.height.unwrap_or(track_rect.height).max(0.0);
          let fill_center_y = track_rect.y + track_rect.height * 0.5;
          let fill_rect = SliderPartRect {
            x: track_rect.x,
            y: fill_center_y - fill_height * 0.5,
            width: ((thumb_rect.x + thumb_rect.width * 0.5) - track_rect.x)
              .max(0.0)
              .min(track_rect.width),
            height: fill_height,
          };
          let fill_color = fill_style.color.unwrap_or(track_color);
          let fill_radius = fill_style
            .border_radius
            .map(|radius| radius.resolve(&self.radii.borrow()))
            .or(track_radius);
          let fill_border = fill_style
            .border
            .as_ref()
            .and_then(|border| border.resolve_with_sizes(&self.palette.borrow(), &self.border_sizes.borrow()));
          push_slider_part_quads(
            quads,
            fill_rect,
            &fill_style,
            fill_color,
            fill_radius,
            fill_border,
            opacity,
            transform,
            clip,
          );
        }
        push_slider_part_quads(
          quads,
          thumb_rect,
          &thumb_style,
          thumb_style.color.unwrap_or(DEFAULT_SLIDER_THUMB_COLOR),
          Some(
            thumb_style
              .border_radius
              .map(|radius| radius.resolve(&self.radii.borrow()))
              .unwrap_or_else(|| BorderRadius::all(thumb_rect.width.min(thumb_rect.height) * 0.5)),
          ),
          thumb_style
            .border
            .as_ref()
            .and_then(|border| border.resolve_with_sizes(&self.palette.borrow(), &self.border_sizes.borrow())),
          opacity,
          transform,
          clip,
        );
      }
      NodeKind::Select { state } => {
        let style = state.style();
        let hovered = node.style_state.is_hovered();
        let focused = node.style_state.is_focused();
        let open = state.is_open();
        let trigger = style.resolved_trigger(hovered, focused, open);

        let background = {
          let palette = self.palette.borrow();
          trigger.background.as_ref().and_then(|color| color.resolve(&palette))
        };
        let radius = trigger
          .border_radius
          .map(|radius| radius.resolve(&self.radii.borrow()))
          .or_else(|| node.get_border_radius(&self.radii.borrow()));
        let border = trigger
          .border
          .as_ref()
          .and_then(|border| border.resolve_with_sizes(&self.palette.borrow(), &self.border_sizes.borrow()))
          .or_else(|| node.get_resolved_border(&self.palette.borrow(), &self.border_sizes.borrow()));

        let (bg_x, bg_y, bg_transform, bg_origin) = transformed_quad_frame(abs_x, abs_y, transform);
        quads.push(Quad {
          x: bg_x,
          y: bg_y,
          width: result.size.width,
          height: result.size.height,
          opacity,
          transform: bg_transform,
          transform_origin: bg_origin,
          content: QuadContent::Rect {
            color: background.unwrap_or(DEFAULT_CONTROL_SURFACE_COLOR),
            gradient: None,
          },
          border_radius: radius,
          border,
          clip,
        });
      }
      _ => {}
    }

    let (child_clip, child_cull_clip, child_culling_enabled) =
      if let LayoutKind::ScrollModifier { state, culling, .. } = node.layout_kind() {
        let viewport_clip = intersect_clip(
          clip,
          ClipRect {
            x: abs_x,
            y: abs_y,
            width: state.viewport_width(),
            height: state.viewport_height(),
            active: true,
            border_radius: node
              .get_border_radius(&self.radii.borrow())
              .map(|radius| radius.clamped_to_rect(state.viewport_width(), state.viewport_height())),
          },
        );
        let child_clip = inset_clip_for_border(viewport_clip, resolved_border);
        let child_cull_clip = if *culling {
          inset_clip_for_border(intersect_clip(cull_clip, viewport_clip), resolved_border)
        } else {
          ClipRect::default()
        };
        (child_clip, child_cull_clip, *culling)
      } else if node.overflow == Overflow::Hidden && hidden_overflow_creates_clip(has_visual, transform) {
        let overflow_clip = intersect_clip(
          clip,
          ClipRect {
            x: abs_x,
            y: abs_y,
            width: result.size.width,
            height: result.size.height,
            active: true,
            border_radius: node
              .get_border_radius(&self.radii.borrow())
              .map(|radius| radius.clamped_to_rect(result.size.width, result.size.height)),
          },
        );
        let child_clip = inset_clip_for_border(overflow_clip, resolved_border);
        let child_cull_clip = if culling_enabled {
          let cull_overflow_clip = intersect_clip(cull_clip, overflow_clip);
          inset_clip_for_border(cull_overflow_clip, resolved_border)
        } else {
          ClipRect::default()
        };
        (child_clip, child_cull_clip, culling_enabled)
      } else {
        (clip, cull_clip, culling_enabled)
      };

    for (child_layout, child_node) in result.children.iter().zip(node.children().iter()) {
      let child_abs_x = abs_x + child_layout.offset.x;
      let child_abs_y = abs_y + child_layout.offset.y;
      if child_culling_enabled
        && clipped_subtree_is_hidden(
          child_node,
          &child_layout.result,
          child_abs_x,
          child_abs_y,
          transform,
          child_cull_clip,
        )
      {
        continue;
      }

      self.collect_quads(
        child_node,
        &child_layout.result,
        child_abs_x,
        child_abs_y,
        abs_x,
        abs_y,
        transform,
        child_clip,
        child_cull_clip,
        child_culling_enabled,
        quads,
      );
    }

    if defer_border_to_overlay {
      let (border_x, border_y, border_transform, border_transform_origin) =
        transformed_quad_frame(abs_x, abs_y, transform);
      quads.push(Quad {
        x: border_x,
        y: border_y,
        width: result.size.width,
        height: result.size.height,
        opacity,
        transform: border_transform,
        transform_origin: border_transform_origin,
        content: QuadContent::Rect {
          color: DEFAULT_TRANSPARENT_COLOR,
          gradient: None,
        },
        border_radius: node.get_border_radius(&self.radii.borrow()),
        border: resolved_border,
        clip,
      });
    }

    if let LayoutKind::ScrollModifier { state, direction, .. } = node.layout_kind() {
      state.set_viewport_position(abs_x, abs_y);
      let sb_style = node.scrollbar_style(self.scrollbar.borrow().clone());
      state.set_style(sb_style.clone());
      let thumb_color = sb_style.thumb_color;

      match direction {
        ScrollDirection::Vertical | ScrollDirection::Both => {
          if let Some(geo) = state.scrollbar_geometry_for_axis(ScrollAxis::Vertical, &sb_style) {
            if sb_style.track_color.a() > 0 {
              quads.push(Quad {
                x: geo.track_x,
                y: geo.track_y,
                width: geo.track_width,
                height: geo.track_height,
                opacity: DEFAULT_QUAD_OPACITY,
                transform: Transform2D::IDENTITY,
                transform_origin: None,
                content: QuadContent::Rect {
                  color: sb_style.track_color,
                  gradient: None,
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
              opacity: DEFAULT_QUAD_OPACITY,
              transform: Transform2D::IDENTITY,
              transform_origin: None,
              content: QuadContent::Rect {
                color: thumb_color,
                gradient: None,
              },
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
          if let Some(geo) = state.scrollbar_geometry_for_axis(ScrollAxis::Horizontal, &sb_style) {
            if sb_style.track_color.a() > 0 {
              quads.push(Quad {
                x: geo.track_x,
                y: geo.track_y,
                width: geo.track_width,
                height: geo.track_height,
                opacity: DEFAULT_QUAD_OPACITY,
                transform: Transform2D::IDENTITY,
                transform_origin: None,
                content: QuadContent::Rect {
                  color: sb_style.track_color,
                  gradient: None,
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
              opacity: DEFAULT_QUAD_OPACITY,
              transform: Transform2D::IDENTITY,
              transform_origin: None,
              content: QuadContent::Rect {
                color: thumb_color,
                gradient: None,
              },
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
    if let Some(cached) = self.layout_node_from_cache(glyph_engine, node, constraints) {
      return cached;
    }

    self.layout_node_uncached_with_child_overrides(glyph_engine, node, constraints, None)
  }

  fn layout_node_from_cache(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
  ) -> Option<LayoutResult> {
    if !node.layout_cache.is_dirty() {
      return node.layout_cache.get(constraints).and_then(|cached| {
        if !Self::cached_result_matches_node_tree(node, &cached) {
          return None;
        }
        Some(self.prepare_cached_result(node, cached))
      });
    }

    if node.layout_cache.is_local_dirty() || !node.layout_cache.is_descendant_dirty() {
      return None;
    }

    // A descendant changed, but this node may still be able to keep its own
    // geometry. Patch dirty child results into the cached tree and only force
    // this parent to relayout if the child no longer fits the cached parent or
    // the parent layout kind needs to reposition siblings around the new size.
    let mut cached = node.layout_cache.get_dirty(constraints)?;
    if !Self::cached_result_matches_node_tree(node, &cached) {
      return None;
    }

    let mut child_overrides = vec![None; cached.children.len()];
    for (index, child) in node.children().iter().enumerate() {
      if !child.layout_cache.is_dirty() {
        continue;
      }

      let child_constraints = child.layout_cache.constraints()?;
      let repaired = self.layout_node(glyph_engine, child, child_constraints);
      let original_offset = cached.children[index].offset;
      child_overrides[index] = Some(ChildLayoutOverride {
        constraints: child_constraints,
        result: repaired.clone(),
      });
      if let Some(rect) = child.element_override_rect()
        && (rect.relative_x != original_offset.x || rect.relative_y != original_offset.y)
      {
        return Some(self.layout_node_uncached_with_child_overrides(
          glyph_engine,
          node,
          constraints,
          Some(&child_overrides),
        ));
      }
      // A cleared bounds override leaves the override baked into the cached
      // offset — only a full relayout recovers the child's natural position.
      if child.take_element_override_cleared() {
        return Some(self.layout_node_uncached_with_child_overrides(
          glyph_engine,
          node,
          constraints,
          Some(&child_overrides),
        ));
      }
      let size_changed = repaired.size != cached.children[index].result.size;
      if size_changed
        && (!Self::layout_kind_can_patch_child_size_change(node.layout_kind())
          || !Self::child_fits_cached_parent(original_offset, repaired.size, cached.size))
      {
        return Some(self.layout_node_uncached_with_child_overrides(
          glyph_engine,
          node,
          constraints,
          Some(&child_overrides),
        ));
      }
      cached.children[index].result = repaired.into();
    }

    let prepared = self.prepare_cached_result(node, cached);
    node.layout_cache.store(constraints, prepared.clone());
    Some(prepared)
  }

  fn cached_result_matches_node_tree(node: &Node, result: &LayoutResult) -> bool {
    // A cached subtree whose geometry contradicts any node's fixed frame is
    // stale no matter how clean its flags are — observed in production as a
    // virtualized list's rows column serving a previous window's result
    // (old spacer heights inside), shifting every row off the viewport. The
    // per-node frame/cache check in `mark_layout_dirty` cannot catch this:
    // the stale geometry lives in the PARENT's cached tree, not in the
    // mismatched child's own cache.
    if let Some(Dimension::Px(px)) = node.frame.width
      && (result.size.width - px).abs() > 0.5
    {
      return false;
    }
    if let Some(Dimension::Px(px)) = node.frame.height
      && (result.size.height - px).abs() > 0.5
    {
      return false;
    }
    result.children.len() == node.children().len()
      && node
        .children()
        .iter()
        .zip(&result.children)
        .all(|(child, child_layout)| {
          // A cached offset that contradicts a live bounds override is stale
          // the same way: consumed dirty flags don't protect a parent cache
          // that predates the override (observed with the overlay-host double
          // compute — the second pass served the root's pre-drag snapshot,
          // pinning dragged elements to their declared position).
          if let Some(rect) = child.element_override_rect()
            && ((child_layout.offset.x - rect.relative_x).abs() > 0.01
              || (child_layout.offset.y - rect.relative_y).abs() > 0.01)
          {
            return false;
          }
          // Likewise a cached offset that contradicts the child's DECLARED
          // absolute position: a position-only change (e.g. an editor undo
          // moving a widget back) doesn't set any dirty flag, so a preserved
          // node cache keeps the parent serving the old offset until an
          // unrelated invalidation. Only absolutely-positioned children have
          // a self-declared offset to check against.
          if let Position::Absolute { x, y, .. } = child.position() {
            let expected = Self::apply_relative_position(child, Offset::new(x, y));
            if (child_layout.offset.x - expected.x).abs() > 0.01 || (child_layout.offset.y - expected.y).abs() > 0.01 {
              return false;
            }
          }
          Self::cached_result_matches_node_tree(child, &child_layout.result)
        })
  }

  fn child_fits_cached_parent(offset: Offset, child_size: Size, parent_size: Size) -> bool {
    const EPSILON: f32 = 0.5;
    offset.x >= -EPSILON
      && offset.y >= -EPSILON
      && offset.x + child_size.width <= parent_size.width + EPSILON
      && offset.y + child_size.height <= parent_size.height + EPSILON
  }

  fn layout_kind_can_patch_child_size_change(layout_kind: &LayoutKind) -> bool {
    matches!(
      layout_kind,
      LayoutKind::Stack { .. } | LayoutKind::LogicalModifier | LayoutKind::ScrollModifier { .. }
    )
  }

  fn prepare_cached_result(&self, node: &Node, mut cached: LayoutResult) -> LayoutResult {
    self.prepare_layout_result_tree(node, &mut cached);
    cached
  }

  fn prepare_layout_result_tree(&self, node: &Node, result: &mut LayoutResult) {
    for (child_node, child_layout) in node.children().iter().zip(result.children.iter_mut()) {
      self.prepare_layout_result_tree(child_node, Arc::make_mut(&mut child_layout.result));
    }

    if let LayoutKind::ScrollModifier { state, direction, .. } = node.layout_kind() {
      let content_width = result.children.first().map(|c| c.result.size.width).unwrap_or(0.0);
      let content_height = result.children.first().map(|c| c.result.size.height).unwrap_or(0.0);
      let style = node.scrollbar_style(self.scrollbar.borrow().clone());
      let mut reserve_vertical = false;
      let mut reserve_horizontal = false;

      for _ in 0..3 {
        let viewport = reserved_viewport(result.size, &style, reserve_vertical, reserve_horizontal);
        let next_reserve_vertical = should_reserve_scrollbar(
          &style,
          *direction,
          ScrollAxis::Vertical,
          content_height,
          viewport.height,
        );
        let next_reserve_horizontal = should_reserve_scrollbar(
          &style,
          *direction,
          ScrollAxis::Horizontal,
          content_width,
          viewport.width,
        );

        if next_reserve_vertical == reserve_vertical && next_reserve_horizontal == reserve_horizontal {
          break;
        }

        reserve_vertical = next_reserve_vertical;
        reserve_horizontal = next_reserve_horizontal;
      }

      let viewport = reserved_viewport(result.size, &style, reserve_vertical, reserve_horizontal);
      state.update_layout_with_container(
        content_width,
        content_height,
        viewport.width,
        viewport.height,
        result.size.width,
        result.size.height,
      );
      if let Some(child) = result.children.first_mut() {
        child.offset.x = -state.scroll_x();
        child.offset.y = -state.scroll_y();
      }
    }
  }

  fn layout_node_uncached_with_child_overrides(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
  ) -> LayoutResult {
    #[cfg(feature = "perf_profile")]
    let profile_start = Instant::now();
    #[cfg(feature = "perf_profile")]
    let profile_local_dirty = node.layout_cache.is_local_dirty();
    #[cfg(feature = "perf_profile")]
    let profile_descendant_dirty = node.layout_cache.is_descendant_dirty();
    #[cfg(feature = "perf_profile")]
    let profile_override_count = child_overrides
      .map(|overrides| {
        overrides
          .iter()
          .filter(|override_result| override_result.is_some())
          .count()
      })
      .unwrap_or(0);

    self.last_recalculated.set(true);
    let mut result = self.layout_node_box(glyph_engine, node, constraints, child_overrides);
    Self::apply_runtime_rect(node, &mut result);
    self.prepare_layout_result_tree(node, &mut result);
    node.layout_cache.store(constraints, result.clone());
    #[cfg(feature = "perf_profile")]
    {
      let elapsed = profile_start.elapsed();
      if elapsed >= SLOW_LAYOUT_NODE_THRESHOLD {
        tracing::trace!(
          target: "layout-profile",
          "[layout-profile] kind={} tag={} children={} overrides={} local_dirty={} descendant_dirty={} constraints={:.1}..{:.1}x{:.1}..{:.1} size={:.1}x{:.1} ms={:.3}",
          layout_kind_profile_name(node.layout_kind()),
          node.tag_name(),
          node.children().len(),
          profile_override_count,
          profile_local_dirty,
          profile_descendant_dirty,
          constraints.min_width,
          constraints.max_width,
          constraints.min_height,
          constraints.max_height,
          result.size.width,
          result.size.height,
          elapsed.as_secs_f64() * 1000.0,
        );
      }
    }
    result
  }

  fn layout_node_box(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
  ) -> LayoutResult {
    let frame = node.effective_frame(FrameConstraints::default());
    let padding = node.effective_padding(&Padding::default());
    let frame_is_flat = frame != FrameConstraints::default();
    let padding_is_flat = padding != Padding::default();

    if frame_is_flat || padding_is_flat {
      return self.layout_flat_box(glyph_engine, node, constraints, &frame, &padding, child_overrides);
    }

    self.layout_node_content(glyph_engine, node, constraints, child_overrides)
  }

  fn layout_node_content(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
  ) -> LayoutResult {
    match node.layout_kind() {
      LayoutKind::Leaf => self.layout_leaf(glyph_engine, node, constraints),
      LayoutKind::Row {
        spacing,
        align,
        justify,
        wrap,
      } => {
        let spacing = spacing.resolve(&self.spacing.borrow(), constraints.max_width);
        self.layout_flex(
          glyph_engine,
          node,
          constraints,
          spacing,
          *align,
          *justify,
          *wrap,
          false,
          child_overrides,
        )
      }
      LayoutKind::Column {
        spacing,
        align,
        justify,
        wrap,
      } => {
        let spacing = spacing.resolve(&self.spacing.borrow(), constraints.max_height);
        self.layout_flex(
          glyph_engine,
          node,
          constraints,
          spacing,
          *align,
          *justify,
          *wrap,
          true,
          child_overrides,
        )
      }
      LayoutKind::Stack { align } => self.layout_stack(glyph_engine, node, constraints, *align, child_overrides),
      LayoutKind::LogicalModifier => self.layout_passthrough(glyph_engine, node, constraints, child_overrides),
      LayoutKind::ScrollModifier { state, direction, .. } => {
        self.layout_scroll(glyph_engine, node, constraints, state, *direction, child_overrides)
      }
    }
  }

  fn apply_runtime_rect(node: &Node, result: &mut LayoutResult) {
    if let Some(rect) = node.element_override_rect() {
      result.size.width = rect.width;
      result.size.height = rect.height;
    }

    for (child_layout, child_node) in result.children.iter_mut().zip(node.children()) {
      if let Some(rect) = child_node.element_override_rect() {
        child_layout.offset.x = rect.relative_x;
        child_layout.offset.y = rect.relative_y;
      }
    }
  }

  fn layout_leaf(&self, glyph_engine: &mut GlyphEngine, node: &Node, constraints: Constraints) -> LayoutResult {
    match node.node_kind() {
      NodeKind::Text { state, style, .. } => {
        let source = node.text_content().unwrap_or_default();
        let style = style.resolve(&self.typography.borrow(), &self.palette.borrow());
        let (content, force_display_text) = (source, false);
        return self.layout_text_node(
          glyph_engine,
          content,
          state,
          &style,
          constraints,
          node.text_wrap,
          node.text_overflow,
          force_display_text,
        );
      }
      #[cfg(feature = "markdown")]
      NodeKind::RichText { state, spans, .. } => {
        return self.layout_rich_text_node(
          glyph_engine,
          spans,
          state,
          constraints,
          node.text_wrap,
          node.text_overflow,
        );
      }
      NodeKind::TextInput {
        state,
        style,
        placeholder_style,
      } => {
        let content = state.rendered_text_for_layout();
        return self.layout_text_input(
          glyph_engine,
          state,
          &content,
          style,
          placeholder_style.as_ref(),
          constraints,
        );
      }
      NodeKind::Checkbox { .. } => {
        let preferred = if let NodeKind::Checkbox { state } = node.node_kind() {
          let (width, height) = state.preferred_size(DEFAULT_CHECKBOX_WIDTH, DEFAULT_CHECKBOX_HEIGHT);
          node.intrinsic_size.unwrap_or(Size::new(width, height))
        } else {
          node
            .intrinsic_size
            .unwrap_or(Size::new(DEFAULT_CHECKBOX_WIDTH, DEFAULT_CHECKBOX_HEIGHT))
        };
        return LayoutResult {
          size: constraints.constrain(preferred),
          children: vec![],
        };
      }
      NodeKind::Slider { .. } => {
        let preferred = if let NodeKind::Slider { state } = node.node_kind() {
          let (width, height) = state.preferred_size(
            DEFAULT_SLIDER_WIDTH,
            DEFAULT_SLIDER_HEIGHT,
            DEFAULT_SLIDER_THUMB_MIN_SIZE,
          );
          node.intrinsic_size.unwrap_or(Size::new(width, height))
        } else {
          node
            .intrinsic_size
            .unwrap_or(Size::new(DEFAULT_SLIDER_WIDTH, DEFAULT_SLIDER_HEIGHT))
        };
        return LayoutResult {
          size: constraints.constrain(preferred),
          children: vec![],
        };
      }
      NodeKind::Select { state } => {
        let trigger = state.style().resolved_trigger(false, false, false);
        let width = node
          .intrinsic_size
          .map(|size| size.width)
          .or(trigger.min_width)
          .unwrap_or(DEFAULT_SELECT_WIDTH);
        let height = node
          .intrinsic_size
          .map(|size| size.height)
          .or(trigger.min_height)
          .unwrap_or(DEFAULT_SELECT_HEIGHT);
        return LayoutResult {
          size: constraints.constrain(Size::new(width, height)),
          children: vec![],
        };
      }
      #[cfg(feature = "image")]
      NodeKind::Image { data } => {
        let preferred = node
          .intrinsic_size
          .unwrap_or(Size::new(data.width() as f32, data.height() as f32));
        return LayoutResult {
          size: constraints.constrain(preferred),
          children: vec![],
        };
      }
      #[cfg(feature = "image")]
      NodeKind::Video { data, .. } => {
        let preferred = node
          .intrinsic_size
          .unwrap_or(Size::new(data.width() as f32, data.height() as f32));
        return LayoutResult {
          size: constraints.constrain(preferred),
          children: vec![],
        };
      }
      #[cfg(feature = "image")]
      NodeKind::ResourceImage { .. } => {
        let preferred = node
          .intrinsic_size
          .unwrap_or(Size::new(DEFAULT_RESOURCE_WIDTH, DEFAULT_RESOURCE_HEIGHT));
        return LayoutResult {
          size: constraints.constrain(preferred),
          children: vec![],
        };
      }
      #[cfg(feature = "svg")]
      NodeKind::Svg { data } => {
        let preferred = node
          .intrinsic_size
          .unwrap_or(Size::new(data.viewbox_width(), data.viewbox_height()));
        return LayoutResult {
          size: constraints.constrain(preferred),
          children: vec![],
        };
      }
      #[cfg(all(feature = "svg", feature = "resources"))]
      NodeKind::ResourceSvg { .. } => {
        let preferred = node
          .intrinsic_size
          .unwrap_or(Size::new(DEFAULT_RESOURCE_WIDTH, DEFAULT_RESOURCE_HEIGHT));
        return LayoutResult {
          size: constraints.constrain(preferred),
          children: vec![],
        };
      }
      NodeKind::Empty => {}
    }

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
    wrap: bool,
  ) -> LayoutResult {
    let max_width = if wrap && bounded_text_width(constraints.max_width) {
      constraints.max_width
    } else {
      f32::MAX
    };
    let measured = glyph_engine.measure_text(text, style, max_width);
    let size = if wrap {
      constraints.constrain(measured)
    } else {
      Size::new(
        measured.width.max(constraints.min_width),
        measured.height.max(constraints.min_height),
      )
    };
    LayoutResult { size, children: vec![] }
  }

  fn layout_text_node(
    &self,
    glyph_engine: &mut GlyphEngine,
    text: &str,
    state: &crate::node::node_kind::TextState,
    style: &TextStyle,
    constraints: Constraints,
    wrap: bool,
    overflow: TextOverflow,
    force_display_text: bool,
  ) -> LayoutResult {
    let effective_wrap = wrap && overflow == TextOverflow::Clip;
    let overflow_display_text = match overflow {
      TextOverflow::Clip => None,
      TextOverflow::Elipsis => self.ellipsize_text(glyph_engine, text, style, constraints.max_width),
    };
    let layout_text = overflow_display_text.as_deref().unwrap_or(text);
    if force_display_text || overflow_display_text.is_some() {
      state.set_display_text(Some(layout_text.to_owned()));
    } else {
      state.set_display_text(None);
    }
    let render_wrap = effective_wrap && bounded_text_width(constraints.max_width);
    state.set_render_wrap(render_wrap);
    let max_width = if render_wrap { constraints.max_width } else { f32::MAX };
    if state.selectable() {
      state.set_caret_positions(glyph_engine.caret_positions(layout_text, style, max_width, effective_wrap));
    }
    if !state.selectable() && overflow_display_text.is_none() && constraints_are_tight(constraints) {
      return LayoutResult {
        size: Size::new(constraints.max_width, constraints.max_height),
        children: vec![],
      };
    }
    self.layout_text(glyph_engine, layout_text, style, constraints, effective_wrap)
  }

  #[cfg(feature = "markdown")]
  fn layout_rich_text_node(
    &self,
    glyph_engine: &mut GlyphEngine,
    spans: &[crate::layout::quad::RichTextSpan],
    state: &crate::node::node_kind::TextState,
    constraints: Constraints,
    wrap: bool,
    overflow: TextOverflow,
  ) -> LayoutResult {
    let effective_wrap = wrap && overflow == TextOverflow::Clip;
    let render_wrap = effective_wrap && bounded_text_width(constraints.max_width);
    state.set_render_wrap(render_wrap);
    let max_width = if render_wrap { constraints.max_width } else { f32::MAX };
    if state.selectable() {
      let display_text = spans.iter().map(|span| span.text.as_str()).collect::<String>();
      state.set_display_text(Some(display_text.clone()));
      if let Some(first) = spans.first() {
        state.set_caret_positions(glyph_engine.caret_positions(&display_text, &first.style, max_width, effective_wrap));
      }
    } else {
      state.set_display_text(None);
    }
    let measured = glyph_engine.measure_rich_text(spans, max_width);
    let size = if effective_wrap {
      constraints.constrain(measured)
    } else {
      Size::new(
        measured.width.max(constraints.min_width),
        measured.height.max(constraints.min_height),
      )
    };
    LayoutResult { size, children: vec![] }
  }

  fn text_width(&self, glyph_engine: &mut GlyphEngine, text: &str, style: &TextStyle) -> f32 {
    glyph_engine.measure_text(text, style, f32::MAX).width
  }

  fn ellipsize_text(
    &self,
    glyph_engine: &mut GlyphEngine,
    text: &str,
    style: &TextStyle,
    max_width: f32,
  ) -> Option<String> {
    if text.is_empty() || !max_width.is_finite() {
      return None;
    }
    if max_width <= 0.0 {
      return Some(String::new());
    }
    if self.text_width(glyph_engine, text, style) <= max_width {
      return None;
    }

    const ELLIPSIS: &str = "…";
    let ellipsis_width = self.text_width(glyph_engine, ELLIPSIS, style);
    if ellipsis_width > max_width {
      return Some(String::new());
    }

    let boundaries: Vec<usize> = text
      .char_indices()
      .map(|(index, _)| index)
      .chain(std::iter::once(text.len()))
      .collect();
    let mut low = 0usize;
    let mut high = boundaries.len() - 1;
    while low < high {
      let mid = (low + high).div_ceil(2);
      let candidate = format!("{}{}", &text[..boundaries[mid]], ELLIPSIS);
      if self.text_width(glyph_engine, &candidate, style) <= max_width {
        low = mid;
      } else {
        high = mid - 1;
      }
    }

    Some(format!("{}{}", &text[..boundaries[low]], ELLIPSIS))
  }

  fn layout_text_input(
    &self,
    glyph_engine: &mut GlyphEngine,
    state: &crate::node::node_kind::TextInputState,
    text: &str,
    style: &TextStyle,
    placeholder_style: Option<&TextStyle>,
    constraints: Constraints,
  ) -> LayoutResult {
    let display_style = text_input_display_style(state, style, placeholder_style);
    let overflow = state.overflow();
    let text_constraints = Constraints {
      min_width: 0.0,
      min_height: 0.0,
      ..constraints
    };
    let text_result = self.layout_text(
      glyph_engine,
      text,
      display_style,
      text_constraints,
      overflow == crate::node::node_kind::TextInputOverflow::Multiline,
    );
    let line_height = (style.font_size * style.line_height).max(1.0);
    let text_height = text_result
      .size
      .height
      .max(explicit_multiline_height(text, line_height));
    let preferred = match overflow {
      crate::node::node_kind::TextInputOverflow::Multiline => {
        let preferred_height = multiline_preferred_height(state, text_height, line_height);
        Size::new(text_result.size.width.max(DEFAULT_TEXT_INPUT_WIDTH), preferred_height)
      }
      crate::node::node_kind::TextInputOverflow::Scroll => Size::new(DEFAULT_TEXT_INPUT_WIDTH, line_height),
    };
    let size = constraints.constrain(preferred);
    // Caret positions must wrap exactly like the rendered text, otherwise
    // soft-wrapped rows collapse onto the first row's y and pointer hit-testing
    // can never reach them. Single-line (Scroll) inputs never wrap, but aligned
    // text still needs the finite content width so caret/selection geometry
    // follows centered or right-aligned glyphs.
    let wraps = overflow == crate::node::node_kind::TextInputOverflow::Multiline;
    let caret_width = if wraps || display_style.text_align != crate::layout::text_style::TextAlign::Left {
      size.width
    } else {
      f32::MAX
    };
    let caret_source = state.caret_source_text();
    let mut caret_positions = glyph_engine.caret_positions(&caret_source, style, caret_width, wraps);
    state.remap_caret_positions(&mut caret_positions);
    state.set_caret_positions(caret_positions);

    state.set_caret_height(line_height);
    state.sync_caret_metrics_to_position(line_height);
    let caret_x = state.caret_x() + state.scroll_x();
    let caret_y = state.caret_y() + state.scroll_y();
    match overflow {
      crate::node::node_kind::TextInputOverflow::Scroll => {
        let caret_width = 1.0;
        let max_scroll = (text_result.size.width + caret_width - size.width).max(0.0);
        let scroll_x = if state.is_focused() {
          let mut scroll_x = state.scroll_x().min(max_scroll);
          if caret_x < scroll_x {
            scroll_x = caret_x;
          } else if caret_x + caret_width > scroll_x + size.width {
            scroll_x = (caret_x + caret_width - size.width).min(max_scroll);
          }
          scroll_x
        } else {
          match state.unfocused_overflow_anchor() {
            crate::node::node_kind::TextInputOverflowAnchor::Start => 0.0,
            crate::node::node_kind::TextInputOverflowAnchor::End => max_scroll,
          }
        };
        state.set_scroll_x(scroll_x);
        state.set_scroll_y(0.0);
      }
      crate::node::node_kind::TextInputOverflow::Multiline => {
        state.set_scroll_x(0.0);
        let max_scroll = (text_height - size.height).max(0.0);
        let mut scroll_y = state.scroll_y().min(max_scroll);
        if caret_y < scroll_y {
          scroll_y = caret_y;
        } else if caret_y + line_height > scroll_y + size.height {
          scroll_y = (caret_y + line_height - size.height).min(max_scroll);
        }
        state.set_scroll_y(scroll_y);
      }
    }
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
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
  ) -> LayoutResult {
    let children = node.children();
    if children.is_empty() {
      return LayoutResult {
        size: constraints.constrain(Size::default()),
        children: vec![],
      };
    }

    if wrap == FlexWrap::Wrap {
      return self.layout_flex_wrap(
        glyph_engine,
        node,
        constraints,
        spacing,
        align,
        justify,
        vertical,
        child_overrides,
      );
    }

    let layout_child_count = children.iter().filter(|child| !child.is_overlay_declaration()).count();
    let total_spacing = spacing * (layout_child_count as f32 - 1.0).max(0.0);
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
      if child.is_overlay_declaration() {
        flex_params_list.push(FlexParams {
          grow: 0.0,
          shrink: 0.0,
          basis: None,
        });
        non_flex_results.push(Some(LayoutResult {
          size: Size::default(),
          children: Vec::new(),
        }));
        continue;
      }

      let flex_params = child.state_flex();

      if let Some(params) = flex_params {
        grow_total += params.grow;
        shrink_total += params.shrink;
        flex_params_list.push(params);
        if params.grow == 0.0 && params.basis.is_none() {
          let child_constraints = Self::non_flex_child_constraints(child, constraints, vertical);
          non_flex_results.push(Some(self.layout_child_node(
            glyph_engine,
            child_overrides,
            non_flex_results.len(),
            child,
            child_constraints,
          )));
        } else {
          non_flex_results.push(None);
        }
      } else {
        flex_params_list.push(FlexParams {
          grow: 0.0,
          shrink: 0.0,
          basis: None,
        });
        let child_constraints = Self::non_flex_child_constraints(child, constraints, vertical);
        non_flex_results.push(Some(self.layout_child_node(
          glyph_engine,
          child_overrides,
          non_flex_results.len(),
          child,
          child_constraints,
        )));
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
            min_width: 0.0,
            max_width: constraints.max_width,
            min_height: flex_size,
            max_height: flex_size,
          }
        } else {
          Constraints {
            min_width: flex_size,
            max_width: flex_size,
            min_height: 0.0,
            max_height: constraints.max_height,
          }
        };
        results.push(self.layout_child_node(glyph_engine, child_overrides, i, child, child_constraints));
      }
    }

    if shrink_total > 0.0 {
      let total_children_main: f32 = results
        .iter()
        .zip(children.iter())
        .filter(|(_, child)| !child.is_overlay_declaration())
        .map(|(r, _)| if vertical { r.size.height } else { r.size.width })
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
      .zip(children.iter())
      .filter(|(_, child)| !child.is_overlay_declaration())
      .map(|(r, _)| if vertical { r.size.width } else { r.size.height })
      .fold(0.0_f32, f32::max);

    let total_main: f32 = results
      .iter()
      .zip(children.iter())
      .filter(|(_, child)| !child.is_overlay_declaration())
      .map(|(r, _)| if vertical { r.size.height } else { r.size.width })
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
        if child.is_overlay_declaration() {
          continue;
        }
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
          results[i] = self.layout_child_node(glyph_engine, child_overrides, i, child, stretch_constraints);
        }
      }
    }

    let child_layouts = self.position_flex_line(children, &results, &size, spacing, align, justify, vertical);

    LayoutResult {
      size,
      children: child_layouts.into(),
    }
  }

  fn non_flex_child_constraints(child: &Node, constraints: Constraints, vertical: bool) -> Constraints {
    let bounds_percent_main = Self::has_percent_main_frame(child, vertical);

    if vertical {
      Constraints {
        min_width: 0.0,
        max_width: constraints.max_width,
        min_height: 0.0,
        max_height: if bounds_percent_main {
          constraints.max_height
        } else {
          f32::INFINITY
        },
      }
    } else {
      Constraints {
        min_width: 0.0,
        max_width: if bounds_percent_main {
          constraints.max_width
        } else {
          f32::INFINITY
        },
        min_height: 0.0,
        max_height: constraints.max_height,
      }
    }
  }

  fn has_percent_main_frame(child: &Node, vertical: bool) -> bool {
    let has_own_percent_frame = child.state_frame().is_some_and(|frame| {
      let dimensions = if vertical {
        [frame.height, frame.min_height, frame.max_height]
      } else {
        [frame.width, frame.min_width, frame.max_width]
      };

      dimensions
        .into_iter()
        .flatten()
        .any(|dimension| matches!(dimension, Dimension::Pct(_)))
    });

    has_own_percent_frame
      || (matches!(child.layout_kind(), LayoutKind::LogicalModifier)
        && child
          .children()
          .first()
          .is_some_and(|child| Self::has_percent_main_frame(child, vertical)))
  }

  fn position_flex_line(
    &self,
    children: &[Node],
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
      .zip(children.iter())
      .filter(|(_, child)| !child.is_overlay_declaration())
      .map(|(r, _)| if vertical { r.size.height } else { r.size.width })
      .sum();
    let free_space = (container_main - children_main).max(0.0);
    let layout_child_count = children.iter().filter(|child| !child.is_overlay_declaration()).count();
    let n = layout_child_count as f32;

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
        let g = if n > 0.0 { free_space / n } else { 0.0 };
        (g / 2.0, g)
      }
      Justify::SpaceEvenly => {
        let g = if n > 0.0 { free_space / (n + 1.0) } else { 0.0 };
        (g, g)
      }
    };

    let mut child_layouts = Vec::with_capacity(results.len());
    let mut main_cursor = leading;
    let mut positioned_layout_children = 0usize;

    for (i, result) in results.iter().enumerate() {
      if children[i].is_overlay_declaration() {
        child_layouts.push(ChildLayout {
          offset: Offset::default(),
          result: result.clone().into(),
        });
        continue;
      }

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
      let offset = Self::apply_relative_position(&children[i], offset);

      positioned_layout_children += 1;
      main_cursor += child_main
        + if positioned_layout_children < layout_child_count {
          gap
        } else {
          0.0
        };
      child_layouts.push(ChildLayout {
        offset,
        result: result.clone().into(),
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
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
  ) -> LayoutResult {
    let children = node.children();
    let max_main = if vertical {
      constraints.max_height
    } else {
      constraints.max_width
    };

    let mut child_results: Vec<Option<LayoutResult>> = children
      .iter()
      .enumerate()
      .map(|(index, child)| {
        // Children measure at their intrinsic size: the container's min
        // cross-constraint must not leak into them (same as the non-wrap
        // path's `non_flex_child_constraints`), or a tight-height container
        // stretches every wrapped item to its own min height.
        let c = if vertical {
          Constraints {
            min_width: 0.0,
            max_width: constraints.max_width,
            min_height: 0.0,
            max_height: f32::INFINITY,
          }
        } else {
          Constraints {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: constraints.max_height,
          }
        };
        Some(self.layout_child_node(glyph_engine, child_overrides, index, child, c))
      })
      .collect();

    let mut lines: Vec<Vec<usize>> = vec![vec![]];
    let mut line_main = 0.0_f32;

    for (i, r) in child_results.iter().enumerate() {
      let r = r.as_ref().unwrap();
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

    let mut all_layouts = Vec::with_capacity(children.len());
    all_layouts.resize_with(children.len(), || ChildLayout {
      offset: Offset::default(),
      result: LayoutResult {
        size: Size::default(),
        children: vec![],
      }
      .into(),
    });
    let mut cross_cursor = 0.0_f32;
    let mut max_main_used = 0.0_f32;

    for line_indices in &lines {
      let line_cross: f32 = line_indices
        .iter()
        .map(|&i| {
          let r = child_results[i].as_ref().unwrap();
          if vertical { r.size.width } else { r.size.height }
        })
        .fold(0.0_f32, f32::max);

      let children_main: f32 = line_indices
        .iter()
        .map(|&i| {
          let r = child_results[i].as_ref().unwrap();
          if vertical { r.size.height } else { r.size.width }
        })
        .sum::<f32>();
      let line_main_total = children_main + spacing * (line_indices.len() as f32 - 1.0).max(0.0);
      max_main_used = max_main_used.max(line_main_total);

      let container_main = if vertical {
        max_main.min(constraints.max_height)
      } else {
        max_main.min(constraints.max_width)
      };
      let free_space = (container_main - children_main).max(0.0);
      let n = line_indices.len() as f32;
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

      let mut main_cursor = leading;
      for (j, &idx) in line_indices.iter().enumerate() {
        let result = child_results[idx].take().unwrap();
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
        let cross_offset = align.cross_offset(line_cross, child_cross) + cross_cursor;
        let offset = if vertical {
          Offset::new(cross_offset, main_cursor)
        } else {
          Offset::new(main_cursor, cross_offset)
        };
        let offset = Self::apply_relative_position(&children[idx], offset);
        main_cursor += child_main + if j < (n as usize - 1) { gap } else { 0.0 };
        all_layouts[idx] = ChildLayout {
          offset,
          result: result.into(),
        };
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
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
  ) -> LayoutResult {
    let children = node.children();
    let child_constraints = constraints.loosen_width().loosen_height();
    let results: Vec<LayoutResult> = children
      .iter()
      .enumerate()
      .map(|(index, child)| match child.position() {
        Position::Absolute { width, height, .. } => {
          let resolved_width = width.and_then(|size| Self::resolve_dimension(size, constraints.max_width));
          let resolved_height = height.and_then(|size| Self::resolve_dimension(size, constraints.max_height));
          let positioned_constraints = Constraints {
            min_width: resolved_width.unwrap_or(0.0),
            max_width: resolved_width.unwrap_or(child_constraints.max_width),
            min_height: resolved_height.unwrap_or(0.0),
            max_height: resolved_height.unwrap_or(child_constraints.max_height),
          };
          let mut result = self.layout_child_node(glyph_engine, child_overrides, index, child, positioned_constraints);
          if let Some(width) = resolved_width {
            result.size.width = width;
          }
          if let Some(height) = resolved_height {
            result.size.height = height;
          }
          result
        }
        _ => self.layout_child_node(glyph_engine, child_overrides, index, child, child_constraints),
      })
      .collect();

    let normal_results: Vec<&LayoutResult> = children
      .iter()
      .zip(results.iter())
      .filter(|(child, _)| !matches!(child.position(), Position::Absolute { .. }))
      .map(|(_, result)| result)
      .collect();

    let max_width = normal_results.iter().map(|r| r.size.width).fold(0.0_f32, f32::max);
    let max_height = normal_results.iter().map(|r| r.size.height).fold(0.0_f32, f32::max);
    let size = constraints.constrain(Size::new(max_width, max_height));

    let child_layouts: Vec<ChildLayout> = results
      .into_iter()
      .zip(children.iter())
      .map(|(result, child)| {
        let offset = match child.position() {
          Position::Absolute { x, y, .. } => Self::apply_relative_position(child, Offset::new(x, y)),
          _ => {
            let child_align = child
              .align_self()
              .map(|align| align.to_stack_alignment())
              .unwrap_or(align);
            Self::apply_relative_position(child, child_align.resolve_offset(size, result.size))
          }
        };
        ChildLayout {
          offset,
          result: result.into(),
        }
      })
      .collect();

    LayoutResult {
      size,
      children: child_layouts.into(),
    }
  }

  fn apply_relative_position(node: &Node, offset: Offset) -> Offset {
    node
      .offset_position()
      .map(|relative| Offset::new(offset.x + relative.x, offset.y + relative.y))
      .unwrap_or(offset)
  }

  pub(crate) fn resolved_padding_for_size(&self, node: &Node, size: Size) -> ResolvedPadding {
    let padding = node.effective_padding(&Padding::default());
    if padding == Padding::default() {
      return ResolvedPadding::default();
    }

    let spacing = self.spacing.borrow();
    ResolvedPadding {
      left: padding.get_left().resolve(&spacing, size.width),
      top: padding.get_top().resolve(&spacing, size.height),
      right: padding.get_right().resolve(&spacing, size.width),
      bottom: padding.get_bottom().resolve(&spacing, size.height),
    }
  }

  fn layout_flat_box(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    frame: &FrameConstraints,
    padding: &Padding,
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
  ) -> LayoutResult {
    let outer_constraints = self.frame_constraints_for_node(node, constraints, frame);
    let parent_w = outer_constraints.max_width;
    let parent_h = outer_constraints.max_height;
    let spacing = self.spacing.borrow();
    let left = padding.get_left().resolve(&spacing, parent_w);
    let right = padding.get_right().resolve(&spacing, parent_w);
    let top = padding.get_top().resolve(&spacing, parent_h);
    let bottom = padding.get_bottom().resolve(&spacing, parent_h);
    let h_pad = left + right;
    let v_pad = top + bottom;

    let inner_constraints = Constraints {
      min_width: (outer_constraints.min_width - h_pad).max(0.0),
      max_width: (outer_constraints.max_width - h_pad).max(0.0),
      min_height: (outer_constraints.min_height - v_pad).max(0.0),
      max_height: (outer_constraints.max_height - v_pad).max(0.0),
    };

    let mut result = self.layout_node_content(glyph_engine, node, inner_constraints, child_overrides);
    result.size = outer_constraints.constrain(Size::new(result.size.width + h_pad, result.size.height + v_pad));

    if left != 0.0 || top != 0.0 {
      for child in &mut result.children {
        child.offset.x += left;
        child.offset.y += top;
      }
    }

    result
  }

  fn frame_constraints_for_node(&self, node: &Node, constraints: Constraints, frame: &FrameConstraints) -> Constraints {
    let resolved_width = frame
      .width
      .and_then(|size| Self::resolve_dimension(size, constraints.max_width));
    let resolved_height = frame
      .height
      .and_then(|size| Self::resolve_dimension(size, constraints.max_height));
    let resolved_min_width = frame
      .min_width
      .and_then(|size| Self::resolve_dimension(size, constraints.max_width));
    let resolved_max_width = frame
      .max_width
      .and_then(|size| Self::resolve_dimension(size, constraints.max_width));
    let resolved_min_height = frame
      .min_height
      .and_then(|size| Self::resolve_dimension(size, constraints.max_height));
    let resolved_max_height = frame
      .max_height
      .and_then(|size| Self::resolve_dimension(size, constraints.max_height));

    let flex_child = node.state_flex().is_some();
    let tight_width = constraints.min_width == constraints.max_width;
    let tight_height = constraints.min_height == constraints.max_height;

    let mut c = constraints;
    if let Some(w) = resolved_width {
      let width = if flex_child && tight_width {
        constraints.max_width
      } else {
        Self::clamp_resolved_dimension(w, resolved_min_width, resolved_max_width)
      };
      c.min_width = width;
      c.max_width = width;
    }
    if let Some(h) = resolved_height {
      let height = if flex_child && tight_height {
        constraints.max_height
      } else {
        Self::clamp_resolved_dimension(h, resolved_min_height, resolved_max_height)
      };
      c.min_height = height;
      c.max_height = height;
    }

    #[cfg(feature = "image")]
    if matches!(
      node.node_kind(),
      NodeKind::Image { .. } | NodeKind::Video { .. } | NodeKind::ResourceImage { .. }
    ) {
      Self::apply_intrinsic_aspect_ratio(node, &mut c, resolved_width, resolved_height);
    }

    #[cfg(all(feature = "svg", feature = "resources"))]
    let is_svg_media = matches!(node.node_kind(), NodeKind::Svg { .. } | NodeKind::ResourceSvg { .. });
    #[cfg(all(feature = "svg", not(feature = "resources")))]
    let is_svg_media = matches!(node.node_kind(), NodeKind::Svg { .. });

    #[cfg(feature = "svg")]
    if is_svg_media {
      Self::apply_intrinsic_aspect_ratio(node, &mut c, resolved_width, resolved_height);
    }

    if resolved_width.is_none()
      && let Some(v) = resolved_min_width
    {
      c.min_width = c.min_width.max(v);
    }
    if resolved_width.is_none()
      && let Some(v) = resolved_max_width
    {
      c.max_width = c.max_width.min(v);
    }
    if resolved_height.is_none()
      && let Some(v) = resolved_min_height
    {
      c.min_height = c.min_height.max(v);
    }
    if resolved_height.is_none()
      && let Some(v) = resolved_max_height
    {
      c.max_height = c.max_height.min(v);
    }

    c.min_width = c.min_width.min(c.max_width);
    c.min_height = c.min_height.min(c.max_height);
    c
  }

  fn clamp_resolved_dimension(value: f32, min: Option<f32>, max: Option<f32>) -> f32 {
    let mut value = value;
    if let Some(min) = min {
      value = value.max(min);
    }
    if let Some(max) = max {
      value = value.min(max);
    }
    value
  }

  #[cfg(any(feature = "image", feature = "svg"))]
  fn apply_intrinsic_aspect_ratio(
    node: &Node,
    constraints: &mut Constraints,
    resolved_width: Option<f32>,
    resolved_height: Option<f32>,
  ) {
    if let Some(intrinsic) = node.intrinsic_size {
      if intrinsic.width > 0.0 && intrinsic.height > 0.0 {
        if let (Some(w), None) = (resolved_width, resolved_height) {
          let h = w * intrinsic.height / intrinsic.width;
          constraints.min_height = h;
          constraints.max_height = h;
        } else if let (None, Some(h)) = (resolved_width, resolved_height) {
          let w = h * intrinsic.width / intrinsic.height;
          constraints.min_width = w;
          constraints.max_width = w;
        }
      }
    }
  }

  fn resolve_dimension(dimension: Dimension, parent_size: f32) -> Option<f32> {
    match dimension {
      Dimension::Auto => None,
      Dimension::Px(value) => Some(value),
      Dimension::Pct(percent) if parent_size.is_finite() => Some(parent_size * percent / 100.0),
      Dimension::Pct(_) => None,
    }
  }

  fn layout_scroll(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    state: &ScrollState,
    direction: ScrollDirection,
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
  ) -> LayoutResult {
    let child = &node.children()[0];
    let style = node.scrollbar_style(self.scrollbar.borrow().clone());

    if style.placement != ScrollBarPlacement::Reserved {
      let child_result = self.layout_child_node(
        glyph_engine,
        child_overrides,
        0,
        child,
        scroll_child_constraints(direction, constraints, constraints.max_width, constraints.max_height),
      );
      let size = scroll_container_size(constraints, &child_result, &style, false, false);
      let viewport = reserved_viewport(size, &style, false, false);
      state.update_layout_with_container(
        child_result.size.width,
        child_result.size.height,
        viewport.width,
        viewport.height,
        size.width,
        size.height,
      );

      return LayoutResult {
        size,
        children: vec![ChildLayout {
          offset: Offset::new(-state.scroll_x(), -state.scroll_y()),
          result: child_result.into(),
        }],
      };
    }

    let mut reserve_vertical = false;
    let mut reserve_horizontal = false;
    let mut child_result = self.layout_child_node(
      glyph_engine,
      child_overrides,
      0,
      child,
      scroll_child_constraints(direction, constraints, constraints.max_width, constraints.max_height),
    );
    let mut size = scroll_container_size(constraints, &child_result, &style, reserve_vertical, reserve_horizontal);

    for _ in 0..3 {
      let viewport = reserved_viewport(size, &style, reserve_vertical, reserve_horizontal);
      child_result = self.layout_child_node(
        glyph_engine,
        child_overrides,
        0,
        child,
        scroll_child_constraints(direction, constraints, viewport.width, viewport.height),
      );
      size = scroll_container_size(constraints, &child_result, &style, reserve_vertical, reserve_horizontal);
      let viewport = reserved_viewport(size, &style, reserve_vertical, reserve_horizontal);
      let next_reserve_vertical = should_reserve_scrollbar(
        &style,
        direction,
        ScrollAxis::Vertical,
        child_result.size.height,
        viewport.height,
      );
      let next_reserve_horizontal = should_reserve_scrollbar(
        &style,
        direction,
        ScrollAxis::Horizontal,
        child_result.size.width,
        viewport.width,
      );

      if next_reserve_vertical == reserve_vertical && next_reserve_horizontal == reserve_horizontal {
        break;
      }

      reserve_vertical = next_reserve_vertical;
      reserve_horizontal = next_reserve_horizontal;
    }

    let viewport = reserved_viewport(size, &style, reserve_vertical, reserve_horizontal);

    state.update_layout_with_container(
      child_result.size.width,
      child_result.size.height,
      viewport.width,
      viewport.height,
      size.width,
      size.height,
    );

    LayoutResult {
      size,
      children: vec![ChildLayout {
        offset: Offset::new(-state.scroll_x(), -state.scroll_y()),
        result: child_result.into(),
      }],
    }
  }

  fn layout_passthrough(
    &self,
    glyph_engine: &mut GlyphEngine,
    node: &Node,
    constraints: Constraints,
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
  ) -> LayoutResult {
    let Some(child) = node.children().first() else {
      return LayoutResult {
        size: Size::new(0.0, 0.0),
        children: Vec::new(),
      };
    };
    let child_result = self.layout_child_node(glyph_engine, child_overrides, 0, child, constraints);
    let size = child_result.size;

    LayoutResult {
      size,
      children: vec![ChildLayout {
        offset: Offset::default(),
        result: child_result.into(),
      }],
    }
  }

  fn layout_child_node(
    &self,
    glyph_engine: &mut GlyphEngine,
    child_overrides: Option<&[Option<ChildLayoutOverride>]>,
    index: usize,
    child: &Node,
    constraints: Constraints,
  ) -> LayoutResult {
    if let Some(Some(child_override)) = child_overrides.and_then(|overrides| overrides.get(index)) {
      if child_override.constraints == constraints {
        return child_override.result.clone();
      }
    }

    self.layout_node(glyph_engine, child, constraints)
  }
}

fn multiline_preferred_height(
  state: &crate::node::node_kind::TextInputState,
  content_height: f32,
  line_height: f32,
) -> f32 {
  let (min_rows, max_rows) = state.row_limits();
  let min_height = min_rows
    .map(|rows| rows.max(1) as f32 * line_height)
    .unwrap_or(line_height);
  let preferred_height = content_height.max(min_height);
  let preferred_height = max_rows
    .map(|rows| preferred_height.min((rows.max(1) as f32 * line_height).max(min_height)))
    .unwrap_or(preferred_height);

  preferred_height
}

fn explicit_multiline_height(text: &str, line_height: f32) -> f32 {
  text.split('\n').count().max(1) as f32 * line_height
}

fn scroll_child_constraints(
  direction: ScrollDirection,
  constraints: Constraints,
  viewport_width: f32,
  viewport_height: f32,
) -> Constraints {
  match direction {
    ScrollDirection::Vertical => Constraints {
      min_width: constraints.min_width.min(viewport_width),
      max_width: viewport_width,
      min_height: 0.0,
      max_height: f32::INFINITY,
    },
    ScrollDirection::Horizontal => Constraints {
      min_width: 0.0,
      max_width: f32::INFINITY,
      min_height: constraints.min_height.min(viewport_height),
      max_height: viewport_height,
    },
    ScrollDirection::Both => Constraints {
      min_width: 0.0,
      max_width: f32::INFINITY,
      min_height: 0.0,
      max_height: f32::INFINITY,
    },
  }
}

fn scroll_container_size(
  constraints: Constraints,
  child_result: &LayoutResult,
  style: &ScrollBarStyle,
  reserve_vertical: bool,
  reserve_horizontal: bool,
) -> Size {
  let reserved_width = if reserve_vertical {
    reserved_scrollbar_size(style)
  } else {
    0.0
  };
  let reserved_height = if reserve_horizontal {
    reserved_scrollbar_size(style)
  } else {
    0.0
  };
  constraints.constrain(Size::new(
    child_result.size.width.max(constraints.min_width) + reserved_width,
    child_result.size.height.max(constraints.min_height) + reserved_height,
  ))
}

fn reserved_viewport(
  container_size: Size,
  style: &ScrollBarStyle,
  reserve_vertical: bool,
  reserve_horizontal: bool,
) -> Size {
  Size::new(
    (container_size.width
      - if reserve_vertical {
        reserved_scrollbar_size(style)
      } else {
        0.0
      })
    .max(0.0),
    (container_size.height
      - if reserve_horizontal {
        reserved_scrollbar_size(style)
      } else {
        0.0
      })
    .max(0.0),
  )
}

fn should_reserve_scrollbar(
  style: &ScrollBarStyle,
  direction: ScrollDirection,
  axis: ScrollAxis,
  content_size: f32,
  viewport_size: f32,
) -> bool {
  if style.placement != ScrollBarPlacement::Reserved || !scroll_direction_has_axis(direction, axis) {
    return false;
  }

  match style.visible {
    ScrollBarVisibility::Never => false,
    ScrollBarVisibility::Always => true,
    ScrollBarVisibility::Auto => content_size > viewport_size,
  }
}

fn reserved_scrollbar_size(style: &ScrollBarStyle) -> f32 {
  style.width + style.padding * 2.0
}

fn scroll_direction_has_axis(direction: ScrollDirection, axis: ScrollAxis) -> bool {
  matches!(
    (direction, axis),
    (ScrollDirection::Horizontal, ScrollAxis::Horizontal)
      | (ScrollDirection::Vertical, ScrollAxis::Vertical)
      | (ScrollDirection::Both, _)
  )
}

fn intersect_clip(parent: ClipRect, child: ClipRect) -> ClipRect {
  if !parent.active {
    return child;
  }
  if !child.active {
    return parent;
  }

  let x1 = parent.x.max(child.x);
  let y1 = parent.y.max(child.y);
  let x2 = (parent.x + parent.width).min(child.x + child.width);
  let y2 = (parent.y + parent.height).min(child.y + child.height);

  ClipRect {
    x: x1,
    y: y1,
    width: (x2 - x1).max(0.0),
    height: (y2 - y1).max(0.0),
    active: true,
    border_radius: intersected_clip_radius(parent, child, x1, y1, x2, y2),
  }
}

fn intersected_clip_radius(
  parent: ClipRect,
  child: ClipRect,
  x1: f32,
  y1: f32,
  x2: f32,
  y2: f32,
) -> Option<crate::node::border::BorderRadius> {
  if same_clip_rect(parent, x1, y1, x2, y2) {
    return parent.border_radius;
  }
  if same_clip_rect(child, x1, y1, x2, y2) {
    return child.border_radius;
  }
  None
}

fn same_clip_rect(clip: ClipRect, x1: f32, y1: f32, x2: f32, y2: f32) -> bool {
  const EPSILON: f32 = 0.001;
  (clip.x - x1).abs() <= EPSILON
    && (clip.y - y1).abs() <= EPSILON
    && (clip.x + clip.width - x2).abs() <= EPSILON
    && (clip.y + clip.height - y2).abs() <= EPSILON
}

fn inset_clip_for_border(clip: ClipRect, border: Option<ResolvedBorders>) -> ClipRect {
  if !clip.active {
    return clip;
  }
  let Some(border) = border else {
    return clip;
  };

  let left = border_clip_inset(border.left);
  let top = border_clip_inset(border.top);
  let right = border_clip_inset(border.right);
  let bottom = border_clip_inset(border.bottom);
  if left == 0.0 && top == 0.0 && right == 0.0 && bottom == 0.0 {
    return clip;
  }

  ClipRect {
    x: clip.x + left,
    y: clip.y + top,
    width: (clip.width - left - right).max(0.0),
    height: (clip.height - top - bottom).max(0.0),
    active: true,
    border_radius: clip
      .border_radius
      .map(|radius| inset_border_radius(radius, left, top, right, bottom)),
  }
}

fn inset_border_radius(
  radius: crate::node::border::BorderRadius,
  left: f32,
  top: f32,
  right: f32,
  bottom: f32,
) -> crate::node::border::BorderRadius {
  crate::node::border::BorderRadius {
    top_left: (radius.top_left - left.max(top)).max(0.0),
    top_right: (radius.top_right - right.max(top)).max(0.0),
    bottom_right: (radius.bottom_right - right.max(bottom)).max(0.0),
    bottom_left: (radius.bottom_left - left.max(bottom)).max(0.0),
  }
}

fn border_clip_inset(border: Option<ResolvedBorder>) -> f32 {
  let Some(border) = border else {
    return 0.0;
  };
  match border.placement {
    BorderPlacement::Inside => border.width,
    BorderPlacement::Center => border.width * 0.5,
    BorderPlacement::Outside => 0.0,
  }
}

fn clipped_subtree_is_hidden(
  node: &Node,
  result: &LayoutResult,
  abs_x: f32,
  abs_y: f32,
  inherited_transform: Transform2D,
  clip: ClipRect,
) -> bool {
  if !clip.active
    || !inherited_transform.is_identity()
    || !node.effective_transform().is_identity()
    || node.overflow == Overflow::Visible
    || border_can_paint_outside(node)
  {
    return false;
  }

  !rect_intersects_clip(abs_x, abs_y, result.size.width, result.size.height, clip)
}

fn node_is_plain_logical_wrapper(node: &Node) -> bool {
  matches!(node.layout_kind(), LayoutKind::LogicalModifier)
    && matches!(node.node_kind(), NodeKind::Empty)
    && node.color.as_ref().is_none()
    && node.gradient.as_ref().is_none()
    && node.border_radius.as_ref().is_none()
    && node.border.as_ref().is_none()
    && node.caret_color.as_ref().is_none()
    && node.caret_mode.as_ref().is_none()
    && node.scrollbar_style.as_ref().is_none()
    && node.state_styles.hovered.is_none()
    && node.state_styles.active.is_none()
    && node.state_styles.focused.is_none()
    && node.opacity == DEFAULT_QUAD_OPACITY
    && node.animation_overrides.is_empty()
    && node.effective_transform().is_identity()
    && node.intrinsic_size.is_none()
    && {
      #[cfg(feature = "image")]
      {
        node.background_image.as_ref().is_none()
      }
      #[cfg(not(feature = "image"))]
      {
        true
      }
    }
}

fn hidden_overflow_creates_clip(has_visual: bool, transform: Transform2D) -> bool {
  transform.is_identity() || has_visual
}

fn border_can_paint_outside(node: &Node) -> bool {
  let Some(borders) = node.get_border() else {
    return false;
  };

  [borders.top, borders.right, borders.bottom, borders.left]
    .into_iter()
    .flatten()
    .any(|border| {
      matches!(
        border.placement,
        crate::node::border::BorderPlacement::Outside | crate::node::border::BorderPlacement::Center
      )
    })
}

fn rect_intersects_clip(x: f32, y: f32, width: f32, height: f32, clip: ClipRect) -> bool {
  width > 0.0
    && height > 0.0
    && x < clip.x + clip.width
    && x + width > clip.x
    && y < clip.y + clip.height
    && y + height > clip.y
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{app::glyph_engine::GlyphEngine, core::Signal, node::Node};

  #[test]
  fn clipped_subtree_culling_keeps_partially_visible_rects() {
    let clip = ClipRect {
      x: 0.0,
      y: 0.0,
      width: 100.0,
      height: 100.0,
      active: true,
      border_radius: None,
    };

    assert!(rect_intersects_clip(90.0, 90.0, 20.0, 20.0, clip));
    assert!(!rect_intersects_clip(120.0, 0.0, 20.0, 20.0, clip));
  }

  #[test]
  fn frame_conflicting_cached_result_is_recomputed() {
    // Production repro (PW-studio text preview): a retained-tree diff can
    // transplant a laid cache onto a node whose fixed frame has since changed,
    // leaving clean flags — a virtualized list's spacer then keeps serving a
    // previous window's height and the viewport goes blank. The layout walk
    // must treat a cache that contradicts the node's own fixed frame as dirty.
    let engine = LayoutEngine::new();
    let mut glyph_engine = GlyphEngine::new();
    let constraints = Constraints::loose(Size::new(400.0, 10_000.0));
    let compute = |engine: &LayoutEngine, glyph_engine: &mut GlyphEngine, node: &Node| {
      engine.compute(
        glyph_engine,
        node,
        constraints,
        ThemePalette::default(),
        ThemeBorderSizes::default(),
        ThemeSpacing::default(),
        ThemeRadii::default(),
        ThemeCaret::default(),
        ScrollBarStyle::default(),
        ThemeTypography::default(),
        false,
      )
    };

    let old: Node = crate::node::Element::from(crate::components::Spacer::new().height(2000.0)).node;
    let laid = compute(&engine, &mut glyph_engine, &old);
    assert_eq!(laid.size.height, 2000.0);

    let mut new: Node = crate::node::Element::from(crate::components::Spacer::new().height(3000.0)).node;
    // The transplant: previously laid cache adopted across a frame change,
    // flags cleared (what `preserve_from` does). Guards are cleared like the
    // retained diff does for unchanged content.
    new.layout_cache.preserve_from(&old.layout_cache);
    new.clear_guards();
    assert!(new.layout_cache.has_cached_result());
    assert!(!new.layout_cache.is_dirty());

    let result = compute(&engine, &mut glyph_engine, &new);
    assert_eq!(
      result.size.height, 3000.0,
      "stale transplanted cache must not override the node's fixed frame"
    );
  }

  #[test]
  fn stale_parent_cache_with_conflicting_child_frame_is_recomputed() {
    // Production repro v2 (PW-studio text preview): the virtualized list's
    // rows COLUMN kept serving a clean-flagged cached result from a previous
    // window build — its spacer child's fixed frame said one height while the
    // column's cached tree still contained the old height, shifting every row
    // off the viewport. The child's own cache was fine, so the per-node
    // frame/cache check didn't fire; the parent's cached tree must be
    // validated against descendant frames.
    let engine = LayoutEngine::new();
    let mut glyph_engine = GlyphEngine::new();
    let constraints = Constraints::loose(Size::new(400.0, 100_000.0));
    let compute = |engine: &LayoutEngine, glyph_engine: &mut GlyphEngine, node: &Node| {
      engine.compute(
        glyph_engine,
        node,
        constraints,
        ThemePalette::default(),
        ThemeBorderSizes::default(),
        ThemeSpacing::default(),
        ThemeRadii::default(),
        ThemeCaret::default(),
        ScrollBarStyle::default(),
        ThemeTypography::default(),
        false,
      )
    };
    let column = |spacer_height: f32| -> Node {
      crate::node::Element::from(
        crate::components::Column::new()
          .spacing(0.0)
          .child(crate::components::Spacer::new().height(spacer_height))
          .child(crate::components::Spacer::new().height(50.0)),
      )
      .node
    };

    let old = column(2000.0);
    let laid = compute(&engine, &mut glyph_engine, &old);
    assert_eq!(laid.size.height, 2050.0);

    let mut new = column(3000.0);
    // Lay the new tree once so every child's own cache is fresh and correct…
    compute(&engine, &mut glyph_engine, &new);
    // …then transplant the OLD column result onto the column node with clean
    // flags (what a retained-diff `preserve_from` chain does).
    new.layout_cache.preserve_from(&old.layout_cache);
    new.clear_guards();
    assert!(!new.layout_cache.is_dirty());

    let result = compute(&engine, &mut glyph_engine, &new);
    assert_eq!(
      result.size.height, 3050.0,
      "a stale parent cache conflicting with a child's fixed frame must be recomputed"
    );
    assert_eq!(result.children[0].result.size.height, 3000.0);
  }

  #[test]
  fn stale_parent_cache_with_conflicting_bounds_override_is_recomputed() {
    // Production repro (PW-studio move drag under an overlay-host root): the
    // overlay flow lays the base subtree first (consuming every dirty flag
    // and repairing the drag's bounds override), then computes the ROOT — the
    // second pass finds nothing dirty and serves the root's cached tree from
    // before the drag, pinning the dragged element to its declared position.
    // A cached offset contradicting a live override must invalidate the tree.
    let engine = LayoutEngine::new();
    let mut glyph_engine = GlyphEngine::new();
    let constraints = Constraints::tight(Size::new(400.0, 300.0));
    let compute = |engine: &LayoutEngine, glyph_engine: &mut GlyphEngine, node: &Node| {
      engine.compute(
        glyph_engine,
        node,
        constraints,
        ThemePalette::default(),
        ThemeBorderSizes::default(),
        ThemeSpacing::default(),
        ThemeRadii::default(),
        ThemeCaret::default(),
        ScrollBarStyle::default(),
        ThemeTypography::default(),
        false,
      )
    };

    let element_ref = crate::core::ElementRefMut::new();
    let root = crate::node::Element::from(
      crate::components::Row::new().child(
        crate::components::Stack::new().size(400.0, 300.0).child(
          crate::components::Rect::new(50.0, 50.0)
            .absolute_position(10.0, 20.0)
            .ref_element(element_ref.clone()),
        ),
      ),
    )
    .node;

    let laid = compute(&engine, &mut glyph_engine, &root);
    let widget_offset = laid.children[0].result.children[0].offset;
    assert_eq!((widget_offset.x, widget_offset.y), (10.0, 20.0));

    // The drag moves the element through its bounds override…
    element_ref.set_bounds(crate::core::ElementRect {
      x: 30.0,
      y: 40.0,
      relative_x: 30.0,
      relative_y: 40.0,
      width: 50.0,
      height: 50.0,
    });
    // …and the base compute (overlay flow's first pass) consumes the dirty
    // flags and repairs the stack subtree.
    let stack = &root.children()[0];
    let repaired = compute(&engine, &mut glyph_engine, stack);
    let repaired_offset = repaired.children[0].offset;
    assert_eq!((repaired_offset.x, repaired_offset.y), (30.0, 40.0));

    // The second, root-level compute must not serve the pre-drag snapshot.
    let result = compute(&engine, &mut glyph_engine, &root);
    let widget_offset = result.children[0].result.children[0].offset;
    assert_eq!(
      (widget_offset.x, widget_offset.y),
      (30.0, 40.0),
      "a cached tree contradicting a live bounds override must be recomputed"
    );
  }

  #[test]
  fn stale_parent_cache_with_conflicting_absolute_position_is_recomputed() {
    // Production repro (PW-studio editor undo): a widget's declared absolute
    // position changes (an edit reverts it) but no dirty flag is set — a
    // retained-diff that preserves the node's cache leaves the parent stack
    // serving the old child offset, so the widget stays put on screen until
    // an unrelated invalidation (a selection click). A cached offset that
    // contradicts the child's declared absolute position must be recomputed.
    let engine = LayoutEngine::new();
    let mut glyph_engine = GlyphEngine::new();
    let constraints = Constraints::tight(Size::new(400.0, 300.0));
    let compute = |engine: &LayoutEngine, glyph_engine: &mut GlyphEngine, node: &Node| {
      engine.compute(
        glyph_engine,
        node,
        constraints,
        ThemePalette::default(),
        ThemeBorderSizes::default(),
        ThemeSpacing::default(),
        ThemeRadii::default(),
        ThemeCaret::default(),
        ScrollBarStyle::default(),
        ThemeTypography::default(),
        false,
      )
    };
    let stack = |x: f32, y: f32| -> Node {
      crate::node::Element::from(
        crate::components::Stack::new()
          .size(400.0, 300.0)
          .child(crate::components::Rect::new(50.0, 50.0).absolute_position(x, y)),
      )
      .node
    };

    let old = stack(10.0, 20.0);
    let laid = compute(&engine, &mut glyph_engine, &old);
    assert_eq!((laid.children[0].offset.x, laid.children[0].offset.y), (10.0, 20.0));

    let mut new = stack(30.0, 40.0);
    // Lay the new tree once so the child's own cache is fresh…
    compute(&engine, &mut glyph_engine, &new);
    // …then transplant the OLD stack cache (old child offset baked in) with
    // clean flags, as a retained-diff `preserve_from` chain would.
    new.layout_cache.preserve_from(&old.layout_cache);
    new.clear_guards();
    assert!(!new.layout_cache.is_dirty());

    let result = compute(&engine, &mut glyph_engine, &new);
    assert_eq!(
      (result.children[0].offset.x, result.children[0].offset.y),
      (30.0, 40.0),
      "a cached offset contradicting the child's declared absolute position must be recomputed"
    );
  }

  #[test]
  fn unchanged_text_input_reuses_cached_layout() {
    let engine = LayoutEngine::new();
    let mut glyph_engine = GlyphEngine::new();
    let node = Node::text_input(Signal::new("Hello".to_owned()));
    let constraints = Constraints::loose(Size::new(400.0, 400.0));

    engine.compute(
      &mut glyph_engine,
      &node,
      constraints,
      ThemePalette::default(),
      ThemeBorderSizes::default(),
      ThemeSpacing::default(),
      ThemeRadii::default(),
      ThemeCaret::default(),
      ScrollBarStyle::default(),
      ThemeTypography::default(),
      false,
    );
    assert!(engine.last_recalculated());

    engine.compute(
      &mut glyph_engine,
      &node,
      constraints,
      ThemePalette::default(),
      ThemeBorderSizes::default(),
      ThemeSpacing::default(),
      ThemeRadii::default(),
      ThemeCaret::default(),
      ScrollBarStyle::default(),
      ThemeTypography::default(),
      false,
    );
    assert!(!engine.last_recalculated());
  }

  #[test]
  fn moved_text_input_caret_invalidates_cached_layout() {
    let engine = LayoutEngine::new();
    let mut glyph_engine = GlyphEngine::new();
    let node = Node::text_input(Signal::new("Hello".to_owned()));
    let constraints = Constraints::loose(Size::new(400.0, 400.0));

    engine.compute(
      &mut glyph_engine,
      &node,
      constraints,
      ThemePalette::default(),
      ThemeBorderSizes::default(),
      ThemeSpacing::default(),
      ThemeRadii::default(),
      ThemeCaret::default(),
      ScrollBarStyle::default(),
      ThemeTypography::default(),
      false,
    );

    let NodeKind::TextInput { state, .. } = node.node_kind() else {
      panic!("expected text input node");
    };
    state.move_left(false);

    engine.compute(
      &mut glyph_engine,
      &node,
      constraints,
      ThemePalette::default(),
      ThemeBorderSizes::default(),
      ThemeSpacing::default(),
      ThemeRadii::default(),
      ThemeCaret::default(),
      ScrollBarStyle::default(),
      ThemeTypography::default(),
      false,
    );
    assert!(engine.last_recalculated());
  }

  #[test]
  fn unchanged_rebuilt_text_input_setters_reuse_cached_layout() {
    let engine = LayoutEngine::new();
    let mut glyph_engine = GlyphEngine::new();
    let constraints = Constraints::loose(Size::new(400.0, 400.0));
    let value = Signal::new(String::new());
    let old = Node::text_input(value.clone())
      .placeholder("Display name")
      .text_input_overflow(TextInputOverflow::Scroll);

    engine.compute(
      &mut glyph_engine,
      &old,
      constraints,
      ThemePalette::default(),
      ThemeBorderSizes::default(),
      ThemeSpacing::default(),
      ThemeRadii::default(),
      ThemeCaret::default(),
      ScrollBarStyle::default(),
      ThemeTypography::default(),
      false,
    );

    let mut new = Node::text_input(value)
      .placeholder("Display name")
      .text_input_overflow(TextInputOverflow::Scroll);
    new.preserve_runtime_state_from(&old);

    engine.compute(
      &mut glyph_engine,
      &new,
      constraints,
      ThemePalette::default(),
      ThemeBorderSizes::default(),
      ThemeSpacing::default(),
      ThemeRadii::default(),
      ThemeCaret::default(),
      ScrollBarStyle::default(),
      ThemeTypography::default(),
      false,
    );

    assert!(!engine.last_recalculated());
  }

  #[test]
  fn dirty_child_that_still_fits_patches_cached_parent_layout() {
    let engine = LayoutEngine::new();
    let mut glyph_engine = GlyphEngine::new();
    let constraints = Constraints::tight(Size::new(200.0, 40.0));
    let old = Node::stack(StackAlignment::TopStart, vec![Node::text("1")]);

    engine.compute(
      &mut glyph_engine,
      &old,
      constraints,
      ThemePalette::default(),
      ThemeBorderSizes::default(),
      ThemeSpacing::default(),
      ThemeRadii::default(),
      ThemeCaret::default(),
      ScrollBarStyle::default(),
      ThemeTypography::default(),
      false,
    );

    let mut new = Node::stack(StackAlignment::TopStart, vec![Node::text("22")]);
    new.preserve_runtime_state_from(&old);

    let result = engine.compute(
      &mut glyph_engine,
      &new,
      constraints,
      ThemePalette::default(),
      ThemeBorderSizes::default(),
      ThemeSpacing::default(),
      ThemeRadii::default(),
      ThemeCaret::default(),
      ScrollBarStyle::default(),
      ThemeTypography::default(),
      false,
    );

    assert!(new.layout_cache.has_cached_result());
    assert_eq!(result.size.width, 200.0);
    assert_eq!(result.size.height, 40.0);
    assert!(result.children[0].result.size.width > 0.0);
    assert!(result.children[0].result.size.width <= result.size.width);
  }

  #[test]
  fn dirty_child_that_no_longer_fits_relayouts_parent() {
    let engine = LayoutEngine::new();
    let mut glyph_engine = GlyphEngine::new();
    let constraints = Constraints::tight(Size::new(20.0, 20.0));
    let old = Node::stack(StackAlignment::TopStart, vec![Node::text("1")]);

    engine.compute(
      &mut glyph_engine,
      &old,
      constraints,
      ThemePalette::default(),
      ThemeBorderSizes::default(),
      ThemeSpacing::default(),
      ThemeRadii::default(),
      ThemeCaret::default(),
      ScrollBarStyle::default(),
      ThemeTypography::default(),
      false,
    );

    let mut new = Node::stack(StackAlignment::TopStart, vec![Node::text("this is too wide")]);
    new.preserve_runtime_state_from(&old);

    engine.compute(
      &mut glyph_engine,
      &new,
      constraints,
      ThemePalette::default(),
      ThemeBorderSizes::default(),
      ThemeSpacing::default(),
      ThemeRadii::default(),
      ThemeCaret::default(),
      ScrollBarStyle::default(),
      ThemeTypography::default(),
      false,
    );

    assert!(engine.last_recalculated());
  }

  #[test]
  fn row_child_size_change_reflows_sibling_offsets() {
    let engine = LayoutEngine::new();
    let mut glyph_engine = GlyphEngine::new();
    let constraints = Constraints::loose(Size::new(400.0, 40.0));
    let old = Node::row(8.0, Alignment::Start, vec![Node::text("1"), Node::text("tail")]);

    engine.compute(
      &mut glyph_engine,
      &old,
      constraints,
      ThemePalette::default(),
      ThemeBorderSizes::default(),
      ThemeSpacing::default(),
      ThemeRadii::default(),
      ThemeCaret::default(),
      ScrollBarStyle::default(),
      ThemeTypography::default(),
      false,
    );

    let mut new = Node::row(
      8.0,
      Alignment::Start,
      vec![Node::text("longer label"), Node::text("tail")],
    );
    new.preserve_runtime_state_from(&old);

    let result = engine.compute(
      &mut glyph_engine,
      &new,
      constraints,
      ThemePalette::default(),
      ThemeBorderSizes::default(),
      ThemeSpacing::default(),
      ThemeRadii::default(),
      ThemeCaret::default(),
      ScrollBarStyle::default(),
      ThemeTypography::default(),
      false,
    );

    let first = &result.children[0];
    let second = &result.children[1];
    assert!(second.offset.x >= first.offset.x + first.result.size.width + 8.0);
  }
}
