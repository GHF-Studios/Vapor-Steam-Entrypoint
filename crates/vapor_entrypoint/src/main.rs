//! Steam-facing entrypoint for Vapor's player and creator modes.
//!
//! A launch without arguments starts the persistently selected composition.
//! The `creator` mode hands off to a standalone Vapor installation, while
//! `chooser` exposes the bundled chooser for local development and recovery.
//! `shell` and `installer` retain recovery paths.

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

const VAPOR_DOWNLOADS_URL: &str = "https://vapor.ghf-studios.site/downloads";

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
    let product_root = discover_product_root(&executable).ok_or_else(|| {
        format!(
            "could not discover product root from executable '{}'",
            executable.display()
        )
    })?;
    let app_root = shell_safe_app_root(app_root);
    let mut log = EntryLog::open();
    let arguments: Vec<OsString> = env::args_os().skip(1).collect();
    log.write(format!(
        "entrypoint executable={} product_root={} args={:?}",
        executable.display(),
        product_root.display(),
        redacted_arguments(&arguments)
    ));

    if arguments
        .first()
        .is_some_and(|argument| argument == "terminal")
    {
        let vapor_home = discover_vapor_home().ok_or_else(|| vapor_missing_message(&mut log))?;
        let (working_directory, program, forwarded) = terminal_request(&arguments)?;
        return launch_terminal_command(
            &vapor_home,
            &product_root,
            &working_directory,
            &program,
            &forwarded,
            &mut log,
        );
    }

    // Steam's primary entrypoint launches the depot-owned default App directly.
    // Machine integration and role setup are optional Installer concerns.
    if arguments.is_empty()
        || arguments
            .first()
            .is_some_and(|argument| argument == "play" || argument == "game")
    {
        return launch_bundled_default(&executable, &app_root, &mut log);
    }

    if arguments
        .first()
        .is_some_and(|argument| argument == "creator")
    {
        return launch_standalone_creator(&mut log);
    }

    if starts_gui(&arguments) {
        let mode = entrypoint_mode(&arguments);
        let vapor_home = discover_vapor_home().ok_or_else(|| vapor_missing_message(&mut log))?;
        if requires_client_service(&arguments) {
            ensure_client_service(&vapor_home, &product_root, &mode, &mut log)?;
        }
        let gui_arguments = gui_arguments(&arguments);
        return launch_gui(&vapor_home, &product_root, &gui_arguments, &mut log);
    }

    if arguments
        .first()
        .is_some_and(|argument| argument == "shell")
    {
        let vapor_home = discover_vapor_home().ok_or_else(|| vapor_missing_message(&mut log))?;
        ensure_client_service(&vapor_home, &product_root, "shell", &mut log)?;
        let shell = vapor_binary(&vapor_home, "vapor");
        if !shell.is_file() {
            return Err(format!("Vapor Shell is missing: {}", shell.display()));
        }
        return launch_terminal_command(
            &vapor_home,
            &product_root,
            &product_root,
            &shell,
            &arguments[1..],
            &mut log,
        );
    }

    Err(format!(
        "unknown Vapor Steam entrypoint mode: {}",
        arguments[0].to_string_lossy()
    ))
}

fn starts_gui(arguments: &[OsString]) -> bool {
    arguments.first().is_some_and(|argument| {
        argument == "studio"
            || argument == "launcher"
            || argument == "chooser"
            || argument == "installer"
            || argument == "ide"
            || argument == "cleanup"
    })
}

fn entrypoint_mode(arguments: &[OsString]) -> String {
    arguments
        .first()
        .map(|argument| argument.to_string_lossy().into_owned())
        .unwrap_or_else(|| "game".to_owned())
}

fn requires_client_service(arguments: &[OsString]) -> bool {
    !arguments.first().is_some_and(|argument| {
        argument == "installer" || argument == "chooser" || argument == "game" || argument == "play"
    })
}

fn gui_arguments(arguments: &[OsString]) -> Vec<OsString> {
    match arguments.first().map(|argument| argument.to_string_lossy()) {
        Some(mode)
            if mode == "studio"
                || mode == "launcher"
                || mode == "chooser"
                || mode == "installer"
                || mode == "ide"
                || mode == "cleanup" =>
        {
            let mut forwarded = vec![OsString::from("--mode"), OsString::from(mode.as_ref())];
            forwarded.extend_from_slice(&arguments[1..]);
            forwarded
        }
        _ => vec![OsString::from("--mode"), OsString::from("chooser")],
    }
}

fn launch_bundled_default(
    entrypoint: &Path,
    app_root: &Path,
    log: &mut EntryLog,
) -> Result<ExitStatus, String> {
    let executable = vapor_binary(vapor_home, "vapor");
    if !executable.is_file() {
        let message = format!("Vapor runtime is missing: {}", executable.display());
        log.write(&message);
        return Err(format!("{message}; repair or locally deploy Vapor Client"));
    }

    log.write(format!(
        "launching the bundled default App through {}",
        executable.display()
    ));
    let output = log.stderr().ok_or_else(|| {
        format!(
            "could not open entrypoint log for game launch: {}",
            log.path.display()
        )
    })?;
    let mut command = Command::new(&executable);
    command
        .args(["launch", "bundled-default"])
        .current_dir(app_root)
        .env("VAPOR_HOME", vapor_home)
        .env("VAPOR_PRODUCT_ROOT", app_root)
        .stdout(output.try_clone().map_err(|error| error.to_string())?)
        .stderr(output);
    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW

    let status = command
        .status()
        .map_err(|error| format!("failed to launch the selected composition: {error}"))?;
    log.write(format!("bundled default launch exited with {status}"));
    Ok(status)
}

fn launch_standalone_creator(log: &mut EntryLog) -> Result<ExitStatus, String> {
    let layout = vapor_product_layout::StandaloneProductLayout::discover()
        .ok_or_else(|| "could not determine Vapor User Data location".to_owned())?;
    let entrypoint = layout
        .active_entrypoint()
        .map_err(|error| format!("could not resolve the installed Vapor release: {error}"))?;

    if let Some(entrypoint) = entrypoint {
        log.write(format!(
            "launching standalone Vapor creator through {}",
            entrypoint.display()
        ));
        let mut command = Command::new(entrypoint);
        command
            .arg("chooser")
            .current_dir(layout.root())
            .env_remove("VAPOR_HOME")
            .env_remove("VAPOR_USER_DATA_ROOT");
        #[cfg(windows)]
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW

        return command
            .status()
            .map_err(|error| format!("failed to launch standalone Vapor: {error}"));
    }

    log.write(format!(
        "standalone Vapor is not installed; opening {VAPOR_DOWNLOADS_URL}"
    ));
    open_downloads_page()
}

fn open_downloads_page() -> Result<ExitStatus, String> {
    let mut command = if cfg!(windows) {
        let mut command = Command::new("cmd.exe");
        command.args(["/C", "start", "", VAPOR_DOWNLOADS_URL]);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(VAPOR_DOWNLOADS_URL);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(VAPOR_DOWNLOADS_URL);
        command
    };

    #[cfg(windows)]
    command.creation_flags(0x08000000); // CREATE_NO_WINDOW

    command
        .status()
        .map_err(|error| format!("failed to open Vapor Downloads: {error}"))
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
    use super::{
        entrypoint_mode, gui_arguments, requires_client_service, starts_gui, terminal_request,
    };
    use std::{ffi::OsString, path::PathBuf};

    #[test]
    fn installer_can_start_without_the_shared_service() {
        assert!(!requires_client_service(&[OsString::from("installer")]));
    }

    #[test]
    fn normal_frontends_keep_the_shared_service_bootstrap() {
        for mode in ["studio", "launcher", "shell", "ide"] {
            assert!(requires_client_service(&[OsString::from(mode)]));
        }
    }

    #[test]
    fn default_entrypoint_is_the_direct_game_route() {
        assert!(!starts_gui(&[]));
        assert_eq!(entrypoint_mode(&[]), "game");
    }

    #[test]
    fn launcher_is_a_separate_player_gui_route() {
        let arguments = [OsString::from("launcher")];
        assert!(starts_gui(&arguments));
        assert_eq!(entrypoint_mode(&arguments), "launcher");
        assert_eq!(
            gui_arguments(&arguments),
            [OsString::from("--mode"), OsString::from("launcher")]
        );
    }

    #[test]
    fn chooser_is_a_bootstrap_free_gui_entrypoint() {
        let arguments = [OsString::from("chooser")];
        assert!(!requires_client_service(&arguments));
        assert!(starts_gui(&arguments));
        assert_eq!(
            gui_arguments(&arguments),
            [OsString::from("--mode"), OsString::from("chooser")]
        );
    }

    #[test]
    fn rust_ide_is_a_gui_mode_that_uses_the_shared_client_service() {
        let arguments = [
            OsString::from("ide"),
            OsString::from("--workspace"),
            OsString::from("/workspace/rust-project"),
        ];
        assert!(starts_gui(&arguments));
        assert!(requires_client_service(&arguments));
        assert_eq!(
            gui_arguments(&arguments),
            [
                OsString::from("--mode"),
                OsString::from("ide"),
                OsString::from("--workspace"),
                OsString::from("/workspace/rust-project"),
            ]
        );
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
    vapor_home: &Path,
    product_root: &Path,
    mode: &str,
    log: &mut EntryLog,
) -> Result<(), String> {
    let service = vapor_binary(vapor_home, "vapor-client-service");
    if !service.is_file() {
        let message = format!("Vapor Client service is missing: {}", service.display());
        log.write(&message);
        return Err(format!("{message}; repair or locally deploy Vapor Client"));
    }
    let application_executable = vapor_binary(vapor_home, mode_binary_name(mode));
    log.write(format!(
        "ensuring shared Vapor Client service from {}",
        service.display()
    ));
    let mut command = Command::new(&service);
    command
        .arg("--ensure")
        .arg("--application-executable")
        .arg(&application_executable)
        .current_dir(product_root)
        .env("VAPOR_HOME", vapor_home)
        .env("VAPOR_PRODUCT_ROOT", product_root);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let status = command
        .status()
        .map_err(|error| format!("could not start Vapor Client service bootstrap: {error}"))?;
    if !status.success() {
        let message = format!("Vapor Client service bootstrap exited with {status}");
        log.write(&message);
        return Err(message);
    }
    Ok(())
}

fn mode_binary_name(mode: &str) -> &'static str {
    match mode {
        "game" | "play" | "shell" => "vapor",
        "launcher" | "studio" | "ide" | "installer" | "chooser" => "vapor-gui",
        _ => "vapor",
    }
}

fn vapor_binary(vapor_home: &Path, name: &str) -> PathBuf {
    vapor_home
        .join("bin")
        .join(host_runtime_target())
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

fn host_runtime_target() -> &'static str {
    #[cfg(all(windows, target_arch = "x86_64"))]
    {
        "x86_64-pc-windows-gnullvm"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "x86_64-unknown-linux-gnu"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "aarch64-unknown-linux-gnu"
    }
    #[cfg(not(any(
        all(windows, target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64")
    )))]
    {
        "unsupported-host"
    }
}

fn discover_vapor_home() -> Option<PathBuf> {
    let home = env::var_os("VAPOR_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            #[cfg(windows)]
            let home = env::var_os("USERPROFILE").or_else(|| env::var_os("HOME"));
            #[cfg(not(windows))]
            let home = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE"));
            home.map(PathBuf::from).map(|home| home.join("Vapor"))
        })?;
    let marker = home.join("Vapor-Home.toml");
    let source = fs::read_to_string(marker).ok()?;
    let mut lines = source.lines().map(str::trim);
    if lines.next() != Some("schema = 1")
        || lines.next() != Some("")
        || lines.next() != Some("[vapor]")
        || lines.next() != Some("id = \"ghf-studios/vapor\"")
        || lines.next().is_some()
    {
        return None;
    }
    Some(home)
}

fn vapor_missing_message(log: &mut EntryLog) -> String {
    let message = "Vapor is not installed. Download it from https://vapor.ghf-studios.site and run its installer, then launch this Steam entry again.";
    log.write(message);
    message.to_owned()
}

fn launch_gui(
    vapor_home: &Path,
    product_root: &Path,
    arguments: &[OsString],
    log: &mut EntryLog,
) -> Result<ExitStatus, String> {
    let mode = arguments
        .windows(2)
        .find(|pair| pair[0] == "--mode")
        .map(|pair| pair[1].to_string_lossy().into_owned())
        .unwrap_or_else(|| "chooser".to_owned());
    let executable = vapor_binary(vapor_home, mode_binary_name(&mode));
    let product = match mode.as_str() {
        "play" | "launcher" => "Vapor Launcher",
        "studio" => "Vapor Studio",
        "ide" => "Vapor IDE",
        "installer" => "Vapor Installer",
        "cleanup" => "Vapor Cleanup",
        _ => "Vapor chooser",
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
        .current_dir(product_root)
        .env("VAPOR_HOME", vapor_home)
        .env("VAPOR_PRODUCT_ROOT", product_root);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    let status = command
        .status()
        .map_err(|error| format!("failed to launch {product}: {error}"))?;
    log.write(format!("{product} exited with {status}"));
    Ok(status)
}

include!("entrypoint/product_root.rs");
include!("entrypoint/terminal.rs");
include!("entrypoint/redaction.rs");
include!("entrypoint/log.rs");

#[cfg(test)]
mod launch_mode_tests {
    use super::{gui_arguments, host_runtime_target, mode_binary_name, starts_gui, vapor_binary};
    use std::ffi::OsString;

    #[test]
    fn no_arguments_launch_the_default_composition_without_a_gui() {
        assert!(!starts_gui(&[]));
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
    fn play_and_game_modes_use_the_vapor_runtime() {
        for mode in ["play", "game"] {
            assert!(!starts_gui(&[OsString::from(mode)]));
            let expected = format!("vapor{}", std::env::consts::EXE_SUFFIX);
            assert_eq!(
                vapor_binary(std::path::Path::new("/vapor"), "vapor")
                    .file_name()
                    .unwrap(),
                std::ffi::OsStr::new(&expected)
            );
        }
    }

    #[test]
    fn each_gui_product_is_resolved_under_vapor_home() {
        let vapor_home = std::path::Path::new("/vapor");
        for (mode, base_name) in [
            ("play", "vapor"),
            ("game", "vapor"),
            ("launcher", "vapor-gui"),
            ("chooser", "vapor-gui"),
            ("studio", "vapor-gui"),
            ("ide", "vapor-gui"),
            ("installer", "vapor-gui"),
        ] {
            let name = format!("{base_name}{}", std::env::consts::EXE_SUFFIX);
            assert_eq!(
                vapor_binary(vapor_home, mode_binary_name(mode)),
                vapor_home
                    .join("bin")
                    .join(host_runtime_target())
                    .join(name)
            );
        }
    }
}
