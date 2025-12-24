import { render } from "@opentui/solid";
import "opentui-spinner/solid";
import { App } from "./App";
import { parseArgs } from "./cli";
import { processManager } from "./services/processManager";

process.once("SIGINT", () => {
	processManager.cleanup();
	process.exit(0);
});

process.once("SIGTERM", () => {
	processManager.cleanup();
	process.exit(0);
});

const args = await parseArgs();
render(() => <App flakePath={args.flake} />);
