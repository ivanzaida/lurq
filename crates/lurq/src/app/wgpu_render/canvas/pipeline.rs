//! Canvas graphics-pipeline contract, shared by ordinary and retained-artwork draws.
use super::*;
pub(super) fn pipeline(
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
    // A whole-tile composite ignores the stencil: its own clipping was already
    // applied to the draws inside the layer.
    compare: if matches!(mode, 0 | 1 | 5 | 6) {
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
  let blend = if mode == 3 || mode == 5 {
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
      buffers: if vs == "vs_main" { &buffers } else { &[] },
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
