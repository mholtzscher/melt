import { dirname, resolve } from "node:path";
import { $ } from "bun";
import type {
	FlakeInput,
	FlakeInputType,
	NixFlakeMetadataResponse,
	Result,
} from "../types";

export interface FlakeService {
	load(pathArg?: string): Promise<Result<FlakeData>>;
	refresh(path: string): Promise<Result<FlakeData>>;
	updateInputs(path: string, inputNames: string[]): Promise<Result<string>>;
	updateAll(path: string): Promise<Result<string>>;
	lockInputToRev(
		path: string,
		inputName: string,
		rev: string,
		owner: string,
		repo: string,
	): Promise<Result<string>>;
}

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

function hasFlakeNix(path: string = "."): Promise<boolean> {
	return Bun.file(`${path}/flake.nix`).exists();
}

function getInputType(
	locked?: { type: string },
	original?: { type: string },
): FlakeInputType {
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

async function fetchMetadata(
	path: string,
): Promise<Result<NixFlakeMetadataResponse>> {
	try {
		const result =
			await $`nix flake metadata --json ${path} 2>/dev/null`.text();
		return { ok: true, data: JSON.parse(result) };
	} catch (err) {
		const msg = err instanceof Error ? err.message : String(err);
		return { ok: false, error: msg };
	}
}

export const flakeService: FlakeService = {
	async load(pathArg?: string): Promise<Result<FlakeData>> {
		const flakePath = resolveFlakePath(pathArg || process.cwd());

		const hasFlake = await hasFlakeNix(flakePath);
		if (!hasFlake) {
			return { ok: false, error: `No flake.nix found in ${flakePath}` };
		}

		const result = await fetchMetadata(flakePath);
		if (!result.ok) {
			return {
				ok: false,
				error: `Failed to load flake metadata: ${result.error}`,
			};
		}

		return {
			ok: true,
			data: {
				path: flakePath,
				description: result.data.description,
				inputs: parseInputs(result.data),
			},
		};
	},

	async refresh(path: string): Promise<Result<FlakeData>> {
		const result = await fetchMetadata(path);
		if (!result.ok) {
			return {
				ok: false,
				error: `Failed to refresh flake metadata: ${result.error}`,
			};
		}

		return {
			ok: true,
			data: {
				path,
				description: result.data.description,
				inputs: parseInputs(result.data),
			},
		};
	},

	async updateInputs(
		path: string,
		inputNames: string[],
	): Promise<Result<string>> {
		if (inputNames.length === 0) {
			return { ok: true, data: "No inputs to update" };
		}

		try {
			const result =
				await $`nix flake update ${inputNames} --flake ${path} 2>&1`.text();
			return { ok: true, data: result };
		} catch (error) {
			return {
				ok: false,
				error: error instanceof Error ? error.message : String(error),
			};
		}
	},

	async updateAll(path: string): Promise<Result<string>> {
		try {
			const result = await $`nix flake update --flake ${path} 2>&1`.text();
			return { ok: true, data: result };
		} catch (error) {
			return {
				ok: false,
				error: error instanceof Error ? error.message : String(error),
			};
		}
	},

	async lockInputToRev(
		path: string,
		inputName: string,
		rev: string,
		owner: string,
		repo: string,
	): Promise<Result<string>> {
		try {
			const overrideUrl = `github:${owner}/${repo}/${rev}`;
			const result =
				await $`nix flake update ${inputName} --override-input ${inputName} ${overrideUrl} --flake ${path} 2>&1`.text();
			return { ok: true, data: result };
		} catch (error) {
			return {
				ok: false,
				error: error instanceof Error ? error.message : String(error),
			};
		}
	},
};
