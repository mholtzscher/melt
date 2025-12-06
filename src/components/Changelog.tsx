import { For, Show, createEffect } from "solid-js";
import type { GitHubCommit, FlakeInput } from "../lib/types";
import { theme, mocha } from "../lib/theme";
import type { ScrollBoxRenderable } from "@opentui/core";

interface ChangelogProps {
  input: FlakeInput;
  commits: GitHubCommit[];
  loading: boolean;
  cursorIndex: number;
  lockedIndex: number;
}

export function Changelog(props: ChangelogProps) {
  let scrollBoxRef: ScrollBoxRenderable | undefined;

  // Scroll to keep cursor visible
  createEffect(() => {
    const cursor = props.cursorIndex;
    if (scrollBoxRef) {
      scrollBoxRef.scrollTop = Math.max(0, cursor - 5);
    }
  });

  return (
    <box flexDirection="column" flexGrow={1}>
      {/* Header */}
      <box
        flexDirection="row"
        backgroundColor={theme.bgDark}
        paddingLeft={1}
        paddingRight={1}
        flexShrink={0}
        height={1}
      >
        <text fg={theme.accent} attributes={1}>
          Changelog: {props.input.name}
        </text>
        <text fg={theme.textDim}> ({props.input.url})</text>
      </box>

      {/* Loading state */}
      <Show when={props.loading}>
        <box paddingLeft={1} flexGrow={1}>
          <text fg={theme.warning}>Loading commits...</text>
        </box>
      </Show>

      {/* Commits list */}
      <Show when={!props.loading}>
        <Show
          when={props.commits.length > 0}
          fallback={
            <box paddingLeft={1} flexGrow={1}>
              <text fg={theme.success}>Already up to date!</text>
            </box>
          }
        >
          <scrollbox
            ref={scrollBoxRef}
            flexGrow={1}
            flexShrink={1}
            paddingLeft={1}
            paddingRight={1}
            overflow="hidden"
          >
            <box flexDirection="column">
              <For each={props.commits}>
                {(commit, index) => {
                  const isCursor = () => props.cursorIndex === index();
                  const isLocked = () => commit.isLocked === true;

                  return (
                    <box
                      flexDirection="row"
                      backgroundColor={isCursor() ? theme.bgHighlight : undefined}
                    >
                      {/* Lock indicator */}
                      <box width={3}>
                        <text fg={mocha.yellow}>
                          {isLocked() ? "\u{1F512}" : "  "}
                        </text>
                      </box>

                      {/* Short SHA */}
                      <box width={9}>
                        <text fg={isLocked() ? mocha.yellow : mocha.peach}>
                          {commit.shortSha}
                        </text>
                      </box>

                      {/* Author */}
                      <box width={16}>
                        <text fg={mocha.blue}>
                          {commit.author.length > 14
                            ? commit.author.substring(0, 14) + ".."
                            : commit.author.padEnd(14)}
                        </text>
                      </box>

                      {/* Date */}
                      <box width={10}>
                        <text fg={theme.textDim}>{commit.date.padEnd(8)}</text>
                      </box>

                      {/* Message (truncated) */}
                      <text
                        fg={isCursor() ? theme.cursor : isLocked() ? mocha.yellow : theme.text}
                        attributes={isCursor() || isLocked() ? 1 : 0}
                      >
                        {commit.message.length > 55
                          ? commit.message.substring(0, 55) + "..."
                          : commit.message}
                      </text>
                    </box>
                  );
                }}
              </For>
            </box>
          </scrollbox>
        </Show>
      </Show>

      {/* Footer with keybinds */}
      <box
        flexDirection="row"
        backgroundColor={theme.bgDark}
        paddingLeft={1}
        paddingRight={1}
        flexShrink={0}
        height={1}
      >
        <text fg={mocha.lavender}>j</text>
        <text fg={theme.textDim}>/</text>
        <text fg={mocha.lavender}>k</text>
        <text fg={theme.textDim}>:scroll </text>

        <text fg={mocha.lavender}>Enter</text>
        <text fg={theme.textDim}>:lock </text>

        <text fg={mocha.lavender}>q</text>
        <text fg={theme.textDim}>/</text>
        <text fg={mocha.lavender}>Esc</text>
        <text fg={theme.textDim}>:back</text>

        <box flexGrow={1} />

        <text fg={theme.textMuted}>
          {props.lockedIndex} ahead
        </text>
        <text fg={mocha.yellow}> {"\u{1F512}"} </text>
        <text fg={theme.textMuted}>
          {props.commits.length - props.lockedIndex - 1} behind
        </text>
      </box>
    </box>
  );
}
