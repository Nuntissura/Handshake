//! Physical and logical geometry.
//!
//! The scale is kept as whole dots per inch, so a conversion never depends on how a factor such as 1.15 rounds as a
//! float, and a logical size survives a round trip through any scale.

/// Dots per inch of an unscaled display.
pub const BASE_DPI: u32 = 96;

/// Largest coordinate or extent the operating system's signed coordinates can hold.
const MAX_EXTENT: f64 = i32::MAX as f64;

/// How many physical pixels one logical pixel takes, as dots per inch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScaleFactor {
    dpi: u32,
}

impl ScaleFactor {
    /// Unscaled: one physical pixel per logical pixel.
    pub const ONE: Self = Self { dpi: BASE_DPI };
    /// The smallest accepted value (50 %).
    pub const MIN_DPI: u32 = 48;
    /// The largest accepted value (1000 %).
    pub const MAX_DPI: u32 = 960;

    /// From dots per inch. `None` outside [`ScaleFactor::MIN_DPI`]..=[`ScaleFactor::MAX_DPI`].
    pub fn from_dpi(dpi: u32) -> Option<Self> {
        (Self::MIN_DPI..=Self::MAX_DPI)
            .contains(&dpi)
            .then_some(Self { dpi })
    }

    /// From a factor such as 1.25, to the nearest whole dot per inch. `None` when it is not a usable number.
    pub fn from_factor(factor: f64) -> Option<Self> {
        if !factor.is_finite() {
            return None;
        }
        let dpi = (factor * f64::from(BASE_DPI)).round();
        if dpi < f64::from(Self::MIN_DPI) || dpi > f64::from(Self::MAX_DPI) {
            return None;
        }
        Self::from_dpi(dpi as u32)
    }

    /// Dots per inch.
    pub fn dpi(self) -> u32 {
        self.dpi
    }

    /// The factor, for example 1.25.
    pub fn factor(self) -> f64 {
        f64::from(self.dpi) / f64::from(BASE_DPI)
    }

    /// A logical length in physical pixels, unrounded.
    pub fn to_physical(self, logical: f64) -> f64 {
        logical * f64::from(self.dpi) / f64::from(BASE_DPI)
    }

    /// A physical length in logical pixels.
    pub fn to_logical(self, physical: f64) -> f64 {
        physical * f64::from(BASE_DPI) / f64::from(self.dpi)
    }
}

/// A size in physical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PhysicalSize {
    /// Width in physical pixels.
    pub width: u32,
    /// Height in physical pixels.
    pub height: u32,
}

impl PhysicalSize {
    /// A size.
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Nothing can be drawn into it.
    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// The same size in logical pixels. Exact: the way back gives this size again.
    pub fn to_logical(self, scale: ScaleFactor) -> LogicalSize {
        LogicalSize {
            width: scale.to_logical(f64::from(self.width)),
            height: scale.to_logical(f64::from(self.height)),
        }
    }
}

/// A size in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalSize {
    /// Width in logical pixels.
    pub width: f64,
    /// Height in logical pixels.
    pub height: f64,
}

impl LogicalSize {
    /// A size.
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// Both extents are numbers, and at least one logical pixel.
    pub fn is_usable(self) -> bool {
        let usable = |extent: f64| extent.is_finite() && (1.0..=MAX_EXTENT).contains(&extent);
        usable(self.width) && usable(self.height)
    }

    /// The same size in whole physical pixels, halves rounded up. An extent that is not a number becomes 0.
    pub fn to_physical(self, scale: ScaleFactor) -> PhysicalSize {
        PhysicalSize {
            width: whole_extent(scale.to_physical(self.width)),
            height: whole_extent(scale.to_physical(self.height)),
        }
    }
}

fn whole_extent(extent: f64) -> u32 {
    if extent.is_nan() {
        return 0;
    }
    extent.round().clamp(0.0, MAX_EXTENT) as u32
}

/// A position in physical pixels of the virtual desktop. Negative left of or above the primary monitor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PhysicalPosition {
    /// Pixels from the primary monitor's left edge.
    pub x: i32,
    /// Pixels from the primary monitor's top edge.
    pub y: i32,
}

impl PhysicalPosition {
    /// A position.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// A point inside a window's client area, in physical pixels, with fractions where the device has them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClientPoint {
    /// Pixels from the client area's left edge.
    pub x: f64,
    /// Pixels from the client area's top edge.
    pub y: f64,
}

/// A rectangle in physical pixels of the virtual desktop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PhysicalRect {
    /// Top left corner.
    pub origin: PhysicalPosition,
    /// Extent.
    pub size: PhysicalSize,
}

impl PhysicalRect {
    /// A rectangle.
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            origin: PhysicalPosition::new(x, y),
            size: PhysicalSize::new(width, height),
        }
    }

    /// The first column right of it.
    pub fn right(self) -> i64 {
        i64::from(self.origin.x) + i64::from(self.size.width)
    }

    /// The first row below it.
    pub fn bottom(self) -> i64 {
        i64::from(self.origin.y) + i64::from(self.size.height)
    }

    /// The pixels two rectangles share.
    pub fn shared_area(self, other: Self) -> i64 {
        let left = i64::from(self.origin.x.max(other.origin.x));
        let top = i64::from(self.origin.y.max(other.origin.y));
        let width = self.right().min(other.right()) - left;
        let height = self.bottom().min(other.bottom()) - top;
        if width <= 0 || height <= 0 {
            0
        } else {
            width.saturating_mul(height)
        }
    }
}
