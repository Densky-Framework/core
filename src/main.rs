use densky_core::{http::http_discover, utils::join_paths, CompileContext};

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
    let example_server = join_paths(rel_path, path.display().to_string()).unwrap();
    let _http_tree = http_discover(CompileContext {
        output_dir: join_paths(".densky", example_server.clone()).unwrap_or_else(|err| {
            println!("{:#?}", err);
            String::from("PATH")
        }),
        routes_path: join_paths("src/routes", example_server.clone()).unwrap(),
        views_path: join_paths("src/views", example_server.clone()).unwrap(),
        static_path: join_paths("src/static", example_server.clone()).unwrap(),
        verbose: true,
        static_prefix: "static/".to_string(),
    });

    println!("{:#?}", _http_tree.map(|tree| tree.borrow().clone()));
}
