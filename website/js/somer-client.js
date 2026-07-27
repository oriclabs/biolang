// Generated browser ESM build from SOMER clients/browser/somer-client.js.
export class SomerApiError extends Error {
  constructor(status, code, message) {
    super(message);
    this.name = "SomerApiError";
    this.status = status;
    this.code = code;
  }
}

export class SomerClient {
  constructor({ baseUrl, token, fetch: fetcher }) {
    this.baseUrl = baseUrl.replace(/\/+$/, "");
    this.token = token;
    this.fetcher = fetcher || globalThis.fetch.bind(globalThis);
  }

  serviceInfo() { return this.request("/v1/service-info", {}, false); }
  me() { return this.request("/v1/me"); }
  resourceProfiles() { return this.request("/v1/resource-profiles"); }
  async listJobs(filter = {}) {
    const query = new URLSearchParams();
    if (filter.status) query.set("status", filter.status);
    if (filter.tag) query.set("tag", filter.tag);
    if (filter.limit !== undefined) query.set("limit", String(filter.limit));
    return (await this.request(`/v1/jobs${query.size ? `?${query}` : ""}`)).jobs;
  }
  submitJob(request) {
    return this.request("/v1/jobs", { method: "POST", body: JSON.stringify(request) });
  }
  getJob(id) { return this.request(`/v1/jobs/${encodeURIComponent(id)}`); }
  cancelJob(id) {
    return this.request(`/v1/jobs/${encodeURIComponent(id)}/cancel`, { method: "POST" });
  }
  retryJob(id) {
    return this.request(`/v1/jobs/${encodeURIComponent(id)}/retry`, { method: "POST" });
  }
  async uploadInput(id, path, body, { offset = 0, chunkSize, totalSize }) {
    const response = await this.fetcher(
      `${this.baseUrl}/v1/jobs/${encodeURIComponent(id)}/inputs/${encodePath(path)}`,
      {
        method: "PUT",
        body,
        headers: {
          authorization: `Bearer ${this.token}`,
          "content-range": `bytes ${offset}-${offset + chunkSize - 1}/${totalSize}`,
        },
      },
    );
    if (!response.ok) await this.throwResponseError(response);
    return response.json();
  }
  finalizeInputs(id) {
    return this.request(`/v1/jobs/${encodeURIComponent(id)}/inputs:complete`, {
      method: "POST",
    });
  }
  async artifacts(id) {
    return (await this.request(`/v1/jobs/${encodeURIComponent(id)}/artifacts`)).artifacts;
  }

  async waitForJob(id, onUpdate, signal, intervalMs = 400) {
    while (true) {
      if (signal?.aborted) throw new DOMException("Aborted", "AbortError");
      const job = await this.getJob(id);
      onUpdate?.(job);
      if (["succeeded", "failed", "cancelled"].includes(job.status)) return job;
      await new Promise((resolve, reject) => {
        const timer = globalThis.setTimeout(resolve, intervalMs);
        signal?.addEventListener("abort", () => {
          globalThis.clearTimeout(timer);
          reject(new DOMException("Aborted", "AbortError"));
        }, { once: true });
      });
    }
  }

  async request(path, init = {}, authenticated = true) {
    const response = await this.fetcher(`${this.baseUrl}${path}`, {
      ...init,
      cache: init.cache || "no-store",
      headers: {
        "content-type": "application/json",
        ...(authenticated ? { authorization: `Bearer ${this.token}` } : {}),
        ...init.headers,
      },
    });
    if (!response.ok) await this.throwResponseError(response);
    return response.json();
  }

  async throwResponseError(response) {
    let payload = {};
    try {
      payload = await response.json();
    } catch {
      // Preserve the HTTP fallback for non-JSON proxy responses.
    }
    throw new SomerApiError(
      response.status,
      payload.error || "request_failed",
      payload.message || `${response.status} ${response.statusText}`,
    );
  }
}

function encodePath(path) {
  return path.split("/").map(encodeURIComponent).join("/");
}
