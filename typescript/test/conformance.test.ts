import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { Ajv2020 } from "ajv/dist/2020.js";
import { parseJson, stringifyJson } from "../src/json.ts";
import { NdselError, normalize, parse } from "../src/index.ts";

const repoRoot = join(import.meta.dirname, "..", "..");
const corpusDir = join(repoRoot, "conformance");
const schemaPath = join(repoRoot, "schema", "ndsel.schema.json");

// strict:false so ajv accepts the schema's `additionalProperties:false` alongside
// `allOf`/`$ref` (a strict-mode warning), matching the lenient Rust/Python validators.
const ajv = new Ajv2020({ strict: false });
const validate = ajv.compile(JSON.parse(readFileSync(schemaPath, "utf8")));

interface Fixture {
  name: string;
  input: unknown;
  normalized?: unknown;
  error?: string;
}

// Parse fixtures losslessly so integers beyond 2^53 become bigint and compare
// equal to the implementation's (also bigint) output.
const fixtures: Fixture[] = [];
for (const file of readdirSync(corpusDir).filter((f) => f.endsWith(".json")).sort()) {
  for (const c of parseJson(readFileSync(join(corpusDir, file), "utf8")) as Fixture[]) {
    fixtures.push(c);
  }
}

test("corpus is populated", () => {
  assert.ok(fixtures.length >= 15, `expected a populated corpus, found ${fixtures.length}`);
});

for (const fixture of fixtures) {
  test(`corpus: ${fixture.name}`, () => {
    const input = stringifyJson(fixture.input); // re-serialize (bigint -> bare digits)
    if (fixture.error !== undefined) {
      // Error inputs may be intentionally schema-invalid; not schema-checked.
      try {
        normalize(parse(input));
        assert.fail(`${fixture.name}: expected an error`);
      } catch (e) {
        assert.ok(e instanceof NdselError, `${fixture.name}: ${String(e)}`);
        assert.equal((e as NdselError).reason, fixture.error);
      }
    } else {
      // ajv validates the JSON structure (numbers); the bigint comparison is done
      // against the implementation's output below.
      assert.ok(validate(JSON.parse(input)), `${fixture.name}: input fails schema`);
      assert.deepStrictEqual(normalize(parse(input)), fixture.normalized);
    }
  });
}
