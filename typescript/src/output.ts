import { NdsqError, Reason } from "./errors.ts";
import { type IndexValue, parseIndexValue, requireInt } from "./values.ts";

/** A canonical output map (its JSON shape). */
export type OutputMapJson =
  | { offset: number }
  | { offset: number; stride: number; input_dimension: number }
  | { offset: number; stride: number; index_array: unknown; index_array_bounds: [IndexValue, IndexValue] };

export function canonicalizeOutputMap(raw: unknown): OutputMapJson {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    throw new NdsqError(Reason.InvalidJson, `output map must be an object, got ${JSON.stringify(raw)}`);
  }
  const m = raw as Record<string, unknown>;
  const offset = "offset" in m ? requireInt(m.offset, "output.offset") : 0;
  const stride = "stride" in m ? requireInt(m.stride, "output.stride") : 1;

  if ("index_array" in m) {
    const b = "index_array_bounds" in m ? m.index_array_bounds : ["-inf", "+inf"];
    if (!Array.isArray(b) || b.length !== 2) {
      throw new NdsqError(Reason.InvalidJson, "index_array_bounds must be a 2-element array");
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
      throw new NdsqError(Reason.InvalidJson, `input_dimension must be non-negative, got ${dim}`);
    }
    return { offset, stride, input_dimension: dim };
  }
  return { offset };
}
