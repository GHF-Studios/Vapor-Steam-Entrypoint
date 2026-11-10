struct EntryLog {
    path: PathBuf,
    file: Option<File>,
}

impl EntryLog {
    fn open() -> Self {
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
            // Supported platforms provide a user-home environment variable.
            // If one is unavailable, fail toward temporary storage rather than
            // writing into the Steam product directory.
            .unwrap_or_else(std::env::temp_dir);
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
