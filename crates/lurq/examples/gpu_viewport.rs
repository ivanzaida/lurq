//! Drawing with your own GPU code: a triangle the app renders with its own wgpu pipeline into a texture on lurq's
//! device, shown by a `GpuViewport` in a column of the layout. Drag the triangle to turn it.
//!
//! The guide that walks through it is `docs/src/content/docs/gpu-viewport.md`.
//!
//! cargo run -p lurq --example gpu_viewport --features winit,wgpu,image

use lurq::{
  app::{
    App, Tree,
    component::{Component, DevtoolsInspectable},
    ctx::Ctx,
    events::{DragEvent, MouseButtonMask},
    wgpu_render::{SharedWgpuContext, WgpuFrameExtension, WgpuFrameInfo, WgpuRenderEngine},
    winit_shell::WinitWindow,
  },
  components::{Column, GpuViewport, Text},
  core::Signal,
  images::WgpuExternalImageSlot,
  node::Element,
};

/// lurq samples the texture as sRGB-encoded color, as it does its own pictures.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

const SHADER: &str = r#"
struct Params { angle: f32, aspect: f32, _pad: vec2<f32> }
@group(0) @binding(0) var<uniform> params: Params;

struct Out {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> Out {
  var corners = array<vec2<f32>, 3>(vec2(0.0, 0.7), vec2(-0.6, -0.5), vec2(0.6, -0.5));
  var colors = array<vec3<f32>, 3>(vec3(1.0, 0.25, 0.2), vec3(0.2, 0.8, 0.3), vec3(0.2, 0.4, 1.0));
  let p = corners[index];
  let c = cos(params.angle);
  let s = sin(params.angle);
  var out: Out;
  out.position = vec4((p.x * c - p.y * s) / params.aspect, p.x * s + p.y * c, 0.0, 1.0);
  out.color = colors[index];
  return out;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
  return vec4(in.color, 1.0);
}
"#;

/// What the UI and the renderer share: the slot the viewport shows, and the triangle's angle.
#[derive(Clone)]
struct Scene {
  slot: WgpuExternalImageSlot,
  angle: Signal<f32>,
}

impl PartialEq for Scene {
  fn eq(&self, other: &Self) -> bool {
    self.slot.image_id() == other.slot.image_id() && self.angle.id() == other.angle.id()
  }
}

impl DevtoolsInspectable for Scene {}

/// The app's renderer: an extension of lurq's wgpu renderer, called before each frame's UI pass.
struct Triangle {
  scene: Scene,
  gpu: Option<Made>,
}

/// What the extension made with lurq's device.
struct Made {
  pipeline: wgpu::RenderPipeline,
  params: wgpu::Buffer,
  bind_group: wgpu::BindGroup,
  /// The texture the viewport shows.
  texture: Option<wgpu::Texture>,
  /// The size and the angle drawn into it last.
  drawn: Option<(u32, u32, f32)>,
}

impl WgpuFrameExtension for Triangle {
  fn initialize(&mut self, gpu: &SharedWgpuContext) {
    let shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("triangle"),
      source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("triangle"),
      layout: None,
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: Some("vs_main"),
        buffers: &[],
        compilation_options: Default::default(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs_main"),
        targets: &[Some(FORMAT.into())],
        compilation_options: Default::default(),
      }),
      primitive: wgpu::PrimitiveState::default(),
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview_mask: None,
      cache: None,
    });
    let params = gpu.device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("triangle params"),
      size: 16,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });
    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("triangle"),
      layout: &pipeline.get_bind_group_layout(0),
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: params.as_entire_binding(),
      }],
    });
    self.gpu = Some(Made {
      pipeline,
      params,
      bind_group,
      texture: None,
      drawn: None,
    });
  }

  fn prepare(&mut self, gpu: &SharedWgpuContext, frame: WgpuFrameInfo<'_>) {
    // Where the viewport is in this frame, in physical pixels; `None` when this window does not show it.
    let (Some(made), Some(rect)) = (&mut self.gpu, frame.viewport(self.scene.slot.image_id())) else {
      return;
    };
    let width = (rect.width.round() as u32).max(1);
    let height = (rect.height.round() as u32).max(1);
    let angle = self.scene.angle.get_untracked();
    if made.drawn == Some((width, height, angle)) {
      // Nothing changed: lurq samples the texture as it is.
      return;
    }
    // A texture of the viewport's size in pixels, made again when the viewport is resized.
    let texture = match &made.texture {
      Some(texture) if (texture.width(), texture.height()) == (width, height) => texture,
      _ => {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
          label: Some("triangle target"),
          size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
          },
          mip_level_count: 1,
          sample_count: 1,
          dimension: wgpu::TextureDimension::D2,
          format: FORMAT,
          usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
          view_formats: &[],
        });
        // The viewport shows this texture from now on, without a copy.
        self
          .scene
          .slot
          .set_texture_view(texture.create_view(&Default::default()), width, height);
        made.texture.insert(texture)
      }
    };

    let mut params = [0u8; 16];
    params[..4].copy_from_slice(&angle.to_ne_bytes());
    params[4..8].copy_from_slice(&(width as f32 / height as f32).to_ne_bytes());
    gpu.queue.write_buffer(&made.params, 0, &params);

    let view = texture.create_view(&Default::default());
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("triangle"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &view,
          depth_slice: None,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
              r: 0.02,
              g: 0.02,
              b: 0.05,
              a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
          },
        })],
        ..Default::default()
      });
      pass.set_pipeline(&made.pipeline);
      pass.set_bind_group(0, &made.bind_group, &[]);
      pass.draw(0..3, 0..1);
    }
    // Submitted before lurq's own pass, on the same queue: this frame shows it.
    gpu.queue.submit([encoder.finish()]);
    made.drawn = Some((width, height, angle));
  }

  fn device_lost(&mut self) {
    // Everything made with the lost device is gone: let go of it. lurq empties the slot itself.
    self.gpu = None;
  }
}

/// The window's content: the viewport and a line under it.
struct Viewer {
  scene: Scene,
}

impl Component for Viewer {
  type Props = Scene;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      scene: ctx.props::<Scene>().clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    // Read here, the angle renders this component again when a drag changes it, which draws a frame.
    let degrees = self.scene.angle.get().to_degrees();
    let angle = self.scene.angle.clone();
    Column::new()
      .padding(24.0)
      .spacing(12.0)
      .child(
        GpuViewport::new(self.scene.slot.clone())
          .size(480.0, 270.0)
          .rounded(8.0)
          .start_drag_buttons(MouseButtonMask::LEFT)
          .on_drag_move(move |event: DragEvent| angle.set(angle.get_untracked() + event.delta_x * 0.01)),
      )
      .child(Text::new(&format!(
        "Turned {degrees:.0} degrees. Drag the triangle to turn it."
      )))
  }
}

fn main() {
  let scene = Scene {
    slot: WgpuExternalImageSlot::new(),
    angle: Signal::new(0.0),
  };
  let mut app = App::new();
  let mut tree = Tree::new();
  // An engine for each window, and another after a lost device: each with an extension of its own.
  let for_engines = scene.clone();
  tree.set_render_engine_factory(move || {
    let triangle = Triangle {
      scene: for_engines.clone(),
      gpu: None,
    };
    Box::new(WgpuRenderEngine::new().with_frame_extension(triangle))
  });
  tree.mount_root::<Viewer>(&mut app, scene);
  WinitWindow::new(app, tree)
    .with_size(528, 360)
    .with_title("GPU viewport")
    .run();
}
