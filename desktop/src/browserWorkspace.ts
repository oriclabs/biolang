import type { WorkspaceSnapshot } from "./types";

export interface BrowserWorkspaceState {
  selected: boolean;
  workspace: WorkspaceSnapshot;
  files: Record<string, string>;
}

const databaseName = "biolang-studio-web";
const storeName = "workspace";
const stateKey = "current";
let databasePromise: Promise<IDBDatabase> | undefined;

function openDatabase(): Promise<IDBDatabase> {
  if (databasePromise) return databasePromise;
  databasePromise = new Promise((resolve, reject) => {
    const request = window.indexedDB.open(databaseName, 1);
    request.onupgradeneeded = () => {
      if (!request.result.objectStoreNames.contains(storeName)) {
        request.result.createObjectStore(storeName);
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error("Cannot open browser workspace"));
  });
  return databasePromise;
}

export async function loadBrowserWorkspace(): Promise<BrowserWorkspaceState | undefined> {
  const database = await openDatabase();
  return new Promise((resolve, reject) => {
    const transaction = database.transaction(storeName, "readonly");
    const request = transaction.objectStore(storeName).get(stateKey);
    request.onsuccess = () => resolve(request.result as BrowserWorkspaceState | undefined);
    request.onerror = () => reject(request.error ?? new Error("Cannot load browser workspace"));
  });
}

export async function saveBrowserWorkspace(state: BrowserWorkspaceState): Promise<void> {
  const database = await openDatabase();
  await new Promise<void>((resolve, reject) => {
    const transaction = database.transaction(storeName, "readwrite");
    transaction.objectStore(storeName).put(structuredClone(state), stateKey);
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error ?? new Error("Cannot save browser workspace"));
    transaction.onabort = () => reject(transaction.error ?? new Error("Browser workspace save was aborted"));
  });
}
