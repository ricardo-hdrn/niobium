//! Niobium CLI — native GUI runtime for CLI AI agents.
//!
//! Usage:
//!   niobium              → starts MCP server (trampolines to Flutter app)
//!   niobium serve        → same as above
//!   niobium install claude → registers with Claude Code
//!   niobium version      → prints version

use std::path::PathBuf;
use std::process::Command;

use clap::{Parser, Subcommand};
use tracing::info;

/// Minimum uptime (ms) before we consider the Flutter app "alive".
/// If it exits faster than this, we assume it crashed on startup.
const CRASH_THRESHOLD_MS: u128 = 2000;

#[derive(Parser)]
#[command(name = "niobium", about = "Native GUI runtime for CLI AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Start MCP server on stdio (default)
    Serve {
        /// Skip Flutter app, run MCP server without UI
        #[arg(long, env = "NIOBIUM_HEADLESS")]
        headless: bool,
    },
    /// Register Niobium with an AI agent
    Install {
        #[command(subcommand)]
        target: InstallTarget,
    },
    /// Print version
    Version,
}

#[derive(Subcommand)]
enum InstallTarget {
    /// Register with Claude Code via `claude mcp add`
    Claude,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Cmd::Serve { headless: false }) {
        Cmd::Serve { headless } => serve(headless),
        Cmd::Install { target } => install(target),
        Cmd::Version => {
            println!("niobium {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}

fn serve(headless: bool) -> anyhow::Result<()> {
    if !headless
        && let Some(flutter_bin) = find_flutter_binary()
    {
            info!("launching Flutter app: {}", flutter_bin.display());

            let start = std::time::Instant::now();
            let status = Command::new(&flutter_bin)
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .status();

            match status {
                Ok(s) if s.success() => {
                    std::process::exit(0);
                }
                Ok(s) => {
                    let uptime = start.elapsed().as_millis();
                    if uptime < CRASH_THRESHOLD_MS {
                        eprintln!(
                            "niobium: Flutter app crashed on startup (exit code: {}, uptime: {}ms) — falling back to headless mode",
                            s.code().unwrap_or(-1),
                            uptime
                        );
                    } else {
                        std::process::exit(s.code().unwrap_or(1));
                    }
                }
                Err(e) => {
                    eprintln!(
                        "niobium: failed to launch Flutter app: {e} — falling back to headless mode"
                    );
                }
            }
    }

    // Headless mode: MCP server works but UI tools return cancelled/false.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    info!(
        "{}",
        if headless {
            "starting in headless mode (--headless)"
        } else {
            "Flutter app not found — starting in headless mode"
        }
    );

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(niobium_mcp::api::start_mcp_server_headless())
}

/// Locate the Flutter app binary adjacent to this binary.
///
/// Checks the same paths as the old config.rs `find_flutter_binary`,
/// plus the `NIOBIUM_FLUTTER_BIN` env override.
fn find_flutter_binary() -> Option<PathBuf> {
    // 1. Environment variable override
    if let Ok(path) = std::env::var("NIOBIUM_FLUTTER_BIN") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Next to our own binary
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;

    // niobium-app (symlink or renamed — also covers macOS symlink to .app)
    let dash = dir.join("niobium-app");
    if dash.exists() {
        return Some(dash);
    }

    // niobium_app (Flutter's default output name)
    let underscore = dir.join("niobium_app");
    if underscore.exists() {
        return Some(underscore);
    }

    // Bundle directory (npm distribution): niobium-app/niobium_app
    let bundle = dir.join("niobium-app").join("niobium_app");
    if bundle.exists() {
        return Some(bundle);
    }

    // Windows: niobium-app/niobium_app.exe
    let bundle_exe = dir.join("niobium-app").join("niobium_app.exe");
    if bundle_exe.exists() {
        return Some(bundle_exe);
    }

    // macOS .app bundle: niobium-app.app/Contents/MacOS/niobium_app
    let app_bundle = dir
        .join("niobium-app.app")
        .join("Contents")
        .join("MacOS")
        .join("niobium_app");
    if app_bundle.exists() {
        return Some(app_bundle);
    }

    None
}

fn install(target: InstallTarget) -> anyhow::Result<()> {
    match target {
        InstallTarget::Claude => install_claude(),
    }
}

fn install_claude() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let exe_str = exe.to_str().expect("binary path is not valid UTF-8");

    println!("Registering Niobium with Claude Code...");

    let status = Command::new("claude")
        .args([
            "mcp",
            "add",
            "--transport",
            "stdio",
            "niobium",
            "--",
            exe_str,
            "serve",
        ])
        .status()?;

    if status.success() {
        println!("Done! Restart Claude Code to pick up the new MCP server.");
    } else {
        anyhow::bail!(
            "`claude mcp add` failed (exit code: {}). Is Claude Code installed?",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}
