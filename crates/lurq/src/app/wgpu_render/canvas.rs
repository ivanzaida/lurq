use std::collections::{HashMap, HashSet, VecDeque};

use wgpu::*;

use super::DynamicBuffer;
use crate::canvas::{CanvasError, CanvasHandle, CanvasId, CanvasSnapshot, CanvasWeak, gpu::*};

struct Texture {
  texture: wgpu::Texture,
  view: TextureView,
  nearest: BindGroup,
  linear: BindGroup,
}
struct Backing {
  owner: CanvasWeak,
  image: Texture,
  generation: u64,
}
struct CachedAsset {
  image: Texture,
  bytes: usize,
  last: u64,
}
pub(super) struct Renderer {
  meshes: MeshCache,
  surfaces: HashMap<CanvasId, Backing>,
  assets: HashMap<u64, CachedAsset>,
  asset_bytes: usize,
  tick: u64,
  generation: u64,
  globals_layout: BindGroupLayout,
  image_layout: BindGroupLayout,
  nearest: Sampler,
  linear: Sampler,
  _scratch: wgpu::Texture,
  scratch_view: TextureView,
  resolve: wgpu::Texture,
  resolve_view: TextureView,
  _stencil: wgpu::Texture,
  stencil_view: TextureView,
  white: Texture,
  seed: RenderPipeline,
  resample: RenderPipeline,
  reset_stencil: RenderPipeline,
  clip: RenderPipeline,
  solid: RenderPipeline,

  premul: RenderPipeline,
  erase: RenderPipeline,
  vertices: DynamicBuffer,
  globals: DynamicBuffer,
}
impl Renderer {
  pub fn new(device: &Device, queue: &Queue) -> Self {
    let globals_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
      label: Some("canvas globals"),
      entries: &[BindGroupLayoutEntry {
        binding: 0,
        visibility: ShaderStages::VERTEX,
        ty: BindingType::Buffer {
          ty: BufferBindingType::Uniform,
          has_dynamic_offset: true,
          min_binding_size: BufferSize::new(32),
        },
        count: None,
      }],
    });
    let image_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
      label: Some("canvas image"),
      entries: &[
        BindGroupLayoutEntry {
          binding: 0,
          visibility: ShaderStages::FRAGMENT,
          ty: BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable: true },
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
          },
          count: None,
        },
        BindGroupLayoutEntry {
          binding: 1,
          visibility: ShaderStages::FRAGMENT,
          ty: BindingType::Sampler(SamplerBindingType::Filtering),
          count: None,
        },
      ],
    });
    let nearest = device.create_sampler(&SamplerDescriptor::default());
    let linear = device.create_sampler(&SamplerDescriptor {
      mag_filter: FilterMode::Linear,
      min_filter: FilterMode::Linear,
      ..Default::default()
    });
    let shader = device.create_shader_module(ShaderModuleDescriptor {
      label: Some("canvas"),
      source: ShaderSource::Wgsl(include_str!("shaders/canvas.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
      label: Some("canvas"),
      bind_group_layouts: &[Some(&globals_layout), Some(&image_layout)],
      immediate_size: 0,
    });
    let make = |vs, fs, samples, mode| pipeline(device, &layout, &shader, vs, fs, samples, mode);
    let seed = make("vs_seed", "fs_premul", 4, 0);
    let resample = make("vs_seed", "fs_premul", 1, 0);
    let reset_stencil = make("vs_seed", "fs_solid", 4, 1);
    let clip = make("vs_main", "fs_solid", 4, 2);
    let solid = make("vs_main", "fs_solid", 4, 3);

    let premul = make("vs_main", "fs_premul", 4, 3);
    let erase = make("vs_main", "fs_solid", 4, 4);
    let texture = |format, samples| {
      device.create_texture(&TextureDescriptor {
        label: Some("canvas shared tile"),
        size: Extent3d {
          width: TILE,
          height: TILE,
          depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: samples,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::RENDER_ATTACHMENT
          | if samples == 1 {
            TextureUsages::COPY_SRC
          } else {
            TextureUsages::empty()
          },
        view_formats: &[],
      })
    };
    let scratch = texture(TextureFormat::Rgba8Unorm, 4);
    let scratch_view = scratch.create_view(&Default::default());
    let resolve = texture(TextureFormat::Rgba8Unorm, 1);
    let resolve_view = resolve.create_view(&Default::default());
    let stencil = texture(TextureFormat::Depth24PlusStencil8, 4);
    let stencil_view = stencil.create_view(&Default::default());
    let white = Texture::new(device, &image_layout, &nearest, &linear, 1, 1);
    queue.write_texture(
      white.texture.as_image_copy(),
      &[255; 4],
      TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(4),
        rows_per_image: Some(1),
      },
      Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
      },
    );
    Self {
      meshes: MeshCache::default(),
      surfaces: HashMap::new(),
      assets: HashMap::new(),
      asset_bytes: 0,
      tick: 0,
      generation: 0,
      globals_layout,
      image_layout,
      nearest,
      linear,
      _scratch: scratch,
      scratch_view,
      resolve,
      resolve_view,
      _stencil: stencil,
      stencil_view,
      white,
      seed,
      resample,
      reset_stencil,
      clip,
      solid,
      premul,
      erase,
      vertices: DynamicBuffer::new("canvas vertices", BufferUsages::VERTEX),
      globals: DynamicBuffer::new("canvas globals", BufferUsages::UNIFORM),
    }
  }
  pub fn snapshot(&self, canvas: &CanvasHandle) -> Option<crate::images::WgpuExternalImageSnapshot> {
    let b = self.surfaces.get(&canvas.surface_id())?;
    Some(crate::images::WgpuExternalImageSnapshot {
      view: b.image.view.clone(),
      width: b.image.texture.width(),
      height: b.image.texture.height(),
      version: b.generation,
    })
  }
  pub fn process(&mut self, device: &Device, queue: &Queue, canvases: &[CanvasHandle]) {
    self.tick += 1;
    let live: HashSet<_> = canvases
      .iter()
      .filter(|c| c.is_attached())
      .map(CanvasHandle::surface_id)
      .collect();
    self.surfaces.retain(|id, b| {
      let keep = live.contains(id);
      if !keep {
        if let Some(c) = b.owner.upgrade() {
          c.set_gpu_bytes(0);
        }
      }
      keep
    });
    for canvas in canvases {
      let Some(mut batch) = canvas.take_batch() else {
        continue;
      };
      let mut commands: VecDeque<_> = batch.take_commands().into();
      if !self.surfaces.contains_key(&canvas.surface_id()) && !matches!(commands.front(), Some(Command::Resize { .. }))
      {
        let metrics = canvas.metrics();
        self.resize(device, queue, canvas, metrics.pixel_width, metrics.pixel_height, false);
      }
      while let Some(command) = commands.pop_front() {
        match command {
          Command::Resize {
            width,
            height,
            preserve,
          } => {
            if !self.surfaces.contains_key(&canvas.surface_id()) && commands.is_empty() {
              continue;
            }
            self.resize(device, queue, canvas, width, height, preserve);
          }
          Command::Readback(done, metrics, revision) => {
            if let Some(backing) = self.surfaces.get(&canvas.surface_id()) {
              readback(device, queue, &backing.image.texture, done, revision);
            } else if metrics.pixel_width != 0 && metrics.pixel_height != 0 {
              done.finish(Err(canvas.status().error.unwrap_or(CanvasError::RendererLost)));
            } else {
              done.finish(Ok(CanvasSnapshot {
                width: metrics.pixel_width,
                height: metrics.pixel_height,
                rgba: Vec::new(),
                revision,
              }));
            }
          }
          first => {
            let mut group = vec![first];
            while commands
              .front()
              .is_some_and(|c| !matches!(c, Command::Resize { .. } | Command::Readback(..)))
            {
              group.push(commands.pop_front().unwrap());
            }
            match Prepared::new(&group, &mut self.meshes) {
              Ok(prepared) => self.draw(device, queue, canvas.surface_id(), &prepared),
              Err(error) => canvas.set_gpu_error(error),
            }
          }
        }
      }
      batch.submit();
    }
    // An LRU byte cap includes images and shaped text. No drawing history is kept.
    while self.asset_bytes > 64 * 1024 * 1024 {
      let Some(id) = self.assets.iter().min_by_key(|(_, a)| a.last).map(|(id, _)| *id) else {
        break;
      };
      self.asset_bytes -= self.assets.remove(&id).unwrap().bytes;
    }
  }
  fn resize(&mut self, device: &Device, queue: &Queue, canvas: &CanvasHandle, width: u32, height: u32, preserve: bool) {
    let id = canvas.surface_id();
    let old = self.surfaces.remove(&id);
    canvas.set_gpu_bytes(0);
    if width == 0 || height == 0 {
      canvas.set_gpu_bytes(0);
      return;
    }
    if width > device.limits().max_texture_dimension_2d || height > device.limits().max_texture_dimension_2d {
      canvas.set_gpu_error(CanvasError::SurfaceTooLarge);
      return;
    }
    let image = Texture::new(device, &self.image_layout, &self.nearest, &self.linear, width, height);
    let mut encoder = device.create_command_encoder(&Default::default());
    let constants = [0., 0., width as f32, height as f32, width as f32, height as f32, 0., 0.];
    let buffer = self.globals.write(device, queue, &constants).unwrap();
    let globals = global_group(device, &self.globals_layout, buffer);
    {
      let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("canvas resize"),
        color_attachments: &[Some(RenderPassColorAttachment {
          view: &image.view,
          depth_slice: None,
          resolve_target: None,
          ops: Operations {
            load: LoadOp::Clear(Color::TRANSPARENT),
            store: StoreOp::Store,
          },
        })],
        ..Default::default()
      });
      if preserve && let Some(old) = &old {
        pass.set_pipeline(&self.resample);
        pass.set_bind_group(0, &globals, &[0]);
        pass.set_bind_group(1, &old.image.linear, &[]);
        pass.draw(0..3, 0..1);
      }
    }
    queue.submit([encoder.finish()]);
    self.generation += 1;
    self.surfaces.insert(
      id,
      Backing {
        owner: canvas.downgrade(),
        image,
        generation: self.generation,
      },
    );
    canvas.set_gpu_bytes(width as usize * height as usize * 4);
  }
  fn draw(&mut self, device: &Device, queue: &Queue, id: CanvasId, prepared: &Prepared) {
    let Some(backing) = self.surfaces.get(&id) else {
      return;
    };
    let (width, height) = (backing.image.texture.width(), backing.image.texture.height());
    let mut uploaded = 0;
    if prepared.draws.iter().filter_map(|d| d.asset.as_ref()).any(|a| {
      a.width > device.limits().max_texture_dimension_2d || a.height > device.limits().max_texture_dimension_2d
    }) {
      if let Some(canvas) = backing.owner.upgrade() {
        canvas.set_gpu_error(CanvasError::SurfaceTooLarge);
      }
      return;
    }
    for draw in &prepared.draws {
      if let Some(asset) = &draw.asset {
        if !self.assets.contains_key(&asset.id) {
          let image = Texture::new(
            device,
            &self.image_layout,
            &self.nearest,
            &self.linear,
            asset.width,
            asset.height,
          );
          let mut converted = Vec::new();
          let data = if asset.premultiplied {
            asset.data.as_slice()
          } else {
            converted.extend_from_slice(&asset.data);
            for p in converted.chunks_exact_mut(4) {
              let a = u16::from(p[3]);
              for c in &mut p[..3] {
                *c = ((u16::from(*c) * a + 127) / 255) as u8;
              }
            }
            &converted
          };
          queue.write_texture(
            image.texture.as_image_copy(),
            data,
            TexelCopyBufferLayout {
              offset: 0,
              bytes_per_row: Some(asset.width * 4),
              rows_per_image: Some(asset.height),
            },
            Extent3d {
              width: asset.width,
              height: asset.height,
              depth_or_array_layers: 1,
            },
          );
          let bytes = asset.data.len().max(64 * 1024);
          self.asset_bytes += bytes;
          uploaded += asset.data.len();
          self.assets.insert(
            asset.id,
            CachedAsset {
              image,
              bytes,
              last: self.tick,
            },
          );
        }
        self.assets.get_mut(&asset.id).unwrap().last = self.tick;
      }
    }
    let mut encoder = device.create_command_encoder(&Default::default());
    if prepared.clear {
      let _pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("canvas clear"),
        color_attachments: &[Some(RenderPassColorAttachment {
          view: &backing.image.view,
          depth_slice: None,
          resolve_target: None,
          ops: Operations {
            load: LoadOp::Clear(Color::TRANSPARENT),
            store: StoreOp::Store,
          },
        })],
        ..Default::default()
      });
    }
    let tiles = prepared.tiles(width, height);
    if !tiles.is_empty() {
      let alignment = device.limits().min_uniform_buffer_offset_alignment as usize / 4;
      let mut constants = vec![0f32; tiles.len() * alignment];
      for (index, tile) in tiles.iter().enumerate() {
        constants[index * alignment..index * alignment + 8].copy_from_slice(&[
          tile[0] as f32,
          tile[1] as f32,
          TILE as f32,
          TILE as f32,
          width as f32,
          height as f32,
          0.,
          0.,
        ]);
      }
      let globals = global_group(
        device,
        &self.globals_layout,
        self.globals.write(device, queue, &constants).unwrap(),
      );
      let vertices = self.vertices.write(device, queue, &prepared.vertices).unwrap();
      for (index, tile) in tiles.iter().copied().enumerate() {
        {
          let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("canvas dirty tile"),
            color_attachments: &[Some(RenderPassColorAttachment {
              view: &self.scratch_view,
              depth_slice: None,
              resolve_target: Some(&self.resolve_view),
              ops: Operations {
                load: LoadOp::Clear(Color::TRANSPARENT),
                store: StoreOp::Discard,
              },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
              view: &self.stencil_view,
              depth_ops: None,
              stencil_ops: Some(Operations {
                load: LoadOp::Clear(0),
                store: StoreOp::Discard,
              }),
            }),
            ..Default::default()
          });
          pass.set_scissor_rect(0, 0, tile[2], tile[3]);
          pass.set_bind_group(0, &globals, &[(index * alignment * 4) as u32]);
          pass.set_bind_group(1, &backing.image.nearest, &[]);
          pass.set_pipeline(&self.seed);
          pass.draw(0..3, 0..1);
          pass.set_vertex_buffer(0, vertices.slice(..));
          let mut clips: Option<&Vec<std::ops::Range<u32>>> = None;
          for draw in prepared.draws.iter().filter(|d| d.intersects(tile)) {
            if clips != Some(&draw.clips) {
              pass.set_bind_group(1, &self.white.nearest, &[]);
              pass.set_pipeline(&self.reset_stencil);
              pass.set_stencil_reference(0);
              pass.draw(0..3, 0..1);
              pass.set_pipeline(&self.clip);
              for (level, range) in draw.clips.iter().enumerate() {
                pass.set_stencil_reference(level as u32);
                pass.draw(range.clone(), 0..1);
              }
              clips = Some(&draw.clips);
            }
            pass.set_stencil_reference(draw.clips.len() as u32);
            let pipeline = if draw.erase {
              &self.erase
            } else if draw.asset.is_some() {
              &self.premul
            } else {
              &self.solid
            };
            pass.set_pipeline(pipeline);
            let texture = draw
              .asset
              .as_ref()
              .map(|a| &self.assets[&a.id].image)
              .unwrap_or(&self.white);
            pass.set_bind_group(1, if draw.smooth { &texture.linear } else { &texture.nearest }, &[]);
            pass.draw(draw.vertices.clone(), 0..1);
          }
        }
        encoder.copy_texture_to_texture(
          self.resolve.as_image_copy(),
          TexelCopyTextureInfo {
            origin: Origin3d {
              x: tile[0],
              y: tile[1],
              z: 0,
            },
            ..backing.image.texture.as_image_copy()
          },
          Extent3d {
            width: tile[2],
            height: tile[3],
            depth_or_array_layers: 1,
          },
        );
      }
    }
    queue.submit([encoder.finish()]);
    if let Some(canvas) = backing.owner.upgrade() {
      canvas.record_gpu_update(prepared.vertices.len(), tiles.len(), uploaded);
    }
  }
}
impl Texture {
  fn new(
    device: &Device,
    layout: &BindGroupLayout,
    nearest: &Sampler,
    linear: &Sampler,
    width: u32,
    height: u32,
  ) -> Self {
    let texture = device.create_texture(&TextureDescriptor {
      label: Some("canvas pixels"),
      size: Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: TextureDimension::D2,
      format: TextureFormat::Rgba8Unorm,
      usage: TextureUsages::TEXTURE_BINDING
        | TextureUsages::RENDER_ATTACHMENT
        | TextureUsages::COPY_DST
        | TextureUsages::COPY_SRC,
      view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let bind = |sampler| {
      device.create_bind_group(&BindGroupDescriptor {
        label: Some("canvas pixels"),
        layout,
        entries: &[
          BindGroupEntry {
            binding: 0,
            resource: BindingResource::TextureView(&view),
          },
          BindGroupEntry {
            binding: 1,
            resource: BindingResource::Sampler(sampler),
          },
        ],
      })
    };
    let nearest = bind(nearest);
    let linear = bind(linear);
    Self {
      texture,
      view,
      nearest,
      linear,
    }
  }
}
fn global_group(device: &Device, layout: &BindGroupLayout, buffer: &Buffer) -> BindGroup {
  device.create_bind_group(&BindGroupDescriptor {
    label: Some("canvas globals"),
    layout,
    entries: &[BindGroupEntry {
      binding: 0,
      resource: BindingResource::Buffer(BufferBinding {
        buffer,
        offset: 0,
        size: BufferSize::new(32),
      }),
    }],
  })
}
fn pipeline(
  device: &Device,
  layout: &PipelineLayout,
  shader: &ShaderModule,
  vs: &str,
  fs: &str,
  samples: u32,
  mode: u8,
) -> RenderPipeline {
  let attributes = vertex_attr_array![0=>Float32x2,1=>Float32x2,2=>Float32x4];
  let buffers = [VertexBufferLayout {
    array_stride: std::mem::size_of::<Vertex>() as u64,
    step_mode: VertexStepMode::Vertex,
    attributes: &attributes,
  }];
  let stencil = StencilFaceState {
    compare: if mode == 1 || mode == 0 {
      CompareFunction::Always
    } else {
      CompareFunction::Equal
    },
    fail_op: StencilOperation::Keep,
    depth_fail_op: StencilOperation::Keep,
    pass_op: match mode {
      1 => StencilOperation::Replace,
      2 => StencilOperation::IncrementClamp,
      _ => StencilOperation::Keep,
    },
  };
  let blend = if mode == 3 {
    Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING)
  } else if mode == 4 {
    Some(BlendState {
      color: BlendComponent {
        src_factor: BlendFactor::Zero,
        dst_factor: BlendFactor::Zero,
        operation: BlendOperation::Add,
      },
      alpha: BlendComponent {
        src_factor: BlendFactor::Zero,
        dst_factor: BlendFactor::Zero,
        operation: BlendOperation::Add,
      },
    })
  } else {
    None
  };
  device.create_render_pipeline(&RenderPipelineDescriptor {
    label: Some("canvas"),
    layout: Some(layout),
    vertex: VertexState {
      module: shader,
      entry_point: Some(vs),
      compilation_options: Default::default(),
      buffers: if vs == "vs_seed" { &[] } else { &buffers },
    },
    fragment: Some(FragmentState {
      module: shader,
      entry_point: Some(fs),
      compilation_options: Default::default(),
      targets: &[Some(ColorTargetState {
        format: TextureFormat::Rgba8Unorm,
        blend,
        write_mask: if mode == 1 || mode == 2 {
          ColorWrites::empty()
        } else {
          ColorWrites::ALL
        },
      })],
    }),
    primitive: PrimitiveState::default(),
    depth_stencil: if samples == 4 {
      Some(DepthStencilState {
        format: TextureFormat::Depth24PlusStencil8,
        depth_write_enabled: Some(false),
        depth_compare: Some(CompareFunction::Always),
        stencil: StencilState {
          front: stencil,
          back: stencil,
          read_mask: 255,
          write_mask: if mode == 1 || mode == 2 { 255 } else { 0 },
        },
        bias: DepthBiasState::default(),
      })
    } else {
      None
    },
    multisample: MultisampleState {
      count: samples,
      ..Default::default()
    },
    multiview_mask: None,
    cache: None,
  })
}
fn readback(device: &Device, queue: &Queue, texture: &wgpu::Texture, done: Completion, revision: u64) {
  let (width, height) = (texture.width(), texture.height());
  let pitch = (width * 4).div_ceil(256) * 256;
  let buffer = device.create_buffer(&BufferDescriptor {
    label: Some("canvas explicit readback"),
    size: u64::from(pitch) * u64::from(height),
    usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
  let mut encoder = device.create_command_encoder(&Default::default());
  encoder.copy_texture_to_buffer(
    texture.as_image_copy(),
    TexelCopyBufferInfo {
      buffer: &buffer,
      layout: TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(pitch),
        rows_per_image: Some(height),
      },
    },
    Extent3d {
      width,
      height,
      depth_or_array_layers: 1,
    },
  );
  queue.submit([encoder.finish()]);
  let device = device.clone();
  std::thread::spawn(move || {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    buffer.slice(..).map_async(MapMode::Read, move |result| {
      let _ = tx.send(result);
    });
    if device.poll(PollType::wait_indefinitely()).is_err() || !matches!(rx.recv(), Ok(Ok(()))) {
      done.finish(Err(CanvasError::RendererLost));
      return;
    }
    let mapped = buffer.slice(..).get_mapped_range();
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for row in mapped.chunks_exact(pitch as usize) {
      rgba.extend_from_slice(&row[..width as usize * 4]);
    }
    drop(mapped);
    buffer.unmap();
    done.finish(Ok(CanvasSnapshot {
      width,
      height,
      rgba: unpremultiply(rgba),
      revision,
    }));
  });
}

impl Drop for Renderer {
  fn drop(&mut self) {
    for backing in self.surfaces.values() {
      if let Some(c) = backing.owner.upgrade() {
        c.set_gpu_bytes(0);
        c.set_gpu_error(CanvasError::RendererLost);
      }
    }
  }
}

#[cfg(test)]
mod camera_tests;
#[cfg(test)]
mod tests;
