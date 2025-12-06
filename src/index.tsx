import { createSignal, onMount, Show, Switch, Match } from "solid-js";
import { render } from "@opentui/solid";
import { useKeyboard } from "@opentui/solid";

import { FlakeList } from "./components/FlakeList";
import { StatusBar } from "./components/StatusBar";
import { ErrorDialog } from "./components/ErrorDialog";
import { Changelog } from "./components/Changelog";

import { theme, mocha } from "./lib/theme";
import {
  hasFlakeNix,
  getFlakeMetadata,
  updateInputs,
  updateAll,
} from "./lib/flake";
import { getChangelog } from "./lib/github";
import type { FlakeInput, GitHubCommit, AppView } from "./lib/types";

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
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setView("error");
      setLoading(false);
    }
  });

  // Refresh flake data
  async function refresh() {
    try {
      const metadata = await getFlakeMetadata(flakePath);
      setInputs(metadata.inputs);
      setDescription(metadata.description);
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
      const commits = await getChangelog(input);
      setChangelogCommits(commits);
    } catch (err) {
      setChangelogCommits([]);
      setStatusMessage(
        `Error loading changelog: ${err instanceof Error ? err.message : err}`
      );
    } finally {
      setChangelogLoading(false);
    }
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
      switch (e.name) {
        case "j":
        case "down":
          moveChangelogCursor(1);
          break;
        case "k":
        case "up":
          moveChangelogCursor(-1);
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
              <Changelog
                input={input()}
                commits={changelogCommits()}
                loading={changelogLoading()}
                cursorIndex={changelogCursor()}
              />
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
