import { getColorLuminance } from "./color";

function assertEqual<T>(actual: T, expected: T, message: string): void {
  if (actual !== expected) {
    throw new Error(
      `${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
    );
  }
}

function assert(condition: boolean, message: string): void {
  if (!condition) {
    throw new Error(`Assertion failed: ${message}`);
  }
}

console.log("Running color test suite...");

// 1. Full Hex 6 digits
const whiteLum = getColorLuminance("#FFFFFF");
assert(whiteLum > 200, "White should have high luminance");

const blackLum = getColorLuminance("#000000");
assertEqual(blackLum, 0, "Black should have 0 luminance");

const brandLum = getColorLuminance("#3B3550");
assert(brandLum < 100, "#3B3550 should have low luminance");

// 2. Short Hex 3 digits
const shortWhite = getColorLuminance("#FFF");
assert(shortWhite > 200, "#FFF should have high luminance");

const shortBlack = getColorLuminance("#000");
assertEqual(shortBlack, 0, "#000 should have 0 luminance");

// 3. Hex 4 digits (RGBA)
const alphaWhiteFull = getColorLuminance("#FFFF");
assert(alphaWhiteFull > 200, "#FFFF with 100% alpha should have high luminance");

const alphaWhiteTransparent = getColorLuminance("#FFF1");
assertEqual(
  alphaWhiteTransparent,
  0,
  "#FFF1 with low alpha (<0.4) should default to 0 luminance for dark surface",
);

// 4. Hex 8 digits (RRGGBBAA)
const hex8Full = getColorLuminance("#FFFFFFEE");
assert(hex8Full > 200, "#FFFFFFEE should have high luminance");

const hex8LowAlpha = getColorLuminance("#FFFFFF20");
assertEqual(
  hex8LowAlpha,
  0,
  "#FFFFFF20 with low alpha should default to 0 luminance for dark surface",
);

// 5. Case and whitespace variations
assertEqual(
  getColorLuminance("  #ffffff  "),
  whiteLum,
  "Whitespace should be trimmed",
);
assertEqual(
  getColorLuminance("#ffffff"),
  whiteLum,
  "Case variation should match",
);

console.log("Color test suite passed successfully.");
