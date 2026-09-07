import type { Filter, Profile } from "./types";
export const SAMPLE_RATE = 48000;
export function coefficients(f: Filter, sr = SAMPLE_RATE): number[] {
  const w = (2 * Math.PI * Math.min(f.frequency, sr / 2 - 1)) / sr,
    c = Math.cos(w),
    s = Math.sin(w),
    a = 10 ** (f.gain / 40),
    alpha = s / (2 * f.q),
    root = 2 * Math.sqrt(a) * alpha;
  switch (f.kind) {
    case "PK":
      return [
        1 + alpha * a,
        -2 * c,
        1 - alpha * a,
        1 + alpha / a,
        -2 * c,
        1 - alpha / a,
      ];
    case "HPQ":
      return [(1 + c) / 2, -(1 + c), (1 + c) / 2, 1 + alpha, -2 * c, 1 - alpha];
    case "LPQ":
      return [(1 - c) / 2, 1 - c, (1 - c) / 2, 1 + alpha, -2 * c, 1 - alpha];
    case "LSC":
      return [
        a * (a + 1 - (a - 1) * c + root),
        2 * a * (a - 1 - (a + 1) * c),
        a * (a + 1 - (a - 1) * c - root),
        a + 1 + (a - 1) * c + root,
        -2 * (a - 1 + (a + 1) * c),
        a + 1 + (a - 1) * c - root,
      ];
    case "HSC":
      return [
        a * (a + 1 + (a - 1) * c + root),
        -2 * a * (a - 1 + (a + 1) * c),
        a * (a + 1 + (a - 1) * c - root),
        a + 1 - (a - 1) * c + root,
        2 * (a - 1 - (a + 1) * c),
        a + 1 - (a - 1) * c - root,
      ];
  }
}
export function filterResponse(
  f: Filter,
  hz: number,
  sr = SAMPLE_RATE,
): number {
  if (!f.enabled) return 0;
  const [b0, b1, b2, a0, a1, a2] = coefficients(f, sr),
    w = (2 * Math.PI * hz) / sr;
  const norm = (x: number, y: number, z: number) =>
    (x + y * Math.cos(w) + z * Math.cos(2 * w)) ** 2 +
    (y * Math.sin(w) + z * Math.sin(2 * w)) ** 2;
  return 10 * Math.log10(Math.max(1e-24, norm(b0, b1, b2) / norm(a0, a1, a2)));
}
export function graphicResponse(p: Profile, hz: number): number {
  const points = p.graphicEq;
  if (!points.length) return 0;
  if (hz <= points[0].frequency) return points[0].gain;
  for (let i = 1; i < points.length; i++) {
    if (hz <= points[i].frequency) {
      const a = points[i - 1],
        b = points[i],
        t = Math.log(hz / a.frequency) / Math.log(b.frequency / a.frequency);
      return a.gain + (b.gain - a.gain) * t;
    }
  }
  return points[points.length - 1].gain;
}
export function response(p: Profile, hz: number, withPreamp = true): number {
  return (
    (withPreamp ? p.preamp : 0) +
    graphicResponse(p, hz) +
    p.filters.reduce((sum, f) => sum + filterResponse(f, hz), 0)
  );
}
export function peak(p: Profile): number {
  let max = -Infinity;
  for (let i = 0; i <= 1024; i++)
    max = Math.max(max, response(p, 10 * 2200 ** (i / 1024), false));
  return max;
}
export function validate(p: Profile): string | null {
  if (!p.name.trim() || p.name.length > 80 || /[\r\n]/.test(p.name))
    return "Enter a profile name (1–80 characters).";
  if (!Number.isFinite(p.preamp) || p.preamp < -60 || p.preamp > 20)
    return "Preamp must be between −60 and +20 dB.";
  for (const f of p.filters) {
    if (
      !Number.isFinite(f.frequency) ||
      f.frequency < 10 ||
      f.frequency > 22000
    )
      return "Frequency must be between 10 and 22000 Hz.";
    if (!Number.isFinite(f.gain) || f.gain < -30 || f.gain > 30)
      return "Gain must be between −30 and +30 dB.";
    if (!Number.isFinite(f.q) || f.q < 0.1 || f.q > 30)
      return "Q must be between 0.1 and 30.";
  }
  return null;
}
