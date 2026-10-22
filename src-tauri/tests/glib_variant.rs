//! Run with --release: the original unsound FFI call can pass debug tests.
#![cfg(target_os = "linux")]

use glib::prelude::*;

#[test]
fn variant_strings_survive_optimized_ffi_iteration() {
    let strings = ["first", "한글", "😀", "", "last"];
    let variant = strings.to_variant();
    assert_eq!(variant.array_iter_str().unwrap().collect::<Vec<_>>(), strings);
    assert_eq!(variant.array_iter_str().unwrap().rev().collect::<Vec<_>>(),
        strings.into_iter().rev().collect::<Vec<_>>());
    assert_eq!(variant.array_iter_str().unwrap().nth(1), Some("한글"));
    assert_eq!(variant.array_iter_str().unwrap().nth_back(2), Some("😀"));
    assert_eq!(variant.array_iter_str().unwrap().last(), Some("last"));
    let mut iter = variant.array_iter_str().unwrap();
    assert_eq!(iter.next(), Some("first"));
    assert_eq!(iter.next_back(), Some("last"));
    assert_eq!(iter.len(), 3);
    assert_eq!(iter.collect::<Vec<_>>(), ["한글", "😀", ""]);
}
