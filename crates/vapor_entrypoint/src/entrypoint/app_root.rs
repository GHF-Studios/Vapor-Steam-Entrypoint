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
    None
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

// The entrypoint must locate diagnostics and legacy tool paths before Vapor's
// service or installer is available. Keep this small bootstrap path independent.
fn user_data_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("AppData/Local")))
            .map(|root| root.join("Vapor"))
    }
    #[cfg(target_os = "macos")]
    {
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("Library/Application Support/Vapor"))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        env::var_os("XDG_DATA_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
            .map(|root| root.join("vapor"))
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
