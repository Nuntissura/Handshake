//! The first frame: every texel the theme's background, and what each texel must read back as.
//!
//! One description serves the swapchain image and the headless image, so a readback of either is compared against the
//! same expectation ([`ClearFrame::expected`], [`compare_texels`]).

use std::fmt;

use handshake_vulkan_renderer::{
    AttachmentFormat, Extent, ImageLayout, ImageTarget, RecordRefusal, Recording,
};

/// A background colour as the theme states it: sRGB-encoded bytes, as they are displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Background {
    /// Red, green and blue, sRGB-encoded.
    pub srgb: [u8; 3],
}

impl Background {
    /// The dark theme's `bg` (`HsPalette::dark` in `src/frontend/handshake_native/src/theme/palette.rs`, from App.css
    /// `[data-theme='dark']`), the theme that host starts with (`current_theme: HsTheme::Dark`).
    pub const DARK: Self = Self {
        srgb: [0x16, 0x21, 0x27],
    };
    /// The light theme's `bg` (`HsPalette::light`, App.css `:root`).
    pub const LIGHT: Self = Self {
        srgb: [0xf8, 0xfa, 0xfc],
    };
}

/// One texel as a readback lays it out: four bytes for the 8-bit formats, eight for half floats.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Texel {
    bytes: [u8; 8],
    len: u8,
}

impl Texel {
    fn from_slice(bytes: &[u8]) -> Self {
        let mut texel = Self {
            bytes: [0; 8],
            len: bytes.len().min(8) as u8,
        };
        texel.bytes[..usize::from(texel.len)].copy_from_slice(&bytes[..usize::from(texel.len)]);
        texel
    }

    /// Its bytes, in memory order.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }
}

impl fmt::Debug for Texel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Texel(")?;
        for byte in self.bytes() {
            write!(f, "{byte:02x}")?;
        }
        f.write_str(")")
    }
}

/// The minimal frame of the shell: the whole image cleared to the background, opaque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClearFrame {
    /// The colour.
    pub background: Background,
}

/// Bytes a channel may differ by in a readback, in the channel's own unit: one 8-bit step, or one half-float step.
/// The Vulkan specification lets a conversion to normalized fixed point return either of the two nearest integers
/// ("Conversion From Floating-Point to Normalized Fixed-Point": "returns one of the two unsigned binary integer values
/// ... which are closest"), and an sRGB encoding or a half-float conversion rounds as well.
pub const TOLERANCE: u16 = 1;

impl ClearFrame {
    /// The frame.
    pub const fn new(background: Background) -> Self {
        Self { background }
    }

    /// The clear colour for an image of `format`, as `VkClearColorValue.float32` (red, green, blue, alpha). The
    /// swapchain's color space is sRGB nonlinear (the owner picks no other), so a UNORM or float image takes the encoded
    /// values as they are and an sRGB image takes linear ones, which the device encodes when it writes them.
    pub fn clear_value(&self, format: AttachmentFormat) -> [f32; 4] {
        let [r, g, b] = self.background.srgb.map(|channel| f32::from(channel) / 255.0);
        if is_srgb(format) {
            [linear(r), linear(g), linear(b), 1.0]
        } else {
            [r, g, b, 1.0]
        }
    }

    /// Record the frame on `image`, an image of `format` in layout `from` now, and leave it in `to`. Needs transfer
    /// destination use (VUID-vkCmdClearColorImage-image-00002); the owner refuses an image without it
    /// ([`RecordRefusal::UsageMissing`]) and chooses the layout the clear itself runs in
    /// (VUID-vkCmdClearColorImage-imageLayout-01394).
    ///
    /// # Errors
    ///
    /// [`RecordRefusal`] from the owner; nothing was recorded.
    pub fn record(
        &self,
        recording: &Recording<'_>,
        image: &impl ImageTarget,
        format: AttachmentFormat,
        from: ImageLayout,
        to: ImageLayout,
    ) -> Result<(), RecordRefusal> {
        recording.clear(image, from, self.clear_value(format), to)
    }

    /// What one texel of the frame reads back as, in an image of `format`.
    pub fn texel(&self, format: AttachmentFormat) -> Texel {
        let [r, g, b] = self.background.srgb;
        match format {
            AttachmentFormat::Rgba8Unorm | AttachmentFormat::Rgba8Srgb => {
                Texel::from_slice(&[r, g, b, 255])
            }
            AttachmentFormat::Bgra8Unorm | AttachmentFormat::Bgra8Srgb => {
                Texel::from_slice(&[b, g, r, 255])
            }
            AttachmentFormat::Rgba16Float => {
                let [r, g, b, a] = self.clear_value(format).map(f16_bits);
                let mut bytes = [0; 8];
                for (at, half) in [r, g, b, a].into_iter().enumerate() {
                    bytes[at * 2..at * 2 + 2].copy_from_slice(&half.to_le_bytes());
                }
                Texel::from_slice(&bytes)
            }
        }
    }

    /// The whole frame as a readback of an image of `format` and `extent` lays it out: rows after each other, no
    /// padding (`Recording::read_back`).
    pub fn expected(&self, format: AttachmentFormat, extent: Extent) -> Vec<u8> {
        let texel = self.texel(format);
        let count = usize::try_from(extent.texels()).unwrap_or(usize::MAX);
        texel.bytes().repeat(count)
    }
}

fn is_srgb(format: AttachmentFormat) -> bool {
    matches!(
        format,
        AttachmentFormat::Rgba8Srgb | AttachmentFormat::Bgra8Srgb
    )
}

/// The sRGB transfer function, inverted (IEC 61966-2-1).
fn linear(encoded: f32) -> f32 {
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

/// The IEEE 754 half-precision bits of `value`, rounded to nearest, ties to even.
fn f16_bits(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32;
    let mantissa = bits & 0x7f_ffff;
    if exponent == 0xff {
        return sign | 0x7c00 | if mantissa == 0 { 0 } else { 0x200 };
    }
    let half_exponent = exponent - 127 + 15;
    if half_exponent >= 0x1f {
        return sign | 0x7c00;
    }
    let (kept, shift) = if half_exponent <= 0 {
        if half_exponent < -10 {
            return sign;
        }
        // Subnormal: the implicit one joins the mantissa, which then counts in units of 2^-24.
        (mantissa | 0x80_0000, (14 - half_exponent) as u32)
    } else {
        (((half_exponent as u32) << 23) | mantissa, 13)
    };
    let half = kept >> shift;
    let rest = kept & ((1 << shift) - 1);
    let middle = 1 << (shift - 1);
    let rounded = if rest > middle || (rest == middle && half & 1 == 1) {
        half + 1
    } else {
        half
    };
    // A carry out of the mantissa raises the exponent, up to infinity: the format's own rounding.
    sign | rounded as u16
}

/// Where a readback first differs from its expectation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mismatch {
    /// Column.
    pub x: u32,
    /// Row.
    pub y: u32,
    /// What the device wrote.
    pub found: Texel,
    /// What the frame says.
    pub expected: Texel,
}

/// A readback compared with its expectation, texel by texel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Comparison {
    /// Texels compared.
    pub texels: u64,
    /// Texels with a channel off by more than [`TOLERANCE`].
    pub mismatched: u64,
    /// The first of them, in row order.
    pub first: Option<Mismatch>,
}

impl Comparison {
    /// Every texel is what it should be.
    pub fn matches(&self) -> bool {
        self.texels > 0 && self.mismatched == 0
    }
}

/// Why two images were not compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompareRefusal {
    /// A byte count is not that of `extent` texels of `format`.
    Length {
        /// What the extent and format need.
        needed: u64,
        /// What the readback holds.
        found: u64,
        /// What the expectation holds.
        expected: u64,
    },
}

/// Compare `found` with `expected`, two images of `format` and `extent` laid out as a readback lays them out, texel by
/// texel, each channel within [`TOLERANCE`].
///
/// # Errors
///
/// [`CompareRefusal::Length`] when either does not hold exactly `extent` texels.
pub fn compare_texels(
    format: AttachmentFormat,
    extent: Extent,
    found: &[u8],
    expected: &[u8],
) -> Result<Comparison, CompareRefusal> {
    let size = format.texel_bytes() as usize;
    let needed = extent.texels().saturating_mul(size as u64);
    if found.len() as u64 != needed || expected.len() as u64 != needed {
        return Err(CompareRefusal::Length {
            needed,
            found: found.len() as u64,
            expected: expected.len() as u64,
        });
    }
    let half = format == AttachmentFormat::Rgba16Float;
    let mut comparison = Comparison {
        texels: extent.texels(),
        mismatched: 0,
        first: None,
    };
    let width = extent.width.max(1) as usize;
    for (index, (seen, wanted)) in found
        .chunks_exact(size)
        .zip(expected.chunks_exact(size))
        .enumerate()
    {
        if within(seen, wanted, half) {
            continue;
        }
        comparison.mismatched += 1;
        if comparison.first.is_none() {
            comparison.first = Some(Mismatch {
                x: (index % width) as u32,
                y: (index / width) as u32,
                found: Texel::from_slice(seen),
                expected: Texel::from_slice(wanted),
            });
        }
    }
    Ok(comparison)
}

fn within(seen: &[u8], wanted: &[u8], half: bool) -> bool {
    if half {
        seen.chunks_exact(2).zip(wanted.chunks_exact(2)).all(|(a, b)| {
            let (a, b) = (
                u16::from_le_bytes([a[0], a[1]]),
                u16::from_le_bytes([b[0], b[1]]),
            );
            a.abs_diff(b) <= TOLERANCE
        })
    } else {
        seen.iter()
            .zip(wanted)
            .all(|(a, b)| u16::from(a.abs_diff(*b)) <= TOLERANCE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DARK: ClearFrame = ClearFrame::new(Background::DARK);

    #[test]
    fn eight_bit_texels_are_the_theme_bytes_in_the_formats_channel_order() {
        assert_eq!(
            DARK.texel(AttachmentFormat::Bgra8Unorm).bytes(),
            [0x27, 0x21, 0x16, 0xff]
        );
        assert_eq!(
            DARK.texel(AttachmentFormat::Rgba8Unorm).bytes(),
            [0x16, 0x21, 0x27, 0xff]
        );
        assert_eq!(
            DARK.texel(AttachmentFormat::Bgra8Srgb),
            DARK.texel(AttachmentFormat::Bgra8Unorm),
            "an sRGB image encodes the linear clear back to the same bytes"
        );
    }

    #[test]
    fn unorm_clears_are_the_encoded_values_and_srgb_clears_linear_ones() {
        let unorm = DARK.clear_value(AttachmentFormat::Bgra8Unorm);
        assert_eq!(unorm, [22.0 / 255.0, 33.0 / 255.0, 39.0 / 255.0, 1.0]);
        let srgb = DARK.clear_value(AttachmentFormat::Bgra8Srgb);
        for (linear, encoded) in srgb.iter().zip(unorm).take(3) {
            assert!(*linear < encoded, "{linear} {encoded}");
        }
        // Encoding the linear value again gives the byte back.
        let encode = |linear: f32| {
            if linear <= 0.003_130_8 {
                linear * 12.92
            } else {
                1.055 * linear.powf(1.0 / 2.4) - 0.055
            }
        };
        for (linear, byte) in srgb.iter().zip(Background::DARK.srgb) {
            assert_eq!((encode(*linear) * 255.0).round() as u8, byte);
        }
    }

    #[test]
    fn half_floats_round_to_nearest_even() {
        assert_eq!(f16_bits(1.0), 0x3c00);
        assert_eq!(f16_bits(0.5), 0x3800);
        assert_eq!(f16_bits(0.0), 0);
        assert_eq!(f16_bits(-2.0), 0xc000);
        assert_eq!(f16_bits(65504.0), 0x7bff);
        assert_eq!(f16_bits(70000.0), 0x7c00);
        assert_eq!(f16_bits(1.0 / 3.0), 0x3555);
        assert_eq!(f16_bits(1.0e-7), 0x0002, "subnormal");
        assert_eq!(f16_bits(1.0e-9), 0, "below the smallest subnormal");
        assert_eq!(f16_bits(f32::NAN) & 0x7c00, 0x7c00);
        let texel = DARK.texel(AttachmentFormat::Rgba16Float);
        assert_eq!(&texel.bytes()[6..], 0x3c00u16.to_le_bytes(), "opaque");
    }

    #[test]
    fn the_expected_frame_is_one_texel_per_texel() {
        let extent = Extent {
            width: 3,
            height: 2,
        };
        let frame = DARK.expected(AttachmentFormat::Rgba16Float, extent);
        assert_eq!(frame.len(), 6 * 8);
        assert!(frame
            .chunks_exact(8)
            .all(|texel| texel == DARK.texel(AttachmentFormat::Rgba16Float).bytes()));
    }

    #[test]
    fn a_comparison_finds_the_first_texel_off_by_more_than_one_step() {
        let format = AttachmentFormat::Bgra8Unorm;
        let extent = Extent {
            width: 4,
            height: 3,
        };
        let expected = DARK.expected(format, extent);
        let mut found = expected.clone();
        found[0] += 1;
        let close = compare_texels(format, extent, &found, &expected).unwrap();
        assert!(close.matches(), "one step is within the tolerance");
        found[4 * 6 + 2] += 2;
        found[4 * 11] = 0;
        let off = compare_texels(format, extent, &found, &expected).unwrap();
        assert_eq!((off.texels, off.mismatched), (12, 2));
        let first = off.first.unwrap();
        assert_eq!((first.x, first.y), (2, 1));
        assert_eq!(first.expected, DARK.texel(format));
        let other = ClearFrame::new(Background::LIGHT).expected(format, extent);
        let all = compare_texels(format, extent, &other, &expected).unwrap();
        assert_eq!(all.mismatched, 12, "another background differs everywhere");
    }

    #[test]
    fn half_float_comparisons_allow_one_step_per_channel() {
        let format = AttachmentFormat::Rgba16Float;
        let extent = Extent {
            width: 1,
            height: 1,
        };
        let expected = DARK.expected(format, extent);
        let mut found = expected.clone();
        let step = |bytes: &mut [u8], by: u16| {
            let value = u16::from_le_bytes([bytes[0], bytes[1]]) + by;
            bytes.copy_from_slice(&value.to_le_bytes());
        };
        step(&mut found[0..2], 1);
        assert!(compare_texels(format, extent, &found, &expected)
            .unwrap()
            .matches());
        step(&mut found[2..4], 2);
        assert_eq!(
            compare_texels(format, extent, &found, &expected)
                .unwrap()
                .mismatched,
            1
        );
    }

    #[test]
    fn images_of_another_size_are_not_compared() {
        let format = AttachmentFormat::Rgba8Unorm;
        let extent = Extent {
            width: 2,
            height: 2,
        };
        let expected = DARK.expected(format, extent);
        let refused = compare_texels(format, extent, &expected[..12], &expected);
        assert_eq!(
            refused,
            Err(CompareRefusal::Length {
                needed: 16,
                found: 12,
                expected: 16
            })
        );
        let nothing = compare_texels(format, Extent { width: 0, height: 0 }, &[], &[]).unwrap();
        assert!(!nothing.matches(), "an empty image proves nothing");
    }
}
