import { CircleStop, Copy, Plus, Play, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import metadata from "../generated/builtin-metadata.json";
import {
  emptyWorkflow,
  parseWorkflow,
  serializeWorkflow,
  validateWorkflow,
  workflowToBioLang,
  type WorkflowDocument,
  type WorkflowNode,
} from "../workflows";

function safeWorkflow(content: string): { workflow: WorkflowDocument; error?: string } {
  try {
    return { workflow: parseWorkflow(content) };
  } catch (error) {
    return { workflow: emptyWorkflow(), error: String(error) };
  }
}

function parametersFor(operation: string) {
  const builtin = metadata.builtins.find((candidate) => candidate.name === operation);
  return (builtin?.parameters ?? []).map((name) => ({ name, value: "" }));
}

export function WorkflowPane({
  content,
  running,
  onChange,
  onRun,
  onStop,
}: {
  content: string;
  running: boolean;
  onChange: (content: string) => void;
  onRun: () => void | Promise<void>;
  onStop: () => void | Promise<void>;
}) {
  const parsed = useMemo(() => safeWorkflow(content), [content]);
  const [operation, setOperation] = useState("read_fasta");
  const workflow = parsed.workflow;
  const validationErrors = useMemo(() => validateWorkflow(workflow), [workflow]);
  const update = (next: WorkflowDocument) => onChange(serializeWorkflow(next));

  const addNode = () => {
    const count = workflow.nodes.length;
    const id = `step_${count + 1}`;
    const previous = workflow.nodes.at(-1);
    const parameters = parametersFor(operation);
    update({
      ...workflow,
      nodes: [...workflow.nodes, {
        id,
        operation,
        arguments: parameters.map((parameter) => parameter.value),
        parameters,
        strategy: "standard",
        x: 70 + count * 210,
        y: 90,
      }],
      edges: previous ? [...workflow.edges, { from: previous.id, to: id }] : workflow.edges,
    });
  };

  const updateNode = (id: string, patch: Partial<WorkflowNode>) => update({
    ...workflow,
    nodes: workflow.nodes.map((node) => node.id === id ? { ...node, ...patch } : node),
  });

  const updateOperation = (node: WorkflowNode, nextOperation: string) => {
    const parameters = parametersFor(nextOperation);
    updateNode(node.id, {
      operation: nextOperation,
      parameters,
      arguments: parameters.map((parameter) => parameter.value),
    });
  };

  const updateParameter = (node: WorkflowNode, index: number, value: string) => {
    const parameters = [...(node.parameters ?? [])];
    parameters[index] = { ...parameters[index], value };
    updateNode(node.id, { parameters, arguments: parameters.map((parameter) => parameter.value) });
  };

  const addParameter = (node: WorkflowNode) => {
    const parameters = [...(node.parameters ?? []), {
      name: `arg${(node.parameters?.length ?? 0) + 1}`,
      value: "",
    }];
    updateNode(node.id, { parameters, arguments: parameters.map((parameter) => parameter.value) });
  };

  const removeParameter = (node: WorkflowNode, index: number) => {
    const parameters = (node.parameters ?? []).filter((_, candidate) => candidate !== index);
    updateNode(node.id, { parameters, arguments: parameters.map((parameter) => parameter.value) });
  };

  const toggleInput = (nodeId: string, sourceId: string, checked: boolean) => {
    const without = workflow.edges.filter((edge) => !(edge.to === nodeId && edge.from === sourceId));
    update({
      ...workflow,
      edges: checked ? [...without, { from: sourceId, to: nodeId }] : without,
    });
  };

  const removeNode = (id: string) => update({
    ...workflow,
    nodes: workflow.nodes.filter((node) => node.id !== id),
    edges: workflow.edges.filter((edge) => edge.from !== id && edge.to !== id),
  });

  return <div className="workflow-pane">
    <header className="workflow-toolbar">
      <input aria-label="Workflow name" value={workflow.name} onChange={(event) => update({ ...workflow, name: event.target.value })} />
      <input list="biolang-operations" aria-label="Operation" value={operation} onChange={(event) => setOperation(event.target.value)} />
      <datalist id="biolang-operations">{metadata.builtins.map((builtin) => <option key={builtin.name} value={builtin.name}>{builtin.signature}</option>)}</datalist>
      <button type="button" onClick={addNode}><Plus size={13} />Add step</button>
      <button
        type="button"
        title="Copy generated BioLang"
        aria-label="Copy generated BioLang"
        disabled={Boolean(validationErrors.length)}
        onClick={() => void navigator.clipboard.writeText(workflowToBioLang(workflow))}
      ><Copy size={13} /></button>
      {running
        ? <button type="button" className="danger" onClick={() => void onStop()}><CircleStop size={13} />Stop</button>
        : <button type="button" className="primary" disabled={!workflow.nodes.length || Boolean(validationErrors.length)} onClick={() => void onRun()}><Play size={13} />Run</button>}
    </header>
    {parsed.error
      ? <div className="workflow-invalid"><strong>Cannot open workflow</strong><span>{parsed.error}</span></div>
      : <div className="workflow-canvas">
          {!!validationErrors.length && <div className="workflow-errors" role="alert">{validationErrors.join(" | ")}</div>}
          <svg className="workflow-edges" aria-hidden="true">
            {workflow.edges.map((edge) => {
              const from = workflow.nodes.find((node) => node.id === edge.from);
              const to = workflow.nodes.find((node) => node.id === edge.to);
              return from && to ? <line key={`${edge.from}-${edge.to}`} x1={from.x + 190} y1={from.y + 39} x2={to.x} y2={to.y + 39} /> : null;
            })}
          </svg>
          {workflow.nodes.map((node) => {
            const parameters = node.parameters ?? [];
            const incoming = new Set(workflow.edges.filter((edge) => edge.to === node.id).map((edge) => edge.from));
            return <section className="workflow-node" key={node.id} style={{ left: node.x, top: node.y }}>
              <header><span>{node.id}</span><button type="button" title="Delete step" aria-label={`Delete ${node.id}`} onClick={() => removeNode(node.id)}><Trash2 size={12} /></button></header>
              <input aria-label={`${node.id} operation`} list="biolang-operations" value={node.operation} onChange={(event) => updateOperation(node, event.target.value)} />
              <label className="workflow-field">
                <span>Mode</span>
                <select aria-label={`${node.id} execution mode`} value={node.strategy ?? "standard"} onChange={(event) => updateNode(node.id, { strategy: event.target.value as WorkflowNode["strategy"] })}>
                  <option value="standard">Standard</option>
                  <option value="scatter">Scatter</option>
                  <option value="gather">Gather</option>
                </select>
              </label>
              {!!workflow.nodes.filter((candidate) => candidate.id !== node.id).length && <details className="workflow-inputs">
                <summary>Inputs ({incoming.size})</summary>
                {workflow.nodes.filter((candidate) => candidate.id !== node.id).map((candidate) =>
                  <label key={candidate.id}><input type="checkbox" checked={incoming.has(candidate.id)} onChange={(event) => toggleInput(node.id, candidate.id, event.target.checked)} />{candidate.id}</label>)}
              </details>}
              <div className="workflow-parameters">
                {parameters.map((parameter, index) => <label key={`${parameter.name}-${index}`}>
                  <span title={parameter.name}>{parameter.name}</span>
                  <input aria-label={`${node.id} ${parameter.name}`} value={parameter.value} placeholder="BioLang value" onChange={(event) => updateParameter(node, index, event.target.value)} />
                  <button type="button" title={`Remove ${parameter.name}`} aria-label={`Remove ${parameter.name}`} onClick={() => removeParameter(node, index)}><Trash2 size={10} /></button>
                </label>)}
                <button type="button" className="workflow-add-parameter" onClick={() => addParameter(node)}><Plus size={10} />Parameter</button>
              </div>
            </section>;
          })}
        </div>}
  </div>;
}
