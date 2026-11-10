fn discover_app_root(executable: &Path) -> Option<PathBuf> {
    let executable = fs::canonicalize(executable).ok()?;
    let directory = executable.parent()?;
    if directory.file_name().is_some_and(|name| name == "bin") {
        return directory.parent().map(Path::to_path_buf);
    }
    if directory
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name == "bin")
    {
        return directory
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf);
    }
    // The Steam Packagepack locator is shipped in the product root. Accept
    // that layout as well as the older bin/<target>/ launch-option location.
    Some(directory.to_path_buf())
}

fn discover_vapor_home() -> Option<PathBuf> {
    if let Some(explicit) = env::var_os("VAPOR_HOME").filter(|value| !value.is_empty()) {
        let explicit = PathBuf::from(explicit);
        if is_vapor_home(&explicit) {
            return fs::canonicalize(explicit).ok();
        }
    }

    if let Some(user_home) = user_home_directory() {
        let default_home = PathBuf::from(user_home).join("Vapor");
        if is_vapor_home(&default_home) {
            return fs::canonicalize(default_home).ok();
        }
    }

    let executable_name = if cfg!(windows) { "vapor.exe" } else { "vapor" };
    env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
        .map(|directory| directory.join(executable_name))
        .filter(|executable| executable.is_file())
        .filter_map(|executable| vapor_home_from_executable(&executable))
        .find(|home| is_vapor_home(home))
        .and_then(|home| fs::canonicalize(home).ok())
}

fn user_home_directory() -> Option<OsString> {
    #[cfg(windows)]
    {
        env::var_os("USERPROFILE").or_else(|| env::var_os("HOME"))
    }
    #[cfg(not(windows))]
    {
        env::var_os("HOME").or_else(|| env::var_os("USERPROFILE"))
    }
}

fn vapor_home_from_executable(executable: &Path) -> Option<PathBuf> {
    let directory = executable.parent()?;
    if directory.file_name().is_some_and(|name| name == "bin") {
        return directory.parent().map(Path::to_path_buf);
    }
    if directory
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name == "bin")
    {
        return directory.parent()?.parent().map(Path::to_path_buf);
    }
    None
}

fn is_vapor_home(path: &Path) -> bool {
    if !path.join("Vapor-Home.toml").is_file() {
        return false;
    }
    let Ok(marker) = fs::read_to_string(path.join("Vapor-Home.toml")) else {
        return false;
    };
    marker.lines().any(|line| line.trim() == "[vapor]")
        && marker.lines().any(|line| line.trim() == "schema = 1")
}

fn runtime_bin_directory(vapor_home: &Path) -> PathBuf {
    vapor_home.join("bin").join(runtime_target())
}

fn runtime_target() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "x86_64-pc-windows-gnullvm"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "x86_64-unknown-linux-gnu"
    }
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64")
    )))]
    {
        "unsupported-target"
    }
}

fn show_user_error(title: &str, message: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let title: Vec<u16> = std::ffi::OsStr::new(title)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let message: Vec<u16> = std::ffi::OsStr::new(message)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        #[link(name = "user32")]
        unsafe extern "system" {
            fn MessageBoxW(window: *mut std::ffi::c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
        }
        // The entrypoint is a GUI-subsystem binary launched by Steam, so
        // stderr alone would leave the player with no actionable message.
        unsafe {
            MessageBoxW(std::ptr::null_mut(), message.as_ptr(), title.as_ptr(), 0x10);
        }
    }

    #[cfg(target_os = "linux")]
    {
        for (program, arguments) in [
            ("zenity", vec!["--error", "--title", title, "--text", message]),
            ("kdialog", vec!["--title", title, "--error", message]),
        ] {
            if Command::new(program).args(arguments).status().is_ok() {
                return;
            }
        }
    }
}

fn platform_script(app_root: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        app_root.join("bin").join("vapor-launch.cmd")
    }
    #[cfg(not(windows))]
    {
        app_root.join("bin").join("vapor-launch.sh")
    }
}

#[cfg(windows)]
fn shell_safe_app_root(path: PathBuf) -> PathBuf {
    strip_windows_verbatim_prefix(&path)
}

#[cfg(not(windows))]
fn shell_safe_app_root(path: PathBuf) -> PathBuf {
    path
}

#[cfg(windows)]
fn strip_windows_verbatim_prefix(path: &Path) -> PathBuf {
    let path = path.as_os_str().to_string_lossy();
    if let Some(path) = path.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{path}"));
    }
    if let Some(path) = path.strip_prefix(r"\\?\") {
        return PathBuf::from(path);
    }
    PathBuf::from(path.as_ref())
}
