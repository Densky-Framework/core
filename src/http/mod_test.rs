use super::{HttpLeaf, HttpTree};

#[test]
fn separing_by_parts() {
    let path = "a/b/c/d".to_string();
    let by_parts: Vec<_> = path.split('/').collect();

    assert_eq!(by_parts.as_slice(), &["a", "b", "c", "d"]);

    let path = "/a/b/c/d/".to_string();
    let by_parts: Vec<_> = path.split('/').collect();

    assert_eq!(by_parts.as_slice(), &["", "a", "b", "c", "d", ""]);
}

#[test]
fn get_common_path() {
    let tree_1 = HttpTree::new_leaf(HttpLeaf {
        path: "".to_string(),
        rel_path: "a/b/c".to_string(),
        file_path: "".into(),
        output_path: "".into(),
        content: None,
    });
    let tree_2 = HttpTree::new_leaf(HttpLeaf {
        path: "".to_string(),
        rel_path: "a/b/d".to_string(),
        file_path: "".into(),
        output_path: "".into(),
        content: None,
    });

    assert_eq!(
        tree_1.get_common_path(tree_2.rel_path),
        Some("a/b".to_string())
    );
}

#[test]
fn resolve_import() {
    let leaf = HttpLeaf {
        path: "".to_string(),
        rel_path: "".to_string(),
        file_path: "/project/path/routes/file1.ts".into(),
        output_path: "/project/path/.densky/http/file1.ts".into(),
        content: None,
    };

    assert_eq!(
        leaf.resolve_import("../utils/foo.ts"),
        Some("../../utils/foo.ts".to_string())
    );
    assert_eq!(leaf.resolve_import("module"), Some("module".to_string()));
}

#[test]
fn get_import() {
    let leaf = HttpLeaf {
        path: "".to_string(),
        rel_path: "".to_string(),
        file_path: "/project/path/routes/file1.ts".into(),
        output_path: "/project/path/.densky/http/file1.ts".into(),
        content: Some(
            "
import toString from \"module-a\";
import { a, b } from \"../utils/foo.ts\";
import \"./side.ts\"

function get_add() {
    return toString(a + b);
}
"
            .to_string(),
        ),
    };

    assert_eq!(leaf.get_imports(), Some(("import toString from \"module-a\";\nimport { a, b } from \"../../utils/foo.ts\";\nimport \"../../routes/side.ts\"".to_string(), "\n\nfunction get_add() {\n    return toString(a + b);\n}\n".to_string())));
}
