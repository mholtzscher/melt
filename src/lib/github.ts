import type { GitHubCommit, FlakeInput, UpdateStatus } from "./types";
import { formatShortRelativeTime } from "./time";

// Get GitHub token from environment for higher rate limits
// Supports GITHUB_TOKEN, GH_TOKEN, and GITHUB_PAT
const GITHUB_TOKEN = process.env.GITHUB_TOKEN || process.env.GH_TOKEN || process.env.GITHUB_PAT;

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
 * Get headers for GitHub API requests
 */
function getGitHubHeaders(): Record<string, string> {
  const headers: Record<string, string> = {
    Accept: "application/vnd.github.v3+json",
    "User-Agent": "melt-tui",
  };

  if (GITHUB_TOKEN) {
    headers.Authorization = `token ${GITHUB_TOKEN}`;
  }

  return headers;
}

/**
 * Fetch commits from GitHub since a specific revision
 * Returns commits from HEAD back to (but not including) the specified rev
 */
export async function getCommitsSinceRev(
  owner: string,
  repo: string,
  sinceRev: string,
  ref?: string
): Promise<GitHubCommit[]> {
  const commits: GitHubCommit[] = [];
  let page = 1;
  const perPage = 100;
  let foundRev = false;

  while (!foundRev && page <= 5) {
    // Limit to 5 pages (500 commits max)
    // Use sha param to specify branch/ref if provided
    const shaParam = ref ? `&sha=${encodeURIComponent(ref)}` : "";
    const url = `https://api.github.com/repos/${owner}/${repo}/commits?per_page=${perPage}&page=${page}${shaParam}`;

    const response = await fetch(url, {
      headers: getGitHubHeaders(),
    });

    if (!response.ok) {
      if (response.status === 403) {
        const remaining = response.headers.get("X-RateLimit-Remaining");
        if (remaining === "0") {
          throw new Error("GitHub API rate limit exceeded");
        }
      }
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
        date: formatShortRelativeTime(commit.commit.author.date),
        url: commit.html_url,
      });
    }

    page++;
  }

  return commits;
}

/**
 * Fetch commits starting from a specific revision (inclusive) and going back in history
 * Returns the specified rev as first item, then older commits
 */
async function getCommitsFromRev(
  owner: string,
  repo: string,
  fromRev: string,
  limit: number = 50
): Promise<GitHubCommit[]> {
  const commits: GitHubCommit[] = [];
  let page = 1;
  const perPage = Math.min(limit, 100);

  while (commits.length < limit && page <= 3) {
    const url = `https://api.github.com/repos/${owner}/${repo}/commits?sha=${fromRev}&per_page=${perPage}&page=${page}`;

    const response = await fetch(url, {
      headers: getGitHubHeaders(),
    });

    if (!response.ok) {
      if (response.status === 403) {
        const remaining = response.headers.get("X-RateLimit-Remaining");
        if (remaining === "0") {
          throw new Error("GitHub API rate limit exceeded");
        }
      }
      throw new Error(`GitHub API error: ${response.status} ${response.statusText}`);
    }

    const data = (await response.json()) as GitHubAPICommit[];

    if (data.length === 0) break;

    for (const commit of data) {
      if (commits.length >= limit) break;

      const message = commit.commit.message.split("\n")[0];

      commits.push({
        sha: commit.sha,
        shortSha: commit.sha.substring(0, 7),
        message: message ?? "",
        author: commit.commit.author.name,
        date: formatShortRelativeTime(commit.commit.author.date),
        url: commit.html_url,
      });
    }

    page++;
  }

  return commits;
}

/**
 * Fetch commit history around a locked revision
 * Returns commits ahead (newer), the locked commit, and commits behind (older)
 * The locked commit is marked with isLocked: true
 */
export async function getCommitHistory(
  owner: string,
  repo: string,
  lockedRev: string,
  ref?: string,
  behindLimit: number = 50
): Promise<{ commits: GitHubCommit[]; lockedIndex: number }> {
  // Fetch commits ahead of the locked rev (newer commits)
  // Use the ref (branch) if specified to get commits from the correct branch
  const commitsAhead = await getCommitsSinceRev(owner, repo, lockedRev, ref);

  // Fetch commits from the locked rev and older
  const commitsFromLocked = await getCommitsFromRev(owner, repo, lockedRev, behindLimit);

  // Mark the first commit (the locked one) with isLocked flag
  const lockedCommit = commitsFromLocked[0];
  if (lockedCommit) {
    lockedCommit.isLocked = true;
  }

  // Combine: ahead commits + locked commit + behind commits
  // commitsAhead is already newest-first
  // commitsFromLocked[0] is the locked commit, rest are older
  const allCommits = [...commitsAhead, ...commitsFromLocked];

  const lockedIndex = commitsAhead.length; // Index of the locked commit

  return { commits: allCommits, lockedIndex };
}

/**
 * Get changelog for a flake input (only works for GitHub inputs)
 * Returns commits ahead, the locked commit, and commits behind
 */
export async function getChangelog(
  input: FlakeInput
): Promise<{ commits: GitHubCommit[]; lockedIndex: number }> {
  if (input.type !== "github" || !input.owner || !input.repo) {
    throw new Error("Changelog is only available for GitHub inputs");
  }

  return getCommitHistory(input.owner, input.repo, input.rev, input.ref);
}

/**
 * Check if an update is available for a flake input (GitHub only)
 */
async function checkForUpdate(input: FlakeInput): Promise<UpdateStatus> {
  try {
    const commits = await getCommitsSinceRev(input.owner!, input.repo!, input.rev, input.ref);
    return {
      hasUpdate: commits.length > 0,
      commitsBehind: commits.length,
      loading: false,
    };
  } catch (error) {
    return {
      hasUpdate: false,
      commitsBehind: 0,
      loading: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

/**
 * Check for updates on multiple inputs in parallel
 * With GITHUB_TOKEN, rate limit is 5000/hour; without it's 60/hour
 */
export async function checkForUpdates(
  inputs: FlakeInput[]
): Promise<Map<string, UpdateStatus>> {
  const results = new Map<string, UpdateStatus>();
  
  // Filter to only GitHub inputs
  const githubInputs = inputs.filter(
    (input) => input.type === "github" && input.owner && input.repo
  );

  const checks = githubInputs.map(async (input) => {
    const status = await checkForUpdate(input);
    results.set(input.name, status);
  });

  await Promise.all(checks);
  return results;
}

/**
 * Check if GitHub token is configured
 */
export function hasGitHubToken(): boolean {
  return !!GITHUB_TOKEN;
}


