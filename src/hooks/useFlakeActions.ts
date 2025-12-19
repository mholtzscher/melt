import type { Accessor, Setter } from "solid-js";
import { flakeService } from "../services/flake";
import { githubService } from "../services/github";
import type { FlakeInput, UpdateStatus } from "../types";

export interface FlakeActionsProps {
	flakePath: Accessor<string>;
	inputs: Accessor<FlakeInput[]>;
	setInputs: Setter<FlakeInput[]>;
	setDescription: Setter<string | undefined>;
	selectedIndices: Accessor<Set<number>>;
	setSelectedIndices: Setter<Set<number>>;
	setUpdateStatuses: Setter<Map<string, UpdateStatus>>;
	setLoading: Setter<boolean>;
	setStatusMessage: Setter<string | undefined>;
}

export function useFlakeActions(props: FlakeActionsProps) {
	let isCheckingUpdates = false;

	async function checkUpdates(inputsList?: FlakeInput[]) {
		if (isCheckingUpdates) return;
		isCheckingUpdates = true;

		const toCheck = inputsList || props.inputs();
		const tokenMsg = githubService.hasGitHubToken()
			? ""
			: " (set GITHUB_TOKEN for higher rate limits)";
		props.setStatusMessage(`Checking for updates...${tokenMsg}`);

		try {
			await githubService.checkForUpdates(toCheck, (name, status) => {
				props.setUpdateStatuses((prev) => {
					const next = new Map(prev);
					next.set(name, status);
					return next;
				});
			});
			props.setStatusMessage(undefined);
		} catch (err) {
			const errorMsg = err instanceof Error ? err.message : String(err);
			if (errorMsg.includes("rate limit")) {
				props.setStatusMessage(`${errorMsg} - set GITHUB_TOKEN env var`);
			} else {
				props.setStatusMessage(`Error checking updates: ${errorMsg}`);
			}
			setTimeout(() => props.setStatusMessage(undefined), 5000);
		} finally {
			isCheckingUpdates = false;
		}
	}

	async function refresh() {
		props.setStatusMessage("Refreshing...");
		const result = await flakeService.refresh(props.flakePath());
		if (!result.ok) {
			props.setStatusMessage(`Error: ${result.error}`);
			setTimeout(() => props.setStatusMessage(undefined), 3000);
			return;
		}

		props.setInputs(result.data.inputs);
		props.setDescription(result.data.description);
		await checkUpdates(result.data.inputs);
	}

	async function updateSelected() {
		const selected = props.selectedIndices();
		if (selected.size === 0) {
			props.setStatusMessage("No inputs selected");
			setTimeout(() => props.setStatusMessage(undefined), 2000);
			return;
		}

		const names = Array.from(selected)
			.map((i) => props.inputs()[i]?.name)
			.filter((n): n is string => !!n);
		props.setStatusMessage(`Updating ${names.join(", ")}...`);
		props.setLoading(true);

		const result = await flakeService.updateInputs(props.flakePath(), names);
		props.setLoading(false);

		if (result.ok) {
			props.setSelectedIndices(new Set<number>());
			await refresh();
			props.setStatusMessage(`Updated ${names.length} input(s)`);
		} else {
			props.setStatusMessage(`Error: ${result.error}`);
		}

		setTimeout(() => props.setStatusMessage(undefined), 3000);
	}

	async function updateAll() {
		props.setStatusMessage("Updating all inputs...");
		props.setLoading(true);

		const result = await flakeService.updateAll(props.flakePath());
		props.setLoading(false);

		if (result.ok) {
			props.setSelectedIndices(new Set<number>());
			await refresh();
			props.setStatusMessage("All inputs updated");
		} else {
			props.setStatusMessage(`Error: ${result.error}`);
		}

		setTimeout(() => props.setStatusMessage(undefined), 3000);
	}

	async function lockToCommit(
		inputName: string,
		sha: string,
		owner: string,
		repo: string,
	): Promise<boolean> {
		props.setStatusMessage(`Locking ${inputName} to ${sha.substring(0, 7)}...`);

		const result = await flakeService.lockInputToRev(
			props.flakePath(),
			inputName,
			sha,
			owner,
			repo,
		);

		if (result.ok) {
			props.setStatusMessage(`Locked ${inputName} to ${sha.substring(0, 7)}`);
			await refresh();
			setTimeout(() => props.setStatusMessage(undefined), 3000);
			return true;
		}
		props.setStatusMessage(`Error: ${result.error}`);
		setTimeout(() => props.setStatusMessage(undefined), 3000);
		return false;
	}

	return {
		checkUpdates,
		refresh,
		updateSelected,
		updateAll,
		lockToCommit,
	};
}
