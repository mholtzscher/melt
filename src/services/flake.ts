import { dirname, resolve } from "node:path";
import { Effect } from "effect";
import type { FlakeInput, FlakeInputType, NixFlakeMetadataResponse } from "../types";
import {
	CommandAbortedError,
	FlakeMetadataError,
	FlakeNotFoundError,
	JsonParseError,
	NixCommandError,
} from "../types/errors";

export interface FlakeData {
	path: string;
	description?: string;
	inputs: FlakeInput[];
}

function resolveFlakePath(path: string): string {
	const resolved = resolve(path);
	if (resolved.endsWith("flake.nix")) {
		return dirname(resolved);
	}
	return resolved;
}

function hasFlakeNix(path: string = "."): Effect.Effect<boolean> {
	return Effect.promise(() => Bun.file(`${path}/flake.nix`).exists());
}

function getInputType(locked?: { type: string }, original?: { type: string }): FlakeInputType {
	const type = locked?.type || original?.type || "other";

	switch (type) {
		case "github":
			return "github";
		case "gitlab":
			return "gitlab";
		case "sourcehut":
			return "sourcehut";
		case "path":
			return "path";
		case "git":
			return "git";
		default:
			return "other";
	}
}

function getInputUrl(
	locked?: {
		type: string;
		owner?: string;
		repo?: string;
		url?: string;
		path?: string;
	},
	original?: {
		type: string;
		owner?: string;
		repo?: string;
		url?: string;
		path?: string;
	},
): string {
	const data = original || locked;
	if (!data) return "unknown";

	switch (data.type) {
		case "github":
			return `github:${data.owner}/${data.repo}`;
		case "gitlab":
			return `gitlab:${data.owner}/${data.repo}`;
		case "sourcehut":
			return `sourcehut:${data.owner}/${data.repo}`;
		case "path":
			return data.path || "path:unknown";
		case "git":
			return data.url || "git:unknown";
		default:
			return data.url || "unknown";
	}
}

function parseInputs(data: NixFlakeMetadataResponse): FlakeInput[] {
	const rootNode = data.locks.nodes[data.locks.root];
	if (!rootNode?.inputs) {
		return [];
	}

	const directInputNames = Object.keys(rootNode.inputs);
	const inputs: FlakeInput[] = [];

	for (const name of directInputNames) {
		const inputRef = rootNode.inputs[name];
		const nodeName = Array.isArray(inputRef) ? inputRef[0] : inputRef;

		if (typeof nodeName !== "string") continue;

		const node = data.locks.nodes[nodeName];
		if (!node?.locked) continue;

		const locked = node.locked;
		const original = node.original;

		inputs.push({
			name,
			type: getInputType(locked, original),
			owner: locked.owner || original?.owner,
			repo: locked.repo || original?.repo,
			ref: original?.ref,
			url: getInputUrl(locked, original),
			rev: locked.rev || "",
			shortRev: locked.rev?.substring(0, 7) || "",
			lastModified: locked.lastModified || 0,
		});
	}

	return inputs;
}

const runNixCommand = (args: string[]): Effect.Effect<string, NixCommandError | CommandAbortedError> =>
	Effect.async((resume, signal) => {
		const proc = Bun.spawn(["nix", ...args], {
			signal,
			stdout: "pipe",
			stderr: "pipe",
		});

		Promise.all([proc.stdout.text(), proc.stderr.text(), proc.exited])
			.then(([stdout, stderr, exitCode]) => {
				if (signal.aborted) {
					resume(Effect.fail(new CommandAbortedError({ command: ["nix", ...args] })));
					return;
				}

				if (exitCode !== 0) {
					resume(
						Effect.fail(
							new NixCommandError({
								command: ["nix", ...args],
								exitCode,
								stderr,
							}),
						),
					);
					return;
				}

				resume(Effect.succeed(stdout || stderr));
			})
			.catch((err) => {
				if (err instanceof Error && err.name === "AbortError") {
					resume(Effect.fail(new CommandAbortedError({ command: ["nix", ...args] })));
				} else {
					resume(
						Effect.fail(
							new NixCommandError({
								command: ["nix", ...args],
								exitCode: 1,
								stderr: err instanceof Error ? err.message : String(err),
							}),
						),
					);
				}
			});
	});

const fetchMetadata = (
	path: string,
): Effect.Effect<NixFlakeMetadataResponse, NixCommandError | CommandAbortedError | JsonParseError> =>
	Effect.gen(function* () {
		const stdout = yield* runNixCommand(["flake", "metadata", "--json", path]);
		return yield* Effect.try({
			try: () => JSON.parse(stdout) as NixFlakeMetadataResponse,
			catch: (cause) => new JsonParseError({ input: stdout, cause }),
		});
	});

export const flakeService = {
	load: (pathArg?: string): Effect.Effect<FlakeData, FlakeNotFoundError | FlakeMetadataError> =>
		Effect.gen(function* () {
			const flakePath = resolveFlakePath(pathArg || process.cwd());

			const hasFlake = yield* hasFlakeNix(flakePath);
			if (!hasFlake) {
				return yield* Effect.fail(new FlakeNotFoundError({ path: flakePath }));
			}

			const metadata = yield* fetchMetadata(flakePath).pipe(
				Effect.mapError(
					(e) =>
						new FlakeMetadataError({
							message: `Failed to load flake metadata: ${e.message}`,
							cause: e,
						}),
				),
			);

			return {
				path: flakePath,
				description: metadata.description,
				inputs: parseInputs(metadata),
			};
		}),

	refresh: (path: string): Effect.Effect<FlakeData, FlakeMetadataError> =>
		Effect.gen(function* () {
			const metadata = yield* fetchMetadata(path).pipe(
				Effect.mapError(
					(e) =>
						new FlakeMetadataError({
							message: `Failed to refresh flake metadata: ${e.message}`,
							cause: e,
						}),
				),
			);

			return {
				path,
				description: metadata.description,
				inputs: parseInputs(metadata),
			};
		}),

	updateInputs: (path: string, inputNames: string[]): Effect.Effect<string, NixCommandError | CommandAbortedError> => {
		if (inputNames.length === 0) {
			return Effect.succeed("No inputs to update");
		}
		return runNixCommand(["flake", "update", ...inputNames, "--flake", path]);
	},

	updateAll: (path: string): Effect.Effect<string, NixCommandError | CommandAbortedError> =>
		runNixCommand(["flake", "update", "--flake", path]),

	lockInputToRev: (
		path: string,
		inputName: string,
		rev: string,
		owner: string,
		repo: string,
	): Effect.Effect<string, NixCommandError | CommandAbortedError> => {
		const overrideUrl = `github:${owner}/${repo}/${rev}`;
		return runNixCommand(["flake", "update", inputName, "--override-input", inputName, overrideUrl, "--flake", path]);
	},
};

export type FlakeService = typeof flakeService;
