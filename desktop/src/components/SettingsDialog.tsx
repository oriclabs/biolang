import { Check, FilePlus2, FolderOpen, HardDrive, RefreshCw, Save, Server, Trash2, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { bridge, isDesktop } from "../bridge";
import { credentialCatalog, type CredentialStatus } from "../credentials";
import {
  chordFromEvent,
  chordsEqual,
  findKeybindingConflicts,
  formatChord,
  KEYBINDING_DEFINITIONS,
  resolveBindings,
  type KeybindingId,
  type KeybindingMap,
} from "../keybindings";
import type { ReferenceBuild, SomerProfile } from "../types";

export type SettingsSection = "editor" | "trust" | "credentials" | "remote" | "references" | "keyboard";

const sections: Array<{ id: SettingsSection; label: string }> = [
  { id: "editor", label: "Editor" },
  { id: "keyboard", label: "Keyboard" },
  { id: "trust", label: "Trust" },
  { id: "credentials", label: "Credentials" },
  { id: "remote", label: "Remote" },
  { id: "references", label: "References" },
];

export function SettingsDialog({
  open,
  onClose,
  initialSection = "editor",
  fontSize,
  setFontSize,
  tabSize,
  setTabSize,
  experienceMode,
  setExperienceMode,
  editorTheme,
  setEditorTheme,
  wordWrap,
  setWordWrap,
  minimap,
  setMinimap,
  formatOnSave,
  setFormatOnSave,
  showInlineResults,
  setShowInlineResults,
  credentialStatuses,
  referenceBuilds,
  onSaveReferenceBuild,
  onDeleteReferenceBuild,
  onSaveCredentialValue,
  onForgetCredentialValue,
  bottomVisible,
  setBottomVisible,
  hasWorkspace,
  workspaceTrusted,
  onToggleTrust,
  somerProfiles,
  setSomerProfiles,
  executionTarget,
  setExecutionTarget,
  somerTokens,
  setSomerTokens,
  connectionState,
  onSaveCredential,
  onForgetCredential,
  onTestConnection,
  keybindingOverrides,
  setKeybindingOverrides,
}: {
  open: boolean;
  onClose: () => void;
  initialSection?: SettingsSection;
  fontSize: number;
  setFontSize: React.Dispatch<React.SetStateAction<number>>;
  tabSize: number;
  setTabSize: React.Dispatch<React.SetStateAction<number>>;
  experienceMode: "learner" | "expert";
  setExperienceMode: React.Dispatch<React.SetStateAction<"learner" | "expert">>;
  editorTheme: "biolang-dark" | "biolang-light" | "vs-dark" | "hc-black";
  setEditorTheme: React.Dispatch<React.SetStateAction<"biolang-dark" | "biolang-light" | "vs-dark" | "hc-black">>;
  wordWrap: boolean;
  setWordWrap: React.Dispatch<React.SetStateAction<boolean>>;
  minimap: boolean;
  setMinimap: React.Dispatch<React.SetStateAction<boolean>>;
  formatOnSave: boolean;
  setFormatOnSave: React.Dispatch<React.SetStateAction<boolean>>;
  showInlineResults: boolean;
  setShowInlineResults: React.Dispatch<React.SetStateAction<boolean>>;
  credentialStatuses: CredentialStatus[];
  referenceBuilds: ReferenceBuild[];
  onSaveReferenceBuild: (name: string, assets: Record<string, string>) => void | Promise<void>;
  onDeleteReferenceBuild: (name: string) => void | Promise<void>;
  onSaveCredentialValue: (name: string, value: string) => void | Promise<void>;
  onForgetCredentialValue: (name: string) => void | Promise<void>;
  bottomVisible: boolean;
  setBottomVisible: React.Dispatch<React.SetStateAction<boolean>>;
  hasWorkspace: boolean;
  workspaceTrusted: boolean;
  onToggleTrust: () => void;
  somerProfiles: SomerProfile[];
  setSomerProfiles: React.Dispatch<React.SetStateAction<SomerProfile[]>>;
  executionTarget: string;
  setExecutionTarget: React.Dispatch<React.SetStateAction<string>>;
  somerTokens: Record<string, string>;
  setSomerTokens: React.Dispatch<React.SetStateAction<Record<string, string>>>;
  connectionState: Record<string, string>;
  onSaveCredential: (profile: SomerProfile) => void | Promise<void>;
  onForgetCredential: (profile: SomerProfile) => void | Promise<void>;
  onTestConnection: (profile: SomerProfile) => void | Promise<void>;
  keybindingOverrides: KeybindingMap;
  setKeybindingOverrides: React.Dispatch<React.SetStateAction<KeybindingMap>>;
}) {
  const [section, setSection] = useState<SettingsSection>(initialSection);
  const [credentialDrafts, setCredentialDrafts] = useState<Record<string, string>>({});
  const [referenceDraft, setReferenceDraft] = useState({ name: "", fasta: "", gtf: "" });
  const [capturingId, setCapturingId] = useState<KeybindingId>();

  useEffect(() => {
    if (open) setSection(initialSection);
  }, [initialSection, open]);

  useEffect(() => {
    if (!capturingId) return;
    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (event.key === "Escape") {
        setCapturingId(undefined);
        return;
      }
      // Modifier-only presses are not chords.
      if (["Control", "Shift", "Alt", "Meta"].includes(event.key)) return;
      const chord = chordFromEvent(event);
      setKeybindingOverrides((current) => {
        const next = { ...current };
        const definition = KEYBINDING_DEFINITIONS.find((entry) => entry.id === capturingId);
        if (definition && chordsEqual(chord, definition.defaultChord)) {
          delete next[capturingId];
        } else {
          next[capturingId] = chord;
        }
        return next;
      });
      setCapturingId(undefined);
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [capturingId, setKeybindingOverrides]);

  const resolvedBindings = useMemo(
    () => resolveBindings(keybindingOverrides),
    [keybindingOverrides],
  );
  const conflicts = useMemo(
    () => findKeybindingConflicts(resolvedBindings),
    [resolvedBindings],
  );

  if (!open) return null;

  const patchProfile = (id: string, patch: Partial<SomerProfile>) => {
    setSomerProfiles((profiles) => profiles.map((profile) =>
      profile.id === id ? { ...profile, ...patch } : profile));
  };
  const addProfile = () => {
    const id = `somer-${Date.now()}`;
    setSomerProfiles((profiles) => [...profiles, {
      id,
      name: "SOMER Remote",
      baseUrl: "https://",
      resourceProfile: "standard",
      connectionMode: "direct",
    }]);
    setExecutionTarget(id);
  };
  const removeProfile = (profile: SomerProfile) => {
    void bridge.deleteSomerSecret(profile.id).catch(() => undefined);
    void bridge.stopSomerTunnel(profile.id).catch(() => undefined);
    setSomerProfiles((profiles) => profiles.filter((candidate) => candidate.id !== profile.id));
    if (executionTarget === profile.id) setExecutionTarget("local");
  };

  const pickPath = async (
    field: "fasta" | "gtf" | "identity",
    profileId?: string,
  ) => {
    const filters = field === "fasta"
      ? [{ name: "FASTA", extensions: ["fa", "fasta", "fna", "faa"] }]
      : field === "gtf"
        ? [{ name: "Annotation", extensions: ["gtf", "gff", "gff3"] }]
        : undefined;
    const path = await bridge.pickPath({
      title: field === "fasta"
        ? "Choose reference FASTA"
        : field === "gtf"
          ? "Choose GTF or GFF annotation"
          : "Choose SSH identity file",
      filters,
    });
    if (!path) return;
    if (field === "identity" && profileId) {
      patchProfile(profileId, { sshIdentityFile: path });
      return;
    }
    if (field === "fasta" || field === "gtf") {
      setReferenceDraft((draft) => ({ ...draft, [field]: path }));
    }
  };

  return <div className="dialog-backdrop" onMouseDown={onClose}>
    <section className="settings-dialog" onMouseDown={(event) => event.stopPropagation()} aria-label="Settings">
      <div className="dialog-heading">
        <span>Settings</span>
        <button type="button" className="icon-button" title="Close" aria-label="Close" onClick={onClose}><X size={14} /></button>
      </div>

      <div className="settings-nav" role="tablist" aria-label="Settings sections">
        {sections.map((entry) => (
          <button
            type="button"
            role="tab"
            key={entry.id}
            aria-selected={section === entry.id}
            className={section === entry.id ? "active" : ""}
            onClick={() => setSection(entry.id)}
          >{entry.label}</button>
        ))}
      </div>

      <div className="settings-body" role="tabpanel">
        {section === "editor" && <>
          <div className="settings-section-title">Editor</div>
          <div className="setting-row">
            <span><strong>Interface mode</strong><small>Learner labels navigation; Expert keeps the compact workbench</small></span>
            <div className="setting-segments" role="group" aria-label="Interface mode setting">
              <button type="button" className={experienceMode === "learner" ? "active" : ""} aria-pressed={experienceMode === "learner"} onClick={() => setExperienceMode("learner")}>Learner</button>
              <button type="button" className={experienceMode === "expert" ? "active" : ""} aria-pressed={experienceMode === "expert"} onClick={() => setExperienceMode("expert")}>Expert</button>
            </div>
          </div>
          <div className="setting-row">
            <span><strong>Font size</strong><small>Editor text size</small></span>
            <div className="stepper">
              <button type="button" onClick={() => setFontSize((value) => Math.max(10, value - 1))}>-</button>
              <output>{fontSize}</output>
              <button type="button" onClick={() => setFontSize((value) => Math.min(24, value + 1))}>+</button>
            </div>
          </div>
          <div className="setting-row">
            <span><strong>Tab width</strong><small>Spaces used for indentation</small></span>
            <select className="setting-select" value={tabSize} onChange={(event) => setTabSize(Number(event.target.value))}>
              <option value={2}>2 spaces</option>
              <option value={4}>4 spaces</option>
              <option value={8}>8 spaces</option>
            </select>
          </div>
          <div className="setting-row">
            <span><strong>Editor theme</strong><small>Workbench and syntax editor contrast</small></span>
            <select aria-label="Editor theme" className="setting-select" value={editorTheme} onChange={(event) => setEditorTheme(event.target.value as typeof editorTheme)}>
              <option value="biolang-dark">BioLang Dark</option>
              <option value="biolang-light">BioLang Light</option>
              <option value="vs-dark">Dark</option>
              <option value="hc-black">High Contrast</option>
            </select>
          </div>
          <label className="setting-row"><span><strong>Word wrap</strong><small>Wrap long lines in the editor</small></span><input type="checkbox" checked={wordWrap} onChange={(event) => setWordWrap(event.target.checked)} /></label>
          <label className="setting-row"><span><strong>Minimap</strong><small>Show the source overview</small></span><input type="checkbox" checked={minimap} onChange={(event) => setMinimap(event.target.checked)} /></label>
          <label className="setting-row"><span><strong>Format on save</strong><small>Apply the canonical BioLang layout when saving a .bl file</small></span><input type="checkbox" checked={formatOnSave} onChange={(event) => setFormatOnSave(event.target.checked)} /></label>
          <label className="setting-row"><span><strong>Inline run results</strong><small>Show printed values beside the line that produced them</small></span><input type="checkbox" checked={showInlineResults} onChange={(event) => setShowInlineResults(event.target.checked)} /></label>
          <label className="setting-row"><span><strong>Bottom panel</strong><small>Keep output and terminal visible</small></span><input type="checkbox" checked={bottomVisible} onChange={(event) => setBottomVisible(event.target.checked)} /></label>
        </>}

        {section === "trust" && <>
          <div className="settings-section-title">Trust & security</div>
          <p className="settings-note">
            Trusted workspaces may run BioLang, open terminals, install packages, and start language services.
            Restricted mode still allows browsing and editing.
          </p>
          {hasWorkspace
            ? <div className="setting-row">
              <span>
                <strong>Workspace trust</strong>
                <small>{workspaceTrusted ? "Execution and native tools are enabled" : "Restricted to browsing and editing"}</small>
              </span>
              <button type="button" className="setting-command" onClick={onToggleTrust}>
                {workspaceTrusted ? "Revoke" : "Trust"}
              </button>
            </div>
            : <p className="settings-note">Open a workspace to manage trust for that folder.</p>}
        </>}

        {section === "credentials" && <>
          <div className="settings-section-title">Bio API credentials</div>
          <p className="settings-note">
            {isDesktop
              ? "Stored in the operating system keyring and passed to every BioLang process. A key exported in your shell takes precedence."
              : "Credentials need BioLang Desktop: the browser build has no keyring and no process to pass them to."}
          </p>
          {credentialCatalog.map((credential) => {
            const status = credentialStatuses.find((candidate) => candidate.name === credential.name);
            const draft = credentialDrafts[credential.name] ?? "";
            return <div className="credential-row" key={credential.name}>
              <div className="credential-heading">
                <span>
                  <strong>{credential.label}</strong>
                  {credential.required && <em className="credential-required">required</em>}
                  {status?.fromEnvironment
                    ? <em className="credential-state env">from environment</em>
                    : status?.configured
                      ? <em className="credential-state stored"><Check size={10} />stored</em>
                      : <em className="credential-state missing">not set</em>}
                </span>
                <button type="button" className="credential-docs" onClick={() => void bridge.openExternal(credential.docsUrl)}>Get a key</button>
              </div>
              <small>{credential.detail}</small>
              <div className="credential-controls">
                <input
                  type="password"
                  aria-label={`${credential.label} API key`}
                  placeholder={status?.configured ? "Stored — enter a new value to replace" : credential.name}
                  value={draft}
                  disabled={!isDesktop}
                  onChange={(event) => setCredentialDrafts((drafts) => ({ ...drafts, [credential.name]: event.target.value }))}
                />
                <button type="button" disabled={!isDesktop || !draft} onClick={() => void onSaveCredentialValue(credential.name, draft)}>Save</button>
                <button type="button" disabled={!isDesktop || !status?.configured} onClick={() => void onForgetCredentialValue(credential.name)}>Forget</button>
              </div>
            </div>;
          })}
        </>}

        {section === "remote" && <>
          <div className="settings-section-title">
            <span>Execution</span>
            <button type="button" onClick={addProfile}><FilePlus2 size={12} />Add connection</button>
          </div>
          <div className="setting-row execution-local">
            <span><strong>Local</strong><small>Integrated BioLang process on this computer</small></span>
            <HardDrive size={16} />
          </div>
          {somerProfiles.map((profile) => <div className="somer-profile" key={profile.id}>
            <div className="somer-profile-heading">
              <span><Server size={14} /><strong>{profile.name || "SOMER"}</strong></span>
              <button type="button" className="icon-button" title={`Remove ${profile.name}`} aria-label={`Remove ${profile.name}`} onClick={() => removeProfile(profile)}><Trash2 size={13} /></button>
            </div>
            <div className="connection-fields">
              <label><span>Name</span><input value={profile.name} onChange={(event) => patchProfile(profile.id, { name: event.target.value })} /></label>
              <label><span>Service URL</span><input value={profile.baseUrl} inputMode="url" placeholder="https://somer.example.org" onChange={(event) => patchProfile(profile.id, { baseUrl: event.target.value })} /></label>
              <label>
                <span>Connection</span>
                <select value={profile.connectionMode ?? "direct"} onChange={(event) => patchProfile(profile.id, { connectionMode: event.target.value as SomerProfile["connectionMode"] })}>
                  <option value="direct">Direct</option>
                  <option value="proxy">HTTP gateway / proxy</option>
                  <option value="ssh">SSH tunnel</option>
                </select>
              </label>
              {profile.connectionMode === "proxy" && (
                <label><span>Proxy URL</span><input value={profile.proxyUrl ?? ""} inputMode="url" placeholder="https://gateway.example.org/somer" onChange={(event) => patchProfile(profile.id, { proxyUrl: event.target.value })} /></label>
              )}
              {profile.connectionMode === "ssh" && <>
                <label><span>SSH host</span><input value={profile.sshHost ?? ""} placeholder="login.internal.example" onChange={(event) => patchProfile(profile.id, { sshHost: event.target.value })} /></label>
                <label><span>SSH user</span><input value={profile.sshUser ?? ""} placeholder="researcher" onChange={(event) => patchProfile(profile.id, { sshUser: event.target.value })} /></label>
                <label><span>SSH port</span><input type="number" min={1} max={65535} value={profile.sshPort ?? 22} onChange={(event) => patchProfile(profile.id, { sshPort: Number(event.target.value) })} /></label>
                <label className="path-field">
                  <span>Identity file</span>
                  <div className="path-field-row">
                    <input value={profile.sshIdentityFile ?? ""} placeholder="Optional private-key path" onChange={(event) => patchProfile(profile.id, { sshIdentityFile: event.target.value })} />
                    {isDesktop && (
                      <button type="button" className="path-browse" onClick={() => void pickPath("identity", profile.id)} aria-label="Browse for SSH identity file">
                        <FolderOpen size={13} />Browse
                      </button>
                    )}
                  </div>
                </label>
              </>}
              <label><span>Bearer token</span><input type="password" value={somerTokens[profile.id] ?? ""} placeholder="Operating-system credential store" autoComplete="off" onChange={(event) => setSomerTokens((tokens) => ({ ...tokens, [profile.id]: event.target.value }))} /></label>
              <label>
                <span>Resources</span>
                <select value={profile.resourceProfile} onChange={(event) => patchProfile(profile.id, { resourceProfile: event.target.value })}>
                  <option value="standard">Standard</option>
                  <option value="high-memory">High Memory</option>
                  <option value="gpu">GPU</option>
                </select>
              </label>
            </div>
            <div className="connection-actions">
              <small>{connectionState[profile.id] ?? "Not tested"}</small>
              <button type="button" onClick={() => void onSaveCredential(profile)}><Save size={12} />Save token</button>
              <button type="button" onClick={() => void onForgetCredential(profile)}><Trash2 size={12} />Forget</button>
              <button type="button" onClick={() => void onTestConnection(profile)}><RefreshCw size={12} />Test connection</button>
            </div>
          </div>)}
        </>}

        {section === "keyboard" && <>
          <div className="settings-section-title">
            <span>Keyboard shortcuts</span>
            <button
              type="button"
              className="setting-command"
              disabled={!Object.keys(keybindingOverrides).length}
              onClick={() => setKeybindingOverrides({})}
            >Reset all</button>
          </div>
          <p className="settings-note">
            Click a shortcut, then press the new keys. Escape cancels capture. Conflicts are highlighted below.
          </p>
          {conflicts.length > 0 && (
            <div className="keybinding-conflicts" role="status">
              {conflicts.map((conflict) => (
                <p key={`${conflict.ids[0]}-${conflict.ids[1]}`}>
                  <strong>{formatChord(conflict.chord)}</strong> is used by{" "}
                  {KEYBINDING_DEFINITIONS.find((entry) => entry.id === conflict.ids[0])?.label}
                  {" "}and{" "}
                  {KEYBINDING_DEFINITIONS.find((entry) => entry.id === conflict.ids[1])?.label}
                </p>
              ))}
            </div>
          )}
          <div className="keybinding-list">
            {KEYBINDING_DEFINITIONS.map((definition) => {
              const chord = resolvedBindings[definition.id];
              const customized = Boolean(keybindingOverrides[definition.id]);
              const conflicted = conflicts.some((conflict) => conflict.ids.includes(definition.id));
              return (
                <div className={`keybinding-row${conflicted ? " conflict" : ""}`} key={definition.id}>
                  <span>
                    <strong>{definition.label}</strong>
                    <small>{definition.category}{customized ? " · customized" : ""}</small>
                  </span>
                  <div className="keybinding-actions">
                    <button
                      type="button"
                      className={`keybinding-capture${capturingId === definition.id ? " capturing" : ""}`}
                      onClick={() => setCapturingId((current) => current === definition.id ? undefined : definition.id)}
                    >
                      {capturingId === definition.id ? "Press keys…" : formatChord(chord)}
                    </button>
                    <button
                      type="button"
                      className="setting-command"
                      disabled={!customized}
                      onClick={() => setKeybindingOverrides((current) => {
                        const next = { ...current };
                        delete next[definition.id];
                        return next;
                      })}
                    >Reset</button>
                  </div>
                </div>
              );
            })}
          </div>
        </>}

        {section === "references" && <>
          <div className="settings-section-title"><span>Reference builds</span></div>
          <p className="settings-note">
            {isDesktop
              ? "Scripts resolve these by name with ref(\"GRCh38\"), so the same analysis runs on any machine that has the build configured."
              : "Reference builds live in ~/.biolang/references.toml and need BioLang Desktop."}
          </p>
          {referenceBuilds.map((build) => (
            <div className="reference-row" key={build.name}>
              <div className="reference-heading">
                <strong>{build.name}</strong>
                {build.missing.length > 0 && (
                  <em className="reference-missing">{build.missing.length} missing</em>
                )}
                <button type="button" onClick={() => void onDeleteReferenceBuild(build.name)}>Remove</button>
              </div>
              {Object.entries(build.assets).map(([key, value]) => (
                <div className={`reference-asset${build.missing.includes(key) ? " missing" : ""}`} key={key}>
                  <span>{key}</span>
                  <code>{value}</code>
                </div>
              ))}
            </div>
          ))}
          {isDesktop && <div className="reference-new">
            <input
              aria-label="Reference build name"
              placeholder="Build name (GRCh38)"
              value={referenceDraft.name}
              onChange={(event) => setReferenceDraft((draft) => ({ ...draft, name: event.target.value }))}
            />
            <div className="path-field-row">
              <input
                aria-label="Reference FASTA path"
                placeholder="FASTA path"
                value={referenceDraft.fasta}
                onChange={(event) => setReferenceDraft((draft) => ({ ...draft, fasta: event.target.value }))}
              />
              <button type="button" className="path-browse" onClick={() => void pickPath("fasta")} aria-label="Browse for reference FASTA">
                <FolderOpen size={13} />Browse
              </button>
            </div>
            <div className="path-field-row">
              <input
                aria-label="Reference annotation path"
                placeholder="GTF or GFF path (optional)"
                value={referenceDraft.gtf}
                onChange={(event) => setReferenceDraft((draft) => ({ ...draft, gtf: event.target.value }))}
              />
              <button type="button" className="path-browse" onClick={() => void pickPath("gtf")} aria-label="Browse for GTF or GFF">
                <FolderOpen size={13} />Browse
              </button>
            </div>
            <button
              type="button"
              disabled={!referenceDraft.name.trim() || !referenceDraft.fasta.trim()}
              onClick={() => {
                void onSaveReferenceBuild(referenceDraft.name, {
                  fasta: referenceDraft.fasta,
                  gtf: referenceDraft.gtf,
                });
                setReferenceDraft({ name: "", fasta: "", gtf: "" });
              }}
            >Add build</button>
          </div>}
        </>}
      </div>
    </section>
  </div>;
}
