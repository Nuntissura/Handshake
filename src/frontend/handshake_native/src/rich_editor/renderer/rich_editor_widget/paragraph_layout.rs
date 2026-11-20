//! Measured paragraph extents remain valid after a one-shot reference scroll completes.
use crate::rich_editor::document_owner::EditorSaveContext;

#[derive(PartialEq)]
struct LayoutKey {
    context: EditorSaveContext,
    width: f32,
    font_size: f32,
    pixels_per_point: f32,
    reading: bool,
}

#[derive(Default)]
pub(super) struct ParagraphLayout {
    key: Option<LayoutKey>,
    fonts: Option<egui::FontDefinitions>,
    heights: Vec<Option<f32>>,
}

impl ParagraphLayout {
    pub(super) fn prepare(
        &mut self,
        ctx: &egui::Context,
        context: Option<EditorSaveContext>,
        width: f32,
        font_size: f32,
        reading: bool,
        blocks: usize,
    ) {
        let key = context.map(|context| LayoutKey {
            context,
            width,
            font_size,
            pixels_per_point: ctx.pixels_per_point(),
            reading,
        });
        let fonts = ctx.fonts(|fonts| fonts.definitions().clone());
        let same_fonts = self.fonts.as_ref().is_some_and(|previous| {
            previous.families == fonts.families
                && previous.font_data.len() == fonts.font_data.len()
                && previous.font_data.iter().zip(&fonts.font_data).all(
                    |((old_name, old), (name, current))| {
                        old_name == name && std::sync::Arc::ptr_eq(old, current)
                    },
                )
        });
        if key.is_none() || self.key != key || !same_fonts {
            self.heights.clear();
        }
        self.key = key;
        self.fonts = Some(fonts);
        self.heights.resize(blocks, None);
    }

    pub(super) fn height(&self, index: usize) -> Option<f32> {
        self.key.as_ref()?;
        self.heights.get(index).copied().flatten()
    }

    pub(super) fn record(&mut self, index: usize, height: f32) {
        if self.key.is_some() {
            if let Some(slot) = self.heights.get_mut(index) {
                *slot = (height.is_finite() && height > 0.0).then_some(height);
            }
        }
    }
}

#[cfg(test)]
mod tests;
