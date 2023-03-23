use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::{mpsc, oneshot};
use xshell::Shell;

use crate::{
    repo::{analyze_repo, update_repo, AnalyzeRepoError, Repo},
    Inspect, RepoId,
};

#[derive(Clone)]
pub struct RepoManager {
    ch: mpsc::Sender<Request>,
}

impl RepoManager {
    pub fn spawn(shell: Shell) -> Self {
        // FIXME: figure out channel capacity
        let (send, recv) = mpsc::channel(42);

        tokio::spawn(worker(shell, recv));

        Self { ch: send }
    }

    pub async fn analyze_repo(&self, repo: RepoId) -> Result<Arc<Repo>, AnalyzeRepoError> {
        let (send, recv) = oneshot::channel();

        self.ch
            .send(Request::Analyze {
                repo_id: repo,
                ret: send,
            })
            .await
            .ok()
            .expect("The worker died");

        recv.await.expect("The worker died")
    }
}

enum Request {
    Analyze {
        repo_id: RepoId,
        ret: oneshot::Sender<Result<Arc<Repo>, AnalyzeRepoError>>,
    },
}

async fn worker(sh: Shell, mut jobs: mpsc::Receiver<Request>) {
    let mut cache = HashMap::<RepoId, (Arc<Repo>, Instant)>::new();

    while let Some(job) = jobs.recv().await {
        match job {
            Request::Analyze { repo_id, ret }
                if cache
                    .get(&repo_id)
                    .map_or(false, |(_, t)| t.elapsed() < Duration::from_secs(60 * 5)) =>
            {
                _ = ret.send(Ok(Arc::clone(&cache[&repo_id].0)));
            }
            Request::Analyze { repo_id, ret } => {
                let res = update_repo(&sh, &repo_id.user, &repo_id.repo)
                    .and_then(|()| analyze_repo(&sh, &repo_id.user, &repo_id.repo))
                    .map(Arc::new)
                    .inspect_(|repo| {
                        cache.insert(repo_id, (Arc::clone(&repo), Instant::now()));
                    });

                _ = ret.send(res);
            }
        }
    }
}
