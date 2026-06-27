//! WP-KERNEL-012 MT-086 (D2 — internal_diagnostics, Tier 2): STATIC GPU/driver identity captured ONCE
//! at startup (Master Spec v02.196 §5.8.2 "resource counters" + §5.8.4 panel hardware line).
//!
//! # What this is — static identity, NOT a per-frame counter
//!
//! Unlike CPU%/RSS (which are periodic counters, MT-086 [`super::resource_counters`]), the GPU/driver
//! identity is STATIC for the session: the adapter does not change while the app runs. So it is read
//! ONCE in [`crate::app::HandshakeApp::new`] from the already-initialized eframe wgpu render state
//! ([`eframe::CreationContext::wgpu_render_state`]) and stored for the Diagnostics Panel's hardware line.
//! No second wgpu device/adapter is created — we read the EXISTING adapter eframe already initialized
//! (RISK-006-5 / AC-006-3): `cc.wgpu_render_state.as_ref()?.adapter.get_info()`.
//!
//! # The integer/string split (RISK-006-4 / AC-006-3) — load-bearing privacy design
//!
//! `wgpu::AdapterInfo` carries BOTH machine identity (`vendor: u32`, `device: u32`, `device_type` enum,
//! `backend` enum) AND human strings (`name`, `driver`, `driver_info` — e.g. "NVIDIA GeForce RTX 4090",
//! "D3D12"). These strings are field-standard HARDWARE identity (not project/sensitive data), so they
//! are allowed for the panel's hardware line — BUT the MT-081 typed ring is INTEGER-ONLY (the
//! typed-allowlist invariant, §5.8.3). So the split is:
//!
//! - **Integer codes** (`vendor_id`, `device_id`, `device_type_code`, `backend_code`) — safe for the
//!   typed ring; emitted as a typed event if/when a ring producer wants them. NO strings reach the ring.
//! - **Human strings** (`name`, `driver`, `driver_info`) — kept ONLY in this in-process [`GpuInfo`] for
//!   the panel's hardware line; they are NEVER pushed into a `DiagEvent`.
//!
//! This module deliberately exposes the integer codes and the strings as SEPARATE fields so a future
//! ring producer can only reach the integers, and the panel reaches the strings — the type boundary
//! keeps the strings out of the ring.
//!
//! # No new dependency family (RISK-006-5)
//!
//! `wgpu` is NOT a direct dependency of this crate — it is reached through eframe's own re-export
//! (`eframe::wgpu`, which eframe 0.33 re-exports as `pub use {egui_wgpu, wgpu};`). So this module adds
//! NO new dependency and uses the SAME wgpu eframe already links. eframe is configured with the `wgpu`
//! renderer always on for this crate (`eframe = { features = ["wgpu", ...] }`), so `eframe::wgpu` is
//! always available and no `cfg(feature)` gate is needed here.

use eframe::wgpu;

/// The captured (once) GPU/driver identity for this session. Integer codes are ring-safe; the human
/// strings are panel-only (NEVER emitted into a typed `DiagEvent`). Cheap to clone (small + a few short
/// strings); stored once on [`crate::app::HandshakeApp`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GpuInfo {
    // ── ring-safe integer codes (the typed-allowlist surface) ──
    /// Backend-specific vendor id (PCI vendor id in the low bytes, e.g. 0x10DE for NVIDIA). A `u32`.
    pub vendor_id: u32,
    /// Backend-specific device id (PCI device id in the low bytes). A `u32`.
    pub device_id: u32,
    /// `wgpu::DeviceType` mapped to a stable `u8` code (see [`device_type_code`]). 0..=4.
    pub device_type_code: u8,
    /// `wgpu::Backend` mapped to its stable `u8` discriminant (Noop=0, Vulkan=1, Metal=2, Dx12=3, Gl=4,
    /// BrowserWebGpu=5). `wgpu::Backend` is `#[repr(u8)]`, so this is the backend's own discriminant.
    pub backend_code: u8,

    // ── panel-only human strings (NEVER pushed into the typed ring) ──
    /// Adapter name (e.g. "NVIDIA GeForce RTX 4090"). Hardware identity for the panel's hardware line.
    /// NOT ring data.
    pub name: String,
    /// Driver name (e.g. "NVIDIA"). Panel-only hardware identity. NOT ring data.
    pub driver: String,
    /// Driver info string (e.g. a version). Panel-only hardware identity. NOT ring data.
    pub driver_info: String,
}

impl GpuInfo {
    /// Capture the GPU/driver identity ONCE from the eframe wgpu render state. Returns `None` when there
    /// is no wgpu render state (a headless build path, or a kittest harness built without the wgpu
    /// backend) — diagnostics degrade gracefully to "no GPU info" rather than panicking.
    ///
    /// This reads the EXISTING adapter eframe already created (`render_state.adapter`) — it creates NO
    /// new device/adapter (RISK-006-5). The whole capture is `adapter.get_info()` + a pure mapping.
    pub fn capture(cc: &eframe::CreationContext<'_>) -> Option<Self> {
        let render_state = cc.wgpu_render_state.as_ref()?;
        Some(Self::from_adapter_info(&render_state.adapter.get_info()))
    }

    /// Map a `wgpu::AdapterInfo` into the typed [`GpuInfo`]: integer codes for the ring-safe surface +
    /// the human strings for the panel. Pure (no GPU calls) so it is unit-testable without a device.
    pub fn from_adapter_info(info: &wgpu::AdapterInfo) -> Self {
        Self {
            vendor_id: info.vendor,
            device_id: info.device,
            device_type_code: device_type_code(info.device_type),
            // wgpu::Backend is #[repr(u8)] with stable discriminants; its own value is the code.
            backend_code: info.backend as u8,
            name: info.name.clone(),
            driver: info.driver.clone(),
            driver_info: info.driver_info.clone(),
        }
    }

    /// True when this carries a real captured identity (a non-default vendor OR device OR a non-empty
    /// name). A `GpuInfo::default()` (the "no wgpu render state" fallback) is NOT captured. Used by the
    /// kittest to assert a real adapter was read (AC-006-3).
    pub fn is_captured(&self) -> bool {
        self.vendor_id != 0 || self.device_id != 0 || !self.name.is_empty()
    }
}

/// Map `wgpu::DeviceType` to a stable `u8` code. `DeviceType` has no `#[repr]`, so we map it explicitly
/// (rather than relying on `as u8`) to PIN the codes against a future variant reorder: Other=0,
/// IntegratedGpu=1, DiscreteGpu=2, VirtualGpu=3, Cpu=4. Kept in sync by an exhaustive `match` (a new
/// variant would force a compile error here).
fn device_type_code(t: wgpu::DeviceType) -> u8 {
    match t {
        wgpu::DeviceType::Other => 0,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::DiscreteGpu => 2,
        wgpu::DeviceType::VirtualGpu => 3,
        wgpu::DeviceType::Cpu => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mapping from `AdapterInfo` is pure and correct: integer codes come from the integer fields +
    /// the typed enum codes; the human strings are carried verbatim for the panel. (AC-006-3 mapping —
    /// no GPU device needed for this pure-mapping proof; the live-capture proof is the kittest.)
    #[test]
    fn maps_adapter_info_to_typed_codes_and_panel_strings() {
        let info = wgpu::AdapterInfo {
            name: "Test Discrete GPU".to_owned(),
            vendor: 0x10DE,
            device: 0x2204,
            device_type: wgpu::DeviceType::DiscreteGpu,
            driver: "TestDriver".to_owned(),
            driver_info: "v1.2.3".to_owned(),
            backend: wgpu::Backend::Dx12,
        };
        let gpu = GpuInfo::from_adapter_info(&info);

        // Ring-safe integer codes.
        assert_eq!(gpu.vendor_id, 0x10DE, "vendor id carried as u32");
        assert_eq!(gpu.device_id, 0x2204, "device id carried as u32");
        assert_eq!(gpu.device_type_code, 2, "DiscreteGpu -> code 2");
        assert_eq!(gpu.backend_code, wgpu::Backend::Dx12 as u8, "Dx12 -> its repr(u8) discriminant");
        assert_eq!(gpu.backend_code, 3, "Dx12 discriminant is 3");

        // Panel-only human strings (verbatim).
        assert_eq!(gpu.name, "Test Discrete GPU");
        assert_eq!(gpu.driver, "TestDriver");
        assert_eq!(gpu.driver_info, "v1.2.3");

        assert!(gpu.is_captured(), "a real adapter info maps to a captured GpuInfo");
    }

    /// Every `DeviceType` variant maps to a distinct, pinned code (a reorder of the enum would not shift
    /// these because the mapping is an explicit exhaustive match).
    #[test]
    fn device_type_codes_are_pinned() {
        assert_eq!(device_type_code(wgpu::DeviceType::Other), 0);
        assert_eq!(device_type_code(wgpu::DeviceType::IntegratedGpu), 1);
        assert_eq!(device_type_code(wgpu::DeviceType::DiscreteGpu), 2);
        assert_eq!(device_type_code(wgpu::DeviceType::VirtualGpu), 3);
        assert_eq!(device_type_code(wgpu::DeviceType::Cpu), 4);
    }

    /// A default `GpuInfo` (the "no wgpu render state" fallback) is NOT captured.
    #[test]
    fn default_is_not_captured() {
        assert!(!GpuInfo::default().is_captured(), "the default fallback is not a real capture");
    }
}
