import { runCli } from "./cli";

runCli(async (flakePath) => {
	// Lazy-load TUI dependencies only when command runs (not for --help/--version)
	const [{ render }, { App }, { shutdown }] = await Promise.all([
		import("@opentui/solid"),
		import("./App"),
		import("./shutdown"),
	]);
	await import("opentui-spinner/solid");

	process.once("SIGINT", () => void shutdown(0));
	process.once("SIGTERM", () => void shutdown(0));
	process.once("SIGHUP", () => void shutdown(0));

	render(() => <App flakePath={flakePath} />);
});
