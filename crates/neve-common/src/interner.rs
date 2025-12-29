//! String interning for efficient symbol handling.
//! 字符串驻留，用于高效的符号处理。

use std::collections::HashMap;

/// An interned string symbol.
/// 驻留的字符串符号。
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(u32);

impl Symbol {
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl std::fmt::Debug for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Symbol({})", self.0)
    }
}

/// A string interner that maps strings to unique symbols.
/// 将字符串映射到唯一符号的字符串驻留器。
#[derive(Default)]
pub struct Interner {
    /// String to symbol mapping. / 字符串到符号的映射。
    map: HashMap<String, Symbol>,
    /// All interned strings. / 所有驻留的字符串。
    strings: Vec<String>,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern a string and return its symbol.
    /// 驻留字符串并返回其符号。
    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(&sym) = self.map.get(s) {
            return sym;
        }

        let sym = Symbol(self.strings.len() as u32);
        self.strings.push(s.to_owned());
        self.map.insert(s.to_owned(), sym);
        sym
    }

    /// Get the string for a symbol.
    /// 获取符号对应的字符串。
    pub fn get(&self, sym: Symbol) -> &str {
        &self.strings[sym.0 as usize]
    }
}
