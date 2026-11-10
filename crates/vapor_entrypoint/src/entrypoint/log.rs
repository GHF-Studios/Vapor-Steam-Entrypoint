struct EntryLog {
    path: PathBuf,
    file: Option<File>,
}

impl EntryLog {
    fn open(app_root: &Path) -> Self {
        let home = std::env::var_os("VAPOR_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                #[cfg(windows)]
                let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
                #[cfg(not(windows))]
                let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
                home.map(PathBuf::from).map(|home| home.join("Vapor"))
            })
            .unwrap_or_else(|| app_root.to_path_buf());
        let directory = home.join("data/logs");
        let path = directory.join("entrypoint.log");
        let file = fs::create_dir_all(&directory).ok().and_then(|_| {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .ok()
        });
        Self { path, file }
    }

    fn stderr(&self) -> Option<File> {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .ok()
    }

    fn write(&mut self, message: impl AsRef<str>) {
        if let Some(file) = &mut self.file {
            let _ = writeln!(file, "[{:?}] {}", SystemTime::now(), message.as_ref());
        }
    }
}
