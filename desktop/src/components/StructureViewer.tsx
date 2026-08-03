import { Pause, Play, RotateCcw } from "lucide-react";
import { Component, Stage, StructureComponent } from "ngl";
import { useEffect, useRef, useState } from "react";

type Representation = "cartoon" | "ball+stick" | "surface";

/// A cartoon is a backbone trace, so it draws nothing at all for a ligand, an
/// ion, or a short peptide fragment. Below this many residues in a single
/// chain the viewer would render an empty scene.
const MIN_CARTOON_RESIDUES = 4;

function longestPolymerRun(component: Component): number {
  const structure = (component as StructureComponent).structure;
  if (!structure) return 0;
  let longest = 0;
  try {
    structure.eachPolymer((polymer) => {
      longest = Math.max(longest, polymer.residueCount);
    });
  } catch {
    // A structure NGL parsed but cannot walk still renders as ball and stick.
  }
  return longest;
}

function applyRepresentation(component: Component, representation: Representation) {
  component.removeAllRepresentations();
  if (representation === "cartoon") {
    component.addRepresentation("cartoon", { colorScheme: "chainname", quality: "high" });
    component.addRepresentation("licorice", { sele: "hetero", colorScheme: "element" });
  } else if (representation === "surface") {
    component.addRepresentation("surface", {
      colorScheme: "hydrophobicity",
      opacity: 0.72,
      surfaceType: "av",
    });
  } else {
    component.addRepresentation("ball+stick", {
      colorScheme: "element",
      multipleBond: true,
      quality: "high",
    });
  }
}

/// Reads the workbench theme rather than hardcoding a dark canvas, so the
/// structure does not sit in a black box on a light background.
function stageBackground(host: HTMLElement): string {
  const value = getComputedStyle(host).getPropertyValue("--bg-deep").trim();
  return value || "#101519";
}

export function StructureViewer({
  source,
  format,
}: {
  source: string;
  format: string;
}) {
  const hostRef = useRef<HTMLDivElement>(null);
  const stageRef = useRef<Stage>();
  const componentRef = useRef<Component>();
  const [representation, setRepresentation] = useState<Representation>("cartoon");
  const [spinning, setSpinning] = useState(false);
  const [status, setStatus] = useState("Loading structure...");

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    const stage = new Stage(host, {
      backgroundColor: stageBackground(host),
      quality: "medium",
      tooltip: true,
      mousePreset: "default",
    });
    stageRef.current = stage;
    const observer = new ResizeObserver(() => stage.handleResize());
    observer.observe(host);
    let disposed = false;
    const extension = ["cif", "mmcif"].includes(format.toLowerCase()) ? "cif" : "pdb";
    void stage.loadFile(new Blob([source], { type: "text/plain" }), { ext: extension })
      .then((component) => {
        if (disposed || !component) return;
        componentRef.current = component;
        // Choose a representation that will actually draw. Defaulting to
        // cartoon left small structures as an empty canvas with no
        // explanation of why.
        const residues = longestPolymerRun(component);
        const drawable: Representation = residues >= MIN_CARTOON_RESIDUES
          ? "cartoon"
          : "ball+stick";
        setRepresentation(drawable);
        applyRepresentation(component, drawable);
        component.autoView();
        // Silent on load: a molecule the viewer picked and drew needs no
        // apology. The warning below is for a cartoon the user asks for.
        setStatus("");
      })
      .catch((error) => {
        if (!disposed) setStatus(`Cannot render structure: ${String(error)}`);
      });
    return () => {
      disposed = true;
      observer.disconnect();
      stage.dispose();
      host.replaceChildren();
      stageRef.current = undefined;
      componentRef.current = undefined;
    };
  }, [format, source]);

  // The stage is built once per file, so a theme switch after that would leave
  // the canvas on the old background until the tab was reopened.
  useEffect(() => {
    const host = hostRef.current;
    const shell = host?.closest<HTMLElement>(".app-shell");
    if (!host || !shell) return;
    const observer = new MutationObserver(() => {
      stageRef.current?.setParameters({ backgroundColor: stageBackground(host) });
    });
    observer.observe(shell, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    const component = componentRef.current;
    if (!component) return;
    applyRepresentation(component, representation);
    // An explicit choice is honoured rather than overridden, but a cartoon
    // that cannot draw says so instead of clearing the canvas silently.
    setStatus(representation === "cartoon"
      && longestPolymerRun(component) < MIN_CARTOON_RESIDUES
      ? "No protein backbone to trace — try ball and stick."
      : "");
  }, [representation]);

  return (
    <section className="structure-viewer" aria-label="Interactive molecular structure">
      <div className="structure-viewer-tools">
        <label>
          <span>Representation</span>
          <select
            aria-label="Structure representation"
            value={representation}
            onChange={(event) => setRepresentation(event.target.value as Representation)}
          >
            <option value="cartoon">Cartoon</option>
            <option value="ball+stick">Ball and stick</option>
            <option value="surface">Molecular surface</option>
          </select>
        </label>
        <button
          type="button"
          title="Reset structure view"
          aria-label="Reset structure view"
          onClick={() => componentRef.current?.autoView(500)}
        >
          <RotateCcw size={13} />
        </button>
        <button
          type="button"
          title={spinning ? "Stop structure rotation" : "Rotate structure"}
          aria-label={spinning ? "Stop structure rotation" : "Rotate structure"}
          onClick={() => {
            const next = !spinning;
            stageRef.current?.setSpin(next);
            setSpinning(next);
          }}
        >
          {spinning ? <Pause size={13} /> : <Play size={13} />}
        </button>
      </div>
      <div className="structure-stage" ref={hostRef} />
      {status && <div className="structure-status">{status}</div>}
    </section>
  );
}
