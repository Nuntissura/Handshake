//! Named Argus facade over the native MCP visual inspection and steering primitives.
//!
//! Argus is the product-facing name for the headless/non-intrusive visual path models use to inspect
//! and steer Handshake. This module deliberately stays thin: it maps Argus method names onto the
//! existing AccessKit/MCP primitives so auth, leases, attribution, screenshots, and action queues stay
//! single-source-of-truth.

pub const ARGUS_INSPECT: &str = "argus.inspect";
pub const ARGUS_CLICK: &str = "argus.click";
pub const ARGUS_SET_VALUE: &str = "argus.set_value";
pub const ARGUS_SCREENSHOT: &str = "argus.screenshot";

pub const PRIMITIVE_LIST_WIDGETS: &str = "list_widgets";
pub const PRIMITIVE_CLICK_WIDGET: &str = "click_widget";
pub const PRIMITIVE_SET_VALUE: &str = "set_value";
pub const PRIMITIVE_SCREENSHOT: &str = "screenshot";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArgusRoute {
    pub method: &'static str,
    pub primitive: &'static str,
}

pub fn route(method: &str) -> Option<ArgusRoute> {
    match method {
        ARGUS_INSPECT => Some(ArgusRoute {
            method: ARGUS_INSPECT,
            primitive: PRIMITIVE_LIST_WIDGETS,
        }),
        ARGUS_CLICK => Some(ArgusRoute {
            method: ARGUS_CLICK,
            primitive: PRIMITIVE_CLICK_WIDGET,
        }),
        ARGUS_SET_VALUE => Some(ArgusRoute {
            method: ARGUS_SET_VALUE,
            primitive: PRIMITIVE_SET_VALUE,
        }),
        ARGUS_SCREENSHOT => Some(ArgusRoute {
            method: ARGUS_SCREENSHOT,
            primitive: PRIMITIVE_SCREENSHOT,
        }),
        _ => None,
    }
}

pub fn primitive_method(method: &str) -> &str {
    route(method).map(|r| r.primitive).unwrap_or(method)
}

pub fn stamp_result(value: &mut serde_json::Value, route: Option<ArgusRoute>) {
    let Some(route) = route else {
        return;
    };
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "argus".to_owned(),
            serde_json::json!({
                "tool": "Argus",
                "method": route.method,
                "primitive": route.primitive,
                "headless": true,
                "non_intrusive": true
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argus_methods_map_to_existing_primitives() {
        assert_eq!(primitive_method(ARGUS_INSPECT), PRIMITIVE_LIST_WIDGETS);
        assert_eq!(primitive_method(ARGUS_CLICK), PRIMITIVE_CLICK_WIDGET);
        assert_eq!(primitive_method(ARGUS_SET_VALUE), PRIMITIVE_SET_VALUE);
        assert_eq!(primitive_method(ARGUS_SCREENSHOT), PRIMITIVE_SCREENSHOT);
        assert_eq!(primitive_method("list_widgets"), "list_widgets");
    }
}
