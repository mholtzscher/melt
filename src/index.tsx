import { render } from "@opentui/solid";
import "opentui-spinner/solid";
import { App } from "./App";
import { parseArgs } from "./cli";
import { interruptAll } from "./runtime";

let isShuttingDown = false;

async function shutdown() {
	if (isShuttingDown) return;
	isShuttingDown = true;

	// Give fibers a chance to clean up, but don't wait forever
	const timeout = new Promise<void>((resolve) => setTimeout(resolve, 1000));
	await Promise.race([interruptAll(), timeout]);

	process.exit(0);
}

process.once("SIGINT", shutdown);
process.once("SIGTERM", shutdown);

const args = await parseArgs();
render(() => <App flakePath={args.flake} />);
