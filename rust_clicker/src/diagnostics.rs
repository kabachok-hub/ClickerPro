//! Small local event log: no images, no network, no per-frame writes.
use std::io::Write;

pub struct HunterLog(Option<std::fs::File>);

impl HunterLog {
    pub fn new() -> Self {
        let file = (|| -> std::io::Result<std::fs::File> {
            let root = std::env::var_os("LOCALAPPDATA").ok_or(std::io::ErrorKind::NotFound)?;
            let directory = std::path::PathBuf::from(root).join("ClickerPro");
            std::fs::create_dir_all(&directory)?;
            let path = directory.join("hunter.log");
            if std::fs::metadata(&path).map(|m| m.len() > 256 * 1024).unwrap_or(false) {
                std::fs::write(&path, b"")?;
            }
            std::fs::OpenOptions::new().create(true).append(true).open(path)
        })().ok();
        Self(file)
    }

    pub fn event(&mut self, message: &str) {
        if let Some(file) = &mut self.0 {
            if file.metadata().map(|m| m.len() > 256 * 1024).unwrap_or(false) { let _ = file.set_len(0); }
            let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
            let _ = writeln!(file, "{ms} {}", message.replace(['\r', '\n'], " "));
        }
    }
}
