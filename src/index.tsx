import { createSignal, onMount, Show, Switch, Match } from "solid-js";
import { render } from "@opentui/solid";
import { useKeyboard } from "@opentui/solid";

import { FlakeList } from "./components/FlakeList";
import { StatusBar } from "./components/StatusBar";
import { ErrorDialog } from "./components/ErrorDialog";
import { Changelog } from "./components/Changelog";
import { ConfirmDialog } from "./components/ConfirmDialog";

import { theme } from "./lib/theme";
import {
  hasFlakeNix,
  getFlakeMetadata,
  updateInputs,
  updateAll,
  lockInputToRev,
} from "./lib/flake";
import { getChangelog, checkForUpdates, hasGitHubToken } from "./lib/github";
import type { FlakeInput, GitHubCommit, AppView, UpdateStatus } from "./lib/types";

// Get the flake path from command line args or use current working directory
const flakePath = process.argv[2] || process.cwd();

function App() {
  // App state
  const [view, setView] = createSignal<AppView>("list");
  const [inputs, setInputs] = createSignal<FlakeInput[]>([]);
  const [cursorIndex, setCursorIndex] = createSignal(0);
  const [selectedIndices, setSelectedIndices] = createSignal<Set<number>>(
    new Set()
  );
  const [loading, setLoading] = createSignal(true);
  const [statusMessage, setStatusMessage] = createSignal<string | undefined>();
  const [error, setError] = createSignal<string | undefined>();
  const [description, setDescription] = createSignal<string | undefined>();

  // Changelog state
  const [changelogInput, setChangelogInput] = createSignal<
    FlakeInput | undefined
  >();
  const [changelogCommits, setChangelogCommits] = createSignal<GitHubCommit[]>(
    []
  );
  const [changelogLoading, setChangelogLoading] = createSignal(false);
  const [changelogCursor, setChangelogCursor] = createSignal(0);
  const [changelogLockedIndex, setChangelogLockedIndex] = createSignal(0);

  // Confirm dialog state
  const [showConfirm, setShowConfirm] = createSignal(false);
  const [confirmCommit, setConfirmCommit] = createSignal<GitHubCommit | undefined>();

  // Update status state
  const [updateStatuses, setUpdateStatuses] = createSignal<Map<string, UpdateStatus>>(
    new Map()
  );

  // Load flake data on mount
  onMount(async () => {
    try {
      const hasFlake = await hasFlakeNix(flakePath);
      if (!hasFlake) {
        setError(`No flake.nix found in ${flakePath}`);
        setView("error");
        setLoading(false);
        return;
      }

      const metadata = await getFlakeMetadata(flakePath);
      setInputs(metadata.inputs);
      setDescription(metadata.description);
      setLoading(false);

      // Check for updates in background
      checkUpdates(metadata.inputs);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setView("error");
      setLoading(false);
    }
  });

  // Guard to prevent concurrent update checks
  let isCheckingUpdates = false;

  // Check for updates on all inputs
  async function checkUpdates(inputsList?: FlakeInput[]) {
    if (isCheckingUpdates) return;
    isCheckingUpdates = true;

    const toCheck = inputsList || inputs();
    const tokenMsg = hasGitHubToken() ? "" : " (set GITHUB_TOKEN for higher rate limits)";
    setStatusMessage(`Checking for updates...${tokenMsg}`);
    
    try {
      const statuses = await checkForUpdates(toCheck);
      setUpdateStatuses(statuses);
      setStatusMessage(undefined);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      if (errorMsg.includes("rate limit")) {
        setStatusMessage(`${errorMsg} - set GITHUB_TOKEN env var`);
      } else {
        setStatusMessage(`Error checking updates: ${errorMsg}`);
      }
      setTimeout(() => setStatusMessage(undefined), 5000);
    } finally {
      isCheckingUpdates = false;
    }
  }

  // Refresh flake data and check for updates
  async function refresh() {
    setStatusMessage("Refreshing...");
    try {
      const metadata = await getFlakeMetadata(flakePath);
      setInputs(metadata.inputs);
      setDescription(metadata.description);
      
      // Re-check for updates after refresh
      await checkUpdates(metadata.inputs);
    } catch (err) {
      setStatusMessage(`Error: ${err instanceof Error ? err.message : err}`);
      setTimeout(() => setStatusMessage(undefined), 3000);
    }
  }

  // Navigation helpers
  function moveCursor(delta: number) {
    const len = inputs().length;
    if (len === 0) return;
    setCursorIndex((prev) => {
      const next = prev + delta;
      if (next < 0) return 0;
      if (next >= len) return len - 1;
      return next;
    });
  }

  function moveChangelogCursor(delta: number) {
    const len = changelogCommits().length;
    if (len === 0) return;
    setChangelogCursor((prev) => {
      const next = prev + delta;
      if (next < 0) return 0;
      if (next >= len) return len - 1;
      return next;
    });
  }

  // Selection helpers
  function toggleSelection() {
    const idx = cursorIndex();
    setSelectedIndices((prev) => {
      const next = new Set(prev);
      if (next.has(idx)) {
        next.delete(idx);
      } else {
        next.add(idx);
      }
      return next;
    });
  }

  // Update handlers
  async function handleUpdateSelected() {
    const selected = selectedIndices();
    if (selected.size === 0) {
      setStatusMessage("No inputs selected");
      setTimeout(() => setStatusMessage(undefined), 2000);
      return;
    }

    const inputsList = inputs();
    const names = Array.from(selected).map((i) => inputsList[i]?.name).filter((n): n is string => !!n);
    setStatusMessage(`Updating ${names.join(", ")}...`);
    setLoading(true);

    const result = await updateInputs(names, flakePath);
    setLoading(false);

    if (result.success) {
      setSelectedIndices(new Set<number>());
      await refresh();
      setStatusMessage(`Updated ${names.length} input(s)`);
    } else {
      setStatusMessage(`Error: ${result.output}`);
    }

    setTimeout(() => setStatusMessage(undefined), 3000);
  }

  async function handleUpdateAll() {
    setStatusMessage("Updating all inputs...");
    setLoading(true);

    const result = await updateAll(flakePath);
    setLoading(false);

    if (result.success) {
      setSelectedIndices(new Set<number>());
      await refresh();
      setStatusMessage("All inputs updated");
    } else {
      setStatusMessage(`Error: ${result.output}`);
    }

    setTimeout(() => setStatusMessage(undefined), 3000);
  }

  // Changelog handler
  async function showChangelog() {
    const input = inputs()[cursorIndex()];
    if (!input) return;

    if (input.type !== "github") {
      setStatusMessage("Changelog only available for GitHub inputs");
      setTimeout(() => setStatusMessage(undefined), 2000);
      return;
    }

    setChangelogInput(input);
    setChangelogLoading(true);
    setChangelogCursor(0);
    setView("changelog");

    try {
      const result = await getChangelog(input);
      setChangelogCommits(result.commits);
      setChangelogLockedIndex(result.lockedIndex);
      // Start cursor at the locked commit
      setChangelogCursor(result.lockedIndex);
    } catch (err) {
      setChangelogCommits([]);
      setChangelogLockedIndex(0);
      setStatusMessage(
        `Error loading changelog: ${err instanceof Error ? err.message : err}`
      );
    } finally {
      setChangelogLoading(false);
    }
  }

  // Lock to selected commit handler
  async function handleLockToCommit() {
    const input = changelogInput();
    const commit = confirmCommit();
    if (!input || !commit || !input.owner || !input.repo) return;

    setShowConfirm(false);
    setStatusMessage(`Locking ${input.name} to ${commit.shortSha}...`);

    const result = await lockInputToRev(
      input.name,
      commit.sha,
      input.owner,
      input.repo,
      flakePath
    );

    if (result.success) {
      setStatusMessage(`Locked ${input.name} to ${commit.shortSha}`);
      // Go back to list view and refresh
      setView("list");
      setChangelogCommits([]);
      setChangelogInput(undefined);
      setConfirmCommit(undefined);
      await refresh();
    } else {
      setStatusMessage(`Error: ${result.output}`);
    }

    setTimeout(() => setStatusMessage(undefined), 3000);
  }

  // Keyboard handling
  useKeyboard((e) => {
    if (e.eventType === "release") return;

    const currentView = view();

    if (currentView === "error") {
      if (e.name === "q" || e.name === "escape") {
        process.exit(1);
      }
      return;
    }

    if (currentView === "changelog") {
      // Handle confirmation dialog keys first
      if (showConfirm()) {
        switch (e.name) {
          case "y":
            handleLockToCommit();
            break;
          case "n":
          case "escape":
            setShowConfirm(false);
            setConfirmCommit(undefined);
            break;
        }
        return;
      }

      switch (e.name) {
        case "j":
        case "down":
          moveChangelogCursor(1);
          break;
        case "k":
        case "up":
          moveChangelogCursor(-1);
          break;
        case "return":
          // Show confirmation dialog to lock to selected commit
          const commits = changelogCommits();
          const selectedCommit = commits[changelogCursor()];
          if (selectedCommit) {
            setConfirmCommit(selectedCommit);
            setShowConfirm(true);
          }
          break;
        case "q":
        case "escape":
          setView("list");
          setChangelogCommits([]);
          setChangelogInput(undefined);
          break;
      }
      return;
    }

    // List view keybindings
    switch (e.name) {
      case "j":
      case "down":
        moveCursor(1);
        break;
      case "k":
      case "up":
        moveCursor(-1);
        break;
      case "space":
        toggleSelection();
        break;
      case "u":
        handleUpdateSelected();
        break;
      case "U":
        handleUpdateAll();
        break;
      case "c":
        showChangelog();
        break;
      case "r":
        refresh();
        break;
      case "q":
        process.exit(0);
      case "escape":
        // Clear selection on escape
        setSelectedIndices(new Set<number>());
        break;
    }
  });

  return (
    <box
      flexDirection="column"
      flexGrow={1}
      backgroundColor={theme.bg}
    >
      <Switch>
        {/* Error view */}
        <Match when={view() === "error"}>
          <ErrorDialog message={error() || "Unknown error"} />
        </Match>

        {/* Changelog view */}
        <Match when={view() === "changelog"}>
          <Show when={changelogInput()}>
            {(input: () => FlakeInput) => (
              <>
                <Changelog
                  input={input()}
                  commits={changelogCommits()}
                  loading={changelogLoading()}
                  cursorIndex={changelogCursor()}
                  lockedIndex={changelogLockedIndex()}
                />
                <ConfirmDialog
                  visible={showConfirm()}
                  inputName={input().name}
                  commit={confirmCommit()}
                />
              </>
            )}
          </Show>
        </Match>

        {/* Main list view */}
        <Match when={view() === "list"}>
          {/* Loading state */}
          <Show when={loading() && inputs().length === 0}>
            <box flexGrow={1} alignItems="center" justifyContent="center">
              <text fg={theme.warning}>Loading flake metadata...</text>
            </box>
          </Show>

          {/* Flake list */}
          <Show when={!loading() || inputs().length > 0}>
            <FlakeList
              inputs={inputs()}
              cursorIndex={cursorIndex()}
              selectedIndices={selectedIndices()}
              updateStatuses={updateStatuses()}
            />
          </Show>

          {/* Status bar */}
          <StatusBar
            statusMessage={statusMessage()}
            loading={loading()}
            selectedCount={selectedIndices().size}
          />
        </Match>
      </Switch>
    </box>
  );
}

render(() => <App />);
