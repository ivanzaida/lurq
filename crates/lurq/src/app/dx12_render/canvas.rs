//! Native offscreen canvas work on the UI renderer's command queue.
use std::collections::VecDeque;

use windows::Win32::Graphics::{Direct3D12::*, Dxgi::Common::*};

use super::*;
use crate::canvas::{CanvasError, CanvasHandle, CanvasId, CanvasSnapshot, CanvasWeak, gpu::*};

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
  batches: Vec<Batch>,
  readbacks: Vec<Readback>,
  pending_stats: Vec<(CanvasWeak, usize, usize, usize)>,
}
impl Renderer {
  pub unsafe fn new(device: &ID3D12Device) -> Result<Self> {
    let root = create_image_root_signature(device, 1)?;
    let seed = pipeline(device, &root, b"vs_seed\0", b"ps_premul\0", 4, 0)?;
    let resample = pipeline(device, &root, b"vs_seed\0", b"ps_premul\0", 1, 0)?;
    let clip = pipeline(device, &root, b"vs_main\0", b"ps_solid\0", 4, 2)?;
    let solid = pipeline(device, &root, b"vs_main\0", b"ps_solid\0", 4, 3)?;
    let image = pipeline(device, &root, b"vs_main\0", b"ps_premul\0", 4, 3)?;
    let erase = pipeline(device, &root, b"vs_main\0", b"ps_solid\0", 4, 4)?;
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
      batches: Vec::new(),
      readbacks: Vec::new(),
      pending_stats: Vec::new(),
    })
  }
  pub fn backing(&self, canvas: &CanvasHandle) -> Option<ID3D12Resource> {
    self.surfaces.get(&canvas.surface_id()).map(|b| b.texture.clone())
  }
  pub unsafe fn encode(&mut self, state: &mut Dx12State, canvases: &[CanvasHandle]) -> Result<()> {
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
              state
                .command_queue
                .ExecuteCommandLists(&[Some(state.command_list.cast()?)]);
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
            match Prepared::new(&group) {
              Ok(prepared) => self.draw(state, canvas.surface_id(), &prepared)?,
              Err(error) => canvas.set_gpu_error(error),
            }
          }
        }
      }
    }
    while self.asset_bytes > 64 * 1024 * 1024 {
      let Some(id) = self.assets.iter().min_by_key(|(_, a)| a.last).map(|(id, _)| *id) else {
        break;
      };
      let asset = self.assets.remove(&id).unwrap();
      self.asset_bytes -= asset.bytes;
      state.canvas_retired[state.frame_index].push(asset.texture);
    }
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
    if self.descriptor >= 16384 {
      return Err(dx12_invalid_arg("canvas descriptor budget exceeded".to_owned()));
    }
    let heap = &self.srvs[state.frame_index];
    let index = self.descriptor;
    self.descriptor += 1;
    create_srv(&state.device, texture, heap.cpu_handle(index));
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
  unsafe fn draw(&mut self, state: &mut Dx12State, id: CanvasId, prepared: &Prepared) -> Result<()> {
    let Some(b) = self.surfaces.get(&id) else {
      return Ok(());
    };
    let (backing, width, height) = (b.texture.clone(), b.width, b.height);
    let seed_srv = self.srv(state, &backing)?;
    let mut assets = HashMap::new();
    let mut uploaded = 0;
    for draw in &prepared.draws {
      if let Some(asset) = &draw.asset {
        if !self.assets.contains_key(&asset.id) {
          let texture = texture(
            &state.device,
            asset.width,
            asset.height,
            1,
            false,
            D3D12_RESOURCE_STATE_COPY_DEST,
          )?;
          upload_asset(state, &texture, asset)?;
          let bytes = asset.data.len().max(64 * 1024);
          self.asset_bytes += bytes;
          uploaded += asset.data.len();
          self.assets.insert(
            asset.id,
            AssetTexture {
              texture,
              bytes,
              last: self.tick,
            },
          );
        }
        let cached = self.assets.get_mut(&asset.id).unwrap();
        cached.last = self.tick;
        let texture = cached.texture.clone();
        if let std::collections::hash_map::Entry::Vacant(entry) = assets.entry(asset.id) {
          entry.insert(self.srv(state, &texture)?);
        }
      }
    }
    if prepared.clear {
      let rtv = self.rtvs.cpu_handle(1);
      state.device.CreateRenderTargetView(&backing, None, rtv);
      state.transition_resource(
        &backing,
        D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
        D3D12_RESOURCE_STATE_RENDER_TARGET,
      );
      state.command_list.ClearRenderTargetView(rtv, &[0.; 4], None);
      state.transition_resource(
        &backing,
        D3D12_RESOURCE_STATE_RENDER_TARGET,
        D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
      );
    }
    let tiles = prepared.tiles(width, height);
    self.pending_stats.push((
      self.surfaces[&id].owner.clone(),
      prepared.vertices.len(),
      tiles.len(),
      uploaded,
    ));
    if prepared.vertices.is_empty() {
      return Ok(());
    }
    let vertices = state.upload_frame_pod_slice(&prepared.vertices, 16)?;
    let rtv = self.rtvs.cpu_handle(0);
    let dsv = self.dsv.cpu_handle(0);
    for tile in tiles {
      let globals = state.upload_frame_constant(&[
        tile[0] as f32,
        tile[1] as f32,
        TILE as f32,
        TILE as f32,
        width as f32,
        height as f32,
        0.,
        0.,
      ])?;
      state.command_list.OMSetRenderTargets(1, Some(&rtv), false, Some(&dsv));
      viewport(&state.command_list, TILE, TILE);
      state.command_list.ClearRenderTargetView(rtv, &[0.; 4], None);
      state.command_list.RSSetScissorRects(&[RECT {
        left: 0,
        top: 0,
        right: tile[2] as i32,
        bottom: tile[3] as i32,
      }]);
      state
        .command_list
        .SetGraphicsRootConstantBufferView(0, globals.gpu_address);
      state.command_list.SetGraphicsRootDescriptorTable(1, seed_srv);
      state
        .command_list
        .SetGraphicsRootDescriptorTable(2, self.samplers.gpu_handle(0));
      state.command_list.SetPipelineState(&self.seed);
      state.command_list.DrawInstanced(3, 1, 0, 0);
      state
        .command_list
        .IASetVertexBuffers(0, Some(&[vertices.vertex_view::<Vertex>()]));
      let mut clips: Option<&Vec<std::ops::Range<u32>>> = None;
      for draw in prepared.draws.iter().filter(|d| d.intersects(tile)) {
        if clips != Some(&draw.clips) {
          state
            .command_list
            .ClearDepthStencilView(dsv, D3D12_CLEAR_FLAG_STENCIL, 1., 0, None);
          state.command_list.SetPipelineState(&self.clip);
          for (level, range) in draw.clips.iter().enumerate() {
            state.command_list.OMSetStencilRef(level as u32);
            state
              .command_list
              .DrawInstanced(range.end - range.start, 1, range.start, 0);
          }
          clips = Some(&draw.clips);
        }
        state.command_list.OMSetStencilRef(draw.clips.len() as u32);
        state.command_list.SetPipelineState(if draw.erase {
          &self.erase
        } else if draw.asset.is_some() {
          &self.image
        } else {
          &self.solid
        });
        if let Some(asset) = &draw.asset {
          state.command_list.SetGraphicsRootDescriptorTable(1, assets[&asset.id]);
          state
            .command_list
            .SetGraphicsRootDescriptorTable(2, self.samplers.gpu_handle(usize::from(draw.smooth)));
        }
        state
          .command_list
          .DrawInstanced(draw.vertices.end - draw.vertices.start, 1, draw.vertices.start, 0);
      }
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
      state.transition_resource(
        &backing,
        D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
        D3D12_RESOURCE_STATE_COPY_DEST,
      );
      let mut src = copy_location(&self.resolve);
      let mut dst = copy_location(&backing);
      state.command_list.CopyTextureRegion(
        &dst,
        tile[0],
        tile[1],
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
      state.transition_resource(
        &backing,
        D3D12_RESOURCE_STATE_COPY_DEST,
        D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
      );
    }
    Ok(())
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
pub(super) unsafe fn create_srv(device: &ID3D12Device, texture: &ID3D12Resource, handle: D3D12_CPU_DESCRIPTOR_HANDLE) {
  device.CreateShaderResourceView(
    texture,
    Some(&D3D12_SHADER_RESOURCE_VIEW_DESC {
      Format: DXGI_FORMAT_R8G8B8A8_UNORM,
      ViewDimension: D3D12_SRV_DIMENSION_TEXTURE2D,
      Shader4ComponentMapping: D3D12_DEFAULT_SHADER_4_COMPONENT_MAPPING,
      Anonymous: D3D12_SHADER_RESOURCE_VIEW_DESC_0 {
        Texture2D: D3D12_TEX2D_SRV {
          MostDetailedMip: 0,
          MipLevels: 1,
          PlaneSlice: 0,
          ResourceMinLODClamp: 0.,
        },
      },
    }),
    handle,
  );
}
unsafe fn texture(
  device: &ID3D12Device,
  width: u32,
  height: u32,
  samples: u32,
  stencil: bool,
  initial: D3D12_RESOURCE_STATES,
) -> Result<ID3D12Resource> {
  let desc = D3D12_RESOURCE_DESC {
    Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
    Width: width as u64,
    Height: height,
    DepthOrArraySize: 1,
    MipLevels: 1,
    Format: if stencil {
      DXGI_FORMAT_D24_UNORM_S8_UINT
    } else {
      DXGI_FORMAT_R8G8B8A8_UNORM
    },
    SampleDesc: DXGI_SAMPLE_DESC {
      Count: samples,
      Quality: 0,
    },
    Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
    Flags: if stencil {
      D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL
    } else {
      D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET
    },
    ..Default::default()
  };
  let clear = D3D12_CLEAR_VALUE {
    Format: desc.Format,
    Anonymous: if stencil {
      D3D12_CLEAR_VALUE_0 {
        DepthStencil: D3D12_DEPTH_STENCIL_VALUE { Depth: 1., Stencil: 0 },
      }
    } else {
      D3D12_CLEAR_VALUE_0 { Color: [0.; 4] }
    },
  };
  let mut resource = None;
  device.CreateCommittedResource(
    &D3D12_HEAP_PROPERTIES {
      Type: D3D12_HEAP_TYPE_DEFAULT,
      CreationNodeMask: 1,
      VisibleNodeMask: 1,
      ..Default::default()
    },
    D3D12_HEAP_FLAG_NONE,
    &desc,
    initial,
    Some(&clear),
    &mut resource,
  )?;
  resource.ok_or_else(Error::from_win32)
}
unsafe fn viewport(list: &ID3D12GraphicsCommandList, width: u32, height: u32) {
  list.RSSetViewports(&[D3D12_VIEWPORT {
    Width: width as f32,
    Height: height as f32,
    MaxDepth: 1.,
    ..Default::default()
  }]);
  list.RSSetScissorRects(&[RECT {
    left: 0,
    top: 0,
    right: width as i32,
    bottom: height as i32,
  }]);
}
fn copy_location(resource: &ID3D12Resource) -> D3D12_TEXTURE_COPY_LOCATION {
  D3D12_TEXTURE_COPY_LOCATION {
    pResource: ManuallyDrop::new(Some(resource.clone())),
    Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
    Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 { SubresourceIndex: 0 },
  }
}
unsafe fn upload_asset(state: &mut Dx12State, texture: &ID3D12Resource, asset: &Asset) -> Result<()> {
  let pitch = (asset.width * 4).div_ceil(256) * 256;
  let mut data = vec![0u8; pitch as usize * asset.height as usize];
  for (src, dst) in asset
    .data
    .chunks_exact(asset.width as usize * 4)
    .zip(data.chunks_exact_mut(pitch as usize))
  {
    dst[..src.len()].copy_from_slice(src);
    if !asset.premultiplied {
      for p in dst[..src.len()].chunks_exact_mut(4) {
        let a = u16::from(p[3]);
        for c in &mut p[..3] {
          *c = ((u16::from(*c) * a + 127) / 255) as u8;
        }
      }
    }
  }
  let upload = state.upload_frame_bytes(&data, 512)?;
  let mut dst = copy_location(texture);
  let mut src = D3D12_TEXTURE_COPY_LOCATION {
    pResource: ManuallyDrop::new(Some(upload.resource)),
    Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
    Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
      PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
        Offset: upload.offset,
        Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
          Format: DXGI_FORMAT_R8G8B8A8_UNORM,
          Width: asset.width,
          Height: asset.height,
          Depth: 1,
          RowPitch: pitch,
        },
      },
    },
  };
  state.command_list.CopyTextureRegion(&dst, 0, 0, 0, &src, None);
  ManuallyDrop::drop(&mut src.pResource);
  ManuallyDrop::drop(&mut dst.pResource);
  state.transition_resource(
    texture,
    D3D12_RESOURCE_STATE_COPY_DEST,
    D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
  );
  Ok(())
}
unsafe fn readback(
  state: &mut Dx12State,
  texture: &ID3D12Resource,
  width: u32,
  height: u32,
  done: Completion,
  revision: u64,
) -> Result<Readback> {
  let pitch = (width * 4).div_ceil(256) * 256;
  let mut buffer = None;
  state.device.CreateCommittedResource(
    &D3D12_HEAP_PROPERTIES {
      Type: D3D12_HEAP_TYPE_READBACK,
      CreationNodeMask: 1,
      VisibleNodeMask: 1,
      ..Default::default()
    },
    D3D12_HEAP_FLAG_NONE,
    &buffer_resource_desc(u64::from(pitch) * u64::from(height)),
    D3D12_RESOURCE_STATE_COPY_DEST,
    None,
    &mut buffer,
  )?;
  let buffer: ID3D12Resource = buffer.ok_or_else(Error::from_win32)?;
  let mut src = copy_location(texture);
  let mut dst = D3D12_TEXTURE_COPY_LOCATION {
    pResource: ManuallyDrop::new(Some(buffer.clone())),
    Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
    Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
      PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
        Offset: 0,
        Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
          Format: DXGI_FORMAT_R8G8B8A8_UNORM,
          Width: width,
          Height: height,
          Depth: 1,
          RowPitch: pitch,
        },
      },
    },
  };
  state.transition_resource(
    texture,
    D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
    D3D12_RESOURCE_STATE_COPY_SOURCE,
  );
  state.command_list.CopyTextureRegion(&dst, 0, 0, 0, &src, None);
  ManuallyDrop::drop(&mut src.pResource);
  ManuallyDrop::drop(&mut dst.pResource);
  state.transition_resource(
    texture,
    D3D12_RESOURCE_STATE_COPY_SOURCE,
    D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
  );
  state.canvas_retired[state.frame_index].push(buffer.clone());
  Ok(Readback {
    done,
    buffer,
    width,
    height,
    pitch,
    revision,
  })
}
unsafe fn pipeline(
  device: &ID3D12Device,
  root: &ID3D12RootSignature,
  vs_entry: &'static [u8],
  ps_entry: &'static [u8],
  samples: u32,
  mode: u8,
) -> Result<ID3D12PipelineState> {
  let vs = compile_shader(include_bytes!("shaders/canvas.hlsl"), vs_entry, b"vs_5_0\0")?;
  let ps = compile_shader(include_bytes!("shaders/canvas.hlsl"), ps_entry, b"ps_5_0\0")?;
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
    BlendEnable: if mode >= 3 { TRUE } else { FALSE },
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
    StencilFunc: if mode == 0 {
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
    InputLayout: if vs_entry == b"vs_seed\0" {
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
