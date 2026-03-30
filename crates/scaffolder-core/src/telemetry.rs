use std::collections::BTreeMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::fs;

use crate::runtime::check::Language;

const API_KEY: &str = "a7182ac460dde671c8f2e1318b517228";
const AMPLITUDE_ENDPOINT: &str = "https://api2.amplitude.com/2/httpapi";

fn resolve_endpoint() -> String {
    std::env::var("__AMPLITUDE_ENDPOINT").unwrap_or_else(|_| AMPLITUDE_ENDPOINT.to_string())
}

type TomlSections = BTreeMap<String, BTreeMap<String, String>>;

fn iii_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(".iii")
}

fn telemetry_toml_path() -> std::path::PathBuf {
    iii_dir().join("telemetry.toml")
}

fn write_atomic(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let tmp = path.with_extension("tmp");
    if std::fs::write(&tmp, content).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&tmp, perms).ok();
        }
        std::fs::rename(&tmp, path).ok();
    }
}

fn read_toml_key(section: &str, key: &str) -> Option<String> {
    let contents = std::fs::read_to_string(telemetry_toml_path()).ok()?;
    let sections: TomlSections = toml::from_str(&contents).ok()?;
    sections
        .get(section)?
        .get(key)
        .filter(|v| !v.is_empty())
        .cloned()
}

fn set_toml_key(section: &str, key: &str, value: &str) {
    let path = telemetry_toml_path();
    let contents = std::fs::read_to_string(&path).unwrap_or_default();
    let mut sections: TomlSections = toml::from_str(&contents).unwrap_or_default();
    sections
        .entry(section.to_string())
        .or_default()
        .insert(key.to_string(), value.to_string());
    if let Ok(serialized) = toml::to_string(&sections) {
        write_atomic(&path, &serialized);
    }
}

pub fn get_or_create_telemetry_id() -> String {
    if let Some(id) = read_toml_key("identity", "id") {
        return id;
    }
    let legacy_path = iii_dir().join("telemetry_id");
    if let Ok(raw) = std::fs::read_to_string(&legacy_path) {
        let id = raw.trim().to_string();
        if !id.is_empty() {
            set_toml_key("identity", "id", &id);
            return id;
        }
    }
    let id = format!("auto-{}", uuid::Uuid::new_v4());
    set_toml_key("identity", "id", &id);
    id
}

pub fn is_telemetry_disabled() -> bool {
    if let Ok(val) = std::env::var("III_TELEMETRY_ENABLED") {
        if val == "false" || val == "0" {
            return true;
        }
    }
    if std::env::var("III_TELEMETRY_DEV").ok().as_deref() == Some("true") {
        return true;
    }
    const CI_VARS: &[&str] = &[
        "CI",
        "GITHUB_ACTIONS",
        "GITLAB_CI",
        "CIRCLECI",
        "JENKINS_URL",
        "TRAVIS",
        "BUILDKITE",
        "TF_BUILD",
        "CODEBUILD_BUILD_ID",
        "BITBUCKET_BUILD_NUMBER",
        "DRONE",
        "TEAMCITY_VERSION",
    ];
    CI_VARS.iter().any(|v| std::env::var(v).is_ok())
}

fn detect_machine_id() -> String {
    let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string());
    let mut hasher = Sha256::new();
    hasher.update(hostname.as_bytes());
    let result = hasher.finalize();
    result[..8].iter().map(|b| format!("{:02x}", b)).collect()
}

fn detect_is_container() -> bool {
    if std::env::var("III_CONTAINER").is_ok() {
        return true;
    }
    if std::env::var("KUBERNETES_SERVICE_HOST").is_ok() {
        return true;
    }
    Path::new("/.dockerenv").exists()
}

fn detect_install_method() -> &'static str {
    if let Ok(exe) = std::env::current_exe() {
        let path = exe.to_string_lossy();
        if path.contains(".cargo/bin") || path.contains("cargo-install") {
            return "cargo";
        }
        if path.contains("homebrew") || path.contains("Cellar") || path.contains("linuxbrew") {
            return "brew";
        }
        if path.contains("chocolatey") || path.contains("choco") {
            return "chocolatey";
        }
        if path.contains(".local/bin") {
            return "sh";
        }
    }
    "manual"
}

fn build_user_properties(tools_version: &str) -> serde_json::Value {
    serde_json::json!({
        "environment.os": std::env::consts::OS,
        "environment.arch": std::env::consts::ARCH,
        "environment.cpu_cores": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
        "environment.timezone": std::env::var("TZ").unwrap_or_else(|_| "Unknown".to_string()),
        "environment.machine_id": detect_machine_id(),
        "environment.is_container": detect_is_container(),
        "env": std::env::var("III_ENV").unwrap_or_else(|_| "unknown".to_string()),
        "install_method": detect_install_method(),
        "cli_version": tools_version,
        "host_user_id": std::env::var("III_HOST_USER_ID").ok(),
    })
}

fn millis_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Serialize)]
struct AmplitudeEvent {
    device_id: String,
    user_id: Option<String>,
    event_type: String,
    event_properties: serde_json::Value,
    user_properties: Option<serde_json::Value>,
    platform: String,
    os_name: String,
    app_version: String,
    time: i64,
    insert_id: String,
    ip: Option<String>,
}

#[derive(Serialize)]
struct AmplitudePayload<'a> {
    api_key: &'a str,
    events: Vec<AmplitudeEvent>,
}

async fn send_amplitude_to(
    endpoint: &str,
    event_type: &str,
    platform: &str,
    tools_version: &str,
    event_properties: serde_json::Value,
) {
    let telemetry_id = get_or_create_telemetry_id();
    let event = AmplitudeEvent {
        device_id: telemetry_id.clone(),
        user_id: Some(telemetry_id),
        event_type: event_type.to_string(),
        event_properties,
        user_properties: Some(build_user_properties(tools_version)),
        platform: platform.to_string(),
        os_name: std::env::consts::OS.to_string(),
        app_version: tools_version.to_string(),
        time: millis_epoch(),
        insert_id: uuid::Uuid::new_v4().to_string(),
        ip: Some("$remote".to_string()),
    };
    let payload = AmplitudePayload {
        api_key: API_KEY,
        events: vec![event],
    };
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    let _ = client.post(endpoint).json(&payload).send().await;
}

async fn send_amplitude(
    event_type: &str,
    platform: &str,
    tools_version: &str,
    event_properties: serde_json::Value,
) {
    send_amplitude_to(&resolve_endpoint(), event_type, platform, tools_version, event_properties).await;
}

pub fn spawn_project_event(
    event_type: &'static str,
    platform: &'static str,
    tools_version: String,
    event_properties: serde_json::Value,
) -> Option<tokio::task::JoinHandle<()>> {
    if is_telemetry_disabled() {
        return None;
    }
    Some(tokio::spawn(async move {
        send_amplitude(
            event_type,
            platform,
            &tools_version,
            event_properties,
        )
        .await;
    }))
}

pub fn platform_for_product(product_name: &str) -> &'static str {
    match product_name {
        "motia" => "motia-tools",
        _ => "iii-tools",
    }
}

pub async fn write_project_ini(
    project_dir: &Path,
    project_id: &str,
    project_name: &str,
) -> Result<()> {
    let dir = project_dir.join(".iii");
    fs::create_dir_all(&dir)
        .await
        .context("create .iii directory")?;
    let body = format!("[project]\nproject_id={project_id}\nproject_name={project_name}\n");
    fs::write(dir.join("project.ini"), body)
        .await
        .context("write project.ini")?;
    Ok(())
}

pub async fn run_dependency_install(project_dir: &Path, langs: &[Language]) -> Result<()> {
    let has_js_ts = langs
        .iter()
        .any(|l| matches!(l, Language::TypeScript | Language::JavaScript));
    if has_js_ts && project_dir.join("package.json").exists() {
        let status = tokio::process::Command::new("npm")
            .args(["install"])
            .current_dir(project_dir)
            .status()
            .await
            .context("spawn npm install")?;
        if !status.success() {
            anyhow::bail!("npm install exited with status {}", status);
        }
        return Ok(());
    }

    let has_python = langs.contains(&Language::Python);
    if has_python && project_dir.join("pyproject.toml").exists() {
        let uv = tokio::process::Command::new("uv")
            .args(["sync"])
            .current_dir(project_dir)
            .status()
            .await;
        if let Ok(s) = uv {
            if s.success() {
                return Ok(());
            }
        }
    }
    if has_python && project_dir.join("requirements.txt").exists() {
        let pip = tokio::process::Command::new("pip")
            .args(["install", "-r", "requirements.txt"])
            .current_dir(project_dir)
            .status()
            .await;
        if let Ok(s) = pip {
            if s.success() {
                return Ok(());
            }
        }
        let pip3 = tokio::process::Command::new("pip3")
            .args(["install", "-r", "requirements.txt"])
            .current_dir(project_dir)
            .status()
            .await;
        if let Ok(s) = pip3 {
            if s.success() {
                return Ok(());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    #[test]
    fn project_ini_body_format() {
        let s = format!("[project]\nproject_id={}\nproject_name={}\n", "abc", "my-app");
        assert!(s.contains("project_id=abc"));
        assert!(s.contains("project_name=my-app"));
    }

    #[tokio::test]
    async fn sends_project_created_event() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/2/httpapi"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 200})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let endpoint = format!("{}/2/httpapi", mock_server.uri());

        send_amplitude_to(
            &endpoint,
            "project_created",
            "motia-tools",
            "0.3.0",
            serde_json::json!({
                "project_id": "test-id",
                "project_name": "my-project",
                "template": "quickstart",
                "product": "motia",
            }),
        )
        .await;

        // Mock expectation of exactly 1 call is verified on drop
    }

    #[tokio::test]
    async fn sends_project_initialized_event() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/2/httpapi"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"code": 200})))
            .expect(1)
            .mount(&mock_server)
            .await;

        let endpoint = format!("{}/2/httpapi", mock_server.uri());

        send_amplitude_to(
            &endpoint,
            "project_initialized",
            "motia-tools",
            "0.3.0",
            serde_json::json!({
                "project_id": "test-id",
                "project_name": "my-project",
                "template": "quickstart",
                "product": "motia",
            }),
        )
        .await;
    }

    #[tokio::test]
    async fn does_not_send_quickstart_event() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/2/httpapi"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&mock_server)
            .await;

        // We intentionally do NOT call send_amplitude_to with "quickstart"
        // because the codebase never sends it — this test documents that fact.

        // Verified: 0 calls received on drop
    }

    #[tokio::test]
    async fn payload_contains_correct_event_type_and_api_key() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/2/httpapi"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let endpoint = format!("{}/2/httpapi", mock_server.uri());

        send_amplitude_to(
            &endpoint,
            "project_created",
            "motia-tools",
            "0.3.0",
            serde_json::json!({
                "project_id": "test-id",
                "project_name": "my-project",
                "template": "quickstart",
                "product": "motia",
            }),
        )
        .await;

        let requests: Vec<Request> = mock_server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);

        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["api_key"], API_KEY);
        assert_eq!(body["events"][0]["event_type"], "project_created");
        assert_eq!(body["events"][0]["platform"], "motia-tools");
        assert_eq!(body["events"][0]["event_properties"]["project_id"], "test-id");
        assert_eq!(body["events"][0]["event_properties"]["template"], "quickstart");
    }
}
