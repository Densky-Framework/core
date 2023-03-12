mod discover;
#[cfg(test)]
mod mod_test;
mod parser;
mod tree;

use std::path::PathBuf;
use std::str::FromStr;

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
}

impl HttpLeaf {
    pub fn new(path: String, file_path: String, output_path: String) -> HttpLeaf {
        HttpLeaf {
            path,
            rel_path: String::new(),
            file_path: PathBuf::from_str(file_path.as_str()).unwrap(),
            output_path: PathBuf::from_str(output_path.as_str()).unwrap(),
        }
    }

    fn resolve_import<P: Into<String>>(self, path: String) -> Option<String> {
        let path: String = path.into();
        if path.chars().nth(0) == Some('.') {
            let input_dirname = self.file_path.parent()?;
            let output_dirname = self.output_path.parent()?;

            let relative =
                match join_paths::<String, String>(input_dirname.display().to_string(), path) {
                    Ok(path) => path,
                    Err(_) => return None,
                };
            relative_path(relative, output_dirname.display().to_string())
                .map(|path| path.display().to_string())
        } else {
            Some(path)
        }
    }
}
