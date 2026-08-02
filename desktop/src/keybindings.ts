/**
 * Configurable workbench keybindings.
 *
 * Defaults match the historical hard-coded App shortcuts. Overrides persist in
 * localStorage; conflicts are reported so two commands do not share a chord.
 */

export type KeybindingId =
  | "help"
  | "save"
  | "saveAs"
  | "newFile"
  | "closeEditor"
  | "run"
  | "sendToConsole"
  | "commandPalette"
  | "goToFile"
  | "togglePanel"
  | "explorer"
  | "search"
  | "scm"
  | "runTests"
  | "goToSymbol"
  | "settings"
  | "splitEditor"
  | "wordWrap"
  | "console"
  | "terminal";

export type KeyChord = {
  /** Lower-case key or special: enter, escape, f1, comma, backslash, backquote */
  key: string;
  /** Ctrl on Windows/Linux; treated as Ctrl-or-Meta on match for macOS. */
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
};

export type KeybindingDefinition = {
  id: KeybindingId;
  label: string;
  category: string;
  defaultChord: KeyChord;
  /** Restrict where the binding fires. */
  when?: "global" | "editor";
};

export const KEYBINDING_DEFINITIONS: KeybindingDefinition[] = [
  { id: "help", label: "Help Center", category: "Help", defaultChord: { key: "f1" } },
  { id: "commandPalette", label: "Command Palette", category: "Navigation", defaultChord: { key: "p", ctrl: true, shift: true } },
  { id: "goToFile", label: "Go to File", category: "Navigation", defaultChord: { key: "p", ctrl: true } },
  { id: "goToSymbol", label: "Go to Symbol", category: "Navigation", defaultChord: { key: "o", ctrl: true, shift: true } },
  { id: "settings", label: "Settings", category: "Preferences", defaultChord: { key: "comma", ctrl: true } },
  { id: "save", label: "Save", category: "File", defaultChord: { key: "s", ctrl: true } },
  { id: "saveAs", label: "Save As", category: "File", defaultChord: { key: "s", ctrl: true, shift: true } },
  { id: "newFile", label: "New File", category: "File", defaultChord: { key: "n", ctrl: true } },
  { id: "closeEditor", label: "Close Editor", category: "File", defaultChord: { key: "w", ctrl: true } },
  { id: "run", label: "Run Active File", category: "Run", defaultChord: { key: "enter", ctrl: true } },
  { id: "sendToConsole", label: "Send Selection to Console", category: "Run", defaultChord: { key: "enter", shift: true }, when: "editor" },
  { id: "runTests", label: "Run Tests", category: "Run", defaultChord: { key: "t", ctrl: true, shift: true } },
  { id: "explorer", label: "Explorer", category: "View", defaultChord: { key: "e", ctrl: true, shift: true } },
  { id: "search", label: "Search", category: "View", defaultChord: { key: "f", ctrl: true, shift: true } },
  { id: "scm", label: "Source Control", category: "View", defaultChord: { key: "g", ctrl: true, shift: true } },
  { id: "togglePanel", label: "Toggle Bottom Panel", category: "View", defaultChord: { key: "j", ctrl: true } },
  { id: "splitEditor", label: "Split Editor Right", category: "View", defaultChord: { key: "backslash", ctrl: true } },
  { id: "wordWrap", label: "Word Wrap", category: "View", defaultChord: { key: "z", alt: true } },
  { id: "console", label: "BioLang Console", category: "Terminal", defaultChord: { key: "backquote", ctrl: true, shift: true } },
  { id: "terminal", label: "New Terminal", category: "Terminal", defaultChord: { key: "backquote", ctrl: true } },
];

export type KeybindingMap = Partial<Record<KeybindingId, KeyChord>>;

const STORAGE_KEY = "biolang.desktop.keybindings";

function normalizeKeyFromEvent(event: KeyboardEvent): string {
  if (event.key === "F1") return "f1";
  if (event.key === "Enter") return "enter";
  if (event.key === "Escape") return "escape";
  if (event.key === "," || event.code === "Comma") return "comma";
  if (event.key === "\\" || event.code === "Backslash") return "backslash";
  if (event.code === "Backquote" || event.key === "`") return "backquote";
  if (event.key.length === 1) return event.key.toLowerCase();
  return event.key.toLowerCase();
}

export function chordFromEvent(event: KeyboardEvent): KeyChord {
  return {
    key: normalizeKeyFromEvent(event),
    ctrl: event.ctrlKey || event.metaKey || undefined,
    shift: event.shiftKey || undefined,
    alt: event.altKey || undefined,
  };
}

export function normalizeChord(chord: KeyChord): KeyChord {
  return {
    key: chord.key.toLowerCase(),
    ctrl: chord.ctrl || undefined,
    shift: chord.shift || undefined,
    alt: chord.alt || undefined,
  };
}

export function chordsEqual(left: KeyChord, right: KeyChord): boolean {
  const a = normalizeChord(left);
  const b = normalizeChord(right);
  return a.key === b.key
    && Boolean(a.ctrl) === Boolean(b.ctrl)
    && Boolean(a.shift) === Boolean(b.shift)
    && Boolean(a.alt) === Boolean(b.alt);
}

export function matchEvent(event: KeyboardEvent, chord: KeyChord): boolean {
  const expected = normalizeChord(chord);
  const actualKey = normalizeKeyFromEvent(event);
  if (actualKey !== expected.key) return false;
  const mod = event.ctrlKey || event.metaKey;
  if (Boolean(expected.ctrl) !== mod) return false;
  if (Boolean(expected.shift) !== event.shiftKey) return false;
  if (Boolean(expected.alt) !== event.altKey) return false;
  return true;
}

export function formatChord(chord: KeyChord, platform: string = navigator.platform): string {
  const isMac = /mac/i.test(platform);
  const parts: string[] = [];
  if (chord.ctrl) parts.push(isMac ? "⌘" : "Ctrl");
  if (chord.alt) parts.push(isMac ? "⌥" : "Alt");
  if (chord.shift) parts.push(isMac ? "⇧" : "Shift");
  const key = chord.key;
  const pretty = key === "enter" ? "Enter"
    : key === "escape" ? "Esc"
    : key === "f1" ? "F1"
    : key === "comma" ? ","
    : key === "backslash" ? "\\"
    : key === "backquote" ? "`"
    : key.length === 1 ? key.toUpperCase()
    : key;
  parts.push(pretty);
  return isMac ? parts.join("") : parts.join("+");
}

export function defaultBindings(): Record<KeybindingId, KeyChord> {
  return Object.fromEntries(
    KEYBINDING_DEFINITIONS.map((entry) => [entry.id, { ...entry.defaultChord }]),
  ) as Record<KeybindingId, KeyChord>;
}

export function resolveBindings(overrides: KeybindingMap = {}): Record<KeybindingId, KeyChord> {
  const resolved = defaultBindings();
  for (const definition of KEYBINDING_DEFINITIONS) {
    const override = overrides[definition.id];
    if (override?.key) resolved[definition.id] = normalizeChord(override);
  }
  return resolved;
}

export function findKeybindingConflicts(
  bindings: Record<KeybindingId, KeyChord>,
): Array<{ ids: [KeybindingId, KeybindingId]; chord: KeyChord }> {
  const conflicts: Array<{ ids: [KeybindingId, KeybindingId]; chord: KeyChord }> = [];
  const ids = KEYBINDING_DEFINITIONS.map((entry) => entry.id);
  for (let i = 0; i < ids.length; i += 1) {
    for (let j = i + 1; j < ids.length; j += 1) {
      const left = ids[i]!;
      const right = ids[j]!;
      if (chordsEqual(bindings[left], bindings[right])) {
        conflicts.push({ ids: [left, right], chord: bindings[left] });
      }
    }
  }
  return conflicts;
}

export function loadKeybindingOverrides(): KeybindingMap {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as KeybindingMap;
    if (!parsed || typeof parsed !== "object") return {};
    const cleaned: KeybindingMap = {};
    for (const definition of KEYBINDING_DEFINITIONS) {
      const value = parsed[definition.id];
      if (value && typeof value.key === "string") cleaned[definition.id] = normalizeChord(value);
    }
    return cleaned;
  } catch {
    return {};
  }
}

export function saveKeybindingOverrides(overrides: KeybindingMap): void {
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(overrides));
}

export function definitionFor(id: KeybindingId): KeybindingDefinition {
  return KEYBINDING_DEFINITIONS.find((entry) => entry.id === id)!;
}

/** Match the first command whose chord equals the event (order = definition order). */
export function commandForEvent(
  event: KeyboardEvent,
  bindings: Record<KeybindingId, KeyChord>,
): KeybindingDefinition | undefined {
  for (const definition of KEYBINDING_DEFINITIONS) {
    if (!matchEvent(event, bindings[definition.id])) continue;
    if (definition.when === "editor") {
      const target = event.target as HTMLElement | null;
      if (!target?.closest(".editor-host, .editor-surface, .monaco-editor")) continue;
    }
    return definition;
  }
  return undefined;
}
