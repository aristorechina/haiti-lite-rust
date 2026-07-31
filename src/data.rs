use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::error::Category;
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct Prototype {
    pub regex: String,
    pub modes: Vec<Mode>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Mode {
    pub john: Option<String>,
    pub hashcat: Option<i64>,
    pub extended: bool,
    pub name: String,
    #[serde(default)]
    pub samples: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct DataSet {
    pub prototypes: Vec<Prototype>,
    pub commons: Vec<String>,
}

#[derive(Debug, Error)]
pub enum DataError {
    #[error("could not read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("schema validation failed for {path}: {message}")]
    Schema { path: PathBuf, message: String },
}

impl DataSet {
    pub fn load(data_dir: &Path) -> Result<Self, DataError> {
        let prototypes_path = data_dir.join("prototypes.json");
        let commons_path = data_dir.join("commons.json");
        let prototypes: Vec<Prototype> = read_json(&prototypes_path)?;
        let commons: Vec<String> = read_json(&commons_path)?;

        validate_prototypes(&prototypes, &prototypes_path)?;
        validate_commons(&commons, &commons_path)?;

        Ok(Self {
            prototypes,
            commons,
        })
    }
}

fn read_json<T>(path: &Path) -> Result<T, DataError>
where
    T: DeserializeOwned,
{
    let contents = fs::read_to_string(path).map_err(|source| DataError::Io {
        path: path.to_owned(),
        source,
    })?;

    serde_json::from_str(&contents).map_err(|source| {
        if matches!(source.classify(), Category::Syntax | Category::Eof) {
            DataError::Json {
                path: path.to_owned(),
                source,
            }
        } else {
            DataError::Schema {
                path: path.to_owned(),
                message: source.to_string(),
            }
        }
    })
}

fn validate_prototypes(prototypes: &[Prototype], path: &Path) -> Result<(), DataError> {
    if prototypes.is_empty() {
        return Err(DataError::Schema {
            path: path.to_owned(),
            message: "the prototype list must not be empty".into(),
        });
    }

    for (prototype_index, prototype) in prototypes.iter().enumerate() {
        if prototype.regex.is_empty() {
            return Err(DataError::Schema {
                path: path.to_owned(),
                message: format!("prototype {prototype_index} has an empty regex"),
            });
        }
        if prototype.modes.is_empty() {
            return Err(DataError::Schema {
                path: path.to_owned(),
                message: format!("prototype {prototype_index} has no modes"),
            });
        }

        for mode in &prototype.modes {
            if mode.name.trim().is_empty() {
                return Err(DataError::Schema {
                    path: path.to_owned(),
                    message: format!("prototype {prototype_index} contains a mode with no name"),
                });
            }
        }
    }

    Ok(())
}

fn validate_commons(commons: &[String], path: &Path) -> Result<(), DataError> {
    let mut names = HashSet::new();
    for name in commons {
        if name.trim().is_empty() {
            return Err(DataError::Schema {
                path: path.to_owned(),
                message: "commons contains an empty name".into(),
            });
        }
        if !names.insert(name) {
            return Err(DataError::Schema {
                path: path.to_owned(),
                message: format!("commons contains duplicate name `{name}`"),
            });
        }
    }

    Ok(())
}
