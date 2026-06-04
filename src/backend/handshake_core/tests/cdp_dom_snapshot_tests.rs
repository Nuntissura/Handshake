use std::path::Path;

use serde_json::json;

use handshake_core::{
    model_manual::{model_manual, CommandStatus},
    operator_foreground::cdp_client::{
        decode_dom_snapshot_response, decode_query_selector_node_id, dom_describe_node_request,
        dom_get_document_request, dom_query_selector_request, DomScope,
    },
};

#[test]
fn cdp_dom_snapshot_tests_full_request_uses_depth_minus_one_and_pierce() {
    let request = dom_get_document_request(7);

    assert_eq!(request["id"], 7);
    assert_eq!(request["method"], "DOM.getDocument");
    assert_eq!(request["params"]["depth"], -1);
    assert_eq!(request["params"]["pierce"], true);
}

#[test]
fn cdp_dom_snapshot_tests_selector_request_and_missing_selector_fail_closed() {
    let query = dom_query_selector_request(8, 101, "#root");
    assert_eq!(query["id"], 8);
    assert_eq!(query["method"], "DOM.querySelector");
    assert_eq!(query["params"]["nodeId"], 101);
    assert_eq!(query["params"]["selector"], "#root");

    let describe = dom_describe_node_request(9, 202);
    assert_eq!(describe["method"], "DOM.describeNode");
    assert_eq!(describe["params"]["nodeId"], 202);
    assert_eq!(describe["params"]["depth"], -1);
    assert_eq!(describe["params"]["pierce"], true);

    let missing = decode_query_selector_node_id(&json!({
        "id": 8,
        "result": { "nodeId": 0 }
    }))
    .expect_err("missing selector must be a hard error");
    assert!(missing.to_string().contains("DOM.querySelector"));
}

#[test]
fn cdp_dom_snapshot_tests_decodes_tree_and_extracts_stable_element_ids_only_from_data_testid() {
    let response = json!({
        "id": 7,
        "result": {
            "root": {
                "nodeId": 1,
                "nodeType": 9,
                "nodeName": "#document",
                "attributes": [],
                "children": [{
                    "nodeId": 2,
                    "nodeType": 1,
                    "nodeName": "HTML",
                    "attributes": ["lang", "en"],
                    "children": [{
                        "nodeId": 3,
                        "nodeType": 1,
                        "nodeName": "BODY",
                        "attributes": ["data-testid", "app-shell", "id", "main"],
                        "children": [{
                            "nodeId": 4,
                            "nodeType": 3,
                            "nodeName": "#text",
                            "nodeValue": "Hello"
                        }]
                    }]
                }]
            }
        }
    });

    let tree = decode_dom_snapshot_response(&response).expect("DOM tree decodes");

    assert_eq!(tree.root.node_name, "#document");
    assert_eq!(tree.root.children[0].attributes["lang"], "en");
    let body = &tree.root.children[0].children[0];
    assert_eq!(body.stable_element_id.as_deref(), Some("app-shell"));
    assert_eq!(body.attributes["id"], "main");
    assert!(tree.contains_stable_element_id("app-shell"));
}

#[test]
fn cdp_dom_snapshot_tests_scope_serializes_and_manual_tauri_registration_are_paired() {
    let full = serde_json::to_value(DomScope::Full).expect("full scope serializes");
    assert_eq!(full["kind"], "full");
    let selector = serde_json::to_value(DomScope::Selector {
        selector: "[data-testid='app-shell']".to_string(),
    })
    .expect("selector scope serializes");
    assert_eq!(selector["kind"], "selector");
    assert_eq!(selector["selector"], "[data-testid='app-shell']");

    let manual = model_manual();
    let command = manual
        .command_reference
        .iter()
        .find(|command| command.id == "visual_debug_dom_snapshot")
        .expect("manual entry for visual_debug_dom_snapshot");
    assert_eq!(command.status, CommandStatus::Wired);
    assert_eq!(
        command.ipc_channel,
        Some("kernel.visual_debug.dom_snapshot")
    );
    assert_eq!(
        command.tauri_command,
        Some("kernel_visual_debug_dom_snapshot")
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("repo root");
    let lib_rs = std::fs::read_to_string(repo_root.join("app/src-tauri/src/lib.rs")).unwrap();
    let visual_debug_rs =
        std::fs::read_to_string(repo_root.join("app/src-tauri/src/visual_debug.rs")).unwrap();

    assert!(lib_rs.contains("visual_debug::kernel_visual_debug_dom_snapshot"));
    assert!(visual_debug_rs.contains("pub async fn kernel_visual_debug_dom_snapshot"));
    assert!(visual_debug_rs.contains("kernel.visual_debug.dom_snapshot"));
}
