//! Existing list/table presentation over model-indexed child painters.
use super::*;

// None denotes the block already dispatched, without a synthetic model-path component.
fn path_in(path: Option<&[usize]>, child_index: Option<usize>) -> Option<&[usize]> {
    match child_index {
        Some(index) => path.and_then(|path| path.strip_prefix(&[index])),
        None => path,
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_children(painter: &egui::Painter, container: &BlockNode, origin: egui::Pos2,
    width: f32, palette: &HsPalette, caret_offset: Option<usize>, bold: bool, font: f32,
    caret_path: Option<&[usize]>, target_path: Option<&[usize]>,
    mut embeds: Option<&mut crate::rich_editor::hs_embeds::Frame>) -> TargetedBlockPaint {
    let mut y=origin.y;
    let mut caret_galley=None;
    let mut target_rect=None;
    for (index, child) in container.children.iter().enumerate() {
        let Some(child)=child.as_block() else { continue };
        let caret_tail=caret_path.and_then(|path|path.strip_prefix(&[index]));
        let target_tail=target_path.and_then(|path|path.strip_prefix(&[index]));
        let painted=paint_block_with_embeds(painter,child,egui::pos2(origin.x,y),width,palette,
            caret_tail.and(caret_offset),bold,font,line_layout::AtomPaint::Visible,caret_tail,target_tail,embeds.as_deref_mut());
        y+=painted.paint.height;
        if painted.paint.caret_galley.is_some() { caret_galley=painted.paint.caret_galley; }
        if painted.target_rect.is_some() { target_rect=painted.target_rect; }
    }
    TargetedBlockPaint { paint:BlockPaint { height:(y-origin.y).max(font+BLOCK_GAP_PTS),caret_galley },target_rect }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_list(painter: &egui::Painter, block: &BlockNode, origin: egui::Pos2,
    width: f32, palette: &HsPalette, caret_offset: Option<usize>, bold: bool, font: f32,
    caret_path: Option<&[usize]>, target_path: Option<&[usize]>,
    mut embeds: Option<&mut crate::rich_editor::hs_embeds::Frame>) -> TargetedBlockPaint {
    let ordered=matches!(block.kind,NodeKind::OrderedList);
    let mut y=origin.y;
    let mut number=0;
    let mut caret_galley=None;
    let mut target_rect=None;
    let items: Vec<_> = if matches!(block.kind, NodeKind::ListItem | NodeKind::TaskItem) {
        vec![(None, block)]
    } else {
        block.children.iter().enumerate().filter_map(|(index, child)|
            child.as_block().filter(|item| matches!(item.kind, NodeKind::ListItem | NodeKind::TaskItem))
                .map(|item| (Some(index), item))).collect()
    };
    for (index,item) in items {
        number+=1;
        let prefix=if ordered { format!("{number}.") } else { "\u{2022}".into() };
        painter.text(egui::pos2(origin.x+LIST_INDENT_PTS*0.25,y),egui::Align2::LEFT_TOP,
            prefix,FontId::proportional(font),palette.text_subtle);
        let item_target=path_in(target_path,index);
        let item_caret=path_in(caret_path,index);
        let painted=paint_children(painter,item,egui::pos2(origin.x+LIST_INDENT_PTS,y),
            (width-LIST_INDENT_PTS).max(1.0),palette,item_caret.and(caret_offset),bold,font,
            item_caret,item_target,embeds.as_deref_mut());
        let height=(painted.paint.height-BLOCK_GAP_PTS).max(font);
        if item_target==Some(&[][..]) {
            target_rect=Some(Rect::from_min_size(egui::pos2(origin.x,y),Vec2::new(width,height)));
        } else if painted.target_rect.is_some() { target_rect=painted.target_rect; }
        if painted.paint.caret_galley.is_some() { caret_galley=painted.paint.caret_galley; }
        y+=height;
    }
    let height=(y-origin.y).max(font);
    if target_path==Some(&[][..]) && number>0 {
        target_rect=Some(Rect::from_min_size(origin,Vec2::new(width,height)));
    }
    TargetedBlockPaint { paint:BlockPaint { height:height+BLOCK_GAP_PTS,caret_galley },target_rect }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_table(painter: &egui::Painter, block: &BlockNode, origin: egui::Pos2,
    width: f32, palette: &HsPalette, caret_offset: Option<usize>, bold: bool, font: f32,
    caret_path: Option<&[usize]>, target_path: Option<&[usize]>,
    mut embeds: Option<&mut crate::rich_editor::hs_embeds::Frame>) -> TargetedBlockPaint {
    let rows: Vec<_> = if matches!(block.kind, NodeKind::Table) {
        block.children.iter().enumerate().filter_map(|(index, child)|
            child.as_block().filter(|row| matches!(row.kind, NodeKind::TableRow))
                .map(|row| (Some(index), row))).collect()
    } else {
        vec![(None, block)]
    };
    let columns=rows.iter().map(|(_,row)| {
        if matches!(row.kind, NodeKind::TableCell | NodeKind::TableHeader) { 1 }
        else { row.children.iter().filter(|child|
            child.as_block().is_some_and(|cell|matches!(cell.kind,NodeKind::TableCell|NodeKind::TableHeader))).count() }
    }).max().unwrap_or(0);
    if columns==0 {
        return TargetedBlockPaint { paint:BlockPaint { height:BLOCK_GAP_PTS,caret_galley:None },target_rect:None };
    }
    let column_width=width/columns as f32;
    let mut y=origin.y;
    let mut target_rect=None;
    let mut caret_galley=None;
    for (row_index,row) in rows {
        let row_target=path_in(target_path,row_index);
        let row_caret=path_in(caret_path,row_index);
        let mut row_height=font+8.0;
        let mut cells=Vec::new();
        let row_cells: Vec<_> = if matches!(row.kind, NodeKind::TableCell | NodeKind::TableHeader) {
            vec![(None, row)]
        } else {
            row.children.iter().enumerate().filter_map(|(index, child)|
                child.as_block().filter(|cell| matches!(cell.kind, NodeKind::TableCell | NodeKind::TableHeader))
                    .map(|cell| (Some(index), cell))).collect()
        };
        for (cell_index,cell) in row_cells {
            let x=origin.x+cells.len() as f32*column_width;
            let cell_target=path_in(row_target,cell_index);
            let cell_caret=path_in(row_caret,cell_index);
            // The column clips horizontally while its actual children determine the row's height.
            // The row ends after its tallest cell, so no painted child crosses into the next row.
            let cell_painter=painter.with_clip_rect(Rect::from_min_max(egui::pos2(x,y),
                egui::pos2(x+column_width,f32::INFINITY)));
            let painted=paint_children(&cell_painter,cell,egui::pos2(x+3.0,y+4.0),
                (column_width-6.0).max(1.0),palette,cell_caret.and(caret_offset),bold,font,
                cell_caret,cell_target,embeds.as_deref_mut());
            row_height=row_height.max(painted.paint.height-BLOCK_GAP_PTS+8.0);
            if painted.paint.caret_galley.is_some() { caret_galley=painted.paint.caret_galley; }
            cells.push((x,cell_target==Some(&[][..]),painted.target_rect));
        }
        for (x,is_target,child_target) in &cells {
            let rect=Rect::from_min_size(egui::pos2(*x,y),Vec2::new(column_width,row_height));
            painter.rect_stroke(rect,0.0,Stroke::new(1.0,palette.border),egui::StrokeKind::Inside);
            if *is_target { target_rect=Some(rect); }
            else if let Some(child_target)=child_target {
                let clipped=child_target.intersect(rect);
                if clipped.is_positive() { target_rect=Some(clipped); }
            }
        }
        if row_target==Some(&[][..]) && !cells.is_empty() {
            target_rect=Some(Rect::from_min_size(egui::pos2(origin.x,y),Vec2::new(cells.len() as f32*column_width,row_height)));
        }
        y+=row_height;
    }
    if target_path==Some(&[][..]) { target_rect=Some(Rect::from_min_size(origin,Vec2::new(width,y-origin.y))); }
    TargetedBlockPaint { paint:BlockPaint { height:y-origin.y+BLOCK_GAP_PTS,caret_galley },target_rect }
}

#[cfg(test)]
mod tests;
