import { Show, Switch, Match, type Accessor } from "solid-js";
import type { FlakeInput } from "./lib/types";
import { render } from "@opentui/solid";
import { useKeyboard } from "@opentui/solid";

import { FlakeList } from "./components/FlakeList";
import { StatusBar } from "./components/StatusBar";
import { ErrorDialog } from "./components/ErrorDialog";
import { Changelog } from "./components/Changelog";
import { ConfirmDialog } from "./components/ConfirmDialog";

import { theme } from "./lib/theme";
import { AppProvider, useApp } from "./context/AppContext";
import { ChangelogProvider, useChangelog } from "./context/ChangelogContext";

function AppContent() {
  const [appState, appActions] = useApp();
  const [changelogState, changelogActions] = useChangelog();

  // Show changelog for current input
  async function showChangelog() {
    const input = appActions.getCurrentInput();
    if (!input) return;

    if (input.type !== "github") {
      appActions.setStatusMessage("Changelog only available for GitHub inputs");
      setTimeout(() => appActions.setStatusMessage(undefined), 2000);
      return;
    }

    appActions.setView("changelog");

    try {
      await changelogActions.open(input);
    } catch (err) {
      appActions.setStatusMessage(
        `Error loading changelog: ${err instanceof Error ? err.message : err}`
      );
    }
  }

  // Handle locking to a commit
  async function handleLockToCommit() {
    const input = changelogState.input;
    const commit = changelogState.confirmCommit;
    if (!input || !commit || !input.owner || !input.repo) return;

    changelogActions.hideConfirmDialog();

    const success = await appActions.lockToCommit(
      input.name,
      commit.sha,
      input.owner,
      input.repo
    );

    if (success) {
      appActions.setView("list");
      changelogActions.close();
    }
  }

  // Keyboard handling
  useKeyboard((e) => {
    if (e.eventType === "release") return;

    const currentView = appState.view;

    // Error view keys
    if (currentView === "error") {
      if (e.name === "escape" || e.name === "q") {
        process.exit(1);
      }
      return;
    }

    // Changelog view keys
    if (currentView === "changelog") {
      // Handle confirmation dialog keys first
      if (changelogState.showConfirm) {
        switch (e.name) {
          case "y":
            handleLockToCommit();
            break;
          case "n":
          case "escape":
          case "q":
            changelogActions.hideConfirmDialog();
            break;
        }
        return;
      }

      switch (e.name) {
        case "j":
        case "down":
          changelogActions.moveCursor(1);
          break;
        case "k":
        case "up":
          changelogActions.moveCursor(-1);
          break;
        case "return":
          changelogActions.showConfirmDialog();
          break;
        case "escape":
        case "q":
          appActions.setView("list");
          changelogActions.close();
          break;
      }
      return;
    }

    // List view keybindings
    switch (e.name) {
      case "j":
      case "down":
        appActions.moveCursor(1);
        break;
      case "k":
      case "up":
        appActions.moveCursor(-1);
        break;
      case "space":
        appActions.toggleSelection();
        break;
      case "u":
        if (e.shift) {
          appActions.updateAll();
        } else {
          appActions.updateSelected();
        }
        break;
      case "c":
        showChangelog();
        break;
      case "r":
        appActions.refresh();
        break;
      case "escape":
      case "q":
        // If selection exists, clear it; otherwise quit
        if (appState.selectedIndices.size > 0) {
          appActions.clearSelection();
        } else {
          process.exit(0);
        }
        break;
    }
  });

  return (
    <box flexDirection="column" flexGrow={1} backgroundColor={theme.bg}>
      <Switch>
        {/* Error view */}
        <Match when={appState.view === "error"}>
          <ErrorDialog message={appState.error || "Unknown error"} />
        </Match>

        {/* Changelog view */}
        <Match when={appState.view === "changelog"}>
          <Show when={changelogState.input}>
            {(input: Accessor<FlakeInput>) => (
              <>
                <Changelog
                  input={input()}
                  commits={changelogState.commits}
                  loading={changelogState.loading}
                  cursorIndex={changelogState.cursorIndex}
                  lockedIndex={changelogState.lockedIndex}
                />
                <ConfirmDialog
                  visible={changelogState.showConfirm}
                  inputName={input().name}
                  commit={changelogState.confirmCommit}
                />
              </>
            )}
          </Show>
        </Match>

        {/* Main list view */}
        <Match when={appState.view === "list"}>
          {/* Loading state */}
          <Show when={appState.loading && appState.inputs.length === 0}>
            <box flexGrow={1} alignItems="center" justifyContent="center">
              <text fg={theme.warning}>Loading flake metadata...</text>
            </box>
          </Show>

          {/* Flake list */}
          <Show when={!appState.loading || appState.inputs.length > 0}>
            <FlakeList
              inputs={appState.inputs}
              cursorIndex={appState.cursorIndex}
              selectedIndices={appState.selectedIndices}
              updateStatuses={appState.updateStatuses}
            />
          </Show>

          {/* Status bar */}
          <StatusBar
            statusMessage={appState.statusMessage}
            loading={appState.loading}
            selectedCount={appState.selectedIndices.size}
          />
        </Match>
      </Switch>
    </box>
  );
}

function App() {
  return (
    <AppProvider>
      <ChangelogProvider>
        <AppContent />
      </ChangelogProvider>
    </AppProvider>
  );
}

render(() => <App />);
