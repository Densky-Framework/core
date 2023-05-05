use densky_core::{
    utils::{join_paths, Fmt},
    walker::walker_tree_discover,
    CompileContext,
};

// fn process_entry(http_tree: &Rc<RefCell<HttpTree>>) {
//     let http_tree = http_tree.borrow();
//
//     if let Some(fallback) = &http_tree.fallback {
//         process_entry(fallback);
//     }
//     if let Some(middleware) = &http_tree.middleware {
//         process_entry(middleware);
//     }
//
//     for child in http_tree.children.iter() {
//         process_entry(child);
//     }
//
//     let output = match http_tree.generate_file() {
//         Ok(o) => o,
//         Err(e) => panic!("{:?}", e),
//     };
//     let output_path = &http_tree.output_path;
//     let _ = fs::create_dir_all(output_path.parent().unwrap());
//     fs::write(output_path, output).unwrap();
// }

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
    let (container, http_tree) = walker_tree_discover(
        "http",
        join_paths("src/routes", &example_server),
        CompileContext {
            output_dir: join_paths(".densky", &example_server),
            routes_path: join_paths("src/routes", &example_server),
            views_path: join_paths("src/views", &example_server),
            static_path: join_paths("src/static", &example_server),
            verbose: true,
            static_prefix: "static/".to_string(),
        },
    )
    .unwrap();

    // process_entry(&http_tree.unwrap());

    let http_tree = http_tree.lock().unwrap();
    println!("{}", Fmt(|f| http_tree.display(f, &container)));
    // println!("{}", http_tree);
    // println!("{}", http_tree.generate_file().unwrap())
    // if let Some(http_leaf) = &http_tree.leaf {
    //     let http_leaf = http_leaf.borrow();
    //     println!("{:?}", http_leaf.generate_file());
    // }
}
