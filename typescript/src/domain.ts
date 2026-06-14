import { NdsqError, Reason } from "./errors.ts";
import {
  type BoundJson,
  type IndexValue,
  type ParsedBound,
  boundToJSON,
  parseBound,
  requireArray,
  requireInt,
  requireStringArray,
} from "./values.ts";

export interface DomainFields {
  input_rank: number;
  input_inclusive_min: BoundJson[];
  input_exclusive_max: BoundJson[];
  input_labels: string[];
}

function bumpInclusiveToExclusive(b: ParsedBound): ParsedBound {
  const value: IndexValue = typeof b.value === "number" ? b.value + 1 : b.value;
  return { value, implicit: b.implicit };
}

function addShape(lo: ParsedBound, sz: ParsedBound): ParsedBound {
  let value: IndexValue;
  if (typeof lo.value === "number" && typeof sz.value === "number") value = lo.value + sz.value;
  else if (lo.value === "+inf" || sz.value === "+inf") value = "+inf";
  else value = "-inf";
  return { value, implicit: lo.implicit };
}

export interface RawDomainFields {
  rank?: unknown;
  inclusive_min?: unknown;
  exclusive_max?: unknown;
  inclusive_max?: unknown;
  shape?: unknown;
  labels?: unknown;
}

export function canonicalizeDomain(fields: RawDomainFields): DomainFields {
  const parseBounds = (raw: unknown, name: string): ParsedBound[] | null =>
    raw === undefined ? null : requireArray(raw, name).map((b) => parseBound(b));

  const imin = parseBounds(fields.inclusive_min, "inclusive_min");
  const emax = parseBounds(fields.exclusive_max, "exclusive_max");
  const incmax = parseBounds(fields.inclusive_max, "inclusive_max");
  const shp = parseBounds(fields.shape, "shape");
  const labels = fields.labels === undefined ? null : requireStringArray(fields.labels, "labels");
  let rank = fields.rank === undefined ? null : requireInt(fields.rank, "rank");
  if (rank !== null && rank < 0) {
    throw new NdsqError(Reason.InvalidJson, `rank must be non-negative, got ${rank}`);
  }

  const upperCount = (emax !== null ? 1 : 0) + (incmax !== null ? 1 : 0) + (shp !== null ? 1 : 0);
  if (upperCount > 1) {
    throw new NdsqError(Reason.MultipleUpperBounds, "specify only one of exclusive_max / inclusive_max / shape");
  }

  let resolved = rank;
  for (const arr of [imin, emax, incmax, shp, labels]) {
    if (arr === null) continue;
    if (resolved !== null && resolved !== arr.length) {
      throw new NdsqError(Reason.RankMismatch, `array length ${arr.length} disagrees with rank ${resolved}`);
    }
    resolved = arr.length;
  }
  const r = resolved ?? 0;

  const iminFinal: ParsedBound[] =
    imin ?? Array.from({ length: r }, () => ({ value: 0, implicit: false }) as ParsedBound);

  let exclusive: ParsedBound[];
  if (emax !== null) exclusive = emax;
  else if (incmax !== null) exclusive = incmax.map(bumpInclusiveToExclusive);
  else if (shp !== null) exclusive = iminFinal.map((lo, i) => addShape(lo, shp[i]));
  else exclusive = Array.from({ length: r }, () => ({ value: "+inf", implicit: true }) as ParsedBound);

  return {
    input_rank: r,
    input_inclusive_min: iminFinal.map(boundToJSON),
    input_exclusive_max: exclusive.map(boundToJSON),
    input_labels: labels ?? Array.from({ length: r }, () => ""),
  };
}
