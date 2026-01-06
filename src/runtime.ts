import { Effect, Exit, FiberSet, Scope } from "effect";

// Lazy-initialized runtime to avoid keeping the process alive for --help/--version
let runtime:
	| {
			scope: Scope.CloseableScope;
			fiberSet: FiberSet.FiberSet<unknown, unknown>;
			runPromise: <A, E>(effect: Effect.Effect<A, E>) => Promise<A>;
	  }
	| undefined;
let isClosed = false;

function ensureInitialized() {
	if (!runtime) {
		const scope = Effect.runSync(Scope.make());
		const fiberSet = Effect.runSync(Scope.extend(FiberSet.make(), scope));
		const runPromise = Effect.runSync(FiberSet.runtimePromise(fiberSet)());
		runtime = { scope, fiberSet, runPromise };
	}
	return runtime;
}

/**
 * Check if the runtime has been closed.
 */
export function isRuntimeClosed(): boolean {
	return isClosed;
}

/**
 * Run an Effect and track it for cleanup on shutdown.
 */
export function runEffect<A, E>(effect: Effect.Effect<A, E>): Promise<A> {
	if (isClosed) {
		return Promise.reject(new Error("Runtime is closed"));
	}
	const { runPromise } = ensureInitialized();
	return runPromise(effect);
}

/**
 * Run an Effect with Either for error handling, tracked for cleanup.
 */
export function runEffectEither<A, E>(
	effect: Effect.Effect<A, E>,
): Promise<{ _tag: "Right"; right: A } | { _tag: "Left"; left: E }> {
	return runEffect(Effect.either(effect)) as Promise<{ _tag: "Right"; right: A } | { _tag: "Left"; left: E }>;
}

/**
 * Interrupt all running fibers and close the global scope.
 */
export async function interruptAll(): Promise<void> {
	if (isClosed) return;
	isClosed = true;
	if (runtime) {
		await Effect.runPromise(
			FiberSet.clear(runtime.fiberSet).pipe(Effect.andThen(Scope.close(runtime.scope, Exit.void))),
		);
	}
}
