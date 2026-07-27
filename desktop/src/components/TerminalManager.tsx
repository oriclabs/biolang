import { Plus, X } from "lucide-react";
import { useState } from "react";
import { TerminalPane } from "./TerminalPane";

type TerminalTab = {
  id: number;
  name: string;
};

export function TerminalManager() {
  const [nextId, setNextId] = useState(2);
  const [tabs, setTabs] = useState<TerminalTab[]>([{ id: 1, name: "Terminal 1" }]);
  const [activeId, setActiveId] = useState(1);

  const add = () => {
    const tab = { id: nextId, name: `Terminal ${nextId}` };
    setNextId((value) => value + 1);
    setTabs((current) => [...current, tab]);
    setActiveId(tab.id);
  };

  const close = (id: number) => {
    setTabs((current) => {
      const next = current.filter((tab) => tab.id !== id);
      if (activeId === id) setActiveId(next.at(-1)?.id ?? 0);
      return next;
    });
  };

  return (
    <div className="terminal-manager">
      <div className="terminal-tabs">
        {tabs.map((tab) => <button type="button" className={tab.id === activeId ? "active" : ""} key={tab.id} onClick={() => setActiveId(tab.id)}>
          <span>{tab.name}</span>
          <i role="button" tabIndex={0} aria-label={`Close ${tab.name}`} onClick={(event) => { event.stopPropagation(); close(tab.id); }}><X size={11} /></i>
        </button>)}
        <button type="button" className="terminal-add" title="New terminal" aria-label="New terminal" onClick={add} disabled={tabs.length >= 6}><Plus size={13} /></button>
      </div>
      <div className="terminal-sessions">
        {tabs.length === 0
          ? <button type="button" className="terminal-empty" onClick={add}><Plus size={13} />New terminal</button>
          : tabs.map((tab) => <div className="terminal-session" hidden={tab.id !== activeId} key={tab.id}><TerminalPane /></div>)}
      </div>
    </div>
  );
}
