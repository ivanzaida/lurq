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
//!
//! What takes the place of a lost device is made on the card the device was lost on. Asked without that,
//! wgpu gives a device of any card that makes one, and while something of the app still holds the lost
//! device its card makes none: on a machine with two cards the window then went to the other one, for
//! good, and the frames of a decoder on the first were copied through memory from then on. So the card is
//! remembered ([`watch`]), asked first and waited for a little ([`the_card_again`]); where another card is
//! taken after all, the engine on it looks for the first one from time to time ([`the_card_gives`]) and
//! says it is to be replaced once that card makes a device again.
//!
//! And a device the card makes while the lost one is still held is not a whole one: on the card this was
//! run on it opened none of the textures a decoder shares, then or later (`E_INVALIDARG`), where a device
//! made after the lost one was let go of opens them. Direct3D tells when a device is destroyed
//! (`ID3DDestructionNotifier`): the lost one is waited for a little, a device made before it is gone is
//! marked, and the engine that draws with a marked device says it is to be replaced once the lost one is
//! gone ([`a_lost_device_is_held`]).

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
  let first = !lost.swap(true, Ordering::AcqRel);
  HAPPENED.store(true, Ordering::Release);
  #[cfg(not(target_arch = "wasm32"))]
  if first {
    card::lost_on_the_card_of(lost);
  }
  #[cfg(target_arch = "wasm32")]
  let _ = first;
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) use card::{Again, a_lost_device_is_held, the_card_again, the_card_gave, the_card_gives};

/// The card a device was lost on, which what takes the device's place is made on.
#[cfg(not(target_arch = "wasm32"))]
mod card {
  use std::{
    sync::{
      Arc, Mutex, Weak,
      atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
  };

  /// A card, as wgpu names it.
  #[derive(Clone, Debug, PartialEq, Eq)]
  struct Card {
    backend: wgpu::Backend,
    vendor: u32,
    device: u32,
    name: String,
  }

  impl Card {
    fn of(adapter: &wgpu::Adapter) -> Self {
      let info = adapter.get_info();
      Self {
        backend: info.backend,
        vendor: info.vendor,
        device: info.device,
        name: info.name,
      }
    }
  }

  /// How long the card a device was lost on is waited for before another card is taken.
  const WAIT: Duration = Duration::from_secs(3);

  /// A device that is watched: what says it is lost, its card, and what says it is destroyed.
  type Watched = (Weak<AtomicBool>, Card, Arc<AtomicBool>);

  /// The card a device was first lost on, when a device was last lost on it, and what says of each device
  /// lost on it that it is destroyed.
  struct Wanted {
    card: Card,
    at: Instant,
    gone: Vec<Arc<AtomicBool>>,
  }

  impl Wanted {
    /// Whether something holds a device that was lost on the card still.
    fn held(&self) -> bool {
      self.gone.iter().any(|gone| !gone.load(Ordering::Acquire))
    }
  }

  static WATCHED: Mutex<Vec<Watched>> = Mutex::new(Vec::new());
  static WANTED: Mutex<Option<Wanted>> = Mutex::new(None);
  /// Whether the card made a device the last time an engine on another card asked it: every engine on
  /// another card is to be replaced then, drawn or not.
  static GIVES: AtomicBool = AtomicBool::new(false);
  /// Has the card a device was lost on make none, as one whose driver is being put in again does: a test
  /// cannot take a card away.
  #[cfg(test)]
  pub(in super::super) static REFUSES: AtomicBool = AtomicBool::new(false);

  /// Remembers which card `device`, which `lost` tells of, is on, and has the system tell when it is
  /// destroyed.
  pub(super) fn remember(device: &wgpu::Device, lost: &Arc<AtomicBool>, adapter: &wgpu::Adapter) {
    let gone = destroyed_teller(device);
    let mut watched = WATCHED.lock().unwrap();
    watched.retain(|(known, ..)| known.strong_count() > 0);
    watched.push((Arc::downgrade(lost), Card::of(adapter), gone));
  }

  /// What says that the device behind `device` is destroyed. Direct3D 12 tells; elsewhere nothing does, and
  /// it is taken to be gone already: nothing is waited for and no device is marked.
  #[cfg(windows)]
  fn destroyed_teller(device: &wgpu::Device) -> Arc<AtomicBool> {
    use windows::{Win32::Graphics::Direct3D::ID3DDestructionNotifier, core::Interface};

    unsafe extern "system" fn destroyed(told: *mut core::ffi::c_void) {
      // SAFETY: `told` is what was given when this was registered, counted for this one call.
      let gone = unsafe { Arc::from_raw(told.cast_const().cast::<AtomicBool>()) };
      gone.store(true, Ordering::Release);
    }

    let gone = Arc::new(AtomicBool::new(false));
    // SAFETY: the device's own handle is read, and asked to call back when it is destroyed.
    let notifier = unsafe { device.as_hal::<wgpu::hal::api::Dx12>() }
      .and_then(|hal| hal.raw_device().cast::<ID3DDestructionNotifier>().ok());
    let told = Arc::into_raw(gone.clone());
    // SAFETY: what is given is counted for the callback, which is called once, and taken back where it is
    // not registered.
    let registered = notifier
      .is_some_and(|notifier| unsafe { notifier.RegisterDestructionCallback(Some(destroyed), told.cast()) }.is_ok());
    if !registered {
      // SAFETY: nothing else has it.
      drop(unsafe { Arc::from_raw(told) });
      gone.store(true, Ordering::Release);
    }
    gone
  }

  #[cfg(not(windows))]
  fn destroyed_teller(_device: &wgpu::Device) -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(true))
  }

  /// Whether something holds a device that was lost still: an engine whose device was made meanwhile is to
  /// be replaced once nothing does.
  pub(in super::super) fn a_lost_device_is_held() -> bool {
    WANTED.lock().unwrap().as_ref().is_some_and(Wanted::held)
  }

  /// The device that `lost` tells of is lost: its card is the one to make another on. A device lost later on
  /// another card, one the window went to meanwhile, changes nothing of that: the first card is still the
  /// one that is wanted.
  pub(super) fn lost_on_the_card_of(lost: &AtomicBool) {
    let watched = WATCHED
      .lock()
      .unwrap()
      .iter()
      .find(|(known, ..)| std::ptr::eq(known.as_ptr(), lost))
      .map(|(_, card, gone)| (card.clone(), gone.clone()));
    let Some((card, gone)) = watched else {
      return;
    };
    let mut wanted = WANTED.lock().unwrap();
    // A device was lost on the card that is wanted: whatever it gave before, it is to be asked again.
    if wanted.as_ref().is_none_or(|wanted| wanted.card == card) {
      GIVES.store(false, Ordering::Release);
    }
    match &mut *wanted {
      Some(wanted) if wanted.card == card => {
        wanted.at = Instant::now();
        wanted.gone.retain(|gone| !gone.load(Ordering::Acquire));
        wanted.gone.push(gone);
      }
      Some(_) => {}
      None => {
        *wanted = Some(Wanted {
          card,
          at: Instant::now(),
          gone: vec![gone],
        });
      }
    }
  }

  /// What the card a device was lost on answers, asked for another.
  pub(in super::super) enum Again {
    /// A device of that card, and whether it was made while something held the lost one still: such a
    /// device is not a whole one, and is made again once the lost one is gone.
    Card(wgpu::Adapter, wgpu::Device, wgpu::Queue, bool),
    /// None yet: the loss is recent, or no other card makes one either. It is asked again later.
    Wait,
    /// None, for too long: a device of another card, from which this one is looked for.
    Elsewhere(wgpu::Adapter, wgpu::Device, wgpu::Queue),
    /// No device was lost on a card that is known: any card that draws is asked, as at the start.
    Unasked,
  }

  /// Asks the card a device was lost on for a device that draws to `surface`, made as `describe` says.
  pub(in super::super) async fn the_card_again(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
    describe: impl Fn(&wgpu::Adapter) -> wgpu::DeviceDescriptor<'static>,
  ) -> Again {
    let wanted = WANTED
      .lock()
      .unwrap()
      .as_ref()
      .map(|wanted| (wanted.card.clone(), wanted.at, wanted.held()));
    let Some((card, at, held)) = wanted else {
      return Again::Unasked;
    };
    // The lost one is let go of in a moment by an app that was told: a device made before that is not whole.
    if held && at.elapsed() < WAIT {
      return Again::Wait;
    }
    match on_the_card(instance, &card, surface, &describe).await {
      Some((adapter, device, queue)) => Again::Card(adapter, device, queue, held),
      None if at.elapsed() < WAIT => Again::Wait,
      None => match on_another_card(instance, &card, surface, &describe).await {
        Some((adapter, device, queue)) => {
          // It made none just now, whatever it answered before.
          GIVES.store(false, Ordering::Release);
          Again::Elsewhere(adapter, device, queue)
        }
        None => Again::Wait,
      },
    }
  }

  /// Whether the card a device was lost on makes a device now. The device is made, for nothing less tells,
  /// and let go of at once.
  pub(in super::super) async fn the_card_gives(
    instance: &wgpu::Instance,
    describe: impl Fn(&wgpu::Adapter) -> wgpu::DeviceDescriptor<'static>,
  ) -> bool {
    let wanted = WANTED.lock().unwrap().as_ref().map(|wanted| wanted.card.clone());
    let gives = match wanted {
      Some(card) => on_the_card(instance, &card, None, &describe).await.is_some(),
      None => false,
    };
    GIVES.store(gives, Ordering::Release);
    gives
  }

  /// Whether the card made a device the last time an engine on another card asked it: the engines of the
  /// other windows, which may not be drawn now and ask nothing, are replaced with the one that asked.
  pub(in super::super) fn the_card_gave() -> bool {
    GIVES.load(Ordering::Acquire)
  }

  async fn on_the_card(
    instance: &wgpu::Instance,
    card: &Card,
    surface: Option<&wgpu::Surface<'_>>,
    describe: &impl Fn(&wgpu::Adapter) -> wgpu::DeviceDescriptor<'static>,
  ) -> Option<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    #[cfg(test)]
    if REFUSES.load(Ordering::Acquire) {
      return None;
    }
    for adapter in instance.enumerate_adapters(wgpu::Backends::all()).await {
      if Card::of(&adapter) != *card || surface.is_some_and(|surface| !adapter.is_surface_supported(surface)) {
        continue;
      }
      if let Ok((device, queue)) = adapter.request_device(&describe(&adapter)).await {
        return Some((adapter, device, queue));
      }
    }
    None
  }

  /// A device of another card than `card`, which made none: of a real card before one of software. The
  /// cards are asked by name, and not as at the start, where any card that draws is taken: `card` may be
  /// listed still and make a device a moment later, and the window is not to be on it by chance while the
  /// engine believes itself elsewhere.
  async fn on_another_card(
    instance: &wgpu::Instance,
    card: &Card,
    surface: Option<&wgpu::Surface<'_>>,
    describe: &impl Fn(&wgpu::Adapter) -> wgpu::DeviceDescriptor<'static>,
  ) -> Option<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    let mut others: Vec<wgpu::Adapter> = instance
      .enumerate_adapters(wgpu::Backends::all())
      .await
      .into_iter()
      .filter(|adapter| Card::of(adapter) != *card)
      .filter(|adapter| surface.is_none_or(|surface| adapter.is_surface_supported(surface)))
      .collect();
    others.sort_by_key(|adapter| match adapter.get_info().device_type {
      wgpu::DeviceType::DiscreteGpu => 0,
      wgpu::DeviceType::IntegratedGpu => 1,
      wgpu::DeviceType::VirtualGpu => 2,
      wgpu::DeviceType::Other => 3,
      wgpu::DeviceType::Cpu => 4,
    });
    for adapter in others {
      if let Ok((device, queue)) = adapter.request_device(&describe(&adapter)).await {
        return Some((adapter, device, queue));
      }
    }
    None
  }
}

/// Has `lost` say when `device`, of `adapter`'s card, is lost, and has an error of a device that is lost end
/// nothing. Any other error ends the program as it did, for it is a fault of the engine's.
pub(super) fn watch(device: &wgpu::Device, lost: &Arc<AtomicBool>, adapter: &wgpu::Adapter) {
  #[cfg(not(target_arch = "wasm32"))]
  card::remember(device, lost, adapter);
  #[cfg(target_arch = "wasm32")]
  let _ = adapter;
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

/// Asks wgpu what the card has finished, as `Device::poll` does, where the device may be lost. wgpu ends the
/// thread that asks it of a lost device (`Error in Device::poll … Parent device is lost`): a screenshot
/// that waited for its pixels ended so, and whoever waited for the screenshot waited for ever. `lost` is
/// looked at first where there is one, and what ends the question all the same, for a device lost while
/// it was asked, is caught. Whether wgpu answered.
pub(super) fn asked(device: &wgpu::Device, lost: Option<&AtomicBool>, how: wgpu::PollType) -> bool {
  if lost.is_some_and(|lost| lost.load(Ordering::Acquire)) {
    return false;
  }
  match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| device.poll(how))) {
    Ok(answered) => answered.is_ok(),
    Err(_) => {
      if let Some(lost) = lost {
        say(lost);
      }
      false
    }
  }
}

/// Whether `device` is lost, as `lost` was told or as its card says now.
///
/// wgpu is asked first what it has to tell: of some losses (a device that was destroyed) it tells only when it
/// is next asked, and nothing but its word tells of a loss where the card cannot be asked itself (Vulkan). Not
/// asked, the engine drew a frame with a lost device it took for a whole one, and what failed in that frame
/// ended the program.
pub(super) fn is_lost(device: &wgpu::Device, lost: &AtomicBool) -> bool {
  if lost.load(Ordering::Acquire) {
    return true;
  }
  asked(device, Some(lost), wgpu::PollType::Poll);
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

/// A device of another backend than Direct3D 12, where only wgpu tells of a loss.
#[cfg(test)]
mod told_by_wgpu {
  use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  };

  #[test]
  fn a_vulkan_device_that_wgpu_says_is_lost_is_lost_and_ends_nothing() {
    let instance = super::super::new_instance(wgpu::Backends::VULKAN);
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())) else {
      eprintln!("skipped: no card with Vulkan here");
      return;
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&super::super::device_descriptor(
      &adapter,
      wgpu::Features::empty(),
      &wgpu::Limits::default(),
    )))
    .expect("a device");
    let lost = Arc::new(AtomicBool::new(false));
    super::watch(&device, &lost, &adapter);
    assert!(!super::is_lost(&device, &lost));
    // Drawn with, it is whole.
    queue.submit([device.create_command_encoder(&Default::default()).finish()]);
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    assert!(!super::is_lost(&device, &lost));

    // No card of Vulkan's is asked whether its device is there: wgpu tells. Destroying the device is what makes
    // wgpu tell without a card that is really reset.
    device.destroy();
    // wgpu tells of it when it is next asked, and the engine asks before each frame: nothing else is to ask.
    assert!(
      !lost.load(Ordering::Acquire),
      "wgpu tells of a device that was destroyed only when it is asked"
    );
    assert!(
      super::is_lost(&device, &lost),
      "asked before the frame, wgpu tells of the loss"
    );
    assert!(lost.load(Ordering::Acquire));
    assert!(super::happened());

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
    super::watch(&device, &lost, &adapter);
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
    // Asked for while the lost one is held still: none is given yet, for a device made now is not a whole one,
    // and nothing ends.
    assert!(
      shared.current().is_none(),
      "a device is made while the lost one is held"
    );
    let card = first.adapter.get_info().name;
    drop(first);
    // Let go of, another is made on the card the first was lost on, once: what shares the device has the same
    // one. The system says that the lost one is destroyed in a moment.
    let started = std::time::Instant::now();
    let again = loop {
      if let Some(again) = shared.current() {
        break again;
      }
      assert!(
        started.elapsed() < std::time::Duration::from_secs(2),
        "no device, though the lost one is let go of"
      );
      std::thread::sleep(std::time::Duration::from_millis(20));
    };
    assert!(!super::is_lost(&again.device, &again.lost));
    assert_eq!(
      again.adapter.get_info().name,
      card,
      "on the card the device was lost on"
    );
    assert!(!again.while_held && !again.displaced);
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

  /// Run by itself, as the tests above: it removes the card's device.
  #[test]
  #[ignore = "removes the card's device from the whole test program: run alone, with --ignored"]
  fn a_card_that_makes_no_device_in_time_is_left_for_another_and_gone_back_to() {
    use std::{
      sync::atomic::Ordering,
      time::{Duration, Instant},
    };

    fn describe(adapter: &wgpu::Adapter) -> wgpu::DeviceDescriptor<'static> {
      super::super::device_descriptor(adapter, wgpu::Features::empty(), &wgpu::Limits::default())
    }

    let Ok(given) = pollster::block_on(super::super::WgpuDevice::request_with(wgpu::Backends::DX12)) else {
      eprintln!("skipped: no card with Direct3D 12 here");
      return;
    };
    let first = given.current().expect("the device that was asked for");
    let card = first.adapter.get_info().name;
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
    drop(first);
    // The card makes no device from here, as one whose driver is being put in again.
    super::card::REFUSES.store(true, Ordering::Release);
    let lost_at = Instant::now();

    // It is waited for: no other card is taken at once.
    assert!(
      given.current().is_none(),
      "another card is taken before the first was waited for"
    );
    assert!(lost_at.elapsed() < Duration::from_secs(2));
    std::thread::sleep(Duration::from_millis(3200));
    // Waited for long enough: a device of another card, which knows itself to be elsewhere.
    let Some(other) = given.current() else {
      super::card::REFUSES.store(false, Ordering::Release);
      eprintln!("the rest is not run: no other card makes a device here");
      return;
    };
    let elsewhere = other.adapter.get_info().name;
    assert_ne!(elsewhere, card, "the card that makes none is not the one taken");
    assert!(other.displaced && !other.while_held);
    assert!(!super::is_lost(&other.device, &other.lost));
    // What shares the device has the same one, and it stays while the first card makes none.
    assert!(std::sync::Arc::ptr_eq(
      &given.clone().current().unwrap().lost,
      &other.lost
    ));
    assert!(!pollster::block_on(super::the_card_gives(&other.instance, describe)));
    assert!(!super::the_card_gave());

    // The card makes a device again: an engine on the other card finds it, and says so for all.
    super::card::REFUSES.store(false, Ordering::Release);
    assert!(pollster::block_on(super::the_card_gives(&other.instance, describe)));
    assert!(super::the_card_gave());
    // Until an engine asks for it, the device of the other card is the one that is given.
    assert!(given.current().is_some_and(|made| made.displaced));
    given.back.store(true, Ordering::Release);
    let back = given.current().expect("a device of the first card");
    assert_eq!(
      back.adapter.get_info().name,
      card,
      "the device is of the card it was lost on again"
    );
    assert!(!back.displaced && !back.while_held);
    assert!(!super::is_lost(&back.device, &back.lost));
    eprintln!("lost on {card:?}, went to {elsewhere:?}, and came back");
  }
}
