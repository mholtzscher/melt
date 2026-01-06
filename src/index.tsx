import { render } from "@opentui/solid";
import "opentui-spinner/solid";
import { App } from "./App";
import { runCli } from "./cli";
import { shutdown } from "./shutdown";

process.once("SIGINT", () => {
	void shutdown(0);
});
process.once("SIGTERM", () => {
	void shutdown(0);
});

runCli((flakePath) => {
	render(() => <App flakePath={flakePath} />);
});
