import { BookOpen, Check, Code2, Copy, FileCode2, FolderOpen, Library, Sparkles } from "lucide-react";
import { Children, isValidElement, type ReactNode, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { HelpEntry, HelpKind } from "../types";

const kindLabels: Record<HelpKind, string> = {
  language: "Language guide",
  builtin: "Builtin reference",
  tutorial: "Tutorial",
  example: "Example",
};

function textContent(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") return String(node);
  if (Array.isArray(node)) return node.map(textContent).join("");
  if (isValidElement<{ children?: ReactNode }>(node)) return textContent(node.props.children);
  return "";
}

function headingId(children: ReactNode) {
  return `help-${textContent(children)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")}`;
}

function CodeBlock({ children }: { children?: ReactNode }) {
  const [copied, setCopied] = useState(false);
  const first = Children.toArray(children)[0];
  const className = isValidElement<{ className?: string }>(first) ? first.props.className : undefined;
  const language = className?.replace("language-", "") || "text";
  const label = language === "biolang"
    ? "BioLang"
    : language === "bash" || language === "shell" || language === "powershell"
      ? "Shell"
      : language;
  const code = textContent(children).replace(/\n$/, "");

  return <div className="help-code">
    <div><span><Code2 size={13} />{label}</span><button type="button" aria-label={`Copy ${label} code`} onClick={() => {
      void navigator.clipboard.writeText(code).then(() => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1_500);
      });
    }}>{copied ? <Check size={12} /> : <Copy size={12} />}{copied ? "Copied" : "Copy"}</button></div>
    <pre>{children}</pre>
  </div>;
}

export default function HelpDocument({
  entry,
  canOpenSource,
  canInsert,
  onOpenSource,
  onInsert,
  onNavigate,
}: {
  entry?: HelpEntry;
  canOpenSource: boolean;
  canInsert: boolean;
  onOpenSource: (path: string) => void;
  onInsert: (text: string) => void;
  onNavigate: (href: string) => void;
}) {
  if (!entry) {
    return (
      <div className="help-empty">
        <Library size={28} />
        <strong>No help entry selected</strong>
        <span>Choose a language topic, builtin, tutorial, or example.</span>
      </div>
    );
  }

  const insertable = entry.example ?? entry.code;
  return (
    <article className="help-document" data-help-kind={entry.kind}>
      <header className="help-document-header">
        <div>
          <span className="help-kind"><BookOpen size={13} />{kindLabels[entry.kind]}</span>
          <h1>{entry.title}</h1>
          <p>{entry.summary}</p>
          <div className="help-meta"><span>{entry.collection}</span><span>{entry.category}</span>{entry.returnType && <span>Returns {entry.returnType}</span>}</div>
        </div>
        <div className="help-document-actions">
          {entry.sourcePath && <button type="button" onClick={() => onOpenSource(entry.sourcePath!)} disabled={!canOpenSource}><FolderOpen size={14} />Open source</button>}
          {insertable && <button type="button" className="primary" onClick={() => onInsert(insertable)} disabled={!canInsert}><FileCode2 size={14} />Insert in editor</button>}
        </div>
      </header>
      <div className="help-markdown">
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          components={{
            a: ({ children, href }) => <button type="button" className="help-link" title={href} onClick={() => href && onNavigate(href)}>{children}</button>,
            img: ({ alt }) => <span className="help-media-placeholder"><Sparkles size={14} />{alt || "Documentation image"}</span>,
            h1: ({ children }) => <h1 id={headingId(children)}>{children}</h1>,
            h2: ({ children }) => <h2 id={headingId(children)}>{children}</h2>,
            h3: ({ children }) => <h3 id={headingId(children)}>{children}</h3>,
            h4: ({ children }) => <h4 id={headingId(children)}>{children}</h4>,
            code: ({ className, children }) => {
              const block = className?.startsWith("language-");
              return block
                ? <code className={className}>{children}</code>
                : <code>{children}</code>;
            },
            pre: ({ children }) => <CodeBlock>{children}</CodeBlock>,
          }}
        >
          {entry.body}
        </ReactMarkdown>
      </div>
    </article>
  );
}
