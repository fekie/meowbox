use static_cell::StaticCell;

use crate::menutree::MenuTree;

static _MENU_TREE: StaticCell<MenuTree> = StaticCell::new();

pub struct MenuResources {
    pub menu_tree: &'static mut MenuTree,
}

impl MenuResources {
    pub fn new() -> Self {
        Self {
            menu_tree: _MENU_TREE.init(MenuTree::new()),
        }
    }
}
