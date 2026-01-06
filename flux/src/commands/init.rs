use anyhow::Result;
use crate::fs::work_dir::WorkDir;

/// Initialize the .work/ directory structure
pub fn execute() -> Result<()> {
    let work_dir = WorkDir::new(".")?;
    work_dir.initialize()?;

    println!("Initialized .work/ directory structure");
    Ok(())
}
