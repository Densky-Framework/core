use colored::*;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;
use std::{cell::RefCell, path::Path};

use crate::utils::{join_paths, relative_path, UrlMatcher};

use super::{HttpLeaf, HttpParseError, REQ_PARAM};

#[derive(Clone)]
pub struct HttpTree {
    /// The absolute path (url) for this leaf
    pub path: String,
    /// The path (url) relative to parent.
    pub rel_path: String,
    pub output_path: PathBuf,

    pub children: Vec<Rc<RefCell<HttpTree>>>,
    pub leaf: Option<Rc<RefCell<HttpLeaf>>>,
    pub middleware: Option<Rc<RefCell<HttpTree>>>,
    pub fallback: Option<Rc<RefCell<HttpTree>>>,
    pub parent: Option<Rc<RefCell<HttpTree>>>,

    pub has_index: bool,
    pub is_container: bool,
    pub is_root: bool,
    pub is_fallback: bool,
    pub is_middleware: bool,
}

impl fmt::Display for HttpTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let has_leaf = self.leaf.is_some();
        let name = format!(
            "{} {}",
            match (self.is_root, has_leaf) {
                (true, true) => "★".yellow(),
                (true, false) => "☆".bright_yellow(),
                (false, true) => "▲".yellow(),
                (false, false) => "△".bright_yellow(),
            },
            &self.rel_path.bold()
        );
        f.write_str(&name)?;

        if self.middleware.is_some() {
            f.write_str(&format!(
                "\n{} ■ {}",
                "|".dimmed().bright_black(),
                "middleware".bright_black()
            ))?;
        }

        for child in &self.children {
            let child = child.borrow();
            let fmtd = format!("{}", child);

            for line in fmtd.split("\n") {
                write!(f, "\n{} {}", "|".dimmed().bright_black(), line)?;
            }
        }
        if self.fallback.is_some() {
            f.write_str(&format!(
                "\n{} {}",
                "|".dimmed().bright_black(),
                "...fallback".bright_black()
            ))?;
        }

        Ok(())
    }
}

impl fmt::Debug for HttpTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = format!(
            "{}HttpTree{}",
            if self.is_root { "ROOT - " } else { "" },
            if self.is_container {
                " - CONTAINER"
            } else {
                ""
            }
        );
        let leaf = if self.leaf.is_some() {
            let leaf = self.leaf.as_ref().unwrap().borrow();

            format!(
                "Some(<Leaf ({}|{}) {}>)",
                leaf.path,
                leaf.rel_path,
                leaf.file_path.display()
            )
        } else {
            "None".to_string()
        };
        let fallback = if self.fallback.is_some() {
            let fallback = self.fallback.as_ref().unwrap().borrow();

            Some(fallback)
        } else {
            None
        };
        let mut children = vec![];
        for child in &self.children {
            children.push(child.borrow());
        }
        f.debug_struct(name.as_str())
            .field("path", &self.path)
            .field("rel_path", &self.rel_path)
            .field("output_path", &self.output_path)
            .field("children", &children)
            .field("leaf", &format_args!("{}", leaf))
            .field("middleware", &self.middleware)
            .field("fallback", &fallback)
            .finish()
    }
}

impl Default for HttpTree {
    fn default() -> Self {
        HttpTree {
            path: "/".to_string(),
            rel_path: "/".to_string(),
            output_path: "/".into(),
            children: vec![],
            leaf: None,
            middleware: None,
            fallback: None,
            parent: None,
            has_index: false,
            is_container: false,
            is_root: false,
            is_fallback: false,
            is_middleware: false,
        }
    }
}

impl HttpTree {
    pub fn new() -> HttpTree {
        HttpTree {
            is_root: false,
            ..Default::default()
        }
    }

    pub fn new_leaf(leaf: HttpLeaf) -> HttpTree {
        HttpTree {
            path: leaf.path.clone(),
            rel_path: leaf.rel_path.clone(),
            output_path: leaf.output_path.clone(),
            leaf: Some(Rc::new(RefCell::new(leaf))),
            ..Default::default()
        }
    }

    pub fn get_id(&self) -> String {
        self.path.clone()
    }

    pub fn get_middlewares(&self) -> Vec<(usize, String)> {
        let mut middlewares: Vec<(usize, String)> = vec![];

        if let Some(parent) = &self.parent {
            for middleware in parent.borrow().get_middlewares() {
                middlewares.push(middleware);
            }
        }

        if let Some(middleware) = &self.middleware {
            middlewares.push((
                middlewares.len(),
                middleware.borrow().output_path.display().to_string(),
            ));
        }

        middlewares
    }

    pub fn resolve_import<P: AsRef<Path>>(&self, path: P) -> Option<String> {
        let path = path.as_ref().display().to_string();
        match path.chars().nth(0) {
            Some('/') => {
                let output_dirname = self.output_path.parent()?;

                relative_path(path, output_dirname).map(|path| path.display().to_string())
            }
            _ => Some(path),
        }
    }

    pub fn generate_file(&self) -> Result<String, HttpParseError> {
        let url_matcher = UrlMatcher::new(self.rel_path.to_string());
        let leaf_parts = self.leaf.as_ref().map(|leaf| leaf.borrow().get_parts());
        let leaf_parts = if let Some(parts) = leaf_parts {
            match parts {
                Ok(expr) => Some(expr),
                Err(e) => return Err(e.clone()),
            }
        } else {
            None
        };

        let middlewares = if self.is_middleware {
            vec![]
        } else {
            self.get_middlewares()
        };

        let empty_string = String::new();
        let leaf_imports = leaf_parts.as_ref().map_or(&empty_string, |parts| &parts.0);
        let leaf_content = leaf_parts.as_ref().map_or(&empty_string, |parts| &parts.2);
        let leaf_handlers = leaf_parts.as_ref().map_or(&empty_string, |parts| &parts.1);
        let is_empty_handlers = String::is_empty(&leaf_handlers);
        let fallback_import = self.fallback.as_ref().map_or_else(
            || String::new(),
            |fallback| {
                format!(
                    "import $__fallback__$ from \"{}\";",
                    self.resolve_import(&fallback.borrow().output_path).unwrap()
                )
            },
        );
        let children_import: Vec<String> = self
            .children
            .iter()
            .enumerate()
            .map(|(index, child)| {
                format!(
                    "import $__child__${} from \"{}\";",
                    index,
                    self.resolve_import(&child.borrow().output_path).unwrap()
                )
            })
            .collect();
        let children_import = children_import.join("\n");
        let middlewares_import = if self.is_middleware {
            String::new()
        } else {
            middlewares
                .iter()
                .map(|(index, middleware)| {
                    format!(
                        "import $__middleware__${} from \"{}\";",
                        index,
                        self.resolve_import(middleware).unwrap()
                    )
                })
                .collect::<Vec<String>>()
                .join("\n")
        };
        let imports = format!(
            "import $_Densky_Runtime_$ from \"densky/runtime\";\n{}\n{}\n{}{}",
            fallback_import, children_import, middlewares_import, leaf_imports
        );

        let middlewares_handlers = if self.is_middleware {
            String::new()
        } else {
            middlewares
                .iter()
                .map(|(index, _)| {
                    format!(
                        "{{ 
                          let _ = $__middleware__${}(__req_param__); 
                          if (_) return _; 
                        }};",
                        index
                    )
                })
                .collect::<Vec<String>>()
                .join("\n")
        };

        let fallback_handler = if self.fallback.is_some() {
            "return $__fallback__$(__req_param__);"
        } else {
            ""
        };

        let top_content = format!(
            "{imports}\n{serial}\n{content}",
            imports = imports,
            content = leaf_content,
            serial = url_matcher.serial_decl(),
        );

        let children_content: Vec<String> = (0..self.children.len())
            .map(|index| {
                format!(
                    "{{ 
                      const _ = $__child__${}({}); 
                      if (_) return _; 
                    }};",
                    index, REQ_PARAM
                )
            })
            .collect();
        let children_content = children_content.join("\n");

        let handler_content = if is_empty_handlers {
            String::new()
        } else {
            format!(
                "if ({exact}) {{ 
              {middlewares} 
              {handlers} 
              ;return new Response(401); 
            }} ",
                middlewares = middlewares_handlers,
                handlers = leaf_handlers,
                exact = url_matcher.exact_decl(REQ_PARAM),
            )
        };

        let handler_content = format!(
            "{}\n{}\n{}",
            handler_content, children_content, fallback_handler
        );

        let inner_content = if self.is_root {
            handler_content
        } else if self.is_fallback {
            leaf_handlers.clone()
        } else {
            format!(
                "if ({start}) {{ 
                  {update} 
                  {inner_content} 
                }}",
                inner_content = handler_content,
                start = url_matcher.start_decl(REQ_PARAM),
                update = url_matcher.update_decl(REQ_PARAM),
            )
        };
        let (pretty, _) = prettify_js::prettyprint(
            format!(
                "{top_content}
;export default function(__req_param__) {{
  {inner_content}
}}",
                top_content = top_content,
                inner_content = inner_content,
            )
            .as_str(),
        );

        return Ok(pretty);
    }
}
