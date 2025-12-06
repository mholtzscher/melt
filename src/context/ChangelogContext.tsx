import { createContext, useContext, type JSX } from "solid-js";
import { createStore } from "solid-js/store";
import type { FlakeInput, GitHubCommit } from "../lib/types";
import { getChangelog } from "../lib/github";

export interface ChangelogState {
  // Current input being viewed
  input?: FlakeInput;

  // Commit data
  commits: GitHubCommit[];
  lockedIndex: number;

  // UI state
  loading: boolean;
  cursorIndex: number;

  // Confirm dialog state
  showConfirm: boolean;
  confirmCommit?: GitHubCommit;
}

export interface ChangelogActions {
  // Show changelog for an input
  open: (input: FlakeInput) => Promise<void>;

  // Close and reset changelog view
  close: () => void;

  // Navigation
  moveCursor: (delta: number) => void;

  // Confirm dialog
  showConfirmDialog: () => void;
  hideConfirmDialog: () => void;
  getSelectedCommit: () => GitHubCommit | undefined;
}

type ChangelogContextValue = [ChangelogState, ChangelogActions];

const ChangelogContext = createContext<ChangelogContextValue>();

export function ChangelogProvider(props: { children: JSX.Element }) {
  const [state, setState] = createStore<ChangelogState>({
    commits: [],
    lockedIndex: 0,
    loading: false,
    cursorIndex: 0,
    showConfirm: false,
  });

  const actions: ChangelogActions = {
    async open(input: FlakeInput) {
      if (input.type !== "github") {
        throw new Error("Changelog only available for GitHub inputs");
      }

      setState("input", input);
      setState("loading", true);
      setState("cursorIndex", 0);
      setState("commits", []);

      try {
        const result = await getChangelog(input);
        setState("commits", result.commits);
        setState("lockedIndex", result.lockedIndex);
        // Start cursor at the locked commit
        setState("cursorIndex", result.lockedIndex);
      } catch (err) {
        setState("commits", []);
        setState("lockedIndex", 0);
        throw err;
      } finally {
        setState("loading", false);
      }
    },

    close() {
      setState("input", undefined);
      setState("commits", []);
      setState("lockedIndex", 0);
      setState("cursorIndex", 0);
      setState("showConfirm", false);
      setState("confirmCommit", undefined);
    },

    moveCursor(delta: number) {
      const len = state.commits.length;
      if (len === 0) return;
      setState("cursorIndex", (prev) => {
        const next = prev + delta;
        if (next < 0) return 0;
        if (next >= len) return len - 1;
        return next;
      });
    },

    showConfirmDialog() {
      const commit = state.commits[state.cursorIndex];
      if (commit) {
        setState("confirmCommit", commit);
        setState("showConfirm", true);
      }
    },

    hideConfirmDialog() {
      setState("showConfirm", false);
      setState("confirmCommit", undefined);
    },

    getSelectedCommit() {
      return state.commits[state.cursorIndex];
    },
  };

  return (
    <ChangelogContext.Provider value={[state, actions]}>
      {props.children}
    </ChangelogContext.Provider>
  );
}

export function useChangelog(): ChangelogContextValue {
  const context = useContext(ChangelogContext);
  if (!context) {
    throw new Error("useChangelog must be used within a ChangelogProvider");
  }
  return context;
}
