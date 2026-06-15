import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import { NdsqError, normalize, parse } from "../src/index.ts";

const repoRoot = join(import.meta.dirname, "..", "..");
const corpusDir = join(repoRoot, "conformance");
const schemaPath = join(repoRoot, "schema", "ndsq.schema.json");

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

const fixtures: Fixture[] = [];
for (const file of readdirSync(corpusDir).filter((f) => f.endsWith(".json")).sort()) {
  for (const c of JSON.parse(readFileSync(join(corpusDir, file), "utf8")) as Fixture[]) {
    fixtures.push(c);
  }
}

test("corpus is populated", () => {
  assert.ok(fixtures.length >= 15, `expected a populated corpus, found ${fixtures.length}`);
});

for (const fixture of fixtures) {
  test(`corpus: ${fixture.name}`, () => {
    const input = JSON.stringify(fixture.input);
    if (fixture.error !== undefined) {
      // Error inputs may be intentionally schema-invalid; not schema-checked.
      try {
        normalize(parse(input));
        assert.fail(`${fixture.name}: expected an error`);
      } catch (e) {
        assert.ok(e instanceof NdsqError, `${fixture.name}: ${String(e)}`);
        assert.equal((e as NdsqError).reason, fixture.error);
      }
    } else {
      assert.ok(validate(fixture.input), `${fixture.name}: input fails schema`);
      assert.deepStrictEqual(normalize(parse(input)), fixture.normalized);
    }
  });
}
