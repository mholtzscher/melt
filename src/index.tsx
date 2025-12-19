import { render } from "@opentui/solid";
import "opentui-spinner/solid";
import { App } from "./components/App";
import { AppProvider } from "./context/AppContext";
import { ChangelogProvider } from "./context/ChangelogContext";
import { parseArgs } from "./lib/cli";
import { FlakeMetadata } from "./lib/flake";

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
	const args = await parseArgs();
	const result = await FlakeMetadata.load(args.flake);
	if (!result.ok) {
		console.error(result.error);
		process.exit(1);
	}

	render(() => <Root flake={result.data} />);
}

main();
