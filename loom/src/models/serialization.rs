use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Trait for types that can be serialized to and from Markdown format.
///
/// This trait provides a unified interface for loading and saving domain objects
/// as Markdown files with YAML frontmatter. It enables consistent file I/O
/// patterns across different model types (Runner, Track, Signal, etc.).
pub trait MarkdownSerializable: Sized {
    /// Parse an instance from markdown content.
    ///
    /// # Arguments
    /// * `content` - The markdown string to parse, including frontmatter
    ///
    /// # Returns
    /// * `Ok(Self)` - Successfully parsed instance
    /// * `Err` - If parsing fails due to missing required fields or invalid format
    fn from_markdown(content: &str) -> Result<Self>;

    /// Serialize the instance to markdown format.
    ///
    /// The output includes YAML frontmatter delimited by `---` and
    /// structured markdown sections for human readability.
    ///
    /// # Returns
    /// * `Ok(String)` - The serialized markdown content
    /// * `Err` - If serialization fails
    fn to_markdown(&self) -> Result<String>;

    /// Load an instance from a file path.
    ///
    /// # Arguments
    /// * `path` - The path to the markdown file
    ///
    /// # Returns
    /// * `Ok(Self)` - Successfully loaded and parsed instance
    /// * `Err` - If file reading or parsing fails
    fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        Self::from_markdown(&content)
    }

    /// Save the instance to a file path.
    ///
    /// Creates parent directories if they don't exist.
    ///
    /// # Arguments
    /// * `path` - The destination path for the markdown file
    ///
    /// # Returns
    /// * `Ok(())` - Successfully written
    /// * `Err` - If writing fails
    fn save(&self, path: &Path) -> Result<()> {
        let content = self.to_markdown()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }

        fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }
}
