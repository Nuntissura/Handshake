//! Strict physical-device selection without a GPU: hardware Vulkan or a typed refusal, never a fallback.

mod support;

use handshake_vulkan_renderer::select::{
    evaluate, meets_baseline, pick, CandidateFacts, Rejection, DRIVER_ID_DOZEN, DRIVER_ID_LLVMPIPE,
    DRIVER_ID_SWIFTSHADER,
};
use handshake_vulkan_renderer::{DeviceClass, Unavailable, BASELINE_API_VERSION};
use support::{KEY, OTHER_KEY};

fn gpu(class: DeviceClass) -> CandidateFacts {
    CandidateFacts {
        key: KEY,
        class,
        api_version: BASELINE_API_VERSION,
        driver_id: 4,
        has_graphics_queue: true,
    }
}

#[test]
fn only_hardware_vulkan_at_the_baseline_is_acceptable() {
    let discrete = gpu(DeviceClass::Discrete);
    assert_eq!(evaluate(&discrete), Ok(0));
    assert_eq!(evaluate(&gpu(DeviceClass::Integrated)), Ok(1));
    assert_eq!(evaluate(&gpu(DeviceClass::Virtual)), Ok(2));
    assert_eq!(
        evaluate(&gpu(DeviceClass::Cpu)),
        Err(Rejection::SoftwareDevice)
    );
    assert_eq!(
        evaluate(&gpu(DeviceClass::Other)),
        Err(Rejection::UnknownDeviceType)
    );
    for (driver_id, rejection) in [
        (DRIVER_ID_SWIFTSHADER, Rejection::SoftwareDevice),
        (DRIVER_ID_LLVMPIPE, Rejection::SoftwareDevice),
        (DRIVER_ID_DOZEN, Rejection::LayeredDriver),
    ] {
        // Also when the device claims to be a hardware type.
        assert_eq!(
            evaluate(&CandidateFacts {
                driver_id,
                ..discrete
            }),
            Err(rejection)
        );
    }
    assert_eq!(
        evaluate(&CandidateFacts {
            has_graphics_queue: false,
            ..discrete
        }),
        Err(Rejection::NoGraphicsQueue)
    );
}

#[test]
fn the_baseline_is_standard_vulkan_1_3() {
    assert!(meets_baseline(BASELINE_API_VERSION));
    assert!(meets_baseline(BASELINE_API_VERSION + 1));
    assert!(!meets_baseline(BASELINE_API_VERSION - 1));
    let other_variant = BASELINE_API_VERSION | (1 << 29);
    assert!(!meets_baseline(other_variant));
    assert_eq!(
        evaluate(&CandidateFacts {
            api_version: other_variant,
            ..gpu(DeviceClass::Discrete)
        }),
        Err(Rejection::ApiBelowBaseline)
    );
}

#[test]
fn the_best_acceptable_device_wins_and_the_first_breaks_a_tie() {
    let devices = [
        gpu(DeviceClass::Cpu),
        gpu(DeviceClass::Virtual),
        gpu(DeviceClass::Integrated),
        gpu(DeviceClass::Discrete),
        gpu(DeviceClass::Discrete),
    ];
    assert_eq!(pick(&devices, None), Ok(3));
    assert_eq!(pick(&devices[..3], None), Ok(2));
}

#[test]
fn a_machine_without_an_acceptable_device_gets_a_typed_refusal() {
    let software = gpu(DeviceClass::Cpu);
    let old = CandidateFacts {
        api_version: BASELINE_API_VERSION - 1,
        ..gpu(DeviceClass::Discrete)
    };
    let layered = CandidateFacts {
        driver_id: DRIVER_ID_DOZEN,
        ..gpu(DeviceClass::Discrete)
    };
    let unusable = CandidateFacts {
        has_graphics_queue: false,
        ..gpu(DeviceClass::Discrete)
    };
    for (devices, expected) in [
        (vec![], Unavailable::NoPhysicalDevice),
        (vec![software], Unavailable::SoftwareDeviceRefused),
        (vec![old, software], Unavailable::SoftwareDeviceRefused),
        (vec![software, layered], Unavailable::LayeredDriverRefused),
        (vec![old], Unavailable::ApiBelowBaseline),
        (vec![unusable], Unavailable::NoSuitableDevice),
    ] {
        assert_eq!(pick(&devices, None), Err(expected));
    }
}

#[test]
fn a_rebind_takes_its_own_adapter_or_none() {
    let other = CandidateFacts {
        key: OTHER_KEY,
        ..gpu(DeviceClass::Discrete)
    };
    let own = gpu(DeviceClass::Integrated);
    assert_eq!(pick(&[other, own], Some(KEY)), Ok(1));
    assert_eq!(
        pick(&[other], Some(KEY)),
        Err(Unavailable::RequiredDeviceMissing)
    );
    let own_broken = CandidateFacts {
        has_graphics_queue: false,
        ..own
    };
    assert_eq!(
        pick(&[other, own_broken], Some(KEY)),
        Err(Unavailable::NoSuitableDevice)
    );
}
