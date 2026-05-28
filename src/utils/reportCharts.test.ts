import { describe, expect, it } from "vitest";
import { chartColorPalette } from "./reportCharts";

describe("chartColorPalette", () => {
  it("returns the requested number of colors", () => {
    expect(chartColorPalette(3)).toHaveLength(3);
    expect(chartColorPalette(20)).toHaveLength(20);
  });

  it("cycles through the base palette", () => {
    const colors = chartColorPalette(15);
    expect(colors[0]).toBe(colors[12]);
  });

  it("returns an empty array for zero colors", () => {
    expect(chartColorPalette(0)).toEqual([]);
  });
});
