import { render } from "@opentui/solid";
import "opentui-spinner/solid";
import { App } from "./App";
import { parseArgs } from "./cli";

const args = await parseArgs();
render(() => <App flakePath={args.flake} />);
