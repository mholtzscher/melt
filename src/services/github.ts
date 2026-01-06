import { Effect } from "effect";
import type { FlakeInput, GitHubCommit, UpdateStatus } from "../types";
import { GitHubApiError, GitHubInputError, GitHubRateLimitError } from "../types/errors";

const API_DELAY_MS = Number(process.env.MELT_API_DELAY_MS) || 0;
const delay = (ms: number): Effect.Effect<void> =>
	Effect.promise(() => new Promise((resolve) => setTimeout(resolve, ms)));

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
 * Make a GitHub API request with error handling
 */
const fetchGitHub = (url: string): Effect.Effect<Response, GitHubRateLimitError | GitHubApiError> =>
	Effect.gen(function* () {
		if (API_DELAY_MS > 0) yield* delay(API_DELAY_MS);

		const response = yield* Effect.tryPromise({
			try: () =>
				fetch(url, {
					headers: getGitHubHeaders(),
				}),
			catch: () =>
				new GitHubApiError({
					status: 0,
					statusText: "Network error",
					url,
				}),
		});

		if (!response.ok) {
			if (response.status === 403) {
				const remaining = response.headers.get("X-RateLimit-Remaining");
				if (remaining === "0") {
					const resetHeader = response.headers.get("X-RateLimit-Reset");
					const resetAt = resetHeader ? new Date(Number(resetHeader) * 1000) : undefined;
					return yield* Effect.fail(new GitHubRateLimitError({ resetAt }));
				}
			}
			return yield* Effect.fail(
				new GitHubApiError({
					status: response.status,
					statusText: response.statusText,
					url,
				}),
			);
		}

		return response;
	});

/**
 * Fetch commits starting from a specific revision (inclusive) and going back in history
 */
const getCommitsFromRev = (
	owner: string,
	repo: string,
	fromRev: string,
	limit: number = 50,
): Effect.Effect<GitHubCommit[], GitHubRateLimitError | GitHubApiError> =>
	Effect.gen(function* () {
		const commits: GitHubCommit[] = [];
		let page = 1;
		const perPage = Math.min(limit, 100);

		while (commits.length < limit && page <= 3) {
			const url = `https://api.github.com/repos/${owner}/${repo}/commits?sha=${fromRev}&per_page=${perPage}&page=${page}`;

			const response = yield* fetchGitHub(url);
			const data = (yield* Effect.tryPromise({
				try: () => response.json() as Promise<GitHubAPICommit[]>,
				catch: () =>
					new GitHubApiError({
						status: 0,
						statusText: "Failed to parse JSON",
						url,
					}),
			})) as GitHubAPICommit[];

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
	});

/**
 * Check if an update is available for a flake input (GitHub only)
 */
const checkForUpdate = (input: FlakeInput): Effect.Effect<UpdateStatus, never> =>
	Effect.gen(function* () {
		if (!input.owner || !input.repo) {
			return {
				commitsBehind: 0,
				loading: false,
				error: "Missing owner or repo",
			};
		}

		const result = yield* githubService
			.getCommitsSinceRev(input.owner, input.repo, input.rev, input.ref)
			.pipe(Effect.either);

		if (result._tag === "Left") {
			return {
				commitsBehind: 0,
				loading: false,
				error: result.left.message,
			};
		}

		return {
			commitsBehind: result.right.length,
			loading: false,
		};
	});

export const githubService = {
	/**
	 * Fetch commits from GitHub since a specific revision
	 * Returns commits from HEAD back to (but not including) the specified rev
	 */
	getCommitsSinceRev: (
		owner: string,
		repo: string,
		sinceRev: string,
		ref?: string,
	): Effect.Effect<GitHubCommit[], GitHubRateLimitError | GitHubApiError> =>
		Effect.gen(function* () {
			const commits: GitHubCommit[] = [];
			let page = 1;
			const perPage = 100;
			let foundRev = false;

			while (!foundRev && page <= 5) {
				// Use sha param to specify branch/ref if provided
				const shaParam = ref ? `&sha=${encodeURIComponent(ref)}` : "";
				const url = `https://api.github.com/repos/${owner}/${repo}/commits?per_page=${perPage}&page=${page}${shaParam}`;

				const response = yield* fetchGitHub(url);
				const data = (yield* Effect.tryPromise({
					try: () => response.json() as Promise<GitHubAPICommit[]>,
					catch: () =>
						new GitHubApiError({
							status: 0,
							statusText: "Failed to parse JSON",
							url,
						}),
				})) as GitHubAPICommit[];

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
		}),

	/**
	 * Fetch commit history around a locked revision
	 * Returns commits ahead (newer), the locked commit, and commits behind (older)
	 */
	getCommitHistory: (
		owner: string,
		repo: string,
		lockedRev: string,
		ref?: string,
		behindLimit: number = 50,
	): Effect.Effect<{ commits: GitHubCommit[]; lockedIndex: number }, GitHubRateLimitError | GitHubApiError> =>
		Effect.gen(function* () {
			// Fetch commits ahead of the locked rev (newer commits)
			const commitsAhead = yield* githubService.getCommitsSinceRev(owner, repo, lockedRev, ref);

			// Fetch commits from the locked rev and older
			const commitsFromLocked = yield* getCommitsFromRev(owner, repo, lockedRev, behindLimit + 1);

			// Mark the first commit (the locked one) with isLocked flag
			const lockedCommit = commitsFromLocked[0];
			if (lockedCommit) {
				lockedCommit.isLocked = true;
			}

			// Combine: ahead commits + locked commit + behind commits
			const allCommits = [...commitsAhead, ...commitsFromLocked];
			const lockedIndex = commitsAhead.length;

			return { commits: allCommits, lockedIndex };
		}),

	/**
	 * Get changelog for a flake input (only works for GitHub inputs)
	 */
	getChangelog: (
		input: FlakeInput,
	): Effect.Effect<
		{ commits: GitHubCommit[]; lockedIndex: number },
		GitHubRateLimitError | GitHubApiError | GitHubInputError
	> =>
		Effect.gen(function* () {
			if (input.type !== "github" || !input.owner || !input.repo) {
				return yield* Effect.fail(
					new GitHubInputError({
						inputName: input.name,
						reason: "Changelog is only available for GitHub inputs",
					}),
				);
			}

			return yield* githubService.getCommitHistory(input.owner, input.repo, input.rev, input.ref);
		}),

	/**
	 * Check for updates on multiple inputs in parallel
	 */
	checkForUpdates: (
		inputs: FlakeInput[],
		onStatusChange?: (name: string, status: UpdateStatus) => void,
	): Effect.Effect<Map<string, UpdateStatus>> =>
		Effect.gen(function* () {
			const results = new Map<string, UpdateStatus>();

			// Filter to only GitHub inputs
			const githubInputs = inputs.filter((input) => input.type === "github" && input.owner && input.repo);

			for (const input of githubInputs) {
				const loadingStatus: UpdateStatus = {
					commitsBehind: 0,
					loading: true,
				};
				results.set(input.name, loadingStatus);
				onStatusChange?.(input.name, loadingStatus);
			}

			// Batch to avoid rate limits (Unauth: 60/hr, Auth: 5000/hr)
			const BATCH_SIZE = githubService.hasGitHubToken() ? 10 : 2;

			for (let i = 0; i < githubInputs.length; i += BATCH_SIZE) {
				const batch = githubInputs.slice(i, i + BATCH_SIZE);
				yield* Effect.forEach(
					batch,
					(input) =>
						Effect.gen(function* () {
							const status = yield* checkForUpdate(input);
							results.set(input.name, status);
							onStatusChange?.(input.name, status);
						}),
					{ concurrency: "unbounded" },
				);
			}

			return results;
		}),

	hasGitHubToken: (): boolean => !!GITHUB_TOKEN,
};

export type GitHubService = typeof githubService;
