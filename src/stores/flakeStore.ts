import { createStore } from "solid-js/store";
import type { FlakeData } from "../services/flake";
import { flakeService } from "../services/flake";
import { githubService } from "../services/github";
import type { FlakeInput, UpdateStatus } from "../types";
import { toast } from "@opentui-ui/toast";

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
	lockToCommit: (
		inputName: string,
		sha: string,
		owner: string,
		repo: string,
	) => Promise<boolean>;
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

	async function checkUpdates(inputsList?: FlakeInput[]) {
		if (isCheckingUpdates) return;
		isCheckingUpdates = true;

		const toCheck = inputsList || state.inputs;
		const tokenMsg = githubService.hasGitHubToken()
			? ""
			: " (set GITHUB_TOKEN for higher rate limits)";
		const toastId = toast.loading(`Checking for updates...${tokenMsg}`);

		try {
			await githubService.checkForUpdates(toCheck, (name, status) => {
				setState("updateStatuses", name, status);
			});
			toast.dismiss(toastId);
		} catch (err) {
			const errorMsg = err instanceof Error ? err.message : String(err);
			if (errorMsg.includes("rate limit")) {
				toast.error(`${errorMsg} - set GITHUB_TOKEN env var`, {
					id: toastId,
					duration: 5000,
				});
			} else {
				toast.error(`Error checking updates: ${errorMsg}`, {
					id: toastId,
					duration: 5000,
				});
			}
		} finally {
			isCheckingUpdates = false;
		}
	}

	async function refresh() {
		const loadingId = toast.loading("Refreshing...");

		const result = await flakeService.refresh(state.path);
		if (!result.ok) {
			toast.error(`Error: ${result.error}`, { id: loadingId, duration: 3000 });
			return;
		}

		setState("inputs", result.data.inputs);
		toast.dismiss(loadingId);

		await checkUpdates(result.data.inputs);
	}

	async function updateSelected(names: string[]) {
		if (names.length === 0) {
			toast.warning("No inputs selected", { duration: 2000 });
			return;
		}

		const loadingId = toast.loading(`Updating ${names.join(", ")}...`);
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

		setState("loading", false);

		if (result.ok) {
			await refresh();
			toast.success(`Updated ${names.length} input(s)`, {
				id: loadingId,
				duration: 3000,
			});
		} else {
			toast.error(`Error: ${result.error}`, {
				id: loadingId,
				duration: 3000,
			});
		}
	}

	async function updateAll() {
		const loadingId = toast.loading("Updating all inputs...");
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

		setState("loading", false);

		if (result.ok) {
			await refresh();
			toast.success("All inputs updated", {
				id: loadingId,
				duration: 3000,
			});
		} else {
			toast.error(`Error: ${result.error}`, {
				id: loadingId,
				duration: 3000,
			});
		}
	}

	async function lockToCommit(
		inputName: string,
		sha: string,
		owner: string,
		repo: string,
	): Promise<boolean> {
		const loadingId = toast.loading(
			`Locking ${inputName} to ${sha.substring(0, 7)}...`,
		);

		const result = await flakeService.lockInputToRev(
			state.path,
			inputName,
			sha,
			owner,
			repo,
		);

		if (result.ok) {
			toast.success(`Locked ${inputName} to ${sha.substring(0, 7)}`, {
				id: loadingId,
				duration: 3000,
			});
			return true;
		}
		toast.error(`Error: ${result.error}`, {
			id: loadingId,
			duration: 3000,
		});
		return false;
	}

	function showChangelog(input: FlakeInput) {
		if (input.type !== "github") {
			toast.info("Changelog only available for GitHub inputs", {
				duration: 2000,
			});
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
