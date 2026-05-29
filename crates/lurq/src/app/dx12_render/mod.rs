#![allow(unsafe_op_in_unsafe_fn)]

#[cfg(feature = "image")]
use std::collections::HashMap;
use std::{ffi::c_void, mem::ManuallyDrop, ptr};

use raw_window_handle::{DisplayHandle, RawWindowHandle, WindowHandle};
#[cfg(feature = "svg")]
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R32_UINT;
use windows::{
  Win32::{
    Foundation::{CloseHandle, FALSE, HANDLE, HWND, RECT, TRUE, WAIT_OBJECT_0},
    Graphics::{
      Direct3D::{
        D3D_FEATURE_LEVEL_11_0, D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
        Fxc::{D3DCOMPILE_DEBUG, D3DCOMPILE_ENABLE_STRICTNESS, D3DCOMPILE_SKIP_OPTIMIZATION, D3DCompile},
        ID3DBlob,
      },
      Direct3D12::{
        D3D_ROOT_SIGNATURE_VERSION_1, D3D12_BLEND_DESC, D3D12_BLEND_INV_SRC_ALPHA, D3D12_BLEND_ONE, D3D12_BLEND_OP_ADD,
        D3D12_BLEND_SRC_ALPHA, D3D12_COLOR_WRITE_ENABLE_ALL, D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC,
        D3D12_COMMAND_QUEUE_FLAG_NONE, D3D12_COMPARISON_FUNC_ALWAYS, D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
        D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_CULL_MODE_NONE,
        D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING, D3D12_DEPTH_STENCIL_DESC, D3D12_DEPTH_WRITE_MASK_ZERO,
        D3D12_DESCRIPTOR_HEAP_DESC, D3D12_DESCRIPTOR_HEAP_FLAG_NONE, D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
        D3D12_DESCRIPTOR_HEAP_FLAGS, D3D12_DESCRIPTOR_HEAP_TYPE, D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
        D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER, D3D12_DESCRIPTOR_RANGE,
        D3D12_DESCRIPTOR_RANGE_OFFSET_APPEND, D3D12_DESCRIPTOR_RANGE_TYPE_SAMPLER, D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
        D3D12_FENCE_FLAG_NONE, D3D12_FILL_MODE_SOLID, D3D12_FILTER_MIN_MAG_MIP_LINEAR, D3D12_GPU_DESCRIPTOR_HANDLE,
        D3D12_GRAPHICS_PIPELINE_STATE_DESC, D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES, D3D12_HEAP_TYPE_DEFAULT,
        D3D12_HEAP_TYPE_UPLOAD, D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_DISABLED, D3D12_INDEX_BUFFER_VIEW,
        D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA, D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
        D3D12_INPUT_ELEMENT_DESC, D3D12_INPUT_LAYOUT_DESC, D3D12_LOGIC_OP_NOOP, D3D12_MEMORY_POOL_UNKNOWN,
        D3D12_PIPELINE_STATE_FLAG_NONE, D3D12_PLACED_SUBRESOURCE_FOOTPRINT, D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
        D3D12_RANGE, D3D12_RASTERIZER_DESC, D3D12_RENDER_TARGET_BLEND_DESC, D3D12_RENDER_TARGET_VIEW_DESC,
        D3D12_RENDER_TARGET_VIEW_DESC_0, D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0,
        D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, D3D12_RESOURCE_BARRIER_FLAG_NONE,
        D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_DESC, D3D12_RESOURCE_DIMENSION_BUFFER,
        D3D12_RESOURCE_DIMENSION_TEXTURE2D, D3D12_RESOURCE_FLAG_NONE, D3D12_RESOURCE_STATE_COPY_DEST,
        D3D12_RESOURCE_STATE_GENERIC_READ, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, D3D12_RESOURCE_STATE_PRESENT,
        D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_STATES, D3D12_RESOURCE_TRANSITION_BARRIER,
        D3D12_ROOT_DESCRIPTOR, D3D12_ROOT_DESCRIPTOR_TABLE, D3D12_ROOT_PARAMETER, D3D12_ROOT_PARAMETER_0,
        D3D12_ROOT_PARAMETER_TYPE_CBV, D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE, D3D12_ROOT_SIGNATURE_DESC,
        D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT, D3D12_RTV_DIMENSION_TEXTURE2D,
        D3D12_SAMPLER_DESC, D3D12_SHADER_BYTECODE, D3D12_SHADER_RESOURCE_VIEW_DESC, D3D12_SHADER_RESOURCE_VIEW_DESC_0,
        D3D12_SHADER_VISIBILITY_ALL, D3D12_SRV_DIMENSION_TEXTURE2D, D3D12_SUBRESOURCE_FOOTPRINT, D3D12_TEX2D_RTV,
        D3D12_TEX2D_SRV, D3D12_TEXTURE_ADDRESS_MODE_CLAMP, D3D12_TEXTURE_COPY_LOCATION, D3D12_TEXTURE_COPY_LOCATION_0,
        D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT, D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
        D3D12_TEXTURE_LAYOUT_ROW_MAJOR, D3D12_TEXTURE_LAYOUT_UNKNOWN, D3D12_VERTEX_BUFFER_VIEW, D3D12_VIEWPORT,
        D3D12CreateDevice, D3D12SerializeRootSignature, ID3D12CommandAllocator, ID3D12CommandList, ID3D12CommandQueue,
        ID3D12DescriptorHeap, ID3D12Device, ID3D12Fence, ID3D12GraphicsCommandList, ID3D12PipelineState,
        ID3D12Resource, ID3D12RootSignature,
      },
      Dxgi::{
        Common::{
          DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_R8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
          DXGI_FORMAT_R16_UINT, DXGI_FORMAT_R32_FLOAT, DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R32G32B32A32_FLOAT,
          DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
        },
        CreateDXGIFactory2, DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_CREATE_FACTORY_DEBUG, DXGI_CREATE_FACTORY_FLAGS,
        DXGI_MWA_NO_ALT_ENTER, DXGI_SCALING_NONE, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_DISCARD,
        DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIAdapter1, IDXGIFactory4, IDXGIOutput, IDXGISwapChain3,
      },
    },
    System::Threading::{CreateEventW, INFINITE, WaitForSingleObject},
  },
  core::{Error, Interface, PCSTR, Result},
};

#[cfg(feature = "image")]
use crate::render::gpu::ImageInstance;
#[cfg(feature = "svg")]
use crate::render::gpu::SvgVertexGpu;
use crate::{
  app::{
    profiler::{ProfileScope, RenderProfile},
    render_engine::RenderEngine,
  },
  layout::{
    quad::ClipRect,
    render_list::{GlyphCmd, RectCmd, RenderList},
  },
  render::gpu::{Globals, GlyphInstance, QuadInstance, QuadVertex},
};

const FRAME_COUNT: usize = 2;
const SRV_DESCRIPTOR_COUNT: u32 = 1024;
const GLYPH_ATLAS_SRV_INDEX: usize = 0;
const FRAME_UPLOAD_ARENA_BYTES: usize = 16 * 1024 * 1024;
const SWAPCHAIN_FORMAT: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT = DXGI_FORMAT_R8G8B8A8_UNORM;
const RENDER_TARGET_FORMAT: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT = DXGI_FORMAT_R8G8B8A8_UNORM_SRGB;

pub struct Dx12RenderEngine {
  state: Option<Dx12State>,
  width: u32,
  height: u32,
  last_profile: RenderProfile,
  profiling_enabled: bool,
}

impl Default for Dx12RenderEngine {
  fn default() -> Self {
    Self::new()
  }
}

impl Dx12RenderEngine {
  pub fn new() -> Self {
    Self {
      state: None,
      width: 800,
      height: 600,
      last_profile: RenderProfile::default(),
      profiling_enabled: false,
    }
  }

  fn ensure_initialized(&mut self, window: WindowHandle<'_>) -> Result<()> {
    if self.state.is_some() {
      return Ok(());
    }

    let hwnd = hwnd_from_window(window)?;
    let state = unsafe { Dx12State::new(hwnd, self.width.max(1), self.height.max(1))? };
    self.state = Some(state);
    Ok(())
  }
}

impl RenderEngine for Dx12RenderEngine {
  fn resize(&mut self, width: u32, height: u32) {
    self.width = width.max(1);
    self.height = height.max(1);
  }

  fn render(&mut self, list: &RenderList, window: WindowHandle<'_>, _display: DisplayHandle<'_>) {
    let profiling_enabled = self.profiling_enabled;
    let total_start = ProfileScope::maybe_start(profiling_enabled);
    let init_start = ProfileScope::maybe_start(profiling_enabled);
    self
      .ensure_initialized(window)
      .expect("failed to initialize native dx12 renderer");
    let init_dur = ProfileScope::elapsed_or_default(&init_start);

    let state = self.state.as_mut().unwrap();
    if state.width != self.width || state.height != self.height {
      unsafe {
        state
          .resize(self.width, self.height)
          .expect("failed to resize native dx12 swapchain");
      }
    }
    let render_profile = unsafe {
      state
        .render(list, profiling_enabled)
        .expect("failed to render native dx12 frame")
    };

    if profiling_enabled {
      self.last_profile = RenderProfile {
        init: init_dur,
        total: ProfileScope::elapsed_or_default(&total_start),
        ..render_profile
      };
    }
  }

  fn set_profiling_enabled(&mut self, enabled: bool) {
    self.profiling_enabled = enabled;
  }

  fn last_profile(&self) -> Option<RenderProfile> {
    Some(self.last_profile)
  }
}

struct Dx12State {
  device: ID3D12Device,
  command_queue: ID3D12CommandQueue,
  swapchain: IDXGISwapChain3,
  rtv_heap: CpuDescriptorHeap,
  srv_heap: CpuDescriptorHeap,
  sampler_heap: CpuDescriptorHeap,
  render_targets: [Option<ID3D12Resource>; FRAME_COUNT],
  quad_buffers: StaticQuadBuffers,
  rect_pipeline: RectPipeline,
  glyph_pipeline: GlyphPipeline,
  #[cfg(feature = "image")]
  image_pipeline: ImagePipeline,
  #[cfg(feature = "svg")]
  svg_pipeline: SvgPipeline,
  glyph_atlas: Option<GlyphAtlasTexture>,
  #[cfg(feature = "image")]
  image_textures: HashMap<u64, CachedImageTexture>,
  #[cfg(feature = "image")]
  next_srv_index: usize,
  frame_arenas: [UploadArena; FRAME_COUNT],
  frame_uploads: [Vec<UploadBuffer>; FRAME_COUNT],
  command_allocators: [ID3D12CommandAllocator; FRAME_COUNT],
  command_list: ID3D12GraphicsCommandList,
  fence: ID3D12Fence,
  fence_event: HANDLE,
  fence_values: [u64; FRAME_COUNT],
  next_fence_value: u64,
  frame_index: usize,
  width: u32,
  height: u32,
}

struct StaticQuadBuffers {
  _vertex_buffer: UploadBuffer,
  _index_buffer: UploadBuffer,
  vertex_view: D3D12_VERTEX_BUFFER_VIEW,
  index_view: D3D12_INDEX_BUFFER_VIEW,
}

struct RectPipeline {
  root_signature: ID3D12RootSignature,
  pipeline_state: ID3D12PipelineState,
}

struct GlyphPipeline {
  root_signature: ID3D12RootSignature,
  pipeline_state: ID3D12PipelineState,
}

#[cfg(feature = "image")]
struct ImagePipeline {
  root_signature: ID3D12RootSignature,
  pipeline_state: ID3D12PipelineState,
}

#[cfg(feature = "svg")]
struct SvgPipeline {
  root_signature: ID3D12RootSignature,
  pipeline_state: ID3D12PipelineState,
}

struct GlyphAtlasTexture {
  texture: ID3D12Resource,
  width: u32,
  height: u32,
  version: u64,
  state: D3D12_RESOURCE_STATES,
}

#[cfg(feature = "image")]
struct CachedImageTexture {
  _texture: ID3D12Resource,
  descriptor_index: usize,
  frame_index: usize,
}

struct CpuDescriptorHeap {
  heap: ID3D12DescriptorHeap,
  descriptor_size: u32,
}

impl CpuDescriptorHeap {
  unsafe fn new(
    device: &ID3D12Device,
    heap_type: D3D12_DESCRIPTOR_HEAP_TYPE,
    num_descriptors: u32,
    flags: D3D12_DESCRIPTOR_HEAP_FLAGS,
  ) -> Result<Self> {
    let desc = D3D12_DESCRIPTOR_HEAP_DESC {
      Type: heap_type,
      NumDescriptors: num_descriptors,
      Flags: flags,
      NodeMask: 0,
    };
    let heap = device.CreateDescriptorHeap(&desc)?;
    let descriptor_size = device.GetDescriptorHandleIncrementSize(heap_type);
    Ok(Self { heap, descriptor_size })
  }

  unsafe fn cpu_handle(&self, index: usize) -> D3D12_CPU_DESCRIPTOR_HANDLE {
    offset_cpu_handle(
      self.heap.GetCPUDescriptorHandleForHeapStart(),
      self.descriptor_size,
      index,
    )
  }

  unsafe fn gpu_handle(&self, index: usize) -> D3D12_GPU_DESCRIPTOR_HANDLE {
    offset_gpu_handle(
      self.heap.GetGPUDescriptorHandleForHeapStart(),
      self.descriptor_size,
      index,
    )
  }
}

impl StaticQuadBuffers {
  unsafe fn new(device: &ID3D12Device) -> Result<Self> {
    let vertex_bytes = bytemuck::cast_slice(&QuadVertex::CORNERS);
    let index_bytes = bytemuck::cast_slice(&QuadVertex::INDICES);
    let vertex_buffer = UploadBuffer::from_bytes(device, vertex_bytes)?;
    let index_buffer = UploadBuffer::from_bytes(device, index_bytes)?;
    let vertex_view = D3D12_VERTEX_BUFFER_VIEW {
      BufferLocation: vertex_buffer.gpu_address,
      SizeInBytes: vertex_buffer.size_in_bytes,
      StrideInBytes: std::mem::size_of::<QuadVertex>() as u32,
    };
    let index_view = D3D12_INDEX_BUFFER_VIEW {
      BufferLocation: index_buffer.gpu_address,
      SizeInBytes: index_buffer.size_in_bytes,
      Format: DXGI_FORMAT_R16_UINT,
    };
    Ok(Self {
      _vertex_buffer: vertex_buffer,
      _index_buffer: index_buffer,
      vertex_view,
      index_view,
    })
  }
}

struct UploadBuffer {
  _resource: ID3D12Resource,
  gpu_address: u64,
  size_in_bytes: u32,
}

#[derive(Clone, Copy)]
struct UploadSlice {
  gpu_address: u64,
  size_in_bytes: u32,
}

struct UploadArena {
  _resource: ID3D12Resource,
  gpu_address: u64,
  mapped: *mut u8,
  capacity: usize,
  offset: usize,
}

impl UploadBuffer {
  unsafe fn from_bytes(device: &ID3D12Device, data: &[u8]) -> Result<Self> {
    Self::from_bytes_padded(device, data, data.len().max(1))
  }

  unsafe fn from_bytes_padded(device: &ID3D12Device, data: &[u8], padded_size: usize) -> Result<Self> {
    let padded_size = padded_size.max(data.len()).max(1);
    let heap_properties = D3D12_HEAP_PROPERTIES {
      Type: D3D12_HEAP_TYPE_UPLOAD,
      CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
      MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
      CreationNodeMask: 1,
      VisibleNodeMask: 1,
    };
    let desc = buffer_resource_desc(padded_size as u64);
    let mut resource = None;
    device.CreateCommittedResource(
      &heap_properties,
      D3D12_HEAP_FLAG_NONE,
      &desc,
      D3D12_RESOURCE_STATE_GENERIC_READ,
      None,
      &mut resource,
    )?;
    let resource: ID3D12Resource = resource.ok_or_else(Error::from_win32)?;

    let read_range = D3D12_RANGE { Begin: 0, End: 0 };
    let mut mapped: *mut c_void = ptr::null_mut();
    resource.Map(0, Some(&read_range), Some(&mut mapped))?;
    ptr::write_bytes(mapped.cast::<u8>(), 0, padded_size);
    if !data.is_empty() {
      ptr::copy_nonoverlapping(data.as_ptr(), mapped.cast::<u8>(), data.len());
    }
    let written_range = D3D12_RANGE {
      Begin: 0,
      End: padded_size,
    };
    resource.Unmap(0, Some(&written_range));

    Ok(Self {
      gpu_address: resource.GetGPUVirtualAddress(),
      size_in_bytes: padded_size as u32,
      _resource: resource,
    })
  }
}

impl UploadSlice {
  fn vertex_view<T>(&self) -> D3D12_VERTEX_BUFFER_VIEW {
    D3D12_VERTEX_BUFFER_VIEW {
      BufferLocation: self.gpu_address,
      SizeInBytes: self.size_in_bytes,
      StrideInBytes: std::mem::size_of::<T>() as u32,
    }
  }
}

impl UploadArena {
  unsafe fn new(device: &ID3D12Device, capacity: usize) -> Result<Self> {
    let heap_properties = D3D12_HEAP_PROPERTIES {
      Type: D3D12_HEAP_TYPE_UPLOAD,
      CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
      MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
      CreationNodeMask: 1,
      VisibleNodeMask: 1,
    };
    let desc = buffer_resource_desc(capacity.max(1) as u64);
    let mut resource = None;
    device.CreateCommittedResource(
      &heap_properties,
      D3D12_HEAP_FLAG_NONE,
      &desc,
      D3D12_RESOURCE_STATE_GENERIC_READ,
      None,
      &mut resource,
    )?;
    let resource: ID3D12Resource = resource.ok_or_else(Error::from_win32)?;
    let read_range = D3D12_RANGE { Begin: 0, End: 0 };
    let mut mapped: *mut c_void = ptr::null_mut();
    resource.Map(0, Some(&read_range), Some(&mut mapped))?;
    Ok(Self {
      gpu_address: resource.GetGPUVirtualAddress(),
      mapped: mapped.cast(),
      capacity: capacity.max(1),
      offset: 0,
      _resource: resource,
    })
  }

  fn reset(&mut self) {
    self.offset = 0;
  }

  unsafe fn upload_bytes(&mut self, data: &[u8], alignment: usize) -> Option<UploadSlice> {
    let alignment = alignment.max(1);
    let start = align_up(self.offset, alignment);
    let padded_size = align_up(data.len().max(1), alignment);
    let end = start.checked_add(padded_size)?;
    if end > self.capacity {
      return None;
    }

    ptr::copy_nonoverlapping(data.as_ptr(), self.mapped.add(start), data.len());
    self.offset = end;
    Some(UploadSlice {
      gpu_address: self.gpu_address + start as u64,
      size_in_bytes: padded_size as u32,
    })
  }
}

impl Drop for UploadArena {
  fn drop(&mut self) {
    unsafe {
      let written_range = D3D12_RANGE {
        Begin: 0,
        End: self.offset,
      };
      self._resource.Unmap(0, Some(&written_range));
    }
  }
}

impl RectPipeline {
  unsafe fn new(device: &ID3D12Device, format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT) -> Result<Self> {
    let root_signature = create_rect_root_signature(device)?;
    let vs = compile_shader(include_bytes!("shaders/quad.hlsl"), b"vs_main\0", b"vs_5_0\0")?;
    let ps = compile_shader(include_bytes!("shaders/quad.hlsl"), b"ps_main\0", b"ps_5_0\0")?;
    let input_elements = rect_input_elements();
    let mut rtv_formats = [DXGI_FORMAT_UNKNOWN; 8];
    rtv_formats[0] = format;

    let blend = D3D12_RENDER_TARGET_BLEND_DESC {
      BlendEnable: TRUE,
      LogicOpEnable: FALSE,
      SrcBlend: D3D12_BLEND_SRC_ALPHA,
      DestBlend: D3D12_BLEND_INV_SRC_ALPHA,
      BlendOp: D3D12_BLEND_OP_ADD,
      SrcBlendAlpha: D3D12_BLEND_ONE,
      DestBlendAlpha: D3D12_BLEND_INV_SRC_ALPHA,
      BlendOpAlpha: D3D12_BLEND_OP_ADD,
      LogicOp: D3D12_LOGIC_OP_NOOP,
      RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8,
    };

    let mut desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
      pRootSignature: ManuallyDrop::new(Some(root_signature.clone())),
      VS: shader_bytecode(&vs),
      PS: shader_bytecode(&ps),
      DS: D3D12_SHADER_BYTECODE::default(),
      HS: D3D12_SHADER_BYTECODE::default(),
      GS: D3D12_SHADER_BYTECODE::default(),
      StreamOutput: Default::default(),
      BlendState: D3D12_BLEND_DESC {
        AlphaToCoverageEnable: FALSE,
        IndependentBlendEnable: FALSE,
        RenderTarget: [blend; 8],
      },
      SampleMask: u32::MAX,
      RasterizerState: D3D12_RASTERIZER_DESC {
        FillMode: D3D12_FILL_MODE_SOLID,
        CullMode: D3D12_CULL_MODE_NONE,
        FrontCounterClockwise: FALSE,
        DepthBias: 0,
        DepthBiasClamp: 0.0,
        SlopeScaledDepthBias: 0.0,
        DepthClipEnable: TRUE,
        MultisampleEnable: FALSE,
        AntialiasedLineEnable: FALSE,
        ForcedSampleCount: 0,
        ConservativeRaster: D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
      },
      DepthStencilState: D3D12_DEPTH_STENCIL_DESC {
        DepthEnable: FALSE,
        DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ZERO,
        DepthFunc: D3D12_COMPARISON_FUNC_ALWAYS,
        StencilEnable: FALSE,
        StencilReadMask: 0,
        StencilWriteMask: 0,
        FrontFace: Default::default(),
        BackFace: Default::default(),
      },
      InputLayout: D3D12_INPUT_LAYOUT_DESC {
        pInputElementDescs: input_elements.as_ptr(),
        NumElements: input_elements.len() as u32,
      },
      IBStripCutValue: D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_DISABLED,
      PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
      NumRenderTargets: 1,
      RTVFormats: rtv_formats,
      DSVFormat: DXGI_FORMAT_UNKNOWN,
      SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
      NodeMask: 0,
      CachedPSO: Default::default(),
      Flags: D3D12_PIPELINE_STATE_FLAG_NONE,
    };

    let pipeline_state = device.CreateGraphicsPipelineState(&desc)?;
    ManuallyDrop::drop(&mut desc.pRootSignature);

    Ok(Self {
      root_signature,
      pipeline_state,
    })
  }
}

impl GlyphPipeline {
  unsafe fn new(device: &ID3D12Device, format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT) -> Result<Self> {
    let root_signature = create_glyph_root_signature(device)?;
    let vs = compile_shader(include_bytes!("shaders/glyph.hlsl"), b"vs_main\0", b"vs_5_0\0")?;
    let ps = compile_shader(include_bytes!("shaders/glyph.hlsl"), b"ps_main\0", b"ps_5_0\0")?;
    let input_elements = glyph_input_elements();
    let mut rtv_formats = [DXGI_FORMAT_UNKNOWN; 8];
    rtv_formats[0] = format;

    let blend = D3D12_RENDER_TARGET_BLEND_DESC {
      BlendEnable: TRUE,
      LogicOpEnable: FALSE,
      SrcBlend: D3D12_BLEND_SRC_ALPHA,
      DestBlend: D3D12_BLEND_INV_SRC_ALPHA,
      BlendOp: D3D12_BLEND_OP_ADD,
      SrcBlendAlpha: D3D12_BLEND_ONE,
      DestBlendAlpha: D3D12_BLEND_INV_SRC_ALPHA,
      BlendOpAlpha: D3D12_BLEND_OP_ADD,
      LogicOp: D3D12_LOGIC_OP_NOOP,
      RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8,
    };

    let mut desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
      pRootSignature: ManuallyDrop::new(Some(root_signature.clone())),
      VS: shader_bytecode(&vs),
      PS: shader_bytecode(&ps),
      DS: D3D12_SHADER_BYTECODE::default(),
      HS: D3D12_SHADER_BYTECODE::default(),
      GS: D3D12_SHADER_BYTECODE::default(),
      StreamOutput: Default::default(),
      BlendState: D3D12_BLEND_DESC {
        AlphaToCoverageEnable: FALSE,
        IndependentBlendEnable: FALSE,
        RenderTarget: [blend; 8],
      },
      SampleMask: u32::MAX,
      RasterizerState: D3D12_RASTERIZER_DESC {
        FillMode: D3D12_FILL_MODE_SOLID,
        CullMode: D3D12_CULL_MODE_NONE,
        FrontCounterClockwise: FALSE,
        DepthBias: 0,
        DepthBiasClamp: 0.0,
        SlopeScaledDepthBias: 0.0,
        DepthClipEnable: TRUE,
        MultisampleEnable: FALSE,
        AntialiasedLineEnable: FALSE,
        ForcedSampleCount: 0,
        ConservativeRaster: D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
      },
      DepthStencilState: D3D12_DEPTH_STENCIL_DESC {
        DepthEnable: FALSE,
        DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ZERO,
        DepthFunc: D3D12_COMPARISON_FUNC_ALWAYS,
        StencilEnable: FALSE,
        StencilReadMask: 0,
        StencilWriteMask: 0,
        FrontFace: Default::default(),
        BackFace: Default::default(),
      },
      InputLayout: D3D12_INPUT_LAYOUT_DESC {
        pInputElementDescs: input_elements.as_ptr(),
        NumElements: input_elements.len() as u32,
      },
      IBStripCutValue: D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_DISABLED,
      PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
      NumRenderTargets: 1,
      RTVFormats: rtv_formats,
      DSVFormat: DXGI_FORMAT_UNKNOWN,
      SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
      NodeMask: 0,
      CachedPSO: Default::default(),
      Flags: D3D12_PIPELINE_STATE_FLAG_NONE,
    };

    let pipeline_state = device.CreateGraphicsPipelineState(&desc)?;
    ManuallyDrop::drop(&mut desc.pRootSignature);

    Ok(Self {
      root_signature,
      pipeline_state,
    })
  }
}

#[cfg(feature = "image")]
impl ImagePipeline {
  unsafe fn new(device: &ID3D12Device, format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT) -> Result<Self> {
    let root_signature = create_glyph_root_signature(device)?;
    let vs = compile_shader(include_bytes!("shaders/image.hlsl"), b"vs_main\0", b"vs_5_0\0")?;
    let ps = compile_shader(include_bytes!("shaders/image.hlsl"), b"ps_main\0", b"ps_5_0\0")?;
    let input_elements = image_input_elements();
    let mut rtv_formats = [DXGI_FORMAT_UNKNOWN; 8];
    rtv_formats[0] = format;

    let blend = D3D12_RENDER_TARGET_BLEND_DESC {
      BlendEnable: TRUE,
      LogicOpEnable: FALSE,
      SrcBlend: D3D12_BLEND_SRC_ALPHA,
      DestBlend: D3D12_BLEND_INV_SRC_ALPHA,
      BlendOp: D3D12_BLEND_OP_ADD,
      SrcBlendAlpha: D3D12_BLEND_ONE,
      DestBlendAlpha: D3D12_BLEND_INV_SRC_ALPHA,
      BlendOpAlpha: D3D12_BLEND_OP_ADD,
      LogicOp: D3D12_LOGIC_OP_NOOP,
      RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8,
    };

    let mut desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
      pRootSignature: ManuallyDrop::new(Some(root_signature.clone())),
      VS: shader_bytecode(&vs),
      PS: shader_bytecode(&ps),
      DS: D3D12_SHADER_BYTECODE::default(),
      HS: D3D12_SHADER_BYTECODE::default(),
      GS: D3D12_SHADER_BYTECODE::default(),
      StreamOutput: Default::default(),
      BlendState: D3D12_BLEND_DESC {
        AlphaToCoverageEnable: FALSE,
        IndependentBlendEnable: FALSE,
        RenderTarget: [blend; 8],
      },
      SampleMask: u32::MAX,
      RasterizerState: D3D12_RASTERIZER_DESC {
        FillMode: D3D12_FILL_MODE_SOLID,
        CullMode: D3D12_CULL_MODE_NONE,
        FrontCounterClockwise: FALSE,
        DepthBias: 0,
        DepthBiasClamp: 0.0,
        SlopeScaledDepthBias: 0.0,
        DepthClipEnable: TRUE,
        MultisampleEnable: FALSE,
        AntialiasedLineEnable: FALSE,
        ForcedSampleCount: 0,
        ConservativeRaster: D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
      },
      DepthStencilState: D3D12_DEPTH_STENCIL_DESC {
        DepthEnable: FALSE,
        DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ZERO,
        DepthFunc: D3D12_COMPARISON_FUNC_ALWAYS,
        StencilEnable: FALSE,
        StencilReadMask: 0,
        StencilWriteMask: 0,
        FrontFace: Default::default(),
        BackFace: Default::default(),
      },
      InputLayout: D3D12_INPUT_LAYOUT_DESC {
        pInputElementDescs: input_elements.as_ptr(),
        NumElements: input_elements.len() as u32,
      },
      IBStripCutValue: D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_DISABLED,
      PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
      NumRenderTargets: 1,
      RTVFormats: rtv_formats,
      DSVFormat: DXGI_FORMAT_UNKNOWN,
      SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
      NodeMask: 0,
      CachedPSO: Default::default(),
      Flags: D3D12_PIPELINE_STATE_FLAG_NONE,
    };

    let pipeline_state = device.CreateGraphicsPipelineState(&desc)?;
    ManuallyDrop::drop(&mut desc.pRootSignature);

    Ok(Self {
      root_signature,
      pipeline_state,
    })
  }
}

#[cfg(feature = "svg")]
impl SvgPipeline {
  unsafe fn new(device: &ID3D12Device, format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT) -> Result<Self> {
    let root_signature = create_rect_root_signature(device)?;
    let vs = compile_shader(include_bytes!("shaders/svg.hlsl"), b"vs_main\0", b"vs_5_0\0")?;
    let ps = compile_shader(include_bytes!("shaders/svg.hlsl"), b"ps_main\0", b"ps_5_0\0")?;
    let input_elements = svg_input_elements();
    let mut rtv_formats = [DXGI_FORMAT_UNKNOWN; 8];
    rtv_formats[0] = format;

    let blend = D3D12_RENDER_TARGET_BLEND_DESC {
      BlendEnable: TRUE,
      LogicOpEnable: FALSE,
      SrcBlend: D3D12_BLEND_SRC_ALPHA,
      DestBlend: D3D12_BLEND_INV_SRC_ALPHA,
      BlendOp: D3D12_BLEND_OP_ADD,
      SrcBlendAlpha: D3D12_BLEND_ONE,
      DestBlendAlpha: D3D12_BLEND_INV_SRC_ALPHA,
      BlendOpAlpha: D3D12_BLEND_OP_ADD,
      LogicOp: D3D12_LOGIC_OP_NOOP,
      RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8,
    };

    let mut desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
      pRootSignature: ManuallyDrop::new(Some(root_signature.clone())),
      VS: shader_bytecode(&vs),
      PS: shader_bytecode(&ps),
      DS: D3D12_SHADER_BYTECODE::default(),
      HS: D3D12_SHADER_BYTECODE::default(),
      GS: D3D12_SHADER_BYTECODE::default(),
      StreamOutput: Default::default(),
      BlendState: D3D12_BLEND_DESC {
        AlphaToCoverageEnable: FALSE,
        IndependentBlendEnable: FALSE,
        RenderTarget: [blend; 8],
      },
      SampleMask: u32::MAX,
      RasterizerState: D3D12_RASTERIZER_DESC {
        FillMode: D3D12_FILL_MODE_SOLID,
        CullMode: D3D12_CULL_MODE_NONE,
        FrontCounterClockwise: FALSE,
        DepthBias: 0,
        DepthBiasClamp: 0.0,
        SlopeScaledDepthBias: 0.0,
        DepthClipEnable: TRUE,
        MultisampleEnable: FALSE,
        AntialiasedLineEnable: FALSE,
        ForcedSampleCount: 0,
        ConservativeRaster: D3D12_CONSERVATIVE_RASTERIZATION_MODE_OFF,
      },
      DepthStencilState: D3D12_DEPTH_STENCIL_DESC {
        DepthEnable: FALSE,
        DepthWriteMask: D3D12_DEPTH_WRITE_MASK_ZERO,
        DepthFunc: D3D12_COMPARISON_FUNC_ALWAYS,
        StencilEnable: FALSE,
        StencilReadMask: 0,
        StencilWriteMask: 0,
        FrontFace: Default::default(),
        BackFace: Default::default(),
      },
      InputLayout: D3D12_INPUT_LAYOUT_DESC {
        pInputElementDescs: input_elements.as_ptr(),
        NumElements: input_elements.len() as u32,
      },
      IBStripCutValue: D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_DISABLED,
      PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
      NumRenderTargets: 1,
      RTVFormats: rtv_formats,
      DSVFormat: DXGI_FORMAT_UNKNOWN,
      SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
      NodeMask: 0,
      CachedPSO: Default::default(),
      Flags: D3D12_PIPELINE_STATE_FLAG_NONE,
    };

    let pipeline_state = device.CreateGraphicsPipelineState(&desc)?;
    ManuallyDrop::drop(&mut desc.pRootSignature);

    Ok(Self {
      root_signature,
      pipeline_state,
    })
  }
}

fn render_target_view_desc() -> D3D12_RENDER_TARGET_VIEW_DESC {
  D3D12_RENDER_TARGET_VIEW_DESC {
    Format: RENDER_TARGET_FORMAT,
    ViewDimension: D3D12_RTV_DIMENSION_TEXTURE2D,
    Anonymous: D3D12_RENDER_TARGET_VIEW_DESC_0 {
      Texture2D: D3D12_TEX2D_RTV {
        MipSlice: 0,
        PlaneSlice: 0,
      },
    },
  }
}

fn buffer_resource_desc(size: u64) -> D3D12_RESOURCE_DESC {
  D3D12_RESOURCE_DESC {
    Dimension: D3D12_RESOURCE_DIMENSION_BUFFER,
    Alignment: 0,
    Width: size,
    Height: 1,
    DepthOrArraySize: 1,
    MipLevels: 1,
    Format: DXGI_FORMAT_UNKNOWN,
    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
    Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR,
    Flags: D3D12_RESOURCE_FLAG_NONE,
  }
}

unsafe fn create_rect_root_signature(device: &ID3D12Device) -> Result<ID3D12RootSignature> {
  let root_parameter = D3D12_ROOT_PARAMETER {
    ParameterType: D3D12_ROOT_PARAMETER_TYPE_CBV,
    Anonymous: D3D12_ROOT_PARAMETER_0 {
      Descriptor: D3D12_ROOT_DESCRIPTOR {
        ShaderRegister: 0,
        RegisterSpace: 0,
      },
    },
    ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
  };
  let desc = D3D12_ROOT_SIGNATURE_DESC {
    NumParameters: 1,
    pParameters: &root_parameter,
    NumStaticSamplers: 0,
    pStaticSamplers: ptr::null(),
    Flags: D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
  };

  let mut blob = None;
  let mut errors = None;
  D3D12SerializeRootSignature(&desc, D3D_ROOT_SIGNATURE_VERSION_1, &mut blob, Some(&mut errors)).map_err(|err| {
    Error::new(
      err.code(),
      format!(
        "failed to serialize dx12 rect root signature{}",
        blob_message(errors.as_ref())
      ),
    )
  })?;
  let blob = blob.ok_or_else(Error::from_win32)?;
  device.CreateRootSignature(0, blob_bytes(&blob))
}

unsafe fn create_glyph_root_signature(device: &ID3D12Device) -> Result<ID3D12RootSignature> {
  let srv_range = D3D12_DESCRIPTOR_RANGE {
    RangeType: D3D12_DESCRIPTOR_RANGE_TYPE_SRV,
    NumDescriptors: 1,
    BaseShaderRegister: 0,
    RegisterSpace: 0,
    OffsetInDescriptorsFromTableStart: D3D12_DESCRIPTOR_RANGE_OFFSET_APPEND,
  };
  let sampler_range = D3D12_DESCRIPTOR_RANGE {
    RangeType: D3D12_DESCRIPTOR_RANGE_TYPE_SAMPLER,
    NumDescriptors: 1,
    BaseShaderRegister: 0,
    RegisterSpace: 0,
    OffsetInDescriptorsFromTableStart: D3D12_DESCRIPTOR_RANGE_OFFSET_APPEND,
  };
  let root_parameters = [
    D3D12_ROOT_PARAMETER {
      ParameterType: D3D12_ROOT_PARAMETER_TYPE_CBV,
      Anonymous: D3D12_ROOT_PARAMETER_0 {
        Descriptor: D3D12_ROOT_DESCRIPTOR {
          ShaderRegister: 0,
          RegisterSpace: 0,
        },
      },
      ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
    },
    D3D12_ROOT_PARAMETER {
      ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
      Anonymous: D3D12_ROOT_PARAMETER_0 {
        DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE {
          NumDescriptorRanges: 1,
          pDescriptorRanges: &srv_range,
        },
      },
      ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
    },
    D3D12_ROOT_PARAMETER {
      ParameterType: D3D12_ROOT_PARAMETER_TYPE_DESCRIPTOR_TABLE,
      Anonymous: D3D12_ROOT_PARAMETER_0 {
        DescriptorTable: D3D12_ROOT_DESCRIPTOR_TABLE {
          NumDescriptorRanges: 1,
          pDescriptorRanges: &sampler_range,
        },
      },
      ShaderVisibility: D3D12_SHADER_VISIBILITY_ALL,
    },
  ];
  let desc = D3D12_ROOT_SIGNATURE_DESC {
    NumParameters: root_parameters.len() as u32,
    pParameters: root_parameters.as_ptr(),
    NumStaticSamplers: 0,
    pStaticSamplers: ptr::null(),
    Flags: D3D12_ROOT_SIGNATURE_FLAG_ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT,
  };

  let mut blob = None;
  let mut errors = None;
  D3D12SerializeRootSignature(&desc, D3D_ROOT_SIGNATURE_VERSION_1, &mut blob, Some(&mut errors)).map_err(|err| {
    Error::new(
      err.code(),
      format!(
        "failed to serialize dx12 glyph root signature{}",
        blob_message(errors.as_ref())
      ),
    )
  })?;
  let blob = blob.ok_or_else(Error::from_win32)?;
  device.CreateRootSignature(0, blob_bytes(&blob))
}

unsafe fn compile_shader(source: &[u8], entry: &'static [u8], target: &'static [u8]) -> Result<ID3DBlob> {
  let flags = if cfg!(debug_assertions) {
    D3DCOMPILE_ENABLE_STRICTNESS | D3DCOMPILE_DEBUG | D3DCOMPILE_SKIP_OPTIMIZATION
  } else {
    D3DCOMPILE_ENABLE_STRICTNESS
  };
  let mut blob = None;
  let mut errors = None;
  D3DCompile(
    source.as_ptr().cast(),
    source.len(),
    PCSTR(b"quad.hlsl\0".as_ptr()),
    None,
    None,
    PCSTR(entry.as_ptr()),
    PCSTR(target.as_ptr()),
    flags,
    0,
    &mut blob,
    Some(&mut errors),
  )
  .map_err(|err| {
    Error::new(
      err.code(),
      format!("failed to compile dx12 rect shader{}", blob_message(errors.as_ref())),
    )
  })?;
  blob.ok_or_else(Error::from_win32)
}

fn blob_message(blob: Option<&ID3DBlob>) -> String {
  let Some(blob) = blob else {
    return String::new();
  };
  unsafe {
    let bytes = std::slice::from_raw_parts(blob.GetBufferPointer().cast::<u8>(), blob.GetBufferSize());
    format!(": {}", String::from_utf8_lossy(bytes).trim_end_matches('\0'))
  }
}

unsafe fn blob_bytes(blob: &ID3DBlob) -> &[u8] {
  std::slice::from_raw_parts(blob.GetBufferPointer().cast::<u8>(), blob.GetBufferSize())
}

unsafe fn shader_bytecode(blob: &ID3DBlob) -> D3D12_SHADER_BYTECODE {
  D3D12_SHADER_BYTECODE {
    pShaderBytecode: blob.GetBufferPointer(),
    BytecodeLength: blob.GetBufferSize(),
  }
}

fn rect_input_elements() -> [D3D12_INPUT_ELEMENT_DESC; 12] {
  [
    input_element(
      0,
      DXGI_FORMAT_R32G32_FLOAT,
      0,
      0,
      D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
      0,
    ),
    input_element(
      1,
      DXGI_FORMAT_R32G32_FLOAT,
      1,
      0,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      2,
      DXGI_FORMAT_R32G32_FLOAT,
      1,
      8,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      3,
      DXGI_FORMAT_R32G32B32A32_FLOAT,
      1,
      16,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      4,
      DXGI_FORMAT_R32G32B32A32_FLOAT,
      1,
      32,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      5,
      DXGI_FORMAT_R32G32B32A32_FLOAT,
      1,
      48,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      6,
      DXGI_FORMAT_R32G32B32A32_FLOAT,
      1,
      64,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      7,
      DXGI_FORMAT_R32G32B32A32_FLOAT,
      1,
      80,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      8,
      DXGI_FORMAT_R32G32B32A32_FLOAT,
      1,
      96,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      9,
      DXGI_FORMAT_R32G32_FLOAT,
      1,
      112,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      10,
      DXGI_FORMAT_R32_FLOAT,
      1,
      120,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      11,
      DXGI_FORMAT_R32_FLOAT,
      1,
      124,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
  ]
}

fn glyph_input_elements() -> [D3D12_INPUT_ELEMENT_DESC; 8] {
  [
    input_element(
      0,
      DXGI_FORMAT_R32G32_FLOAT,
      0,
      0,
      D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
      0,
    ),
    input_element(
      1,
      DXGI_FORMAT_R32G32_FLOAT,
      1,
      0,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      2,
      DXGI_FORMAT_R32G32_FLOAT,
      1,
      8,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      3,
      DXGI_FORMAT_R32G32B32A32_FLOAT,
      1,
      16,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      4,
      DXGI_FORMAT_R32G32_FLOAT,
      1,
      32,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      5,
      DXGI_FORMAT_R32G32_FLOAT,
      1,
      40,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      6,
      DXGI_FORMAT_R32G32B32A32_FLOAT,
      1,
      48,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      7,
      DXGI_FORMAT_R32G32_FLOAT,
      1,
      64,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
  ]
}

#[cfg(feature = "image")]
fn image_input_elements() -> [D3D12_INPUT_ELEMENT_DESC; 9] {
  [
    input_element(
      0,
      DXGI_FORMAT_R32G32_FLOAT,
      0,
      0,
      D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
      0,
    ),
    input_element(
      1,
      DXGI_FORMAT_R32G32_FLOAT,
      1,
      0,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      2,
      DXGI_FORMAT_R32G32_FLOAT,
      1,
      8,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      3,
      DXGI_FORMAT_R32G32B32A32_FLOAT,
      1,
      16,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      4,
      DXGI_FORMAT_R32G32B32A32_FLOAT,
      1,
      32,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      5,
      DXGI_FORMAT_R32G32_FLOAT,
      1,
      48,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      6,
      DXGI_FORMAT_R32G32_FLOAT,
      1,
      56,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      7,
      DXGI_FORMAT_R32G32_FLOAT,
      1,
      64,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
    input_element(
      8,
      DXGI_FORMAT_R32G32B32A32_FLOAT,
      1,
      72,
      D3D12_INPUT_CLASSIFICATION_PER_INSTANCE_DATA,
      1,
    ),
  ]
}

#[cfg(feature = "svg")]
fn svg_input_elements() -> [D3D12_INPUT_ELEMENT_DESC; 2] {
  [
    input_element(
      0,
      DXGI_FORMAT_R32G32_FLOAT,
      0,
      0,
      D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
      0,
    ),
    input_element(
      1,
      DXGI_FORMAT_R32G32B32A32_FLOAT,
      0,
      8,
      D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
      0,
    ),
  ]
}

fn input_element(
  semantic_index: u32,
  format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT,
  slot: u32,
  offset: u32,
  slot_class: windows::Win32::Graphics::Direct3D12::D3D12_INPUT_CLASSIFICATION,
  step_rate: u32,
) -> D3D12_INPUT_ELEMENT_DESC {
  D3D12_INPUT_ELEMENT_DESC {
    SemanticName: PCSTR(b"TEXCOORD\0".as_ptr()),
    SemanticIndex: semantic_index,
    Format: format,
    InputSlot: slot,
    AlignedByteOffset: offset,
    InputSlotClass: slot_class,
    InstanceDataStepRate: step_rate,
  }
}

fn glyph_instance(glyph: &GlyphCmd) -> GlyphInstance {
  GlyphInstance {
    pos: [glyph.x, glyph.y],
    size: [glyph.width, glyph.height],
    color: glyph.color,
    uv_min: glyph.uv_min,
    uv_max: glyph.uv_max,
    transform: glyph.transform,
    xf_origin: glyph.transform_origin,
  }
}

unsafe fn create_linear_clamp_sampler(device: &ID3D12Device, handle: D3D12_CPU_DESCRIPTOR_HANDLE) {
  let desc = D3D12_SAMPLER_DESC {
    Filter: D3D12_FILTER_MIN_MAG_MIP_LINEAR,
    AddressU: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
    AddressV: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
    AddressW: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
    MipLODBias: 0.0,
    MaxAnisotropy: 1,
    ComparisonFunc: D3D12_COMPARISON_FUNC_ALWAYS,
    BorderColor: [0.0; 4],
    MinLOD: 0.0,
    MaxLOD: f32::MAX,
  };
  device.CreateSampler(&desc, handle);
}

unsafe fn create_r8_texture(device: &ID3D12Device, width: u32, height: u32) -> Result<ID3D12Resource> {
  let heap_properties = D3D12_HEAP_PROPERTIES {
    Type: D3D12_HEAP_TYPE_DEFAULT,
    CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
    MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
    CreationNodeMask: 1,
    VisibleNodeMask: 1,
  };
  let desc = D3D12_RESOURCE_DESC {
    Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
    Alignment: 0,
    Width: width as u64,
    Height: height,
    DepthOrArraySize: 1,
    MipLevels: 1,
    Format: DXGI_FORMAT_R8_UNORM,
    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
    Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
    Flags: D3D12_RESOURCE_FLAG_NONE,
  };
  let mut resource = None;
  device.CreateCommittedResource(
    &heap_properties,
    D3D12_HEAP_FLAG_NONE,
    &desc,
    D3D12_RESOURCE_STATE_COPY_DEST,
    None,
    &mut resource,
  )?;
  resource.ok_or_else(Error::from_win32)
}

#[cfg(feature = "image")]
unsafe fn create_rgba_texture(device: &ID3D12Device, width: u32, height: u32) -> Result<ID3D12Resource> {
  let heap_properties = D3D12_HEAP_PROPERTIES {
    Type: D3D12_HEAP_TYPE_DEFAULT,
    CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
    MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
    CreationNodeMask: 1,
    VisibleNodeMask: 1,
  };
  let desc = D3D12_RESOURCE_DESC {
    Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
    Alignment: 0,
    Width: width as u64,
    Height: height,
    DepthOrArraySize: 1,
    MipLevels: 1,
    Format: DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
    SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
    Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
    Flags: D3D12_RESOURCE_FLAG_NONE,
  };
  let mut resource = None;
  device.CreateCommittedResource(
    &heap_properties,
    D3D12_HEAP_FLAG_NONE,
    &desc,
    D3D12_RESOURCE_STATE_COPY_DEST,
    None,
    &mut resource,
  )?;
  resource.ok_or_else(Error::from_win32)
}

fn rect_instances(rect: &RectCmd) -> Vec<QuadInstance> {
  let mut instances = Vec::with_capacity(2);
  instances.push(QuadInstance {
    pos: [rect.x, rect.y],
    size: [rect.width, rect.height],
    color: rect.color.to_linear_f32_array(),
    radii_h: rect.radii,
    radii_v: rect.radii,
    stroke: [0.0; 4],
    pattern: [0.0; 4],
    transform: rect.transform,
    xf_origin: rect.transform_origin,
    shadow_sigma: 0.0,
    gradient_offset: -1.0,
  });
  if rect.stroke.iter().any(|width| *width > 0.0) {
    instances.push(QuadInstance {
      pos: [rect.x, rect.y],
      size: [rect.width, rect.height],
      color: rect.stroke_color.to_linear_f32_array(),
      radii_h: rect.radii,
      radii_v: rect.radii,
      stroke: rect.stroke,
      pattern: [0.0; 4],
      transform: rect.transform,
      xf_origin: rect.transform_origin,
      shadow_sigma: 0.0,
      gradient_offset: -1.0,
    });
  }
  instances
}

fn same_clip(a: ClipRect, b: ClipRect) -> bool {
  a.active == b.active && a.x == b.x && a.y == b.y && a.width == b.width && a.height == b.height
}

fn scissor_rect(clip: ClipRect, vw: f32, vh: f32) -> Option<RECT> {
  let viewport_w = vw.ceil().max(1.0) as i32;
  let viewport_h = vh.ceil().max(1.0) as i32;
  if clip.active {
    let left_f = clip.x.floor().max(0.0);
    let top_f = clip.y.floor().max(0.0);
    let right_f = (clip.x + clip.width).ceil().clamp(0.0, viewport_w as f32);
    let bottom_f = (clip.y + clip.height).ceil().clamp(0.0, viewport_h as f32);
    if right_f <= left_f || bottom_f <= top_f {
      return None;
    }

    let left = left_f as i32;
    let top = top_f as i32;
    if left >= viewport_w || top >= viewport_h {
      return None;
    }
    let width = ((right_f - left_f).ceil() as i32).min(viewport_w.saturating_sub(left));
    let height = ((bottom_f - top_f).ceil() as i32).min(viewport_h.saturating_sub(top));
    if width <= 0 || height <= 0 {
      return None;
    }
    Some(RECT {
      left,
      top,
      right: left + width,
      bottom: top + height,
    })
  } else {
    Some(RECT {
      left: 0,
      top: 0,
      right: viewport_w,
      bottom: viewport_h,
    })
  }
}

fn align_up(value: usize, alignment: usize) -> usize {
  (value + alignment - 1) & !(alignment - 1)
}

impl Dx12State {
  unsafe fn new(hwnd: HWND, width: u32, height: u32) -> Result<Self> {
    let factory = create_factory()?;
    let adapter = choose_adapter(&factory)?;
    let device = create_device(&adapter)?;

    let queue_desc = D3D12_COMMAND_QUEUE_DESC {
      Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
      Priority: 0,
      Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
      NodeMask: 0,
    };
    let command_queue: ID3D12CommandQueue = device.CreateCommandQueue(&queue_desc)?;

    let swapchain_desc = DXGI_SWAP_CHAIN_DESC1 {
      Width: width.max(1),
      Height: height.max(1),
      Format: SWAPCHAIN_FORMAT,
      Stereo: FALSE,
      SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
      BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
      BufferCount: FRAME_COUNT as u32,
      Scaling: DXGI_SCALING_NONE,
      SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
      AlphaMode: DXGI_ALPHA_MODE_IGNORE,
      Flags: 0,
    };
    let swapchain1 = factory.CreateSwapChainForHwnd(
      &command_queue.cast::<windows::core::IUnknown>()?,
      hwnd,
      &swapchain_desc,
      None,
      None::<&IDXGIOutput>,
    )?;
    let swapchain: IDXGISwapChain3 = swapchain1.cast()?;
    factory.MakeWindowAssociation(hwnd, DXGI_MWA_NO_ALT_ENTER)?;

    let rtv_heap = CpuDescriptorHeap::new(
      &device,
      D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
      FRAME_COUNT as u32,
      D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
    )?;
    let srv_heap = CpuDescriptorHeap::new(
      &device,
      D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
      SRV_DESCRIPTOR_COUNT,
      D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
    )?;
    let sampler_heap = CpuDescriptorHeap::new(
      &device,
      D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER,
      1,
      D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
    )?;
    create_linear_clamp_sampler(&device, sampler_heap.cpu_handle(0));

    let quad_buffers = StaticQuadBuffers::new(&device)?;
    let rect_pipeline = RectPipeline::new(&device, RENDER_TARGET_FORMAT)?;
    let glyph_pipeline = GlyphPipeline::new(&device, RENDER_TARGET_FORMAT)?;
    #[cfg(feature = "image")]
    let image_pipeline = ImagePipeline::new(&device, RENDER_TARGET_FORMAT)?;
    #[cfg(feature = "svg")]
    let svg_pipeline = SvgPipeline::new(&device, RENDER_TARGET_FORMAT)?;

    let command_allocators = [
      device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)?,
      device.CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)?,
    ];
    let command_list: ID3D12GraphicsCommandList = device.CreateCommandList(
      0,
      D3D12_COMMAND_LIST_TYPE_DIRECT,
      &command_allocators[0],
      None::<&ID3D12PipelineState>,
    )?;
    command_list.Close()?;

    let fence: ID3D12Fence = device.CreateFence(0, D3D12_FENCE_FLAG_NONE)?;
    let fence_event = CreateEventW(None, FALSE, FALSE, None)?;
    let frame_arenas = [
      UploadArena::new(&device, FRAME_UPLOAD_ARENA_BYTES)?,
      UploadArena::new(&device, FRAME_UPLOAD_ARENA_BYTES)?,
    ];

    let mut state = Self {
      device,
      command_queue,
      swapchain,
      rtv_heap,
      srv_heap,
      sampler_heap,
      render_targets: [None, None],
      quad_buffers,
      rect_pipeline,
      glyph_pipeline,
      #[cfg(feature = "image")]
      image_pipeline,
      #[cfg(feature = "svg")]
      svg_pipeline,
      glyph_atlas: None,
      #[cfg(feature = "image")]
      image_textures: HashMap::new(),
      #[cfg(feature = "image")]
      next_srv_index: 1,
      frame_arenas,
      frame_uploads: std::array::from_fn(|_| Vec::new()),
      command_allocators,
      command_list,
      fence,
      fence_event,
      fence_values: [0, 0],
      next_fence_value: 1,
      frame_index: 0,
      width: width.max(1),
      height: height.max(1),
    };
    state.create_render_targets()?;
    state.frame_index = state.swapchain.GetCurrentBackBufferIndex() as usize;
    Ok(state)
  }

  unsafe fn resize(&mut self, width: u32, height: u32) -> Result<()> {
    let width = width.max(1);
    let height = height.max(1);
    if self.width == width && self.height == height {
      return Ok(());
    }

    self.wait_for_gpu()?;
    self.render_targets = [None, None];
    for uploads in &mut self.frame_uploads {
      uploads.clear();
    }
    self
      .swapchain
      .ResizeBuffers(FRAME_COUNT as u32, width, height, SWAPCHAIN_FORMAT, Default::default())?;
    self.width = width;
    self.height = height;
    self.create_render_targets()?;
    self.frame_index = self.swapchain.GetCurrentBackBufferIndex() as usize;
    Ok(())
  }

  unsafe fn render(&mut self, list: &RenderList, profiling_enabled: bool) -> Result<RenderProfile> {
    let total_start = ProfileScope::maybe_start(profiling_enabled);
    let acquire_start = ProfileScope::maybe_start(profiling_enabled);
    self.frame_index = self.swapchain.GetCurrentBackBufferIndex() as usize;
    self.wait_for_frame(self.frame_index)?;
    self.frame_uploads[self.frame_index].clear();
    self.frame_arenas[self.frame_index].reset();
    let allocator = &self.command_allocators[self.frame_index];
    allocator.Reset()?;
    self.command_list.Reset(allocator, None::<&ID3D12PipelineState>)?;
    let acquire_dur = ProfileScope::elapsed_or_default(&acquire_start);

    let encode_start = ProfileScope::maybe_start(profiling_enabled);
    let target = self.current_render_target();
    self.transition_resource(
      &target,
      D3D12_RESOURCE_STATE_PRESENT,
      D3D12_RESOURCE_STATE_RENDER_TARGET,
    );
    let rtv = self.current_rtv_handle();
    self.command_list.OMSetRenderTargets(1, Some(&rtv), FALSE, None);
    self
      .command_list
      .ClearRenderTargetView(rtv, &[1.0, 1.0, 1.0, 1.0], None);

    let atlas_start = ProfileScope::maybe_start(profiling_enabled);
    self.update_glyph_atlas(list)?;
    let atlas_dur = ProfileScope::elapsed_or_default(&atlas_start);
    self.draw_ordered(list)?;

    self.transition_resource(
      &target,
      D3D12_RESOURCE_STATE_RENDER_TARGET,
      D3D12_RESOURCE_STATE_PRESENT,
    );

    self.command_list.Close()?;
    let encode_dur = ProfileScope::elapsed_or_default(&encode_start);

    let submit_start = ProfileScope::maybe_start(profiling_enabled);
    let command_list: ID3D12CommandList = self.command_list.cast()?;
    self.command_queue.ExecuteCommandLists(&[Some(command_list)]);
    self.signal_current_frame()?;
    let submit_dur = ProfileScope::elapsed_or_default(&submit_start);

    let present_start = ProfileScope::maybe_start(profiling_enabled);
    self.swapchain.Present(1, Default::default()).ok()?;
    self.frame_index = self.swapchain.GetCurrentBackBufferIndex() as usize;
    let present_dur = ProfileScope::elapsed_or_default(&present_start);

    Ok(RenderProfile {
      acquire: acquire_dur,
      atlas_upload: atlas_dur,
      encode: encode_dur,
      submit: submit_dur,
      present: present_dur,
      total: ProfileScope::elapsed_or_default(&total_start),
      ..RenderProfile::default()
    })
  }

  unsafe fn upload_frame_bytes(&mut self, data: &[u8], alignment: usize) -> Result<UploadSlice> {
    if let Some(slice) = self.frame_arenas[self.frame_index].upload_bytes(data, alignment) {
      return Ok(slice);
    }

    let padded_size = align_up(data.len().max(1), alignment.max(1));
    let upload = UploadBuffer::from_bytes_padded(&self.device, data, padded_size)?;
    let slice = UploadSlice {
      gpu_address: upload.gpu_address,

      size_in_bytes: upload.size_in_bytes,
    };
    self.frame_uploads[self.frame_index].push(upload);
    Ok(slice)
  }

  unsafe fn upload_frame_pod_slice<T: bytemuck::Pod>(&mut self, data: &[T], alignment: usize) -> Result<UploadSlice> {
    self.upload_frame_bytes(bytemuck::cast_slice(data), alignment)
  }

  unsafe fn upload_frame_constant<T: bytemuck::Pod>(&mut self, data: &T) -> Result<UploadSlice> {
    self.upload_frame_bytes(bytemuck::bytes_of(data), 256)
  }

  unsafe fn draw_ordered(&mut self, list: &RenderList) -> Result<()> {
    let has_draws = !list.rects.is_empty() || !list.glyphs.is_empty();
    #[cfg(feature = "image")]
    let has_draws = has_draws || !list.images.is_empty();
    #[cfg(feature = "svg")]
    let has_draws = has_draws || !list.svgs.is_empty();
    if !has_draws {
      return Ok(());
    }

    let viewport = D3D12_VIEWPORT {
      TopLeftX: 0.0,
      TopLeftY: 0.0,
      Width: self.width as f32,
      Height: self.height as f32,
      MinDepth: 0.0,
      MaxDepth: 1.0,
    };
    self.command_list.RSSetViewports(std::slice::from_ref(&viewport));

    let globals = Globals {
      viewport: [self.width as f32, self.height as f32, 0.0, 0.0],
      clip_rect: [0.0; 4],
      clip_radii_h: [0.0; 4],
      clip_radii_v: [0.0; 4],
      clip_active: [0.0; 4],
    };
    let globals_upload = self.upload_frame_constant(&globals)?;
    let globals_address = globals_upload.gpu_address;

    self
      .command_list
      .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    self.command_list.IASetIndexBuffer(Some(&self.quad_buffers.index_view));

    enum OrderedDraw {
      Rect(usize),
      Glyph {
        start: usize,
        count: usize,
      },
      #[cfg(feature = "image")]
      Image(usize),
      #[cfg(feature = "svg")]
      Svg(usize),
    }

    let mut ordered_draws = Vec::new();
    for (index, rect) in list.rects.iter().enumerate() {
      ordered_draws.push((rect.order, OrderedDraw::Rect(index)));
    }
    let mut glyph_start = 0;
    while glyph_start < list.glyphs.len() {
      let order = list.glyphs[glyph_start].order;
      let clip = list.glyphs[glyph_start].clip;
      let mut glyph_end = glyph_start + 1;
      while glyph_end < list.glyphs.len()
        && list.glyphs[glyph_end].order == order
        && same_clip(list.glyphs[glyph_end].clip, clip)
      {
        glyph_end += 1;
      }
      ordered_draws.push((
        order,
        OrderedDraw::Glyph {
          start: glyph_start,
          count: glyph_end - glyph_start,
        },
      ));
      glyph_start = glyph_end;
    }
    #[cfg(feature = "image")]
    for (index, image) in list.images.iter().enumerate() {
      ordered_draws.push((image.order, OrderedDraw::Image(index)));
    }
    #[cfg(feature = "svg")]
    for (index, svg) in list.svgs.iter().enumerate() {
      ordered_draws.push((svg.order, OrderedDraw::Svg(index)));
    }
    ordered_draws.sort_by_key(|(order, _)| *order);

    for (_, draw) in ordered_draws {
      match draw {
        OrderedDraw::Rect(index) => self.draw_rect(&list.rects[index], globals_address)?,
        OrderedDraw::Glyph { start, count } => self.draw_glyphs(&list.glyphs[start..start + count], globals_address)?,
        #[cfg(feature = "image")]
        OrderedDraw::Image(index) => self.draw_image(&list.images[index], globals_address)?,
        #[cfg(feature = "svg")]
        OrderedDraw::Svg(index) => self.draw_svg(&list.svgs[index], globals_address)?,
      }
    }

    Ok(())
  }

  unsafe fn draw_rect(&mut self, rect: &RectCmd, globals_address: u64) -> Result<()> {
    let Some(scissor) = scissor_rect(rect.clip, self.width as f32, self.height as f32) else {
      return Ok(());
    };

    self.command_list.SetPipelineState(&self.rect_pipeline.pipeline_state);
    self
      .command_list
      .SetGraphicsRootSignature(&self.rect_pipeline.root_signature);
    self.command_list.SetGraphicsRootConstantBufferView(0, globals_address);

    let instances = rect_instances(rect);
    let instance_upload = self.upload_frame_pod_slice(&instances, 16)?;
    let instance_view = instance_upload.vertex_view::<QuadInstance>();

    let vertex_views = [self.quad_buffers.vertex_view, instance_view];
    self.command_list.RSSetScissorRects(std::slice::from_ref(&scissor));
    self.command_list.IASetVertexBuffers(0, Some(&vertex_views));
    self
      .command_list
      .DrawIndexedInstanced(QuadVertex::INDICES.len() as u32, instances.len() as u32, 0, 0, 0);

    Ok(())
  }

  unsafe fn draw_glyphs(&mut self, glyphs: &[GlyphCmd], globals_address: u64) -> Result<()> {
    if glyphs.is_empty() || self.glyph_atlas.is_none() {
      return Ok(());
    }
    let Some(scissor) = scissor_rect(glyphs[0].clip, self.width as f32, self.height as f32) else {
      return Ok(());
    };

    let instances: Vec<GlyphInstance> = glyphs.iter().map(glyph_instance).collect();
    let instance_upload = self.upload_frame_pod_slice(&instances, 16)?;
    let instance_view = instance_upload.vertex_view::<GlyphInstance>();

    let descriptor_heaps = [Some(self.srv_heap.heap.clone()), Some(self.sampler_heap.heap.clone())];
    self.command_list.SetDescriptorHeaps(&descriptor_heaps);
    self.command_list.SetPipelineState(&self.glyph_pipeline.pipeline_state);
    self
      .command_list
      .SetGraphicsRootSignature(&self.glyph_pipeline.root_signature);
    self.command_list.SetGraphicsRootConstantBufferView(0, globals_address);
    self
      .command_list
      .SetGraphicsRootDescriptorTable(1, self.srv_heap.gpu_handle(GLYPH_ATLAS_SRV_INDEX));
    self
      .command_list
      .SetGraphicsRootDescriptorTable(2, self.sampler_heap.gpu_handle(0));

    let vertex_views = [self.quad_buffers.vertex_view, instance_view];
    self.command_list.RSSetScissorRects(std::slice::from_ref(&scissor));
    self.command_list.IASetVertexBuffers(0, Some(&vertex_views));
    self
      .command_list
      .DrawIndexedInstanced(QuadVertex::INDICES.len() as u32, instances.len() as u32, 0, 0, 0);

    Ok(())
  }

  #[cfg(feature = "image")]
  unsafe fn draw_image(&mut self, image: &crate::images::ImageCmd, globals_address: u64) -> Result<()> {
    if image.image_width == 0 || image.image_height == 0 {
      return Ok(());
    }
    let Some(scissor) = scissor_rect(image.clip, self.width as f32, self.height as f32) else {
      return Ok(());
    };
    let descriptor_index = self.ensure_image_texture(image)?;

    let instance = ImageInstance {
      pos: [image.x, image.y],
      size: [image.width, image.height],
      opacity: [1.0, 0.0, 0.0, 0.0],
      transform: [1.0, 0.0, 0.0, 1.0],
      xf_origin: [0.0, 0.0],
      uv_min: image.uv_min,
      uv_max: image.uv_max,
      radii: image.radii,
    };
    let instance_upload = self.upload_frame_pod_slice(&[instance], 16)?;
    let instance_view = instance_upload.vertex_view::<ImageInstance>();

    let descriptor_heaps = [Some(self.srv_heap.heap.clone()), Some(self.sampler_heap.heap.clone())];
    self.command_list.SetDescriptorHeaps(&descriptor_heaps);
    self.command_list.SetPipelineState(&self.image_pipeline.pipeline_state);
    self
      .command_list
      .SetGraphicsRootSignature(&self.image_pipeline.root_signature);
    self.command_list.SetGraphicsRootConstantBufferView(0, globals_address);
    self
      .command_list
      .SetGraphicsRootDescriptorTable(1, self.srv_heap.gpu_handle(descriptor_index));
    self
      .command_list
      .SetGraphicsRootDescriptorTable(2, self.sampler_heap.gpu_handle(0));

    let vertex_views = [self.quad_buffers.vertex_view, instance_view];
    self.command_list.RSSetScissorRects(std::slice::from_ref(&scissor));
    self.command_list.IASetVertexBuffers(0, Some(&vertex_views));
    self
      .command_list
      .DrawIndexedInstanced(QuadVertex::INDICES.len() as u32, 1, 0, 0, 0);

    Ok(())
  }

  #[cfg(feature = "svg")]
  unsafe fn draw_svg(&mut self, svg: &crate::svg::SvgCmd, globals_address: u64) -> Result<()> {
    if svg.mesh.vertices.is_empty() || svg.mesh.indices.is_empty() {
      return Ok(());
    }
    let Some(scissor) = scissor_rect(svg.clip, self.width as f32, self.height as f32) else {
      return Ok(());
    };

    let vertices: Vec<SvgVertexGpu> = svg
      .mesh
      .vertices
      .iter()
      .map(|vertex| SvgVertexGpu {
        position: [vertex.position[0] + svg.x, vertex.position[1] + svg.y],
        color: vertex.color,
      })
      .collect();
    let vertex_upload = self.upload_frame_pod_slice(&vertices, 16)?;
    let index_upload = self.upload_frame_pod_slice(&svg.mesh.indices, 4)?;
    let vertex_view = vertex_upload.vertex_view::<SvgVertexGpu>();
    let index_view = D3D12_INDEX_BUFFER_VIEW {
      BufferLocation: index_upload.gpu_address,
      SizeInBytes: index_upload.size_in_bytes,
      Format: DXGI_FORMAT_R32_UINT,
    };

    self.command_list.SetPipelineState(&self.svg_pipeline.pipeline_state);
    self
      .command_list
      .SetGraphicsRootSignature(&self.svg_pipeline.root_signature);
    self.command_list.SetGraphicsRootConstantBufferView(0, globals_address);
    self.command_list.RSSetScissorRects(std::slice::from_ref(&scissor));
    self
      .command_list
      .IASetVertexBuffers(0, Some(std::slice::from_ref(&vertex_view)));
    self.command_list.IASetIndexBuffer(Some(&index_view));
    self
      .command_list
      .DrawIndexedInstanced(svg.mesh.indices.len() as u32, 1, 0, 0, 0);
    self.command_list.IASetIndexBuffer(Some(&self.quad_buffers.index_view));

    Ok(())
  }

  unsafe fn update_glyph_atlas(&mut self, list: &RenderList) -> Result<()> {
    let atlas = &list.atlas;
    if list.glyphs.is_empty() || atlas.width == 0 || atlas.height == 0 {
      return Ok(());
    }

    let recreate = self.glyph_atlas.as_ref().map_or(true, |texture| {
      texture.width != atlas.width || texture.height != atlas.height
    });
    if recreate {
      let texture = create_r8_texture(&self.device, atlas.width, atlas.height)?;
      let srv_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
        Format: DXGI_FORMAT_R8_UNORM,
        ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
        Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
        Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
          Texture2D: D3D12_TEX2D_SRV {
            MostDetailedMip: 0,
            MipLevels: 1,
            PlaneSlice: 0,
            ResourceMinLODClamp: 0.0,
          },
        },
      };
      self.device.CreateShaderResourceView(
        &texture,
        Some(&srv_desc),
        self.srv_heap.cpu_handle(GLYPH_ATLAS_SRV_INDEX),
      );
      self.glyph_atlas = Some(GlyphAtlasTexture {
        texture,
        width: atlas.width,
        height: atlas.height,
        version: u64::MAX,
        state: D3D12_RESOURCE_STATE_COPY_DEST,
      });
    }

    let needs_upload = self
      .glyph_atlas
      .as_ref()
      .map_or(false, |texture| texture.version != atlas.version);
    if !needs_upload {
      return Ok(());
    }

    let texture = self.glyph_atlas.as_ref().unwrap().texture.clone();
    let current_state = self.glyph_atlas.as_ref().unwrap().state;
    if current_state != D3D12_RESOURCE_STATE_COPY_DEST {
      self.transition_resource(&texture, current_state, D3D12_RESOURCE_STATE_COPY_DEST);
    }

    let row_pitch = align_up(atlas.width as usize, 256);
    let upload_size = row_pitch * atlas.height as usize;
    let mut upload_bytes = vec![0u8; upload_size];
    for row in 0..atlas.height as usize {
      let src_start = row * atlas.width as usize;
      if src_start >= atlas.data.len() {
        break;
      }
      let src_end = (src_start + atlas.width as usize).min(atlas.data.len());
      let dst_start = row * row_pitch;
      upload_bytes[dst_start..dst_start + (src_end - src_start)].copy_from_slice(&atlas.data[src_start..src_end]);
    }

    let upload = UploadBuffer::from_bytes(&self.device, &upload_bytes)?;
    let mut dst = D3D12_TEXTURE_COPY_LOCATION {
      pResource: ManuallyDrop::new(Some(texture.clone())),
      Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
      Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0 },
    };
    let mut src = D3D12_TEXTURE_COPY_LOCATION {
      pResource: ManuallyDrop::new(Some(upload._resource.clone())),
      Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
      Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
        PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
          Offset: 0,
          Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
            Format: DXGI_FORMAT_R8_UNORM,
            Width: atlas.width,
            Height: atlas.height,
            Depth: 1,
            RowPitch: row_pitch as u32,
          },
        },
      },
    };
    self.command_list.CopyTextureRegion(&dst, 0, 0, 0, &src, None);
    ManuallyDrop::drop(&mut dst.pResource);
    ManuallyDrop::drop(&mut src.pResource);
    self.transition_resource(
      &texture,
      D3D12_RESOURCE_STATE_COPY_DEST,
      D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
    );
    self.frame_uploads[self.frame_index].push(upload);

    if let Some(glyph_atlas) = &mut self.glyph_atlas {
      glyph_atlas.version = atlas.version;
      glyph_atlas.state = D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE;
    }

    Ok(())
  }

  #[cfg(feature = "image")]
  unsafe fn ensure_image_texture(&mut self, image: &crate::images::ImageCmd) -> Result<usize> {
    if let Some(cached) = self.image_textures.get(&image.image_id) {
      let descriptor_index = cached.descriptor_index;
      let frame_index = cached.frame_index;
      let texture = cached._texture.clone();
      if frame_index != image.frame_index {
        self.upload_image_texture(&texture, image, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE)?;
        if let Some(cached) = self.image_textures.get_mut(&image.image_id) {
          cached.frame_index = image.frame_index;
        }
      }
      return Ok(descriptor_index);
    }
    if self.next_srv_index >= SRV_DESCRIPTOR_COUNT as usize {
      return Err(Error::from_win32());
    }

    let descriptor_index = self.next_srv_index;
    self.next_srv_index += 1;
    let texture = create_rgba_texture(&self.device, image.image_width, image.image_height)?;
    let srv_desc = D3D12_SHADER_RESOURCE_VIEW_DESC {
      Format: DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
      ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
      Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
      Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
        Texture2D: D3D12_TEX2D_SRV {
          MostDetailedMip: 0,
          MipLevels: 1,
          PlaneSlice: 0,
          ResourceMinLODClamp: 0.0,
        },
      },
    };
    self
      .device
      .CreateShaderResourceView(&texture, Some(&srv_desc), self.srv_heap.cpu_handle(descriptor_index));

    self.upload_image_texture(&texture, image, D3D12_RESOURCE_STATE_COPY_DEST)?;

    self.image_textures.insert(
      image.image_id,
      CachedImageTexture {
        _texture: texture,
        descriptor_index,
        frame_index: image.frame_index,
      },
    );
    Ok(descriptor_index)
  }

  #[cfg(feature = "image")]
  unsafe fn upload_image_texture(
    &mut self,
    texture: &ID3D12Resource,
    image: &crate::images::ImageCmd,
    before_state: D3D12_RESOURCE_STATES,
  ) -> Result<()> {
    if before_state != D3D12_RESOURCE_STATE_COPY_DEST {
      self.transition_resource(texture, before_state, D3D12_RESOURCE_STATE_COPY_DEST);
    }

    let row_bytes = image.image_width as usize * 4;
    let row_pitch = align_up(row_bytes, 256);
    let upload_size = row_pitch * image.image_height as usize;
    let mut upload_bytes = vec![0u8; upload_size];
    for row in 0..image.image_height as usize {
      let src_start = row * row_bytes;
      if src_start >= image.data.len() {
        break;
      }
      let src_end = (src_start + row_bytes).min(image.data.len());
      let dst_start = row * row_pitch;
      upload_bytes[dst_start..dst_start + (src_end - src_start)].copy_from_slice(&image.data[src_start..src_end]);
    }

    let upload = UploadBuffer::from_bytes(&self.device, &upload_bytes)?;
    let mut dst = D3D12_TEXTURE_COPY_LOCATION {
      pResource: ManuallyDrop::new(Some(texture.to_owned())),
      Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
      Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0 },
    };
    let mut src = D3D12_TEXTURE_COPY_LOCATION {
      pResource: ManuallyDrop::new(Some(upload._resource.clone())),
      Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
      Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
        PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
          Offset: 0,
          Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
            Format: DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,
            Width: image.image_width,
            Height: image.image_height,
            Depth: 1,
            RowPitch: row_pitch as u32,
          },
        },
      },
    };
    self.command_list.CopyTextureRegion(&dst, 0, 0, 0, &src, None);
    ManuallyDrop::drop(&mut dst.pResource);
    ManuallyDrop::drop(&mut src.pResource);
    self.transition_resource(
      texture,
      D3D12_RESOURCE_STATE_COPY_DEST,
      D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
    );
    self.frame_uploads[self.frame_index].push(upload);
    Ok(())
  }

  unsafe fn create_render_targets(&mut self) -> Result<()> {
    let rtv_desc = render_target_view_desc();
    for index in 0..FRAME_COUNT {
      let target: ID3D12Resource = self.swapchain.GetBuffer(index as u32)?;
      let handle = self.rtv_heap.cpu_handle(index);
      self.device.CreateRenderTargetView(&target, Some(&rtv_desc), handle);
      self.render_targets[index] = Some(target);
    }
    Ok(())
  }

  unsafe fn transition_resource(
    &self,
    resource: &ID3D12Resource,
    before: windows::Win32::Graphics::Direct3D12::D3D12_RESOURCE_STATES,
    after: windows::Win32::Graphics::Direct3D12::D3D12_RESOURCE_STATES,
  ) {
    let mut barrier = D3D12_RESOURCE_BARRIER {
      Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
      Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
      Anonymous: D3D12_RESOURCE_BARRIER_0 {
        Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
          pResource: ManuallyDrop::new(Some(resource.clone())),
          Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
          StateBefore: before,
          StateAfter: after,
        }),
      },
    };
    self.command_list.ResourceBarrier(std::slice::from_ref(&barrier));
    ManuallyDrop::drop(&mut (*barrier.Anonymous.Transition).pResource);
  }

  fn current_render_target(&self) -> ID3D12Resource {
    self.render_targets[self.frame_index]
      .as_ref()
      .expect("missing dx12 render target")
      .clone()
  }

  unsafe fn current_rtv_handle(&self) -> D3D12_CPU_DESCRIPTOR_HANDLE {
    self.rtv_heap.cpu_handle(self.frame_index)
  }

  unsafe fn signal_current_frame(&mut self) -> Result<()> {
    let fence_value = self.next_fence_value;
    self.next_fence_value += 1;
    self.command_queue.Signal(&self.fence, fence_value)?;
    self.fence_values[self.frame_index] = fence_value;
    Ok(())
  }

  unsafe fn wait_for_frame(&self, frame_index: usize) -> Result<()> {
    let fence_value = self.fence_values[frame_index];
    if fence_value != 0 && self.fence.GetCompletedValue() < fence_value {
      self.fence.SetEventOnCompletion(fence_value, self.fence_event)?;
      let wait = WaitForSingleObject(self.fence_event, INFINITE);
      debug_assert_eq!(wait, WAIT_OBJECT_0);
    }
    Ok(())
  }

  unsafe fn wait_for_gpu(&mut self) -> Result<()> {
    let fence_value = self.next_fence_value;
    self.next_fence_value += 1;
    self.command_queue.Signal(&self.fence, fence_value)?;
    self.fence.SetEventOnCompletion(fence_value, self.fence_event)?;
    let wait = WaitForSingleObject(self.fence_event, INFINITE);
    debug_assert_eq!(wait, WAIT_OBJECT_0);
    self.fence_values = [0; FRAME_COUNT];
    Ok(())
  }
}

impl Drop for Dx12State {
  fn drop(&mut self) {
    unsafe {
      let _ = self.wait_for_gpu();
      let _ = CloseHandle(self.fence_event);
    }
  }
}

unsafe fn choose_adapter(factory: &IDXGIFactory4) -> Result<IDXGIAdapter1> {
  let mut index = 0;
  loop {
    let adapter = match factory.EnumAdapters1(index) {
      Ok(adapter) => adapter,
      Err(err) => {
        if index == 0 {
          return Err(err);
        }
        break;
      }
    };
    let desc = adapter.GetDesc1()?;
    if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 == 0 && create_device(&adapter).is_ok() {
      return Ok(adapter);
    }
    index += 1;
  }

  factory.EnumAdapters1(0)
}

unsafe fn create_device(adapter: &IDXGIAdapter1) -> Result<ID3D12Device> {
  let mut device = None;
  D3D12CreateDevice(adapter, D3D_FEATURE_LEVEL_11_0, &mut device)?;
  device.ok_or_else(Error::from_win32)
}

fn dxgi_factory_flags() -> DXGI_CREATE_FACTORY_FLAGS {
  if cfg!(debug_assertions) {
    DXGI_CREATE_FACTORY_DEBUG
  } else {
    DXGI_CREATE_FACTORY_FLAGS(0)
  }
}

unsafe fn create_factory() -> Result<IDXGIFactory4> {
  match CreateDXGIFactory2(dxgi_factory_flags()) {
    Ok(factory) => Ok(factory),
    Err(_) if cfg!(debug_assertions) => CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)),
    Err(err) => Err(err),
  }
}

fn hwnd_from_window(window: WindowHandle<'_>) -> Result<HWND> {
  match window.as_raw() {
    RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get() as *mut _)),
    _ => Err(Error::from_win32()),
  }
}

fn offset_cpu_handle(
  handle: D3D12_CPU_DESCRIPTOR_HANDLE,
  descriptor_size: u32,
  index: usize,
) -> D3D12_CPU_DESCRIPTOR_HANDLE {
  D3D12_CPU_DESCRIPTOR_HANDLE {
    ptr: handle.ptr + descriptor_size as usize * index,
  }
}

fn offset_gpu_handle(
  handle: D3D12_GPU_DESCRIPTOR_HANDLE,
  descriptor_size: u32,
  index: usize,
) -> D3D12_GPU_DESCRIPTOR_HANDLE {
  D3D12_GPU_DESCRIPTOR_HANDLE {
    ptr: handle.ptr + descriptor_size as u64 * index as u64,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn scissor_expands_fractional_clip_to_include_bottom_right_edge() {
    let rect = scissor_rect(
      ClipRect {
        x: 10.6,
        y: 20.2,
        width: 30.1,
        height: 40.6,
        active: true,
      },
      100.0,
      100.0,
    )
    .unwrap();

    assert_eq!(rect.left, 10);
    assert_eq!(rect.top, 20);
    assert_eq!(rect.right, 41);
    assert_eq!(rect.bottom, 61);
  }

  #[test]
  fn dx12_hlsl_shaders_compile() {
    unsafe {
      compile_shader(include_bytes!("shaders/quad.hlsl"), b"vs_main\0", b"vs_5_0\0").unwrap();
      compile_shader(include_bytes!("shaders/quad.hlsl"), b"ps_main\0", b"ps_5_0\0").unwrap();
      compile_shader(include_bytes!("shaders/glyph.hlsl"), b"vs_main\0", b"vs_5_0\0").unwrap();
      compile_shader(include_bytes!("shaders/glyph.hlsl"), b"ps_main\0", b"ps_5_0\0").unwrap();
      #[cfg(feature = "image")]
      {
        compile_shader(include_bytes!("shaders/image.hlsl"), b"vs_main\0", b"vs_5_0\0").unwrap();
        compile_shader(include_bytes!("shaders/image.hlsl"), b"ps_main\0", b"ps_5_0\0").unwrap();
      }
      #[cfg(feature = "svg")]
      {
        compile_shader(include_bytes!("shaders/svg.hlsl"), b"vs_main\0", b"vs_5_0\0").unwrap();
        compile_shader(include_bytes!("shaders/svg.hlsl"), b"ps_main\0", b"ps_5_0\0").unwrap();
      }
    }
  }
}
