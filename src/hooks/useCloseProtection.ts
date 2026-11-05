import { useEffect, useRef } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export default function useCloseProtection(
  protectedState: boolean,
  onBlocked: () => void,
) {
  const callback = useRef(onBlocked);
  callback.current = onBlocked;
  useEffect(() => {
    if (!protectedState || !isTauri()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    getCurrentWindow()
      .onCloseRequested((event) => {
        event.preventDefault();
        callback.current();
      })
      .then((cleanup) => {
        if (disposed) cleanup();
        else unlisten = cleanup;
      })
      .catch(() => callback.current());
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [protectedState]);
}
