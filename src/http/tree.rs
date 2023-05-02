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

    pub fn set_rel_path(&mut self, rel_path: String) {
        self.rel_path = rel_path.clone();
        match &self.leaf {
            Some(leaf) => {
                let mut leaf = leaf.borrow_mut();
                leaf.rel_path = rel_path;
            }
            None => {}
        }
    }

    pub fn is_convention(&self) -> bool {
        let last_part = PathBuf::from_str(&self.path).unwrap();
        let last_part = last_part.iter().nth_back(0).unwrap();
        let last_part = last_part.to_str().unwrap().to_string();

        // Ignore all routes that starts with '_'
        last_part == "_fallback" || last_part == "_middleware"
    }

    /// Verify if the path is direct child of `self` and
    /// also if ends with the provided pattern
    fn ends_with(&self, path: &String, pattern: &str) -> bool {
        let slash_len = if self.is_root { 0 } else { 1 };
        path.len() == self.path.len() + slash_len + pattern.len() && path.ends_with(pattern)
    }

    /// Add the child to the tree. For that exists many ways:
    /// - *fallback* (`*/_fallback`): The file is used as fallback of this route and children.
    /// - *middleware* (`*/_middleware`): Same as fallback but with the middleware
    /// - *index* (`*/_index`): Move route to be the leaf of its parent.
    /// - *Any other*: Pass through an algoritnm to decide other many ways:
    ///   + *Merge*: If two routes share some segment on the `rel_path` then make
    ///            a new tree with that segment as `rel_path` and make it as container.
    ///            Both routes are moved in to that container.
    ///   + *Pull*: If two route share some segment on the `rel_path` and the route that already
    ///           exists is a container then move the child to that container.
    ///   + *Index*: This is just for any `_index` that doesn't have a container slibing, create
    ///            a tree as container and `rel_path` equal to child owner (`rel_path` - `_index`).
    ///            Move the child the created container.
    ///   + *Any other*: Just add it as child.
    pub fn add_child(self_: Rc<RefCell<Self>>, child: &mut HttpTree, output_path: &String) {
        let this = self_.clone();
        let mut this = this.borrow_mut();
        child.parent = Some(self_.clone());

        let path = child.path.clone();
        if this.ends_with(&path, "_fallback") {
            child.set_rel_path("<FALLBACK>".to_string());
            child.is_fallback = true;
            this.fallback = Some(Rc::new(RefCell::new(child.clone())));
        } else if this.ends_with(&path, "_middleware") {
            child.set_rel_path("<MIDDLEWARE>".to_string());
            child.is_middleware = true;
            this.middleware = Some(Rc::new(RefCell::new(child.clone())));
        } else if this.ends_with(&path, "_index") {
            child.set_rel_path(this.rel_path.clone());
            this.leaf = child.leaf.clone();
            if let Some(leaf) = &child.leaf {
                this.output_path = leaf.borrow().output_path.clone();
            }
        } else {
            let last_part = PathBuf::from_str(&path).unwrap();
            let prefix_part = match last_part.parent() {
                Some(expr) => expr,
                None => return,
            };
            let prefix_part = prefix_part.display().to_string();
            let last_part = last_part.iter().nth_back(0).unwrap();
            let last_part = last_part.to_str().unwrap();
            let is_index = last_part == "_index";

            // Ignore all routes that starts with '_'
            if last_part.starts_with('_')
                && !is_index
                && last_part != "_fallback"
                && last_part != "_middleware"
            {
                return;
            }

            // Update relative path and fix any '/' at start
            let rel_path = &path[this.path.len()..];
            let rel_path = if rel_path.starts_with('/') {
                &rel_path[1..]
            } else {
                rel_path
            };
            child.set_rel_path(rel_path.to_string());

            // When the leaf has a common path with other leaf
            // then make a common branch for both or merge on
            // the index.
            // From:
            // /convention/some-route
            // /convention/with-index
            // /convention/with-index/index-child
            //
            // To:
            // /
            // | /convention
            // | | /some-route
            // | | /with-index *
            // | | | /index-child
            //
            // Steps:
            // - Make for both:
            //   /convention/some-route
            //   /convention/with-index
            //   To:
            //   /convention
            //   | /some-route
            //   | /with-index *
            //
            // - Merge:
            //   /convention
            //   /convention/with-index/index-child
            //   To:
            //   /convention
            //   | /some-route
            //   | /with-index *
            //   | /with-index/index-child
            //
            // - Repeat
            //

            let common_path = this.children.iter().find_map(|child| {
                child
                    .borrow()
                    .get_common_path(rel_path.to_string())
                    .map(|common_path| (child, common_path))
            });

            let leaf = if let Some((common_child_, common_path)) = common_path {
                let common_child = common_child_.as_ref().borrow();
                let common_child_path = common_child.path.clone();

                let is_container = common_child.is_container;
                let is_container = is_container && path.starts_with(&common_child_path);

                // If is container, then insert the new child to it
                if is_container {
                    child.rel_path = (&path[common_child.path.len()..]).to_string();
                    drop(common_child);
                    HttpTree::add_child(common_child_.clone(), child, output_path);
                    None
                } else {
                    // else, then merge into one common container
                    drop(common_child);
                    let common_child_id = common_child_.borrow().get_id();
                    let path = if this.path.as_str() == "/" {
                        format!("/{}", common_path)
                    } else {
                        format!("{}/{}", &this.path, common_path)
                    };
                    let output = join_paths("_index.ts", join_paths(&path[1..], output_path));
                    let parent = HttpTree {
                        path,
                        rel_path: common_path,
                        output_path: output.into(),
                        is_container: true,
                        parent: Some(self_.clone()),
                        ..Default::default()
                    };
                    let parent = Rc::new(RefCell::new(parent));
                    HttpTree::add_child(
                        parent.clone(),
                        &mut common_child_.borrow_mut(),
                        output_path,
                    );
                    HttpTree::add_child(parent.clone(), child, output_path);
                    Some((parent.clone(), Some(common_child_id)))
                }
            } else if is_index {
                // If try to put an _index without sliblings, then create a
                // container for it and use the child as leaf
                let rel_path = PathBuf::from_str(&child.rel_path).unwrap();
                let rel_path = rel_path.parent().unwrap().to_str().unwrap().to_string();

                // Update the rel_path for leaf
                child.set_rel_path(rel_path.clone());

                let parent = HttpTree {
                    path: prefix_part,
                    rel_path: rel_path.clone(),
                    is_container: true,
                    output_path: if let Some(leaf) = &child.leaf {
                        leaf.borrow().output_path.clone()
                    } else {
                        "/".into()
                    },
                    leaf: child.leaf.clone(),
                    parent: Some(self_.clone()),
                    ..Default::default()
                };
                Some((Rc::new(RefCell::new(parent)), None))
            } else {
                // If there's no common slibling, then put it inside
                Some((Rc::new(RefCell::new(child.clone())), None))
            };

            // This is for borrowing errors, all are computed on the above
            // block and the actions are executed here.
            if let Some((leaf, remove_id)) = leaf {
                if let Some(remove_id) = remove_id {
                    this.children
                        .retain(|child| child.borrow().get_id() != remove_id.clone());
                }
                this.children.push(leaf)
            }
        }
    }

    /// Get the shared path between two branchs.
    /// Eg.
    /// ```rust
    /// use densky_core::http::HttpTree;
    ///
    /// let branch_1 = HttpTree {
    ///     rel_path: "a/b/c/and/more".to_string(),
    ///     ..Default::default()
    /// };
    ///
    /// // Just need the relative path
    /// let branch_2 = "a/b/some/other".to_string();
    ///
    /// let common_path = branch_1.get_common_path(branch_2).unwrap();
    ///
    /// assert_eq!(common_path, "a/b".to_string());
    /// ```
    pub fn get_common_path(&self, other_path: String) -> Option<String> {
        // All segments of the path: a/b/c -> vec!["a", "b", "c"]
        let by_segments: Vec<_> = other_path.split('/').collect();
        // The accumulator of common path
        let mut carrier = "".to_string();

        for segment in by_segments {
            // Prevent wrong paths like "a//b/c", "/a/b/c" or "a/b/c/"
            if segment.len() == 0 {
                return None;
            }

            let is_first = carrier.as_str() == "";
            // The expected path
            let next = if is_first {
                segment.to_string()
            } else {
                format!("{}/{}", &carrier, &segment)
            };

            if !self.rel_path.starts_with(&next) {
                if is_first {
                    return None;
                } else {
                    return Some(carrier);
                }
            }

            if !is_first {
                carrier.push_str("/");
            }
            carrier.push_str(segment);
        }

        return Some(other_path);
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

        let empty_string = String::new();
        let leaf_imports = leaf_parts.as_ref().map_or(&empty_string, |parts| &parts.0);
        let leaf_content = leaf_parts.as_ref().map_or(&empty_string, |parts| &parts.2);
        let leaf_handlers = leaf_parts.as_ref().map_or(&empty_string, |parts| &parts.1);
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
        let imports = format!(
            "import $_Densky_Runtime_$ from \"densky/runtime\";\n{}\n{}{}",
            fallback_import, children_import, leaf_imports
        );

        let fallback_handler = if self.fallback.is_some() {
            ";return $__fallback__$(__req_param__);"
        } else {
            ""
        };

        let top_content = format!(
            "{imports}
{serial}
{content}",
            imports = imports,
            content = leaf_content,
            serial = url_matcher.serial_decl(),
        );

        let children_content: Vec<String> = (0..self.children.len())
            .map(|index| {
                format!(
                    ";{{
      const _ = $__child__${}({});
      if (_) return _;
    }}",
                    index, REQ_PARAM
                )
            })
            .collect();
        let children_content = children_content.join("\n");

        let handler_content = format!(
            "if ({exact}) {{
      {handlers}
      {fallback_handler}
    }}
    {children}
",
            handlers = leaf_handlers,
            exact = url_matcher.exact_decl(REQ_PARAM),
            fallback_handler = fallback_handler,
            children = children_content
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
  }}
",
                inner_content = handler_content,
                start = url_matcher.start_decl(REQ_PARAM),
                update = url_matcher.update_decl(REQ_PARAM),
            )
        };

        return Ok(format!(
            "{top_content}
;export default function(__req_param__) {{
  {inner_content}
}}",
            top_content = top_content,
            inner_content = inner_content,
        ));
    }
}
