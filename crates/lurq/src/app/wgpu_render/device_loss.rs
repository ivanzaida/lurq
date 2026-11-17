//! A device that is lost: its card was reset (a driver that hung, or one that was updated) or taken away.
//!
//! Nothing is drawn with such a device again, and wgpu answers what is asked of it with errors, which end the
//! program unless something takes them. An engine that made its own device watches it ([`watch`]): once it is
//! lost the engine asks nothing more of it ([`is_lost`]), an error that came of the loss ends nothing, and the
//! engine says that it is lost (`RenderEngine::lost`), for the tree to make another in its place, with a device
//! of its own.
//!
//! Direct3D 12 has one device for a card in a program, whatever asks for it: the engines of all the windows
//! lose it together. Direct3D says that the card gives no other while anything holds the lost one; a card
//! this was run on gave one all the same. Both are taken: a tree lets go of the lost engines of all its
//! windows before another device is asked for, and after a loss a device that is not given is asked for
//! again later ([`happened`]) and ends nothing, as it does where the program starts on a machine with no
//! card that can draw.

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

/// Whether a device was lost in this program.
static HAPPENED: AtomicBool = AtomicBool::new(false);

/// Whether a device was lost in this program: from then on a device that is not given is asked for again.
pub(super) fn happened() -> bool {
  HAPPENED.load(Ordering::Acquire)
}

fn say(lost: &AtomicBool) {
  lost.store(true, Ordering::Release);
  HAPPENED.store(true, Ordering::Release);
}

/// Has `lost` say when `device` is lost, and has an error of a device that is lost end nothing. Any other
/// error ends the program as it did, for it is a fault of the engine's.
pub(super) fn watch(device: &wgpu::Device, lost: &Arc<AtomicBool>) {
  device.set_device_lost_callback({
    let lost = lost.clone();
    move |_, _| say(&lost)
  });
  let removed = removed_asker(device);
  let lost = lost.clone();
  device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
    // wgpu may say what failed before it says that the device is lost: the card is asked itself.
    if lost.load(Ordering::Acquire) || removed() {
      say(&lost);
      return;
    }
    panic!("wgpu error: {error}");
  }));
}

/// What `make` makes of `device`, or `None` where the device refuses it: something it was given is not its
/// own. A view an app made before a loss is of the lost device, and the device that takes its place refuses
/// it, which would end the program as a fault of the engine's does; here it ends nothing, and what would
/// have been drawn of it is not drawn. In a browser nothing is waited for, and `make` is as it was.
pub(super) fn taken<T>(device: &wgpu::Device, make: impl FnOnce() -> T) -> Option<T> {
  #[cfg(not(target_arch = "wasm32"))]
  {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let made = make();
    pollster::block_on(scope.pop()).is_none().then_some(made)
  }
  #[cfg(target_arch = "wasm32")]
  {
    let _ = device;
    Some(make())
  }
}

/// Whether `device` is lost, as `lost` was told or as its card says now.
pub(super) fn is_lost(device: &wgpu::Device, lost: &AtomicBool) -> bool {
  if lost.load(Ordering::Acquire) {
    return true;
  }
  let removed = removed_asker(device)();
  if removed {
    say(lost);
  }
  removed
}

/// Asks the card behind `device` whether the device was removed. Direct3D 12 says so at once; elsewhere it is
/// wgpu that tells of a loss, when it finds it.
#[cfg(windows)]
fn removed_asker(device: &wgpu::Device) -> impl Fn() -> bool + Send + Sync + 'static {
  // SAFETY: the device's own handle is read, and kept to ask it why it was removed, which changes nothing.
  let raw = unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }.map(|hal| hal.raw_device().clone());
  move || {
    raw
      .as_ref()
      // SAFETY: the device is asked a question.
      .is_some_and(|raw| unsafe { raw.GetDeviceRemovedReason() }.is_err())
  }
}

#[cfg(not(windows))]
fn removed_asker(_device: &wgpu::Device) -> impl Fn() -> bool + Send + Sync + 'static {
  || false
}

#[cfg(test)]
mod refused {
  /// Two devices of one card, as the one that was lost and the one that takes its place are.
  fn two_devices() -> Option<(wgpu::Device, wgpu::Device)> {
    let instance = super::super::new_instance(super::super::default_backends());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
    let device = || {
      pollster::block_on(adapter.request_device(&super::super::device_descriptor(
        &adapter,
        wgpu::Features::empty(),
        &wgpu::Limits::default(),
      )))
      .ok()
      .map(|(device, _)| device)
    };
    Some((device()?, device()?))
  }

  fn view(device: &wgpu::Device) -> wgpu::TextureView {
    device
      .create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
          width: 4,
          height: 4,
          depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
      })
      .create_view(&Default::default())
  }

  #[test]
  fn a_view_of_another_device_is_refused_and_ends_nothing() {
    let Some((first, second)) = two_devices() else {
      eprintln!("skipped: no card to make two devices on");
      return;
    };
    let layout = second.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: None,
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
          sample_type: wgpu::TextureSampleType::Float { filterable: true },
          view_dimension: wgpu::TextureViewDimension::D2,
          multisampled: false,
        },
        count: None,
      }],
    });
    let bound = |view: &wgpu::TextureView| {
      let bind = || {
        second.create_bind_group(&wgpu::BindGroupDescriptor {
          label: None,
          layout: &layout,
          entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(view),
          }],
        })
      };
      super::taken(&second, bind)
    };
    // A view an app made with the device that was lost, given to the device that takes its place.
    assert!(bound(&view(&first)).is_none(), "a view of another device is bound");
    // A view of the device's own is bound as ever, before and after one that was refused.
    assert!(bound(&view(&second)).is_some());
  }
}

#[cfg(all(test, windows))]
mod tests {
  use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  };

  use windows::{Win32::Graphics::Direct3D12::ID3D12Device5, core::Interface};

  /// Run by itself (`--ignored`): Direct3D 12 has one device for a card in a program, and removing it takes it
  /// from every test that draws at the same time.
  #[test]
  #[ignore = "removes the card's device from the whole test program: run alone, with --ignored"]
  fn a_device_that_is_removed_is_lost_and_its_errors_end_nothing() {
    let instance = wgpu::Instance::new(super::super::instance_descriptor(wgpu::Backends::DX12));
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
      eprintln!("skipped: no card with Direct3D 12 here");
      return;
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&super::super::device_descriptor(
      &adapter,
      wgpu::Features::empty(),
      &wgpu::Limits::default(),
    )))
    .expect("a device");
    let lost = Arc::new(AtomicBool::new(false));
    super::watch(&device, &lost);
    assert!(!super::is_lost(&device, &lost));

    // SAFETY: the device of this test's own is removed, as a card that is reset removes it.
    unsafe {
      device
        .as_hal::<wgpu::hal::api::Dx12>()
        .expect("a device of Direct3D 12")
        .raw_device()
        .cast::<ID3D12Device5>()
        .expect("a device that can be removed")
        .RemoveDevice();
    }
    // wgpu has not been asked anything yet, and so has told of nothing: the card says it itself.
    assert!(super::is_lost(&device, &lost));
    assert!(lost.load(Ordering::Acquire));

    // What is asked of the device now fails, and ends nothing.
    let texture = device.create_texture(&wgpu::TextureDescriptor {
      label: None,
      size: wgpu::Extent3d {
        width: 16,
        height: 16,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Rgba8Unorm,
      usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
      view_formats: &[],
    });
    let _view = texture.create_view(&Default::default());
    queue.submit([device.create_command_encoder(&Default::default()).finish()]);
    let _ = device.poll(wgpu::PollType::Poll);
  }

  /// Run by itself, as the test above: it removes the card's device.
  #[test]
  #[ignore = "removes the card's device from the whole test program: run alone, with --ignored"]
  fn a_device_made_ahead_is_made_again_for_all_that_share_it() {
    let Ok(given) = pollster::block_on(super::super::WgpuDevice::request_with(wgpu::Backends::DX12)) else {
      eprintln!("skipped: no card with Direct3D 12 here");
      return;
    };
    let shared = given.clone();
    let first = given.current().expect("the device that was asked for");
    assert!(!super::is_lost(&first.device, &first.lost));
    // SAFETY: the device of this test's own is removed, as a card that is reset removes it.
    unsafe {
      first
        .device
        .as_hal::<wgpu::hal::api::Dx12>()
        .expect("a device of Direct3D 12")
        .raw_device()
        .cast::<ID3D12Device5>()
        .expect("a device that can be removed")
        .RemoveDevice();
    }
    assert!(super::is_lost(&first.device, &first.lost));
    // Asked for while the lost one is held still: Direct3D says the card gives none then, and this one may
    // give one all the same. Nothing ends, and what is given is not the lost one.
    if let Some(held) = shared.current() {
      assert!(!super::is_lost(&held.device, &held.lost));
    }
    drop(first);
    // Let go of, another is made, once: what shares the device has the same one.
    let again = shared
      .current()
      .expect("another device, once the lost one is let go of");
    assert!(!super::is_lost(&again.device, &again.lost));
    let same = given.current().expect("the device, for what shares it");
    assert!(
      std::sync::Arc::ptr_eq(&again.lost, &same.lost),
      "one device for all that share it"
    );
    // And it draws: a texture is made and written.
    let texture = again.device.create_texture(&wgpu::TextureDescriptor {
      label: None,
      size: wgpu::Extent3d {
        width: 4,
        height: 4,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::Rgba8Unorm,
      usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
      view_formats: &[],
    });
    again.queue.write_texture(
      texture.as_image_copy(),
      &[7_u8; 64],
      wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(16),
        rows_per_image: None,
      },
      texture.size(),
    );
    again.queue.submit([]);
    again
      .device
      .poll(wgpu::PollType::wait_indefinitely())
      .expect("the card answers");
    assert!(!super::is_lost(&again.device, &again.lost));
  }
}
