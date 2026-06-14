import { NdsqError, Reason } from "./errors.ts";

/** A single index coordinate: a finite integer, or an infinity sentinel. */
export type IndexValue = number | "-inf" | "+inf";

/** A JSON-level bound: a value, or that value wrapped in a 1-element array (implicit). */
export type BoundJson = IndexValue | [IndexValue];

/** Internal parsed bound: the value plus its explicit/implicit flag. */
export interface ParsedBound {
  value: IndexValue;
  implicit: boolean;
}

export function parseIndexValue(raw: unknown): IndexValue {
  if (typeof raw === "number" && Number.isInteger(raw)) return raw;
  if (raw === "-inf" || raw === "+inf") return raw;
  throw new NdsqError(Reason.InvalidJson, `invalid index value: ${JSON.stringify(raw)}`);
}

export function parseBound(raw: unknown): ParsedBound {
  if (Array.isArray(raw)) {
    if (raw.length !== 1) {
      throw new NdsqError(Reason.InvalidJson, "implicit bound must be a 1-element array");
    }
    return { value: parseIndexValue(raw[0]), implicit: true };
  }
  return { value: parseIndexValue(raw), implicit: false };
}

export function boundToJSON(b: ParsedBound): BoundJson {
  return b.implicit ? [b.value] : b.value;
}

export function requireInt(raw: unknown, what: string): number {
  if (typeof raw === "number" && Number.isInteger(raw)) return raw;
  throw new NdsqError(Reason.InvalidJson, `${what} must be an integer, got ${JSON.stringify(raw)}`);
}

export function requireArray(raw: unknown, what: string): unknown[] {
  if (Array.isArray(raw)) return raw;
  throw new NdsqError(Reason.InvalidJson, `${what} must be an array, got ${JSON.stringify(raw)}`);
}

export function requireIntArray(raw: unknown, what: string): number[] {
  return requireArray(raw, what).map((v, i) => requireInt(v, `${what}[${i}]`));
}

export function requireStringArray(raw: unknown, what: string): string[] {
  return requireArray(raw, what).map((v, i) => {
    if (typeof v !== "string") {
      throw new NdsqError(Reason.InvalidJson, `${what}[${i}] must be a string, got ${JSON.stringify(v)}`);
    }
    return v;
  });
}
