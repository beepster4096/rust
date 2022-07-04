// build-pass
// compile-flags: -Z mir-opt-level=4

#![crate_type = "lib"]
#![feature(lang_items, super_pointer)]
#![no_std]

#[lang = "owned_box"]
pub struct Box<T: ?Sized>(*super T);

impl<T: ?Sized> Drop for Box<T> {
    fn drop(&mut self) {}
}

#[lang = "box_free"]
#[inline(always)]
unsafe fn box_free<T: ?Sized>(ptr: *super T) {
    dealloc(ptr as *mut T)
}

#[inline(never)]
fn dealloc<T: ?Sized>(_: *mut T) {}

pub struct Foo<T>(T);

pub fn foo(a: Option<Box<Foo<usize>>>) -> usize {
    let f = match a {
        None => Foo(0),
        Some(vec) => *vec,
    };
    f.0
}
