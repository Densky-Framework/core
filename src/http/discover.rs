use std::sync::{Arc, Mutex};

use crate::{
    walker::{walker_tree_discover, WalkerContainer, WalkerDiscoverError, WalkerTree},
    CompileContext,
};

pub fn http_discover(
    compile_context: &CompileContext,
) -> Result<(WalkerContainer, Arc<Mutex<WalkerTree>>), WalkerDiscoverError> {
    return walker_tree_discover(
        "http",
        compile_context.routes_path.clone(),
        &compile_context,
    );
}
