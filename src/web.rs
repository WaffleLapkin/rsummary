use std::{collections::HashSet, net::SocketAddr, sync::Arc};

use axum::{
    extract::{Path, Query},
    routing::get,
    Extension, Router,
};

use crate::{repo_manager::RepoManager, FullRepoName};

use eyre::WrapErr;

pub async fn run(
    addr: &SocketAddr,
    manager: RepoManager,
    allow_list: HashSet<FullRepoName>,
) -> eyre::Result<()> {
    let router = Router::new()
        .route("/:user/:repo", get(root_user_repo_get))
        .layer(Extension(Arc::new(allow_list)))
        .layer(Extension(manager));

    axum::Server::bind(addr)
        .serve(router.into_make_service())
        .await
        .wrap_err("Failed to run web server")
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
enum Filter {
    AuthoredBy {
        #[serde(rename = "a")]
        authored_by: String,
    },
    ReviewedBy {
        #[serde(rename = "r")]
        approved_by: String,
    },
}

async fn root_user_repo_get(
    Path(repo): Path<FullRepoName>,
    Query(filter): Query<Filter>,
    allow_list: Extension<Arc<HashSet<FullRepoName>>>,
    manager: Extension<RepoManager>,
) -> String {
    // FIXME: print the errors in a better way/return an error HTTP code

    if !allow_list.contains(&repo) {
        return "err0".to_owned();
    }

    let Ok(repo) = manager.analyze_repo(repo).await else { return "err1".to_owned(); };

    match filter {
        Filter::AuthoredBy { authored_by } => repo.print_authored_by(&authored_by),
        Filter::ReviewedBy { approved_by } => repo.print_approved_by(&approved_by),
    }
    .unwrap_or_else(|| "err2".to_owned())
}
