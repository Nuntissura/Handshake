mod model;
mod schema;

pub const MOODBOARD_SCHEMA_ID: &str = "hsk.atelier.moodboard@1";
pub const MOODBOARD_SCHEMA_VERSION: i64 = 1;

pub use model::{
    MoodboardDocument, MoodboardCanvas, MoodboardPoint, MoodboardSize, MoodboardLayer, MoodboardImageElement, MoodboardTextElement, MoodboardShapeElement, MoodboardConnector, MoodboardFolder, MoodboardGuide, MoodboardFlags, MoodboardStyle, MoodboardHistoryEntry,
    parse_and_validate_moodboard, require_non_empty_trimmed,
};
