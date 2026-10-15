mod openpose;
mod raster;

pub use openpose::{openpose_points, render_posekit_openpose_png};

/// OpenPose keypoint-array cardinalities (legacy source `rigToOpenposeJson`):
/// body-18, face-70, hand-21. Each keypoint is an `(x, y, confidence)` triple,
/// so flattened arrays have `count * 3` numbers.
pub const BODY_KEYPOINT_COUNT: usize = 18;
pub const FACE_KEYPOINT_COUNT: usize = 70;
pub const HAND_KEYPOINT_COUNT: usize = 21;
pub const POSEKIT_OPENPOSE_EXPORT_WIDTH: i32 = 768;
pub const POSEKIT_OPENPOSE_EXPORT_HEIGHT: i32 = 768;
