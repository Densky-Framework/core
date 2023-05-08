use std::{
    fs,
    sync::{Arc, Mutex},
};

use densky_core::{
    http::{HttpLeaf, HttpTree},
    utils::{join_paths, Fmt},
    views::ViewLeaf,
    walker::{
        container::WalkerContainer,
        discover::{simple_discover, walker_tree_discover},
        WalkerLeaf, WalkerTree,
    },
    CompileContext,
};

fn process_leaf(http_leaf: Arc<Mutex<WalkerLeaf>>) {
    let http_tree = http_leaf.lock().unwrap();
    let output = match HttpLeaf::generate_file(&http_tree) {
        Ok(o) => o,
        Err(e) => panic!("{:?}", e),
    };
    let output_path = &http_tree.output_path;
    println!("{}", output_path.display());
    let _ = fs::create_dir_all(output_path.parent().unwrap());
    fs::write(output_path, output).unwrap();
}

fn process_entry(http_tree: Arc<Mutex<WalkerTree>>, container: &mut WalkerContainer) {
    let mut http_tree = http_tree.lock().unwrap();

    let output = match HttpTree::generate_file(&mut http_tree, container) {
        Ok(o) => o,
        Err(e) => panic!("{:?}", e),
    };
    let output_path = &http_tree.output_path;
    println!("{}", output_path.display());
    let _ = fs::create_dir_all(output_path.parent().unwrap());
    fs::write(output_path, output).unwrap();

    let children = http_tree.children.clone();

    if let Some(fallback) = &http_tree.fallback {
        let fallback = container.get_leaf(*fallback).unwrap();
        process_leaf(fallback);
    }
    if let Some(middleware) = &http_tree.middleware {
        let middleware = container.get_leaf(*middleware).unwrap();
        process_leaf(middleware);
    }

    drop(http_tree);

    for child in children.iter() {
        process_entry(container.get_tree(*child).unwrap(), container);
    }
}

fn process_view(view: ViewLeaf) -> Option<()> {
    let output = view
        .generate_file()
        .map(|c| prettify_js::prettyprint(&c.to_owned()).0)?;

    let output_path = view.output_path();
    println!("{}", output_path.display());
    let _ = fs::create_dir_all(output_path.parent().unwrap());
    fs::write(output_path, output).unwrap();

    Some(())
}

fn main() {
    let path = std::env::current_dir().unwrap();
    let mut rel_path = std::env::args();
    let rel_path = match rel_path.nth(1) {
        None => panic!("Provide a server path"),
        Some(path) => {
            if path.len() == 0 {
                panic!("Provide a server path")
            } else {
                path
            }
        }
    };
    let example_server = join_paths(rel_path, path);

    let compile_context = CompileContext {
        output_dir: join_paths(".densky", &example_server),
        routes_path: join_paths("src/routes", &example_server),
        views_path: join_paths("src/views", &example_server),
        static_path: join_paths("src/static", &example_server),
        verbose: true,
        static_prefix: "static/".to_owned(),
    };

    let views = simple_discover(
        "views",
        compile_context.views_path.clone(),
        &compile_context,
    )
    .filter_map(|a| a)
    .map(ViewLeaf::from)
    .map(process_view);

    for _ in views {}

    let (mut container, http_tree) = walker_tree_discover(
        "http",
        compile_context.routes_path.clone(),
        &compile_context,
    )
    .unwrap();

    process_entry(http_tree, &mut container);

    // let http_tree = http_tree.lock().unwrap();
    // println!("{}", Fmt(|f| http_tree.display(f, &container)));
    // println!("{}", http_tree);
    // println!("{}", http_tree.generate_file().unwrap())
    // if let Some(http_leaf) = &http_tree.leaf {
    //     let http_leaf = http_leaf.borrow();
    //     println!("{:?}", http_leaf.generate_file());
    // }
}
