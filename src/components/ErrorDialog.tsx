import { theme, mocha } from "../lib/theme";

interface ErrorDialogProps {
  message: string;
}

export function ErrorDialog(props: ErrorDialogProps) {
  return (
    <box
      flexGrow={1}
      alignItems="center"
      justifyContent="center"
      backgroundColor={theme.bg}
    >
      <box
        flexDirection="column"
        alignItems="center"
        border={true}
        borderColor={theme.error}
        padding={2}
        minWidth={40}
      >
        {/* Error icon/title */}
        <text fg={theme.error} attributes={1}>
          Error
        </text>

        {/* Error message */}
        <box marginTop={1}>
          <text fg={theme.text}>{props.message}</text>
        </box>

        {/* Instructions */}
        <box marginTop={2} flexDirection="row">
          <text fg={theme.textDim}>Press </text>
          <text fg={mocha.lavender}>q/esc</text>
          <text fg={theme.textDim}> to quit</text>
        </box>
      </box>
    </box>
  );
}
