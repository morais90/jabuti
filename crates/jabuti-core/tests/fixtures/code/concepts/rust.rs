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

fn asserted() {
    assert!(true);
}

fn written(mut log: String, out: &mut std::io::Stdout, report: &mut String) {
    let mut text = String::new();
    writeln!(text, "line").expect("formatting into memory");
    write!(log, "line").unwrap();
    write!(report, "line").unwrap();
    writeln!(out, "line").expect("the terminal is there");
}

const _: () = {
    let mut text = String::new();
    write!(text, "line").unwrap();
};

fn shadowed(path: &str) -> std::io::Result<()> {
    let mut out = String::new();
    write!(out, "line").unwrap();
    let mut out = std::fs::File::create(path)?;
    writeln!(out, "line").unwrap();
    let mut bytes = format!("{path}").into_bytes();
    write!(bytes, "line").unwrap();
    let mut text = String::new();
    write!(text, "line").unwrap(); let n = path.parse::<u8>().unwrap();
    Ok(())
}

fn spelled<'a>(first: &'a mut String, second: &mut std::string::String) {
    write!(first, "line").unwrap();
    write!(second, "line").unwrap();
    fn inner() {
        let first = 0;
    }
}

const _: () = {
    write!(unknown, "line").unwrap();
};
