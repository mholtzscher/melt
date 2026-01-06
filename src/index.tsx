import { render } from "@opentui/solid";
import "opentui-spinner/solid";
import { App } from "./App";
import { parseArgs } from "./cli";
import { shutdown } from "./shutdown";

process.once("SIGINT", () => {
	void shutdown(0);
});
process.once("SIGTERM", () => {
	void shutdown(0);
});

const args = await parseArgs();
render(() => <App flakePath={args.flake} />);
