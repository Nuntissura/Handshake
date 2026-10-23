mod api {
    mod authority {
        mod local_account_http_tests {
            #[tokio::test]
            async fn local_account_http_setup_login_vault_logout_and_constant_denials() {
                handshake_core::api::authority::local_account_http_test_support::run_local_account_http_setup_login_vault_logout_and_constant_denials()
                    .await;
            }
        }
    }
}
