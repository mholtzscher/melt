use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use tracing::warn;

use crate::error::GitError;
use crate::model::{ChangelogData, Commit, GitInput};

pub struct GithubClient<'a> {
    pub client: &'a Client,
    pub token: Option<&'a str>,
}

impl<'a> GithubClient<'a> {
    pub async fn check_updates(&self, input: &GitInput) -> Result<usize, GitError> {
        let branch = input.reference.as_deref().unwrap_or("HEAD");
        let url = format!(
            "https://api.github.com/repos/{}/{}/compare/{}...{}",
            input.owner, input.repo, input.rev, branch
        );

        let mut req = self.client.get(&url);
        if let Some(token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        let status = resp.status();

        if status.as_u16() == 403 || status.as_u16() == 429 {
            let remaining = resp
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);

            if remaining == 0 {
                warn!(input = %input.name, "GitHub API rate limit exceeded");
                return Err(GitError::NetworkError(
                    "GitHub API rate limit exceeded. Set GITHUB_TOKEN for higher limits."
                        .to_string(),
                ));
            }
        }

        if !status.is_success() {
            return Err(GitError::ApiFallbackRequired);
        }

        #[derive(Deserialize)]
        struct CompareResponse {
            ahead_by: usize,
        }

        let data: CompareResponse = resp
            .json()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        Ok(data.ahead_by)
    }

    pub async fn get_changelog(&self, input: &GitInput) -> Result<ChangelogData, GitError> {
        let branch = input.reference.as_deref().unwrap_or("HEAD");

        let url = format!(
            "https://api.github.com/repos/{}/{}/commits?sha={}&per_page=100",
            input.owner, input.repo, branch
        );

        let mut req = self.client.get(&url);
        if let Some(token) = self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }

        let resp = req
            .send()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        let status = resp.status();

        if status.as_u16() == 403 || status.as_u16() == 429 {
            let remaining = resp
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);

            if remaining == 0 {
                return Err(GitError::NetworkError(
                    "GitHub API rate limit exceeded. Set GITHUB_TOKEN for higher limits."
                        .to_string(),
                ));
            }
        }

        if !status.is_success() {
            return Err(GitError::ApiFallbackRequired);
        }

        #[derive(Deserialize)]
        struct GitHubAuthor {
            name: Option<String>,
            date: Option<String>,
        }

        #[derive(Deserialize)]
        struct GitHubCommitData {
            message: String,
            author: Option<GitHubAuthor>,
        }

        #[derive(Deserialize)]
        struct GitHubCommit {
            sha: String,
            commit: GitHubCommitData,
        }

        let commits: Vec<GitHubCommit> = resp
            .json()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        let mut result_commits = Vec::new();
        let mut locked_idx = None;

        for (idx, c) in commits.iter().enumerate() {
            let is_locked = c.sha.starts_with(&input.rev) || c.sha == input.rev;
            if is_locked {
                locked_idx = Some(idx);
            }

            let date = c
                .commit
                .author
                .as_ref()
                .and_then(|a| a.date.as_ref())
                .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let author = c
                .commit
                .author
                .as_ref()
                .and_then(|a| a.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            let message = c.commit.message.lines().next().unwrap_or("").to_string();

            result_commits.push(Commit {
                sha: c.sha.clone(),
                message,
                author,
                date,
                is_locked,
            });
        }

        Ok(ChangelogData {
            commits: result_commits,
            locked_idx,
        })
    }
}
