import { describe, it, expect } from "vitest";
import { filterResponse, response, peak, validate } from "./eq";
import type { Filter, Profile } from "./types";
const filter: Filter = {
  id: "f",
  kind: "PK",
  enabled: true,
  frequency: 1000,
  gain: 6,
  q: 1,
};
const profile: Profile = {
  schemaVersion: 1,
  id: "p",
  name: "test",
  icon: "music",
  headphoneId: "default",
  preamp: -6,
  filters: [filter],
  graphicEq: [],
  provenance: "test",
};
describe("EQ response", () => {
  it("peaking filter reaches its specified center gain", () =>
    expect(filterResponse(filter, 1000)).toBeCloseTo(6, 8));
  it("disabled filters contribute zero", () =>
    expect(filterResponse({ ...filter, enabled: false }, 1000)).toBe(0));
  it("preamp adds in decibels", () =>
    expect(response(profile, 1000)).toBeCloseTo(0, 8));
  it("flat EQ is flat", () =>
    expect(response({ ...profile, preamp: 0, filters: [] }, 45)).toBe(0));
  it("shelves converge to their shelf gain", () => {
    expect(filterResponse({ ...filter, kind: "LSC" }, 10)).toBeCloseTo(6, 2);
    expect(filterResponse({ ...filter, kind: "HSC" }, 22000)).toBeCloseTo(6, 2);
  });
  it("pass filters attenuate beyond cutoff", () => {
    expect(
      filterResponse({ ...filter, kind: "HPQ", q: Math.SQRT1_2 }, 100),
    ).toBeLessThan(-39);
    expect(
      filterResponse({ ...filter, kind: "LPQ", q: Math.SQRT1_2 }, 10000),
    ).toBeLessThan(-39);
  });
  it("estimates sufficient headroom", () =>
    expect(peak(profile)).toBeCloseTo(6, 2));
  it("graphic EQ interpolates on log frequency", () =>
    expect(
      response(
        {
          ...profile,
          preamp: 0,
          filters: [],
          graphicEq: [
            { frequency: 100, gain: 0 },
            { frequency: 10000, gain: 10 },
          ],
        },
        1000,
      ),
    ).toBeCloseTo(5));
  it("rejects incomplete and nonfinite edits", () => {
    expect(validate({ ...profile, preamp: NaN })).toBeTruthy();
    expect(
      validate({ ...profile, filters: [{ ...filter, q: 0 }] }),
    ).toBeTruthy();
  });
});
