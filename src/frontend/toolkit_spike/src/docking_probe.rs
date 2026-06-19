// Probe (b): docking layout save/restore.
// REAL test: build a horizontal split of two panes with non-default shares (0.30/0.70),
// serialize the egui_tiles::Tree to JSON, drop it, deserialize a fresh Tree from the blob
// (simulating window destroy+recreate), and assert tile count + pane proportions round-trip
// within tolerance. No GUI/window needed — this is a deterministic data round-trip, which is
// exactly what "save layout to a JSON blob and restore it later" must guarantee.

use egui_tiles::{Container, Tile, Tiles, Tree};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpikePane {
    Left,
    Main,
}

pub struct ProbeResult {
    pub pass: bool,
    pub notes: String,
}

const LEFT_SHARE: f32 = 0.30;
const MAIN_SHARE: f32 = 0.70;
const TOL: f32 = 0.01;

fn read_linear_shares(tree: &Tree<SpikePane>) -> Option<(f32, f32, usize, usize)> {
    // Locate the root linear container and read the normalized shares of its two children.
    let root = tree.root?;
    if let Some(Tile::Container(Container::Linear(lin))) = tree.tiles.get(root) {
        let children = &lin.children;
        if children.len() != 2 {
            return None;
        }
        let a = lin.shares[children[0]];
        let b = lin.shares[children[1]];
        // normalize to proportions
        let sum = a + b;
        if sum <= 0.0 {
            return None;
        }
        return Some((a / sum, b / sum, children.len(), tree.tiles.len()));
    }
    None
}

pub fn run() -> ProbeResult {
    // 1. Build a docked layout: left sidebar + main area in a horizontal split.
    let mut tiles: Tiles<SpikePane> = Tiles::default();
    let left = tiles.insert_pane(SpikePane::Left);
    let main = tiles.insert_pane(SpikePane::Main);
    let root = tiles.insert_horizontal_tile(vec![left, main]);

    // 2. Set non-default proportions so the round-trip actually proves data preservation
    //    (default would be 50/50 and could pass trivially).
    if let Some(Tile::Container(Container::Linear(lin))) = tiles.get_mut(root) {
        lin.shares.set_share(left, LEFT_SHARE);
        lin.shares.set_share(main, MAIN_SHARE);
    } else {
        return ProbeResult {
            pass: false,
            notes: "root tile was not a Linear container".into(),
        };
    }

    let tree = Tree::new("spike_dock", root, tiles);
    let before = match read_linear_shares(&tree) {
        Some(v) => v,
        None => {
            return ProbeResult {
                pass: false,
                notes: "could not read shares before serialize".into(),
            }
        }
    };
    let original_tile_count = tree.tiles.len();

    // 3. Serialize the layout to a JSON blob (what a "save layout" button stores).
    let blob = match serde_json::to_string(&tree) {
        Ok(s) => s,
        Err(e) => {
            return ProbeResult {
                pass: false,
                notes: format!("serialize failed: {e}"),
            }
        }
    };

    // 4. Destroy + recreate: deserialize a brand-new Tree from the blob.
    drop(tree);
    let restored: Tree<SpikePane> = match serde_json::from_str(&blob) {
        Ok(t) => t,
        Err(e) => {
            return ProbeResult {
                pass: false,
                notes: format!("deserialize failed: {e}"),
            }
        }
    };

    // 5. Assert tile count and proportions survived the round-trip.
    let after = match read_linear_shares(&restored) {
        Some(v) => v,
        None => {
            return ProbeResult {
                pass: false,
                notes: "could not read shares after restore".into(),
            }
        }
    };

    let count_ok = restored.tiles.len() == original_tile_count && after.2 == 2;
    let left_ok = (after.0 - LEFT_SHARE).abs() <= TOL;
    let main_ok = (after.1 - MAIN_SHARE).abs() <= TOL;
    let pass = count_ok && left_ok && main_ok;

    ProbeResult {
        pass,
        notes: format!(
            "before(left={:.3},main={:.3},children={},tiles={}) -> restored(left={:.3},main={:.3},children={},tiles={}); tol={}",
            before.0, before.1, before.2, before.3, after.0, after.1, after.2, after.3, TOL
        ),
    }
}
