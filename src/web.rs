use std::{collections::HashSet, net::SocketAddr, sync::Arc};

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::get,
    Extension, Router,
};

use crate::{repo_manager::RepoManager, RepoId};

use eyre::WrapErr;

pub async fn run(
    addr: &SocketAddr,
    manager: RepoManager,
    allow_list: HashSet<RepoId>,
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
    Path(repo_id): Path<RepoId>,
    Query(filter): Query<Filter>,
    allow_list: Extension<Arc<HashSet<RepoId>>>,
    manager: Extension<RepoManager>,
) -> Result<String, (StatusCode, String)> {
    if !allow_list.contains(&repo_id) {
        return Err((
            StatusCode::FORBIDDEN,
            format!("Repository `{repo_id}` is not in the allow list"),
        ));
    }

    let repo = manager.analyze_repo(repo_id.clone()).await.map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Internal error while analyzing `{repo_id}`: {err:?}"),
        )
    })?;

    let res = match &filter {
        Filter::AuthoredBy { authored_by } => {
            repo.print_authored_by(authored_by).unwrap_or_else(|| {
                format!("{authored_by} does not have any pull requests merged into `{repo_id}`")
            })
        }
        Filter::ReviewedBy { approved_by } => {
            repo.print_approved_by(approved_by).unwrap_or_else(|| {
                format!("{approved_by} haven't approved any pull requests in `{repo_id}`")
            })
        }
    };

    Ok(res)
}
