use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about)]
pub struct CLIArgs {
    /// Add branch to checkout to
    #[clap(short, long)]
    pub add: Option<String>,

    /// Remove a branch from saved branches
    #[clap(short, long)]
    pub remove: Option<String>,

    /// Add a project
    #[clap(short = 'A', long)]
    pub add_project: Option<String>,

    /// Remove a project
    #[clap(short = 'R', long)]
    pub remove_project: Option<String>,

    /// List all projects
    #[clap(short, long)]
    pub list: bool,

    /// Checkout with stash
    #[clap(short, long)]
    pub checkout: Option<String>,
}
