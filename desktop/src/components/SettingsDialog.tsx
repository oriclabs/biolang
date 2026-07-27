import { FilePlus2, HardDrive, RefreshCw, Save, Server, Trash2, X } from "lucide-react";
import { bridge } from "../bridge";
import type { SomerProfile } from "../types";

export function SettingsDialog({
  open,
  onClose,
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
}: {
  open: boolean;
  onClose: () => void;
  fontSize: number;
  setFontSize: React.Dispatch<React.SetStateAction<number>>;
  tabSize: number;
  setTabSize: React.Dispatch<React.SetStateAction<number>>;
  experienceMode: "learner" | "expert";
  setExperienceMode: React.Dispatch<React.SetStateAction<"learner" | "expert">>;
  editorTheme: "biolang-dark" | "vs-dark" | "hc-black";
  setEditorTheme: React.Dispatch<React.SetStateAction<"biolang-dark" | "vs-dark" | "hc-black">>;
  wordWrap: boolean;
  setWordWrap: React.Dispatch<React.SetStateAction<boolean>>;
  minimap: boolean;
  setMinimap: React.Dispatch<React.SetStateAction<boolean>>;
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
}) {
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

  return <div className="dialog-backdrop" onMouseDown={onClose}>
    <section className="settings-dialog" onMouseDown={(event) => event.stopPropagation()} aria-label="Settings">
      <div className="dialog-heading"><span>Settings</span><button type="button" className="icon-button" title="Close" aria-label="Close" onClick={onClose}><X size={14} /></button></div>
      <div className="settings-section-title">Editor</div>
      <div className="setting-row"><span><strong>Interface mode</strong><small>Learner labels navigation; Expert keeps the compact workbench</small></span><div className="setting-segments" role="group" aria-label="Interface mode setting"><button type="button" className={experienceMode === "learner" ? "active" : ""} aria-pressed={experienceMode === "learner"} onClick={() => setExperienceMode("learner")}>Learner</button><button type="button" className={experienceMode === "expert" ? "active" : ""} aria-pressed={experienceMode === "expert"} onClick={() => setExperienceMode("expert")}>Expert</button></div></div>
      <div className="setting-row"><span><strong>Font size</strong><small>Editor text size</small></span><div className="stepper"><button type="button" onClick={() => setFontSize((value) => Math.max(10, value - 1))}>-</button><output>{fontSize}</output><button type="button" onClick={() => setFontSize((value) => Math.min(24, value + 1))}>+</button></div></div>
      <div className="setting-row"><span><strong>Tab width</strong><small>Spaces used for indentation</small></span><select className="setting-select" value={tabSize} onChange={(event) => setTabSize(Number(event.target.value))}><option value={2}>2 spaces</option><option value={4}>4 spaces</option><option value={8}>8 spaces</option></select></div>
      <div className="setting-row"><span><strong>Editor theme</strong><small>Syntax editor contrast</small></span><select className="setting-select" value={editorTheme} onChange={(event) => setEditorTheme(event.target.value as typeof editorTheme)}><option value="biolang-dark">BioLang Dark</option><option value="vs-dark">Dark</option><option value="hc-black">High Contrast</option></select></div>
      <label className="setting-row"><span><strong>Word wrap</strong><small>Wrap long lines in the editor</small></span><input type="checkbox" checked={wordWrap} onChange={(event) => setWordWrap(event.target.checked)} /></label>
      <label className="setting-row"><span><strong>Minimap</strong><small>Show the source overview</small></span><input type="checkbox" checked={minimap} onChange={(event) => setMinimap(event.target.checked)} /></label>
      <label className="setting-row"><span><strong>Bottom panel</strong><small>Keep output and terminal visible</small></span><input type="checkbox" checked={bottomVisible} onChange={(event) => setBottomVisible(event.target.checked)} /></label>
      {hasWorkspace && <div className="setting-row"><span><strong>Workspace trust</strong><small>{workspaceTrusted ? "Execution and native tools are enabled" : "Restricted to browsing and editing"}</small></span><button type="button" className="setting-command" onClick={onToggleTrust}>{workspaceTrusted ? "Revoke" : "Trust"}</button></div>}
      <div className="settings-section-title"><span>Execution</span><button type="button" onClick={addProfile}><FilePlus2 size={12} />Add connection</button></div>
      <div className="setting-row execution-local"><span><strong>Local</strong><small>Integrated BioLang process on this computer</small></span><HardDrive size={16} /></div>
      {somerProfiles.map((profile) => <div className="somer-profile" key={profile.id}>
        <div className="somer-profile-heading"><span><Server size={14} /><strong>{profile.name || "SOMER"}</strong></span><button type="button" className="icon-button" title={`Remove ${profile.name}`} aria-label={`Remove ${profile.name}`} onClick={() => removeProfile(profile)}><Trash2 size={13} /></button></div>
        <div className="connection-fields">
          <label><span>Name</span><input value={profile.name} onChange={(event) => patchProfile(profile.id, { name: event.target.value })} /></label>
          <label><span>Service URL</span><input value={profile.baseUrl} inputMode="url" placeholder="https://somer.example.org" onChange={(event) => patchProfile(profile.id, { baseUrl: event.target.value })} /></label>
          <label><span>Connection</span><select value={profile.connectionMode ?? "direct"} onChange={(event) => patchProfile(profile.id, { connectionMode: event.target.value as SomerProfile["connectionMode"] })}><option value="direct">Direct</option><option value="proxy">HTTP gateway / proxy</option><option value="ssh">SSH tunnel</option></select></label>
          {profile.connectionMode === "proxy" && <label><span>Proxy URL</span><input value={profile.proxyUrl ?? ""} inputMode="url" placeholder="https://gateway.example.org/somer" onChange={(event) => patchProfile(profile.id, { proxyUrl: event.target.value })} /></label>}
          {profile.connectionMode === "ssh" && <>
            <label><span>SSH host</span><input value={profile.sshHost ?? ""} placeholder="login.internal.example" onChange={(event) => patchProfile(profile.id, { sshHost: event.target.value })} /></label>
            <label><span>SSH user</span><input value={profile.sshUser ?? ""} placeholder="researcher" onChange={(event) => patchProfile(profile.id, { sshUser: event.target.value })} /></label>
            <label><span>SSH port</span><input type="number" min={1} max={65535} value={profile.sshPort ?? 22} onChange={(event) => patchProfile(profile.id, { sshPort: Number(event.target.value) })} /></label>
            <label><span>Identity file</span><input value={profile.sshIdentityFile ?? ""} placeholder="Optional private-key path" onChange={(event) => patchProfile(profile.id, { sshIdentityFile: event.target.value })} /></label>
          </>}
          <label><span>Bearer token</span><input type="password" value={somerTokens[profile.id] ?? ""} placeholder="Operating-system credential store" autoComplete="off" onChange={(event) => setSomerTokens((tokens) => ({ ...tokens, [profile.id]: event.target.value }))} /></label>
          <label><span>Resources</span><select value={profile.resourceProfile} onChange={(event) => patchProfile(profile.id, { resourceProfile: event.target.value })}><option value="standard">Standard</option><option value="high-memory">High Memory</option><option value="gpu">GPU</option></select></label>
        </div>
        <div className="connection-actions"><small>{connectionState[profile.id] ?? "Not tested"}</small><button type="button" onClick={() => void onSaveCredential(profile)}><Save size={12} />Save token</button><button type="button" onClick={() => void onForgetCredential(profile)}><Trash2 size={12} />Forget</button><button type="button" onClick={() => void onTestConnection(profile)}><RefreshCw size={12} />Test connection</button></div>
      </div>)}
    </section>
  </div>;
}
