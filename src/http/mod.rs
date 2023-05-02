mod discover;
#[cfg(test)]
mod mod_test;
mod parser;
mod tree;

use std::cell::RefCell;
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
        match path.chars().nth(0) {
            Some(char) if char == '.' || char == '/' => {
                let absolute = if char == '.' {
                    let input_dirname = self.file_path.parent()?;
                    join_paths(path, input_dirname)
                } else {
                    path
                };

                let output_dirname = self.output_path.parent()?;

                relative_path(absolute, output_dirname).map(|path| path.display().to_string())
            }
            _ => Some(path),
        }
    }

    fn get_imports(&self, content: String) -> Result<(String, String), HttpParseError> {
        let content = RefCell::new(content);

        let mut imports: Vec<String> = vec![];

        loop {
            let mut content_mut = content.borrow_mut();
            let import_idx = match &content_mut.find("import") {
                Some(idx) => idx.clone(),
                None => break,
            };

            let content = &content_mut[(import_idx + "import ".len())..];
            let quote_idx = match &content.find("\"") {
                Some(idx) => idx.clone(),
                None => break,
            };

            let inner = if quote_idx < "  from ".len() {
                None
            } else {
                let from_idx = match &content.find("from") {
                    Some(idx) => idx.clone(),
                    None => {
                        return Err(HttpParseError::InvalidSyntax(
                            self.rel_path.clone(),
                            "Malformed import. Missing 'from' keyword".to_string(),
                        ))
                    }
                };
                Some(&content[..(from_idx - 1)])
            };

            let last_quote_idx = match &content.chars().skip(quote_idx + 1).position(|c| c == '"') {
                Some(idx) => idx.clone(),
                None => {
                    return Err(HttpParseError::InvalidSyntax(
                        self.rel_path.clone(),
                        "Malformed import. Missing closing quote.".to_string(),
                    ))
                }
            };

            let out_idx = quote_idx + last_quote_idx + 2;
            let path = &content[(quote_idx + 1)..(out_idx - 1)];
            let path = self.resolve_import(path).unwrap();
            let import_statement = if let Some(inner) = inner {
                format!("import {} from \"{}\"", inner, path)
            } else {
                format!("import \"{}\"", path)
            };
            let content = &content[(out_idx)..];
            *content_mut = content.to_string();

            imports.push(import_statement);
        }

        let content = content.borrow();
        Ok((imports.join(";\n"), content.to_string()))
    }

    fn get_handlers(&self, content: String) -> Result<(String, String), HttpParseError> {
        let (handlers, content) = match http_parse(content, self.file_path.display().to_string()) {
            Ok(h) => h,
            Err(e) => return Err(e),
        };
        let handlers: Vec<String> = handlers
            .borrow()
            .iter()
            .map(|handler| {
                let if_condition = match &handler.method {
                    &HTTPMethod::ANY => None,
                    &HTTPMethod::GET => Some("GET"),
                    &HTTPMethod::POST => Some("POST"),
                    &HTTPMethod::PATCH => Some("PATCH"),
                    &HTTPMethod::DELETE => Some("DELETE"),
                    &HTTPMethod::OPTIONS => Some("OPTIONS"),
                };

                if let Some(if_condition) = if_condition {
                    format!(
                        "if ({}.method == \"{}\") {{\n        {}\n      }}",
                        REQ_PARAM, if_condition, &handler.body
                    )
                } else {
                    handler.body.to_string()
                }
            })
            .collect();
        Ok((handlers.join("\n"), content))
    }

    pub fn get_parts(&self) -> Result<(String, String, String), HttpParseError> {
        let content = match self.get_content() {
            Ok(c) => c,
            Err(_) => return Err(HttpParseError::Empty(self.rel_path.clone())),
        };

        let (imports, content) = self.get_imports(content)?;
        let (handlers, content) = self.get_handlers(content)?;

        return Ok((imports, handlers, content));
    }
}
