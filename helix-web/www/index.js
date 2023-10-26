import * as xterm from "xterm";
import { FitAddon } from 'xterm-addon-fit';
import { WebglAddon } from 'xterm-addon-webgl';
import * as wasm from "helix-web";

const terminalOptions = xterm.ITerminalOptions = {
    theme: { background: "#282a36" },
    fontSize: 20,
    scrollback: 0,
  };

const term = new xterm.Terminal(terminalOptions);
const fitAddon = new FitAddon();

term.open(document.getElementById('terminal'));
term.loadAddon(fitAddon);
term.loadAddon(new WebglAddon());
fitAddon.fit();

term.write('Hello from \x1B[1;3;31mxterm.js\x1B[0m $ ')