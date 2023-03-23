use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct MergeCommit {
    pub hash: String,
    pub author: String,
    pub branch: String,
    pub approved_by: Vec<String>,
    pub pr: u32,
    pub rollup_by: Option<String>,
}

#[derive(Debug, derive_more::Display)]
pub enum ParseError {
    // FIXME: document errors here
    NoPr,
    NoDash,
    NoAuthorAndBranch,
    NoR,
    InvalidAuthorAndBranch,
    InvalidPrNumber,
    NoDelimiters,
}

/// Parses output of `git log --pretty="%H::::%s::::%an"` as a merge commit description in `@bors` managed repositories.
///
/// An example of `git log --pretty="%H::::%s::::%an"` output:
/// ```text
/// f77bfb7336f21bfe6a5fb5f7358d4406e2597289::::Auto merge of #108620 - Dylan-DPC:rollup-o5c4evy, r=Dylan-DPC::::bors
/// 02e4eefd88a55776cbb163c1ba025f0736e52026::::Rollup merge of #108605 - JohnTitor:issue-105821, r=compiler-errors::::Dylan DPC
/// ```
#[tracing::instrument(err)]
pub fn parse_merge_commit(source: &str) -> Result<Option<MergeCommit>, ParseError> {
    let &[hash, subject, commit_author] = source.split("::::").collect::<Vec<_>>().deref() else { return Err(ParseError::NoDelimiters) };

    let Some(Subject { author_and_branch, pr, r, is_rollup }) = parse_subject(subject)? else { return Ok(None) };

    let rollup_by = match (is_rollup, commit_author) {
        (true, by) => Some(by.to_owned()),
        (false, "bors") => None,
        (false, _) => return Ok(None),
    };

    let (author, branch) = author_and_branch
        .trim_end_matches(',')
        .split_once(':')
        .ok_or(ParseError::InvalidAuthorAndBranch)?;

    let pr = pr
        .strip_prefix('#')
        .and_then(|n| n.parse().ok())
        .ok_or(ParseError::InvalidPrNumber)?;

    let reviewers = r
        .strip_prefix("r=")
        .ok_or(ParseError::NoR)?
        .split(',')
        .map(<_>::to_owned)
        .collect();

    let res = MergeCommit {
        hash: hash.to_owned(),
        author: author.to_owned(),
        branch: branch.to_owned(),
        approved_by: reviewers,
        pr,
        rollup_by,
    };

    Ok(Some(res))
}

/// Minimally parsed subject line of a `@bors` managed repo
struct Subject<'a> {
    /// `author:branch,`
    author_and_branch: &'a str,
    /// `#12345`
    pr: &'a str,
    /// `r=...`
    r: &'a str,
    /// `true` if the message starts with `Rollup merge of `
    is_rollup: bool,
}

#[tracing::instrument(err)]
fn parse_subject(mut subject: &str) -> Result<Option<Subject>, ParseError> {
    let is_rollup;

    if let Some(s) = subject.strip_prefix("Rollup merge of ") {
        subject = s;
        is_rollup = true;
    } else if let Some(s) = subject.strip_prefix("Auto merge of ") {
        subject = s;
        is_rollup = false;
    } else {
        return Ok(None);
    }

    let mut split = subject.split(" ");

    let pr = split.next().ok_or(ParseError::NoPr)?;
    let _ = split
        .next()
        .filter(|&s| s == "-")
        .ok_or(ParseError::NoDash)?;
    let author_and_branch = split.next().ok_or(ParseError::NoAuthorAndBranch)?;
    let r = split.next().ok_or(ParseError::NoR)?;

    let res = Subject {
        author_and_branch,
        pr,
        r,
        is_rollup,
    };
    Ok(Some(res))
}
