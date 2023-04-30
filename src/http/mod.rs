mod discover;
#[cfg(test)]
mod mod_test;
mod parser;
mod tree;

use std::io;
use std::str::FromStr;
use std::{fs, path::PathBuf};

pub use discover::*;
pub use parser::*;
pub use tree::*;

use crate::utils::{join_paths, relative_path};

/// Http endpoint node
#[derive(Debug, Clone)]
pub struct HttpLeaf {
    /// The absolute path (url) for this leaf
    pub path: String,
    /// The path (url) relative to parent.
    pub rel_path: String,

    /// The path (fs) to the current file
    pub file_path: PathBuf,
    /// The path (fs) to the output file
    pub output_path: PathBuf,

    pub content: Option<String>,
}

impl HttpLeaf {
    pub fn new(path: String, file_path: String, output_path: String) -> HttpLeaf {
        HttpLeaf {
            path,
            rel_path: String::new(),
            file_path: PathBuf::from_str(file_path.as_str()).unwrap(),
            output_path: PathBuf::from_str(output_path.as_str()).unwrap(),
            content: None,
        }
    }

    pub fn cache_content(&mut self) -> io::Result<()> {
        let content = fs::read_to_string(&self.file_path)?;
        self.content = Some(content);
        Ok(())
    }

    pub fn get_content(&self) -> io::Result<String> {
        if let Some(content) = &self.content {
            Ok(content.to_string())
        } else {
            fs::read_to_string(&self.file_path)
        }
    }

    pub fn resolve_import<P: AsRef<str>>(&self, path: P) -> Option<String> {
        let path = path.as_ref().to_string();
        if path.chars().nth(0) == Some('.') {
            let input_dirname = self.file_path.parent()?;
            let output_dirname = self.output_path.parent()?;

            let absolute = join_paths::<String, String>(path, input_dirname.display().to_string());
            relative_path(absolute, output_dirname.display().to_string())
                .map(|path| path.display().to_string())
        } else {
            Some(path)
        }
    }

    pub fn get_imports(&self) {}
}
