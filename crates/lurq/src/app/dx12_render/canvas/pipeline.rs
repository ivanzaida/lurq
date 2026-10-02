//! Fixed Canvas graphics-pipeline creation and shader input contracts.
use super::*;

pub(super) unsafe fn pipeline(
  device: &ID3D12Device,
  root: &ID3D12RootSignature,
  vs_entry: &'static [u8],
  ps_entry: &'static [u8],
  samples: u32,
  mode: u8,
) -> Result<ID3D12PipelineState> {
  let vs = compile_shader(include_bytes!("../shaders/canvas.hlsl"), vs_entry, b"vs_5_0\0")?;
  let ps = compile_shader(include_bytes!("../shaders/canvas.hlsl"), ps_entry, b"ps_5_0\0")?;
  let elements = [
    (b"POSITION\0".as_slice(), DXGI_FORMAT_R32G32_FLOAT, 0),
    (b"TEXCOORD\0".as_slice(), DXGI_FORMAT_R32G32_FLOAT, 8),
    (b"COLOR\0".as_slice(), DXGI_FORMAT_R32G32B32A32_FLOAT, 16),
  ]
  .map(|(name, format, offset)| D3D12_INPUT_ELEMENT_DESC {
    SemanticName: PCSTR(name.as_ptr()),
    Format: format,
    AlignedByteOffset: offset,
    InputSlotClass: D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
    ..Default::default()
  });
  let blend = D3D12_RENDER_TARGET_BLEND_DESC {
    // Mode 6 writes the finished composite itself, so it must not be blended.
    BlendEnable: if mode >= 3 && mode != 6 { TRUE } else { FALSE },
    SrcBlend: if mode == 4 { D3D12_BLEND_ZERO } else { D3D12_BLEND_ONE },
    DestBlend: if mode == 4 {
      D3D12_BLEND_ZERO
    } else {
      D3D12_BLEND_INV_SRC_ALPHA
    },
    BlendOp: D3D12_BLEND_OP_ADD,
    SrcBlendAlpha: if mode == 4 { D3D12_BLEND_ZERO } else { D3D12_BLEND_ONE },
    DestBlendAlpha: if mode == 4 {
      D3D12_BLEND_ZERO
    } else {
      D3D12_BLEND_INV_SRC_ALPHA
    },
    BlendOpAlpha: D3D12_BLEND_OP_ADD,
    LogicOp: D3D12_LOGIC_OP_NOOP,
    RenderTargetWriteMask: if mode == 2 { 0 } else { 15 },
    ..Default::default()
  };
  let face = D3D12_DEPTH_STENCILOP_DESC {
    StencilFailOp: D3D12_STENCIL_OP_KEEP,
    StencilDepthFailOp: D3D12_STENCIL_OP_KEEP,
    StencilPassOp: if mode == 2 {
      D3D12_STENCIL_OP_INCR_SAT
    } else {
      D3D12_STENCIL_OP_KEEP
    },
    // A whole-tile composite ignores the stencil: the draws inside the layer
    // were already clipped when they were drawn.
    StencilFunc: if mode == 0 || mode >= 5 {
      D3D12_COMPARISON_FUNC_ALWAYS
    } else {
      D3D12_COMPARISON_FUNC_EQUAL
    },
  };
  let mut formats = [DXGI_FORMAT_UNKNOWN; 8];
  formats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;
  let mut desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
    pRootSignature: ManuallyDrop::new(Some(root.clone())),
    VS: shader_bytecode(&vs),
    PS: shader_bytecode(&ps),
    BlendState: D3D12_BLEND_DESC {
      RenderTarget: [blend; 8],
      ..Default::default()
    },
    SampleMask: u32::MAX,
    RasterizerState: D3D12_RASTERIZER_DESC {
      FillMode: D3D12_FILL_MODE_SOLID,
      CullMode: D3D12_CULL_MODE_NONE,
      DepthClipEnable: TRUE,
      MultisampleEnable: TRUE,
      ConservativeRaster: D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
      ..Default::default()
    },
    DepthStencilState: D3D12_DEPTH_STENCIL_DESC {
      DepthEnable: FALSE,
      DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ZERO,
      DepthFunc: D3D12_COMPARISON_FUNC_ALWAYS,
      StencilEnable: if samples == 4 { TRUE } else { FALSE },
      StencilReadMask: 255,
      StencilWriteMask: if mode == 2 { 255 } else { 0 },
      FrontFace: face,
      BackFace: face,
    },
    InputLayout: if vs_entry == b"vs_seed\0" || vs_entry == b"vs_tile\0" {
      Default::default()
    } else {
      D3D12_INPUT_LAYOUT_DESC {
        pInputElementDescs: elements.as_ptr(),
        NumElements: 3,
      }
    },
    PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
    NumRenderTargets: 1,
    RTVFormats: formats,
    DSVFormat: if samples == 4 {
      DXGI_FORMAT_D24_UNORM_S8_UINT
    } else {
      DXGI_FORMAT_UNKNOWN
    },
    SampleDesc: DXGI_SAMPLE_DESC {
      Count: samples,
      Quality: 0,
    },
    ..Default::default()
  };
  let result = device.CreateGraphicsPipelineState(&desc);
  ManuallyDrop::drop(&mut desc.pRootSignature);
  result
}
