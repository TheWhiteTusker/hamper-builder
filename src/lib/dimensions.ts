/** Units and dimension-to-area conversion for cost lines. */

import { num, round2 } from "./numbers.ts";

export type DimensionUnit = "inch" | "mm" | "cm";

export const COMMON_UNITS = [
  "inch",
  "mm",
  "cm",
  "feet",
  "meter",
  "sq ft",
  "sq inch",
  "sq mm",
  "sq cm",
  "running ft",
  "piece",
  "kg",
  "set",
  "min",
  "hour",
] as const;

export type UnitType = (typeof COMMON_UNITS)[number];

/**
 * Calculates the unit area/quantity based on entered dimensions and the pricing unit.
 */
export function dimensionArea(
  lengthVal: unknown,
  breadthVal: unknown,
  dimensionUnit: DimensionUnit = "inch",
  targetUnit: string = "sq ft",
): number {
  const l = num(lengthVal);
  const b = num(breadthVal);

  const unitLower = (targetUnit || "").trim().toLowerCase();

  const isLinear = [
    "feet",
    "ft",
    "running ft",
    "rft",
    "inch",
    "in",
    "mm",
    "cm",
    "meter",
    "m",
  ].includes(unitLower);

  if (isLinear) {
    if (l <= 0) return 1;

    if (
      unitLower === "feet" ||
      unitLower === "ft" ||
      unitLower === "running ft" ||
      unitLower === "rft"
    ) {
      let lInches = l;
      if (dimensionUnit === "mm") lInches = l / 25.4;
      else if (dimensionUnit === "cm") lInches = l / 2.54;
      return lInches / 12;
    }

    if (unitLower === "inch" || unitLower === "in") {
      let lInches = l;
      if (dimensionUnit === "mm") lInches = l / 25.4;
      else if (dimensionUnit === "cm") lInches = l / 2.54;
      return lInches;
    }

    if (unitLower === "mm") {
      let lMm = l;
      if (dimensionUnit === "inch") lMm = l * 25.4;
      else if (dimensionUnit === "cm") lMm = l * 10;
      return lMm;
    }

    if (unitLower === "cm") {
      let lCm = l;
      if (dimensionUnit === "inch") lCm = l * 2.54;
      else if (dimensionUnit === "mm") lCm = l / 10;
      return lCm;
    }

    if (unitLower === "meter" || unitLower === "m") {
      let lMeters = l;
      if (dimensionUnit === "cm") lMeters = l / 100;
      else if (dimensionUnit === "mm") lMeters = l / 1000;
      else if (dimensionUnit === "inch") lMeters = (l * 2.54) / 100;
      return lMeters;
    }
  }

  if (l <= 0 || b <= 0) return 1;

  // Convert dimensions to inches first as a common baseline
  let lInches = l;
  let bInches = b;

  if (dimensionUnit === "mm") {
    lInches = l / 25.4;
    bInches = b / 25.4;
  } else if (dimensionUnit === "cm") {
    lInches = l / 2.54;
    bInches = b / 2.54;
  }

  if (unitLower === "sq ft" || unitLower === "sqft" || unitLower === "sft") {
    return (lInches * bInches) / 144;
  }

  if (unitLower === "sq inch" || unitLower === "sqinch" || unitLower === "sq in" || unitLower === "sqin") {
    return lInches * bInches;
  }

  if (unitLower === "sq mm" || unitLower === "sqmm") {
    const lMm =
      dimensionUnit === "mm"
        ? l
        : dimensionUnit === "cm"
          ? l * 10
          : lInches * 25.4;
    const bMm =
      dimensionUnit === "mm"
        ? b
        : dimensionUnit === "cm"
          ? b * 10
          : bInches * 25.4;
    return lMm * bMm;
  }

  if (unitLower === "sq cm" || unitLower === "sqcm") {
    const lCm =
      dimensionUnit === "cm"
        ? l
        : dimensionUnit === "mm"
          ? l / 10
          : lInches * 2.54;
    const bCm =
      dimensionUnit === "cm"
        ? b
        : dimensionUnit === "mm"
          ? b / 10
          : bInches * 2.54;
    return lCm * bCm;
  }

  // For piece / each / nos or other units, dimensions are descriptive rather than multiplying
  return 1;
}

/** dimensionArea() rounded to 2 decimals, for display. */
export const calculateDimensionArea = (...args: Parameters<typeof dimensionArea>) => round2(dimensionArea(...args));
