//! Docker containers via the `docker` CLI (works with Docker Desktop, Colima, OrbStack, Podman's shim…).

use crate::system;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Clone, Debug)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    /// running, exited, created, paused, restarting, dead
    pub state: String,
    /// Human text from docker, e.g. "Up 3 hours" / "Exited (0) 2 days ago"
    pub status: String,
    /// Raw port mapping text from docker ps
    pub ports: String,
    /// Host ports published by this container, e.g. [8080, 5432]
    pub published_ports: Vec<u16>,
    pub running: bool,
    /// Unix time the container started, estimated from docker's "Up 2 days" status.
    pub started_at: Option<u64>,
    /// docker compose project / service / working dir, when the container comes from compose.
    pub project: Option<String>,
    pub service: Option<String>,
    pub working_dir: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DockerStatus {
    Ok { containers: Vec<ContainerInfo> },
    NotInstalled,
    NotRunning { message: String },
}

#[derive(Deserialize)]
struct PsRow {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Names")]
    names: String,
    #[serde(rename = "Image")]
    image: String,
    #[serde(rename = "State")]
    state: String,
    #[serde(rename = "Status", default)]
    status: String,
    #[serde(rename = "Ports", default)]
    ports: String,
    #[serde(rename = "Labels", default)]
    labels: String,
}

/// Docker prints labels as "k1=v1,k2=v2". Values may contain commas, so a
/// segment without '=' is glued back onto the previous value.
fn label(labels: &str, key: &str) -> Option<String> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for part in labels.split(',') {
        match part.split_once('=') {
            Some((k, v)) if !k.contains(' ') => pairs.push((k.to_string(), v.to_string())),
            _ => {
                if let Some(last) = pairs.last_mut() {
                    last.1.push(',');
                    last.1.push_str(part);
                }
            }
        }
    }
    pairs
        .into_iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
        .filter(|v| !v.is_empty())
}

/// Seconds a container has been up, from docker's human status: "Up 3 hours (healthy)",
/// "Up About an hour", "Up Less than a second", "Up 2 weeks".
pub fn uptime_secs(status: &str) -> Option<u64> {
    let rest = status.strip_prefix("Up ")?;
    let rest = rest.split(" (").next().unwrap_or(rest).to_ascii_lowercase();
    if rest.starts_with("less than") {
        return Some(0);
    }
    let mut words = rest.split_whitespace();
    let first = words.next()?;
    let n: u64 = match first {
        "a" | "an" => 1,
        "about" => {
            let _ = words.next(); // "a" / "an"
            1
        }
        _ => first.parse().ok()?,
    };
    let unit = words.next()?;
    let secs = match unit.trim_end_matches('s') {
        "second" => 1,
        "minute" => 60,
        "hour" => 3600,
        "day" => 86_400,
        "week" => 604_800,
        "month" => 2_592_000,
        "year" => 31_536_000,
        _ => return None,
    };
    Some(n * secs)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn docker_cmd() -> Option<Command> {
    let bin = system::find_docker()?;
    let mut cmd = Command::new(bin);
    cmd.env("PATH", system::extended_path());
    system::hide_console(&mut cmd);
    Some(cmd)
}

/// Extract host ports from docker's port text: "0.0.0.0:8080->80/tcp, [::]:8080->80/tcp, 5432/tcp"
fn published_ports(text: &str) -> Vec<u16> {
    let mut ports: Vec<u16> = Vec::new();
    for mapping in text.split(',') {
        let Some((host, _container)) = mapping.trim().split_once("->") else {
            continue;
        };
        let Some(port_str) = host.rsplit(':').next() else {
            continue;
        };
        if let Ok(p) = port_str.parse::<u16>() {
            if !ports.contains(&p) {
                ports.push(p);
            }
        }
    }
    ports.sort_unstable();
    ports
}

pub fn list() -> DockerStatus {
    let Some(mut cmd) = docker_cmd() else {
        return DockerStatus::NotInstalled;
    };
    let out = match cmd.args(["ps", "-a", "--format", "{{json .}}"]).output() {
        Ok(o) => o,
        Err(e) => {
            return DockerStatus::NotRunning {
                message: e.to_string(),
            }
        }
    };
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let message = err
            .lines()
            .next()
            .unwrap_or("docker ps failed")
            .trim()
            .to_string();
        return DockerStatus::NotRunning { message };
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut containers: Vec<ContainerInfo> = stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<PsRow>(line).ok())
        .map(|r| ContainerInfo {
            started_at: uptime_secs(&r.status).map(|u| now_secs().saturating_sub(u)),
            project: label(&r.labels, "com.docker.compose.project"),
            service: label(&r.labels, "com.docker.compose.service"),
            working_dir: label(&r.labels, "com.docker.compose.project.working_dir")
                .map(|d| system::tilde(&d)),
            running: r.state == "running",
            published_ports: published_ports(&r.ports),
            id: r.id.chars().take(12).collect(),
            name: r.names,
            image: r.image,
            state: r.state,
            status: r.status,
            ports: r.ports,
        })
        .collect();
    containers.sort_by(|a, b| b.running.cmp(&a.running).then_with(|| a.name.cmp(&b.name)));
    DockerStatus::Ok { containers }
}

/// Find a container by id (or name) and return its project key, for the lock check.
pub fn project_key(id: &str) -> Option<String> {
    let DockerStatus::Ok { containers } = list() else {
        return None;
    };
    containers
        .into_iter()
        .find(|c| c.id.starts_with(id) || id.starts_with(&c.id) || c.name == id)
        .map(|c| {
            crate::settings::key_for_container(
                c.working_dir.as_deref(),
                c.project.as_deref(),
                &c.name,
            )
        })
}

fn valid_ref(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

pub fn action(id: &str, action: &str) -> Result<(), String> {
    if !valid_ref(id) {
        return Err("invalid container id".into());
    }
    let args: Vec<&str> = match action {
        "start" | "stop" | "restart" | "pause" | "unpause" => vec![action, id],
        "remove" => vec!["rm", "-f", id],
        _ => return Err(format!("unknown action {action}")),
    };
    let mut cmd = docker_cmd().ok_or("docker not found")?;
    let out = cmd.args(&args).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(())
}

pub fn open_logs(name: &str) -> Result<(), String> {
    if !valid_ref(name) {
        return Err("invalid container name".into());
    }
    let bin = system::find_docker().ok_or("docker not found")?;
    let command = format!("\"{}\" logs -f --tail 200 {name}", bin.display());
    system::open_in_terminal(&command)
}

pub fn open_desktop() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Docker"])
            .status()
            .map_err(|e| e.to_string())
            .and_then(|s| {
                if s.success() {
                    Ok(())
                } else {
                    Err("Docker.app not found".into())
                }
            })
    }
    #[cfg(target_os = "windows")]
    {
        let exe = r"C:\Program Files\Docker\Docker\Docker Desktop.exe";
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "", exe]);
        system::hide_console(&mut cmd);
        cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
    }
    #[cfg(target_os = "linux")]
    {
        Err("Start the Docker daemon, e.g. `sudo systemctl start docker`".into())
    }
}

#[cfg(test)]
mod tests {
    use super::published_ports;

    #[test]
    fn parses_host_ports() {
        let p = published_ports(
            "0.0.0.0:8080->80/tcp, [::]:8080->80/tcp, 127.0.0.1:3000->3000/tcp, 5432/tcp",
        );
        assert_eq!(p, vec![3000, 8080]);
    }

    #[test]
    fn parses_compose_labels() {
        let l = "com.docker.compose.project=shop,com.docker.compose.project.working_dir=/a/b,c,com.docker.compose.service=api";
        assert_eq!(
            super::label(l, "com.docker.compose.project").as_deref(),
            Some("shop")
        );
        assert_eq!(
            super::label(l, "com.docker.compose.project.working_dir").as_deref(),
            Some("/a/b,c")
        );
        assert_eq!(
            super::label(l, "com.docker.compose.service").as_deref(),
            Some("api")
        );
        assert_eq!(super::label(l, "missing"), None);
    }

    #[test]
    fn parses_uptime() {
        use super::uptime_secs;
        assert_eq!(uptime_secs("Up 3 hours (healthy)"), Some(3 * 3600));
        assert_eq!(uptime_secs("Up About an hour"), Some(3600));
        assert_eq!(uptime_secs("Up Less than a second"), Some(0));
        assert_eq!(uptime_secs("Up 10 days"), Some(10 * 86_400));
        assert_eq!(uptime_secs("Up 1 minute"), Some(60));
        assert_eq!(uptime_secs("Exited (0) 2 days ago"), None);
    }

    #[test]
    fn empty_ports() {
        assert!(published_ports("").is_empty());
    }
}
