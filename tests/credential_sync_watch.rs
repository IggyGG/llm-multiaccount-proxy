use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use llmap::secrets::{SecretBox, parse_master_key};
use llmap::storage::SqliteStore;

const MASTER_KEY: &str = "ERERERERERERERERERERERERERERERERERERERERERE=";

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn legacy(access_token: &str) -> String {
    format!(
        r#"CLAUDE_ACCOUNT_1_NAME="OAuth primary"
CLAUDE_ACCOUNT_1='{{"claudeAiOauth":{{"accessToken":"{access_token}","refreshToken":"fake-refresh","expiresAt":1900000000000,"scopes":["user:inference"]}}}}'
"#
    )
}

#[test]
fn watch_mode_rereads_and_synchronizes_changed_credentials() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("llmap.db");
    let config = directory.path().join("llmap.toml");
    let input = directory.path().join("legacy.env");
    std::fs::write(
        &config,
        format!(
            r#"[server]
bind = "127.0.0.1:0"
allowed_upstream_hosts = ["api.anthropic.com"]
[forward_proxy]
enabled = false
[auth]
mode = "enforce"
[oauth]
refresh_mode = "external"
[storage]
database_path = "{}"
master_key_env = "LLMAP_MASTER_KEY"
[admin]
username = "admin"
bootstrap_password_env = "LLMAP_ADMIN_PASSWORD"
[telemetry]
audit_retention_days = 30
"#,
            database.display()
        ),
    )
    .unwrap();
    std::fs::write(&input, legacy("fake-initial-access")).unwrap();

    let import = Command::new(env!("CARGO_BIN_EXE_llmap"))
        .args([
            "migrate",
            "claudeproxy-env",
            "--config",
            config.to_str().unwrap(),
            "--input",
            input.to_str().unwrap(),
        ])
        .env("LLMAP_MASTER_KEY", MASTER_KEY)
        .output()
        .unwrap();
    assert!(import.status.success(), "initial import failed");

    let child = Command::new(env!("CARGO_BIN_EXE_llmap"))
        .args([
            "migrate",
            "claudeproxy-env",
            "--config",
            config.to_str().unwrap(),
            "--input",
            input.to_str().unwrap(),
            "--credentials-only",
            "--watch-interval-seconds",
            "1",
        ])
        .env("LLMAP_MASTER_KEY", MASTER_KEY)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut child = ChildGuard(child);

    thread::sleep(Duration::from_millis(200));
    std::fs::write(&input, legacy("fake-rotated-access")).unwrap();

    let key = parse_master_key(MASTER_KEY).unwrap();
    let store = SqliteStore::open(&database, SecretBox::new(key)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        let (_, credential) = store.load_account("claudeproxy-1").unwrap();
        if credential.expose() == "fake-rotated-access" {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "watch sync did not observe the changed file"
        );
        thread::sleep(Duration::from_millis(100));
    }

    assert!(
        child.0.try_wait().unwrap().is_none(),
        "watch mode exited unexpectedly"
    );
}
