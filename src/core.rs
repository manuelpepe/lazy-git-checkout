use std::{io::Write, process::Output, vec};

use anyhow::{anyhow, Result};

const DB_PATH: &str = "/etc/lazy-git-checkout.db.txt";
const PROJECT_PATH_DELIMITER: &str = ";;;;";

#[derive(Debug, Clone)]
pub struct Branch {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub path: String,
    pub branches: Vec<Branch>,
}

impl Project {
    pub fn new(path: String) -> Project {
        Project {
            path,
            branches: Vec::new(),
        }
    }

    fn add_branch(&mut self, branch: String) {
        self.branches.push(Branch { name: branch });
    }

    fn remove_branch(&mut self, branch: String) {
        self.branches.retain(|b| b.name != branch);
    }
}

#[derive(Debug, Clone)]
struct DB {
    projects: Vec<Project>,
}

impl DB {
    fn new() -> DB {
        DB {
            projects: Vec::new(),
        }
    }

    fn add_project(&mut self, project: Project) {
        self.projects.push(project);
    }

    fn remove_project(&mut self, path: &str) {
        self.projects.retain(|p| p.path != path);
    }

    fn get_project_mut(&mut self, path: &str) -> Option<&mut Project> {
        self.projects.iter_mut().find(|p| p.path == path)
    }

    pub fn write_to_disk(&self) -> Result<()> {
        let mut file = std::fs::File::create(DB_PATH)?;
        for project in &self.projects {
            file.write_all(format!("{}{}\n", PROJECT_PATH_DELIMITER, project.path).as_bytes())?;
            for branch in &project.branches {
                file.write_all(format!("{}\n", branch.name).as_bytes())?;
            }
        }
        Ok(())
    }

    fn read_db_file() -> Result<String> {
        let file = std::fs::read_to_string(DB_PATH);
        if let Err(e) = file {
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(String::new()); // if file is not found return empty and wait for write later
            } else {
                return Err(anyhow!(e));
            }
        }
        Ok(file.unwrap())
    }

    pub fn load_from_disk() -> Result<Self> {
        let file = DB::read_db_file()?;
        let lines = file.lines();
        let mut path = "";
        let mut db = DB::new();
        for line in lines {
            if line.starts_with(PROJECT_PATH_DELIMITER) {
                path = line.trim_start_matches(PROJECT_PATH_DELIMITER);
                db.add_project(Project::new(path.to_string()));
            } else if !path.is_empty() {
                let branch = line.to_string();
                let project = db.get_project_mut(path);
                if project.is_none() {
                    panic!("Invalid file format");
                }
                project.unwrap().add_branch(branch);
            } else {
                panic!("Invalid file format");
            }
        }
        Ok(db)
    }
}

pub struct Git {
    pub path: String,
}

impl Git {
    pub fn new(path: String) -> Git {
        Git { path }
    }

    pub fn checkout(&self, branch: &str) -> Result<()> {
        let cur_branch = self.get_current_branch()?;
        let stash_name = format!("lazy-git-checkout:{}", cur_branch);
        println!("> stashing...");
        self.stream_git_command(vec!["stash", "-m", stash_name.as_str()])?;
        println!("> checkout...");
        self.stream_git_command(vec!["checkout", branch])?;
        let last_stashed = self.get_last_stashed(branch);
        if let Some(last_stashed) = last_stashed {
            println!("> popping stash...");
            self.stream_git_command(vec!["stash", "pop", last_stashed.as_ref()])?;
        }
        Ok(())
    }

    pub fn all_project_branches(&self) -> Result<Vec<String>> {
        let output = self.run_git_command(vec!["branch", "-a"])?;
        let branches = String::from_utf8(output.stdout)?;
        let branches = branches.split('\n');
        let branches = branches
            .map(|b| b.trim())
            .filter(|b| !b.is_empty())
            .map(|b| b.trim_start_matches('*'))
            .map(|b| b.trim())
            .map(|b| b.to_string())
            .collect::<Vec<String>>();
        Ok(branches)
    }

    pub fn get_current_branch(&self) -> Result<String> {
        let output = self.run_git_command(vec!["rev-parse", "--abbrev-ref", "HEAD"])?;
        let branch = String::from_utf8(output.stdout)?;
        Ok(branch.trim().to_string())
    }

    fn run_git_command(&self, command: Vec<&str>) -> Result<Output> {
        let output = std::process::Command::new("git")
            .args(command)
            .current_dir(self.path.as_str())
            .output()?;
        if !output.status.success() {
            let error = String::from_utf8(output.stderr)?;
            return Err(anyhow::anyhow!(error));
        }
        Ok(output)
    }

    fn stream_git_command(&self, command: Vec<&str>) -> Result<()> {
        std::process::Command::new("git")
            .args(command)
            .current_dir(self.path.as_str())
            .spawn()?
            .wait()?;
        Ok(())
    }

    fn get_last_stashed(&self, branch: &str) -> Option<String> {
        let output = self.run_git_command(vec!["stash", "list"]).unwrap();
        let stashes = String::from_utf8(output.stdout).unwrap();
        let stashes = stashes.split('\n');
        let stash_name = format!("lazy-git-checkout:{}", branch);
        let stashes = stashes.filter(|s| s.ends_with(stash_name.as_str()));
        let stashes = stashes.collect::<Vec<&str>>();
        let last_stash = stashes.first()?;
        let last_stash = last_stash.split(':').collect::<Vec<&str>>();
        Some(last_stash[0].to_string())
    }
}

pub fn add_project(path: &str) -> Result<()> {
    let mut db = DB::load_from_disk()?;
    db.add_project(Project::new(path.to_string()));
    db.write_to_disk()?;
    Ok(())
}

pub fn remove_project(path: &str) -> Result<()> {
    let mut db = DB::load_from_disk()?;
    db.remove_project(path);
    db.write_to_disk()?;
    Ok(())
}

pub fn add_branch(path: &str, branch: String) -> Result<()> {
    let mut db = DB::load_from_disk()?;
    db.get_project_mut(path)
        .ok_or(anyhow!("no project found"))?
        .add_branch(branch);
    db.write_to_disk()?;
    Ok(())
}

pub fn remove_branch(path: &str, branch: String) -> Result<()> {
    let mut db = DB::load_from_disk()?;
    db.get_project_mut(path)
        .ok_or(anyhow!("no project found"))?
        .remove_branch(branch);
    db.write_to_disk()?;
    Ok(())
}

pub fn get_branches(path: &str) -> Result<Vec<Branch>> {
    let db = DB::load_from_disk()?;
    let project = db.projects.iter().find(|p| path == p.path.as_str());
    if let Some(project) = project {
        Ok(project.branches.clone())
    } else {
        Err(anyhow!("no project found in path"))
    }
}

pub fn set_branches(path: &str, branches: Vec<&str>) -> Result<()> {
    let mut db = DB::load_from_disk()?;
    let project = db.projects.iter_mut().find(|p| path == p.path.as_str());
    if let Some(project) = project {
        project.branches = branches
            .iter()
            .map(|b| Branch {
                name: b.to_string(),
            })
            .collect::<Vec<Branch>>();
    } else {
        return Err(anyhow!("no project found in path"));
    }
    db.write_to_disk()?;
    Ok(())
}

pub fn list_projects() -> Result<()> {
    let db = DB::load_from_disk()?;
    for project in &db.projects {
        println!("{}", project.path);
        for branch in &project.branches {
            println!("  {}", branch.name);
        }
    }
    Ok(())
}

// returns the first project that matches with the path.
// the path passed can be a subdirectory of a projects path.
// for example:
// if the project path is /home/user/project
// and the path passed is /home/user/project/src/mod/a/b/c
// the project will be returned.
pub fn get_project_from_path(path: &std::path::Path) -> Result<Project> {
    let db = DB::load_from_disk()?;
    let path = path.canonicalize()?;
    let path = path.to_str().unwrap();
    let project = db
        .projects
        .iter()
        .find(|p| path.starts_with(p.path.as_str()));
    if let Some(proj) = project {
        Ok(proj.clone())
    } else {
        Err(anyhow!("no project found in path"))
    }
}
