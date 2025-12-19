import type { Accessor, Setter } from "solid-js";

// Generic result type for operations that can fail
export type Result<T> = { ok: true; data: T } | { ok: false; error: string };

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
	error?: string;
}

// App state types
export type AppView = "list" | "changelog" | "updating";

// State interfaces (signal-based)
export interface FlakeState {
	inputs: Accessor<FlakeInput[]>;
	setInputs: Setter<FlakeInput[]>;
	description: Accessor<string | undefined>;
	setDescription: Setter<string | undefined>;
	flakePath: Accessor<string>;
}

export interface ListNavigationState {
	cursorIndex: Accessor<number>;
	setCursorIndex: Setter<number>;
	selectedIndices: Accessor<Set<number>>;
	setSelectedIndices: Setter<Set<number>>;
}

export interface UpdateStatusState {
	updateStatuses: Accessor<Map<string, UpdateStatus>>;
	setUpdateStatuses: Setter<Map<string, UpdateStatus>>;
}

export interface UIState {
	view: Accessor<AppView>;
	setView: Setter<AppView>;
	loading: Accessor<boolean>;
	setLoading: Setter<boolean>;
	statusMessage: Accessor<string | undefined>;
	setStatusMessage: Setter<string | undefined>;
}

export interface ChangelogState {
	changelogInput: Accessor<FlakeInput | undefined>;
	setChangelogInput: Setter<FlakeInput | undefined>;
	commits: Accessor<GitHubCommit[]>;
	setCommits: Setter<GitHubCommit[]>;
	lockedIndex: Accessor<number>;
	setLockedIndex: Setter<number>;
	changelogCursorIndex: Accessor<number>;
	setChangelogCursorIndex: Setter<number>;
	changelogLoading: Accessor<boolean>;
	setChangelogLoading: Setter<boolean>;
}

export interface ConfirmDialogState {
	showConfirm: Accessor<boolean>;
	setShowConfirm: Setter<boolean>;
	confirmCommit: Accessor<GitHubCommit | undefined>;
	setConfirmCommit: Setter<GitHubCommit | undefined>;
}
