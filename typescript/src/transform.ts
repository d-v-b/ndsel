import { type DomainFields, canonicalizeDomain } from "./domain.ts";
import type { TransformMessage } from "./messages.ts";
import { type OutputMapJson, canonicalizeOutputMap } from "./output.ts";
import { requireArray } from "./values.ts";

/** The canonical core: the domain fields plus explicit output maps (a JSON body). */
export interface Transform extends DomainFields {
  output: OutputMapJson[];
}

export function identityOutput(rank: number): OutputMapJson[] {
  return Array.from({ length: rank }, (_unused, k) => ({ offset: 0, stride: 1, input_dimension: k }));
}

export function canonicalizeTransform(msg: TransformMessage): Transform {
  const domain = canonicalizeDomain({
    rank: msg.input_rank,
    inclusive_min: msg.input_inclusive_min,
    exclusive_max: msg.input_exclusive_max,
    inclusive_max: msg.input_inclusive_max,
    shape: msg.input_shape,
    labels: msg.input_labels,
  });
  const output =
    msg.output === undefined
      ? identityOutput(domain.input_rank)
      : requireArray(msg.output, "output").map(canonicalizeOutputMap);
  return { ...domain, output };
}
