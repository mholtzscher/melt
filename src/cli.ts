import { Args, Command } from "@effect/cli";
import { BunContext, BunRuntime } from "@effect/platform-bun";
import { Deferred, Effect, Option } from "effect";

export interface CliArgs {
	flake: Option.Option<string>;
}

const flakeArg = Args.optional(
	Args.directory({
		name: "flake",
	}).pipe(Args.withDescription("Path to flake directory (defaults to current directory)")),
);

// Deferred that signals when the TUI should exit
let shutdownDeferred: Deferred.Deferred<void, never> | undefined;

/**
 * Signal the CLI program that the TUI has completed.
 * Call this from your shutdown handler.
 */
export function signalShutdown(): void {
	if (shutdownDeferred) {
		Effect.runSync(Deferred.succeed(shutdownDeferred, undefined));
	}
}

/**
 * Run the CLI program. The handler receives parsed args and a shutdown signal.
 * The program stays alive until signalShutdown() is called.
 */
export function runCli(handler: (flakePath: string | undefined) => void): void {
	const program = Effect.gen(function* () {
		const deferred = yield* Deferred.make<void, never>();
		shutdownDeferred = deferred;

		const command = Command.make("melt", { flake: flakeArg }, (args) =>
			Effect.gen(function* () {
				const flakePath = Option.getOrUndefined(args.flake);
				handler(flakePath);

				// Keep the Effect alive until shutdown is signaled
				yield* Deferred.await(deferred);
			}),
		);

		const cli = Command.run(command, {
			name: "melt",
			version: "0.1.0",
		});

		yield* cli(process.argv);
	}).pipe(Effect.provide(BunContext.layer));

	BunRuntime.runMain(program);
}
