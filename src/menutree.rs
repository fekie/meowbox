use core::fmt;

use either::Either;
use esp_println::{dbg, println};
use heapless::{String, Vec};

#[derive(Debug)]
pub enum MenuFolder {
    Tests,
}

// impl fmt::Display for MenuFolder {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let s = match self {
//             MenuFolder::Tests => "Tests",
//         };

//         write!(f, "{}", s)
//     }
// }

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

// impl fmt::Display for MenuProgram {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let s = match self {
//             MenuProgram::LightShow => "LightShow",
//             MenuProgram::Beeper => "Beeper",
//             MenuProgram::BuzzerTest => "BuzzerTest",
//         };

//         write!(f, "{}", s)
//     }
// }

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
