#![allow(unsafe_op_in_unsafe_fn)]

use std::{ffi::c_void, mem::ManuallyDrop, ptr};

mod vertex;

use raw_window_handle::{DisplayHandle, RawWindowHandle, WindowHandle};
use vertex::QuadVertex;
use windows::{
  Win32::{
    Foundation::{CloseHandle, FALSE, HANDLE, HWND, WAIT_OBJECT_0},
    Graphics::{
      Direct3D::D3D_FEATURE_LEVEL_11_0,
      Direct3D12::{
        D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC, D3D12_COMMAND_QUEUE_FLAG_NONE,
        D3D12_CPU_DESCRIPTOR_HANDLE, D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_DESCRIPTOR_HEAP_DESC,
        D3D12_DESCRIPTOR_HEAP_FLAG_NONE, D3D12_DESCRIPTOR_HEAP_FLAGS, D3D12_DESCRIPTOR_HEAP_TYPE,
        D3D12_DESCRIPTOR_HEAP_TYPE_RTV, D3D12_FENCE_FLAG_NONE, D3D12_HEAP_FLAG_NONE, D3D12_HEAP_PROPERTIES,
        D3D12_HEAP_TYPE_UPLOAD, D3D12_INDEX_BUFFER_VIEW, D3D12_MEMORY_POOL_UNKNOWN, D3D12_RANGE,
        D3D12_RESOURCE_BARRIER, D3D12_RESOURCE_BARRIER_0, D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
        D3D12_RESOURCE_BARRIER_FLAG_NONE, D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, D3D12_RESOURCE_DESC,
        D3D12_RESOURCE_DIMENSION_BUFFER, D3D12_RESOURCE_FLAG_NONE, D3D12_RESOURCE_STATE_GENERIC_READ,
        D3D12_RESOURCE_STATE_PRESENT, D3D12_RESOURCE_STATE_RENDER_TARGET, D3D12_RESOURCE_TRANSITION_BARRIER,
        D3D12_TEXTURE_LAYOUT_ROW_MAJOR, D3D12_VERTEX_BUFFER_VIEW, D3D12CreateDevice, ID3D12CommandAllocator,
        ID3D12CommandList, ID3D12CommandQueue, ID3D12DescriptorHeap, ID3D12Device, ID3D12Fence,
        ID3D12GraphicsCommandList, ID3D12PipelineState, ID3D12Resource,
      },
      Dxgi::{
        Common::{
          DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R16_UINT, DXGI_FORMAT_UNKNOWN,
          DXGI_SAMPLE_DESC,
        },
        CreateDXGIFactory2, DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_CREATE_FACTORY_DEBUG, DXGI_CREATE_FACTORY_FLAGS,
        DXGI_MWA_NO_ALT_ENTER, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_DISCARD,
        DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIAdapter1, IDXGIFactory4, IDXGIOutput, IDXGISwapChain3,
      },
    },
    System::Threading::{CreateEventW, INFINITE, WaitForSingleObject},
  },
  core::{Error, Interface, Result},
};

use crate::{
  app::{
    profiler::{ProfileScope, RenderProfile},
    render_engine::RenderEngine,
  },
  layout::render_list::RenderList,
};

const FRAME_COUNT: usize = 2;

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
    if let Some(state) = &mut self.state {
      unsafe {
        state
          .resize(self.width, self.height)
          .expect("failed to resize native dx12 swapchain");
      }
    }
  }

  fn render(&mut self, _list: &RenderList, window: WindowHandle<'_>, _display: DisplayHandle<'_>) {
    let profiling_enabled = self.profiling_enabled;
    let total_start = ProfileScope::maybe_start(profiling_enabled);
    let init_start = ProfileScope::maybe_start(profiling_enabled);
    self
      .ensure_initialized(window)
      .expect("failed to initialize native dx12 renderer");
    let init_dur = ProfileScope::elapsed_or_default(&init_start);

    let render_start = ProfileScope::maybe_start(profiling_enabled);
    let state = self.state.as_mut().unwrap();
    unsafe {
      state.render_clear().expect("failed to render native dx12 frame");
    }
    let render_dur = ProfileScope::elapsed_or_default(&render_start);

    if profiling_enabled {
      self.last_profile = RenderProfile {
        init: init_dur,
        encode: render_dur,
        total: ProfileScope::elapsed_or_default(&total_start),
        ..RenderProfile::default()
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
  render_targets: [Option<ID3D12Resource>; FRAME_COUNT],
  _quad_buffers: StaticQuadBuffers,
  command_allocators: [ID3D12CommandAllocator; FRAME_COUNT],
  command_list: ID3D12GraphicsCommandList,
  fence: ID3D12Fence,
  fence_event: HANDLE,
  fence_values: [u64; FRAME_COUNT],
  frame_index: usize,
  width: u32,
  height: u32,
}

struct StaticQuadBuffers {
  _vertex_buffer: UploadBuffer,
  _index_buffer: UploadBuffer,
  _vertex_view: D3D12_VERTEX_BUFFER_VIEW,
  _index_view: D3D12_INDEX_BUFFER_VIEW,
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
      _vertex_view: vertex_view,
      _index_view: index_view,
    })
  }
}

struct UploadBuffer {
  _resource: ID3D12Resource,
  gpu_address: u64,
  size_in_bytes: u32,
}

impl UploadBuffer {
  unsafe fn from_bytes(device: &ID3D12Device, data: &[u8]) -> Result<Self> {
    let size = data.len().max(1) as u64;
    let heap_properties = D3D12_HEAP_PROPERTIES {
      Type: D3D12_HEAP_TYPE_UPLOAD,
      CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
      MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
      CreationNodeMask: 1,
      VisibleNodeMask: 1,
    };
    let desc = buffer_resource_desc(size);
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
    ptr::copy_nonoverlapping(data.as_ptr(), mapped.cast::<u8>(), data.len());
    let written_range = D3D12_RANGE {
      Begin: 0,
      End: data.len(),
    };
    resource.Unmap(0, Some(&written_range));

    Ok(Self {
      gpu_address: resource.GetGPUVirtualAddress(),
      size_in_bytes: data.len() as u32,
      _resource: resource,
    })
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
      Format: DXGI_FORMAT_R8G8B8A8_UNORM,
      Stereo: FALSE,
      SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
      BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
      BufferCount: FRAME_COUNT as u32,
      Scaling: DXGI_SCALING_STRETCH,
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
    let quad_buffers = StaticQuadBuffers::new(&device)?;

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

    let mut state = Self {
      device,
      command_queue,
      swapchain,
      rtv_heap,
      render_targets: [None, None],
      _quad_buffers: quad_buffers,
      command_allocators,
      command_list,
      fence,
      fence_event,
      fence_values: [0, 0],
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
    self.swapchain.ResizeBuffers(
      FRAME_COUNT as u32,
      width,
      height,
      DXGI_FORMAT_R8G8B8A8_UNORM,
      Default::default(),
    )?;
    self.width = width;
    self.height = height;
    self.create_render_targets()?;
    self.frame_index = self.swapchain.GetCurrentBackBufferIndex() as usize;
    Ok(())
  }

  unsafe fn render_clear(&mut self) -> Result<()> {
    self.frame_index = self.swapchain.GetCurrentBackBufferIndex() as usize;
    let allocator = &self.command_allocators[self.frame_index];
    allocator.Reset()?;
    self.command_list.Reset(allocator, None::<&ID3D12PipelineState>)?;

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
    self.transition_resource(
      &target,
      D3D12_RESOURCE_STATE_RENDER_TARGET,
      D3D12_RESOURCE_STATE_PRESENT,
    );

    self.command_list.Close()?;
    let command_list: ID3D12CommandList = self.command_list.cast()?;
    self.command_queue.ExecuteCommandLists(&[Some(command_list)]);
    self.swapchain.Present(1, Default::default()).ok()?;
    self.move_to_next_frame()
  }

  unsafe fn create_render_targets(&mut self) -> Result<()> {
    for index in 0..FRAME_COUNT {
      let target: ID3D12Resource = self.swapchain.GetBuffer(index as u32)?;
      let handle = self.rtv_heap.cpu_handle(index);
      self.device.CreateRenderTargetView(&target, None, handle);
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

  unsafe fn move_to_next_frame(&mut self) -> Result<()> {
    let current_fence_value = self.fence_values[self.frame_index] + 1;
    self.command_queue.Signal(&self.fence, current_fence_value)?;
    self.fence_values[self.frame_index] = current_fence_value;

    self.frame_index = self.swapchain.GetCurrentBackBufferIndex() as usize;
    if self.fence.GetCompletedValue() < self.fence_values[self.frame_index] {
      self
        .fence
        .SetEventOnCompletion(self.fence_values[self.frame_index], self.fence_event)?;
      let wait = WaitForSingleObject(self.fence_event, INFINITE);
      debug_assert_eq!(wait, WAIT_OBJECT_0);
    }

    self.fence_values[self.frame_index] = current_fence_value;
    Ok(())
  }

  unsafe fn wait_for_gpu(&mut self) -> Result<()> {
    let fence_value = self.fence_values[self.frame_index] + 1;
    self.command_queue.Signal(&self.fence, fence_value)?;
    self.fence_values[self.frame_index] = fence_value;
    self.fence.SetEventOnCompletion(fence_value, self.fence_event)?;
    let wait = WaitForSingleObject(self.fence_event, INFINITE);
    debug_assert_eq!(wait, WAIT_OBJECT_0);
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
