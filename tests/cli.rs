use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_haiti-lite-rust")
}

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("data")
}

fn run(args: &[&str]) -> Output {
    Command::new(binary())
        .args(args)
        .output()
        .expect("binary should start")
}

fn run_with_stdin(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(binary())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary should start");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(input.as_bytes())
        .expect("stdin should accept input");
    child.wait_with_output().expect("binary should finish")
}

fn json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON")
}

fn args(mode: &str, hash: &str) -> Vec<String> {
    vec![
        "--data-dir".into(),
        data_dir().display().to_string(),
        mode.into(),
        hash.into(),
    ]
}

#[test]
fn cli_preserves_modes_filters_and_json_output() {
    let hc_args = args("hc", "5f4dcc3b5aa765d61d8327deb882cf99");
    let hc = run(&hc_args.iter().map(String::as_str).collect::<Vec<_>>());
    assert!(hc.status.success());
    let hc_json = json(&hc);
    assert_eq!(hc_json["mode"], "hc");
    assert_eq!(hc_json["identified"], true);
    assert_eq!(hc_json["matches"][0]["name"], "MD5");
    assert_eq!(hc_json["matches"][0]["reference"], 0);
    assert!(!hc_json.to_string().contains('\u{1b}'));
    assert!(!hc_json.to_string().contains("Double MD5"));

    let mut extended_args = args("hc", "5f4dcc3b5aa765d61d8327deb882cf99");
    extended_args.insert(2, "--extended".into());
    let extended = run(&extended_args.iter().map(String::as_str).collect::<Vec<_>>());
    assert!(extended.status.success());
    let extended_json = json(&extended);
    assert!(
        extended_json["matches"]
            .as_array()
            .unwrap()
            .iter()
            .any(|record| record["name"] == "Double MD5")
    );

    let jtr_args = args("jtr", "5f4dcc3b5aa765d61d8327deb882cf99");
    let jtr = run(&jtr_args.iter().map(String::as_str).collect::<Vec<_>>());
    assert!(jtr.status.success());
    let jtr_json = json(&jtr);
    assert_eq!(jtr_json["mode"], "jtr");
    assert_eq!(jtr_json["matches"][0]["name"], "MD5");
    assert_eq!(jtr_json["matches"][0]["reference"], "raw-md5");
}

#[test]
fn cli_supports_stdin_unknown_debug_and_version() {
    let stdin_args = args("hc", "-");
    let stdin = run_with_stdin(
        &stdin_args.iter().map(String::as_str).collect::<Vec<_>>(),
        "5f4dcc3b5aa765d61d8327deb882cf99\r\n",
    );
    assert!(stdin.status.success());
    assert_eq!(json(&stdin)["hash"], "5f4dcc3b5aa765d61d8327deb882cf99");

    let unknown_args = args("hc", "definitely-not-a-hash");
    let unknown = run(&unknown_args.iter().map(String::as_str).collect::<Vec<_>>());
    assert!(unknown.status.success());
    let unknown_json = json(&unknown);
    assert_eq!(unknown_json["identified"], false);
    assert_eq!(unknown_json["matches"].as_array().unwrap().len(), 0);

    let mut debug_args = args("hc", "5f4dcc3b5aa765d61d8327deb882cf99");
    debug_args.insert(2, "--debug".into());
    let debug = run(&debug_args.iter().map(String::as_str).collect::<Vec<_>>());
    assert!(debug.status.success());
    assert_eq!(json(&debug)["debug"]["extended"], false);

    let version = run(&["--version"]);
    assert!(version.status.success());
    let version_json = json(&version);
    assert_eq!(version_json["name"], "haiti-lite-rust");
    assert_eq!(version_json["version"], "0.1.0");
}

#[test]
fn help_is_rejected_and_matching_does_not_create_files() {
    let help = run(&["--help"]);
    assert!(!help.status.success());
    let help_json: serde_json::Value =
        serde_json::from_slice(&help.stderr).expect("errors should be valid JSON");
    assert!(
        help_json["error"]
            .as_str()
            .unwrap()
            .contains("unexpected argument")
    );
    assert!(!help_json.to_string().contains('\u{1b}'));

    let before = snapshot(&data_dir());
    let matching_args = args("hc", "5f4dcc3b5aa765d61d8327deb882cf99");
    let matching = run(&matching_args.iter().map(String::as_str).collect::<Vec<_>>());
    assert!(matching.status.success());
    assert_eq!(before, snapshot(&data_dir()));
}

fn snapshot(path: &Path) -> Vec<String> {
    let mut entries = fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    entries.sort();
    entries
}
