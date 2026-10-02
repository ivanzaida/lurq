use super::*;

#[test]
fn canvas_sampled_asset_has_copy_and_sampling_format_without_target_metadata() {
  // Small and wide textures must use the same sampled policy: the change is
  // capability selection, not an unmeasured small-allocation/cache heuristic.
  for (width, height) in [(1, 1), (769, 3), (8192, 128)] {
    let (desc, clear) = texture_definition(width, height, 1, TextureUsage::Sampled);
    assert_eq!(desc.Dimension, D3D12_RESOURCE_DIMENSION_TEXTURE2D);
    assert_eq!((desc.Width, desc.Height), (u64::from(width), height));
    assert_eq!((desc.DepthOrArraySize, desc.MipLevels), (1, 1));
    assert_eq!(desc.Format, DXGI_FORMAT_R8G8B8A8_UNORM);
    assert_eq!((desc.SampleDesc.Count, desc.SampleDesc.Quality), (1, 0));
    assert_eq!(desc.Layout, D3D12_TEXTURE_LAYOUT_UNKNOWN);
    assert_eq!(desc.Flags, D3D12_RESOURCE_FLAG_NONE);
    assert_eq!(desc.Alignment, 0);
    assert!(clear.is_none(), "sampled assets cannot use target fast-clear metadata");
  }
}

#[test]
fn canvas_sampled_asset_change_preserves_color_and_stencil_target_contracts() {
  for samples in [1, 4] {
    let (color, clear) = texture_definition(TILE, TILE, samples, TextureUsage::RenderTarget);
    assert_eq!(color.Format, DXGI_FORMAT_R8G8B8A8_UNORM);
    assert_eq!(color.Flags, D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET);
    assert_eq!(color.SampleDesc.Count, samples);
    let clear = clear.expect("existing color targets retain optimized zero clear");
    assert_eq!(clear.Format, color.Format);
    assert_eq!(unsafe { clear.Anonymous.Color }, [0.; 4]);

    let (depth, clear) = texture_definition(TILE, TILE, samples, TextureUsage::DepthStencil);
    assert_eq!(depth.Format, DXGI_FORMAT_D24_UNORM_S8_UINT);
    assert_eq!(depth.Flags, D3D12_RESOURCE_FLAG_ALLOW_DEPTH_STENCIL);
    assert_eq!(depth.SampleDesc.Count, samples);
    let clear = clear.expect("existing stencil targets retain depth clear");
    assert_eq!(clear.Format, depth.Format);
    let value = unsafe { clear.Anonymous.DepthStencil };
    assert_eq!((value.Depth, value.Stencil), (1.0, 0));
  }
}
