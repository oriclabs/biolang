/**
 * The bio API credentials the workbench can store.
 *
 * `bl-apis` reads these from the process environment, which meant the only way
 * to use a key was to export it in the shell you happened to launch from — so
 * clicking an NCBI example in the API browser and running it produced a rate
 * limit error with nothing on screen explaining why. The Desktop build stores
 * them in the OS keyring and injects them into every BioLang process.
 *
 * `services` is what makes the API browser able to say *which* browsable API a
 * missing key affects.
 */
export type CredentialDefinition = {
  name: string;
  label: string;
  detail: string;
  /** Where to get one. */
  docsUrl: string;
  /** Package names in the API browser that use this credential. */
  services: string[];
  /** True when the service refuses to work at all without it. */
  required: boolean;
};

export const credentialCatalog: CredentialDefinition[] = [
  {
    name: "NCBI_API_KEY",
    label: "NCBI",
    detail: "Raises E-utilities rate limits from 3 to 10 requests per second. Used by ClinVar, GEO, and NCBI Datasets.",
    docsUrl: "https://account.ncbi.nlm.nih.gov/settings/",
    services: ["clinvar", "geo", "ncbi", "ncbi_datasets"],
    required: false,
  },
  {
    name: "COSMIC_API_KEY",
    label: "COSMIC",
    detail: "Required. COSMIC refuses every request without credentials.",
    docsUrl: "https://cancer.sanger.ac.uk/cosmic/login",
    services: ["cosmic"],
    required: true,
  },
  {
    name: "ANTHROPIC_API_KEY",
    label: "Anthropic",
    detail: "Enables the Claude-backed BioLang LLM builtins.",
    docsUrl: "https://console.anthropic.com/settings/keys",
    services: ["llm"],
    required: false,
  },
  {
    name: "OPENAI_API_KEY",
    label: "OpenAI",
    detail: "Enables the OpenAI-backed BioLang LLM builtins.",
    docsUrl: "https://platform.openai.com/api-keys",
    services: ["llm"],
    required: false,
  },
  {
    name: "LLM_API_KEY",
    label: "Generic LLM",
    detail: "Fallback key for a custom or self-hosted model endpoint.",
    docsUrl: "https://lang.bio/docs",
    services: ["llm"],
    required: false,
  },
  {
    name: "GITHUB_TOKEN",
    label: "GitHub",
    detail: "Raises GitHub API rate limits when resolving packages and nf-core pipelines.",
    docsUrl: "https://github.com/settings/tokens",
    services: ["nfcore"],
    required: false,
  },
  {
    name: "TELEGRAM_BOT_TOKEN",
    label: "Telegram",
    detail: "Lets long pipelines notify a chat when they finish.",
    docsUrl: "https://core.telegram.org/bots#how-do-i-create-a-bot",
    services: [],
    required: false,
  },
];

/** Whether a value is stored, and whether the environment already supplies it. */
export type CredentialStatus = {
  name: string;
  configured: boolean;
  fromEnvironment: boolean;
};

/** Credentials a given API package uses, for the "key required" badge. */
export function credentialsForService(service: string): CredentialDefinition[] {
  return credentialCatalog.filter((credential) => credential.services.includes(service));
}

/** True when this credential is neither stored nor exported by the environment. */
export function isMissing(
  definition: CredentialDefinition,
  statuses: CredentialStatus[],
): boolean {
  const status = statuses.find((candidate) => candidate.name === definition.name);
  return !status?.configured && !status?.fromEnvironment;
}
