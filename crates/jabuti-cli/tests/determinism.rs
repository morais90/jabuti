mod common;

use common::{function_of, project, shaped_like};
use tempfile::TempDir;

fn busy_project() -> TempDir {
    let mut files: Vec<(String, String)> = vec![(
        "jabuti.toml".to_owned(),
        "[rules]\nfunction-lines = { limit = 3 }\nduplicate-block = { limit = 40 }\n".to_owned(),
    )];

    for index in 0..24 {
        files.push((
            format!("src/module_{index:02}.rs"),
            format!(
                "{}\n{}\nfn read(value: Option<u32>) -> u32 {{\n    value.unwrap()\n}}\n",
                function_of(&format!("long_{index}"), 6),
                shaped_like(&format!("parse_{index}"), "parts", "key"),
            ),
        ));
        files.push((
            format!("web/page_{index:02}.ts"),
            format!(
                "export function render{index}(value: number): number {{\n  let total = value;\n  total += 1;\n  total += 2;\n  return total;\n}}\n"
            ),
        ));
        files.push((
            format!("app/Screen{index:02}.kt"),
            format!("fun screen{index}(value: Int): Int {{\n    val doubled = value * 2\n    val tripled = value * 3\n    return doubled + tripled\n}}\n"),
        ));
    }

    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(name, contents)| (name.as_str(), contents.as_str()))
        .collect();
    project(&borrowed)
}

fn output_with(directory: &TempDir, threads: &str, format: &str) -> Vec<u8> {
    common::binary(directory)
        .env("RAYON_NUM_THREADS", threads)
        .arg("check")
        .arg(".")
        .arg("--format")
        .arg(format)
        .arg("--limit")
        .arg("100000")
        .output()
        .expect("jabuti runs")
        .stdout
}

#[test]
fn the_output_is_byte_identical_whatever_the_number_of_threads() {
    let directory = busy_project();

    for format in ["agent", "json", "measures"] {
        let alone = output_with(&directory, "1", format);
        let crowded = output_with(&directory, "8", format);

        assert!(
            alone.len() > 10_000,
            "{format} carries too little to prove anything: {} bytes",
            alone.len()
        );
        assert!(alone == crowded, "{format} changed with the thread count");
    }
}
