import { createStore } from "solid-js/store";
import type { FlakeData } from "../services/flake";
import { flakeService } from "../services/flake";
import { githubService } from "../services/github";
import type { FlakeInput, UpdateStatus } from "../types";

export interface FlakeState {
  path: string;
  inputs: FlakeInput[];
  updateStatuses: Record<string, UpdateStatus>;
  loading: boolean;
  statusMessage: string | undefined;
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
    statusMessage: undefined,
    changelogInput: undefined,
  });

  let isCheckingUpdates = false;
  let statusTimeout: ReturnType<typeof setTimeout> | undefined;

  function setStatusMessage(message: string | undefined, timeout?: number) {
    if (statusTimeout) {
      clearTimeout(statusTimeout);
      statusTimeout = undefined;
    }
    setState("statusMessage", message);
    if (message && timeout) {
      statusTimeout = setTimeout(() => {
        setState("statusMessage", undefined);
        statusTimeout = undefined;
      }, timeout);
    }
  }

  async function checkUpdates(inputsList?: FlakeInput[]) {
    if (isCheckingUpdates) return;
    isCheckingUpdates = true;

    const toCheck = inputsList || state.inputs;
    const tokenMsg = githubService.hasGitHubToken()
      ? ""
      : " (set GITHUB_TOKEN for higher rate limits)";
    setStatusMessage(`Checking for updates...${tokenMsg}`);

    try {
      await githubService.checkForUpdates(toCheck, (name, status) => {
        setState("updateStatuses", name, status);
      });
      setStatusMessage(undefined);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      if (errorMsg.includes("rate limit")) {
        setStatusMessage(`${errorMsg} - set GITHUB_TOKEN env var`, 5000);
      } else {
        setStatusMessage(`Error checking updates: ${errorMsg}`, 5000);
      }
    } finally {
      isCheckingUpdates = false;
    }
  }

  async function refresh() {
    setStatusMessage("Refreshing...");
    const result = await flakeService.refresh(state.path);
    if (!result.ok) {
      setStatusMessage(`Error: ${result.error}`, 3000);
      return;
    }

    setState("inputs", result.data.inputs);
    await checkUpdates(result.data.inputs);
  }

  async function updateSelected(names: string[]) {
    if (names.length === 0) {
      setStatusMessage("No inputs selected", 2000);
      return;
    }

    setStatusMessage(`Updating ${names.join(", ")}...`);
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
      setStatusMessage(`Updated ${names.length} input(s)`, 3000);
    } else {
      setStatusMessage(`Error: ${result.error}`, 3000);
    }
  }

  async function updateAll() {
    setStatusMessage("Updating all inputs...");
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
      setStatusMessage("All inputs updated", 3000);
    } else {
      setStatusMessage(`Error: ${result.error}`, 3000);
    }
  }

  async function lockToCommit(
    inputName: string,
    sha: string,
    owner: string,
    repo: string,
  ): Promise<boolean> {
    setStatusMessage(`Locking ${inputName} to ${sha.substring(0, 7)}...`);

    const result = await flakeService.lockInputToRev(state.path, inputName, sha, owner, repo);

    if (result.ok) {
      setStatusMessage(`Locked ${inputName} to ${sha.substring(0, 7)}`, 3000);
      return true;
    }
    setStatusMessage(`Error: ${result.error}`, 3000);
    return false;
  }

  function showChangelog(input: FlakeInput) {
    if (input.type !== "github") {
      setStatusMessage("Changelog only available for GitHub inputs", 2000);
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
