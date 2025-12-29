//! Integration tests for neve-common crate.

use neve_common::{BytePos, Interner, Span};

#[test]
fn test_span_merge() {
    let a = Span::from_usize(10, 20);
    let b = Span::from_usize(15, 30);
    let merged = a.merge(b);
    assert_eq!(merged.start.0, 10);
    assert_eq!(merged.end.0, 30);
}

#[test]
fn test_span_len() {
    let span = Span::from_usize(5, 15);
    assert_eq!(span.len(), 10);
}

#[test]
fn test_span_is_empty() {
    let empty = Span::from_usize(5, 5);
    let non_empty = Span::from_usize(5, 10);
    assert!(empty.is_empty());
    assert!(!non_empty.is_empty());
}

#[test]
fn test_span_range() {
    let span = Span::from_usize(5, 15);
    assert_eq!(span.range(), 5..15);
}

#[test]
fn test_byte_pos_offset() {
    let pos = BytePos(10);
    assert_eq!(pos.offset(5), BytePos(15));
}

#[test]
fn test_intern() {
    let mut interner = Interner::new();
    let a = interner.intern("hello");
    let b = interner.intern("world");
    let c = interner.intern("hello");

    assert_eq!(a, c);
    assert_ne!(a, b);
    assert_eq!(interner.get(a), "hello");
    assert_eq!(interner.get(b), "world");
}

#[test]
fn test_symbol_as_u32() {
    let mut interner = Interner::new();
    let sym = interner.intern("test");
    assert_eq!(sym.as_u32(), 0);

    let sym2 = interner.intern("another");
    assert_eq!(sym2.as_u32(), 1);
}
