import {
  evaluateMathExpression,
  formatCalculatorNumber,
  getIntegerQuickAnswerDetails,
  matchCurrencyConversion,
  matchTimeZoneConversion,
  matchDateDuration,
  matchUnitConversion,
  matchPracticalCalculation,
  matchColorConversion,
  matchPasswordGeneration,
  setCurrencyCacheForTesting,
  getCurrencyCache,
} from "./calculator";
import { buildSections } from "./mock-data";

function assertEqual<T>(actual: T, expected: T, message: string): void {
  if (actual !== expected) {
    throw new Error(
      `${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
    );
  }
}

function assertCalcValue(query: string, expectedValue: string): void {
  const result = evaluateMathExpression(query);
  if (!result) {
    throw new Error(`Expected "${query}" to evaluate to "${expectedValue}", got null`);
  }
  assertEqual(result.value, expectedValue, `Value for "${query}"`);
  assertEqual(result.raw, expectedValue, `Raw value for "${query}"`);
}

function assertCalcNull(query: string): void {
  const result = evaluateMathExpression(query);
  if (result !== null) {
    throw new Error(
      `Expected "${query}" to return null, got ${JSON.stringify(result)}`,
    );
  }
}

export function runCalculatorTests(): number {
  let passed = 0;

  const check = (fn: () => void): void => {
    fn();
    passed += 1;
  };

  // 1. Basic arithmetic and operator precedence
  check(() => assertCalcValue("2 + 2", "4"));
  check(() => assertCalcValue("10 - 3 - 2", "5"));
  check(() => assertCalcValue("6 * 7", "42"));
  check(() => assertCalcValue("20 / 5 * 2", "8"));
  check(() => assertCalcValue("2 + 3 * 4", "14"));
  check(() => assertCalcValue("(2 + 3) * 4", "20"));
  check(() => assertCalcValue("17 % 5", "2"));
  check(() => assertCalcValue("2 ^ 8", "256"));
  check(() => assertCalcValue("2 ** 10", "1024"));
  check(() => assertCalcValue("2 ^ 3 ^ 2", "512"));

  // 2. Unicode operators and trailing equals
  check(() => assertCalcValue("12 × 3", "36"));
  check(() => assertCalcValue("15 ÷ 4", "3.75"));
  check(() => assertCalcValue("10 − 4", "6"));
  check(() => assertCalcValue("7 * 8 =", "56"));
  check(() => assertCalcValue("7 * 8 =?", "56"));

  // 3. Decimals (dot, comma, leading dot, IEEE 754 precision)
  check(() => assertCalcValue("0.1 + 0.2", "0.3"));
  check(() => assertCalcValue("3,14 + 2,86", "6"));
  check(() => assertCalcValue("12,5 * 2", "25"));
  check(() => assertCalcValue(".5 + .25", "0.75"));
  check(() => assertCalcValue("(128 * 1.19) / 3", "50.77333333"));

  // 4. Parentheses, unary operators, and implicit multiplication
  check(() => assertCalcValue("-5 + 12", "7"));
  check(() => assertCalcValue("--5 + 1", "6"));
  check(() => assertCalcValue("2 ^ -3", "0.125"));
  check(() => assertCalcValue("-2 ^ 2", "-4"));
  check(() => assertCalcValue("(-2) ^ 2", "4"));
  check(() => assertCalcValue("-(3 + 4)", "-7"));
  check(() => assertCalcValue("2(3 + 4)", "14"));
  check(() => assertCalcValue("(1 + 2)(3 + 4)", "21"));
  check(() => assertCalcValue("((2 + (3 * (4 - 1))))", "11"));
  check(() => assertCalcValue("-0 * 5", "0"));

  // 5. Mathematical functions and constants
  check(() => assertCalcValue("sqrt(144)", "12"));
  check(() => assertCalcValue("cbrt(27)", "3"));
  check(() => assertCalcValue("abs(-42.5)", "42.5"));
  check(() => assertCalcValue("round(3.6)", "4"));
  check(() => assertCalcValue("floor(3.9)", "3"));
  check(() => assertCalcValue("ceil(3.1)", "4"));
  check(() => assertCalcValue("trunc(-3.9)", "-3"));
  check(() => assertCalcValue("log(1000)", "3"));
  check(() => assertCalcValue("log10(100)", "2"));
  check(() => assertCalcValue("ln(e)", "1"));
  check(() => assertCalcValue("sin(0)", "0"));
  check(() => assertCalcValue("cos(0)", "1"));
  check(() => assertCalcValue("sin(pi / 2)", "1"));
  check(() => assertCalcValue("cos(π)", "-1"));
  check(() => assertCalcValue("2 * pi", "6.28318531"));
  check(() => assertCalcValue("2pi", "6.28318531"));
  check(() => assertCalcValue("3sqrt(16)", "12"));

  // 6. Currency conversions and caching
  check(() => {
    const conv = matchCurrencyConversion("25 EUR to USD");
    if (!conv) {
      throw new Error("Expected currency conversion for '25 EUR to USD'");
    }
    assertEqual(conv.value, "$27.00", "Currency formatted value");
    assertEqual(conv.raw, "27.00", "Currency raw value");
    assertEqual(conv.expression, "25 EUR", "Currency expression");
    assertEqual(conv.categoryLabel, "Currency", "Currency category label");
  });
  check(() => {
    const conv = evaluateMathExpression("10,5 eur in usd");
    if (!conv) {
      throw new Error("Expected currency conversion for '10,5 eur in usd'");
    }
    assertEqual(conv.raw, "11.34", "Comma decimal currency raw value");
  });
  check(() => {
    // Testing stale currency notice and custom cache
    const testCache = {
      rates: { USD: 1, EUR: 0.9, JPY: 150 },
      lastUpdated: "2026-10-18T00:00:00Z",
      timestamp: Date.now() - 72 * 3600 * 1000, // 3 days ago (stale)
      provider: "open.er-api.com (ECB)",
    };
    setCurrencyCacheForTesting(testCache);
    const conv = matchCurrencyConversion("100 EUR to USD");
    if (!conv) throw new Error("Expected conversion with test cache");
    assertEqual(conv.raw, "111.11", "Custom cache conversion");
    if (!conv.label.includes("cached")) {
      throw new Error("Expected stale cached note in currency label");
    }
    // Restore default cache
    setCurrencyCacheForTesting({
      rates: { USD: 1, EUR: 0.926, GBP: 0.79, BRL: 5.05, JPY: 151.2 },
      lastUpdated: "2026-10-21",
      timestamp: Date.now(),
      provider: "open.er-api.com (ECB)",
    });
  });

  // 7. Time zones and time conversion
  check(() => {
    const res = matchTimeZoneConversion("time in Tokyo");
    if (!res) throw new Error("Expected time in Tokyo");
    assertEqual(res.categoryLabel, "Time Zone", "Time zone category label");
    if (!res.expression.includes("Tokyo")) throw new Error("Expression should mention Tokyo");
  });
  check(() => {
    const res = matchTimeZoneConversion("time difference Lisbon London");
    if (!res) throw new Error("Expected time difference Lisbon London");
    assertEqual(res.categoryLabel, "Time Zone", "Time zone difference category");
  });
  check(() => {
    const res = matchTimeZoneConversion("9:30 Lisbon to New York");
    if (!res) throw new Error("Expected conversion for '9:30 Lisbon to New York'");
    assertEqual(res.categoryLabel, "Time Zone", "Time conversion category");
    if (!res.value.includes("AM") && !res.value.includes("PM")) {
      throw new Error("Expected formatted time result");
    }
  });
  check(() => {
    // Midnight crossing test: 11 PM London to Tokyo (Tokyo is +8/9 hours ahead) -> next day (+1 day)
    const res = matchTimeZoneConversion("11 PM London to Tokyo");
    if (!res) throw new Error("Expected conversion for '11 PM London to Tokyo'");
    if (!res.value.includes("(+1 day)")) {
      throw new Error(`Expected midnight crossing note (+1 day), got: ${res.value}`);
    }
  });
  check(() => {
    const res = evaluateMathExpression("10 PM here to London");
    if (!res) throw new Error("Expected '10 PM here to London' via evaluateMathExpression");
    assertEqual(res.categoryLabel, "Time Zone", "Time zone category");
  });

  // 8. Dates and durations
  check(() => {
    const ref = new Date(2026, 9, 21); // 21 Oct 2026
    const res = matchDateDuration("90 days from today", ref);
    if (!res) throw new Error("Expected 90 days from today");
    assertEqual(res.categoryLabel, "Date", "Date category");
    if (!res.value.includes("19") || !res.value.includes("January")) {
      throw new Error(`Expected January 19 2027, got: ${res.value}`);
    }
  });
  check(() => {
    const ref = new Date(2026, 9, 21); // 21 Oct 2026
    const res = matchDateDuration("days until Christmas", ref);
    if (!res) throw new Error("Expected days until Christmas");
    assertEqual(res.value, "65 days", "Days until Christmas from 21 Oct");
  });
  check(() => {
    const ref = new Date(2026, 9, 21);
    const res = matchDateDuration("days between 12 Oct and 21 Oct", ref);
    if (!res) throw new Error("Expected days between 12 Oct and 21 Oct");
    assertEqual(res.value, "9 days", "Calendar days between 12 Oct and 21 Oct");
  });
  check(() => {
    // Leap year boundary: 2028 is a leap year (Feb has 29 days)
    const ref = new Date(2028, 1, 28); // 28 Feb 2028
    const res = matchDateDuration("1 days from today", ref);
    if (!res) throw new Error("Expected leap year day addition");
    if (!res.value.includes("29") || !res.value.includes("February")) {
      throw new Error(`Expected Feb 29 2028, got: ${res.value}`);
    }
  });

  // 9. Unit conversions
  check(() => {
    // Distance
    const res = matchUnitConversion("100 km to miles");
    if (!res) throw new Error("Expected 100 km to miles");
    assertEqual(res.value, "62.137119 miles", "100 km in miles");
    assertEqual(res.categoryLabel, "Unit", "Unit category");
  });
  check(() => {
    // Temperature: Celsius to Fahrenheit and Kelvin
    const resF = matchUnitConversion("100 C to F");
    if (!resF) throw new Error("Expected 100 C to F");
    assertEqual(resF.value, "212 °F", "100 C in F");

    const resK = matchUnitConversion("0 C to K");
    if (!resK) throw new Error("Expected 0 C to K");
    assertEqual(resK.value, "273.15 K", "0 C in K");
  });
  check(() => {
    // Weight
    const res = matchUnitConversion("50 kg to lbs");
    if (!res) throw new Error("Expected 50 kg to lbs");
    assertEqual(res.value, "110.231131 lbs", "50 kg in lbs");
  });
  check(() => {
    // Digital storage: verify distinction between Decimal SI (GB) and Binary IEC (GiB)
    const resSiToIec = matchUnitConversion("1 GB to GiB");
    if (!resSiToIec) throw new Error("Expected 1 GB to GiB");
    assertEqual(resSiToIec.value, "0.931323 GiB", "1 GB to GiB");

    const resIecToSi = matchUnitConversion("1 GiB to MB");
    if (!resIecToSi) throw new Error("Expected 1 GiB to MB");
    assertEqual(resIecToSi.value, "1,073.741824 MB", "1 GiB to MB");
  });

  // 10. Practical calculations
  check(() => {
    // Percentage
    const res = matchPracticalCalculation("15% of 240");
    if (!res) throw new Error("Expected 15% of 240");
    assertEqual(res.value, "36", "15% of 240");
  });
  check(() => {
    // VAT addition
    const res = matchPracticalCalculation("VAT 23% on 100");
    if (!res) throw new Error("Expected VAT 23% on 100");
    assertEqual(res.value, "123.00", "Total with 23% VAT on 100");
    assertEqual(res.categoryLabel, "Tax", "Tax category");
  });
  check(() => {
    // VAT removal / extraction
    const res = matchPracticalCalculation("remove 23% VAT from 123");
    if (!res) throw new Error("Expected remove 23% VAT from 123");
    assertEqual(res.value, "100.00", "Net after removing 23% VAT from 123");
    assertEqual(res.categoryLabel, "Tax", "Tax category");
  });
  check(() => {
    // Bill split
    const res = matchPracticalCalculation("split 87.50 between 4");
    if (!res) throw new Error("Expected split 87.50 between 4");
    assertEqual(res.value, "21.88 each", "Split 87.50 between 4");
    assertEqual(res.raw, "21.88", "Raw split amount");
  });

  // 11. Color conversions
  check(() => {
    // Hex to RGB and HSL
    const res = matchColorConversion("#798F78 to RGB");
    if (!res) throw new Error("Expected #798F78 to RGB");
    assertEqual(res.value, "rgb(121, 143, 120)", "Hex to RGB value");
    assertEqual(res.colorPreview, "#798f78", "Color preview hex");
    assertEqual(res.categoryLabel, "Color", "Color category");
  });
  check(() => {
    const res = matchColorConversion("#798F78 to HSL");
    if (!res) throw new Error("Expected #798F78 to HSL");
    assertEqual(res.value, "hsl(117, 9%, 52%)", "Hex to HSL value");
  });
  check(() => {
    // RGB to HEX
    const res = matchColorConversion("rgb(121, 143, 120) to HEX");
    if (!res) throw new Error("Expected rgb(121, 143, 120) to HEX");
    assertEqual(res.value, "#798f78", "RGB to HEX value");
  });
  check(() => {
    // HSL to HEX
    const res = matchColorConversion("hsl(117, 9%, 52%) to HEX");
    if (!res) throw new Error("Expected hsl(117, 9%, 52%) to HEX");
    assertEqual(res.value, "#7b907a", "HSL to HEX value");
  });
  check(() => {
    // Invalid color out of range
    const res = matchColorConversion("rgb(300, 10, 10)");
    if (res !== null) throw new Error("Expected null for out of range RGB");
  });

  // 12. Secure password generation
  check(() => {
    const res = matchPasswordGeneration("generate password 24");
    if (!res) throw new Error("Expected generate password 24");
    assertEqual(res.value.length, 24, "Generated password length 24");
    assertEqual(res.categoryLabel, "Password", "Password category");
  });
  check(() => {
    const res = matchPasswordGeneration("generate password");
    if (!res) throw new Error("Expected default generate password");
    assertEqual(res.value.length, 16, "Default password length 16");
  });
  check(() => {
    // Bounds: < 8 or > 128
    const resShort = matchPasswordGeneration("generate password 4");
    if (!resShort || !resShort.value.includes("between 8 and 128")) {
      throw new Error("Expected rejection for password length 4");
    }
  });
  check(() => {
    // Should NOT trigger for non-generation queries containing "password"
    assertCalcNull("password manager");
    assertCalcNull("my password");
    assertCalcNull("change password");
  });

  // 13. Plain queries, app names, files should NOT be captured as quick answers
  check(() => assertCalcNull(""));
  check(() => assertCalcNull("   "));
  check(() => assertCalcNull("42"));
  check(() => assertCalcNull("-42"));
  check(() => assertCalcNull("+3.14"));
  check(() => assertCalcNull("(42)"));
  check(() => assertCalcNull("pi"));
  check(() => assertCalcNull("e"));
  check(() => assertCalcNull("firefox"));
  check(() => assertCalcNull("wedding-selects-0412.jpg"));
  check(() => assertCalcNull("IMG_4821.jpg"));
  check(() => assertCalcNull("London"));

  // 14. Incomplete, malformed, and unsafe inputs must return null without throwing
  check(() => assertCalcNull("2 +"));
  check(() => assertCalcNull("* 5"));
  check(() => assertCalcNull("(2 + 3"));
  check(() => assertCalcNull("2 + 3)"));
  check(() => assertCalcNull("()"));
  check(() => assertCalcNull("sqrt()"));
  check(() => assertCalcNull("sqrt(9"));
  check(() => assertCalcNull("sqrt 9"));
  check(() => assertCalcNull("1.2.3 + 1"));
  check(() => assertCalcNull("1,2,3 + 1"));
  check(() => assertCalcNull("2 3"));
  check(() => assertCalcNull("1 / 0"));
  check(() => assertCalcNull("10 % 0"));
  check(() => assertCalcNull("sqrt(-1)"));
  check(() => assertCalcNull("log(0)"));
  check(() => assertCalcNull("log(-10)"));
  check(() => assertCalcNull("ln(0)"));
  check(() => assertCalcNull("asin(2)"));
  check(() => assertCalcNull("acos(-2)"));
  check(() => assertCalcNull("10 ^ 1000"));
  check(() => assertCalcNull("alert(1)"));
  check(() => assertCalcNull("Function('return 1')()"));
  check(() => assertCalcNull("process.exit(1)"));
  check(() => assertCalcNull("25 ABC to XYZ"));

  // 15. Deep nesting guard (> MAX_PARSE_DEPTH)
  check(() => {
    const deeplyNested = "(".repeat(80) + "1 + 1" + ")".repeat(80);
    assertCalcNull(deeplyNested);
  });

  // 16. Section builder integration
  check(() => {
    const sections = buildSections("(128 * 1.19) / 3");
    assertEqual(sections[0]?.category, "calc", "First section should be calc");
    const calcItem = sections[0]?.items[0];
    assertEqual(calcItem?.name, "50.77333333", "Calc item display value");
    assertEqual(calcItem?.primaryLabel, "Copy Result", "Calc item primary action");
    assertEqual(
      calcItem?.actions[1]?.label,
      "Copy Expression",
      "Calc item secondary action",
    );
  });
  check(() => {
    const sections = buildSections("generate password 24");
    assertEqual(sections[0]?.category, "calc", "Password section should be calc");
    const item = sections[0]?.items[0];
    assertEqual(item?.primaryLabel, "Copy Password", "Password item primary action");
    assertEqual(item?.rightLabel, "Password", "Password item right label");
  });
  check(() => {
    const sections = buildSections("#798F78 to RGB");
    const item = sections[0]?.items[0];
    assertEqual(item?.color, "#798f78", "Color item swatch");
    assertEqual(item?.rightLabel, "Color", "Color item right label");
  });

  // 17. Documented feature guide query verification
  const documentedQueries = [
    "1 + 1",
    "25 EUR to USD",
    "10 PM here to London",
    "100 km to miles",
    "VAT 23% on 100",
    "#798F78 to RGB",
    "generate password 24",
    "(128 * 1.19) / 3",
    "sqrt(144) + 2^8",
    "15% of 240",
    "remove 23% VAT from 123",
    "split 87.50 between 4",
    "time in Tokyo",
    "time difference Lisbon London",
    "90 days from today",
    "days until Christmas",
    "days between 12 Oct and 21 Oct",
    "100 C to F",
    "1 GiB to MB",
  ];

  for (const q of documentedQueries) {
    check(() => {
      const res = evaluateMathExpression(q);
      if (!res) {
        throw new Error(`Documented query example failed to produce a quick answer: "${q}"`);
      }
    });
  }

  check(() => {
    assertEqual(formatCalculatorNumber(-0), "0", "Formats -0 as 0");
  });

  // 18. Integer quick answer details
  check(() => {
    const result = evaluateMathExpression("32 / 16");
    if (!result) {
      throw new Error("Expected integer expression to evaluate");
    }
    const details = getIntegerQuickAnswerDetails(result);
    assertEqual(details[0]?.label, "Hex", "Integer hex label");
    assertEqual(details[0]?.value, "0x2", "Integer hex value");
    assertEqual(details[1]?.label, "Binary", "Integer binary label");
    assertEqual(details[1]?.value, "10", "Integer binary value");
  });
  check(() => {
    const details = getIntegerQuickAnswerDetails({
      value: "-5",
      expression: "-5",
      label: "Calculator",
      raw: "-5",
      categoryLabel: "Calculation",
    });
    assertEqual(details[0]?.value, "-0x5", "Negative integer hex value");
    assertEqual(details[1]?.value, "-101", "Negative integer binary value");
  });
  check(() => {
    const result = evaluateMathExpression("10 / 4");
    if (!result) {
      throw new Error("Expected decimal expression to evaluate");
    }
    assertEqual(
      getIntegerQuickAnswerDetails(result).length,
      0,
      "Decimal results have no integer details",
    );
  });
  check(() => {
    assertEqual(
      getIntegerQuickAnswerDetails({
        value: "8",
        expression: "8",
        label: "Unit conversion",
        raw: "8",
        categoryLabel: "Conversion",
      }).length,
      0,
      "Non-calculator results have no integer details",
    );
    assertEqual(
      getIntegerQuickAnswerDetails({
        value: "9007199254740992",
        expression: "9007199254740992",
        label: "Calculator",
        raw: "9007199254740992",
        categoryLabel: "Calculator",
      }).length,
      0,
      "Unsafe integers have no integer details",
    );
  });

  return passed;
}

const totalPassed = runCalculatorTests();
console.log(`Calculator test suite passed (${totalPassed} assertions).`);
