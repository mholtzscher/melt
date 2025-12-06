import { For, Show } from "solid-js";
import type { FlakeInput } from "../lib/types";
import { theme } from "../lib/theme";
import { formatRelativeTime } from "../lib/flake";

interface FlakeListProps {
  inputs: FlakeInput[];
  cursorIndex: number;
  selectedIndices: Set<number>;
}

function getTypeBadgeColor(type: FlakeInput["type"]): string {
  switch (type) {
    case "github":
      return theme.github;
    case "gitlab":
      return theme.gitlab;
    case "sourcehut":
      return theme.sourcehut;
    case "path":
      return theme.path;
    case "git":
      return theme.git;
    default:
      return theme.other;
  }
}

export function FlakeList(props: FlakeListProps) {
  return (
    <box
      flexDirection="column"
      flexGrow={1}
      paddingLeft={1}
      paddingRight={1}
    >
      {/* Column headers */}
      <box flexDirection="row">
        <box width={5}>
          <text fg={theme.textDim}> </text>
        </box>
        <box width={35}>
          <text fg={theme.textDim}>NAME</text>
        </box>
        <box width={12}>
          <text fg={theme.textDim}>TYPE</text>
        </box>
        <box width={10}>
          <text fg={theme.textDim}>REV</text>
        </box>
        <text fg={theme.textDim}>UPDATED</text>
      </box>

      <For each={props.inputs}>
        {(input, index) => {
          const isSelected = () => props.selectedIndices.has(index());
          const isCursor = () => props.cursorIndex === index();
          const badgeColor = getTypeBadgeColor(input.type);

          return (
            <box
              flexDirection="row"
              backgroundColor={isCursor() ? theme.bgHighlight : undefined}
            >
              {/* Selection indicator */}
              <box width={5}>
                <text
                  fg={isSelected() ? theme.selected : theme.textDim}
                  attributes={isSelected() ? 1 : 0}
                >
                  {isSelected() ? "[x] " : "[ ] "}
                </text>
              </box>

              {/* Input name */}
              <box width={35}>
                <text
                  fg={isCursor() ? theme.cursor : theme.text}
                  attributes={isCursor() ? 1 : 0}
                >
                  {input.name}
                </text>
              </box>

              {/* Type badge */}
              <box width={12}>
                <text fg={badgeColor}>{input.type.padEnd(10)}</text>
              </box>

              {/* Short revision */}
              <box width={10}>
                <text fg={theme.accent}>{input.shortRev}</text>
              </box>

              {/* Last modified (relative time) */}
              <text fg={theme.textMuted}>
                {formatRelativeTime(input.lastModified)}
              </text>
            </box>
          );
        }}
      </For>

      {/* Empty state */}
      <Show when={props.inputs.length === 0}>
        <box justifyContent="center" paddingTop={2} paddingBottom={2}>
          <text fg={theme.textMuted}>No flake inputs found</text>
        </box>
      </Show>
    </box>
  );
}
