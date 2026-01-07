import { render } from "@opentui/solid";
import "opentui-spinner/solid";
import { binary, command, optional, positional, run, string } from "cmd-ts";
import { App } from "./App";
import { processManager } from "./services/processManager";

process.once("SIGINT", () => {
	processManager.cleanup();
	process.exit(0);
});

process.once("SIGTERM", () => {
	processManager.cleanup();
	process.exit(0);
});

const app = command({
	name: "melt",
	description: "Interactive TUI for managing Nix flake inputs",
	args: {
		flake: positional({
			type: optional(string),
			displayName: "flake",
			description: "Path to flake directory or flake.nix file (defaults to current directory)",
		}),
	},
	handler: (args) => {
		render(() => <App flakePath={args.flake} />);
	},
});

run(binary(app), process.argv);
