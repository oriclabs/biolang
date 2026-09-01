import {
  BioLang,
  type BioSequenceValue,
  type BioValueHandle as BioHandleType,
  BioValueHandle,
  type RunResult,
} from "../index.js";
import {
  lambda,
  mean,
  program,
  read_csv,
  ref,
  return_,
  type BioExpression,
} from "../expressions.js";

const analysis: BioExpression = read_csv("nhanes.csv")
  .filter(lambda("row", (row) => row.Age.gte(18)))
  .column("BMI")
  .mean();

async function execute(): Promise<RunResult> {
  const session = await BioLang.create({ cwd: ".", network: false });
  const result = session.run(analysis);
  const directMean: number = session.mean([1, 2, 3]);
  const directSequence: BioSequenceValue | BioHandleType = session.dna("ATGC");
  const directGc: number = session.gcContent(directSequence);
  const actualMean = session.callValue("mean", [[1, 2, 3]]);
  session.setValue("values", [1, 2, 3]);
  const actualValues = session.getValue("values");
  const possiblyLarge = session.evalValue("values", { maximumInlineBytes: 8 });
  if (possiblyLarge instanceof BioValueHandle) possiblyLarge.page({ limit: 2 });
  session.registerFunction(
    "js_double",
    { parameters: ["Number"], returns: "Number" },
    (value) => Number(value) * 2,
  );
  // @ts-expect-error misspelled builtin names must not bypass generated types
  session.summry([1, 2, 3]);
  const objectResult = session.csvExpression("nhanes.csv").where({ Age: { gte: 18 } }).column("BMI").mean();
  session.run(objectResult);
  session.run(session.tableExpression([{ Age: 20, BMI: 22 }]).column("BMI").mean());
  session.inspectVariable("nh", { offset: 0, limit: 20 });
  await session.connectSomer({ baseUrl: "https://example.org", token: "token" });
  session.dispose();
  void directMean;
  void directSequence;
  void directGc;
  void actualMean;
  void actualValues;
  return result;
}

const source = program(return_(mean(ref("values"))));
void source;
void execute;
