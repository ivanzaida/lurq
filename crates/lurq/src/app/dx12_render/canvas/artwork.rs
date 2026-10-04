//! Texture-only artwork reuse; resources remain on the existing fenced command queue.
use super::*;
impl Renderer {
  pub(super) unsafe fn forget_artwork(&mut self, state: &mut Dx12State, canvas: &CanvasHandle) {
    let id = canvas.surface_id();
    for back in self
      .surfaces
      .get_mut(&id)
      .into_iter()
      .chain(self.fronts.get_mut(&id).map(|(_, b)| b))
    {
      if let Some(art) = back.artwork.take() {
        self.retired_artwork.push(ArtworkLease {
          fence: state.next_fence_value,
          art,
        });
      }
    }
    self.account_presentation(state, canvas);
  }

  pub(super) unsafe fn capture_artwork(&mut self, state: &mut Dx12State, canvas: &CanvasHandle) -> Result<()> {
    let id = canvas.surface_id();
    let bytes = self.surfaces.get(&id).map_or(0, |back| {
      resources::target_allocation_bytes(&state.device, back.width, back.height, true)
    });
    if !self.allocation_admitted(state, bytes) {
      canvas.set_gpu_error(CanvasError::PresentationBusy);
      return Ok(());
    }
    let Some(back) = self.surfaces.get_mut(&id) else {
      return Ok(());
    };
    let next = resources::asset_texture(&state.device, back.width, back.height)?;
    state.transition_resource(
      &back.texture,
      D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
      D3D12_RESOURCE_STATE_COPY_SOURCE,
    );
    state.command_list.CopyResource(&next, &back.texture);
    state.transition_resource(
      &back.texture,
      D3D12_RESOURCE_STATE_COPY_SOURCE,
      D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
    );
    state.transition_resource(
      &next,
      D3D12_RESOURCE_STATE_COPY_DEST,
      D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
    );
    if let Some(old) = back.artwork.replace(std::sync::Arc::new(Artwork {
      bytes: resources::target_allocation_bytes(&state.device, back.width, back.height, true),
      texture: next,
      width: back.width,
      height: back.height,
    })) {
      self.retired_artwork.push(ArtworkLease {
        fence: state.next_fence_value,
        art: old,
      });
    }
    self.account_presentation(state, canvas);
    Ok(())
  }
  pub(super) unsafe fn draw_artwork(
    &mut self,
    state: &mut Dx12State,
    canvas: &CanvasHandle,
    matrix: crate::node::transform::Transform2D,
  ) -> Result<()> {
    let Some(back) = self.surfaces.get(&canvas.surface_id()) else {
      return Ok(());
    };
    let Some(artwork) = back.artwork.clone() else {
      canvas.set_gpu_error(CanvasError::StateLimit);
      return Ok(());
    };
    let target = back.texture.clone();
    let (width, height) = (back.width, back.height);
    let vertices = state.upload_frame_pod_slice(&artwork_vertices(artwork.width, artwork.height, matrix), 16)?;
    let globals =
      state.upload_frame_constant(&[0., 0., width as f32, height as f32, width as f32, height as f32, 0., 0.])?;
    let srv = self.srv(state, &artwork.texture)?;
    let rtv = self.rtvs.cpu_handle(1);
    state.device.CreateRenderTargetView(&target, None, rtv);
    state.transition_resource(
      &target,
      D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
      D3D12_RESOURCE_STATE_RENDER_TARGET,
    );
    state.command_list.OMSetRenderTargets(1, Some(&rtv), false, None);
    viewport(&state.command_list, width, height);
    state.command_list.RSSetScissorRects(&[RECT {
      left: 0,
      top: 0,
      right: width as i32,
      bottom: height as i32,
    }]);
    state
      .command_list
      .SetGraphicsRootConstantBufferView(0, globals.gpu_address);
    state.command_list.SetGraphicsRootDescriptorTable(1, srv);
    state
      .command_list
      .SetGraphicsRootDescriptorTable(2, self.samplers.gpu_handle(1));
    state
      .command_list
      .IASetVertexBuffers(0, Some(&[vertices.vertex_view::<Vertex>()]));
    state.command_list.SetPipelineState(&self.retained);
    state.command_list.DrawInstanced(6, 1, 0, 0);
    state.transition_resource(
      &target,
      D3D12_RESOURCE_STATE_RENDER_TARGET,
      D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE,
    );
    canvas.presentation_event(PresentationEvent::Quad);
    Ok(())
  }
}
