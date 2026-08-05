import { useCallback, useEffect, useState } from "react";
import { isDesktop } from "../bridge";

interface InstallPromptEvent extends Event {
  prompt: () => Promise<void>;
  userChoice: Promise<{ outcome: "accepted" | "dismissed"; platform: string }>;
}

export function usePwa(showNotice: (message: string) => void) {
  const [installPrompt, setInstallPrompt] = useState<InstallPromptEvent>();
  const [online, setOnline] = useState(() => window.navigator.onLine);
  const standalone = window.matchMedia("(display-mode: standalone)").matches;

  useEffect(() => {
    if (isDesktop) return;
    const updateOnline = () => setOnline(window.navigator.onLine);
    const capturePrompt = (event: Event) => {
      event.preventDefault();
      setInstallPrompt(event as InstallPromptEvent);
    };
    const installed = () => {
      setInstallPrompt(undefined);
      showNotice("BioLang Workbench Web installed");
    };
    window.addEventListener("online", updateOnline);
    window.addEventListener("offline", updateOnline);
    window.addEventListener("beforeinstallprompt", capturePrompt);
    window.addEventListener("appinstalled", installed);

    if (import.meta.env.PROD && "serviceWorker" in navigator) {
      void navigator.serviceWorker.register("./sw.js", { scope: "./" })
        .catch((error) => showNotice(`Offline support unavailable: ${String(error)}`));
    }

    return () => {
      window.removeEventListener("online", updateOnline);
      window.removeEventListener("offline", updateOnline);
      window.removeEventListener("beforeinstallprompt", capturePrompt);
      window.removeEventListener("appinstalled", installed);
    };
  }, [showNotice]);

  const install = useCallback(async () => {
    if (!installPrompt) return;
    await installPrompt.prompt();
    const choice = await installPrompt.userChoice;
    if (choice.outcome === "accepted") setInstallPrompt(undefined);
  }, [installPrompt]);

  return {
    canInstall: !isDesktop && !standalone && Boolean(installPrompt),
    install,
    online,
    standalone,
  };
}
