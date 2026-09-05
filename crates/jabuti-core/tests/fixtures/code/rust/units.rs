mod parser {
    pub struct Config {
        pub depth: usize,
    }

    impl Config {
        pub fn new(depth: usize) -> Self {
            Self { depth }
        }

        pub fn doubled(&self) -> usize {
            let scale = |value: usize| value * 2;
            scale(self.depth)
        }
    }

    pub trait Visitor {
        fn visit(&self, depth: usize) -> bool {
            depth > 0
        }
    }
}

enum Mode {
    Fast,
    Deep,
}

fn outer() -> usize {
    fn inner() -> usize {
        1
    }

    inner()
}
