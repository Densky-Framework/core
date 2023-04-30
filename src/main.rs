use std::fs;

use densky_core::{
    http::{http_discover, http_parse, HttpTree},
    utils::{join_paths, UrlMatcher},
    CompileContext,
};

fn process_entry(http_tree: &HttpTree) {
    if let Some(leaf) = &http_tree.leaf {
        let file_path = &leaf.borrow().file_path;
        let handler = match http_parse(
            fs::read_to_string(file_path.clone()).unwrap(),
            file_path.display().to_string(),
        ) {
            Ok(handler) => handler,
            Err(densky_core::http::HttpParseError::Empty(_)) => return,
            Err(err) => panic!("{}", err),
        };
        // if !http_tree.is_convention() {
        //     let matcher = UrlMatcher::new("TARGET".to_owned(), http_tree.rel_path.to_owned());
        //     println!("-------\n{:#?}\n--------", &matcher);
        //     println!("--- SERIAL_DECL");
        //     println!("{}", matcher.serial_decl());
        //     println!("--- PREPARE_DECL");
        //     println!("{}", matcher.prepare_decl("req".to_owned()));
        //     println!("--- START_DECL");
        //     println!("{}", matcher.start_decl(Some("req.params".to_owned())));
        //     println!("--- EXACT_DECL");
        //     println!("{}", matcher.exact_decl(Some("req.params".to_owned())));
        //     println!("--- UPDATE_DECL");
        //     println!("{}", matcher.update_decl("req".to_owned()));
        // }

        // let output = format!("{:#?}", handler.borrow());
        println!("{:#?}", handler.borrow());
        // let output_path = leaf.borrow().output_path.display().to_string();
        // println!("{}", &output_path);
        // let _ = fs::create_dir_all(join_paths("..", output_path.clone()).unwrap());
        // fs::write(&output_path, output).unwrap();
    }

    if let Some(fallback) = &http_tree.fallback {
        process_entry(&fallback.borrow());
    }
    if let Some(middleware) = &http_tree.middleware {
        process_entry(&middleware.borrow());
    }

    for child in http_tree.children.iter() {
        process_entry(&child.borrow());
    }
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
    let example_server = join_paths(rel_path, path.display().to_string());
    let http_tree = http_discover(CompileContext {
        output_dir: join_paths(".densky", example_server.clone()),
        routes_path: join_paths("src/routes", example_server.clone()),
        views_path: join_paths("src/views", example_server.clone()),
        static_path: join_paths("src/static", example_server.clone()),
        verbose: true,
        static_prefix: "static/".to_string(),
    });

    // process_entry(&http_tree.unwrap().borrow());

    // let matcher = UrlMatcher::new("TARGET".to_owned(), "abc/$VAR/def".to_owned());
    // println!("{:#?}", &matcher);
    // println!("--- SERIAL_DECL");
    // println!("{}", matcher.serial_decl());
    // println!("--- PREPARE_DECL");
    // println!("{}", matcher.prepare_decl("req".to_owned()));
    // println!("--- START_DECL");
    // println!("{}", matcher.start_decl(Some("req.params".to_owned())));
    // println!("--- EXACT_DECL");
    // println!("{}", matcher.exact_decl(Some("req.params".to_owned())));
    // println!("--- UPDATE_DECL");
    // println!("{}", matcher.update_decl("req".to_owned()));

    let http_tree = http_tree.map(|tree| tree.borrow().clone());

    if let Ok(http_tree) = &http_tree {
        // println!("{:#?}", http_tree);
        println!("{}", http_tree);
    } else {
        println!("{:#?}", http_tree);
    }
}
