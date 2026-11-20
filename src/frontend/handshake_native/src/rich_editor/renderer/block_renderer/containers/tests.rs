use super::*;
use crate::theme::HsTheme;

fn container(kind:NodeKind,children:Vec<BlockNode>)->BlockNode {
    BlockNode::with_children(kind,children.into_iter().map(Child::Block).collect())
}
struct PaintedText { text:String,origin:egui::Pos2,rect:Rect,clip:Rect }
fn render(block:&BlockNode,target:&[usize],caret:Option<&[usize]>,width:f32)
    ->(TargetedBlockPaint,Vec<PaintedText>,Vec<Rect>) {
    render_at_font(block,target,caret,width,16.0)
}
fn render_at_font(block:&BlockNode,target:&[usize],caret:Option<&[usize]>,width:f32,font:f32)
    ->(TargetedBlockPaint,Vec<PaintedText>,Vec<Rect>) {
    let ctx=egui::Context::default();
    crate::app::HandshakeApp::install_fonts(&ctx);
    let mut painted=None;
    let output=ctx.run(Default::default(),|ctx| {
        painted=Some(paint_block_with_target(&ctx.layer_painter(egui::LayerId::background()),block,
            egui::pos2(20.0,30.0),width,&HsTheme::Dark.palette(),caret.map(|_|1),
            line_layout::bold_family_available(ctx),font,line_layout::AtomPaint::ChipCovered,caret,Some(target)));
    });
    fn collect(shape:&egui::Shape,clip:Rect,text:&mut Vec<PaintedText>,rects:&mut Vec<Rect>) {
        match shape {
            egui::Shape::Text(shape)=>text.push(PaintedText {text:shape.galley.job.text.clone(),origin:shape.pos,
                rect:shape.galley.rect.translate(shape.pos.to_vec2()),clip}),
            egui::Shape::Rect(shape)=>rects.push(shape.rect),
            egui::Shape::Vec(shapes)=>for shape in shapes {collect(shape,clip,text,rects)},
            _=>(),
        }
    }
    let mut text=Vec::new();let mut rects=Vec::new();
    for shape in &output.shapes {collect(&shape.shape,shape.clip_rect,&mut text,&mut rects)}
    (painted.unwrap(),text,rects)
}

#[test]
fn list_later_child_and_nested_item_keep_model_paths_and_actual_caret_geometry() {
    let nested=container(NodeKind::OrderedList,vec![container(NodeKind::ListItem,
        vec![BlockNode::paragraph("nested-first"),BlockNode::paragraph("nested-later")])]);
    let list=container(NodeKind::BulletList,vec![container(NodeKind::ListItem,
        vec![BlockNode::paragraph("outer-first"),nested]),container(NodeKind::TaskItem,
        vec![BlockNode::paragraph("task-first"),BlockNode::paragraph("task-later")])]);
    let (painted,text,_)=render(&list,&[0,1,0,1],Some(&[0,1,0,1,0]),300.0);
    for label in ["outer-first","nested-first","nested-later","task-first","task-later"] {
        assert!(text.iter().any(|text|text.text==label),"missing {label}");
    }
    let target=text.iter().find(|text|text.text=="nested-later").unwrap();
    assert_eq!(painted.target_rect.unwrap().min,target.origin);
    assert_eq!(target.origin.x,20.0+2.0*LIST_INDENT_PTS);
    let (galley,origin)=painted.paint.caret_galley.unwrap();
    assert_eq!(galley.job.text,"nested-later");assert_eq!(origin,target.origin);
    let item=render(&list,&[1],None,300.0).0.target_rect.unwrap();
    let task=text.iter().find(|text|text.text=="task-later").unwrap();
    assert!(item.contains(task.origin));
    assert!(item.max.y>=task.rect.max.y);
}

fn table()->BlockNode {
    container(NodeKind::Table,vec![
        container(NodeKind::TableRow,vec![
            container(NodeKind::TableCell,vec![BlockNode::paragraph("first cell child"),
                BlockNode::paragraph("later cell text wraps to several lines at a narrow column width")]),
            container(NodeKind::TableHeader,vec![BlockNode::paragraph("neighbor header")])]),
        container(NodeKind::TableRow,vec![container(NodeKind::TableCell,vec![BlockNode::paragraph("next row")])])])
}

#[test]
fn table_later_children_set_row_height_and_remain_clipped_to_their_own_column() {
    let (painted,text,_)=render(&table(),&[0,0,1],Some(&[0,0,1,0]),260.0);
    let later=text.iter().find(|text|text.text.starts_with("later cell")).unwrap();
    let neighbor=text.iter().find(|text|text.text=="neighbor header").unwrap();
    let next=text.iter().find(|text|text.text=="next row").unwrap();
    assert_eq!(painted.target_rect.unwrap().min,later.origin);
    assert!(later.clip.max.x<=neighbor.origin.x);
    assert!(next.origin.y>later.rect.max.y,"later cell content must not overlap the next row");
    assert_eq!(painted.paint.caret_galley.unwrap().0.job.text,later.text);
    let cell=render(&table(),&[0,0],None,260.0).0.target_rect.unwrap();
    let (row,_,borders)=render(&table(),&[0],None,260.0);
    let row=row.target_rect.unwrap();
    assert!(borders.contains(&cell),"cell target must be its actual painted border");
    assert_eq!(row.height(),cell.height());
    assert!(row.contains(neighbor.origin));
    assert!(cell.max.y>=later.rect.max.y);
}

#[test]
fn cell_nested_quote_and_reflow_keep_exact_descendant_rectangles() {
    let cell=container(NodeKind::TableCell,vec![BlockNode::paragraph(&"prefix wraps ".repeat(12)),
        container(NodeKind::Blockquote,vec![BlockNode::paragraph("quote-first"),BlockNode::paragraph("quote-target")])]);
    let table=container(NodeKind::Table,vec![container(NodeKind::TableRow,vec![cell])]);
    let (wide,_,_)=render(&table,&[0,0,1,1],None,500.0);
    let (narrow,text,_)=render(&table,&[0,0,1,1],None,140.0);
    let target=text.iter().find(|text|text.text=="quote-target").unwrap();
    assert_eq!(narrow.target_rect.unwrap().min,target.origin);
    assert!(narrow.target_rect.unwrap().min.y>wide.target_rect.unwrap().min.y);
    assert!(narrow.paint.height>wide.paint.height);
}

#[test]
fn absent_and_unpainted_descendants_do_not_resolve_to_list_item_row_or_cell_rectangles() {
    let list=container(NodeKind::BulletList,vec![container(NodeKind::ListItem,vec![BlockNode::paragraph("visible")])]);
    for path in [vec![9],vec![0,9],vec![0,0,0]] {
        assert!(render(&list,&path,None,220.0).0.target_rect.is_none());
    }
    for path in [vec![9],vec![0,9],vec![0,0,9],vec![0,0,0,0]] {
        assert!(render(&table(),&path,None,220.0).0.target_rect.is_none());
    }
    let mut embed=BlockNode::new(NodeKind::HsEmbed);
    embed.attrs.insert("refValue".into(),serde_json::json!("owning-note"));
    let unsupported=container(NodeKind::Table,vec![container(NodeKind::TableRow,vec![
        container(NodeKind::TableCell,vec![embed])])]);
    crate::rich_editor::document_model::schema::validate_tree(&BlockNode::doc(vec![unsupported.clone()])).unwrap();
    assert!(render(&unsupported,&[0,0,0],None,220.0).0.target_rect.is_none());
}


#[test]
fn schema_legal_standalone_structures_paint_all_children_and_keep_exact_paths() {
    for kind in [NodeKind::ListItem,NodeKind::TaskItem,NodeKind::TableRow,NodeKind::TableCell,NodeKind::TableHeader] {
        let children=vec![BlockNode::paragraph(&"preceding wrapped content ".repeat(10)),BlockNode::paragraph("target child")];
        let (block,path)=if kind==NodeKind::TableRow {
            (container(kind,vec![container(NodeKind::TableCell,children),
                container(NodeKind::TableHeader,vec![BlockNode::paragraph("neighbor")])]),vec![0,1])
        } else { (container(kind,children),vec![1]) };
        crate::rich_editor::document_model::schema::validate_tree(&BlockNode::doc(vec![block.clone()])).unwrap();
        let mut caret_path=path.clone();caret_path.push(0);
        let (wide,_,_)=render(&block,&path,None,600.0);
        let (narrow,text,_)=render(&block,&path,Some(&caret_path),180.0);
        let painted_target=text.iter().find(|text|text.text=="target child").unwrap();
        assert!(text.iter().any(|text|text.text.starts_with("preceding wrapped content")),"{kind:?}");
        assert_eq!(narrow.target_rect.unwrap().min,painted_target.origin,"{kind:?}");
        assert!(narrow.target_rect.unwrap().max.y>=painted_target.rect.max.y,"{kind:?}");
        assert!(narrow.target_rect.unwrap().min.y>wide.target_rect.unwrap().min.y,"{kind:?}");
        let (galley,origin)=narrow.paint.caret_galley.unwrap();
        assert_eq!(galley.job.text,"target child");assert_eq!(origin,painted_target.origin);
        let (own,_,borders)=render(&block,&[],None,180.0);
        let own=own.target_rect.unwrap();
        assert!(own.contains(painted_target.origin));assert!(own.max.y>=painted_target.rect.max.y);
        if matches!(kind,NodeKind::TableCell|NodeKind::TableHeader) {
            assert!(borders.contains(&own),"cell identity must use its painted border");
        }
        if kind==NodeKind::TableRow {
            let neighbor=text.iter().find(|text|text.text=="neighbor").unwrap();
            assert!(painted_target.clip.max.x<=neighbor.origin.x);
            assert!(own.contains(neighbor.origin));
        }
        assert!(render(&block,&[90],None,180.0).0.target_rect.is_none());
    }
}

#[test]
fn standalone_structures_nested_in_legal_containers_keep_descendant_caret_geometry() {
    let cell=container(NodeKind::TableCell,vec![BlockNode::paragraph("cell first"),
        container(NodeKind::TaskItem,vec![BlockNode::paragraph("task first"),BlockNode::paragraph("nested target")])]);
    let block=container(NodeKind::Blockquote,vec![cell]);
    crate::rich_editor::document_model::schema::validate_tree(&BlockNode::doc(vec![block.clone()])).unwrap();
    let (painted,text,borders)=render(&block,&[0,1,1],Some(&[0,1,1,0]),250.0);
    let target=text.iter().find(|text|text.text=="nested target").unwrap();
    assert_eq!(painted.target_rect.unwrap().min,target.origin);
    let (galley,origin)=painted.paint.caret_galley.unwrap();
    assert_eq!(galley.job.text,"nested target");assert_eq!(origin,target.origin);
    let cell=render(&block,&[0],None,250.0).0.target_rect.unwrap();
    assert!(borders.contains(&cell));assert!(cell.max.y>=target.rect.max.y);
}

fn font_row_height(font:f32)->f32 {
    let ctx=egui::Context::default();
    crate::app::HandshakeApp::install_fonts(&ctx);
    let mut height=0.0;
    let _=ctx.run(Default::default(),|ctx| {
        height=ctx.fonts_mut(|fonts|fonts.row_height(&FontId::proportional(font)));
    });
    height
}

#[test]
fn hard_break_reserves_a_real_blank_line_without_inventing_text_or_descendant_targets() {
    let line_break=BlockNode::new(NodeKind::HardBreak);
    for font in [16.0,27.0] {
        let row_height=font_row_height(font);
        let (single,text,borders)=render_at_font(&line_break,&[],Some(&[]),220.0,font);
        assert!(text.is_empty());assert!(borders.is_empty());assert!(single.paint.caret_galley.is_none());
        assert!((single.target_rect.unwrap().height()-row_height).abs()<0.01);
        assert!((single.paint.height-row_height-BLOCK_GAP_PTS).abs()<0.01);
        assert!(render_at_font(&line_break,&[0],None,220.0,font).0.target_rect.is_none());
        let block=container(NodeKind::TableCell,vec![BlockNode::paragraph("before"),line_break.clone(),BlockNode::paragraph("after")]);
        crate::rich_editor::document_model::schema::validate_tree(&BlockNode::doc(vec![block.clone()])).unwrap();
        let (painted,text,_)=render_at_font(&block,&[1],Some(&[1]),220.0,font);
        let before=text.iter().find(|text|text.text=="before").unwrap();
        let after=text.iter().find(|text|text.text=="after").unwrap();
        let blank=painted.target_rect.unwrap();
        assert!(blank.min.y>=before.rect.max.y);
        assert!((blank.height()-row_height).abs()<0.01);
        assert!((after.origin.y-blank.max.y-BLOCK_GAP_PTS).abs()<0.01);
        assert!(painted.paint.caret_galley.is_none());
    }
}
