//! Steam-facing entrypoint for Vapor Studio and terminal recovery modes.
//!
//! Steam starts this executable directly into Studio. The `play` argument asks
//! Studio to run the selected composition; `shell` and `installer` retain their
//! terminal-based recovery paths.

#![cfg_attr(windows, windows_subsystem = "windows")]

use std::{
    env,
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    time::SystemTime,
};

#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

fn main() {
    let status = match run() {
        Ok(status) => status.code().unwrap_or(1),
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "vapor-entrypoint: {error}");
            1
        }
    };
    std::process::exit(status);
}

fn run() -> Result<ExitStatus, String> {
    let executable = env::current_exe()
        .map_err(|error| format!("failed to resolve vapor-entrypoint executable: {error}"))?;
    let app_root = discover_app_root(&executable).ok_or_else(|| {
        format!(
            "could not discover app root from executable '{}'",
            executable.display()
        )
    })?;
    let app_root = shell_safe_app_root(app_root);
    let mut log = EntryLog::open(&app_root);
    let arguments: Vec<OsString> = env::args_os().skip(1).collect();
    log.write(format!(
        "entrypoint executable={} app_root={} args={:?}",
        executable.display(),
        app_root.display(),
        redacted_arguments(&arguments)
    ));

    if starts_studio(&arguments) {
        let studio_arguments = studio_arguments(&arguments);
        return launch_studio(&executable, &app_root, &studio_arguments, &mut log);
    }

    let script = platform_script(&app_root);
    if !script.is_file() {
        let message = format!("launch script is missing: {}", script.display());
        log.write(&message);
        return Err(message);
    }

    launch_terminal(&app_root, &script, &arguments, &mut log)
}

fn starts_studio(arguments: &[OsString]) -> bool {
    arguments
        .first()
        .is_none_or(|argument| argument == "studio" || argument == "play")
}

fn studio_arguments(arguments: &[OsString]) -> Vec<OsString> {
    match arguments.first().map(|argument| argument.to_string_lossy()) {
        Some(mode) if mode == "studio" => arguments[1..].to_vec(),
        Some(mode) if mode == "play" => {
            let mut forwarded = vec![OsString::from("--play")];
            forwarded.extend_from_slice(&arguments[1..]);
            forwarded
        }
        _ => arguments.to_vec(),
    }
}

fn studio_executable(entrypoint: &Path) -> PathBuf {
    entrypoint
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(if cfg!(windows) {
            "vapor-studio.exe"
        } else {
            "vapor-studio"
        })
}

fn launch_studio(
    entrypoint: &Path,
    app_root: &Path,
    arguments: &[OsString],
    log: &mut EntryLog,
) -> Result<ExitStatus, String> {
    let executable = studio_executable(entrypoint);
    if !executable.is_file() {
        let message = format!("Vapor Studio is missing: {}", executable.display());
        log.write(&message);
        return Err(format!(
            "{message}; build and deploy the Vapor Client workspace"
        ));
    }
    log.write(format!("launching Studio {}", executable.display()));
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .current_dir(app_root)
        .env("VAPOR_HOME", app_root);
    let status = command
        .status()
        .map_err(|error| format!("failed to launch Vapor Studio: {error}"))?;
    log.write(format!("Vapor Studio exited with {status}"));
    Ok(status)
}

#[cfg(test)]
mod launch_mode_tests {
    use super::{starts_studio, studio_arguments, studio_executable};
    use std::ffi::OsString;

    #[test]
    fn no_arguments_launch_studio() {
        assert!(starts_studio(&[]));
    }

    #[test]
    fn studio_mode_is_removed_before_forwarding() {
        let arguments = [
            OsString::from("studio"),
            OsString::from("--workspace"),
            OsString::from("."),
        ];
        assert!(starts_studio(&arguments));
        assert_eq!(studio_arguments(&arguments), arguments[1..]);
    }

    #[test]
    fn game_and_shell_modes_keep_terminal_routing() {
        for mode in [OsString::from("shell"), OsString::from("installer")] {
            assert!(!starts_studio(&[mode]));
        }
    }

    #[test]
    fn play_opens_studio_in_direct_play_mode() {
        let arguments = [OsString::from("play")];
        assert!(starts_studio(&arguments));
        assert_eq!(studio_arguments(&arguments), [OsString::from("--play")]);
    }

    #[test]
    fn studio_binary_is_resolved_as_entrypoint_sibling_for_any_target_triple() {
        let entrypoint =
            std::path::Path::new("/vapor/bin/x86_64-pc-windows-gnullvm/vapor-entrypoint.exe");
        let expected_name = if cfg!(windows) {
            "vapor-studio.exe"
        } else {
            "vapor-studio"
        };

        assert_eq!(
            studio_executable(entrypoint),
            entrypoint.parent().unwrap().join(expected_name)
        );
    }
}

include!("entrypoint/app_root.rs");
include!("entrypoint/terminal.rs");
include!("entrypoint/redaction.rs");
include!("entrypoint/log.rs");
