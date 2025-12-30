import { createStore } from "solid-js/store";
import type { FlakeData } from "../services/flake";
import { flakeService } from "../services/flake";
import { githubService } from "../services/github";
import { toast } from "../services/toast";
import type { FlakeInput, UpdateStatus } from "../types";

export interface FlakeState {
	path: string;
	inputs: FlakeInput[];
	updateStatuses: Record<string, UpdateStatus>;
	loading: boolean;
	changelogInput: FlakeInput | undefined;
}

export interface FlakeActions {
	checkUpdates: (inputsList?: FlakeInput[]) => Promise<void>;
	refresh: () => Promise<void>;
	updateSelected: (names: string[]) => Promise<void>;
	updateAll: () => Promise<void>;
	lockToCommit: (inputName: string, sha: string, owner: string, repo: string) => Promise<boolean>;
	showChangelog: (input: FlakeInput) => void;
	closeChangelog: () => void;
}

export interface FlakeStore {
	state: FlakeState;
	actions: FlakeActions;
}

export function createFlakeStore(initialFlake: FlakeData): FlakeStore {
	const [state, setState] = createStore<FlakeState>({
		path: initialFlake.path,
		inputs: initialFlake.inputs,
		updateStatuses: {},
		loading: false,
		changelogInput: undefined,
	});

	let isCheckingUpdates = false;

	function getErrorToast(errorMsg: string): { id: string; message: string } {
		const normalized = errorMsg.toLowerCase();

		if (normalized.includes("rate limit")) {
			return {
				id: "error:rate-limit",
				message: "GitHub rate limit exceeded; set GITHUB_TOKEN",
			};
		}

		if (normalized.includes("bad credentials") || normalized.includes("requires authentication")) {
			return {
				id: "error:auth",
				message: "GitHub authentication failed - check GITHUB_TOKEN",
			};
		}

		if (normalized.includes("404") || normalized.includes("not found")) {
			return {
				id: "error:not-found",
				message: "GitHub repository not found",
			};
		}

		if (normalized.includes("fetch failed") || normalized.includes("enotfound") || normalized.includes("network")) {
			return {
				id: "error:network",
				message: "Network error checking GitHub",
			};
		}

		if (normalized.includes("missing owner or repo")) {
			return {
				id: "error:missing-owner-repo",
				message: "Invalid GitHub input (missing owner/repo)",
			};
		}

		if (normalized.includes("github api error")) {
			return {
				id: "error:github-api",
				message: "GitHub API error checking updates",
			};
		}

		return {
			id: "error:unknown",
			message: "Error checking updates",
		};
	}

	async function checkUpdates(inputsList?: FlakeInput[]) {
		if (isCheckingUpdates) return;
		isCheckingUpdates = true;

		const toCheck = inputsList || state.inputs;

		try {
			const errorsByType = new Set<string>();
			await githubService.checkForUpdates(toCheck, (name, status) => {
				setState("updateStatuses", name, status);
				if (status.error) {
					errorsByType.add(status.error);
				}
			});

			const toastsById = new Map<string, string>();
			for (const error of errorsByType) {
				const toastMeta = getErrorToast(error);
				toastsById.set(toastMeta.id, toastMeta.message);
			}

			for (const [id, message] of toastsById) {
				toast.error(message, { id });
			}
		} catch (err) {
			const errorMsg = err instanceof Error ? err.message : String(err);
			const toastMeta = getErrorToast(errorMsg);
			toast.error(toastMeta.message, { id: toastMeta.id });
		} finally {
			isCheckingUpdates = false;
		}
	}

	async function refresh() {
		const result = await flakeService.refresh(state.path);
		if (!result.ok) {
			toast.error(result.error);
			return;
		}

		setState("inputs", result.data.inputs);
		await checkUpdates(result.data.inputs);
	}

	async function updateSelected(names: string[]) {
		if (names.length === 0) {
			toast.warning("No inputs selected");
			return;
		}

		setState("loading", true);

		for (const name of names) {
			setState("updateStatuses", name, (prev) => ({
				...prev,
				hasUpdate: prev?.hasUpdate ?? false,
				commitsBehind: prev?.commitsBehind ?? 0,
				loading: prev?.loading ?? false,
				updating: true,
			}));
		}

		try {
			const result = await flakeService.updateInputs(state.path, names);

			for (const name of names) {
				setState("updateStatuses", name, (prev) => ({
					...prev,
					hasUpdate: prev?.hasUpdate ?? false,
					commitsBehind: prev?.commitsBehind ?? 0,
					loading: prev?.loading ?? false,
					updating: false,
				}));
			}

			if (result.ok) {
				await refresh();
			} else {
				toast.error(result.error);
			}
		} finally {
			setState("loading", false);
		}
	}

	async function updateAll() {
		setState("loading", true);

		for (const input of state.inputs) {
			setState("updateStatuses", input.name, (prev) => ({
				...prev,
				hasUpdate: prev?.hasUpdate ?? false,
				commitsBehind: prev?.commitsBehind ?? 0,
				loading: prev?.loading ?? false,
				updating: true,
			}));
		}

		try {
			const result = await flakeService.updateAll(state.path);

			for (const input of state.inputs) {
				setState("updateStatuses", input.name, (prev) => ({
					...prev,
					hasUpdate: prev?.hasUpdate ?? false,
					commitsBehind: prev?.commitsBehind ?? 0,
					loading: prev?.loading ?? false,
					updating: false,
				}));
			}

			if (result.ok) {
				await refresh();
			} else {
				toast.error(result.error);
			}
		} finally {
			setState("loading", false);
		}
	}

	async function lockToCommit(inputName: string, sha: string, owner: string, repo: string): Promise<boolean> {
		const toastId = toast.loading(`Locking ${inputName} to ${sha.substring(0, 7)}...`);

		const result = await flakeService.lockInputToRev(state.path, inputName, sha, owner, repo);

		if (result.ok) {
			toast.success(`Locked ${inputName} to ${sha.substring(0, 7)}`, {
				id: toastId,
			});
			return true;
		}
		toast.error(result.error, { id: toastId });
		return false;
	}

	function showChangelog(input: FlakeInput) {
		if (input.type !== "github") {
			toast.warning("Changelog only available for GitHub inputs");
			return;
		}
		setState("changelogInput", input);
	}

	function closeChangelog() {
		setState("changelogInput", undefined);
	}

	const actions: FlakeActions = {
		checkUpdates,
		refresh,
		updateSelected,
		updateAll,
		lockToCommit,
		showChangelog,
		closeChangelog,
	};

	return { state, actions };
}
