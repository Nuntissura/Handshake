//! MT-111 — EAGLE-3 upgrade-hook scaffold.
//!
//! Per operator E-4 + MT-109 contract: EAGLE-3 speculative decoding is a
//! post-merge dependency upgrade for llama.cpp PR #18039. This module is
//! the **single source of truth** the rest of the codebase reads when
//! deciding whether EAGLE-3 is available, plus a typed-error placeholder
//! the adapter can call right now without panicking.
//!
//! Single-source-of-truth design:
//!   - `EAGLE3_AVAILABLE: bool` is a const set to `false`. The future
//!     upgrade MT flips this single constant to `true`.
//!   - `eagle3_available() -> bool` is the runtime accessor everything
//!     else reads. Adapter capability publishers (`ModelCapabilities::
//!     supports_eagle3`) gate on this; the MT-109 technique surface's
//!     `validate_mode` for `SpeculativeMode::Eagle3` re-uses the same
//!     accessor when the adapter has not yet wired its own flag.
//!   - `eagle3_decode(...)` is the stub the production code path can
//!     call today; it returns `Err(EagleError::PendingUpstream{ pr_url
//!     })` so a misrouted call surfaces a typed error rather than a
//!     panic / silent zero-token return.
//!
//! Upgrade-MT TODO checklist (post-merge of llama.cpp PR #18039):
//!   1. [ ] Bump the llama-cpp-2 crate to a release that includes the
//!          PR-#18039 changes (or to the upstream tag if vendored).
//!   2. [ ] Swap the `eagle3_decode` body for the real EAGLE-3 draft-
//!          token derivation that delegates into llama-cpp-2's new
//!          EAGLE-3 API (signature kept stable so call sites do not
//!          change).
//!   3. [ ] Flip `EAGLE3_AVAILABLE` from `false` to `true`.
//!   4. [ ] In every per-adapter `ModelCapabilities` publisher, derive
//!          `supports_eagle3` from `eagle3_available()` (or hard-code
//!          `true` if the adapter is itself the publisher).
//!   5. [ ] Remove the deferral badge wiring in
//!          `app/src/lib/ipc/speculative.ts::specModeOptions` (the
//!          `eagle3_deferred` branch becomes `eagle3` and gates on
//!          `supportsEagle3`).
//!   6. [ ] Remove the deferral note in
//!          `app/src/components/inference_lab/SpeculativeDecodingPanel.tsx`.
//!   7. [ ] In `model_runtime/techniques/speculative_decoding.rs` the
//!          existing `validate_mode` Eagle3 arm already accepts when
//!          `supports_eagle3` is true; no change needed there.

use std::fmt;

/// Single-source-of-truth flag. The future upgrade MT flips this to
/// `true` after the llama.cpp PR-#18039 merge + crate bump + real impl
/// land together; until then every Eagle3 caller surfaces the typed
/// PendingUpstream error.
pub const EAGLE3_AVAILABLE: bool = false;

/// Public canonical URL for the deferral message. Kept here so the
/// frontend deferral note + the typed error in `eagle3_decode` cite
/// the same single string.
pub const EAGLE3_UPSTREAM_PR_URL: &str = "https://github.com/ggerganov/llama.cpp/pull/18039";

/// Runtime accessor. All production code that wants to ask "is Eagle3
/// available today?" MUST go through this function rather than reading
/// the `EAGLE3_AVAILABLE` const directly, so a future MT could swap to
/// a feature-flag or dynamic detection scheme without touching every
/// call site.
pub fn eagle3_available() -> bool {
    EAGLE3_AVAILABLE
}

/// Placeholder draft-tokens type. Real impl will replace this with the
/// llama-cpp-2 EAGLE-3 draft-token shape; keeping the type local lets
/// the upgrade MT swap the impl without breaking the public crate API.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DraftTokens {
    pub tokens: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EagleError {
    /// EAGLE-3 is gated on llama.cpp PR #18039 merging upstream. The
    /// `pr_url` field is a static reference to `EAGLE3_UPSTREAM_PR_URL`
    /// so any consumer logging this error gets a citation.
    PendingUpstream { pr_url: &'static str },
}

impl fmt::Display for EagleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PendingUpstream { pr_url } => {
                write!(f, "EAGLE-3 is pending upstream merge: {pr_url}")
            }
        }
    }
}

impl std::error::Error for EagleError {}

/// Stub EAGLE-3 entry point. Production callers can invoke this today
/// and they'll receive a typed `PendingUpstream` error citing the PR
/// URL — no panic, no silent zero-token return. The upgrade MT swaps
/// the body without changing the signature.
pub fn eagle3_decode(_prefix_tokens: &[u32], _max_draft: u32) -> Result<DraftTokens, EagleError> {
    if eagle3_available() {
        // Upgrade-MT slot: insert the llama-cpp-2 EAGLE-3 call here.
        // Today this branch is unreachable because EAGLE3_AVAILABLE is
        // false; tomorrow it'll carry the real implementation.
        Ok(DraftTokens::default())
    } else {
        Err(EagleError::PendingUpstream {
            pr_url: EAGLE3_UPSTREAM_PR_URL,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eagle3_available_const_is_false_until_upgrade_mt_flips_it() {
        assert!(
            !EAGLE3_AVAILABLE,
            "MT-111 contract: EAGLE3_AVAILABLE must stay false until the post-PR-#18039 upgrade MT flips it",
        );
        assert!(!eagle3_available());
    }

    #[test]
    fn eagle3_decode_returns_typed_pending_upstream_error() {
        let err = eagle3_decode(&[1, 2, 3], 4).expect_err("Eagle3 must be unavailable today");
        assert!(matches!(
            err,
            EagleError::PendingUpstream { pr_url } if pr_url == EAGLE3_UPSTREAM_PR_URL
        ));
        // The Display impl must include the URL so logs can be grep'd
        // for the pending-upstream marker.
        assert!(format!("{err}").contains(EAGLE3_UPSTREAM_PR_URL));
    }

    #[test]
    fn eagle3_upstream_pr_url_is_the_canonical_pr_link() {
        // Single source of truth — the frontend deferral note + this
        // typed error must cite the same URL.
        assert_eq!(
            EAGLE3_UPSTREAM_PR_URL,
            "https://github.com/ggerganov/llama.cpp/pull/18039"
        );
    }
}
