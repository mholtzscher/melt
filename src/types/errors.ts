import { Data } from "effect";

// Base error class for all Melt errors
export class MeltError extends Data.TaggedError("MeltError")<{
	message: string;
	cause?: unknown;
}> {}

// Flake-related errors
export class FlakeNotFoundError extends Data.TaggedError("FlakeNotFoundError")<{
	path: string;
}> {
	override get message() {
		return `No flake.nix found in ${this.path}`;
	}
}

export class FlakeMetadataError extends Data.TaggedError("FlakeMetadataError")<{
	message: string;
	cause?: unknown;
}> {}

export class NixCommandError extends Data.TaggedError("NixCommandError")<{
	command: string[];
	exitCode: number;
	stderr: string;
}> {
	override get message() {
		return this.stderr.trim() || `Process exited with code ${this.exitCode}`;
	}
}

export class CommandAbortedError extends Data.TaggedError("CommandAbortedError")<{
	command?: string[];
}> {
	override get message() {
		return "Command aborted";
	}
}

// GitHub-related errors
export class GitHubRateLimitError extends Data.TaggedError("GitHubRateLimitError")<{
	resetAt?: Date;
}> {
	override get message() {
		return "GitHub API rate limit exceeded";
	}
}

export class GitHubApiError extends Data.TaggedError("GitHubApiError")<{
	status: number;
	statusText: string;
	url: string;
}> {
	override get message() {
		return `GitHub API error: ${this.status} ${this.statusText}`;
	}
}

export class GitHubInputError extends Data.TaggedError("GitHubInputError")<{
	inputName: string;
	reason: string;
}> {
	override get message() {
		return `Cannot process input "${this.inputName}": ${this.reason}`;
	}
}

// JSON parsing errors
export class JsonParseError extends Data.TaggedError("JsonParseError")<{
	input: string;
	cause?: unknown;
}> {
	override get message() {
		return "Failed to parse JSON response";
	}
}

// Union types for service errors
export type FlakeError =
	| FlakeNotFoundError
	| FlakeMetadataError
	| NixCommandError
	| CommandAbortedError
	| JsonParseError;

export type GitHubError = GitHubRateLimitError | GitHubApiError | GitHubInputError;
