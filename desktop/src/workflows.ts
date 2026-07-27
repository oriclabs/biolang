export interface PipelineStep {
  id: string;
  operation: string;
  line: number;
  expression: string;
}

export interface WorkflowParameter {
  name: string;
  value: string;
}

export interface WorkflowNode {
  id: string;
  operation: string;
  arguments: string[];
  parameters?: WorkflowParameter[];
  strategy?: "standard" | "scatter" | "gather";
  x: number;
  y: number;
}

export interface WorkflowEdge {
  from: string;
  to: string;
}

export interface WorkflowDocument {
  schemaVersion: 1;
  name: string;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
}

const ignoredCalls = new Set(["if", "for", "while", "fn"]);

export function inspectPipeline(source: string): PipelineStep[] {
  const steps: PipelineStep[] = [];
  for (const [lineIndex, line] of source.split(/\r?\n/).entries()) {
    for (const match of line.matchAll(/\b([a-z_][a-z0-9_]*)\s*\(/gi)) {
      const operation = match[1];
      if (ignoredCalls.has(operation)) continue;
      steps.push({
        id: `step-${steps.length + 1}`,
        operation,
        line: lineIndex + 1,
        expression: line.trim(),
      });
    }
  }
  return steps;
}

function identifier(value: string) {
  const normalized = value.replace(/\W/g, "_");
  return /^\d/.test(normalized) ? `step_${normalized}` : normalized;
}

function normalizeNode(node: WorkflowNode): WorkflowNode {
  const argumentsList = Array.isArray(node.arguments) ? node.arguments : [];
  const parameters = Array.isArray(node.parameters)
    ? node.parameters.filter((parameter) =>
        parameter && typeof parameter.name === "string" && typeof parameter.value === "string")
    : argumentsList.map((value, index) => ({ name: `arg${index + 1}`, value }));
  return {
    ...node,
    arguments: parameters.map((parameter) => parameter.value),
    parameters,
    strategy: node.strategy ?? "standard",
  };
}

export function validateWorkflow(workflow: WorkflowDocument): string[] {
  const errors: string[] = [];
  const ids = new Set<string>();
  const generatedIds = new Map<string, string>();
  for (const node of workflow.nodes) {
    if (!node.id.trim()) errors.push("Every node requires an id");
    else if (ids.has(node.id)) errors.push(`Duplicate node id '${node.id}'`);
    ids.add(node.id);
    const generatedId = identifier(node.id);
    const previousId = generatedIds.get(generatedId);
    if (previousId && previousId !== node.id) {
      errors.push(`Node ids '${previousId}' and '${node.id}' generate the same BioLang variable`);
    } else {
      generatedIds.set(generatedId, node.id);
    }
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(node.operation)) {
      errors.push(`Invalid operation '${node.operation}' in ${node.id || "unnamed node"}`);
    }
  }
  const edges = new Set<string>();
  for (const edge of workflow.edges) {
    if (!ids.has(edge.from) || !ids.has(edge.to)) {
      errors.push(`Edge '${edge.from}' -> '${edge.to}' references a missing node`);
    }
    if (edge.from === edge.to) errors.push(`Node '${edge.from}' cannot connect to itself`);
    const key = `${edge.from}\0${edge.to}`;
    if (edges.has(key)) errors.push(`Duplicate edge '${edge.from}' -> '${edge.to}'`);
    edges.add(key);
  }
  try {
    topologicalWorkflowNodes(workflow);
  } catch (error) {
    errors.push(String(error instanceof Error ? error.message : error));
  }
  return [...new Set(errors)];
}

export function topologicalWorkflowNodes(workflow: WorkflowDocument): WorkflowNode[] {
  const byId = new Map(workflow.nodes.map((node) => [node.id, node]));
  const indegree = new Map(workflow.nodes.map((node) => [node.id, 0]));
  const outgoing = new Map(workflow.nodes.map((node) => [node.id, [] as string[]]));
  for (const edge of workflow.edges) {
    if (!byId.has(edge.from) || !byId.has(edge.to) || edge.from === edge.to) continue;
    indegree.set(edge.to, (indegree.get(edge.to) ?? 0) + 1);
    outgoing.get(edge.from)?.push(edge.to);
  }
  const queue = workflow.nodes.filter((node) => indegree.get(node.id) === 0);
  const sorted: WorkflowNode[] = [];
  while (queue.length) {
    const node = queue.shift()!;
    sorted.push(node);
    for (const next of outgoing.get(node.id) ?? []) {
      const remaining = (indegree.get(next) ?? 0) - 1;
      indegree.set(next, remaining);
      if (remaining === 0) queue.push(byId.get(next)!);
    }
  }
  if (sorted.length !== workflow.nodes.length) {
    throw new Error("Workflow contains a cycle");
  }
  return sorted;
}

export function parseWorkflow(source: string): WorkflowDocument {
  const value = JSON.parse(source) as Partial<WorkflowDocument>;
  if (value.schemaVersion !== 1 || typeof value.name !== "string"
    || !Array.isArray(value.nodes) || !Array.isArray(value.edges)) {
    throw new Error("Invalid .blflow document");
  }
  const workflow: WorkflowDocument = {
    schemaVersion: 1,
    name: value.name,
    nodes: value.nodes.map((node) => normalizeNode(node)),
    edges: value.edges,
  };
  return workflow;
}

export function serializeWorkflow(workflow: WorkflowDocument) {
  const normalized = {
    ...workflow,
    nodes: workflow.nodes.map((node) => {
      const parameters = node.parameters ?? node.arguments.map((value, index) => ({
        name: `arg${index + 1}`,
        value,
      }));
      return {
        ...node,
        arguments: parameters.map((parameter) => parameter.value),
        parameters,
        strategy: node.strategy ?? "standard",
      };
    }),
  };
  return `${JSON.stringify(normalized, null, 2)}\n`;
}

export function emptyWorkflow(name = "BioLang workflow"): WorkflowDocument {
  return { schemaVersion: 1, name, nodes: [], edges: [] };
}

export function workflowToBioLang(workflow: WorkflowDocument) {
  const errors = validateWorkflow(workflow);
  if (errors.length) throw new Error(errors.join("; "));
  const lines = ["# Generated from a BioLang .blflow workflow"];
  const sorted = topologicalWorkflowNodes(workflow);
  for (const node of sorted) {
    const incoming = workflow.edges
      .filter((edge) => edge.to === node.id)
      .map((edge) => identifier(edge.from));
    const parameters = (node.parameters ?? node.arguments.map((value, index) => ({
      name: `arg${index + 1}`,
      value,
    }))).map((parameter) => parameter.value).filter((value) => value.trim());
    const operation = node.operation;
    let expression: string;
    if (node.strategy === "scatter") {
      if (incoming.length !== 1) {
        throw new Error(`Scatter node '${node.id}' requires exactly one input`);
      }
      const args = ["item", ...parameters].join(", ");
      expression = `${incoming[0]} |> map(|item| ${operation}(${args}))`;
    } else if (node.strategy === "gather") {
      if (!incoming.length) throw new Error(`Gather node '${node.id}' requires at least one input`);
      expression = `[${incoming.join(", ")}] |> ${operation}(${parameters.join(", ")})`;
    } else if (incoming.length === 1) {
      expression = `${incoming[0]} |> ${operation}(${parameters.join(", ")})`;
    } else {
      expression = `${operation}(${[...incoming, ...parameters].join(", ")})`;
    }
    lines.push(`let ${identifier(node.id)} = ${expression}`);
  }
  const sources = new Set(workflow.edges.map((edge) => edge.from));
  const sinks = sorted.filter((node) => !sources.has(node.id));
  for (const sink of sinks) lines.push(`println(${identifier(sink.id)})`);
  return `${lines.join("\n")}\n`;
}
