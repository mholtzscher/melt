import { binary, command, optional, positional, run, string } from "cmd-ts";

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
	handler: (args) => args,
});

export interface CliArgs {
	flake?: string;
}

export async function parseArgs(): Promise<CliArgs> {
	const result = await run(binary(app), process.argv);
	return result;
}
