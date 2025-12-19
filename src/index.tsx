import { render } from "@opentui/solid";
import "opentui-spinner/solid";
import { App } from "./App";
import { parseArgs } from "./cli";
import { flakeService } from "./services/flake";

const args = await parseArgs();
const result = await flakeService.load(args.flake);

if (!result.ok) {
	console.error(result.error);
	process.exit(1);
}

render(() => <App initialFlake={result.data} />);
