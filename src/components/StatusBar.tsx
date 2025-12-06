import { Show } from "solid-js";
import { theme, mocha } from "../lib/theme";

interface StatusBarProps {
  statusMessage?: string;
  loading?: boolean;
  selectedCount: number;
}

export function StatusBar(props: StatusBarProps) {
  return (
    <box
      flexDirection="row"
      backgroundColor={theme.bgDark}
      paddingLeft={1}
      paddingRight={1}
    >
      {/* Status message or keybinds */}
      <Show
        when={props.statusMessage}
        fallback={
          <box flexDirection="row" flexGrow={1}>
            <text fg={mocha.lavender}>j</text>
            <text fg={theme.textDim}>/</text>
            <text fg={mocha.lavender}>k</text>
            <text fg={theme.textDim}>:nav </text>

            <text fg={mocha.lavender}>space</text>
            <text fg={theme.textDim}>:select </text>

            <text fg={mocha.lavender}>u</text>
            <text fg={theme.textDim}>:update </text>

            <text fg={mocha.lavender}>U</text>
            <text fg={theme.textDim}>:all </text>

            <text fg={mocha.lavender}>c</text>
            <text fg={theme.textDim}>:log </text>

            <text fg={mocha.lavender}>r</text>
            <text fg={theme.textDim}>:refresh </text>

            <text fg={mocha.lavender}>q</text>
            <text fg={theme.textDim}>:quit</text>
          </box>
        }
      >
        <text fg={props.loading ? theme.warning : theme.info}>
          {props.statusMessage}
        </text>
      </Show>

      {/* Selection count */}
      <Show when={props.selectedCount > 0}>
        <box marginLeft={2}>
          <text fg={theme.selected}>
            {props.selectedCount} selected
          </text>
        </box>
      </Show>
    </box>
  );
}
