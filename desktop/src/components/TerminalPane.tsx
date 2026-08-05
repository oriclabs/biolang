import { useEffect, useRef, useState } from "react";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { bridge, isDesktop, onTerminalOutput } from "../bridge";

export function TerminalPane() {
  const containerRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal>();
  const fitRef = useRef<FitAddon>();
  const sessionRef = useRef<number>();
  const [state, setState] = useState<"starting" | "ready" | "error">("starting");

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: "bar",
      convertEol: true,
      fontFamily: '"Cascadia Code", "SFMono-Regular", Consolas, monospace',
      fontSize: 12,
      lineHeight: 1.25,
      scrollback: 5_000,
      theme: {
        background: "#111418",
        foreground: "#d4d9e1",
        cursor: "#65c7b4",
        selectionBackground: "#31534f",
        black: "#15181d",
        red: "#ef7b7b",
        green: "#87c991",
        yellow: "#e0b86e",
        blue: "#76a7d8",
        magenta: "#bd91d8",
        cyan: "#65c7b4",
        white: "#d4d9e1",
        brightBlack: "#69727f",
      },
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(container);
    terminalRef.current = terminal;
    fitRef.current = fit;

    let disposed = false;
    let fitFrame = 0;
    let lastWidth = -1;
    let lastHeight = -1;
    let unlisten: () => void = () => undefined;
    const pendingOutput: Array<{ sessionId: number; data: string }> = [];
    const writeOutput = (data: string) => {
      terminal.write(data);
      if (import.meta.env.DEV) {
        const tail = `${container.dataset.outputTail ?? ""}${data}`.slice(-2_000);
        container.dataset.outputTail = tail;
      }
    };
    void onTerminalOutput((event) => {
      if (sessionRef.current == null) {
        pendingOutput.push(event);
      } else if (event.sessionId === sessionRef.current) {
        writeOutput(event.data);
      }
    }).then(async (dispose) => {
      if (disposed) {
        dispose();
        return;
      }
      unlisten = dispose;
      try {
        const sessionId = await bridge.startTerminal(terminal.cols, terminal.rows);
        if (disposed) {
          await bridge.closeTerminal(sessionId);
          return;
        }
        sessionRef.current = sessionId;
        for (const event of pendingOutput) {
          if (event.sessionId === sessionId) writeOutput(event.data);
        }
        setState("ready");
        if (!isDesktop) {
          terminal.writeln("\x1b[38;2;101;199;180mBioLang Workbench Web\x1b[0m");
          terminal.writeln("A native PTY is not available in the browser.");
          terminal.writeln("Use the BioLang Console for local WASM expressions or select a SOMER runtime.\r\n");
        }
      } catch (error) {
        terminal.writeln(`\x1b[31m${String(error)}\x1b[0m`);
        setState("error");
      }
    });

    const input = terminal.onData((data) => {
      const sessionId = sessionRef.current;
      if (sessionId && isDesktop) void bridge.writeTerminal(sessionId, data);
    });
    const resize = terminal.onResize(({ cols, rows }) => {
      const sessionId = sessionRef.current;
      if (sessionId && isDesktop) void bridge.resizeTerminal(sessionId, cols, rows);
    });
    const scheduleFit = () => {
      const bounds = container.getBoundingClientRect();
      const width = Math.round(bounds.width);
      const height = Math.round(bounds.height);
      if (width < 20 || height < 20 || (width === lastWidth && height === lastHeight)) return;
      lastWidth = width;
      lastHeight = height;
      window.cancelAnimationFrame(fitFrame);
      fitFrame = window.requestAnimationFrame(() => {
        if (disposed || !container.isConnected) return;
        const dimensions = fit.proposeDimensions();
        if (
          dimensions
          && (dimensions.cols !== terminal.cols || dimensions.rows !== terminal.rows)
        ) {
          fit.fit();
        }
      });
    };
    const observer = new ResizeObserver(scheduleFit);
    observer.observe(container);
    scheduleFit();

    return () => {
      disposed = true;
      window.cancelAnimationFrame(fitFrame);
      observer.disconnect();
      input.dispose();
      resize.dispose();
      unlisten();
      if (sessionRef.current) void bridge.closeTerminal(sessionRef.current).catch(() => undefined);
      terminal.dispose();
    };
  }, []);

  return (
    <div className="terminal-wrap" data-state={state} data-session={sessionRef.current ?? ""}>
      {state === "starting" && <span className="terminal-state">Starting shell...</span>}
      <div ref={containerRef} className="terminal-host" aria-label="Integrated terminal" />
    </div>
  );
}
