#[cfg(target_os = "linux")]
fn launch_terminal(
    app_root: &Path,
    script: &Path,
    arguments: &[OsString],
    log: &mut EntryLog,
) -> Result<ExitStatus, String> {
    if env::var_os("DISPLAY").is_none() && env::var_os("WAYLAND_DISPLAY").is_none() {
        let message = "Konsole launch requires DISPLAY or WAYLAND_DISPLAY".to_owned();
        log.write(&message);
        return Err(message);
    }

    if is_steam_pressure_vessel() && Path::new("/run/host/usr/bin/konsole").is_file() {
        if let Some(loader) = host_loader() {
            log.write(format!(
                "launching host Konsole through loader {}",
                loader.display()
            ));
            let mut command = Command::new(loader);
            command
                .arg("--library-path")
                .arg(host_library_path())
                .arg("/run/host/usr/bin/konsole")
                .args(["--nofork", "--hold", "-p", "tabtitle=Vapor"])
                .arg("--workdir")
                .arg(app_root)
                .arg("-e")
                .arg("/usr/bin/env")
                .arg(format!("PATH={}", linux_child_path(app_root)))
                .arg(script)
                .arg("--hold")
                .args(arguments)
                .current_dir(app_root)
                .env_remove("LD_LIBRARY_PATH")
                .env_remove("LD_PRELOAD")
                .env_remove("LD_AUDIT")
                .env_remove("STEAM_RUNTIME_LIBRARY_PATH")
                .env(
                    "PATH",
                    format!(
                        "/run/host/usr/local/bin:/run/host/usr/bin:/run/host/bin:{}",
                        env::var_os("PATH")
                            .unwrap_or_else(|| OsString::from("/usr/bin:/bin"))
                            .to_string_lossy()
                    ),
                )
                .env(
                    "QT_PLUGIN_PATH",
                    "/run/host/usr/lib/qt6/plugins:/run/host/usr/lib/x86_64-linux-gnu/qt6/plugins",
                )
                .env(
                    "XDG_DATA_DIRS",
                    "/run/host/usr/local/share:/run/host/usr/share:/usr/share",
                );
            return wait_for_terminal(command, "host Konsole", log);
        }
        log.write("host Konsole exists but no /run/host dynamic loader was found");
    }

    let konsole = if Path::new("/usr/bin/konsole").is_file() {
        PathBuf::from("/usr/bin/konsole")
    } else {
        PathBuf::from("konsole")
    };
    log.write(format!("launching Konsole through {}", konsole.display()));
    let mut command = Command::new(konsole);
    command
        .args(["--nofork", "--hold", "-p", "tabtitle=Vapor"])
        .arg("--workdir")
        .arg(app_root)
        .arg("-e")
        .arg(script)
        .arg("--hold")
        .args(arguments)
        .current_dir(app_root);
    wait_for_terminal(command, "Konsole", log)
}

#[cfg(target_os = "linux")]
fn launch_terminal_command(
    app_root: &Path,
    working_directory: &Path,
    program: &Path,
    arguments: &[OsString],
    log: &mut EntryLog,
) -> Result<ExitStatus, String> {
    if env::var_os("DISPLAY").is_none() && env::var_os("WAYLAND_DISPLAY").is_none() {
        let message = "terminal launch requires DISPLAY or WAYLAND_DISPLAY".to_owned();
        log.write(&message);
        return Err(message);
    }

    let host_konsole = is_steam_pressure_vessel()
        .then(|| Path::new("/run/host/usr/bin/konsole"))
        .filter(|path| path.is_file());
    let mut command = if let (Some(konsole), Some(loader)) = (host_konsole, host_loader()) {
        log.write(format!("launching through host Konsole: {}", program.display()));
        let mut command = Command::new(loader);
        command
            .arg("--library-path")
            .arg(host_library_path())
            .arg(konsole)
            .args(["--nofork", "--hold", "-p", "tabtitle=Vapor"])
            .arg("--workdir")
            .arg(working_directory)
            .arg("-e")
            .arg("/usr/bin/env")
            .arg(format!("PATH={}", linux_child_path(app_root)))
            .arg(program)
            .args(arguments)
            .current_dir(working_directory)
            .env_remove("LD_LIBRARY_PATH")
            .env_remove("LD_PRELOAD")
            .env_remove("LD_AUDIT")
            .env_remove("STEAM_RUNTIME_LIBRARY_PATH")
            .env(
                "PATH",
                format!(
                    "/run/host/usr/local/bin:/run/host/usr/bin:/run/host/bin:{}",
                    env::var_os("PATH").unwrap_or_else(|| OsString::from("/usr/bin:/bin"))
                        .to_string_lossy()
                ),
            )
            .env(
                "QT_PLUGIN_PATH",
                "/run/host/usr/lib/qt6/plugins:/run/host/usr/lib/x86_64-linux-gnu/qt6/plugins",
            )
            .env(
                "XDG_DATA_DIRS",
                "/run/host/usr/local/share:/run/host/usr/share:/usr/share",
            );
        command
    } else {
        let konsole = if Path::new("/usr/bin/konsole").is_file() {
            PathBuf::from("/usr/bin/konsole")
        } else {
            PathBuf::from("konsole")
        };
        log.write(format!("launching through {}: {}", konsole.display(), program.display()));
        let mut command = Command::new(konsole);
        command
            .args(["--nofork", "--hold", "-p", "tabtitle=Vapor"])
            .arg("--workdir")
            .arg(working_directory)
            .arg("-e")
            .arg(program)
            .args(arguments)
            .current_dir(working_directory)
            .env("PATH", linux_child_path(app_root));
        command
    };
    if host_konsole.is_none() {
        command
            .env_remove("LD_LIBRARY_PATH")
            .env_remove("LD_PRELOAD")
            .env_remove("LD_AUDIT")
            .env_remove("STEAM_RUNTIME_LIBRARY_PATH");
    }
    wait_for_terminal(command, "terminal window", log)
}

#[cfg(windows)]
fn launch_terminal(
    app_root: &Path,
    _script: &Path,
    arguments: &[OsString],
    log: &mut EntryLog,
) -> Result<ExitStatus, String> {
    let shell = env::var_os("ComSpec").unwrap_or_else(|| OsString::from("cmd.exe"));
    log.write(format!(
        "launching command prompt through {}",
        PathBuf::from(&shell).display()
    ));
    let payload = windows_cmd_payload(arguments);
    log.write(format!(
        "command prompt args={:?}",
        redacted_arguments(arguments)
    ));
    let mut command = Command::new(shell);
    command
        .args(["/D", "/C"])
        .raw_arg(payload)
        .current_dir(app_root);
    wait_for_terminal(command, "Command Prompt", log)
}

#[cfg(windows)]
fn launch_terminal_command(
    _app_root: &Path,
    working_directory: &Path,
    program: &Path,
    arguments: &[OsString],
    log: &mut EntryLog,
) -> Result<ExitStatus, String> {
    let shell = env::var_os("ComSpec").unwrap_or_else(|| OsString::from("cmd.exe"));
    log.write(format!("launching terminal program {}", program.display()));
    let payload = windows_command_payload(program, arguments);
    let mut command = Command::new(shell);
    command
        .args(["/D", "/C"])
        .raw_arg(payload)
        .current_dir(working_directory);
    wait_for_terminal(command, "Command Prompt", log)
}

#[cfg(windows)]
fn windows_command_payload(program: &Path, arguments: &[OsString]) -> String {
    let mut command = if program.extension().is_some_and(|extension| extension == "cmd") {
        format!("call {}", quote_windows_cmd_part(program.as_os_str()))
    } else {
        quote_windows_cmd_part(program.as_os_str())
    };
    for argument in arguments {
        command.push(' ');
        command.push_str(&quote_windows_cmd_part(argument));
    }
    format!("\"{command}\"")
}

#[cfg(windows)]
fn windows_cmd_payload(arguments: &[OsString]) -> String {
    let mut command = format!(
        "call {} {}",
        quote_windows_cmd_part(OsStr::new(r"bin\vapor-launch.cmd")),
        quote_windows_cmd_part(OsStr::new("--hold"))
    );
    for argument in arguments {
        command.push(' ');
        command.push_str(&quote_windows_cmd_part(argument));
    }
    format!("\"{command}\"")
}

#[cfg(windows)]
fn quote_windows_cmd_part(value: &OsStr) -> String {
    let mut quoted = String::from("\"");
    for character in value.to_string_lossy().chars() {
        if character == '"' {
            quoted.push_str("\"\"");
        } else {
            quoted.push(character);
        }
    }
    quoted.push('"');
    quoted
}

#[cfg(not(any(target_os = "linux", windows)))]
fn launch_terminal(
    _app_root: &Path,
    _script: &Path,
    _arguments: &[OsString],
    log: &mut EntryLog,
) -> Result<ExitStatus, String> {
    let message = format!(
        "vapor-entrypoint has no terminal adapter for {}",
        env::consts::OS
    );
    log.write(&message);
    Err(message)
}

#[cfg(not(any(target_os = "linux", windows)))]
fn launch_terminal_command(
    _app_root: &Path,
    _working_directory: &Path,
    _program: &Path,
    _arguments: &[OsString],
    log: &mut EntryLog,
) -> Result<ExitStatus, String> {
    let message = format!(
        "vapor-entrypoint has no terminal adapter for {}",
        env::consts::OS
    );
    log.write(&message);
    Err(message)
}

fn wait_for_terminal(
    mut command: Command,
    label: &str,
    log: &mut EntryLog,
) -> Result<ExitStatus, String> {
    if let Some(stderr) = log.stderr() {
        command.stderr(Stdio::from(stderr));
    }
    log.write(format!("waiting for {label}"));
    let status = command
        .status()
        .map_err(|error| format!("failed to launch {label}: {error}"))?;
    log.write(format!("{label} exited with {status}"));
    Ok(status)
}

#[cfg(target_os = "linux")]
fn is_steam_pressure_vessel() -> bool {
    env::var_os("PRESSURE_VESSEL_RUNTIME").is_some()
        || env::var_os("container").is_some_and(|container| container == "pressure-vessel")
}

#[cfg(target_os = "linux")]
fn host_loader() -> Option<PathBuf> {
    [
        "/run/host/lib64/ld-linux-x86-64.so.2",
        "/run/host/usr/lib64/ld-linux-x86-64.so.2",
        "/run/host/lib/ld-linux-x86-64.so.2",
        "/run/host/usr/lib/ld-linux-x86-64.so.2",
        "/run/host/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2",
        "/run/host/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|candidate| candidate.is_file())
}

#[cfg(target_os = "linux")]
fn host_library_path() -> &'static std::ffi::OsStr {
    std::ffi::OsStr::new(
        "/run/host/usr/lib:/run/host/usr/lib64:/run/host/usr/lib/x86_64-linux-gnu:/run/host/usr/lib/pulseaudio:/run/host/usr/lib/libproxy:/run/host/usr/lib/qt6/plugins:/run/host/usr/lib/x86_64-linux-gnu/qt6/plugins:/run/host/lib:/run/host/lib64:/run/host/lib/x86_64-linux-gnu",
    )
}

#[cfg(target_os = "linux")]
fn linux_child_path(app_root: &Path) -> String {
    let mut paths = vec![
        app_root.join("bin/x86_64-unknown-linux-gnu"),
        app_root.join("bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ];
    if let Some(existing) = env::var_os("PATH") {
        paths.extend(
            env::split_paths(&existing).filter(|path| !is_raw_app_tool_path(app_root, path)),
        );
    }
    env::join_paths(paths)
        .unwrap_or_else(|_| OsString::from("/usr/bin:/bin"))
        .to_string_lossy()
        .into_owned()
}

#[cfg(target_os = "linux")]
fn is_raw_app_tool_path(app_root: &Path, path: &Path) -> bool {
    let raw_exact = [
        app_root.join("cargo-home/bin"),
        app_root.join("rustup/bin"),
        app_root.join("tools/steamcmd"),
        app_root.join("tools/zig"),
        app_root.join("tools/cross/bin"),
        app_root.join("tools/llvm-mingw/bin"),
    ];
    if raw_exact.iter().any(|raw| path == raw) {
        return true;
    }
    let toolchains = app_root.join("rustup-home/toolchains");
    path.starts_with(&toolchains) && path.file_name().is_some_and(|name| name == "bin")
}
