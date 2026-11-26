fn discover_product_root(executable: &Path) -> Option<PathBuf> {
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

fn user_data_root() -> Option<PathBuf> {
    vapor_product_layout::user_data_root()
}

#[cfg(windows)]
fn shell_safe_product_root(path: PathBuf) -> PathBuf {
    strip_windows_verbatim_prefix(&path)
}

#[cfg(not(windows))]
fn shell_safe_product_root(path: PathBuf) -> PathBuf {
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
