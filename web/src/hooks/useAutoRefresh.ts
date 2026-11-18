export function useAutoRefresh(callback: () => void, intervalMs: number): () => void {
  let stopped = false;
  const timer = setInterval(() => {
    if (!stopped) callback();
  }, intervalMs);

  return () => {
    stopped = true;
    clearInterval(timer);
  };
}
