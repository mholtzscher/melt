import type { GitHubCommit, FlakeInput } from "./types";

interface GitHubAPICommit {
  sha: string;
  commit: {
    message: string;
    author: {
      name: string;
      date: string;
    };
  };
  html_url: string;
}

/**
 * Fetch commits from GitHub since a specific revision
 * Returns commits from HEAD back to (but not including) the specified rev
 */
export async function getCommitsSinceRev(
  owner: string,
  repo: string,
  sinceRev: string
): Promise<GitHubCommit[]> {
  const commits: GitHubCommit[] = [];
  let page = 1;
  const perPage = 100;
  let foundRev = false;

  while (!foundRev && page <= 5) {
    // Limit to 5 pages (500 commits max)
    const url = `https://api.github.com/repos/${owner}/${repo}/commits?per_page=${perPage}&page=${page}`;

    const response = await fetch(url, {
      headers: {
        Accept: "application/vnd.github.v3+json",
        "User-Agent": "melt-tui",
      },
    });

    if (!response.ok) {
      throw new Error(`GitHub API error: ${response.status} ${response.statusText}`);
    }

    const data = (await response.json()) as GitHubAPICommit[];

    if (data.length === 0) break;

    for (const commit of data) {
      // Stop when we find the locked revision
      if (commit.sha === sinceRev || commit.sha.startsWith(sinceRev)) {
        foundRev = true;
        break;
      }

      const message = commit.commit.message.split("\n")[0];

      commits.push({
        sha: commit.sha,
        shortSha: commit.sha.substring(0, 7),
        message: message ?? "",
        author: commit.commit.author.name,
        date: formatCommitDate(commit.commit.author.date),
        url: commit.html_url,
      });
    }

    page++;
  }

  return commits;
}

/**
 * Get changelog for a flake input (only works for GitHub inputs)
 */
export async function getChangelog(input: FlakeInput): Promise<GitHubCommit[]> {
  if (input.type !== "github" || !input.owner || !input.repo) {
    throw new Error("Changelog is only available for GitHub inputs");
  }

  return getCommitsSinceRev(input.owner, input.repo, input.rev);
}

/**
 * Format a date string as relative time
 */
function formatCommitDate(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSecs = Math.floor(diffMs / 1000);

  const minute = 60;
  const hour = minute * 60;
  const day = hour * 24;
  const week = day * 7;
  const month = day * 30;

  if (diffSecs < minute) {
    return "just now";
  } else if (diffSecs < hour) {
    const mins = Math.floor(diffSecs / minute);
    return `${mins}m ago`;
  } else if (diffSecs < day) {
    const hours = Math.floor(diffSecs / hour);
    return `${hours}h ago`;
  } else if (diffSecs < week) {
    const days = Math.floor(diffSecs / day);
    return `${days}d ago`;
  } else if (diffSecs < month) {
    const weeks = Math.floor(diffSecs / week);
    return `${weeks}w ago`;
  } else {
    // Format as date for older commits
    return date.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
    });
  }
}
