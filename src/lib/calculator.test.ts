import {
  evaluateMathExpression,
  formatCalculatorNumber,
  matchCurrencyConversion,
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
  check(() => assertCalcValue("2 ^ 3 ^ 2", "512")); // right-associative: 2^(3^2) = 512

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

  // 6. Currency conversions
  check(() => {
    const conv = matchCurrencyConversion("25 EUR to USD");
    if (!conv) {
      throw new Error("Expected currency conversion for '25 EUR to USD'");
    }
    assertEqual(conv.value, "$27.00", "Currency formatted value");
    assertEqual(conv.raw, "27.00", "Currency raw value");
    assertEqual(conv.expression, "25 EUR", "Currency expression");
  });
  check(() => {
    const conv = evaluateMathExpression("10,5 eur in usd");
    if (!conv) {
      throw new Error("Expected currency conversion for '10,5 eur in usd'");
    }
    assertEqual(conv.raw, "11.34", "Comma decimal currency raw value");
  });

  // 7. Plain numbers, constants alone, and non-math queries should return null
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

  // 8. Incomplete, malformed, and unsafe inputs must return null without throwing
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

  // 9. Deep nesting guard (> MAX_PARSE_DEPTH)
  check(() => {
    const deeplyNested = "(".repeat(80) + "1 + 1" + ")".repeat(80);
    assertCalcNull(deeplyNested);
  });

  // 10. Section builder integration
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
    assertEqual(formatCalculatorNumber(-0), "0", "Formats -0 as 0");
  });

  return passed;
}

const totalPassed = runCalculatorTests();
console.log(`Calculator test suite passed (${totalPassed} assertions).`);
