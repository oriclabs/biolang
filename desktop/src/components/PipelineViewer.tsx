import { ArrowRight, Braces, Code2 } from "lucide-react";
import { inspectPipeline } from "../workflows";

export function PipelineViewer({ source, onOpenSource }: { source: string; onOpenSource: () => void }) {
  const steps = inspectPipeline(source);
  return <div className="pipeline-viewer">
    <header className="pipeline-toolbar">
      <strong>Pipeline</strong>
      <span>{steps.length} operations</span>
      <button type="button" onClick={onOpenSource}><Code2 size={13} />Source</button>
    </header>
    <div className="pipeline-canvas">
      {steps.length
        ? <div className="pipeline-flow">{steps.map((step, index) => <div className="pipeline-stage" key={step.id}>
            {index > 0 && <ArrowRight className="pipeline-arrow" size={18} />}
            <button type="button" title={step.expression}>
              <Braces size={15} />
              <strong>{step.operation}</strong>
              <span>Line {step.line}</span>
            </button>
          </div>)}</div>
        : <div className="pipeline-empty"><Braces size={22} /><strong>No callable pipeline stages found</strong></div>}
    </div>
  </div>;
}
