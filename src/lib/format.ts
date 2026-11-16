export function formatPlaytime(secs: number): string {
  if (secs < 60) return secs > 0 ? "< 1m" : "0m";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h === 0) return `${m}m`;
  if (h >= 100) return `${h}h`;
  return m ? `${h}h ${m}m` : `${h}h`;
}

/** Large totals read better as whole hours: "1,588h". */
export function formatHours(secs: number): string {
  const h = secs / 3600;
  return h >= 10 ? `${Math.floor(h).toLocaleString()}h` : formatPlaytime(secs);
}

/** Compact card style: "45 m", "2.5 h", "118.4 h". */
export function formatHoursDecimal(secs: number): string {
  if (secs < 3600) return `${Math.max(1, Math.round(secs / 60))} m`;
  return `${(Math.floor(secs / 360) / 10).toLocaleString(undefined, { minimumFractionDigits: 1 })} h`;
}

export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(value >= 100 || unit === 0 ? 0 : 1)} ${units[unit]}`;
}

export function formatDate(unixSecs: number): string {
  return new Date(unixSecs * 1000).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

export function formatRelative(unixSecs: number): string {
  const diff = unixSecs - Date.now() / 1000;
  const abs = Math.abs(diff);
  if (abs < 60) return "just now";
  if (abs < 3600) return rtf.format(Math.round(diff / 60), "minute");
  if (abs < 86400) return rtf.format(Math.round(diff / 3600), "hour");
  if (abs < 86400 * 30) return rtf.format(Math.round(diff / 86400), "day");
  if (abs < 86400 * 365) return rtf.format(Math.round(diff / (86400 * 30)), "month");
  return rtf.format(Math.round(diff / (86400 * 365)), "year");
}

/** Deterministic hue for a title, used for cover placeholders. */
export function titleHue(title: string): number {
  let h = 0;
  for (let i = 0; i < title.length; i++) h = (h * 31 + title.charCodeAt(i)) >>> 0;
  return h % 360;
}

export function initials(title: string): string {
  const words = title
    .replace(/[^\p{L}\p{N}\s]/gu, " ")
    .split(/\s+/)
    .filter(Boolean);
  if (words.length === 0) return "?";
  if (words.length === 1) return words[0].slice(0, 2).toUpperCase();
  return (words[0][0] + words[1][0]).toUpperCase();
}
