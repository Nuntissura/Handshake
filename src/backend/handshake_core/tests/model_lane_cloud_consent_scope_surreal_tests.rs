// WP-1 MT-006 exact-scope and embedded-SurrealDB-only proof.

mod cloud_model_lane_surreal_support;

use cloud_model_lane_surreal_support::{exact_scope, projection, receipt, valid_window, Harness};

#[tokio::test]
async fn two_accounts_cannot_read_or_reuse_each_others_cloud_consent_authority() {
    let harness = Harness::create("owner-a").await;
    let (valid_from, valid_until) = valid_window();
    let plan = harness
        .store
        .record_cloud_projection_plan(projection("run-private", "lane-private", &harness.scope))
        .await
        .expect("owner A persists projection");
    harness
        .store
        .record_cloud_consent_receipt(receipt(
            "run-private",
            "lane-private",
            &plan.projection_plan_hash,
            &harness.scope,
            &valid_from,
            &valid_until,
        ))
        .await
        .expect("owner A persists consent");

    let owner_b_scope = exact_scope("owner-b");
    let owner_b = harness.store_for_scope(owner_b_scope.clone()).await;
    let replay = owner_b
        .replay_cloud_consent_authority("run-private")
        .await
        .expect("cross-account replay discloses no authority rows");
    assert!(replay.projection_plans.is_empty());
    assert!(replay.consent_receipts.is_empty());
    let reuse = owner_b
        .record_cloud_consent_receipt(receipt(
            "run-private",
            "lane-private",
            &plan.projection_plan_hash,
            &owner_b_scope,
            &valid_from,
            &valid_until,
        ))
        .await
        .expect_err("owner B cannot reuse owner A projection");
    assert!(reuse.to_string().contains("CX-MM-007"));
    harness.close().await;
}
