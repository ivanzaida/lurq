//! Offscreen targets for flattened group opacity (see
//! [`crate::layout::opacity_layer`]) and the pipeline that composites them.
//!
//! Layer textures are pooled across frames: a frame takes the smallest free
//! texture that fits each layer, creating one (rounded up to
//! [`LAYER_SIZE_STEP`]) only when none does, and textures unused for
//! [`LAYER_IDLE_FRAMES`] frames are released. Nothing is created until a
//! frame has a layer.

use super::vertex::QuadVertex;
use crate::layout::opacity_layer::{LayerCmd, LayerComposite, TargetSpace};

/// Layer textures grow in steps of this many pixels, so a layer that changes
/// size a little (an animation) keeps its texture.
const LAYER_SIZE_STEP: u32 = 64;
/// Frames a pooled layer texture may stay unused before it is released.
const LAYER_IDLE_FRAMES: u64 = 120;

pub(super) struct LayerCompositor {
  pipeline: wgpu::RenderPipeline,
  bind_group_layout: wgpu::BindGroupLayout,
  format: wgpu::TextureFormat,
  slots: Vec<LayerSlot>,
  frame: u64,
}

struct LayerSlot {
  view: wgpu::TextureView,
  uniform: wgpu::Buffer,
  bind_group: wgpu::BindGroup,
  width: u32,
  height: u32,
  in_use: bool,
  last_used: u64,
}

impl LayerCompositor {
  /// `format` is the render target format the other pipelines draw in; the
  /// layers use it too, so every pipeline draws into them unchanged.
  pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: Some("lurq_layer_bgl"),
      entries: &[
        wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
          },
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
          },
          count: None,
        },
      ],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("lurq_layer_shader"),
      source: wgpu::ShaderSource::Wgsl(include_str!("shaders/layer.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("lurq_layer_pl"),
      bind_group_layouts: &[Some(&bind_group_layout)],
      immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("lurq_layer_pipeline"),
      layout: Some(&layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: Some("vs_main"),
        buffers: &[QuadVertex::desc()],
        compilation_options: Default::default(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs_main"),
        targets: &[Some(wgpu::ColorTargetState {
          format,
          blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
          write_mask: wgpu::ColorWrites::ALL,
        })],
        compilation_options: Default::default(),
      }),
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        ..Default::default()
      },
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview_mask: None,
      cache: None,
    });
    Self {
      pipeline,
      bind_group_layout,
      format,
      slots: Vec::new(),
      frame: 0,
    }
  }

  pub(super) fn format(&self) -> wgpu::TextureFormat {
    self.format
  }

  pub(super) fn begin_frame(&mut self) {
    self.frame += 1;
    for slot in &mut self.slots {
      slot.in_use = false;
    }
  }

  /// A free texture of at least `width` x `height` for this frame.
  pub(super) fn acquire(&mut self, device: &wgpu::Device, width: u32, height: u32) -> usize {
    let fitting = self
      .slots
      .iter()
      .enumerate()
      .filter(|(_, slot)| !slot.in_use && slot.width >= width && slot.height >= height)
      .min_by_key(|(_, slot)| u64::from(slot.width) * u64::from(slot.height))
      .map(|(index, _)| index);
    let index = fitting.unwrap_or_else(|| {
      let limit = device.limits().max_texture_dimension_2d;
      let size = |value: u32| {
        value
          .div_ceil(LAYER_SIZE_STEP)
          .saturating_mul(LAYER_SIZE_STEP)
          .min(limit)
      };
      self.slots.push(self.create_slot(device, size(width), size(height)));
      self.slots.len() - 1
    });
    let slot = &mut self.slots[index];
    slot.in_use = true;
    slot.last_used = self.frame;
    index
  }

  fn create_slot(&self, device: &wgpu::Device, width: u32, height: u32) -> LayerSlot {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("lurq_opacity_layer"),
      size: wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: self.format,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
      view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let uniform = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("lurq_layer_composite"),
      size: std::mem::size_of::<LayerComposite>() as wgpu::BufferAddress,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("lurq_layer_bg"),
      layout: &self.bind_group_layout,
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: uniform.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::TextureView(&view),
        },
      ],
    });
    LayerSlot {
      view,
      uniform,
      bind_group,
      width,
      height,
      in_use: false,
      last_used: 0,
    }
  }

  /// The pixel space of `layer` painted into `slot`.
  pub(super) fn space(&self, slot: usize, layer: &LayerCmd) -> TargetSpace {
    let slot = &self.slots[slot];
    TargetSpace::layer(layer.bounds, slot.width, slot.height)
  }

  pub(super) fn view(&self, slot: usize) -> &wgpu::TextureView {
    &self.slots[slot].view
  }

  /// Draws `slot`, holding `layer`, into the pass painting `parent`.
  pub(super) fn composite(
    &self,
    queue: &wgpu::Queue,
    pass: &mut wgpu::RenderPass<'_>,
    slot: usize,
    layer: &LayerCmd,
    parent: &TargetSpace,
  ) {
    let slot = &self.slots[slot];
    // Each layer is composited once per frame, so its own buffer holds the
    // values until the frame's submit.
    queue.write_buffer(
      &slot.uniform,
      0,
      bytemuck::bytes_of(&LayerComposite::new(layer, parent)),
    );
    pass.set_pipeline(&self.pipeline);
    pass.set_bind_group(0, &slot.bind_group, &[]);
    pass.draw_indexed(0..QuadVertex::INDICES.len() as u32, 0, 0..1);
  }

  /// Releases textures no frame has used for a while.
  pub(super) fn end_frame(&mut self) {
    let frame = self.frame;
    self
      .slots
      .retain(|slot| slot.in_use || frame.saturating_sub(slot.last_used) <= LAYER_IDLE_FRAMES);
  }

  #[cfg(all(test, feature = "screenshot"))]
  pub(super) fn texture_count(&self) -> usize {
    self.slots.len()
  }
}
