import { render } from "@opentui/solid";
import "opentui-spinner/solid";
import { App } from "./components/App";
import { AppProvider } from "./context/AppContext";
import { ChangelogProvider } from "./context/ChangelogContext";
import { getFlakeMetadata, hasFlakeNix, resolveFlakePath } from "./lib/flake";
import type { FlakeMetadata } from "./lib/types";

function Root(props: { flake: FlakeMetadata }) {
	return (
		<AppProvider flake={props.flake}>
			<ChangelogProvider>
				<App />
			</ChangelogProvider>
		</AppProvider>
	);
}

async function main() {
	const flakePath = resolveFlakePath(process.argv[2] || process.cwd());

	const hasFlake = await hasFlakeNix(flakePath);
	if (!hasFlake) {
		console.error(`No flake.nix found in ${flakePath}`);
		process.exit(1);
	}

	let flake: FlakeMetadata;
	try {
		flake = await getFlakeMetadata(flakePath);
	} catch (err) {
		const msg = err instanceof Error ? err.message : String(err);
		console.error(`Failed to load flake metadata: ${msg}`);
		process.exit(1);
	}

	render(() => <Root flake={flake} />);
}

main();
