struct EntryLog {
    path: PathBuf,
    file: Option<File>,
}

impl EntryLog {
    fn open(vapor_home: &Path) -> Self {
        let directory = vapor_home.join("data/logs");
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
