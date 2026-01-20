//! Skill routing module for automated skill recommendations in signals
//!
//! This module provides functionality to:
//! - Load skill metadata from SKILL.md files in ~/.claude/skills/
//! - Build an inverted index of trigger keywords
//! - Match stage descriptions against triggers to recommend relevant skills
//!
//! # Example
//!
//! ```ignore
//! use loom::skills::SkillIndex;
//! use std::path::Path;
//!
//! let index = SkillIndex::load_from_directory(Path::new("~/.claude/skills/"))?;
//! let matches = index.match_skills("implement OAuth login flow", 5);
//!
//! for skill in matches {
//!     println!("Recommended: {} (score: {})", skill.name, skill.score);
//! }
//! ```

mod index;
mod matcher;
mod types;

pub use index::SkillIndex;
pub use types::{SkillMatch, SkillMetadata};
