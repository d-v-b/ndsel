import { NdselError, Reason } from "./errors.ts";
import { type IndexValue, type Int, parseIndexValue, requireInt } from "./values.ts";

/** A canonical output map (its JSON shape). */
export type OutputMapJson =
  | { offset: Int }
  | { offset: Int; stride: Int; input_dimension: Int }
  | { offset: Int; stride: Int; index_array: unknown; index_array_bounds: [IndexValue, IndexValue] };

const OUTPUT_MAP_FIELDS = new Set([
  "offset",
  "stride",
  "input_dimension",
  "index_array",
  "index_array_bounds",
]);

export function canonicalizeOutputMap(raw: unknown): OutputMapJson {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    throw new NdselError(Reason.InvalidJson, `output map must be an object, got ${JSON.stringify(raw)}`);
  }
  const m = raw as Record<string, unknown>;
  for (const key of Object.keys(m)) {
    if (!OUTPUT_MAP_FIELDS.has(key)) {
      throw new NdselError(Reason.UnknownField, `unrecognized output map field: ${key}`);
    }
  }
  if ("index_array" in m && "input_dimension" in m) {
    throw new NdselError(Reason.OutputMapConflict, "output map must not carry both input_dimension and index_array");
  }
  const offset = "offset" in m ? requireInt(m.offset, "output.offset") : 0;
  const stride = "stride" in m ? requireInt(m.stride, "output.stride") : 1;

  if ("index_array" in m) {
    const b = "index_array_bounds" in m ? m.index_array_bounds : ["-inf", "+inf"];
    if (!Array.isArray(b) || b.length !== 2) {
      throw new NdselError(Reason.InvalidJson, "index_array_bounds must be a 2-element array");
    }
    return {
      offset,
      stride,
      index_array: m.index_array,
      index_array_bounds: [parseIndexValue(b[0]), parseIndexValue(b[1])],
    };
  }
  if ("input_dimension" in m) {
    const dim = requireInt(m.input_dimension, "output.input_dimension");
    if (dim < 0) {
      throw new NdselError(Reason.InvalidJson, `input_dimension must be non-negative, got ${dim}`);
    }
    return { offset, stride, input_dimension: dim };
  }
  return { offset };
}
