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

    if arguments
        .first()
        .is_some_and(|argument| argument == "terminal")
    {
        let (working_directory, program, forwarded) = terminal_request(&arguments)?;
        return launch_terminal_command(
            &app_root,
            &working_directory,
            &program,
            &forwarded,
            &mut log,
        );
    }

    if starts_gui(&arguments) {
        if arguments
            .first()
            .is_some_and(|argument| argument == "installer")
            && !gui_executable(&executable, false).is_file()
        {
            let script = platform_script(&app_root);
            if !script.is_file() {
                return Err(format!(
                    "installer fallback script is missing: {}",
                    script.display()
                ));
            }
            return launch_terminal(&app_root, &script, &arguments, &mut log);
        }
        if requires_client_service(&arguments) {
            ensure_client_service(
                &executable,
                &app_root,
                arguments.first().is_some_and(|argument| argument == "play"),
                &mut log,
            )?;
        } else {
            log.write("launching Installer without the shared service for recovery access");
        }
        let gui_arguments = gui_arguments(&arguments);
        return launch_gui(&executable, &app_root, &gui_arguments, &mut log);
    }

    let script = platform_script(&app_root);
    if !script.is_file() {
        let message = format!("launch script is missing: {}", script.display());
        log.write(&message);
        return Err(message);
    }

    ensure_client_service(&executable, &app_root, false, &mut log)?;
    launch_terminal(&app_root, &script, &arguments, &mut log)
}

fn starts_gui(arguments: &[OsString]) -> bool {
    arguments.first().is_none_or(|argument| {
        argument == "studio" || argument == "play" || argument == "installer"
    })
}

fn requires_client_service(arguments: &[OsString]) -> bool {
    !arguments
        .first()
        .is_some_and(|argument| argument == "installer")
}

fn gui_arguments(arguments: &[OsString]) -> Vec<OsString> {
    match arguments.first().map(|argument| argument.to_string_lossy()) {
        Some(mode) if mode == "studio" || mode == "play" || mode == "installer" => {
            let mut forwarded = vec![OsString::from("--mode"), OsString::from(mode.as_ref())];
            forwarded.extend_from_slice(&arguments[1..]);
            forwarded
        }
        _ => vec![OsString::from("--mode"), OsString::from("studio")],
    }
}

fn terminal_request(arguments: &[OsString]) -> Result<(PathBuf, PathBuf, Vec<OsString>), String> {
    if arguments.len() < 5 || arguments[1] != "--working-directory" || arguments[3] != "--" {
        return Err(
            "terminal launch expects: terminal --working-directory PATH -- PROGRAM [ARGS...]"
                .to_owned(),
        );
    }
    let working_directory = PathBuf::from(&arguments[2]);
    if !working_directory.is_dir() {
        return Err(format!(
            "terminal launch working directory does not exist: {}",
            working_directory.display()
        ));
    }
    let program = PathBuf::from(&arguments[4]);
    if !program.is_file() {
        return Err(format!(
            "terminal launch program does not exist: {}",
            program.display()
        ));
    }
    Ok((working_directory, program, arguments[5..].to_vec()))
}

#[cfg(test)]
mod tests {
    use super::{requires_client_service, terminal_request};
    use std::{ffi::OsString, path::PathBuf};

    #[test]
    fn installer_can_start_without_the_shared_service() {
        assert!(!requires_client_service(&[OsString::from("installer")]));
    }

    #[test]
    fn normal_frontends_keep_the_shared_service_bootstrap() {
        for mode in ["studio", "play", "shell"] {
            assert!(requires_client_service(&[OsString::from(mode)]));
        }
    }

    #[test]
    fn terminal_request_requires_an_existing_working_directory_and_program() {
        let arguments = [
            OsString::from("terminal"),
            OsString::from("--working-directory"),
            OsString::from("/tmp"),
            OsString::from("--"),
            OsString::from("/bin/true"),
            OsString::from("argument"),
        ];
        let (working_directory, program, forwarded) = terminal_request(&arguments).unwrap();
        assert_eq!(working_directory, PathBuf::from("/tmp"));
        assert_eq!(program, PathBuf::from("/bin/true"));
        assert_eq!(forwarded, [OsString::from("argument")]);
        assert!(terminal_request(&arguments[..4]).is_err());
    }
}

fn ensure_client_service(
    entrypoint: &Path,
    app_root: &Path,
    player_frontend: bool,
    log: &mut EntryLog,
) -> Result<(), String> {
    let service = entrypoint
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(if cfg!(windows) {
            "vapor-client-service.exe"
        } else {
            "vapor-client-service"
        });
    if !service.is_file() {
        let message = format!("Vapor Client service is missing: {}", service.display());
        log.write(&message);
        return Err(format!("{message}; repair or locally deploy Vapor Client"));
    }
    let application_executable = gui_executable(entrypoint, player_frontend);
    log.write(format!(
        "ensuring shared Vapor Client service from {}",
        service.display()
    ));
    let status = Command::new(&service)
        .arg("--ensure")
        .arg("--application-executable")
        .arg(&application_executable)
        .arg("--workshop-app-id")
        .arg("2122620")
        .current_dir(app_root)
        .env("VAPOR_HOME", app_root)
        .status()
        .map_err(|error| format!("could not start Vapor Client service bootstrap: {error}"))?;
    if !status.success() {
        let message = format!("Vapor Client service bootstrap exited with {status}");
        log.write(&message);
        return Err(message);
    }
    Ok(())
}

fn gui_executable(entrypoint: &Path, player_frontend: bool) -> PathBuf {
    let executable = match (player_frontend, cfg!(windows)) {
        (true, true) => "vapor-launcher.exe",
        (true, false) => "vapor-launcher",
        (false, true) => "vapor-gui.exe",
        (false, false) => "vapor-gui",
    };
    entrypoint
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(executable)
}

fn launch_gui(
    entrypoint: &Path,
    app_root: &Path,
    arguments: &[OsString],
    log: &mut EntryLog,
) -> Result<ExitStatus, String> {
    let player_frontend = arguments
        .windows(2)
        .any(|pair| pair[0] == "--mode" && pair[1] == "play");
    let executable = gui_executable(entrypoint, player_frontend);
    let product = if player_frontend {
        "Vapor Launcher"
    } else {
        "Vapor GUI"
    };
    if !executable.is_file() {
        let message = format!("{product} is missing: {}", executable.display());
        log.write(&message);
        return Err(format!(
            "{message}; build and deploy the Vapor Client workspace"
        ));
    }
    log.write(format!("launching {product} {}", executable.display()));
    let mut command = Command::new(executable);
    command
        .args(arguments)
        .current_dir(app_root)
        .env("VAPOR_HOME", app_root);
    let status = command
        .status()
        .map_err(|error| format!("failed to launch {product}: {error}"))?;
    log.write(format!("{product} exited with {status}"));
    Ok(status)
}

include!("entrypoint/app_root.rs");
include!("entrypoint/terminal.rs");
include!("entrypoint/redaction.rs");
include!("entrypoint/log.rs");

#[cfg(test)]
mod launch_mode_tests {
    use super::{gui_arguments, gui_executable, starts_gui};
    use std::ffi::OsString;

    #[test]
    fn no_arguments_launch_studio_gui() {
        assert!(starts_gui(&[]));
    }

    #[test]
    fn studio_mode_is_forwarded_to_gui_host() {
        let arguments = [
            OsString::from("studio"),
            OsString::from("--workspace"),
            OsString::from("."),
        ];
        assert!(starts_gui(&arguments));
        assert_eq!(
            gui_arguments(&arguments),
            [
                OsString::from("--mode"),
                OsString::from("studio"),
                OsString::from("--workspace"),
                OsString::from(".")
            ]
        );
    }

    #[test]
    fn game_and_shell_modes_keep_terminal_routing() {
        assert!(!starts_gui(&[OsString::from("shell")]));
        assert!(!starts_gui(&[OsString::from("game")]));
    }

    #[test]
    fn play_uses_the_dedicated_launcher_binary() {
        let arguments = [OsString::from("play")];
        assert!(starts_gui(&arguments));
        assert_eq!(
            gui_arguments(&arguments),
            [OsString::from("--mode"), OsString::from("play")]
        );
    }

    #[test]
    fn gui_binary_is_resolved_as_entrypoint_sibling_for_any_target_triple() {
        let entrypoint =
            std::path::Path::new("/vapor/bin/x86_64-pc-windows-gnullvm/vapor-entrypoint.exe");
        let expected_name = if cfg!(windows) {
            "vapor-gui.exe"
        } else {
            "vapor-gui"
        };

        assert_eq!(
            gui_executable(entrypoint, false),
            entrypoint.parent().unwrap().join(expected_name)
        );
        let launcher_name = if cfg!(windows) {
            "vapor-launcher.exe"
        } else {
            "vapor-launcher"
        };
        assert_eq!(
            gui_executable(entrypoint, true),
            entrypoint.parent().unwrap().join(launcher_name)
        );
    }
}
