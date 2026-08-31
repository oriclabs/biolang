import {
  BioLang,
  lambda,
  mean,
  program,
  read_csv,
  ref,
  return_,
  type BioExpression,
  type RunResult,
} from "../index.js";

const analysis: BioExpression = read_csv("nhanes.csv")
  .filter(lambda("row", (row) => row.Age.gte(18)))
  .column("BMI")
  .mean();

async function execute(): Promise<RunResult> {
  const session = await BioLang.create({ cwd: ".", network: false });
  const result = session.run(analysis);
  const objectResult = session.csv("nhanes.csv").where({ Age: { gte: 18 } }).column("BMI").mean();
  session.run(objectResult);
  session.run(session.table([{ Age: 20, BMI: 22 }]).column("BMI").mean());
  session.inspectVariable("nh", { offset: 0, limit: 20 });
  await session.connectSomer({ baseUrl: "https://example.org", token: "token" });
  return result;
}

const source = program(return_(mean(ref("values"))));
void source;
void execute;
