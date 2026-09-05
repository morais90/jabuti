pub struct Exposed;

impl Exposed {
    pub fn open(&self) {}
}

pub struct Hidden;

pub fn helper() -> u32 {
    2
}

#[derive(Debug)]
pub struct Plain;

#[no_mangle]
pub extern "C" fn entry() {}
