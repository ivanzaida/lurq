//! How a device is asked for: what an engine, or an app that makes a device ahead, says of it before there is one.

use super::{WgpuBackends, WgpuFeatures, WgpuInstanceFlags, WgpuLimits, WgpuMemoryHints, default_backends};
use crate::app::GraphicsCard;

/// How a GPU device is asked for: among which backends, with which flags of wgpu's instance, with which
/// optional features and limits, how its memory is asked of the card, and on which card.
///
/// An engine is told each of these by its own builders (`WgpuRenderEngine::with_backends`,
/// `with_instance_flags`, `with_optional_device_features`, `with_optional_device_limits`, `with_memory_hints`,
/// `with_graphics_card`). This is the same for a device made ahead of its engine, which
/// [`WgpuDevice::request_as`](super::WgpuDevice::request_as) takes.
///
/// ```no_run
/// # async fn start() {
/// use lurq::app::{
///   GraphicsCard,
///   wgpu_render::{WgpuDevice, WgpuDeviceRequest},
/// };
///
/// let request = WgpuDeviceRequest::new().graphics_card(GraphicsCard::HighPerformance);
/// let device = WgpuDevice::request_as(request).await.expect("a GPU device");
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct WgpuDeviceRequest {
  pub(super) backends: WgpuBackends,
  pub(super) instance_flags: WgpuInstanceFlags,
  pub(super) features: WgpuFeatures,
  pub(super) limits: WgpuLimits,
  pub(super) memory_hints: WgpuMemoryHints,
  pub(super) card: GraphicsCard,
}

impl Default for WgpuDeviceRequest {
  fn default() -> Self {
    Self::new()
  }
}

impl WgpuDeviceRequest {
  /// A device as an engine that was told nothing asks for it: lurq's default backends, the flags of the build,
  /// no optional features, wgpu's default limits, `WgpuMemoryHints::MemoryUsage`, and the card the system
  /// chooses.
  pub fn new() -> Self {
    Self {
      backends: default_backends(),
      instance_flags: WgpuInstanceFlags::from_build_config(),
      features: WgpuFeatures::empty(),
      limits: WgpuLimits::default(),
      memory_hints: WgpuMemoryHints::MemoryUsage,
      card: GraphicsCard::Any,
    }
  }

  /// Among these backends only; see `WgpuRenderEngine::with_backends`.
  pub fn backends(mut self, backends: WgpuBackends) -> Self {
    self.backends = backends;
    self
  }

  /// The flags of wgpu's instance; see `WgpuRenderEngine::with_instance_flags`. wgpu's environment variables
  /// are applied over them.
  pub fn instance_flags(mut self, flags: WgpuInstanceFlags) -> Self {
    self.instance_flags = flags;
    self
  }

  /// These features too, where the card has them.
  pub fn optional_features(mut self, features: WgpuFeatures) -> Self {
    self.features |= features;
    self
  }

  /// These limits, as far as the card has them.
  pub fn optional_limits(mut self, limits: WgpuLimits) -> Self {
    self.limits = limits;
    self
  }

  /// How the device asks the card for memory; see `WgpuRenderEngine::with_memory_hints`.
  pub fn memory_hints(mut self, hints: WgpuMemoryHints) -> Self {
    self.memory_hints = hints;
    self
  }

  /// The card the device is on; see `WgpuRenderEngine::with_graphics_card`.
  pub fn graphics_card(mut self, card: GraphicsCard) -> Self {
    self.card = card;
    self
  }

  /// Whether `other` asks for a device as this does: two engines that ask alike draw with one device.
  pub(super) fn same(&self, other: &Self) -> bool {
    self.backends == other.backends
      && self.instance_flags == other.instance_flags
      && self.features == other.features
      && self.limits == other.limits
      && same_hints(&self.memory_hints, &other.memory_hints)
      && self.card == other.card
  }

  /// The instance of wgpu the device is asked of.
  pub(super) fn instance(&self) -> wgpu::Instance {
    super::new_instance_with(self.backends, self.instance_flags)
  }

  /// The device asked of `adapter`.
  pub(super) fn describe(&self, adapter: &wgpu::Adapter) -> wgpu::DeviceDescriptor<'static> {
    super::device_descriptor_with(adapter, self.features, &self.limits, &self.memory_hints)
  }

  /// The adapter of `instance` the device is asked of: the card asked for, which draws to `surface` where
  /// there is one to draw to. Where a card was asked for by its identifier and none of it draws there, the
  /// system's choice.
  pub(super) async fn adapter(
    &self,
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'static>>,
  ) -> Result<wgpu::Adapter, wgpu::RequestAdapterError> {
    if let Some(luid) = self.card.luid() {
      #[cfg(windows)]
      if let Some(adapter) = of_luid(instance, self.backends, luid, surface).await {
        return Ok(adapter);
      }
      tracing::warn!(
        low = luid.0,
        high = luid.1,
        "no graphics card of this identifier draws to the window: the system's choice is taken"
      );
    }
    instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: power_preference(self.card),
        compatible_surface: surface,
        force_fallback_adapter: false,
      })
      .await
  }
}

/// What wgpu is told of the card: which of two it prefers. A card asked for by its identifier is found
/// otherwise, and where it is not, nothing is preferred.
fn power_preference(card: GraphicsCard) -> wgpu::PowerPreference {
  match card {
    GraphicsCard::Any | GraphicsCard::Luid(..) => wgpu::PowerPreference::None,
    GraphicsCard::LowPower => wgpu::PowerPreference::LowPower,
    GraphicsCard::HighPerformance => wgpu::PowerPreference::HighPerformance,
  }
}

/// Whether two hints are the same: wgpu's type does not compare.
fn same_hints(one: &WgpuMemoryHints, other: &WgpuMemoryHints) -> bool {
  match (one, other) {
    (WgpuMemoryHints::Performance, WgpuMemoryHints::Performance)
    | (WgpuMemoryHints::MemoryUsage, WgpuMemoryHints::MemoryUsage) => true,
    (
      WgpuMemoryHints::Manual {
        suballocated_device_memory_block_size: one,
      },
      WgpuMemoryHints::Manual {
        suballocated_device_memory_block_size: other,
      },
    ) => one == other,
    _ => false,
  }
}

/// The adapter of `instance`, among `backends`, that is the card of the identifier `luid` and draws to
/// `surface`.
///
/// wgpu does not say a card's identifier, and the system does: the system's cards are asked for the one of
/// `luid`, and wgpu's adapter is the one that names the same maker and device, as the card a device was lost
/// on is found again. Two cards of one model are not told apart by this.
#[cfg(windows)]
async fn of_luid(
  instance: &wgpu::Instance,
  backends: WgpuBackends,
  luid: (u32, i32),
  surface: Option<&wgpu::Surface<'static>>,
) -> Option<wgpu::Adapter> {
  use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1};

  // SAFETY: the system is asked for its cards, and what each says of itself is read.
  let (vendor, device) = unsafe {
    let cards = CreateDXGIFactory1::<IDXGIFactory1>().ok()?;
    (0..)
      .map_while(|index| cards.EnumAdapters1(index).ok())
      .filter_map(|card| card.GetDesc1().ok())
      .find(|said| said.AdapterLuid.LowPart == luid.0 && said.AdapterLuid.HighPart == luid.1)
      .map(|said| (said.VendorId, said.DeviceId))?
  };
  instance.enumerate_adapters(backends).await.into_iter().find(|adapter| {
    let info = adapter.get_info();
    info.vendor == vendor
      && info.device == device
      && surface.is_none_or(|surface| adapter.is_surface_supported(surface))
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_request_asks_for_small_blocks_of_memory_unless_told() {
    assert!(matches!(
      WgpuDeviceRequest::new().memory_hints,
      WgpuMemoryHints::MemoryUsage
    ));
    let told = WgpuDeviceRequest::new().memory_hints(WgpuMemoryHints::Performance);
    assert!(matches!(told.memory_hints, WgpuMemoryHints::Performance));
  }

  #[test]
  fn requests_are_the_same_only_where_all_they_say_is() {
    let plain = WgpuDeviceRequest::new();
    assert!(plain.same(&WgpuDeviceRequest::new()));
    assert!(!plain.same(&WgpuDeviceRequest::new().memory_hints(WgpuMemoryHints::Performance)));
    assert!(!plain.same(&WgpuDeviceRequest::new().graphics_card(GraphicsCard::LowPower)));
    assert!(!plain.same(&WgpuDeviceRequest::new().graphics_card(GraphicsCard::Luid(1, 0))));
    assert!(!plain.same(&WgpuDeviceRequest::new().instance_flags(!plain.instance_flags)));
    assert!(!plain.same(&WgpuDeviceRequest::new().optional_features(WgpuFeatures::TIMESTAMP_QUERY)));

    let blocks = |size: std::ops::Range<u64>| {
      WgpuDeviceRequest::new().memory_hints(WgpuMemoryHints::Manual {
        suballocated_device_memory_block_size: size,
      })
    };
    assert!(blocks(1..2).same(&blocks(1..2)));
    assert!(!blocks(1..2).same(&blocks(1..3)));
  }

  #[test]
  fn a_card_is_preferred_by_its_power_and_not_by_its_identifier() {
    assert_eq!(power_preference(GraphicsCard::Any), wgpu::PowerPreference::None);
    assert_eq!(
      power_preference(GraphicsCard::LowPower),
      wgpu::PowerPreference::LowPower
    );
    assert_eq!(
      power_preference(GraphicsCard::HighPerformance),
      wgpu::PowerPreference::HighPerformance
    );
    assert_eq!(power_preference(GraphicsCard::Luid(1, 0)), wgpu::PowerPreference::None);
  }

  #[test]
  fn an_engine_asks_as_its_builders_say() {
    let engine = super::super::WgpuRenderEngine::new()
      .with_memory_hints(WgpuMemoryHints::Performance)
      .with_instance_flags(WgpuInstanceFlags::empty())
      .with_graphics_card(GraphicsCard::HighPerformance);
    let asked = WgpuDeviceRequest::new()
      .memory_hints(WgpuMemoryHints::Performance)
      .instance_flags(WgpuInstanceFlags::empty())
      .graphics_card(GraphicsCard::HighPerformance);
    assert!(engine.request.same(&asked));
    assert!(
      super::super::WgpuRenderEngine::new()
        .request
        .same(&WgpuDeviceRequest::new())
    );
  }

  /// The device a request describes carries its hints, on a card that is there.
  #[test]
  fn the_device_is_described_with_the_hints_asked_for() {
    let request = WgpuDeviceRequest::new();
    let instance = request.instance();
    let Ok(adapter) = pollster::block_on(request.adapter(&instance, None)) else {
      eprintln!("skipped: no card to make a device on");
      return;
    };
    assert!(matches!(
      request.describe(&adapter).memory_hints,
      WgpuMemoryHints::MemoryUsage
    ));
    let large = request.clone().memory_hints(WgpuMemoryHints::Performance);
    assert!(matches!(
      large.describe(&adapter).memory_hints,
      WgpuMemoryHints::Performance
    ));
    // A device is made as described.
    assert!(pollster::block_on(adapter.request_device(&request.describe(&adapter))).is_ok());
  }

  /// Engines that ask for other flags are given another instance, and those that ask alike the same one.
  #[test]
  fn an_instance_is_shared_by_those_that_ask_for_the_same_flags() {
    let plain = WgpuDeviceRequest::new().instance_flags(WgpuInstanceFlags::empty());
    let checked = WgpuDeviceRequest::new().instance_flags(WgpuInstanceFlags::VALIDATION);
    let count = || super::super::INSTANCES.lock().unwrap().len();
    let _ = plain.instance();
    let _ = checked.instance();
    let both = count();
    let _ = plain.instance();
    let _ = checked.instance();
    assert_eq!(count(), both);
  }
}
