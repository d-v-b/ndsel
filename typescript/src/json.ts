// Lossless JSON for the i64 value range.
//
// `JSON.parse` is exact only to 2^53; integers beyond that lose precision. Using
// Node's reviver source-text access, `parseJson` converts every integer literal
// that is NOT a safe JS integer into a `bigint` (small integers stay `number`,
// floats stay `number`). `stringifyJson` is the inverse: it emits `bigint` as a
// bare integer literal (plain `JSON.stringify` throws on `bigint`).

/** A reviver context carrying the original source text of the value (Node 21.7+). */
interface ReviverContext {
  source: string;
}

/** Parse JSON, representing integers outside the JS safe range as `bigint`. */
export function parseJson(text: string): unknown {
  return JSON.parse(text, (_key: string, value: unknown, context?: ReviverContext): unknown => {
    if (
      typeof value === "number" &&
      !Number.isSafeInteger(value) &&
      context !== undefined &&
      /^-?\d+$/.test(context.source)
    ) {
      return BigInt(context.source);
    }
    return value;
  });
}

/** Serialize a value to JSON, emitting `bigint` as a bare integer literal. */
export function stringifyJson(value: unknown): string {
  if (typeof value === "bigint") return value.toString();
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(stringifyJson).join(",")}]`;
  const members = Object.entries(value as Record<string, unknown>)
    .filter(([, v]) => v !== undefined)
    .map(([k, v]) => `${JSON.stringify(k)}:${stringifyJson(v)}`);
  return `{${members.join(",")}}`;
}
