//! WP-KERNEL-012 MT-132 AC-132-4: the reachability guard.
//!
//! A widget can be AccessKit-DISCOVERABLE at bounds `X` while not being pointer-HITTABLE at `X`.
//! Two independent defects in this work packet had exactly that shape, and both survived every
//! existing proof — because those proofs assert node presence, role and label, none of which
//! notices that a click at the node's own advertised centre is delivered somewhere else:
//!
//! * MT-132 — the editor-body context surface covered the sticky band and was registered AFTER it,
//!   so egui's last-widget-wins tie-break (`hit_test.rs:436-440`) handed every sticky-header click
//!   to the context surface. The header `Label` never reported `clicked()`.
//! * MT-133 — the rename Apply button fell outside its window's clip rect, which empties
//!   `interact_rect` (`ui.rs:1140`) and drops it from hit-testing (`hit_test.rs:421-422`), while
//!   `response.rs:836-841` kept publishing the UNCLIPPED bounds to AccessKit.
//!
//! In both cases the tree said the widget was there and the hit test said nothing was.

use egui_kittest::kittest::{NodeT, Queryable};

/// Click the widget carrying `label` at the centre of the bounds it advertises to AccessKit, and
/// assert that THIS widget is the one egui actually delivered the click to.
///
/// Use this in place of `harness.get_by_label(label).click()` wherever a click is the thing under
/// test. It performs the same click, so it costs nothing extra, and it converts a silent no-op into
/// a named failure.
///
/// ## Why it reads `clicked` and nothing else
///
/// Proven the hard way, by restoring the MT-132 shadowing and watching the guard stay green:
///
/// * `contains_pointer` holds EVERY widget whose rect covers the point (`hit_test.rs:20-27` calls it
///   "both a Window and the Button in it"), so a shadowed widget is still a member. Vacuous.
/// * `hovered` likewise resolves to a SET including ancestry, so the shadowed header remained in it.
///   Also vacuous.
/// * `WidgetHits::click` is the exact "if the user clicked now, this is what would be clicked"
///   answer, but it lives on the private viewport state with no public accessor.
///
/// So the guard performs a real press/release and reads `InteractionSnapshot::clicked`, which is a
/// single `Option<Id>` — the widget that actually received it. That is a positive control on the
/// real hit path rather than a re-derivation of it, so it cannot drift from the rules it checks.
pub fn click_label_asserting_it_receives_the_click<T>(
    harness: &mut egui_kittest::Harness<'_, T>,
    label: &str,
) {
    let (target_bits, centre) = {
        let node = harness.get_by_label(label);
        (node.accesskit_node().id().0, node.rect().center())
    };
    click_resolved_target(harness, label, target_bits, centre);
}

/// The `author_id` variant of [`click_label_asserting_it_receives_the_click`], for the surfaces this
/// packet addresses by stable id rather than by visible label.
pub fn click_author_id_asserting_it_receives_the_click<T>(
    harness: &mut egui_kittest::Harness<'_, T>,
    author_id: &str,
) {
    let (target_bits, centre) = {
        let node = harness
            .root()
            .children_recursive()
            .find(|node| node.accesskit_node().author_id() == Some(author_id))
            .unwrap_or_else(|| panic!("reachability guard: no live node for author_id {author_id}"));
        (node.accesskit_node().id().0, node.rect().center())
    };
    click_resolved_target(harness, author_id, target_bits, centre);
}

fn click_resolved_target<T>(
    harness: &mut egui_kittest::Harness<'_, T>,
    label: &str,
    target_bits: u64,
    centre: egui::Pos2,
) {

    let modifiers = egui::Modifiers::default();
    for pressed in [true, false] {
        harness.input_mut().events.push(egui::Event::PointerButton {
            pos: centre,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        });
    }
    harness.step();

    let receiver = harness.ctx.interaction_snapshot(|snapshot| snapshot.clicked);
    let reached_the_target = receiver.is_some_and(|id| id.value() == target_bits);

    assert!(
        reached_the_target,
        "reachability guard: {label:?} advertises AccessKit bounds centred at {centre:?}, but the \
         click at that exact point was delivered to {receiver:?}, not to it. The node is \
         discoverable and not clickable — either a later same-layer widget shadows it (egui \
         hit_test.rs:436-440) or it is clipped out of hit-testing (ui.rs:1140, hit_test.rs:421-422) \
         while still publishing unclipped bounds (response.rs:836-841)."
    );
}
