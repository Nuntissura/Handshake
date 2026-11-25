//! Physical and logical geometry. No window is created.

use handshake_shell_window::{LogicalSize, PhysicalRect, PhysicalSize, ScaleFactor};

fn scale(dpi: u32) -> ScaleFactor {
    ScaleFactor::from_dpi(dpi).unwrap()
}

/// Every scale Windows offers, and the odd ones a custom setting gives.
const SCALES: [u32; 14] = [
    96, 97, 106, 110, 115, 120, 125, 144, 168, 192, 240, 288, 384, 480,
];

#[test]
fn a_scale_is_whole_dots_per_inch_within_bounds() {
    for dpi in [0, 1, 47, 961, u32::MAX] {
        assert_eq!(ScaleFactor::from_dpi(dpi), None, "{dpi}");
    }
    assert_eq!(scale(48).factor(), 0.5);
    assert_eq!(scale(120).factor(), 1.25);
    for dpi in ScaleFactor::MIN_DPI..=ScaleFactor::MAX_DPI {
        assert_eq!(
            ScaleFactor::from_factor(scale(dpi).factor()),
            Some(scale(dpi))
        );
    }
    for factor in [f64::NAN, f64::INFINITY, -1.0, 0.0, 0.1, 100.0] {
        assert_eq!(ScaleFactor::from_factor(factor), None, "{factor}");
    }
}

#[test]
fn a_physical_size_survives_the_way_to_logical_and_back_at_every_scale() {
    for dpi in SCALES {
        for extent in (0..4000).chain([7679, 7680, 16_384, 65_535, 1_000_003]) {
            let size = PhysicalSize::new(extent, extent / 2);
            assert_eq!(
                size.to_logical(scale(dpi)).to_physical(scale(dpi)),
                size,
                "{extent} at {dpi}"
            );
        }
    }
}

#[test]
fn a_logical_size_is_rounded_by_at_most_half_a_pixel_at_every_scale() {
    for width in (200..1200).step_by(7) {
        for dpi in SCALES {
            let there = LogicalSize::new(f64::from(width), 333.0).to_physical(scale(dpi));
            let exact = f64::from(width) * f64::from(dpi) / 96.0;
            assert!(
                (f64::from(there.width) - exact).abs() <= 0.5,
                "{width} at {dpi}"
            );
        }
    }
}

#[test]
fn halves_round_up_and_nonsense_becomes_nothing() {
    let size = |width, dpi| LogicalSize::new(width, 1.0).to_physical(scale(dpi)).width;
    assert_eq!(size(1.0, 144), 2, "1.5");
    assert_eq!(size(3.0, 144), 5, "4.5");
    assert_eq!(size(-5.0, 96), 0);
    assert_eq!(size(f64::NAN, 96), 0);
    assert_eq!(size(f64::INFINITY, 96), i32::MAX as u32);
    assert!(!LogicalSize::new(f64::NAN, 1.0).is_usable());
    assert!(!LogicalSize::new(0.5, 10.0).is_usable());
    assert!(!LogicalSize::new(10.0, 1e12).is_usable());
    assert!(LogicalSize::new(1.0, 1.0).is_usable());
}

#[test]
fn shared_area_is_exact_at_the_edges_of_the_coordinates() {
    let a = PhysicalRect::new(0, 0, 100, 100);
    assert_eq!(a.shared_area(PhysicalRect::new(50, 90, 100, 100)), 500);
    assert_eq!(
        a.shared_area(PhysicalRect::new(100, 0, 10, 10)),
        0,
        "touching shares nothing"
    );
    let huge = PhysicalRect::new(i32::MIN, i32::MIN, u32::MAX, u32::MAX);
    assert_eq!(huge.shared_area(a), 10_000);
    assert!(huge.shared_area(huge) > 0);
}
