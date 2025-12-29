// Generic result type for operations that can fail
export type Result<T> = { ok: true; data: T } | { ok: false; error: string };

// Flake input types
export type FlakeInputType = "github" | "gitlab" | "sourcehut" | "path" | "git" | "other";

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
	updating?: boolean;
	error?: string;
}
