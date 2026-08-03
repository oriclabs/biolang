import { Globe2, X } from "lucide-react";
import { useState } from "react";

type Props = {
  onClose: () => void;
  onImport: (url: string) => Promise<void>;
};

export function ImportUrlDialog({ onClose, onImport }: Props) {
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    const value = url.trim();
    if (!/^https?:\/\//i.test(value)) return;
    setBusy(true);
    try {
      await onImport(value);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="dialog-backdrop" onMouseDown={onClose}>
      <form
        className="prompt-dialog"
        aria-label="Import script from URL"
        onMouseDown={(event) => event.stopPropagation()}
        onSubmit={(event) => {
          event.preventDefault();
          void submit();
        }}
      >
        <div className="dialog-heading">
          <span><Globe2 size={14} /> Import Script from URL</span>
          <button type="button" className="icon-button" aria-label="Close" onClick={onClose}>
            <X size={14} />
          </button>
        </div>
        <label htmlFor="import-script-url">HTTP or HTTPS URL</label>
        <input
          id="import-script-url"
          type="url"
          autoFocus
          value={url}
          placeholder="https://example.org/analysis.py"
          onChange={(event) => setUrl(event.target.value)}
          spellCheck={false}
        />
        <div className="dialog-actions">
          <button type="button" onClick={onClose}>Cancel</button>
          <button
            type="submit"
            className="primary"
            disabled={busy || !/^https?:\/\//i.test(url.trim())}
          >
            {busy ? "Downloading..." : "Download and Review"}
          </button>
        </div>
      </form>
    </div>
  );
}
