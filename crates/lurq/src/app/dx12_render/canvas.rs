//! Native offscreen canvas work on the UI renderer's command queue.
use std::collections::VecDeque;

use windows::Win32::Graphics::{Direct3D12::*, Dxgi::Common::*};

mod draw;
mod pipeline;
mod resources;

use super::*;
#[cfg(feature = "perf_profile")]
use crate::app::profile_types::canvas_upload::{AssetUploadStage, CanvasAssetUploadProfile};
use crate::canvas::{BlendMode, CanvasError, CanvasHandle, CanvasId, CanvasSnapshot, CanvasWeak, GradientKind, gpu::*};
use pipeline::pipeline;
pub(super) use resources::create_srv;
use resources::{copy_location, readback, texture};

struct Backing {
  owner: CanvasWeak,
  texture: ID3D12Resource,
  width: u32,
  height: u32,
}
struct AssetTexture {
  texture: ID3D12Resource,
  bytes: usize,
  last: u64,
}
struct Readback {
  done: Completion,
  buffer: ID3D12Resource,
  width: u32,
  height: u32,
  pitch: u32,
  revision: u64,
}
pub(super) struct Renderer {
  #[cfg(feature = "perf_profile")]
  pub(super) profile: crate::app::profile_types::CanvasProfile,
  #[cfg(feature = "perf_profile")]
  pub(super) profile_context: Option<crate::app::profiler::ProfileContext>,
  meshes: MeshCache,
  surfaces: HashMap<CanvasId, Backing>,
  assets: HashMap<u64, AssetTexture>,
  asset_bytes: usize,
  tick: u64,
  scratch: ID3D12Resource,
  resolve: ID3D12Resource,
  _stencil: ID3D12Resource,
  rtvs: CpuDescriptorHeap,
  dsv: CpuDescriptorHeap,
  srvs: [CpuDescriptorHeap; FRAME_COUNT],
  samplers: CpuDescriptorHeap,
  descriptor: usize,
  root: ID3D12RootSignature,
  seed: ID3D12PipelineState,
  resample: ID3D12PipelineState,
  clip: ID3D12PipelineState,
  solid: ID3D12PipelineState,
  image: ID3D12PipelineState,
  erase: ID3D12PipelineState,
  gradient_linear: ID3D12PipelineState,
  gradient_radial: ID3D12PipelineState,
  gradient_angular: ID3D12PipelineState,
  /// Composites a finished isolated layer onto what was under it, source-over.
  compose: ID3D12PipelineState,
  /// Puts a saved tile back as the base of the draw that composites over it.
  restore: ID3D12PipelineState,
  /// The same composite for the other seventeen blend modes, which read the
  /// backdrop instead of relying on fixed-function blending.
  blend: ID3D12PipelineState,
  /// Tile-sized copies for isolated layers, allocated on first use.
  saved: Vec<ID3D12Resource>,
  layer: Option<ID3D12Resource>,
  batches: Vec<Batch>,
  readbacks: Vec<Readback>,
  pending_stats: Vec<(CanvasWeak, usize, usize, usize)>,
}
impl Renderer {
  pub unsafe fn new(device: &ID3D12Device) -> Result<Self> {
    // Two shader resources: every pipeline binds a pair so that one root
    // signature serves the blend composite, which needs its backdrop, as well.
    let root = create_image_root_signature(device, 2)?;
    let seed = pipeline(device, &root, b"vs_seed\0", b"ps_premul\0", 4, 0)?;
    let resample = pipeline(device, &root, b"vs_seed\0", b"ps_premul\0", 1, 0)?;
    let clip = pipeline(device, &root, b"vs_main\0", b"ps_solid\0", 4, 2)?;
    let solid = pipeline(device, &root, b"vs_main\0", b"ps_solid\0", 4, 3)?;
    let image = pipeline(device, &root, b"vs_main\0", b"ps_premul\0", 4, 3)?;
    let erase = pipeline(device, &root, b"vs_main\0", b"ps_solid\0", 4, 4)?;
    let gradient_linear = pipeline(device, &root, b"vs_main\0", b"ps_gradient_linear\0", 4, 3)?;
    let gradient_radial = pipeline(device, &root, b"vs_main\0", b"ps_gradient_radial\0", 4, 3)?;
    let gradient_angular = pipeline(device, &root, b"vs_main\0", b"ps_gradient_angular\0", 4, 3)?;
    let compose = pipeline(device, &root, b"vs_tile\0", b"ps_premul\0", 4, 5)?;
    let restore = pipeline(device, &root, b"vs_tile\0", b"ps_tile\0", 4, 6)?;
    let blend = pipeline(device, &root, b"vs_tile\0", b"ps_blend\0", 4, 6)?;
    let scratch = texture(device, TILE, TILE, 4, false, D3D12_RESOURCE_STATE_RENDER_TARGET)?;
    let resolve = texture(device, TILE, TILE, 1, false, D3D12_RESOURCE_STATE_COPY_SOURCE)?;
    let stencil = texture(device, TILE, TILE, 4, true, D3D12_RESOURCE_STATE_DEPTH_WRITE)?;
    let rtvs = CpuDescriptorHeap::new(
      device,
      D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
      2,
      D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
    )?;
    device.CreateRenderTargetView(&scratch, None, rtvs.cpu_handle(0));
    let dsv = CpuDescriptorHeap::new(
      device,
      D3D12_DESCRIPTOR_HEAP_TYPE_DSV,
      1,
      D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
    )?;
    device.CreateDepthStencilView(&stencil, None, dsv.cpu_handle(0));
    let srvs = [
      CpuDescriptorHeap::new(
        device,
        D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
        16384,
        D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
      )?,
      CpuDescriptorHeap::new(
        device,
        D3D12_DESCRIPTOR_HEAP_TYPE_CBV_SRV_UAV,
        16384,
        D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
      )?,
    ];
    let samplers = CpuDescriptorHeap::new(
      device,
      D3D12_DESCRIPTOR_HEAP_TYPE_SAMPLER,
      2,
      D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE,
    )?;
    for (index, filter) in [D3D12_FILTER_MIN_MAG_MIP_POINT, D3D12_FILTER_MIN_MAG_MIP_LINEAR]
      .into_iter()
      .enumerate()
    {
      device.CreateSampler(
        &D3D12_SAMPLER_DESC {
          Filter: filter,
          AddressU: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
          AddressV: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
          AddressW: D3D12_TEXTURE_ADDRESS_MODE_CLAMP,
          ComparisonFunc: D3D12_COMPARISON_FUNC_NEVER,
          MaxLOD: f32::MAX,
          MaxAnisotropy: 1,
          ..Default::default()
        },
        samplers.cpu_handle(index),
      );
    }
    Ok(Self {
      #[cfg(feature = "perf_profile")]
      profile: Default::default(),
      #[cfg(feature = "perf_profile")]
      profile_context: None,
      meshes: MeshCache::default(),
      surfaces: HashMap::new(),
      assets: HashMap::new(),
      asset_bytes: 0,
      tick: 0,
      scratch,
      resolve,
      _stencil: stencil,
      rtvs,
      dsv,
      srvs,
      samplers,
      descriptor: 0,
      root,
      seed,
      resample,
      clip,
      solid,
      image,
      erase,
      gradient_linear,
      gradient_radial,
      gradient_angular,
      compose,
      restore,
      blend,
      saved: Vec::new(),
      layer: None,
      batches: Vec::new(),
      readbacks: Vec::new(),
      pending_stats: Vec::new(),
    })
  }
  pub fn backing(&self, canvas: &CanvasHandle) -> Option<ID3D12Resource> {
    self.surfaces.get(&canvas.surface_id()).map(|b| b.texture.clone())
  }
  pub unsafe fn encode(&mut self, state: &mut Dx12State, canvases: &[CanvasHandle]) -> Result<()> {
    profile_if! { self.profile = Default::default(); }
    profile_if! {
      self.profile.asset_upload_details = CanvasAssetUploadProfile::capture(
        self.profile_context.as_ref().is_some_and(|context| context.capture_active()),
        self.asset_bytes,
        self.assets.len(),
      );
    }
    let _process_start = profile_scope!();
    #[cfg(feature = "perf_profile")]
    let _phase = self
      .profile_context
      .as_ref()
      .map(|context| context.phase(crate::app::profiler::Phase::CanvasBackend));
    self.tick += 1;
    self.descriptor = 0;
    let live: HashSet<_> = canvases
      .iter()
      .filter(|c| c.is_attached())
      .map(CanvasHandle::surface_id)
      .collect();
    self.surfaces.retain(|id, b| {
      if live.contains(id) {
        true
      } else {
        state.canvas_retired[state.frame_index].push(b.texture.clone());
        if let Some(c) = b.owner.upgrade() {
          c.set_gpu_bytes(0);
        }
        false
      }
    });
    state.command_list.SetDescriptorHeaps(&[
      Some(self.srvs[state.frame_index].heap.clone()),
      Some(self.samplers.heap.clone()),
    ]);
    state.command_list.SetGraphicsRootSignature(&self.root);
    state
      .command_list
      .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
    for canvas in canvases {
      let Some(mut batch) = canvas.take_batch() else {
        continue;
      };
      profile_if! { self.profile.batches += 1; }
      let mut commands: VecDeque<_> = batch.take_commands().into();
      self.batches.push(batch);
      if !self.surfaces.contains_key(&canvas.surface_id()) && !matches!(commands.front(), Some(Command::Resize { .. }))
      {
        let metrics = canvas.metrics();
        self.resize(state, canvas, metrics.pixel_width, metrics.pixel_height, false)?;
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
            self.resize(state, canvas, width, height, preserve)?;
          }
          Command::Readback(done, metrics, revision) => {
            if let Some(b) = self.surfaces.get(&canvas.surface_id()) {
              let readback = readback(state, &b.texture, b.width, b.height, done, revision)?;
              self.readbacks.push(readback);
              // An explicit submission boundary keeps the readback copy ahead
              // of subsequent rendering/fast clears of the same texture.
              state.command_list.Close()?;
              let _submit_start = profile_scope!();
              state
                .command_queue
                .ExecuteCommandLists(&[Some(state.command_list.cast()?)]);
              profile_if! { self.profile.submit += profile_elapsed!(_submit_start); }
              self.flush_stats();
              state.command_list.Reset(
                &state.command_allocators[state.frame_index],
                None::<&ID3D12PipelineState>,
              )?;
              state.command_list.SetDescriptorHeaps(&[
                Some(self.srvs[state.frame_index].heap.clone()),
                Some(self.samplers.heap.clone()),
              ]);
              state.command_list.SetGraphicsRootSignature(&self.root);
              state
                .command_list
                .IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
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
            let before = self.meshes.stats();
            let _tessellation_start = profile_scope!();
            #[cfg(feature = "perf_profile")]
            let _tessellation_phase = self
              .profile_context
              .as_ref()
              .and_then(|context| context.detail_phase(crate::app::profiler::Phase::CanvasTessellation));
            let prepared = Prepared::new(&group, &mut self.meshes);
            profile_if! {
              self.profile.tessellation += profile_elapsed!(_tessellation_start);
              self.profile.command_groups += 1;
              drop(_tessellation_phase);
            }
            canvas.record_mesh_cache(before, self.meshes.stats());
            match prepared {
              Ok(prepared) => self.draw(state, canvas.surface_id(), &prepared)?,
              Err(error) => canvas.set_gpu_error(error),
            }
          }
        }
      }
    }
    #[cfg(feature = "perf_profile")]
    let _eviction_start = CanvasAssetUploadProfile::start_timer(self.profile.asset_upload_details.as_ref());
    while self.asset_bytes > 64 * 1024 * 1024 {
      let Some(id) = self.assets.iter().min_by_key(|(_, a)| a.last).map(|(id, _)| *id) else {
        break;
      };
      let asset = self.assets.remove(&id).unwrap();
      self.asset_bytes -= asset.bytes;
      state.canvas_retired[state.frame_index].push(asset.texture);
      profile_if! {
        if let Some(detail) = self.profile.asset_upload_details.as_mut() {
          detail.cache_evictions += 1;
        }
      }
    }
    profile_if! {
      if let Some(detail) = self.profile.asset_upload_details.as_mut() {
        detail.add_stage(AssetUploadStage::CacheEviction, _eviction_start);
        detail.cache_state(self.asset_bytes, self.assets.len());
      }
    }
    profile_if! { self.profile.total = profile_elapsed!(_process_start); }
    Ok(())
  }
  pub fn submitted(&mut self) {
    self.flush_stats();
    for batch in self.batches.drain(..) {
      batch.submit();
    }
  }
  fn flush_stats(&mut self) {
    for (owner, vertices, tiles, uploaded) in self.pending_stats.drain(..) {
      if let Some(canvas) = owner.upgrade() {
        canvas.record_gpu_update(vertices, tiles, uploaded);
      }
    }
  }
  pub fn finish_readbacks(&mut self, fence: &ID3D12Fence, value: u64) {
    for readback in self.readbacks.drain(..) {
      let fence = fence.clone();
      std::thread::spawn(move || unsafe {
        let result = (|| -> Result<CanvasSnapshot> {
          let event = CreateEventW(None, false, false, None)?;
          let wait = (|| -> Result<()> {
            fence.SetEventOnCompletion(value, event)?;
            if WaitForSingleObject(event, 30_000) != WAIT_OBJECT_0 {
              return Err(dx12_invalid_arg("canvas readback fence timeout".to_owned()));
            }
            Ok(())
          })();
          let _ = CloseHandle(event);
          wait?;
          let size = readback.pitch as usize * readback.height as usize;
          let mut mapped = ptr::null_mut();
          readback
            .buffer
            .Map(0, Some(&D3D12_RANGE { Begin: 0, End: size }), Some(&mut mapped))?;
          let source = std::slice::from_raw_parts(mapped.cast::<u8>(), size);
          let mut rgba = Vec::with_capacity(readback.width as usize * readback.height as usize * 4);
          for row in source.chunks_exact(readback.pitch as usize) {
            rgba.extend_from_slice(&row[..readback.width as usize * 4]);
          }
          readback.buffer.Unmap(0, Some(&D3D12_RANGE { Begin: 0, End: 0 }));
          Ok(CanvasSnapshot {
            width: readback.width,
            height: readback.height,
            rgba: unpremultiply(rgba),
            revision: readback.revision,
          })
        })();
        readback.done.finish(result.map_err(|_| CanvasError::RendererLost));
      });
    }
  }
  unsafe fn srv(&mut self, state: &Dx12State, texture: &ID3D12Resource) -> Result<D3D12_GPU_DESCRIPTOR_HANDLE> {
    self.srv_pair(state, texture, texture)
  }
  /// Two adjacent descriptors, because the root signature's table is a pair.
  /// Only the blend composite reads the second one.
  unsafe fn srv_pair(
    &mut self,
    state: &Dx12State,
    texture: &ID3D12Resource,
    backdrop: &ID3D12Resource,
  ) -> Result<D3D12_GPU_DESCRIPTOR_HANDLE> {
    if self.descriptor + 2 > 16384 {
      return Err(dx12_invalid_arg("canvas descriptor budget exceeded".to_owned()));
    }
    let heap = &self.srvs[state.frame_index];
    let index = self.descriptor;
    self.descriptor += 2;
    create_srv(&state.device, texture, heap.cpu_handle(index));
    create_srv(&state.device, backdrop, heap.cpu_handle(index + 1));
    Ok(heap.gpu_handle(index))
  }
  unsafe fn resize(
    &mut self,
    state: &mut Dx12State,
    canvas: &CanvasHandle,
    width: u32,
    height: u32,
    preserve: bool,
  ) -> Result<()> {
    let id = canvas.surface_id();
    let old = self.surfaces.remove(&id);
    if width == 0 || height == 0 {
      if let Some(old) = old {
        state.canvas_retired[state.frame_index].push(old.texture);
      }
      canvas.set_gpu_bytes(0);
      return Ok(());
    }
    let next = texture(
      &state.device,
      width,
      height,
      1,
      false,
      D3D12_RESOURCE_STATE_RENDER_TARGET,
    )?;
    let rtv = self.rtvs.cpu_handle(1);
    state.device.CreateRenderTargetView(&next, None, rtv);
    state.command_list.OMSetRenderTargets(1, Some(&rtv), false, None);
    state.command_list.ClearRenderTargetView(rtv, &[0.; 4], None);
    if let Some(old) = old {
      if preserve {
        let srv = self.srv(state, &old.texture)?;
        let globals = state.upload_frame_constant(&[
          0f32,
          0.,
          width as f32,
          height as f32,
          width as f32,
          height as f32,
          0.,
          0.,
        ])?;
        viewport(&state.command_list, width, height);
        state.command_list.SetPipelineState(&self.resample);
        state
          .command_list
          .SetGraphicsRootConstantBufferView(0, globals.gpu_address);
        state.command_list.SetGraphicsRootDescriptorTable(1, srv);
        state
          .command_list
          .SetGraphicsRootDescriptorTable(2, self.samplers.gpu_handle(1));
        state.command_list.DrawInstanced(3, 1, 0, 0);
      }
      state.canvas_retired[state.frame_index].push(old.texture);
    }
    state.transition_resource(
      &next,
      D3D12_RESOURCE_STATE_RENDER_TARGET,
      D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
    );
    self.surfaces.insert(
      id,
      Backing {
        owner: canvas.downgrade(),
        texture: next,
        width,
        height,
      },
    );
    canvas.set_gpu_bytes(width as usize * height as usize * 4);
    Ok(())
  }
  /// Tile-sized copies an isolated layer needs: `saved[d]` is what was under the
  /// layer opened at depth `d`, and one shared `layer` holds the finished layer
  /// being composited. Allocated on first use, so a canvas that never opens a
  /// layer pays nothing; each is TILE × TILE × 4 bytes.
  unsafe fn reserve_layers(&mut self, device: &ID3D12Device, depth: usize) -> Result<()> {
    while self.saved.len() < depth {
      self.saved.push(texture(
        device,
        TILE,
        TILE,
        1,
        false,
        D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
      )?);
    }
    if depth > 0 && self.layer.is_none() {
      self.layer = Some(texture(
        device,
        TILE,
        TILE,
        1,
        false,
        D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
      )?);
    }
    Ok(())
  }

  /// Resolves the multisampled tile into `self.resolve`, which the caller then
  /// copies wherever this segment belongs.
  unsafe fn resolve_tile(&self, state: &mut Dx12State) {
    state.transition_resource(
      &self.scratch,
      D3D12_RESOURCE_STATE_RENDER_TARGET,
      D3D12_RESOURCE_STATE_RESOLVE_SOURCE,
    );
    state.transition_resource(
      &self.resolve,
      D3D12_RESOURCE_STATE_COPY_SOURCE,
      D3D12_RESOURCE_STATE_RESOLVE_DEST,
    );
    state
      .command_list
      .ResolveSubresource(&self.resolve, 0, &self.scratch, 0, DXGI_FORMAT_R8G8B8A8_UNORM);
    state.transition_resource(
      &self.scratch,
      D3D12_RESOURCE_STATE_RESOLVE_SOURCE,
      D3D12_RESOURCE_STATE_RENDER_TARGET,
    );
    state.transition_resource(
      &self.resolve,
      D3D12_RESOURCE_STATE_RESOLVE_DEST,
      D3D12_RESOURCE_STATE_COPY_SOURCE,
    );
  }

  unsafe fn copy_resolved(
    &self,
    state: &mut Dx12State,
    target: &ID3D12Resource,
    from: D3D12_RESOURCE_STATES,
    at: (u32, u32),
    tile: [u32; 4],
  ) {
    state.transition_resource(target, from, D3D12_RESOURCE_STATE_COPY_DEST);
    let mut src = copy_location(&self.resolve);
    let mut dst = copy_location(target);
    state.command_list.CopyTextureRegion(
      &dst,
      at.0,
      at.1,
      0,
      &src,
      Some(&D3D12_BOX {
        left: 0,
        top: 0,
        front: 0,
        right: tile[2],
        bottom: tile[3],
        back: 1,
      }),
    );
    ManuallyDrop::drop(&mut src.pResource);
    ManuallyDrop::drop(&mut dst.pResource);
    state.transition_resource(target, D3D12_RESOURCE_STATE_COPY_DEST, from);
  }
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
