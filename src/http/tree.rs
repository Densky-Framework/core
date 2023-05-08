use std::path::Path;
use std::sync::MutexGuard;

use crate::utils::{relative_path, UrlMatcher};
use crate::walker::container::WalkerContainer;
use crate::walker::WalkerTree;

use super::{HttpLeaf, HttpParseError, REQ_PARAM};

pub struct HttpTree;

impl HttpTree {
    pub fn resolve_import<P: AsRef<Path>>(
        this: &MutexGuard<'_, WalkerTree>,
        path: P,
    ) -> Option<String> {
        let path = path.as_ref().display().to_string();
        match path.chars().nth(0) {
            Some('/') => {
                let output_dirname = this.output_path.parent()?;

                relative_path(path, output_dirname).map(|path| path.display().to_string())
            }
            _ => Some(path),
        }
    }

    pub fn generate_file(
        this: &mut MutexGuard<'_, WalkerTree>,
        container: &mut WalkerContainer,
    ) -> Result<String, HttpParseError> {
        let url_matcher = UrlMatcher::new(this.rel_path.to_owned());
        let leaf_parts = this
            .leaf
            .as_ref()
            .map(|&leaf| HttpLeaf::get_parts(&container.get_leaf_locked(leaf).unwrap()));
        let leaf_parts = if let Some(parts) = leaf_parts {
            match parts {
                Ok(expr) => Some(expr),
                Err(e) => return Err(e.clone()),
            }
        } else {
            None
        };

        let middlewares = if this.is_middleware {
            vec![]
        } else {
            this.get_middlewares(container)
        };

        let empty_string = String::new();
        let leaf_imports = leaf_parts.as_ref().map_or(&empty_string, |parts| &parts.0);
        let leaf_content = leaf_parts.as_ref().map_or(&empty_string, |parts| &parts.2);
        let leaf_handlers = leaf_parts.as_ref().map_or(&empty_string, |parts| &parts.1);
        let is_empty_handlers = String::is_empty(&leaf_handlers);
        let fallback_import = this.fallback.as_ref().map_or_else(
            || String::new(),
            |&fallback| {
                format!(
                    "import $__fallback__$ from \"{}\";",
                    Self::resolve_import(
                        this,
                        &container.get_leaf_locked(fallback).unwrap().output_path
                    )
                    .unwrap()
                )
            },
        );
        let children_import: Vec<String> = this
            .children
            .iter()
            .enumerate()
            .map(|(index, &child)| {
                format!(
                    "import $__child__${} from \"{}\";",
                    index,
                    Self::resolve_import(
                        this,
                        &container.get_tree_locked(child).unwrap().output_path
                    )
                    .unwrap()
                )
            })
            .collect();
        let children_import = children_import.join("\n");
        let middlewares_import = if this.is_middleware {
            String::new()
        } else {
            middlewares
                .iter()
                .enumerate()
                .map(|(index, &middleware)| {
                    format!(
                        "import $__middleware__${} from \"{}\";",
                        index,
                        Self::resolve_import(
                            this,
                            &container.get_leaf_locked(middleware).unwrap().output_path
                        )
                        .unwrap()
                    )
                })
                .collect::<Vec<String>>()
                .join("\n")
        };
        let imports = format!(
            "import $_Densky_Runtime_$ from \"densky/runtime\";\n{}\n{}\n{}{}",
            fallback_import, children_import, middlewares_import, leaf_imports
        );

        let middlewares_handlers = if this.is_middleware {
            String::new()
        } else {
            middlewares
                .iter()
                .enumerate()
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

        let fallback_handler = if this.fallback.is_some() {
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

        let children_content: Vec<String> = (0..this.children.len())
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

        let inner_content = if this.is_root {
            handler_content
        } else if this.is_fallback {
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
