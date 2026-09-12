#![allow(clippy::all)]

fn live() {
    let first = read().unwrap();
    let second = read().expect("available");
    let _ = save();
    let optional = read().ok();
    match read() { Ok(value) => value, Err(_) => {} };
    let handled = read().unwrap_or_default();
}

#[test]
fn checks() {
    let hidden = read().unwrap();
}

#[allow(dead_code)]
fn unused() {}
