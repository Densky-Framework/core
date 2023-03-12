use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use super::HttpLeaf;

#[derive(Clone)]
pub struct HttpTree {
    /// The absolute path (url) for this leaf
    pub path: String,
    /// The path (url) relative to parent.
    pub rel_path: String,
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
                leaf.file_path.display().to_string()
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

    /// Verify if the path is direct child of `self` and
    /// also if ends with the provided pattern
    fn ends_with(&self, path: &String, pattern: &str) -> bool {
        let slash_len = if self.is_root { 0 } else { 1 };
        path.len() == self.path.len() + slash_len + pattern.len() && path.ends_with(pattern)
    }

    /// Add the child to the tree. For that exists many ways:
    /// - *fallback* (`*/_fallback`): The file is used as fallback of this route and children.
    /// - *middleware* (`*/_middleware`): Same as fallback but with the middleware
    /// - *index* (`*/_index`): Move route to be the leaf of them parent.
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
    pub fn add_child(self_: Rc<RefCell<Self>>, child: &mut HttpTree) {
        let this = self_.clone();
        let mut this = this.borrow_mut();
        child.parent = Some(self_.clone());

        let path = child.path.clone();
        if this.ends_with(&path, "_fallback") {
            child.set_rel_path("<FALLBACK>".to_string());
            this.fallback = Some(Rc::new(RefCell::new(child.clone())));
        } else if this.ends_with(&path, "_middleware") {
            child.set_rel_path("<MIDDLEWARE>".to_string());
            this.middleware = Some(Rc::new(RefCell::new(child.clone())));
        } else if this.ends_with(&path, "_index") {
            let child_ = child.leaf.clone();
            if let Some(child) = child_ {
                child.borrow_mut().path = this.path.clone();
                child.borrow_mut().rel_path = this.rel_path.clone();

                this.leaf = Some(child);
            }
            drop(child);
        } else {
            let last_part = path.split('/').last().unwrap();
            let is_ignored = last_part.starts_with('_');
            let is_index = last_part == "_index";
            let is_convention = last_part == "_fallback" || last_part == "_middleware" || is_index;

            // Ignore all routes that starts with '_'
            if is_ignored && !is_convention {
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
                if is_container {
                    child.rel_path = (&path[common_child.path.len()..]).to_string();
                    drop(common_child);
                    HttpTree::add_child(common_child_.clone(), child);
                    None
                } else {
                    drop(common_child);
                    let common_child_id = common_child_.borrow().get_id();
                    let parent = HttpTree {
                        path: if this.path.as_str() == "/" {
                            format!("/{}", common_path)
                        } else {
                            format!("{}/{}", &this.path, common_path)
                        },
                        rel_path: common_path,
                        is_container: true,
                        ..Default::default()
                    };
                    let parent = Rc::new(RefCell::new(parent));
                    HttpTree::add_child(parent.clone(), &mut common_child_.borrow_mut());
                    HttpTree::add_child(parent.clone(), child);
                    Some((parent.clone(), Some(common_child_id)))
                }
            } else if is_index {
                let path = &child.path[0..(child.path.len() - last_part.len() - 1)];
                let common_path = &path[(this.path.len() + 1)..];
                let parent = HttpTree {
                    path: path.to_string(),
                    rel_path: common_path.to_string(),
                    is_container: true,
                    ..Default::default()
                };
                let parent = Rc::new(RefCell::new(parent));
                HttpTree::add_child(parent.clone(), child);
                Some((parent.clone(), None))
            } else {
                Some((Rc::new(RefCell::new(child.clone())), None))
            };

            // This is for borrowing errors, all are computed on the above
            // block and here the actions are executed.
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
}
