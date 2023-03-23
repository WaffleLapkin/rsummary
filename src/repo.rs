use std::{collections::HashMap, fmt::Write, fs, io};

use xshell::{cmd, Shell};

use crate::{
    parse::{parse_merge_commit, MergeCommit, ParseError},
    Also,
};

#[derive(Debug, Default)]
pub struct Repo {
    merges: Vec<MergeCommit>,
    approved_by_user: HashMap<String, Vec<usize>>,
    authored_by_user: HashMap<String, Vec<usize>>,
}

#[derive(Debug, derive_more::From)]
pub enum AnalyzeRepoError {
    Io(io::Error),
    Shell(xshell::Error),
    Parse(ParseError),
}

pub fn update_repo(sh: &Shell, user: &str, repo: &str) -> Result<(), AnalyzeRepoError> {
    // eg `./repos/rust-lang`
    let user_path = sh.current_dir().also(|p| p.push(user));
    fs::create_dir_all(&user_path)?;

    // eg `./repos/rust-lang/rust/.git`
    let dot_git = user_path.join(&format!("{repo}/.git"));

    if !dot_git.exists() {
        let _guard = sh.push_dir(user);

        cmd!(sh, "git clone https://github.com/{user}/{repo}.git").run()?;
    }

    let _guard = sh.push_dir(&format!("{user}/{repo}"));
    cmd!(sh, "git pull origin master").run()?;

    Ok(())
}

pub fn analyze_repo(sh: &Shell, user: &str, repo: &str) -> Result<Repo, AnalyzeRepoError> {
    let _guard = sh.push_dir(&format!("{user}/{repo}"));

    let log = cmd!(sh, "git log --pretty=%H::::%s::::%an").read()?;

    let mut repo = Repo::default();

    repo.merges = log
        .lines()
        // FIXME: for now we ignore all errors, it might make sense to record them somehow
        //        (although it's not _that_ important, since we log them via tracing)
        .filter_map(|line| parse_merge_commit(line).ok().flatten())
        .collect();

    for (idx, merge) in repo.merges.iter().enumerate() {
        for approved_by in &merge.approved_by {
            repo.approved_by_user
                .entry(approved_by.clone())
                .or_default()
                .push(idx);
        }

        repo.authored_by_user
            .entry(merge.author.clone())
            .or_default()
            .push(idx);
    }

    Ok(repo)
}

impl Repo {
    pub fn authored_by(&self, user: &str) -> impl Iterator<Item = &MergeCommit> {
        self.authored_by_user
            .get(user)
            .map(|indices| indices.iter().map(|&idx| &self.merges[idx]))
            .into_iter()
            .flatten()
    }

    pub fn authored_by_grouped(&self, user: &str) -> Vec<(String, Vec<&MergeCommit>)> {
        let mut grouped = HashMap::<_, Vec<_>>::new();

        for mc in self.authored_by(user) {
            for approved_by in &mc.approved_by {
                grouped.entry(approved_by.clone()).or_default().push(mc);
            }
        }

        grouped.into_iter().collect()
    }

    pub fn print_authored_by(&self, user: &str) -> Option<String> {
        print(self.authored_by_grouped(user), 'r')
    }

    pub fn approved_by(&self, user: &str) -> impl Iterator<Item = &MergeCommit> {
        self.approved_by_user
            .get(user)
            .map(|indices| indices.iter().map(|&idx| &self.merges[idx]))
            .into_iter()
            .flatten()
    }

    pub fn approved_by_grouped(&self, user: &str) -> Vec<(String, Vec<&MergeCommit>)> {
        let mut grouped = HashMap::<_, Vec<_>>::new();

        for mc in self.approved_by(user) {
            grouped.entry(mc.author.clone()).or_default().push(mc);
        }

        grouped.into_iter().collect()
    }

    pub fn print_approved_by(&self, user: &str) -> Option<String> {
        print(self.approved_by_grouped(user), 'a')
    }
}

fn print(mut grouped: Vec<(String, Vec<&MergeCommit>)>, mode: char) -> Option<String> {
    grouped.sort_unstable_by_key(|(_, mcs)| mcs.len());

    let (_, max) = grouped.last()?;
    let max_len = max.len().to_string().len();

    let mut res = String::new();

    for (user, mcs) in grouped.iter().rev() {
        let num = mcs.len();
        _ = writeln!(res, "{num:max_len$} {mode}={user}");
    }

    Some(res)
}
