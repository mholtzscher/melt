import type { FlakeInput } from "../../types";
// Use git CLI provider for everything - no API calls
import { gitProvider } from "./providers/git";
import type { ChangelogResult, RepoInfo, UpdateStatus, VCSService } from "./types";
import { parseRepoInfo } from "./utils";

// Re-export types
export type { ChangelogResult, Commit, UpdateStatus } from "./types";

/**
 * Check if an input is supported for VCS operations
 * Returns the parsed repo info if supported, null otherwise
 */
function getSupportedInput(input: FlakeInput): RepoInfo | null {
	// Skip path inputs - they're local and don't have remote VCS
	if (input.type === "path") {
		return null;
	}

	return parseRepoInfo(input);
}

/**
 * Unified VCS service using git CLI for all operations
 * No API calls - uses bare clone caching for efficiency
 */
export const vcsService: VCSService = {
	async checkForUpdates(
		inputs: FlakeInput[],
		onStatusChange?: (name: string, status: UpdateStatus) => void,
	): Promise<Map<string, UpdateStatus>> {
		const results = new Map<string, UpdateStatus>();

		// Filter to inputs we can check
		const checkable: Array<{ input: FlakeInput; repoInfo: RepoInfo }> = [];

		for (const input of inputs) {
			const repoInfo = getSupportedInput(input);
			if (repoInfo) {
				checkable.push({ input, repoInfo });

				// Set loading state
				const loadingStatus: UpdateStatus = {
					commitsBehind: 0,
					loading: true,
				};
				results.set(input.name, loadingStatus);
				onStatusChange?.(input.name, loadingStatus);
			}
		}

		// Process in parallel batches of 10
		const BATCH_SIZE = 10;

		for (let i = 0; i < checkable.length; i += BATCH_SIZE) {
			const batch = checkable.slice(i, i + BATCH_SIZE);

			await Promise.all(
				batch.map(async ({ input, repoInfo }) => {
					try {
						const commits = await gitProvider.getCommitsSinceRev(input, repoInfo);
						const status: UpdateStatus = {
							commitsBehind: commits.length,
							loading: false,
						};
						results.set(input.name, status);
						onStatusChange?.(input.name, status);
					} catch (error) {
						const status: UpdateStatus = {
							commitsBehind: 0,
							loading: false,
							error: error instanceof Error ? error.message : String(error),
						};
						results.set(input.name, status);
						onStatusChange?.(input.name, status);
					}
				}),
			);
		}

		return results;
	},

	async getChangelog(input: FlakeInput): Promise<ChangelogResult> {
		const repoInfo = getSupportedInput(input);
		if (!repoInfo) {
			throw new Error(`Changelog not available for ${input.type} inputs`);
		}

		return gitProvider.getChangelog(input, repoInfo);
	},

	supportsChangelog(input: FlakeInput): boolean {
		return getSupportedInput(input) !== null;
	},

	getLockUrl(input: FlakeInput, rev: string): string | null {
		const repoInfo = getSupportedInput(input);
		if (!repoInfo) {
			return null;
		}

		return gitProvider.getLockUrl(input, repoInfo, rev);
	},
};
