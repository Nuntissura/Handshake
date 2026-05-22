MT-058 — sandbox escape negative-test catalog

The 10 escape attempts that comprise the canonical negative-test suite
live in source (not JSON), at:

    src/backend/handshake_core/src/test_harness/escape_attempts.rs
    -> pub fn escape_catalog() -> Vec<EscapeAttempt>

Catalog entries (kebab-case ids stable across the WP):

  ESC-FS-READ-OUT-OF-BIND
  ESC-FS-WRITE-OUT-OF-BIND
  ESC-FS-SYMLINK-TRAVERSAL
  ESC-NET-DENY-ALL-DNS
  ESC-NET-DENY-ALL-TCP
  ESC-NET-LOOPBACK-ONLY-EXTERNAL
  ESC-PRIV-ESCALATE-UID
  ESC-NAMESPACE-PID
  ESC-DEVICE-ACCESS
  ESC-WIN32-FOREGROUND-INJECT  (Windows-only; HBR-QUIET-001 acid test)

Outcome legend (per attempt × adapter cell):

  GREEN    adapter denied the escape (intended behavior)
  RED      adapter allowed the escape — blocks WP integration validation
  YELLOW   adapter behavior matches a documented weaker-enforcement
           marker (e.g., Docker without --userns=keep-id shows UID 0;
           recorded per RW-1 scoring rather than treated as a failure)
  SKIPPED  adapter unavailable, attempt OS-restricted, or fixture setup
           failed; skipped_adapters list surfaces coverage gaps

Test driver:

    src/backend/handshake_core/tests/sandbox_escape_negative_tests.rs

Always-on tests verify the harness mechanics (catalog completeness,
JSON shape, RED detection). Env-gated tests under the wsl2-integration /
docker-integration / win-native-integration Cargo features exercise the
real adapter implementations. SandboxEscapeReport is persisted via
persist_to_artifacts() to:

    D:/Projects/LLM projects/Handshake/Handshake_Artifacts/sandbox-escape-results-<run-id>.json

Validators read that artifact to confirm GREEN/YELLOW coverage and
investigate any RED.
