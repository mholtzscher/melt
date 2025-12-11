import { render } from "@opentui/solid";
import "opentui-spinner/solid";
import { App } from "./components/App";
import { AppProvider } from "./context/AppContext";
import { ChangelogProvider } from "./context/ChangelogContext";
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
	const result = await FlakeMetadata.load(process.argv[2]);
	if (!result.ok) {
		console.error(result.error);
		process.exit(1);
	}

	render(() => <Root flake={result.data} />);
}

main();
