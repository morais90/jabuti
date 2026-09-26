use std::path::{Path, PathBuf};

use jabuti_core::diff::Diff;
use jabuti_core::model::Span;
use rstest::rstest;

const UNIFIED: &str = "\
diff --git a/src/lib.rs b/src/lib.rs
index 1111111..2222222 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -3,0 +4,2 @@ fn existing() -> i32 {
+fn added() {}
+
@@ -10 +12 @@ fn later() {
-    old();
+    new();
@@ -20,2 +21,0 @@ fn gone() {
-    first();
-    second();
diff --git a/src/new.rs b/src/new.rs
new file mode 100644
index 0000000..3333333
--- /dev/null
+++ b/src/new.rs
@@ -0,0 +1,3 @@
+fn fresh() {
+    1;
+}
diff --git a/src/removed.rs b/src/removed.rs
deleted file mode 100644
index 4444444..0000000
--- a/src/removed.rs
+++ /dev/null
@@ -1,2 +0,0 @@
-fn stale() {
-}
diff --git a/src/trimmed.rs b/src/trimmed.rs
index 5555555..6666666 100644
--- a/src/trimmed.rs
+++ b/src/trimmed.rs
@@ -5 +4,0 @@ fn kept() {
-    noise();
";

fn lines(start_line: u32, end_line: u32) -> Span {
    Span {
        start_line,
        end_line,
    }
}

#[rstest]
#[case("src/lib.rs", lines(4, 4), true)]
#[case("src/lib.rs", lines(5, 9), true)]
#[case("src/lib.rs", lines(1, 3), false)]
#[case("src/lib.rs", lines(6, 11), false)]
#[case("src/lib.rs", lines(12, 12), true)]
#[case("src/lib.rs", lines(13, 40), false)]
#[case("src/lib.rs", lines(1, 40), true)]
#[case("src/new.rs", lines(1, 3), true)]
#[case("src/new.rs", lines(4, 9), false)]
#[case("src/removed.rs", lines(1, 2), false)]
#[case("src/trimmed.rs", lines(1, 9), false)]
#[case("src/untouched.rs", lines(1, 9), false)]
fn a_span_is_touched_only_where_it_overlaps_an_added_line(
    #[case] path: &str,
    #[case] span: Span,
    #[case] touched: bool,
) {
    let diff = Diff::parse(UNIFIED);

    assert_eq!(diff.touches(Path::new(path), span), touched);
}

#[rstest]
#[case("src/lib.rs", true)]
#[case("src/new.rs", true)]
#[case("src/removed.rs", false)]
#[case("src/trimmed.rs", false)]
#[case("src/untouched.rs", false)]
fn only_a_file_that_gained_lines_is_covered(#[case] path: &str, #[case] covered: bool) {
    let diff = Diff::parse(UNIFIED);

    assert_eq!(diff.covers(Path::new(path)), covered);
}

#[test]
fn a_file_added_whole_is_touched_on_every_line() {
    let mut diff = Diff::parse(UNIFIED);

    diff.add_whole(PathBuf::from("src/untracked.rs"));

    assert!(diff.covers(Path::new("src/untracked.rs")));
    assert!(diff.touches(Path::new("src/untracked.rs"), lines(500, 500)));
}

#[test]
fn an_empty_diff_covers_nothing() {
    let diff = Diff::parse("");

    assert!(!diff.covers(Path::new("src/lib.rs")));
    assert_eq!(diff, Diff::default());
}
