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
    });
    let tree_2 = HttpTree::new_leaf(HttpLeaf {
        path: "".to_string(),
        rel_path: "a/b/d".to_string(),
        file_path: "".into(),
        output_path: "".into(),
    });

    assert_eq!(
        tree_1.get_common_path(tree_2.rel_path),
        Some("a/b".to_string())
    );
}
