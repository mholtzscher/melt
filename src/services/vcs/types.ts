import type { FlakeInput } from "../../types";

export interface Commit {
	sha: string;
	shortSha: string;
	message: string;
	author: string;
	date: string;
	url?: string;
	isLocked?: boolean;
}

export interface UpdateStatus {
	commitsBehind: number;
	loading: boolean;
	error?: string;
}

export interface ChangelogResult {
	commits: Commit[];
	lockedIndex: number;
}

export type ForgeType = "github" | "gitlab" | "codeberg" | "sourcehut" | "gitea" | "generic";

export interface RepoInfo {
	forge: ForgeType;
	owner: string;
	repo: string;
	host?: string;
}

export interface VCSProvider {
	getCommitsSinceRev(input: FlakeInput, repoInfo: RepoInfo): Promise<Commit[]>;

	getChangelog(input: FlakeInput, repoInfo: RepoInfo): Promise<ChangelogResult>;

	getLockUrl(input: FlakeInput, repoInfo: RepoInfo, rev: string): string;
}

export interface VCSService {
	checkForUpdates(
		inputs: FlakeInput[],
		onStatusChange?: (name: string, status: UpdateStatus) => void,
	): Promise<Map<string, UpdateStatus>>;

	getChangelog(input: FlakeInput): Promise<ChangelogResult>;

	supportsChangelog(input: FlakeInput): boolean;

	getLockUrl(input: FlakeInput, rev: string): string | null;
}
