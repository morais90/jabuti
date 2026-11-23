use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use jabuti_core::catalog::{Rule, RuleId};
use jabuti_core::diff::{self, Diff, Placement};
use jabuti_core::model::{Detail, Finding, Span};
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

fn finding(path: &str, start_line: u32, end_line: u32) -> Finding {
    Finding {
        rule: RuleId::Native(Rule::FunctionLines),
        path: path.to_owned(),
        span: lines(start_line, end_line),
        subject: None,
        detail: Detail::Threshold {
            measured: 71,
            limit: 60,
        },
    }
}

#[test]
fn trimming_keeps_only_findings_on_added_lines_wherever_the_project_sits_and_through_a_link() {
    let placement = Placement {
        project: PathBuf::from("src"),
        aliases: BTreeMap::from([(PathBuf::from("mirror.rs"), PathBuf::from("src/new.rs"))]),
    };
    let mut findings = vec![
        finding("lib.rs", 4, 4),
        finding("lib.rs", 6, 11),
        finding("mirror.rs", 2, 2),
        finding("new.rs", 9, 9),
        finding("untouched.rs", 1, 3),
    ];

    Diff::parse(UNIFIED).trim(&mut findings, &placement);

    assert_eq!(
        findings,
        [finding("lib.rs", 4, 4), finding("mirror.rs", 2, 2)]
    );
}

#[test]
fn a_project_at_the_repository_root_places_every_file_where_it_is_shown() {
    let placement = Placement::default();

    assert_eq!(
        placement.in_repository(Path::new("src/lib.rs")),
        PathBuf::from("src/lib.rs")
    );
}

const LOOKALIKE: &str = "\
diff --git a/src/query.rs b/src/query.rs
index 1111111..2222222 100644
--- a/src/query.rs
+++ b/src/query.rs
@@ -2,2 +2 @@ fn query() {
-    stale();
--- the old comment
+++ the new comment
@@ -9,0 +10,2 @@ fn later() {
+    first();
+    second();
diff --git a/src/tail.rs b/src/tail.rs
index 3333333..4444444 100644
--- a/src/tail.rs
+++ b/src/tail.rs
@@ -1 +1 @@
-fn old() {}
\\ No newline at end of file
+fn new() {}
\\ No newline at end of file
";

#[test]
fn a_changed_line_that_looks_like_a_file_header_stays_inside_its_hunk() {
    let diff = Diff::parse(LOOKALIKE);

    assert_eq!(
        [
            diff.touches(Path::new("src/query.rs"), lines(2, 2)),
            diff.touches(Path::new("src/query.rs"), lines(10, 11)),
            diff.touches(Path::new("src/query.rs"), lines(3, 9)),
            diff.covers(Path::new("the new comment")),
            diff.touches(Path::new("src/tail.rs"), lines(1, 1)),
        ],
        [true, true, false, false, true]
    );
}

const WITH_CONTEXT: &str = "\
diff --git a/src/query.rs b/src/query.rs
index 1111111..2222222 100644
--- a/src/query.rs
+++ b/src/query.rs
@@ -1,3 +1,4 @@
 fn query() {
+    prepare();
     run();
 }
diff --git a/src/tail.rs b/src/tail.rs
index 3333333..4444444 100644
--- a/src/tail.rs
+++ b/src/tail.rs
@@ -1 +1 @@
-fn old() {}
+fn new() {}
";

#[test]
fn a_hunk_with_context_lines_ends_where_both_of_its_sides_do() {
    let diff = Diff::parse(WITH_CONTEXT);

    assert!(diff.covers(Path::new("src/query.rs")));
    assert!(diff.touches(Path::new("src/tail.rs"), lines(1, 1)));
}

fn measured(rule: Rule, subject: &str, start_line: u32, measured: u32) -> Finding {
    Finding {
        rule: RuleId::Native(rule),
        path: "src/lib.rs".to_owned(),
        span: lines(start_line, start_line + 9),
        subject: Some(subject.to_owned()),
        detail: Detail::Threshold { measured, limit: 4 },
    }
}

fn told(subject: &str, start_line: u32) -> Finding {
    Finding {
        rule: RuleId::Native(Rule::ErrorMasking),
        path: "src/lib.rs".to_owned(),
        span: lines(start_line, start_line),
        subject: Some(subject.to_owned()),
        detail: Detail::Message {
            message: "the failure is dropped without being read".to_owned(),
        },
    }
}

fn anywhere(_: &Finding) -> bool {
    true
}

fn unmoved(finding: &Finding) -> u32 {
    finding.span.start_line
}

#[test]
fn a_unit_over_its_limit_is_reported_only_when_the_change_made_its_measure_grow() {
    let base = [
        measured(Rule::Parameters, "grew", 1, 5),
        measured(Rule::Parameters, "held", 20, 6),
        measured(Rule::Parameters, "shrank", 40, 9),
    ];
    let current = vec![
        measured(Rule::Parameters, "grew", 3, 6),
        measured(Rule::Parameters, "held", 22, 6),
        measured(Rule::Parameters, "shrank", 42, 7),
    ];

    assert_eq!(
        diff::worsened(current, &base, anywhere, unmoved),
        [measured(Rule::Parameters, "grew", 3, 6)]
    );
}

#[test]
fn a_unit_the_base_never_had_over_its_limit_is_reported() {
    let base = [measured(Rule::FunctionLines, "old", 1, 70)];
    let current = vec![
        measured(Rule::FunctionLines, "old", 1, 70),
        measured(Rule::FunctionLines, "fresh", 30, 61),
        measured(Rule::Parameters, "old", 1, 5),
    ];

    assert_eq!(
        diff::worsened(current, &base, anywhere, unmoved),
        [
            measured(Rule::FunctionLines, "fresh", 30, 61),
            measured(Rule::Parameters, "old", 1, 5),
        ]
    );
}

#[test]
fn units_sharing_a_name_are_compared_in_the_order_they_appear() {
    let base = [
        measured(Rule::Parameters, "new", 1, 8),
        measured(Rule::Parameters, "new", 30, 5),
    ];
    let current = vec![
        measured(Rule::Parameters, "new", 1, 6),
        measured(Rule::Parameters, "new", 30, 7),
    ];

    assert_eq!(
        diff::worsened(current, &base, anywhere, unmoved),
        [measured(Rule::Parameters, "new", 30, 7)]
    );
}

#[test]
fn a_unit_sharing_its_name_with_one_that_grew_keeps_its_own_comparison() {
    let base = [
        measured(Rule::Parameters, "new", 1, 8),
        measured(Rule::Parameters, "new", 30, 5),
    ];
    let current = vec![
        measured(Rule::Parameters, "new", 1, 8),
        measured(Rule::Parameters, "new", 30, 9),
    ];

    assert_eq!(
        diff::worsened(current, &base, anywhere, unmoved),
        [measured(Rule::Parameters, "new", 30, 9)]
    );
}

#[test]
fn a_fact_is_reported_only_as_many_times_as_the_change_added_it_preferring_changed_lines() {
    let base = [told("load", 3), told("load", 9)];
    let current = vec![told("load", 3), told("load", 6), told("load", 12)];

    let reported = diff::worsened(
        current,
        &base,
        |finding| finding.span.start_line == 6,
        unmoved,
    );

    assert_eq!(reported, [told("load", 6)]);
}

#[test]
fn a_fact_the_change_moved_but_did_not_add_is_not_reported() {
    let base = [told("load", 3)];
    let current = vec![told("load", 40)];

    assert_eq!(diff::worsened(current, &base, anywhere, unmoved), []);
}

#[test]
fn facts_with_different_messages_or_units_are_counted_apart() {
    let mut other = told("load", 5);
    other.detail = Detail::Message {
        message: "another failure".to_owned(),
    };
    let base = [told("load", 3)];
    let current = vec![told("load", 3), told("save", 8), other.clone()];

    assert_eq!(
        diff::worsened(current, &base, anywhere, unmoved),
        [other, told("save", 8)]
    );
}

#[test]
fn a_renamed_file_is_compared_with_the_path_it_came_from() {
    let diff = Diff::parse(
        "diff --git a/src/old.rs b/src/new.rs\n\
         similarity index 90%\n\
         rename from src/old.rs\n\
         rename to src/new.rs\n\
         index 1111111..2222222 100644\n\
         --- a/src/old.rs\n\
         +++ b/src/new.rs\n\
         @@ -3 +3 @@\n\
         -    old();\n\
         +    new();\n\
         diff --git a/src/kept.rs b/src/kept.rs\n\
         index 3333333..4444444 100644\n\
         --- a/src/kept.rs\n\
         +++ b/src/kept.rs\n\
         @@ -1 +1 @@\n\
         -rename from src/fake.rs\n\
         +rename to src/kept.rs\n",
    );

    assert_eq!(
        diff.origin(Path::new("src/new.rs")),
        Path::new("src/old.rs")
    );
    assert_eq!(
        diff.origin(Path::new("src/kept.rs")),
        Path::new("src/kept.rs")
    );
    assert!(diff.touches(Path::new("src/new.rs"), lines(3, 3)));
}

#[test]
fn a_new_unit_added_above_one_sharing_its_name_is_not_mistaken_for_it() {
    let base = [measured(Rule::FunctionLines, "new", 1, 80)];
    let current = vec![
        measured(Rule::FunctionLines, "new", 1, 70),
        measured(Rule::FunctionLines, "new", 75, 80),
    ];

    let reported = diff::worsened(current, &base, anywhere, |finding| {
        finding.span.start_line + 74
    });

    assert_eq!(reported, [measured(Rule::FunctionLines, "new", 1, 70)]);
}

#[test]
fn a_line_of_the_base_is_moved_by_every_hunk_above_it() {
    let diff = Diff::parse(
        "diff --git a/src/lib.rs b/src/lib.rs\n\
         --- a/src/lib.rs\n\
         +++ b/src/lib.rs\n\
         @@ -2,0 +3,4 @@\n\
         +a\n\
         +b\n\
         +c\n\
         +d\n\
         @@ -10,3 +14,1 @@\n\
         -e\n\
         -f\n\
         -g\n\
         +h\n\
         @@ -20 +22 @@\n\
         -i\n\
         +j\n",
    );
    let moved = |line| diff.moved(Path::new("src/lib.rs"), line);

    assert_eq!(
        [1, 2, 3, 9, 10, 12, 13, 19, 20, 21].map(moved),
        [1, 2, 7, 13, 14, 14, 15, 21, 22, 23]
    );
    assert_eq!(diff.moved(Path::new("src/other.rs"), 9), 9);
}
