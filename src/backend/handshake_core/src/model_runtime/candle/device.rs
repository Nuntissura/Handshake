#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CandleDevicePreference {
    #[default]
    Auto,
    Cpu,
    Cuda {
        ordinal: usize,
    },
    Metal {
        ordinal: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandleDeviceKind {
    Cpu,
    Cuda { ordinal: usize },
    Metal { ordinal: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandleDeviceSelection {
    preference: CandleDevicePreference,
    selected: CandleDeviceKind,
    native_engines_enabled: bool,
    fallback_reason: Option<String>,
}

impl CandleDeviceSelection {
    pub fn new(
        preference: CandleDevicePreference,
        selected: CandleDeviceKind,
        native_engines_enabled: bool,
        fallback_reason: Option<String>,
    ) -> Self {
        Self {
            preference,
            selected,
            native_engines_enabled,
            fallback_reason,
        }
    }

    pub fn preference(&self) -> CandleDevicePreference {
        self.preference
    }

    pub fn selected(&self) -> CandleDeviceKind {
        self.selected
    }

    pub fn native_engines_enabled(&self) -> bool {
        self.native_engines_enabled
    }

    pub fn fallback_reason(&self) -> Option<&str> {
        self.fallback_reason.as_deref()
    }
}

pub fn select_candle_device(preference: CandleDevicePreference) -> CandleDeviceSelection {
    #[cfg(feature = "candle-runtime-engine")]
    {
        select_native_candle_device(preference)
    }

    #[cfg(not(feature = "candle-runtime-engine"))]
    {
        let fallback_reason = match preference {
            CandleDevicePreference::Auto | CandleDevicePreference::Cpu => None,
            CandleDevicePreference::Cuda { .. } => {
                Some("candle-runtime-engine feature disabled; falling back to CPU".to_string())
            }
            CandleDevicePreference::Metal { .. } => {
                Some("candle-runtime-engine feature disabled; falling back to CPU".to_string())
            }
        };
        CandleDeviceSelection::new(preference, CandleDeviceKind::Cpu, false, fallback_reason)
    }
}

#[cfg(feature = "candle-runtime-engine")]
fn select_native_candle_device(preference: CandleDevicePreference) -> CandleDeviceSelection {
    match preference {
        CandleDevicePreference::Auto => {
            let cuda =
                candle_core::Device::cuda_if_available(0).unwrap_or(candle_core::Device::Cpu);
            if device_kind(&cuda) == (CandleDeviceKind::Cuda { ordinal: 0 }) {
                return CandleDeviceSelection::new(
                    preference,
                    CandleDeviceKind::Cuda { ordinal: 0 },
                    true,
                    None,
                );
            }

            let metal =
                candle_core::Device::metal_if_available(0).unwrap_or(candle_core::Device::Cpu);
            if device_kind(&metal) == (CandleDeviceKind::Metal { ordinal: 0 }) {
                return CandleDeviceSelection::new(
                    preference,
                    CandleDeviceKind::Metal { ordinal: 0 },
                    true,
                    None,
                );
            }

            CandleDeviceSelection::new(
                preference,
                CandleDeviceKind::Cpu,
                true,
                Some("no CUDA or Metal device available; falling back to CPU".to_string()),
            )
        }
        CandleDevicePreference::Cpu => {
            CandleDeviceSelection::new(preference, CandleDeviceKind::Cpu, true, None)
        }
        CandleDevicePreference::Cuda { ordinal } => {
            let device =
                candle_core::Device::cuda_if_available(ordinal).unwrap_or(candle_core::Device::Cpu);
            let selected = device_kind(&device);
            let fallback_reason = (selected == CandleDeviceKind::Cpu)
                .then(|| format!("CUDA device {ordinal} unavailable; falling back to CPU"));
            CandleDeviceSelection::new(preference, selected, true, fallback_reason)
        }
        CandleDevicePreference::Metal { ordinal } => {
            let device = candle_core::Device::metal_if_available(ordinal)
                .unwrap_or(candle_core::Device::Cpu);
            let selected = device_kind(&device);
            let fallback_reason = (selected == CandleDeviceKind::Cpu)
                .then(|| format!("Metal device {ordinal} unavailable; falling back to CPU"));
            CandleDeviceSelection::new(preference, selected, true, fallback_reason)
        }
    }
}

#[cfg(feature = "candle-runtime-engine")]
pub(crate) fn native_device_for_selection(
    selection: &CandleDeviceSelection,
) -> candle_core::Device {
    match selection.selected {
        CandleDeviceKind::Cpu => candle_core::Device::Cpu,
        CandleDeviceKind::Cuda { ordinal } => {
            candle_core::Device::cuda_if_available(ordinal).unwrap_or(candle_core::Device::Cpu)
        }
        CandleDeviceKind::Metal { ordinal } => {
            candle_core::Device::metal_if_available(ordinal).unwrap_or(candle_core::Device::Cpu)
        }
    }
}

#[cfg(feature = "candle-runtime-engine")]
fn device_kind(device: &candle_core::Device) -> CandleDeviceKind {
    match device {
        candle_core::Device::Cpu => CandleDeviceKind::Cpu,
        candle_core::Device::Cuda(_) => CandleDeviceKind::Cuda { ordinal: 0 },
        candle_core::Device::Metal(_) => CandleDeviceKind::Metal { ordinal: 0 },
    }
}
