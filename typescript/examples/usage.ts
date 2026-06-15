/**
 * Runnable example usage of the ndsel TypeScript API.
 *
 *     cd typescript && node examples/usage.ts
 */
import {
  NdselError,
  box,
  normalize,
  parse,
  point,
  points,
  slice,
  stringifyJson,
} from "../src/index.ts";

function show(label: string, value: unknown): void {
  console.log(`  ${label.padEnd(18)} ${stringifyJson(value)}`);
}

console.log("# parse a JSON message, then normalize to the canonical transform");
show("slice [10:20:2]", normalize(parse('{"kind":"slice","start":[10],"stop":[20],"step":[2]}')));

console.log("\n# build each kind programmatically (camelCase builder functions)");
show("point (4, 7)", normalize(point([4, 7])));
show("box shape 3x4", normalize(box({ shape: [3, 4] })));
show("slice [0:10:2]", normalize(slice({ start: [0], stop: [10], step: [2] })));
show("points", normalize(points([[1, 10], [2, 20]])));
show("transform 3x4", normalize(parse('{"kind":"transform","input_inclusive_min":[0,0],"input_exclusive_max":[3,4]}')));

console.log("\n# error handling: stable, machine-readable reason codes");
for (const bad of [
  '{"kind":"wat"}',
  '{"kind":"slice","start":[0],"stop":[10],"step":[0]}',
  '{"kind":"slice","start":[0],"stop":[10],"step":[-1]}',
]) {
  try {
    normalize(parse(bad));
  } catch (e) {
    if (e instanceof NdselError) console.log(`  ${bad.padEnd(52)} -> ${e.reason}`);
  }
}

console.log("\n# edge cases");
show("big i64 (2n**60n)", normalize(point([2n ** 60n])));
show("unbounded box", normalize(box({ inclusiveMin: [0], exclusiveMax: ["+inf"] })));
