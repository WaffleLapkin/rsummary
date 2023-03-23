use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::{mpsc, oneshot};
use xshell::Shell;

use crate::{
    repo::{analyze_repo, update_repo, AnalyzeRepoError, Repo},
    FullRepoName,
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

    pub async fn analyze_repo(&self, repo: FullRepoName) -> Result<Arc<Repo>, AnalyzeRepoError> {
        let (send, recv) = oneshot::channel();

        self.ch
            .send(Request::Analyze { repo, ret: send })
            .await
            .ok()
            .expect("The worker died");

        recv.await.expect("The worker died")
    }
}

enum Request {
    Analyze {
        repo: FullRepoName,
        ret: oneshot::Sender<Result<Arc<Repo>, AnalyzeRepoError>>,
    },
}

async fn worker(sh: Shell, mut jobs: mpsc::Receiver<Request>) {
    let mut cache = HashMap::<FullRepoName, (Arc<Repo>, Instant)>::new();

    while let Some(job) = jobs.recv().await {
        match job {
            Request::Analyze {
                repo: repo_name,
                ret,
            } if cache
                .get(&repo_name)
                .map_or(false, |(_, t)| t.elapsed() < Duration::from_secs(60 * 5)) =>
            {
                _ = ret.send(Ok(Arc::clone(&cache[&repo_name].0)));
            }
            Request::Analyze { repo, ret } => {
                let res = update_repo(&sh, &repo.user, &repo.repo)
                    .and_then(|()| analyze_repo(&sh, &repo.user, &repo.repo));

                let res = match res {
                    Ok(res) => {
                        let res = Arc::new(res);
                        let res2 = Arc::clone(&res);

                        cache.insert(repo, (res, Instant::now()));
                        Ok(res2)
                    }
                    Err(err) => Err(err),
                };

                _ = ret.send(res);
            }
        }
    }
}
