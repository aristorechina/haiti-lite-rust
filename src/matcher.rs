use std::collections::HashSet;

use fancy_regex::{Regex, RegexBuilder};
use serde::Serialize;
use thiserror::Error;

use crate::cli::OutputKind;
use crate::data::{DataSet, Mode};

struct CompiledPrototype {
    regex_source: String,
    regex: Regex,
    modes: Vec<Mode>,
}

pub struct CompiledRules {
    prototypes: Vec<CompiledPrototype>,
    commons: HashSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Reference {
    Hashcat(i64),
    John(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MatchRecord {
    pub name: String,
    pub reference: Reference,
}

#[derive(Debug, Error)]
pub enum MatchError {
    #[error("could not compile prototype {prototype_index} regex `{regex}`: {source}")]
    Compile {
        prototype_index: usize,
        regex: String,
        #[source]
        source: Box<fancy_regex::Error>,
    },
    #[error("could not evaluate prototype {prototype_index} regex `{regex}`: {source}")]
    Evaluate {
        prototype_index: usize,
        regex: String,
        #[source]
        source: Box<fancy_regex::Error>,
    },
}

impl CompiledRules {
    pub fn compile(data: &DataSet) -> Result<Self, MatchError> {
        let prototypes = data
            .prototypes
            .iter()
            .enumerate()
            .map(|(prototype_index, prototype)| {
                let mut builder = RegexBuilder::new(&prototype.regex);
                builder.case_insensitive(true);
                let regex = builder.build().map_err(|source| MatchError::Compile {
                    prototype_index,
                    regex: prototype.regex.clone(),
                    source: Box::new(source),
                })?;
                Ok(CompiledPrototype {
                    regex_source: prototype.regex.clone(),
                    regex,
                    modes: prototype.modes.clone(),
                })
            })
            .collect::<Result<Vec<_>, MatchError>>()?;

        Ok(Self {
            prototypes,
            commons: data.commons.iter().cloned().collect(),
        })
    }

    pub fn identify(&self, hash: &str) -> Result<Vec<&Mode>, MatchError> {
        let mut matches = Vec::new();
        for (prototype_index, prototype) in self.prototypes.iter().enumerate() {
            let is_match = self.matches_prototype(prototype_index, hash)?;
            if is_match {
                matches.extend(prototype.modes.iter());
            }
        }

        matches.sort_by_key(|mode| usize::from(!self.commons.contains(&mode.name)));
        Ok(matches)
    }

    pub fn matches_prototype(
        &self,
        prototype_index: usize,
        hash: &str,
    ) -> Result<bool, MatchError> {
        let prototype = self
            .prototypes
            .get(prototype_index)
            .expect("prototype index must come from the loaded data");
        prototype
            .regex
            .is_match(hash)
            .map_err(|source| MatchError::Evaluate {
                prototype_index,
                regex: prototype.regex_source.clone(),
                source: Box::new(source),
            })
    }

    pub fn render(
        &self,
        matches: &[&Mode],
        output_kind: OutputKind,
        extended: bool,
    ) -> Vec<MatchRecord> {
        matches
            .iter()
            .filter(|mode| extended || !mode.extended)
            .filter_map(|mode| match output_kind {
                OutputKind::Hashcat => mode.hashcat.map(|reference| MatchRecord {
                    name: mode.name.clone(),
                    reference: Reference::Hashcat(reference),
                }),
                OutputKind::John => mode.john.as_ref().map(|reference| MatchRecord {
                    name: mode.name.clone(),
                    reference: Reference::John(reference.clone()),
                }),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{CompiledRules, MatchRecord, Reference};
    use crate::cli::OutputKind;
    use crate::data::{DataSet, Mode, Prototype};

    fn mode(name: &str, hashcat: Option<i64>, john: Option<&str>, extended: bool) -> Mode {
        Mode {
            name: name.into(),
            hashcat,
            john: john.map(str::to_owned),
            extended,
            samples: None,
        }
    }

    #[test]
    fn supports_ruby_anchors_lookahead_and_case_insensitivity() {
        let data = DataSet {
            prototypes: vec![Prototype {
                regex: r"\A(?=.*[a-z])[a-z0-9]{4}\Z".into(),
                modes: vec![mode("Example", Some(123), Some("example"), false)],
            }],
            commons: vec![],
        };
        let rules = CompiledRules::compile(&data).unwrap();
        let matches = rules.identify("A1B2").unwrap();
        assert_eq!(
            rules.render(&matches, OutputKind::Hashcat, false),
            [MatchRecord {
                name: "Example".into(),
                reference: Reference::Hashcat(123),
            }]
        );
    }

    #[test]
    fn preserves_common_order_and_filters_extended_output() {
        let data = DataSet {
            prototypes: vec![Prototype {
                regex: r"\A[a-z]+\Z".into(),
                modes: vec![
                    mode("Other", Some(2), None, false),
                    mode("Common", Some(1), None, false),
                    mode("Salted", Some(3), None, true),
                ],
            }],
            commons: vec!["Common".into()],
        };
        let rules = CompiledRules::compile(&data).unwrap();
        let matches = rules.identify("hash").unwrap();
        assert_eq!(
            rules.render(&matches, OutputKind::Hashcat, false),
            [
                MatchRecord {
                    name: "Common".into(),
                    reference: Reference::Hashcat(1),
                },
                MatchRecord {
                    name: "Other".into(),
                    reference: Reference::Hashcat(2),
                }
            ]
        );
        assert_eq!(
            rules.render(&matches, OutputKind::Hashcat, true),
            [
                MatchRecord {
                    name: "Common".into(),
                    reference: Reference::Hashcat(1),
                },
                MatchRecord {
                    name: "Other".into(),
                    reference: Reference::Hashcat(2),
                },
                MatchRecord {
                    name: "Salted".into(),
                    reference: Reference::Hashcat(3),
                }
            ]
        );
    }

    #[test]
    fn reports_unsupported_or_invalid_expressions_before_matching() {
        let data = DataSet {
            prototypes: vec![Prototype {
                regex: "[".into(),
                modes: vec![mode("Broken", Some(1), None, false)],
            }],
            commons: vec![],
        };
        let error = match CompiledRules::compile(&data) {
            Ok(_) => panic!("invalid regex unexpectedly compiled"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("prototype 0"));
        assert!(error.to_string().contains("["));
    }
}
