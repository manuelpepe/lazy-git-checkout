use std::path::Path;

use anyhow::{anyhow, Result};
use clap::Parser;

mod cli;
mod core;

fn main() -> Result<()> {
    let args = cli::CLIArgs::parse();

    if let Some(branch) = args.checkout {
        let cwd = std::env::current_dir()?;
        let proj = core::get_project_from_path(cwd.as_path())?;
        core::checkout(proj.path.as_str(), branch.as_str())?;
    } else if let Some(branch) = args.add {
        let cwd = std::env::current_dir()?;
        let proj = core::get_project_from_path(cwd.as_path())?;
        core::add_branch(proj.path.as_str(), branch)?;
    } else if let Some(branch) = args.remove {
        let cwd = std::env::current_dir()?;
        let proj = core::get_project_from_path(cwd.as_path())?;
        core::remove_branch(proj.path.as_str(), branch)?;
    } else if let Some(project) = args.add_project {
        let path = Path::new(project.as_str());
        core::add_project(path.canonicalize()?.to_str().ok_or(anyhow!("bad path"))?)?;
    } else if let Some(project) = args.remove_project {
        core::remove_project(project.as_str())?;
    } else if args.list {
        core::list_projects()?;
    } else {
        println!("No args. Running UI.");
    }

    Ok(())
}
