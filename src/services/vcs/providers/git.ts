import { createHash } from "node:crypto";
import { mkdir, rm } from "node:fs/promises";
import type { FlakeInput } from "../../../types";
import { processManager } from "../../processManager";
import type { ChangelogResult, Commit, RepoInfo, VCSProvider } from "../types";
import { formatShortRelativeTime, getCacheDir, revisionMatches } from "../utils";

/**
 * Get the git clone URL based on detected forge type
 */
function getCloneUrl(input: FlakeInput, repoInfo: RepoInfo): string {
	switch (repoInfo.forge) {
		case "github":
			return `https://github.com/${repoInfo.owner}/${repoInfo.repo}.git`;
		case "gitlab":
			return `https://${repoInfo.host || "gitlab.com"}/${repoInfo.owner}/${repoInfo.repo}.git`;
		case "sourcehut": {
			const owner = repoInfo.owner.startsWith("~") ? repoInfo.owner : `~${repoInfo.owner}`;
			return `https://${repoInfo.host || "git.sr.ht"}/${owner}/${repoInfo.repo}`;
		}
		case "codeberg":
		case "gitea":
			return `https://${repoInfo.host || "codeberg.org"}/${repoInfo.owner}/${repoInfo.repo}.git`;
		default:
			return input.url;
	}
}

/**
 * Get the web URL for a commit (for clickable links in UI)
 */
function getCommitUrl(repoInfo: RepoInfo, sha: string): string | undefined {
	switch (repoInfo.forge) {
		case "github":
			return `https://github.com/${repoInfo.owner}/${repoInfo.repo}/commit/${sha}`;
		case "gitlab":
			return `https://${repoInfo.host || "gitlab.com"}/${repoInfo.owner}/${repoInfo.repo}/-/commit/${sha}`;
		case "sourcehut": {
			const owner = repoInfo.owner.startsWith("~") ? repoInfo.owner : `~${repoInfo.owner}`;
			return `https://${repoInfo.host || "git.sr.ht"}/${owner}/${repoInfo.repo}/commit/${sha}`;
		}
		case "codeberg":
		case "gitea":
			return `https://${repoInfo.host || "codeberg.org"}/${repoInfo.owner}/${repoInfo.repo}/commit/${sha}`;
		default:
			return undefined;
	}
}

/**
 * Get the nix lock URL for a specific revision
 */
function getLockUrlForInput(input: FlakeInput, repoInfo: RepoInfo, rev: string): string {
	switch (repoInfo.forge) {
		case "github":
			return `github:${repoInfo.owner}/${repoInfo.repo}/${rev}`;
		case "gitlab":
			if (!repoInfo.host || repoInfo.host === "gitlab.com") {
				return `gitlab:${repoInfo.owner}/${repoInfo.repo}/${rev}`;
			}
			return `git+https://${repoInfo.host}/${repoInfo.owner}/${repoInfo.repo}?rev=${rev}`;
		case "sourcehut": {
			const owner = repoInfo.owner.startsWith("~") ? repoInfo.owner : `~${repoInfo.owner}`;
			return `sourcehut:${owner}/${repoInfo.repo}/${rev}`;
		}
		case "codeberg":
		case "gitea":
			return `git+https://${repoInfo.host || "codeberg.org"}/${repoInfo.owner}/${repoInfo.repo}?rev=${rev}`;
		default: {
			// For generic git repos, construct a git+ URL with revision
			const url = input.url;
			if (!url) {
				return `git+https://${repoInfo.owner}/${repoInfo.repo}?rev=${rev}`;
			}
			if (url.startsWith("git+")) {
				return `${url.split("?")[0]}?rev=${rev}`;
			}
			if (url.startsWith("git@") || url.startsWith("ssh://")) {
				const normalized = `git+ssh://${url.replace("git@", "").replace(":", "/")}`;
				return `${normalized.split("?")[0]}?rev=${rev}`;
			}
			if (url.startsWith("http")) {
				return `git+${url.split("?")[0]}?rev=${rev}`;
			}
			return `git+https://${url.split("?")[0]}?rev=${rev}`;
		}
	}
}

/**
 * Get or create a bare clone cache directory for a URL
 */
function getCachePathForUrl(url: string): string {
	const hash = createHash("sha256").update(url).digest("hex").substring(0, 16);
	const safeName = url.replace(/[^a-zA-Z0-9]/g, "_").substring(0, 32);
	return `${getCacheDir()}/git/${safeName}_${hash}`;
}

/**
 * Ensure the git cache directory exists
 */
async function ensureCacheDir(): Promise<void> {
	await mkdir(`${getCacheDir()}/git`, { recursive: true });
}

/**
 * Run a git command and return stdout
 */
async function runGit(args: string[], cwd?: string): Promise<string> {
	if (processManager.getSignal().aborted) {
		throw new Error("Command aborted");
	}

	const proc = Bun.spawn(["git", ...args], {
		cwd,
		stdout: "pipe",
		stderr: "pipe",
		signal: processManager.getSignal(),
	});

	const [stdout, stderr, exitCode] = await Promise.all([
		new Response(proc.stdout).text(),
		new Response(proc.stderr).text(),
		proc.exited,
	]);

	if (processManager.getSignal().aborted) {
		throw new Error("Command aborted");
	}

	if (exitCode !== 0) {
		throw new Error(`Git error: ${stderr.trim() || `exit code ${exitCode}`}`);
	}

	return stdout.trim();
}

/**
 * Clone or update a bare repository in cache
 */
async function ensureRepo(url: string, ref?: string): Promise<string> {
	await ensureCacheDir();
	const cachePath = getCachePathForUrl(url);

	try {
		// Check if already cloned
		await runGit(["rev-parse", "--git-dir"], cachePath);
		// Already exists, fetch latest
		await runGit(["fetch", "--all", "--prune"], cachePath);
	} catch {
		// Need to clone - use single-branch for speed if ref is specified
		await rm(cachePath, { recursive: true, force: true });
		const cloneArgs = ["clone", "--bare", "--filter=blob:none"];
		if (ref && ref !== "HEAD") {
			cloneArgs.push("--single-branch", "--branch", ref);
		}
		cloneArgs.push(url, cachePath);
		await runGit(cloneArgs);
	}

	return cachePath;
}

/**
 * Parse git log output into commits
 */
function parseGitLog(output: string, repoInfo: RepoInfo): Commit[] {
	if (!output.trim()) return [];

	const commits: Commit[] = [];

	for (const line of output.trim().split("\n")) {
		const [sha, message, author, date] = line.split("|");
		if (!sha) continue;

		const commit: Commit = {
			sha,
			shortSha: sha.substring(0, 7),
			message: message ?? "",
			author: author ?? "Unknown",
			date: date ? formatShortRelativeTime(date) : "unknown",
		};

		const url = getCommitUrl(repoInfo, sha);
		if (url) {
			commit.url = url;
		}

		commits.push(commit);
	}

	return commits;
}

export const gitProvider: VCSProvider = {
	async getCommitsSinceRev(input: FlakeInput, repoInfo: RepoInfo): Promise<Commit[]> {
		const cloneUrl = getCloneUrl(input, repoInfo);
		const ref = input.ref || "HEAD";
		const repoPath = await ensureRepo(cloneUrl, input.ref);

		try {
			// Get commits between locked rev and current ref
			const output = await runGit(
				["log", `${input.rev}..${ref}`, "--pretty=format:%H|%s|%an|%aI", "--max-count=500"],
				repoPath,
			);

			return parseGitLog(output, repoInfo);
		} catch {
			// If the revision doesn't exist on the ref, fall back to getting recent commits
			// This can happen if the locked rev is on a different branch
			const output = await runGit(["log", ref, "--pretty=format:%H|%s|%an|%aI", "--max-count=100"], repoPath);

			const commits = parseGitLog(output, repoInfo);
			// Find the locked rev index
			const lockedIdx = commits.findIndex((c) => revisionMatches(c.sha, input.rev));
			if (lockedIdx >= 0) {
				return commits.slice(0, lockedIdx);
			}

			// Couldn't find locked rev, return all commits
			return commits;
		}
	},

	async getChangelog(input: FlakeInput, repoInfo: RepoInfo): Promise<ChangelogResult> {
		const cloneUrl = getCloneUrl(input, repoInfo);
		const repoPath = await ensureRepo(cloneUrl, input.ref);

		// Get commits ahead of locked rev
		const commitsAhead = await this.getCommitsSinceRev(input, repoInfo);

		// Get commits from locked rev going back
		let commitsFromLocked: Commit[] = [];
		try {
			const output = await runGit(["log", input.rev, "--pretty=format:%H|%s|%an|%aI", "--max-count=51"], repoPath);
			commitsFromLocked = parseGitLog(output, repoInfo);
		} catch {
			// Locked rev might not be accessible, just return ahead commits
		}

		// Mark the first commit from locked as the locked one
		const lockedCommit = commitsFromLocked[0];
		if (lockedCommit) {
			lockedCommit.isLocked = true;
		}

		const allCommits = [...commitsAhead, ...commitsFromLocked];
		// Return -1 if locked rev wasn't found (force-pushed away, different branch, etc.)
		const lockedIndex = commitsFromLocked.length > 0 ? commitsAhead.length : -1;

		return { commits: allCommits, lockedIndex };
	},

	getLockUrl(input: FlakeInput, repoInfo: RepoInfo, rev: string): string {
		return getLockUrlForInput(input, repoInfo, rev);
	},
};

/**
 * Clear the git cache directory
 */
export async function clearGitCache(): Promise<void> {
	await rm(`${getCacheDir()}/git`, { recursive: true, force: true });
}
