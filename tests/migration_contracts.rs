use chrono::{Duration, TimeZone, Utc};
use llmap::auth::{AuthError, AuthMode, Authenticator};
use llmap::data_plane::AccountRepository;
use llmap::migration::{
    MigrationError, import_claudeproxy_env, parse_claudeproxy_env, sync_claudeproxy_credentials,
};
use llmap::providers::ProviderKind;
use llmap::secrets::{SecretBox, parse_master_key};
use llmap::storage::SqliteStore;

const LEGACY: &str = r#"
CLAUDE_ACCOUNT_1_NAME="OAuth primary"
CLAUDE_ACCOUNT_1='{"claudeAiOauth":{"accessToken":"fake-oauth-access","refreshToken":"fake-oauth-refresh","expiresAt":1900000000000,"scopes":["user:profile","user:inference"]}}'
# CLAUDE_ACCOUNT_2_NAME="Paused compatible"
# CLAUDE_ACCOUNT_2='{"type":"minimax","apiKey":"fake-compatible-key","baseUrl":"https://api.minimax.invalid","model":"MiniMax-M3","proxyUrl":"socks5h://fake-user:fake-pass@residential.invalid:1080"}'
CLAUDE_ACCOUNT_3_NAME="Bedrock"
CLAUDE_ACCOUNT_3='{"type":"bedrock","region":"eu-north-1","accessKeyId":"AKIDEXAMPLE","secretAccessKey":"fake-aws-secret","modelMap":{"default":"eu.anthropic.claude-sonnet"}}'
"#;

#[test]
fn legacy_accounts_map_without_exposing_credentials() {
    let accounts = parse_claudeproxy_env(LEGACY).unwrap();
    assert_eq!(accounts.len(), 3);
    assert_eq!(accounts[0].account.kind, ProviderKind::ClaudeOauth);
    assert!(accounts[0].account.enabled);
    assert_eq!(
        accounts[1].account.base_url.as_str(),
        "https://api.minimax.invalid/anthropic/"
    );
    assert!(!accounts[1].account.enabled);
    assert_eq!(accounts[2].account.kind, ProviderKind::BedrockSigV4);
    assert_eq!(
        accounts[2].account.base_url.host_str(),
        Some("bedrock-runtime.eu-north-1.amazonaws.com")
    );
}

#[test]
fn import_is_encrypted_idempotent_and_replace_is_explicit() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("llmap.db");
    let key = parse_master_key("ERERERERERERERERERERERERERERERERERERERERERE=").unwrap();
    let store = SqliteStore::open(&database, SecretBox::new(key)).unwrap();

    let first =
        import_claudeproxy_env(&store, parse_claudeproxy_env(LEGACY).unwrap(), false).unwrap();
    assert_eq!(first.imported, 3);
    let second =
        import_claudeproxy_env(&store, parse_claudeproxy_env(LEGACY).unwrap(), false).unwrap();
    assert_eq!(second.skipped_existing, 3);
    assert_eq!(store.list_accounts().unwrap().len(), 3);
    let (_, oauth_secret) = store.load_account("claudeproxy-1").unwrap();
    assert!(oauth_secret.expose().contains("fake-oauth-access"));
    let (compatible, _) = store.load_account("claudeproxy-2").unwrap();
    assert_eq!(
        compatible.egress_proxies,
        vec!["socks5h://fake-user:fake-pass@residential.invalid:1080"]
    );

    let bytes = std::fs::read(database).unwrap();
    for secret in [
        "fake-oauth-access",
        "fake-oauth-refresh",
        "fake-compatible-key",
        "fake-pass",
        "fake-aws-secret",
    ] {
        assert!(
            !bytes
                .windows(secret.len())
                .any(|window| window == secret.as_bytes())
        );
    }
}

#[tokio::test]
async fn credential_sync_preserves_local_account_policy_and_rotates_client_auth() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("llmap.db");
    let key = parse_master_key("ERERERERERERERERERERERERERERERERERERERERERE=").unwrap();
    let store = SqliteStore::open(&database, SecretBox::new(key)).unwrap();

    import_claudeproxy_env(&store, parse_claudeproxy_env(LEGACY).unwrap(), false).unwrap();
    let (mut locally_managed, _) = store.load_account("claudeproxy-1").unwrap();
    locally_managed.label = "Locally managed label".into();
    locally_managed.egress_proxies =
        vec!["socks5h://local-user:local-pass@residential.invalid:1080".into()];
    let original = parse_claudeproxy_env(LEGACY).unwrap().remove(0);
    store
        .upsert_account(&locally_managed, &original.credential)
        .unwrap();

    let updated_source = LEGACY
        .replace("OAuth primary", "Legacy renamed label")
        .replace("fake-oauth-access", "fake-synced-oauth-access");
    let rotation_time = Utc.with_ymd_and_hms(2026, 9, 4, 11, 0, 0).unwrap();
    let summary = sync_claudeproxy_credentials(
        &store,
        parse_claudeproxy_env(&updated_source).unwrap(),
        rotation_time + Duration::minutes(10),
    )
    .unwrap();

    assert_eq!(summary.synced, 1);
    assert_eq!(summary.unchanged, 2);
    let idempotent = sync_claudeproxy_credentials(
        &store,
        parse_claudeproxy_env(&updated_source).unwrap(),
        rotation_time + Duration::minutes(15),
    )
    .unwrap();
    assert_eq!(idempotent.synced, 0);
    assert_eq!(idempotent.unchanged, 3);
    let (synced, provider_credential) = store.load_account("claudeproxy-1").unwrap();
    assert_eq!(synced.label, "Locally managed label");
    assert_eq!(synced.egress_proxies, locally_managed.egress_proxies);
    assert!(synced.enabled);
    assert_eq!(provider_credential.expose(), "fake-synced-oauth-access");
    assert!(!store.load_account("claudeproxy-2").unwrap().0.enabled);

    let authenticator = Authenticator::new([91; 32]);
    let snapshot = store
        .credential_snapshot(&authenticator, rotation_time)
        .await
        .unwrap();
    for token in ["fake-oauth-access", "fake-synced-oauth-access"] {
        assert!(
            authenticator
                .authorize(AuthMode::Enforce, Some(token), &snapshot, rotation_time)
                .unwrap()
                .allowed
        );
    }
    assert_eq!(
        authenticator.authorize(
            AuthMode::Enforce,
            Some("fake-oauth-access"),
            &snapshot,
            rotation_time + Duration::minutes(11),
        ),
        Err(AuthError::Unauthorized)
    );
    assert!(
        authenticator
            .authorize(
                AuthMode::Enforce,
                Some("fake-synced-oauth-access"),
                &snapshot,
                Utc.with_ymd_and_hms(2031, 1, 1, 0, 0, 0).unwrap(),
            )
            .unwrap()
            .allowed
    );
}

#[test]
fn credential_sync_preflights_all_accounts_before_writing() {
    let directory = tempfile::tempdir().unwrap();
    let key = parse_master_key("ERERERERERERERERERERERERERERERERERERERERERE=").unwrap();
    let store = SqliteStore::open(&directory.path().join("llmap.db"), SecretBox::new(key)).unwrap();
    import_claudeproxy_env(&store, parse_claudeproxy_env(LEGACY).unwrap(), false).unwrap();
    store.delete_account("claudeproxy-3").unwrap();
    let updated_source = LEGACY.replace("fake-oauth-access", "fake-must-not-be-written");

    let error = sync_claudeproxy_credentials(
        &store,
        parse_claudeproxy_env(&updated_source).unwrap(),
        Utc.with_ymd_and_hms(2026, 9, 4, 11, 10, 0).unwrap(),
    )
    .unwrap_err();

    assert!(matches!(error, MigrationError::MissingSyncAccount(id) if id == "claudeproxy-3"));
    assert_eq!(
        store.load_account("claudeproxy-1").unwrap().1.expose(),
        "fake-oauth-access"
    );
}
