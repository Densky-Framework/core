use std::sync::{Arc, Mutex, MutexGuard};

use crate::utils::join_paths;

use super::{WalkerEntity, WalkerLeaf, WalkerTree};

type SyncNode<T> = Arc<Mutex<T>>;

pub struct WalkerContainer {
    output_dir: String,
    root: Option<usize>,
    tree: Vec<SyncNode<WalkerTree>>,
    leaf: Vec<SyncNode<WalkerLeaf>>,
}

impl WalkerContainer {
    pub fn new<O>(output_dir: O) -> WalkerContainer
    where
        O: AsRef<str>,
    {
        WalkerContainer {
            output_dir: output_dir.as_ref().to_string(),
            root: None,
            tree: vec![],
            leaf: vec![],
        }
    }

    pub fn get_output_dir(&self) -> String {
        self.output_dir.clone()
    }

    pub fn create_root(&mut self) -> SyncNode<WalkerTree> {
        let mut root = WalkerTree::new();
        root.id = self.id_tree();
        root.output_path = join_paths("_index", &self.output_dir).into();
        root.is_root = true;
        self.root = Some(root.id);

        let root = Arc::new(Mutex::new(root));
        self.tree.push(root.clone());

        root
    }

    pub fn id_tree(&self) -> usize {
        self.tree.len() + 1
    }

    pub fn id_leaf(&self) -> usize {
        self.leaf.len() + 1
    }

    pub fn add_tree(&mut self, mut new_node: WalkerTree) -> usize {
        let new_id = self.id_tree();
        new_node.set_id(new_id.clone());
        self.tree.push(Arc::new(Mutex::new(new_node)));
        new_id
    }

    pub fn add_leaf(&mut self, mut new_node: WalkerLeaf) -> usize {
        let new_id = self.id_leaf();
        new_node.set_id(new_id.clone());
        self.leaf.push(Arc::new(Mutex::new(new_node)));
        new_id
    }

    pub fn get_tree(&self, id: usize) -> Option<SyncNode<WalkerTree>> {
        self.tree.get(id - 1).cloned()
    }

    pub fn get_tree_locked(&self, id: usize) -> Option<MutexGuard<'_, WalkerTree>> {
        let arc = self.tree.get(id - 1)?;
        match arc.lock() {
            Ok(val) => Some(val),
            Err(_) => None,
        }
    }

    pub fn get_leaf(&self, id: usize) -> Option<SyncNode<WalkerLeaf>> {
        self.leaf.get(id - 1).cloned()
    }

    pub fn get_leaf_locked(&self, id: usize) -> Option<MutexGuard<'_, WalkerLeaf>> {
        let arc = self.leaf.get(id - 1)?;
        match arc.lock() {
            Ok(val) => Some(val),
            Err(_) => None,
        }
    }

    pub fn get_root(&self) -> Option<SyncNode<WalkerTree>> {
        if let Some(root) = self.root {
            self.get_tree(root)
        } else {
            None
        }
    }

    pub fn get_root_locked(&self) -> Option<MutexGuard<'_, WalkerTree>> {
        if let Some(root) = self.root {
            self.get_tree_locked(root)
        } else {
            None
        }
    }

    pub fn get_root_id(&self) -> Option<usize> {
        self.root.clone()
    }

    #[cfg(not(debug_assertions))]
    pub fn debug_tree(&self) {}

    #[cfg(debug_assertions)]
    pub fn debug_tree(&self) {
        println!(
            "[CONTAINER TREE VIEW DEBUG]\n{:#?}\n[/CONTAINER TREE VIEW DEBUG]",
            &self.tree
        );
    }

    #[cfg(not(debug_assertions))]
    pub fn debug_leaf(&self) {}

    #[cfg(debug_assertions)]
    pub fn debug_leaf(&self) {
        println!(
            "[CONTAINER LEAF VIEW DEBUG]\n{:#?}\n[/CONTAINER LEAF VIEW DEBUG]",
            &self.leaf
        );
    }
}
