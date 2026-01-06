import type { FlakeInput } from "../../types";

/** Generic commit representation across all VCS providers */
export interface Commit {
	sha: string;
	shortSha: string;
	message: string;
	author: string;
	date: string;
	url?: string; // Optional - git CLI provider won't have this
	isLocked?: boolean;
}

/** Update status for a flake input */
export interface UpdateStatus {
	commitsBehind: number;
	loading: boolean;
	error?: string;
}

/** Result of fetching changelog */
export interface ChangelogResult {
	commits: Commit[];
	lockedIndex: number;
}

/** Detected forge type for routing to appropriate provider */
export type ForgeType = "github" | "gitlab" | "codeberg" | "sourcehut" | "gitea" | "generic";

/** Parsed repository info extracted from URLs or input metadata */
export interface RepoInfo {
	forge: ForgeType;
	owner: string;
	repo: string;
	host?: string; // For self-hosted instances
}

/** VCS Provider interface - implemented by each forge provider */
export interface VCSProvider {
	/** Get commits from HEAD back to (but not including) the locked revision */
	getCommitsSinceRev(input: FlakeInput, repoInfo: RepoInfo): Promise<Commit[]>;

	/** Get full changelog with commits before and after locked revision */
	getChangelog(input: FlakeInput, repoInfo: RepoInfo): Promise<ChangelogResult>;

	/** Get URL suitable for nix flake lock --override-input */
	getLockUrl(input: FlakeInput, repoInfo: RepoInfo, rev: string): string;
}

/** Unified VCS service interface exposed to UI */
export interface VCSService {
	/** Check for updates on multiple inputs, with progress callback */
	checkForUpdates(
		inputs: FlakeInput[],
		onStatusChange?: (name: string, status: UpdateStatus) => void,
	): Promise<Map<string, UpdateStatus>>;

	/** Get changelog for a single input */
	getChangelog(input: FlakeInput): Promise<ChangelogResult>;

	/** Check if changelog is supported for this input type */
	supportsChangelog(input: FlakeInput): boolean;

	/** Check if locking to a specific commit is supported */
	supportsLocking(input: FlakeInput): boolean;

	/** Get the nix-compatible URL for locking to a specific revision */
	getLockUrl(input: FlakeInput, rev: string): string | null;
}
