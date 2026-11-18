export function formatTime(value: string | null | undefined): string {
  return value ? new Date(value).toLocaleString('zh-CN', { hour12: false }) : '—';
}

export function formatBoolean(value: boolean): string {
  return value ? '是' : '否';
}
