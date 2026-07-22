import { NdselError, Reason } from "./errors.ts";

/**
 * An index integer. Values within the JS safe-integer range are `number`;
 * values outside it (but within i64) are `bigint` (§3.5). The canonical form is
 * representation-independent: a value is `number` iff it is a safe integer.
 */
export type Int = number | bigint;

/** A single index coordinate: an integer, or an infinity sentinel. */
export type IndexValue = Int | "-inf" | "+inf";

/** A JSON-level bound: a value, or that value wrapped in a 1-element array (implicit). */
export type BoundJson = IndexValue | [IndexValue];

/** Internal parsed bound: the value plus its explicit/implicit flag. */
export interface ParsedBound {
  value: IndexValue;
  implicit: boolean;
}

// 64-bit signed range (the canonical contract, §3.5) as bigints.
const I64_MIN = -(2n ** 63n);
const I64_MAX = 2n ** 63n - 1n;
const SAFE_MAX = BigInt(Number.MAX_SAFE_INTEGER); // 2^53 - 1

/** Promote an `Int` to `bigint` for arithmetic. */
export function toBig(x: Int): bigint {
  return typeof x === "bigint" ? x : BigInt(x);
}

/** Demote a `bigint` to `number` when it is a safe integer; keep `bigint` otherwise. */
export function demote(x: bigint): Int {
  return x >= -SAFE_MAX && x <= SAFE_MAX ? Number(x) : x;
}

function checkI64(x: bigint, what: string): Int {
  if (x < I64_MIN || x > I64_MAX) {
    throw new NdselError(Reason.InvalidJson, `${what} value ${x} is out of 64-bit signed range`);
  }
  return demote(x);
}

export function parseIndexValue(raw: unknown): IndexValue {
  if (typeof raw === "bigint") return checkI64(raw, "index");
  if (typeof raw === "number" && Number.isSafeInteger(raw)) return raw;
  if (raw === "-inf" || raw === "+inf") return raw;
  throw new NdselError(Reason.InvalidJson, `invalid index value: ${JSON.stringify(raw)}`);
}

export function parseBound(raw: unknown): ParsedBound {
  if (Array.isArray(raw)) {
    if (raw.length !== 1) {
      throw new NdselError(Reason.InvalidJson, "implicit bound must be a 1-element array");
    }
    return { value: parseIndexValue(raw[0]), implicit: true };
  }
  return { value: parseIndexValue(raw), implicit: false };
}

export function boundToJSON(b: ParsedBound): BoundJson {
  return b.implicit ? [b.value] : b.value;
}

/** Validate a plain integer in 64-bit signed range (no sentinels). */
export function requireInt(raw: unknown, what: string): Int {
  if (typeof raw === "bigint") return checkI64(raw, what);
  if (typeof raw === "number" && Number.isSafeInteger(raw)) return raw;
  throw new NdselError(Reason.InvalidJson, `${what} must be an integer, got ${JSON.stringify(raw)}`);
}

export function requireArray(raw: unknown, what: string): unknown[] {
  if (Array.isArray(raw)) return raw;
  throw new NdselError(Reason.InvalidJson, `${what} must be an array, got ${JSON.stringify(raw)}`);
}

export function requireIntArray(raw: unknown, what: string): Int[] {
  return requireArray(raw, what).map((v, i) => requireInt(v, `${what}[${i}]`));
}

export function requireStringArray(raw: unknown, what: string): string[] {
  return requireArray(raw, what).map((v, i) => {
    if (typeof v !== "string") {
      throw new NdselError(Reason.InvalidJson, `${what}[${i}] must be a string, got ${JSON.stringify(v)}`);
    }
    return v;
  });
}
