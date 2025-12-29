import type { FlakeInput, GitHubCommit, UpdateStatus } from "../types";

export interface GitHubService {
  getCommitsSinceRev(
    owner: string,
    repo: string,
    sinceRev: string,
    ref?: string,
  ): Promise<GitHubCommit[]>;
  getCommitHistory(
    owner: string,
    repo: string,
    lockedRev: string,
    ref?: string,
    behindLimit?: number,
  ): Promise<{ commits: GitHubCommit[]; lockedIndex: number }>;
  getChangelog(input: FlakeInput): Promise<{ commits: GitHubCommit[]; lockedIndex: number }>;
  checkForUpdates(
    inputs: FlakeInput[],
    onStatusChange?: (name: string, status: UpdateStatus) => void,
  ): Promise<Map<string, UpdateStatus>>;
  hasGitHubToken(): boolean;
}

const API_DELAY_MS = Number(process.env.MELT_API_DELAY_MS) || 0;
const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

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
 * Format a date string as short relative time (e.g., "2d ago")
 */
function formatShortRelativeTime(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSecs = Math.floor(diffMs / 1000);

  if (diffSecs < 60) {
    return "just now";
  }

  const TIME_UNITS = [
    { seconds: 365 * 24 * 60 * 60, short: "y" },
    { seconds: 30 * 24 * 60 * 60, short: "mo" },
    { seconds: 7 * 24 * 60 * 60, short: "w" },
    { seconds: 24 * 60 * 60, short: "d" },
    { seconds: 60 * 60, short: "h" },
    { seconds: 60, short: "m" },
  ];

  for (const unit of TIME_UNITS) {
    if (diffSecs >= unit.seconds) {
      const count = Math.floor(diffSecs / unit.seconds);
      // For months and beyond, use date format
      if (unit.short === "mo" || unit.short === "y") {
        return date.toLocaleDateString("en-US", {
          month: "short",
          day: "numeric",
        });
      }
      return `${count}${unit.short} ago`;
    }
  }

  return "just now";
}

/**
 * Fetch commits starting from a specific revision (inclusive) and going back in history
 */
async function getCommitsFromRev(
  owner: string,
  repo: string,
  fromRev: string,
  limit: number = 50,
): Promise<GitHubCommit[]> {
  const commits: GitHubCommit[] = [];
  let page = 1;
  const perPage = Math.min(limit, 100);

  while (commits.length < limit && page <= 3) {
    const url = `https://api.github.com/repos/${owner}/${repo}/commits?sha=${fromRev}&per_page=${perPage}&page=${page}`;

    if (API_DELAY_MS > 0) await delay(API_DELAY_MS);

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
 * Check if an update is available for a flake input (GitHub only)
 */
async function checkForUpdate(input: FlakeInput): Promise<UpdateStatus> {
  if (!input.owner || !input.repo) {
    return {
      hasUpdate: false,
      commitsBehind: 0,
      loading: false,
      error: "Missing owner or repo",
    };
  }

  try {
    const commits = await githubService.getCommitsSinceRev(
      input.owner,
      input.repo,
      input.rev,
      input.ref,
    );
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

export const githubService: GitHubService = {
  /**
   * Fetch commits from GitHub since a specific revision
   * Returns commits from HEAD back to (but not including) the specified rev
   */
  async getCommitsSinceRev(
    owner: string,
    repo: string,
    sinceRev: string,
    ref?: string,
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

      if (API_DELAY_MS > 0) await delay(API_DELAY_MS);

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
  },

  /**
   * Fetch commit history around a locked revision
   * Returns commits ahead (newer), the locked commit, and commits behind (older)
   */
  async getCommitHistory(
    owner: string,
    repo: string,
    lockedRev: string,
    ref?: string,
    behindLimit: number = 50,
  ): Promise<{ commits: GitHubCommit[]; lockedIndex: number }> {
    // Fetch commits ahead of the locked rev (newer commits)
    const commitsAhead = await this.getCommitsSinceRev(owner, repo, lockedRev, ref);

    // Fetch commits from the locked rev and older
    const commitsFromLocked = await getCommitsFromRev(owner, repo, lockedRev, behindLimit + 1);

    // Mark the first commit (the locked one) with isLocked flag
    const lockedCommit = commitsFromLocked[0];
    if (lockedCommit) {
      lockedCommit.isLocked = true;
    }

    // Combine: ahead commits + locked commit + behind commits
    const allCommits = [...commitsAhead, ...commitsFromLocked];
    const lockedIndex = commitsAhead.length;

    return { commits: allCommits, lockedIndex };
  },

  /**
   * Get changelog for a flake input (only works for GitHub inputs)
   */
  async getChangelog(input: FlakeInput): Promise<{ commits: GitHubCommit[]; lockedIndex: number }> {
    if (input.type !== "github" || !input.owner || !input.repo) {
      throw new Error("Changelog is only available for GitHub inputs");
    }

    return this.getCommitHistory(input.owner, input.repo, input.rev, input.ref);
  },

  /**
   * Check for updates on multiple inputs in parallel
   */
  async checkForUpdates(
    inputs: FlakeInput[],
    onStatusChange?: (name: string, status: UpdateStatus) => void,
  ): Promise<Map<string, UpdateStatus>> {
    const results = new Map<string, UpdateStatus>();

    // Filter to only GitHub inputs
    const githubInputs = inputs.filter(
      (input) => input.type === "github" && input.owner && input.repo,
    );

    for (const input of githubInputs) {
      const loadingStatus: UpdateStatus = {
        hasUpdate: false,
        commitsBehind: 0,
        loading: true,
      };
      results.set(input.name, loadingStatus);
      onStatusChange?.(input.name, loadingStatus);
    }

    // Batch to avoid rate limits (Unauth: 60/hr, Auth: 5000/hr)
    const BATCH_SIZE = this.hasGitHubToken() ? 10 : 2;

    for (let i = 0; i < githubInputs.length; i += BATCH_SIZE) {
      const batch = githubInputs.slice(i, i + BATCH_SIZE);
      await Promise.all(
        batch.map(async (input) => {
          const status = await checkForUpdate(input);
          results.set(input.name, status);
          onStatusChange?.(input.name, status);
        }),
      );
    }

    return results;
  },

  hasGitHubToken(): boolean {
    return !!GITHUB_TOKEN;
  },
};
