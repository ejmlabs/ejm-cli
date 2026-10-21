// Bounded observer for the optional saved-session PTY measurements.
// Use the maintained emulator installed with npm ci --prefix scripts.
import { createInterface } from "node:readline";
import headless from "@xterm/headless";
const { Terminal } = headless;
const terminal = new Terminal({
  cols: 100,
  rows: 30,
  scrollback: 0,
  allowProposedApi: true,
});
try {
  for await (const line of createInterface({ input: process.stdin })) {
    if (line.length > 128 * 1024)
      throw new Error("terminal observer frame exceeds its bound");
    await new Promise((resolve) =>
      terminal.write(Buffer.from(line, "base64"), resolve),
    );
    const lines = Array.from(
      { length: 30 },
      (_, row) =>
        terminal.buffer.active.getLine(row)?.translateToString(true) ?? "",
    );
    process.stdout.write(`${JSON.stringify(lines)}\n`);
  }
} finally {
  terminal.dispose();
}
