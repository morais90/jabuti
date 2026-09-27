pub(crate) mod since;

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

pub(crate) fn run(arguments: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(arguments)
        .output()
        .context("running git")?;

    collected(arguments, output)
}

pub(crate) fn run_at(root: &Path, arguments: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .context("running git")?;

    collected(arguments, output)
}

pub(crate) fn blobs(revision: &str, paths: &[PathBuf]) -> Result<BTreeMap<PathBuf, String>> {
    let arguments = ["cat-file", "--batch"];
    let mut child = Command::new("git")
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("running git")?;
    let mut stdin = child.stdin.take().context("opening git input")?;
    let requests = requests_for(revision, paths);
    let writer = std::thread::spawn(move || stdin.write_all(requests.as_bytes()));

    let output = child.wait_with_output().context("running git")?;
    succeeded(&arguments, &output)?;
    writer
        .join()
        .unwrap_or(Ok(()))
        .context("sending requests to git")?;

    texts(paths, &output.stdout)
}

fn requests_for(revision: &str, paths: &[PathBuf]) -> String {
    let mut requests = String::new();
    for path in paths {
        requests.push_str(revision);
        requests.push(':');
        requests.push_str(&path.to_string_lossy());
        requests.push('\n');
    }

    requests
}

fn texts(paths: &[PathBuf], output: &[u8]) -> Result<BTreeMap<PathBuf, String>> {
    let mut found = BTreeMap::new();
    let mut rest = output;

    for path in paths {
        let Some((body, after)) = response(rest) else {
            bail!("git cat-file answered fewer requests than it was sent");
        };
        rest = after;

        if let Some(text) = body.and_then(|bytes| String::from_utf8(bytes.to_vec()).ok()) {
            found.insert(path.clone(), text);
        }
    }

    Ok(found)
}

fn response(bytes: &[u8]) -> Option<(Option<&[u8]>, &[u8])> {
    let (header, after_header) = split_line(bytes)?;
    let Some((kind, size)) = object(header) else {
        return Some((None, after_header));
    };

    let (body, after_body) = after_header.split_at(size.min(after_header.len()));
    let rest = after_body.strip_prefix(b"\n").unwrap_or(after_body);
    let blob = (kind == "blob").then_some(body);

    Some((blob, rest))
}

fn split_line(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let end = bytes.iter().position(|byte| *byte == b'\n')?;

    Some((&bytes[..end], &bytes[end + 1..]))
}

fn object(header: &[u8]) -> Option<(&str, usize)> {
    let header = std::str::from_utf8(header).ok()?;
    let mut fields = header.rsplit(' ');
    let size = fields.next()?.parse().ok()?;
    let kind = fields.next()?;

    Some((kind, size))
}

fn collected(arguments: &[&str], output: std::process::Output) -> Result<String> {
    succeeded(arguments, &output)?;

    String::from_utf8(output.stdout).context("git produced output that is not utf8")
}

fn succeeded(arguments: &[&str], output: &std::process::Output) -> Result<()> {
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}
