use core::{
    fmt,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
};

use either::Either;
use esp_println::{dbg, println};
use heapless::{String, Vec};
use static_cell::StaticCell;

static _MENU_TREE: StaticCell<MenuTree> = StaticCell::new();

pub static _MENU_SCROLL: AtomicUsize = AtomicUsize::new(0);
pub static _MENU_NEEDS_UPDATE: AtomicBool = AtomicBool::new(true);
pub static _MENU_LAYER_0_SIZE: AtomicUsize = AtomicUsize::new(0);
pub static _MENU_LAYER_1_SIZE: AtomicUsize = AtomicUsize::new(0);
pub static _LAYER: AtomicUsize = AtomicUsize::new(0);

/// A zero cost struct for accessing the scroll index across tasks.
pub struct MenuStatusHandle {}

impl MenuStatusHandle {
    pub fn new() -> Self {
        Self {}
    }

    pub fn scroll(&self) -> usize {
        _MENU_SCROLL.load(SeqCst)
    }

    pub fn set_scroll(&self, value: usize) {
        _MENU_SCROLL.store(value, SeqCst);
    }

    pub fn needs_update(&self) -> bool {
        _MENU_NEEDS_UPDATE.load(SeqCst)
    }

    pub fn set_needs_update(&self, value: bool) {
        _MENU_NEEDS_UPDATE.store(value, SeqCst);
    }

    pub fn layer_0_size(&self) -> usize {
        _MENU_LAYER_0_SIZE.load(SeqCst)
    }

    pub fn set_layer_0_size(&self, value: usize) {
        _MENU_LAYER_0_SIZE.store(value, SeqCst);
    }

    pub fn layer_1_size(&self) -> usize {
        _MENU_LAYER_1_SIZE.load(SeqCst)
    }

    pub fn set_layer_1_size(&self, value: usize) {
        _MENU_LAYER_1_SIZE.store(value, SeqCst);
    }

    pub fn layer(&self) -> usize {
        _LAYER.load(SeqCst)
    }

    pub fn set_layer(&self, value: usize) {
        _LAYER.store(value, SeqCst);
    }

    pub fn current_layer_size(&self) -> usize {
        match _LAYER.load(SeqCst) {
            0 => _MENU_LAYER_0_SIZE.load(SeqCst),
            1 => _MENU_LAYER_1_SIZE.load(SeqCst),
            _ => panic!(),
        }
    }
}

#[derive(Debug)]
pub enum MenuFolder {
    Tests,
}

impl MenuFolder {
    pub fn as_str(&self) -> &'static str {
        match self {
            MenuFolder::Tests => "Tests",
        }
    }
}

#[derive(Debug)]
pub enum MenuProgram {
    LightShow,
    Beeper,

    // Exists under folder Test
    BuzzerTest,
}

impl MenuProgram {
    pub fn as_str(&self) -> &'static str {
        match self {
            MenuProgram::LightShow => "LightShow",
            MenuProgram::Beeper => "Beeper",
            MenuProgram::BuzzerTest => "BuzzerTest",
        }
    }
}

#[derive(Debug)]
pub enum MenuGeneralItem {
    MenuProgram(MenuProgram),
    MenuFolder(MenuFolder),
}

#[derive(Debug)]
pub struct MenuTree {
    /// How much the menu has been "scrolled down"
    pub offset: usize,
    pub layer_0: Vec<MenuGeneralItem, 10>,
    pub layer_1: Vec<MenuGeneralItem, 10>,
}

/// This is where the definition of the menu tree for this program
/// exists.
fn generate_menu_definition() -> MenuTree {
    //MenuTree { inner: arena }

    let mut layer_0 = Vec::new();
    layer_0
        .push(MenuGeneralItem::MenuProgram(MenuProgram::LightShow))
        .unwrap();
    layer_0
        .push(MenuGeneralItem::MenuProgram(MenuProgram::Beeper))
        .unwrap();

    layer_0
        .push(MenuGeneralItem::MenuFolder(MenuFolder::Tests))
        .unwrap();

    let mut layer_1 = Vec::new();

    layer_1
        .push(MenuGeneralItem::MenuProgram(MenuProgram::BuzzerTest))
        .unwrap();

    // update the layer length
    let handle = MenuStatusHandle::new();

    handle.set_layer_0_size(layer_0.len());
    handle.set_layer_1_size(layer_1.len());

    MenuTree {
        layer_0,
        layer_1,
        offset: 0,
    }
}

impl MenuTree {
    pub fn new() -> Self {
        dbg!(generate_menu_definition())
    }
}

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
