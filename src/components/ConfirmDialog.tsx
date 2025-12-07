import { Show } from "solid-js";
import type { GitHubCommit } from "../lib/types";
import { theme, mocha } from "../lib/theme";

interface ConfirmDialogProps {
  visible: boolean;
  inputName: string;
  commit: GitHubCommit | undefined;
}

export function ConfirmDialog(props: ConfirmDialogProps) {
  return (
    <Show when={props.visible && props.commit}>
      <box
        position="absolute"
        top={0}
        left={0}
        right={0}
        bottom={0}
        alignItems="center"
        justifyContent="center"
      >
        <box
          flexDirection="column"
          backgroundColor={theme.bgDark}
          borderStyle="rounded"
          borderColor={theme.accent}
          padding={1}
          minWidth={45}
        >
          {/* Title */}
          <box height={1} flexDirection="row">
            <text fg={theme.text} attributes={1}>
              Lock {props.inputName} to {props.commit?.shortSha}?
            </text>
          </box>

          {/* Commit message preview */}
          <box height={1} marginTop={1}>
            <text fg={theme.textDim}>
              {props.commit?.message && props.commit.message.length > 40
                ? props.commit.message.substring(0, 40) + "..."
                : props.commit?.message}
            </text>
          </box>

          {/* Actions */}
          <box height={1} marginTop={1} flexDirection="row" justifyContent="center">
            <text fg={mocha.green}>[y]</text>
            <text fg={theme.textDim}> confirm  </text>
            <text fg={mocha.red}>[n/q]</text>
            <text fg={theme.textDim}> cancel</text>
          </box>
        </box>
      </box>
    </Show>
  );
}
