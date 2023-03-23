use std::{collections::HashSet, fs};

use eyre::Context;
use xshell::Shell;

use crate::repo_manager::RepoManager;

mod parse;
mod repo;
mod repo_manager;
mod web;

#[tokio::main(flavor = "current_thread")]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let fmt_subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(fmt_subscriber)
        .wrap_err("Failed to set tracing default subscriber")?;

    fs::create_dir_all("./repos").wrap_err("Failed to create directory \"./repos\"")?;

    let sh = Shell::new().wrap_err("Failed to obtain shell")?;
    sh.change_dir("./repos");

    // FIXME: read to config
    let addr = "0.0.0.0:3000".parse().unwrap();

    let manager = RepoManager::spawn(sh);
    let allow_list = HashSet::from([FullRepoName {
        user: "rust-lang".to_owned(),
        repo: "rust".to_owned(),
    }]);

    web::run(&addr, manager, allow_list).await?;

    Ok(())
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize)]
#[serde(from = "(String, String)")]
pub struct FullRepoName {
    pub user: String,
    pub repo: String,
}

impl From<(String, String)> for FullRepoName {
    fn from((user, repo): (String, String)) -> Self {
        Self { user, repo }
    }
}

pub(crate) trait Also: Sized {
    fn also(mut self, f: impl FnOnce(&mut Self)) -> Self {
        f(&mut self);
        self
    }
}

impl<T> Also for T {}
