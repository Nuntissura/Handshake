use super::*;
use super::super::tests::embed;

const ENDPOINT: &str = "http://127.0.0.1:9911";
fn fixture() -> (Cache,DocumentEditOwner,Arc<AuthenticatedContext>,BlockNode) {
    let document = BlockNode::doc(vec![embed("occurrence", "source", "target")]);
    let owner = DocumentEditOwner::new(document.clone());
    let account = crate::local_account::mock_account_context(ENDPOINT);
    owner.bind_context("workspace", "destination", ENDPOINT, None, Some(account.clone())).unwrap();
    (Cache::default(),owner,account,document)
}

#[test]
fn changed_target_under_same_occurrence_does_not_reuse_or_accept_previous_preview() {
    let (mut cache,owner,account,document) = fixture();
    let old = cache.frame(&owner,Some(account.clone()),ENDPOINT,&document,24.0);
    let key = old.entries.keys().next().unwrap().clone();
    cache.entries.insert(key.clone(),Status::Ready { text:"old source".into(),source_version:1 });
    let delayed = cache.deliveries.clone();
    let mut candidate = owner.prepare_edit().unwrap();
    candidate.parts_mut().0.children[0].as_block_mut().unwrap().attrs.insert("target_block_id".into(),json!("different"));
    owner.commit_edit(candidate).unwrap();
    let document = owner.document_snapshot().unwrap();
    let current = cache.frame(&owner,Some(account.clone()),ENDPOINT,&document,24.0);
    assert!(!current.entries.contains_key(&key));
    assert!(current.entries.values().all(|(_,status)|matches!(status,Status::Idle)));
    delayed.lock().unwrap().push_back((key,Status::Ready { text:"late old source".into(),source_version:1 }));
    let current = cache.frame(&owner,Some(account),ENDPOINT,&document,24.0);
    assert!(current.entries.values().all(|(_,status)|matches!(status,Status::Idle)));
}

#[test]
fn owner_account_workspace_and_endpoint_changes_drop_pending_and_cached_content() {
    for change in ["owner","account","workspace","endpoint","revocation"] {
        let (mut cache,mut owner,mut account,document) = fixture();
        let original = cache.frame(&owner,Some(account.clone()),ENDPOINT,&document,24.0);
        let key = original.entries.keys().next().unwrap().clone();
        cache.entries.insert(key.clone(),Status::Ready { text:"private".into(),source_version:1 });
        let delayed = cache.deliveries.clone();
        let mut endpoint = ENDPOINT;
        match change {
            "owner" | "workspace" => {
                owner = DocumentEditOwner::new(document.clone());
                owner.bind_context(if change=="workspace" {"other"} else {"workspace"},
                    "destination",ENDPOINT,None,Some(account.clone())).unwrap();
            }
            "account" => {
                account = crate::local_account::mock_account_context(ENDPOINT);
                owner.bind_context("workspace","destination",ENDPOINT,None,Some(account.clone())).unwrap();
            }
            "endpoint" => { endpoint = "http://127.0.0.1:9912"; }
            "revocation" => account.invalidate(),
            _ => unreachable!(),
        }
        delayed.lock().unwrap().push_back((key,Status::Ready { text:"late private".into(),source_version:1 }));
        let current = cache.frame(&owner,Some(account),endpoint,&document,24.0);
        assert!(current.entries.values().all(|(_,status)|matches!(status,Status::Idle)),"{change}");
        assert!(cache.entries.values().all(|status|matches!(status,Status::Idle)),"{change}");
    }
}

#[test]
fn invalidation_does_not_replenish_slots_while_previous_owner_requests_remain_in_flight() {
    let (mut cache,owner,account,document)=fixture();
    cache.frame(&owner,Some(account.clone()),ENDPOINT,&document,24.0);
    let permits:Vec<_>=(0..MAX_IN_FLIGHT_PREVIEWS).map(|_|cache.slots.clone().try_acquire_owned().unwrap()).collect();
    cache.clear();
    cache.frame(&owner,Some(account),ENDPOINT,&document,24.0);
    assert!(cache.slots.clone().try_acquire_owned().is_err());
    drop(permits);
    assert_eq!(cache.slots.available_permits(),MAX_IN_FLIGHT_PREVIEWS);
}
