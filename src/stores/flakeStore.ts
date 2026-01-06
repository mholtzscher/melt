import { batch } from "solid-js";
import { createStore } from "solid-js/store";
import type { FlakeData } from "../services/flake";
import { flakeService } from "../services/flake";
import { toast, toastForError } from "../services/toast";
import { vcsService } from "../services/vcs";
import type { FlakeInput, UpdateStatus } from "../types";

export interface FlakeState {
	path: string;
	inputs: FlakeInput[];
	updateStatuses: Record<string, UpdateStatus>;
	changelogInput: FlakeInput | undefined;
}

export interface FlakeActions {
	checkUpdates: (inputsList?: FlakeInput[]) => Promise<void>;
	refresh: () => Promise<void>;
	updateSelected: (names: string[]) => Promise<void>;
	updateAll: () => Promise<void>;
	lockToCommit: (input: FlakeInput, sha: string) => Promise<boolean>;
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
		changelogInput: undefined,
	});

	let isCheckingUpdates = false;

	async function checkUpdates(inputsList?: FlakeInput[]) {
		if (isCheckingUpdates) return;
		isCheckingUpdates = true;

		const toCheck = inputsList || state.inputs;

		try {
			await vcsService.checkForUpdates(toCheck, (name, status) => {
				setState("updateStatuses", name, status);
				if (status.error) {
					const toastMeta = toastForError(status.error);
					toast.error(toastMeta.message, toastMeta.id);
				}
			});
		} catch (err) {
			const errorMsg = err instanceof Error ? err.message : String(err);
			const toastMeta = toastForError(errorMsg);
			toast.error(toastMeta.message, toastMeta.id);
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

		batch(() => {
			for (const name of names) {
				setState("updateStatuses", name, (prev) => ({
					...prev,
					commitsBehind: prev?.commitsBehind ?? 0,
					loading: true,
				}));
			}
		});

		const result = await flakeService.updateInputs(state.path, names);

		batch(() => {
			for (const name of names) {
				setState("updateStatuses", name, (prev) => ({
					...prev,
					commitsBehind: prev?.commitsBehind ?? 0,
					loading: false,
				}));
			}
		});

		if (result.ok) {
			await refresh();
		} else {
			toast.error(result.error);
		}
	}

	async function updateAll() {
		batch(() => {
			for (const input of state.inputs) {
				setState("updateStatuses", input.name, (prev) => ({
					...prev,
					commitsBehind: prev?.commitsBehind ?? 0,
					loading: true,
				}));
			}
		});

		const result = await flakeService.updateAll(state.path);

		batch(() => {
			for (const input of state.inputs) {
				setState("updateStatuses", input.name, (prev) => ({
					...prev,
					commitsBehind: prev?.commitsBehind ?? 0,
					loading: false,
				}));
			}
		});

		if (result.ok) {
			await refresh();
		} else {
			toast.error(result.error);
		}
	}

	async function lockToCommit(input: FlakeInput, sha: string): Promise<boolean> {
		const toastId = toast.loading(`Locking ${input.name} to ${sha.substring(0, 7)}...`);

		const lockUrl = vcsService.getLockUrl(input, sha);
		if (!lockUrl) {
			toast.error(`Cannot lock ${input.type} inputs`, toastId);
			return false;
		}

		const result = await flakeService.lockInputToRev(state.path, input.name, lockUrl);

		if (result.ok) {
			toast.success(`Locked ${input.name} to ${sha.substring(0, 7)}`, toastId);
			return true;
		}
		toast.error(result.error, toastId);
		return false;
	}

	function showChangelog(input: FlakeInput) {
		if (!vcsService.supportsChangelog(input)) {
			toast.warning(`Changelog not available for ${input.type} inputs`);
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
