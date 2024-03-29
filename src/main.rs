use core::Project;
use std::path::Path;

use anyhow::{anyhow, Result};
use clap::Parser;

mod cli;
mod core;
mod ui;
mod widgets;

fn main() -> Result<()> {
    let args = cli::CLIArgs::parse();

    if let Some(branch) = args.checkout {
        let proj = cur_project()?;
        let git = core::Git::new(proj.path);
        git.checkout(branch.as_str())?;
    } else if let Some(branch) = args.add {
        let proj = cur_project()?;
        core::add_branch(proj.path.as_str(), branch)?;
    } else if let Some(branch) = args.remove {
        let proj = cur_project()?;
        core::remove_branch(proj.path.as_str(), branch)?;
    } else if let Some(project) = args.add_project {
        let path = Path::new(project.as_str());
        core::add_project(path.canonicalize()?.to_str().ok_or(anyhow!("bad path"))?)?;
    } else if let Some(project) = args.remove_project {
        core::remove_project(project.as_str())?;
    } else if args.list {
        core::list_projects()?;
    } else {
        let proj = cur_project()?;
        let git: core::Git = core::Git::new(proj.path.clone());
        ui::start_ui(proj, git)?;
    }

    Ok(())
}

fn cur_project() -> Result<Project> {
    let cwd = std::env::current_dir()?;
    let proj = core::get_project_from_path(cwd.as_path())?;
    Ok(proj)
}
