import type { SetStoreFunction } from "solid-js/store";
import { produce } from "solid-js/store";
import type { AppState } from "../context/AppContext";
import type { FlakeMetadata } from "../lib/flake";
import { checkForUpdates, hasGitHubToken } from "../lib/github";
import type { FlakeInput } from "../lib/types";

export function createFlakeLogic(
	state: AppState,
	setState: SetStoreFunction<AppState>,
	flake: FlakeMetadata,
) {
	// Guard to prevent concurrent update checks
	let isCheckingUpdates = false;

	async function checkUpdates(inputsList?: FlakeInput[]) {
		if (isCheckingUpdates) return;
		isCheckingUpdates = true;

		const toCheck = inputsList || state.inputs;
		const tokenMsg = hasGitHubToken()
			? ""
			: " (set GITHUB_TOKEN for higher rate limits)";
		setState("statusMessage", `Checking for updates...${tokenMsg}`);

		try {
			await checkForUpdates(toCheck, (name, status) => {
				setState(
					produce((s) => {
						const newMap = new Map(s.updateStatuses);
						newMap.set(name, status);
						s.updateStatuses = newMap;
					}),
				);
			});
			setState("statusMessage", undefined);
		} catch (err) {
			const errorMsg = err instanceof Error ? err.message : String(err);
			if (errorMsg.includes("rate limit")) {
				setState("statusMessage", `${errorMsg} - set GITHUB_TOKEN env var`);
			} else {
				setState("statusMessage", `Error checking updates: ${errorMsg}`);
			}
			setTimeout(() => setState("statusMessage", undefined), 5000);
		} finally {
			isCheckingUpdates = false;
		}
	}

	async function refresh() {
		setState("statusMessage", "Refreshing...");
		const result = await flake.refresh();
		if (!result.ok) {
			setState("statusMessage", `Error: ${result.error}`);
			setTimeout(() => setState("statusMessage", undefined), 3000);
			return;
		}

		setState("inputs", result.flake.inputs);
		setState("description", result.flake.description);

		// Re-check for updates after refresh
		await checkUpdates(result.flake.inputs);
	}

	async function updateSelected() {
		const selected = state.selectedIndices;
		if (selected.size === 0) {
			setState("statusMessage", "No inputs selected");
			setTimeout(() => setState("statusMessage", undefined), 2000);
			return;
		}

		const names = Array.from(selected)
			.map((i) => state.inputs[i]?.name)
			.filter((n): n is string => !!n);
		setState("statusMessage", `Updating ${names.join(", ")}...`);
		setState("loading", true);

		const result = await flake.updateInputs(names);
		setState("loading", false);

		if (result.success) {
			setState("selectedIndices", new Set<number>());
			await refresh();
			setState("statusMessage", `Updated ${names.length} input(s)`);
		} else {
			setState("statusMessage", `Error: ${result.output}`);
		}

		setTimeout(() => setState("statusMessage", undefined), 3000);
	}

	async function updateAll() {
		setState("statusMessage", "Updating all inputs...");
		setState("loading", true);

		const result = await flake.updateAll();
		setState("loading", false);

		if (result.success) {
			setState("selectedIndices", new Set<number>());
			await refresh();
			setState("statusMessage", "All inputs updated");
		} else {
			setState("statusMessage", `Error: ${result.output}`);
		}

		setTimeout(() => setState("statusMessage", undefined), 3000);
	}

	async function lockToCommit(
		inputName: string,
		sha: string,
		owner: string,
		repo: string,
	): Promise<boolean> {
		setState(
			"statusMessage",
			`Locking ${inputName} to ${sha.substring(0, 7)}...`,
		);

		const result = await flake.lockInputToRev(inputName, sha, owner, repo);

		if (result.success) {
			setState(
				"statusMessage",
				`Locked ${inputName} to ${sha.substring(0, 7)}`,
			);
			await refresh();
			setTimeout(() => setState("statusMessage", undefined), 3000);
			return true;
		} else {
			setState("statusMessage", `Error: ${result.output}`);
			setTimeout(() => setState("statusMessage", undefined), 3000);
			return false;
		}
	}

	return {
		checkUpdates,
		refresh,
		updateSelected,
		updateAll,
		lockToCommit,
	};
}
