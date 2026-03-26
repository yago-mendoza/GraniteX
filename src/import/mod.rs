mod stl;
mod obj;

use std::path::Path;
use crate::renderer::Mesh;

#[derive(thiserror::Error, Debug)]
pub enum ImportError {
    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
}

/// Load a mesh from a file, dispatching by extension.
pub fn load_file(path: &Path) -> Result<Mesh, ImportError> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "stl" => stl::load(path),
        "obj" => obj::load(path),
        _ => Err(ImportError::UnsupportedFormat(ext)),
    }
}

/// File filter extensions for the open dialog.
pub fn supported_extensions() -> &'static [&'static str] {
    &["stl", "obj"]
}
