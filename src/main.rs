use std::fs;

use eyre::Context;
use xshell::Shell;

use crate::repo_manager::RepoManager;

mod config;
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

    let config = config::config().wrap_err("Failed to obtain config")?;

    let allow_list = config
        .allowed
        .into_iter()
        .map(|allowed| RepoId {
            repo: allowed.repo,
            user: allowed.user,
        })
        .collect();

    let manager = RepoManager::spawn(sh, config.cache_timeout.0);

    web::run(&config.addr, manager, allow_list).await?;

    Ok(())
}

/// (string) Identifier of a certain repository in GitHub.
///
/// For example `rust-lang/rust` (`RepoId { user: "rust-lang", repo: "rust" }`).
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize)]
#[serde(from = "(String, String)")]
pub struct RepoId {
    pub user: String,
    pub repo: String,
}

impl From<(String, String)> for RepoId {
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

pub(crate) trait Inspect {
    type Item;

    /// <https://github.com/rust-lang/rust/issues/91345> :(
    fn inspect_(self, f: impl FnOnce(&mut Self::Item)) -> Self;
}

impl<T, E> Inspect for Result<T, E> {
    type Item = T;

    fn inspect_(mut self, f: impl FnOnce(&mut Self::Item)) -> Self {
        if let Ok(v) = &mut self {
            f(v);
        }

        self
    }
}
