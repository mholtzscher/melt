use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;

use crate::error::GitError;
use crate::model::{ChangelogData, Commit, GitInput};

pub struct GitlabClient<'a> {
    pub client: &'a Client,
}

/// Simple URL encoding for project paths
fn urlencoding(s: &str) -> String {
    s.replace('/', "%2F")
}

impl<'a> GitlabClient<'a> {
    pub async fn check_updates(&self, input: &GitInput) -> Result<usize, GitError> {
        let host = input.host.as_deref().unwrap_or("gitlab.com");
        let branch = input.reference.as_deref().unwrap_or("HEAD");
        let project = format!("{}/{}", input.owner, input.repo);
        let encoded_project = urlencoding(&project);

        let url = format!(
            "https://{}/api/v4/projects/{}/repository/compare?from={}&to={}",
            host, encoded_project, input.rev, branch
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(GitError::ApiFallbackRequired);
        }

        #[derive(Deserialize)]
        struct CompareResponse {
            commits: Vec<serde_json::Value>,
        }

        let data: CompareResponse = resp
            .json()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        Ok(data.commits.len())
    }

    pub async fn get_changelog(&self, input: &GitInput) -> Result<ChangelogData, GitError> {
        let host = input.host.as_deref().unwrap_or("gitlab.com");
        let branch = input.reference.as_deref().unwrap_or("HEAD");

        let project = format!("{}/{}", input.owner, input.repo);
        let encoded_project = urlencoding(&project);

        let url = format!(
            "https://{}/api/v4/projects/{}/repository/commits?ref_name={}&per_page=100",
            host, encoded_project, branch
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(GitError::ApiFallbackRequired);
        }

        #[derive(Deserialize)]
        struct GitLabCommit {
            id: String,
            title: String,
            author_name: String,
            created_at: String,
        }

        let commits: Vec<GitLabCommit> = resp
            .json()
            .await
            .map_err(|e| GitError::NetworkError(e.to_string()))?;

        let mut result_commits = Vec::new();
        let mut locked_idx = None;

        for (idx, c) in commits.iter().enumerate() {
            let is_locked = c.id.starts_with(&input.rev) || c.id == input.rev;
            if is_locked {
                locked_idx = Some(idx);
            }

            let date = chrono::DateTime::parse_from_rfc3339(&c.created_at)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            result_commits.push(Commit {
                sha: c.id.clone(),
                message: c.title.clone(),
                author: c.author_name.clone(),
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
