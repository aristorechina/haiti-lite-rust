use std::fs;
use std::path::{Path, PathBuf};

use haiti_lite_rust::cli::OutputKind;
use haiti_lite_rust::data::{DataError, DataSet};
use haiti_lite_rust::matcher::{CompiledRules, MatchRecord, Reference};

fn upstream_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("data")
}

#[test]
fn every_upstream_sample_matches_its_declared_prototype() {
    let data = DataSet::load(&upstream_data_dir()).unwrap();
    let rules = CompiledRules::compile(&data).unwrap();
    let mut sample_count = 0;

    for (prototype_index, prototype) in data.prototypes.iter().enumerate() {
        for mode in &prototype.modes {
            for sample in mode.samples.as_deref().unwrap_or_default() {
                sample_count += 1;
                assert!(
                    rules.matches_prototype(prototype_index, sample).unwrap(),
                    "prototype {prototype_index} `{}` does not match sample `{sample}`",
                    prototype.regex
                );
            }
        }
    }

    assert_eq!(sample_count, 925);
}

#[test]
fn upstream_samples_are_case_insensitive_when_letters_are_present() {
    let data = DataSet::load(&upstream_data_dir()).unwrap();
    let rules = CompiledRules::compile(&data).unwrap();

    for (prototype_index, prototype) in data.prototypes.iter().enumerate() {
        for mode in &prototype.modes {
            for sample in mode.samples.as_deref().unwrap_or_default() {
                let uppercase = sample.to_ascii_uppercase();
                if uppercase != *sample {
                    assert!(
                        rules
                            .matches_prototype(prototype_index, &uppercase)
                            .unwrap(),
                        "prototype {prototype_index} `{}` did not match case variant `{uppercase}`",
                        prototype.regex
                    );
                }
            }
        }
    }
}

#[test]
fn data_loader_supports_paths_with_spaces_and_external_updates() {
    let root = unique_temp_dir("haiti parsable rust data");
    fs::create_dir_all(&root).unwrap();
    write_data(
        &root,
        r#"[{"regex":"\\A[a-z]+\\Z","modes":[{"name":"First","hashcat":1,"john":null,"extended":false}]}]"#,
        r#"["First"]"#,
    );
    let first = DataSet::load(&root).unwrap();
    let first_rules = CompiledRules::compile(&first).unwrap();
    let first_matches = first_rules.identify("hash").unwrap();
    assert_eq!(
        first_rules.render(&first_matches, OutputKind::Hashcat, false),
        [MatchRecord {
            name: "First".into(),
            reference: Reference::Hashcat(1),
        }]
    );

    write_data(
        &root,
        r#"[{"regex":"\\A[0-9]+\\Z","modes":[{"name":"Second","hashcat":2,"john":null,"extended":false}]}]"#,
        r#"["Second"]"#,
    );
    let second = DataSet::load(&root).unwrap();
    let second_rules = CompiledRules::compile(&second).unwrap();
    let second_matches = second_rules.identify("123").unwrap();
    assert_eq!(
        second_rules.render(&second_matches, OutputKind::Hashcat, false),
        [MatchRecord {
            name: "Second".into(),
            reference: Reference::Hashcat(2),
        }]
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn data_loader_reports_missing_malformed_and_incompatible_files() {
    let root = unique_temp_dir("haiti parsable rust invalid");
    fs::create_dir_all(&root).unwrap();

    let missing = DataSet::load(&root).unwrap_err();
    assert!(missing.to_string().contains("prototypes.json"));

    fs::write(root.join("prototypes.json"), "not-json").unwrap();
    fs::write(root.join("commons.json"), "[]").unwrap();
    let malformed = DataSet::load(&root).unwrap_err();
    assert!(matches!(malformed, DataError::Json { .. }));

    fs::write(
        root.join("prototypes.json"),
        r#"[{"regex":123,"modes":[]}]"#,
    )
    .unwrap();
    let incompatible = DataSet::load(&root).unwrap_err();
    assert!(matches!(incompatible, DataError::Schema { .. }));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rendered_records_never_contain_ansi_escape_sequences() {
    let data = DataSet::load(&upstream_data_dir()).unwrap();
    let rules = CompiledRules::compile(&data).unwrap();
    let matches = rules.identify("5f4dcc3b5aa765d61d8327deb882cf99").unwrap();
    for record in rules.render(&matches, OutputKind::Hashcat, false) {
        assert!(!serde_json::to_string(&record).unwrap().contains('\u{1b}'));
    }
}

fn unique_temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{label} {} {}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ))
}

fn write_data(root: &Path, prototypes: &str, commons: &str) {
    fs::write(root.join("prototypes.json"), prototypes).unwrap();
    fs::write(root.join("commons.json"), commons).unwrap();
}
