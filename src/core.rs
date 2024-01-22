use std::{io::Write, process::Output, vec};

use anyhow::{anyhow, Error, Result};

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

#[derive(Debug)]
pub struct Git {
    pub path: String,
    tx: Option<tokio::sync::mpsc::Sender<GitCommand>>,
    checkout_rx: Option<tokio::sync::mpsc::Receiver<GitResponse>>,
    checkout_tx: Option<tokio::sync::mpsc::Sender<GitResponse>>,
}

impl Git {
    pub async fn new(path: String) -> Result<Git> {
        let mut git = Git {
            path,
            tx: None,
            checkout_rx: None,
            checkout_tx: None,
        };
        git.spawn_worker().await?;
        Ok(git)
    }

    pub async fn spawn_worker(&mut self) -> Result<()> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        self.tx = Some(tx);
        let mut worker = GitWorker {
            path: self.path.clone(),
            rx,
        };
        tokio::spawn(async move {
            worker.run().await;
        });
        Ok(())
    }

    pub async fn all_project_branches(&self) -> Result<Vec<String>> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx
            .as_ref()
            .ok_or(anyhow!("worker not spawned"))?
            .send(GitCommand::AllProjectBranches(tx))
            .await?;
        match rx.await {
            Ok(GitResponse::AllProjectBranches(branches)) => branches,
            _ => Err(anyhow!("invalid response")),
        }
    }

    pub async fn checkout(&mut self, branch: &str) -> Result<()> {
        if self.checkout_rx.is_some() {
            return Err(anyhow!("checkout already in progress"));
        }
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        self.checkout_rx = Some(rx);
        self.checkout_tx = Some(tx.clone());
        self.tx
            .as_ref()
            .ok_or(anyhow!("worker not spawned"))?
            .send(GitCommand::Checkout(tx.clone(), branch.to_string()))
            .await?;
        // TODO: Try to create loop that always rx.recv() and write results to mutexed vec.
        //       Then make poll_checkout_status() check the mutexed vec and copy data to
        //       a non-mutexed vec that can be quickly read by the UI.
        Ok(())
    }

    pub async fn poll_checkout_status(&mut self) -> Result<Option<CheckoutStatus>> {
        let rx = match self.checkout_rx.as_mut() {
            Some(rx) => rx,
            None => return Ok(None),
        };

        let status = rx.recv().await;
        Ok(match status {
            Some(GitResponse::Checkout(status)) => match status {
                CheckoutStatus::Progress(_) => Some(status),
                CheckoutStatus::Done => {
                    self.checkout_rx = None;
                    self.checkout_tx = None;
                    Some(status)
                }
                CheckoutStatus::Failed(_) => {
                    self.checkout_rx = None;
                    self.checkout_tx = None;
                    Some(status)
                }
            },
            _ => None,
        })
    }

    pub async fn get_current_branch(&self) -> Result<String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.tx
            .as_ref()
            .ok_or(anyhow!("worker not spawned"))?
            .send(GitCommand::GetCurrentBranch(tx))
            .await?;
        match rx.await {
            Ok(GitResponse::GetCurrentBranch(result)) => result,
            _ => Err(anyhow!("invalid response")),
        }
    }
}

#[derive(Debug)]
enum GitCommand {
    Checkout(tokio::sync::mpsc::Sender<GitResponse>, String),
    AllProjectBranches(tokio::sync::oneshot::Sender<GitResponse>),
    GetCurrentBranch(tokio::sync::oneshot::Sender<GitResponse>),
}

#[derive(Debug)]
pub enum CheckoutStatus {
    Progress(String),
    Done,
    Failed(Error),
}

#[derive(Debug)]
enum GitResponse {
    Checkout(CheckoutStatus),
    AllProjectBranches(Result<Vec<String>>),
    GetCurrentBranch(Result<String>),
}

#[derive(Debug)]
pub struct GitWorker {
    path: String,
    rx: tokio::sync::mpsc::Receiver<GitCommand>,
}

impl GitWorker {
    pub async fn run(&mut self) {
        while let Some(command) = self.rx.recv().await {
            match command {
                GitCommand::Checkout(tx, branch) => {
                    tx.try_send(GitResponse::Checkout(CheckoutStatus::Progress(
                        "Stashing current changes...".to_string(),
                    )))
                    .unwrap();
                    self.checkout(tx, branch.as_str()).await
                }
                GitCommand::AllProjectBranches(tx) => self.all_project_branches(tx).await,
                GitCommand::GetCurrentBranch(tx) => self.send_current_branch(tx).await,
            }
        }
    }

    async fn checkout(&self, tx: tokio::sync::mpsc::Sender<GitResponse>, branch: &str) {
        let cur_branch = match self.get_current_branch().await {
            Ok(branch) => branch,
            Err(e) => {
                tx.send(GitResponse::Checkout(CheckoutStatus::Failed(e)))
                    .await
                    .unwrap();
                return;
            }
        };

        let stash_name = format!("lazy-git-checkout:{}", cur_branch);
        let stash_name = stash_name.as_str();

        let res = self.run_git_command(vec!["stash", "-m", stash_name]).await;
        self.send_output_or_err(tx.clone(), res).await;

        let res = self.run_git_command(vec!["checkout", branch]).await;
        self.send_output_or_err(tx.clone(), res).await;

        let last_stashed = self.get_last_stashed(branch).await;
        if let Some(last_stashed) = last_stashed {
            let last_stashed = last_stashed.as_ref();
            let res = self
                .run_git_command(vec!["stash", "pop", last_stashed])
                .await;
            self.send_output_or_err(tx.clone(), res).await;
        }

        tx.send(GitResponse::Checkout(CheckoutStatus::Done))
            .await
            .unwrap();
    }

    async fn send_output_or_err(
        &self,
        tx: tokio::sync::mpsc::Sender<GitResponse>,
        output: Result<Output>,
    ) {
        match output {
            Ok(output) => {
                let data = match String::from_utf8(output.stdout) {
                    Ok(data) => data,
                    Err(e) => {
                        let e = anyhow!(e);
                        tx.send(GitResponse::Checkout(CheckoutStatus::Failed(e)))
                            .await
                            .unwrap();
                        return;
                    }
                };
                tx.try_send(GitResponse::Checkout(CheckoutStatus::Progress(data)))
                    .unwrap();
            }
            Err(e) => {
                tx.send(GitResponse::Checkout(CheckoutStatus::Failed(e)))
                    .await
                    .unwrap();
            }
        }
    }

    async fn all_project_branches(&self, tx: tokio::sync::oneshot::Sender<GitResponse>) {
        let output = match self.run_git_command(vec!["branch", "-a"]).await {
            Ok(output) => output,
            Err(e) => {
                tx.send(GitResponse::AllProjectBranches(Err(e))).unwrap();
                return;
            }
        };
        let branches = match String::from_utf8(output.stdout) {
            Ok(branches) => branches,
            Err(e) => {
                tx.send(GitResponse::AllProjectBranches(Err(anyhow!(e))))
                    .unwrap();
                return;
            }
        };
        let branches = branches.split('\n');
        let branches = branches
            .map(|b| b.trim())
            .filter(|b| !b.is_empty())
            .map(|b| b.trim_start_matches('*'))
            .map(|b| b.trim())
            .map(|b| b.to_string())
            .collect::<Vec<String>>();
        tx.send(GitResponse::AllProjectBranches(Ok(branches)))
            .unwrap();
    }

    async fn send_current_branch(&self, tx: tokio::sync::oneshot::Sender<GitResponse>) {
        let res = self.get_current_branch().await;
        tx.send(GitResponse::GetCurrentBranch(res)).unwrap();
    }

    async fn get_current_branch(&self) -> Result<String> {
        let output = self
            .run_git_command(vec!["rev-parse", "--abbrev-ref", "HEAD"])
            .await?;
        let branch = String::from_utf8(output.stdout)?;
        Ok(branch.trim().to_string())
    }

    async fn run_git_command(&self, command: Vec<&str>) -> Result<Output> {
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

    async fn get_last_stashed(&self, branch: &str) -> Option<String> {
        let output = self.run_git_command(vec!["stash", "list"]).await.unwrap();
        let stashes = String::from_utf8(output.stdout).unwrap();
        let stashes = stashes.split('\n');
        let stash_name = format!("lazy-git-checkout:{}", branch);
        let stashes = stashes.filter(|s| s.contains(stash_name.as_str()));
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
