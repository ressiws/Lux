import { resolvePrefix } from "./prefix";

function assertEqual<T>(actual: T, expected: T, message?: string): void {
  if (actual !== expected) {
    throw new Error(
      `${message ? message + ": " : ""}expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
    );
  }
}

const assert = {
  equal: assertEqual,
};

console.log("Running Flow Launcher prefix tests...");

// 1. Basic prefix stripping
{
  const res = resolvePrefix("et Visual Studios", true, ["et"]);
  assert.equal(res.hasPrefix, true);
  assert.equal(res.prefix, "et");
  assert.equal(res.effectiveQuery, "Visual Studios");
}

// 2. Case-insensitive matching
{
  const res = resolvePrefix("ET visual studio", true, ["et"]);
  assert.equal(res.hasPrefix, true);
  assert.equal(res.prefix, "et");
  assert.equal(res.effectiveQuery, "visual studio");
}

// 3. Leading whitespace
{
  const res = resolvePrefix("   et   notepad", true, ["et"]);
  assert.equal(res.hasPrefix, true);
  assert.equal(res.prefix, "et");
  assert.equal(res.effectiveQuery, "notepad");
}

// 4. Do NOT strip when part of a word
{
  const res = resolvePrefix("ethernet settings", true, ["et"]);
  assert.equal(res.hasPrefix, false);
  assert.equal(res.prefix, null);
  assert.equal(res.effectiveQuery, "ethernet settings");
}

// 5. Do NOT strip from the middle or end of query
{
  const res = resolvePrefix("open internet", true, ["et"]);
  assert.equal(res.hasPrefix, false);
  assert.equal(res.prefix, null);
  assert.equal(res.effectiveQuery, "open internet");
}

// 6. Multiple custom prefixes
{
  const prefixes = ["et", "wiki", "gh"];
  const res1 = resolvePrefix("wiki Rust (programming language)", true, prefixes);
  assert.equal(res1.hasPrefix, true);
  assert.equal(res1.prefix, "wiki");
  assert.equal(res1.effectiveQuery, "Rust (programming language)");

  const res2 = resolvePrefix("gh lux", true, prefixes);
  assert.equal(res2.hasPrefix, true);
  assert.equal(res2.prefix, "gh");
  assert.equal(res2.effectiveQuery, "lux");
}

// 7. Disabled compatibility
{
  const res = resolvePrefix("et Visual Studios", false, ["et"]);
  assert.equal(res.hasPrefix, false);
  assert.equal(res.prefix, null);
  assert.equal(res.effectiveQuery, "et Visual Studios");
}

// 8. Standalone prefix without remainder
{
  const res = resolvePrefix("et", true, ["et"]);
  assert.equal(res.hasPrefix, true);
  assert.equal(res.prefix, "et");
  assert.equal(res.effectiveQuery, "");
}

// 9. Calculator with prefix
{
  const res = resolvePrefix("et 42 * 2", true, ["et"]);
  assert.equal(res.hasPrefix, true);
  assert.equal(res.effectiveQuery, "42 * 2");
}

console.log("Flow Launcher prefix tests passed (9 test scenarios).");
