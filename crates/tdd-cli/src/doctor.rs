use std::env;
use std::fs;
use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use url::Url;

use tdd_core::config::{BootstrapConfig, CiConfig, LlmProvider, TddConfig};
use tdd_core::logging::BootstrapLogEntry;
use tdd_exec::vcs::{GitVcs, Vcs};

use crate::executor::{absolutize_path, resolve_root};
use crate::init::BOOTSTRAP_STATE_FILE;

#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub git_clean: bool,
    pub ci: CiDoctorReport,
    pub llm: LlmDoctorReport,
    pub bootstrap: Option<BootstrapDoctorReport>,
    pub issues: Vec<String>,
}

impl DoctorReport {
    pub fn format_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.push(format!("Git clean: {}", yes_no(self.git_clean)));
        lines.push(format!(
            "CI binaries available → fmt: {} | check: {} | test: {}",
            yes_no(self.ci.fmt_ready),
            yes_no(self.ci.check_ready),
            yes_no(self.ci.test_ready)
        ));
        if !self.ci.missing_binaries.is_empty() {
            lines.push(format!(
                "  Missing: {}",
                self.ci.missing_binaries.join(", ")
            ));
        }
        lines.push(format!(
            "LLM token ({}) loaded: {}",
            self.llm.token_env,
            yes_no(self.llm.token_loaded)
        ));
        lines.push(format!(
            "LLM base URL ({}) reachable: {}",
            self.llm.base_url,
            yes_no(self.llm.base_url_reachable)
        ));
        if let Some(bootstrap) = &self.bootstrap {
            lines.push(format!(
                "Bootstrap healthy (state: {}): {}",
                bootstrap.state_file.display(),
                yes_no(bootstrap.healthy)
            ));
            if !bootstrap.skip_markers_present.is_empty() {
                let markers = bootstrap
                    .skip_markers_present
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                lines.push(format!("  Skip markers present: {markers}"));
            }
        }
        if self.issues.is_empty() {
            lines.push("✅ All doctor checks passed".into());
        } else {
            lines.push("⚠️ Issues detected:".into());
            for issue in &self.issues {
                lines.push(format!("- {issue}"));
            }
        }
        lines
    }
}

#[derive(Debug, Clone)]
pub struct CiDoctorReport {
    pub fmt_ready: bool,
    pub check_ready: bool,
    pub test_ready: bool,
    pub missing_binaries: Vec<String>,
}

impl CiDoctorReport {
    fn all_ready(&self) -> bool {
        self.fmt_ready && self.check_ready && self.test_ready
    }
}

#[derive(Debug, Clone)]
pub struct LlmDoctorReport {
    pub provider: LlmProvider,
    pub token_env: String,
    pub token_loaded: bool,
    pub base_url: String,
    pub base_url_reachable: bool,
}

#[derive(Debug, Clone)]
pub struct BootstrapDoctorReport {
    pub state_file: PathBuf,
    pub skip_markers_present: Vec<PathBuf>,
    pub last_entry: Option<BootstrapLogEntry>,
    pub healthy: bool,
    pub issues: Vec<String>,
}

pub fn run_doctor(config_path: impl AsRef<Path>) -> Result<DoctorReport> {
    let config_path = absolutize_path(config_path.as_ref())?;
    let config = TddConfig::load_from_file(&config_path)?;
    let root = resolve_root(&config_path)?;

    let mut issues = Vec::new();

    let (git_clean, git_issue) = evaluate_git_state(&root);
    if let Some(issue) = git_issue {
        issues.push(issue);
    }

    let ci = check_ci_commands(&config.ci);
    if !ci.all_ready() {
        issues.push(format!(
            "Missing CI binaries: {}",
            ci.missing_binaries.join(", ")
        ));
    }

    let llm = evaluate_llm(&config);
    if !llm.token_loaded {
        issues.push(format!(
            "LLM token {} is not set in the environment",
            llm.token_env
        ));
    }
    if !llm.base_url_reachable {
        issues.push(format!(
            "Cannot reach configured LLM base URL {}",
            llm.base_url
        ));
    }

    let bootstrap = config
        .workspace
        .bootstrap
        .as_ref()
        .map(|cfg| inspect_bootstrap(&root, cfg));
    if let Some(report) = &bootstrap {
        issues.extend(report.issues.clone());
    }

    Ok(DoctorReport {
        git_clean,
        ci,
        llm,
        bootstrap,
        issues,
    })
}

fn evaluate_git_state(root: &Path) -> (bool, Option<String>) {
    let git_dir = root.join(".git");
    if !git_dir.exists() {
        return (
            false,
            Some("Git repository not initialized. Run `tdd-cli init` first.".into()),
        );
    }

    match GitVcs::open_or_init(root)
        .context("failed to open git repository")
        .and_then(|vcs| vcs.state().context("failed to read git status"))
    {
        Ok(state) => {
            if state.is_clean {
                (true, None)
            } else {
                (
                    false,
                    Some(
                        "Workspace has uncommitted changes; stash or commit before running `tdd-cli run`.".
                            into(),
                    ),
                )
            }
        }
        Err(err) => (false, Some(format!("Failed to inspect git repository: {err}"))),
    }
}

fn check_ci_commands(ci: &CiConfig) -> CiDoctorReport {
    let fmt_ready = command_binary_available(ci.fmt.first());
    let check_ready = command_binary_available(ci.check.first());
    let test_ready = command_binary_available(ci.test.first());

    let mut missing = Vec::new();
    if !fmt_ready {
        if let Some(cmd) = ci.fmt.first() {
            missing.push(cmd.clone());
        }
    }
    if !check_ready {
        if let Some(cmd) = ci.check.first() {
            missing.push(cmd.clone());
        }
    }
    if !test_ready {
        if let Some(cmd) = ci.test.first() {
            missing.push(cmd.clone());
        }
    }

    CiDoctorReport {
        fmt_ready,
        check_ready,
        test_ready,
        missing_binaries: missing,
    }
}

fn evaluate_llm(config: &TddConfig) -> LlmDoctorReport {
    let token_env = config.llm.api_key_env.clone();
    let token_loaded = env::var(&token_env)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let base_url = config.llm.base_url.clone();
    let base_url_reachable = is_base_url_reachable(&base_url);

    LlmDoctorReport {
        provider: config.llm.provider,
        token_env,
        token_loaded,
        base_url,
        base_url_reachable,
    }
}

fn inspect_bootstrap(root: &Path, config: &BootstrapConfig) -> BootstrapDoctorReport {
    let skip_markers_present = config
        .skip_files
        .iter()
        .map(|path| root.join(path))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();

    let state_file = root.join(BOOTSTRAP_STATE_FILE);
    let mut issues = Vec::new();
    let last_entry = match fs::read_to_string(&state_file) {
        Ok(contents) => match serde_json::from_str::<BootstrapLogEntry>(&contents) {
            Ok(entry) => Some(entry),
            Err(err) => {
                issues.push(format!(
                    "Failed to parse bootstrap state file {}: {err}",
                    state_file.display()
                ));
                None
            }
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            issues.push(format!(
                "Bootstrap configured but no state file found at {}. Run `tdd-cli provision`.",
                state_file.display()
            ));
            None
        }
        Err(err) => {
            issues.push(format!(
                "Failed to read bootstrap state file {}: {err}",
                state_file.display()
            ));
            None
        }
    };

    let mut healthy = true;
    if let Some(entry) = &last_entry {
        if entry.skipped {
            if skip_markers_present.is_empty() {
                healthy = false;
                issues.push(
                    "Bootstrap skipped but no skip markers are currently present".into(),
                );
            }
        } else if entry.exit_code != Some(0) {
            healthy = false;
            issues.push(format!(
                "Last bootstrap run exited with code {:?}",
                entry.exit_code
            ));
        }
    } else {
        healthy = false;
    }

    BootstrapDoctorReport {
        state_file,
        skip_markers_present,
        last_entry,
        healthy,
        issues,
    }
}

fn command_binary_available(command: Option<&String>) -> bool {
    let binary = match command {
        Some(value) => value,
        None => return false,
    };
    let path = Path::new(binary);
    if path.components().count() > 1 || path.is_absolute() {
        return path.is_file();
    }

    if let Some(paths) = env::var_os("PATH") {
        for dir in env::split_paths(&paths) {
            let candidate = dir.join(binary);
            if candidate.is_file() {
                return true;
            }
        }
    }
    false
}

fn is_base_url_reachable(url: &str) -> bool {
    let parsed = match Url::parse(url) {
        Ok(parsed) => parsed,
        Err(_) => return false,
    };

    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return false;
    }
    let host = match parsed.host_str() {
        Some(host) => host,
        None => return false,
    };
    let port = parsed.port().unwrap_or(if scheme == "https" { 443 } else { 80 });
    let addr = format!("{host}:{port}");

    match addr.to_socket_addrs() {
        Ok(mut addrs) => {
            let timeout = Duration::from_secs(2);
            addrs.any(|addr| TcpStream::connect_timeout(&addr, timeout).is_ok())
        }
        Err(_) => false,
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
