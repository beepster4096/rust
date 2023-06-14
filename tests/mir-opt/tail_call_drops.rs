#![allow(incomplete_features)]
#![feature(explicit_tail_calls)]

// EMIT_MIR tail_call_drops.f.built.after.mir
//   Expected result:
//   drop(_d) -> drop(_c) -> drop(_a) -> tailcall g()
//
// EMIT_MIR tail_call_drops.f.ElaborateDrops.diff
//   Expected result:
//   drop(_d) ->             drop(_a) -> tailcall g()
fn f() {

    let _a = String::new();
    let _b = 12;
    let _c = String::new();
    let _d = String::new();

    drop(_c);

    become g();
}

fn g() {}

fn main() {}
