//! Plan YAML schema definitions and validation

use crate::validation::validate_id;
use serde::{Deserialize, Serialize};

/// Root structure of the loom metadata block
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoomMetadata {
    pub loom: LoomConfig,
}

/// Main loom configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoomConfig {
    pub version: u32,
    pub stages: Vec<StageDefinition>,
}

/// Stage definition from plan metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub parallel_group: Option<String>,
    #[serde(default)]
    pub acceptance: Vec<String>,
    #[serde(default)]
    pub setup: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
}

/// Validation error with context
#[derive(Debug)]
pub struct ValidationError {
    pub message: String,
    pub stage_id: Option<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(id) = &self.stage_id {
            write!(f, "Stage '{}': {}", id, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validate the loom metadata
pub fn validate(metadata: &LoomMetadata) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // Check version
    if metadata.loom.version != 1 {
        errors.push(ValidationError {
            message: format!(
                "Unsupported version: {}. Only version 1 is supported.",
                metadata.loom.version
            ),
            stage_id: None,
        });
    }

    // Check for empty stages
    if metadata.loom.stages.is_empty() {
        errors.push(ValidationError {
            message: "No stages defined".to_string(),
            stage_id: None,
        });
    }

    // Collect all stage IDs
    let stage_ids: std::collections::HashSet<_> =
        metadata.loom.stages.iter().map(|s| &s.id).collect();

    // Validate each stage
    for stage in &metadata.loom.stages {
        // Check for empty ID
        if stage.id.is_empty() {
            errors.push(ValidationError {
                message: "Stage ID cannot be empty".to_string(),
                stage_id: None,
            });
            continue;
        }

        // Validate stage ID is safe for file paths (prevents path traversal attacks)
        if let Err(e) = validate_id(&stage.id) {
            errors.push(ValidationError {
                message: format!("Invalid stage ID: {e}"),
                stage_id: Some(stage.id.clone()),
            });
        }

        // Check for empty name
        if stage.name.is_empty() {
            errors.push(ValidationError {
                message: "Stage name cannot be empty".to_string(),
                stage_id: Some(stage.id.clone()),
            });
        }

        // Validate dependencies exist and have valid IDs
        for dep in &stage.dependencies {
            // Validate dependency ID format (prevents path traversal in dependency refs)
            if let Err(e) = validate_id(dep) {
                errors.push(ValidationError {
                    message: format!("Invalid dependency ID '{dep}': {e}"),
                    stage_id: Some(stage.id.clone()),
                });
                continue;
            }

            if !stage_ids.contains(dep) {
                errors.push(ValidationError {
                    message: format!("Unknown dependency: '{dep}'"),
                    stage_id: Some(stage.id.clone()),
                });
            }

            // Check for self-dependency
            if dep == &stage.id {
                errors.push(ValidationError {
                    message: "Stage cannot depend on itself".to_string(),
                    stage_id: Some(stage.id.clone()),
                });
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_metadata() -> LoomMetadata {
        LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![
                    StageDefinition {
                        id: "stage-1".to_string(),
                        name: "Stage One".to_string(),
                        description: None,
                        dependencies: vec![],
                        parallel_group: None,
                        acceptance: vec![],
                        setup: vec![],
                        files: vec![],
                    },
                    StageDefinition {
                        id: "stage-2".to_string(),
                        name: "Stage Two".to_string(),
                        description: Some("Second stage".to_string()),
                        dependencies: vec!["stage-1".to_string()],
                        parallel_group: Some("group-a".to_string()),
                        acceptance: vec!["cargo test".to_string()],
                        setup: vec!["source .venv/bin/activate".to_string()],
                        files: vec!["src/*.rs".to_string()],
                    },
                ],
            },
        }
    }

    #[test]
    fn test_validate_valid_metadata() {
        let metadata = create_valid_metadata();
        assert!(validate(&metadata).is_ok());
    }

    #[test]
    fn test_validate_unsupported_version() {
        let mut metadata = create_valid_metadata();
        metadata.loom.version = 2;

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("Unsupported version"));
    }

    #[test]
    fn test_validate_empty_stages() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("No stages defined")));
    }

    #[test]
    fn test_validate_empty_stage_id() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![StageDefinition {
                    id: "".to_string(),
                    name: "Test".to_string(),
                    description: None,
                    dependencies: vec![],
                    parallel_group: None,
                    acceptance: vec![],
                    setup: vec![],
                    files: vec![],
                }],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("ID cannot be empty")));
    }

    #[test]
    fn test_validate_empty_stage_name() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![StageDefinition {
                    id: "stage-1".to_string(),
                    name: "".to_string(),
                    description: None,
                    dependencies: vec![],
                    parallel_group: None,
                    acceptance: vec![],
                    setup: vec![],
                    files: vec![],
                }],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("name cannot be empty")));
    }

    #[test]
    fn test_validate_unknown_dependency() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![StageDefinition {
                    id: "stage-1".to_string(),
                    name: "Stage One".to_string(),
                    description: None,
                    dependencies: vec!["nonexistent".to_string()],
                    parallel_group: None,
                    acceptance: vec![],
                    setup: vec![],
                    files: vec![],
                }],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("Unknown dependency")));
        assert!(errors.iter().any(|e| e.message.contains("nonexistent")));
    }

    #[test]
    fn test_validate_self_dependency() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![StageDefinition {
                    id: "stage-1".to_string(),
                    name: "Stage One".to_string(),
                    description: None,
                    dependencies: vec!["stage-1".to_string()],
                    parallel_group: None,
                    acceptance: vec![],
                    setup: vec![],
                    files: vec![],
                }],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("cannot depend on itself")));
    }

    #[test]
    fn test_validate_multiple_errors() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 2,
                stages: vec![
                    StageDefinition {
                        id: "".to_string(),
                        name: "".to_string(),
                        description: None,
                        dependencies: vec![],
                        parallel_group: None,
                        acceptance: vec![],
                        setup: vec![],
                        files: vec![],
                    },
                    StageDefinition {
                        id: "stage-2".to_string(),
                        name: "Stage Two".to_string(),
                        description: None,
                        dependencies: vec!["stage-2".to_string(), "nonexistent".to_string()],
                        parallel_group: None,
                        acceptance: vec![],
                        setup: vec![],
                        files: vec![],
                    },
                ],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        // Should have multiple errors: unsupported version, empty ID, empty name, self-dependency, unknown dependency
        assert!(errors.len() >= 4);
    }

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError {
            message: "Test error".to_string(),
            stage_id: Some("stage-1".to_string()),
        };
        assert_eq!(error.to_string(), "Stage 'stage-1': Test error");

        let error_no_stage = ValidationError {
            message: "General error".to_string(),
            stage_id: None,
        };
        assert_eq!(error_no_stage.to_string(), "General error");
    }

    #[test]
    fn test_stage_definition_serde_defaults() {
        let yaml = r#"
id: test-stage
name: Test Stage
"#;
        let stage: StageDefinition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(stage.id, "test-stage");
        assert_eq!(stage.name, "Test Stage");
        assert_eq!(stage.description, None);
        assert_eq!(stage.dependencies.len(), 0);
        assert_eq!(stage.parallel_group, None);
        assert_eq!(stage.acceptance.len(), 0);
        assert_eq!(stage.setup.len(), 0);
        assert_eq!(stage.files.len(), 0);
    }

    #[test]
    fn test_complex_dependency_chain() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![
                    StageDefinition {
                        id: "stage-1".to_string(),
                        name: "Stage 1".to_string(),
                        description: None,
                        dependencies: vec![],
                        parallel_group: None,
                        acceptance: vec![],
                        setup: vec![],
                        files: vec![],
                    },
                    StageDefinition {
                        id: "stage-2".to_string(),
                        name: "Stage 2".to_string(),
                        description: None,
                        dependencies: vec!["stage-1".to_string()],
                        parallel_group: None,
                        acceptance: vec![],
                        setup: vec![],
                        files: vec![],
                    },
                    StageDefinition {
                        id: "stage-3".to_string(),
                        name: "Stage 3".to_string(),
                        description: None,
                        dependencies: vec!["stage-1".to_string(), "stage-2".to_string()],
                        parallel_group: None,
                        acceptance: vec![],
                        setup: vec![],
                        files: vec![],
                    },
                ],
            },
        };

        assert!(validate(&metadata).is_ok());
    }

    #[test]
    fn test_validate_stage_id_path_traversal() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![StageDefinition {
                    id: "../etc/passwd".to_string(),
                    name: "Malicious Stage".to_string(),
                    description: None,
                    dependencies: vec![],
                    parallel_group: None,
                    acceptance: vec![],
                    setup: vec![],
                    files: vec![],
                }],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("Invalid stage ID")));
    }

    #[test]
    fn test_validate_stage_id_with_slashes() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![StageDefinition {
                    id: "stage/with/slashes".to_string(),
                    name: "Stage".to_string(),
                    description: None,
                    dependencies: vec![],
                    parallel_group: None,
                    acceptance: vec![],
                    setup: vec![],
                    files: vec![],
                }],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("invalid characters")));
    }

    #[test]
    fn test_validate_stage_id_with_dots() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![StageDefinition {
                    id: "stage.with.dots".to_string(),
                    name: "Stage".to_string(),
                    description: None,
                    dependencies: vec![],
                    parallel_group: None,
                    acceptance: vec![],
                    setup: vec![],
                    files: vec![],
                }],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("invalid characters")));
    }

    #[test]
    fn test_validate_stage_id_reserved_name_dotdot() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![StageDefinition {
                    id: "..".to_string(),
                    name: "Stage".to_string(),
                    description: None,
                    dependencies: vec![],
                    parallel_group: None,
                    acceptance: vec![],
                    setup: vec![],
                    files: vec![],
                }],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_stage_id_reserved_name_con() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![StageDefinition {
                    id: "CON".to_string(),
                    name: "Stage".to_string(),
                    description: None,
                    dependencies: vec![],
                    parallel_group: None,
                    acceptance: vec![],
                    setup: vec![],
                    files: vec![],
                }],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("reserved name")));
    }

    #[test]
    fn test_validate_dependency_id_path_traversal() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![StageDefinition {
                    id: "stage-1".to_string(),
                    name: "Stage One".to_string(),
                    description: None,
                    dependencies: vec!["../etc/passwd".to_string()],
                    parallel_group: None,
                    acceptance: vec![],
                    setup: vec![],
                    files: vec![],
                }],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("Invalid dependency ID")));
    }

    #[test]
    fn test_validate_stage_id_too_long() {
        let long_id = "a".repeat(129);
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![StageDefinition {
                    id: long_id,
                    name: "Stage".to_string(),
                    description: None,
                    dependencies: vec![],
                    parallel_group: None,
                    acceptance: vec![],
                    setup: vec![],
                    files: vec![],
                }],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.message.contains("too long")));
    }

    #[test]
    fn test_validate_stage_id_with_spaces() {
        let metadata = LoomMetadata {
            loom: LoomConfig {
                version: 1,
                stages: vec![StageDefinition {
                    id: "stage with spaces".to_string(),
                    name: "Stage".to_string(),
                    description: None,
                    dependencies: vec![],
                    parallel_group: None,
                    acceptance: vec![],
                    setup: vec![],
                    files: vec![],
                }],
            },
        };

        let result = validate(&metadata);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("invalid characters")));
    }
}
