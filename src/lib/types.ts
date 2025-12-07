// Flake input types
export type FlakeInputType =
	| "github"
	| "gitlab"
	| "sourcehut"
	| "path"
	| "git"
	| "other";

export interface FlakeInput {
	name: string;
	type: FlakeInputType;
	owner?: string;
	repo?: string;
	ref?: string; // branch/tag reference (e.g., "nixos-unstable")
	url: string;
	rev: string;
	shortRev: string;
	lastModified: number;
}

export interface FlakeMetadata {
	description?: string;
	inputs: FlakeInput[];
	path: string;
}

// GitHub types for changelog
export interface GitHubCommit {
	sha: string;
	shortSha: string;
	message: string;
	author: string;
	date: string;
	url: string;
	isLocked?: boolean;
}

// Update status for flake inputs
export interface UpdateStatus {
	hasUpdate: boolean;
	commitsBehind: number;
	loading: boolean;
	error?: string;
}

// App state types
export type AppView = "list" | "changelog" | "error" | "updating";

export interface AppState {
	view: AppView;
	inputs: FlakeInput[];
	selectedIndices: Set<number>;
	cursorIndex: number;
	loading: boolean;
	error?: string;
	statusMessage?: string;
	changelogInput?: FlakeInput;
	changelogCommits?: GitHubCommit[];
}

// Utility type for nix flake metadata JSON response
export interface NixFlakeMetadataResponse {
	description?: string;
	path: string;
	locks: {
		nodes: Record<
			string,
			{
				inputs?: Record<string, string | string[]>;
				locked?: {
					type: string;
					owner?: string;
					repo?: string;
					rev?: string;
					lastModified?: number;
					url?: string;
					path?: string;
				};
				original?: {
					type: string;
					owner?: string;
					repo?: string;
					ref?: string;
					url?: string;
					path?: string;
				};
			}
		>;
		root: string;
	};
}
