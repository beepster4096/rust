// run-pass
#![feature(explicit_tail_calls)]
use std::sync::atomic::{self, AtomicU8};

fn main() {
   tail_recursive(0, &AtomicU8::default());
   simply_recursive(0, &AtomicU8::default());
}

fn tail_recursive(n: u8, order: &AtomicU8) {
    if n > 128 {
        return;
    }

    let _local = AssertDropOrder(n, order);

    become tail_recursive(n + 1, order)
}

fn simply_recursive(n: u8, order: &AtomicU8) {
    if n > 128 {
        return;
    }

    let _local = AssertDropOrder(128 - n, order);

    return simply_recursive(n + 1, order)
}

struct AssertDropOrder<'a>(u8, &'a AtomicU8);

impl Drop for AssertDropOrder<'_> {
    #[track_caller]
    fn drop(&mut self) {
        let order = self.1.fetch_add(1, atomic::Ordering::Relaxed);
        assert_eq!(order, self.0, "Drop out of order!");
    }
}
