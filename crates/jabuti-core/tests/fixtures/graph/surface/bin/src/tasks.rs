pub fn run() {
    Job::new().start();
    let limit = MAX;
    let handler = orphan_handler;
    handler(limit);
}

pub struct Job;

impl Job {
    pub fn new() -> Self {
        Job
    }

    pub fn start(&self) {}

    pub fn pause(&self) {}
}

pub const MAX: u32 = 5;

pub fn orphan() {}

pub fn orphan_handler(_: u32) {}

#[no_mangle]
/// Exported for the runtime.
pub extern "C" fn entry() {}

pub struct A;

impl A {
    fn make() -> A {
        A
    }
}

pub struct B;

impl B {
    pub fn make() -> B {
        B
    }
}

#[cfg(test)]
mod tests {
    pub fn helper() {}
}
