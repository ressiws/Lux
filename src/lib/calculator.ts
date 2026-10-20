import type { CalcPreview } from "./types";

const CURRENCY_RATES: Readonly<Record<string, number>> = {
  USD: 1,
  EUR: 0.926,
  GBP: 0.79,
  BRL: 5.05,
  JPY: 151.2,
};

type BinaryOperator = "+" | "-" | "*" | "/" | "%" | "^";

type Token =
  | { readonly kind: "number"; readonly value: number }
  | { readonly kind: "op"; readonly op: BinaryOperator }
  | { readonly kind: "lparen" }
  | { readonly kind: "rparen" }
  | { readonly kind: "ident"; readonly name: string };

const CONSTANTS: Readonly<Record<string, number>> = {
  pi: Math.PI,
  π: Math.PI,
  e: Math.E,
};

const FUNCTIONS: Readonly<Record<string, (arg: number) => number | null>> = {
  sqrt: (x) => (x < 0 ? null : Math.sqrt(x)),
  cbrt: (x) => Math.cbrt(x),
  sin: (x) => Math.sin(x),
  cos: (x) => Math.cos(x),
  tan: (x) => Math.tan(x),
  asin: (x) => (x < -1 || x > 1 ? null : Math.asin(x)),
  acos: (x) => (x < -1 || x > 1 ? null : Math.acos(x)),
  atan: (x) => Math.atan(x),
  log: (x) => (x <= 0 ? null : Math.log10(x)),
  log10: (x) => (x <= 0 ? null : Math.log10(x)),
  ln: (x) => (x <= 0 ? null : Math.log(x)),
  abs: (x) => Math.abs(x),
  round: (x) => Math.round(x),
  floor: (x) => Math.floor(x),
  ceil: (x) => Math.ceil(x),
  trunc: (x) => Math.trunc(x),
};

const MAX_PARSE_DEPTH = 64;

function isDigit(ch: string | undefined): boolean {
  return ch !== undefined && ch >= "0" && ch <= "9";
}

function isIdentifierStart(ch: string | undefined): boolean {
  if (ch === undefined) {
    return false;
  }
  return (
    (ch >= "a" && ch <= "z") ||
    (ch >= "A" && ch <= "Z") ||
    ch === "π"
  );
}

function isIdentifierPart(ch: string | undefined): boolean {
  return isIdentifierStart(ch) || isDigit(ch);
}

function isConstantName(name: string): boolean {
  return Object.prototype.hasOwnProperty.call(CONSTANTS, name);
}

function isFunctionName(name: string): boolean {
  return Object.prototype.hasOwnProperty.call(FUNCTIONS, name);
}

function shouldInsertImplicitMultiply(prev: Token, next: Token): boolean {
  if (prev.kind === "number") {
    return next.kind === "lparen" || next.kind === "ident";
  }
  if (prev.kind === "rparen") {
    return (
      next.kind === "lparen" ||
      next.kind === "number" ||
      next.kind === "ident"
    );
  }
  if (prev.kind === "ident" && isConstantName(prev.name)) {
    return next.kind === "lparen" || next.kind === "ident";
  }
  return false;
}

function tokenize(input: string): Token[] | null {
  const rawTokens: Token[] = [];
  let index = 0;

  while (index < input.length) {
    const ch = input[index];

    if (ch === " " || ch === "\t" || ch === "\n" || ch === "\r") {
      index += 1;
      continue;
    }

    if (
      isDigit(ch) ||
      ((ch === "." || ch === ",") && isDigit(input[index + 1]))
    ) {
      let numText = "";
      let hasDecimal = false;

      while (index < input.length) {
        const cur = input[index];
        if (isDigit(cur)) {
          numText += cur;
          index += 1;
        } else if (cur === "." || cur === ",") {
          if (hasDecimal) {
            return null;
          }
          hasDecimal = true;
          numText += ".";
          index += 1;
        } else {
          break;
        }
      }

      if (numText === "." || numText === "") {
        return null;
      }

      const parsed = Number(numText);
      if (!Number.isFinite(parsed)) {
        return null;
      }

      rawTokens.push({ kind: "number", value: parsed });
      continue;
    }

    if (ch === "(") {
      rawTokens.push({ kind: "lparen" });
      index += 1;
      continue;
    }

    if (ch === ")") {
      rawTokens.push({ kind: "rparen" });
      index += 1;
      continue;
    }

    if (ch === "*" && input[index + 1] === "*") {
      rawTokens.push({ kind: "op", op: "^" });
      index += 2;
      continue;
    }

    if (ch === "+" || ch === "-" || ch === "*" || ch === "/" || ch === "%" || ch === "^") {
      rawTokens.push({ kind: "op", op: ch });
      index += 1;
      continue;
    }

    if (ch === "×" || ch === "⋅") {
      rawTokens.push({ kind: "op", op: "*" });
      index += 1;
      continue;
    }

    if (ch === "÷") {
      rawTokens.push({ kind: "op", op: "/" });
      index += 1;
      continue;
    }

    if (ch === "−" || ch === "–") {
      rawTokens.push({ kind: "op", op: "-" });
      index += 1;
      continue;
    }

    if (isIdentifierStart(ch)) {
      let ident = "";
      if (ch === "π") {
        ident = "π";
        index += 1;
      } else {
        while (index < input.length && isIdentifierPart(input[index])) {
          ident += input[index];
          index += 1;
        }
      }
      const lowered = ident.toLowerCase();
      if (!isConstantName(lowered) && !isFunctionName(lowered)) {
        return null;
      }
      rawTokens.push({ kind: "ident", name: lowered });
      continue;
    }

    return null;
  }

  if (rawTokens.length === 0) {
    return null;
  }

  const tokens: Token[] = [];
  for (const token of rawTokens) {
    const prev = tokens[tokens.length - 1];
    if (prev && shouldInsertImplicitMultiply(prev, token)) {
      tokens.push({ kind: "op", op: "*" });
    }
    tokens.push(token);
  }

  return tokens;
}

class ExpressionParser {
  private readonly tokens: readonly Token[];
  private pos = 0;
  private depth = 0;
  public operationCount = 0;

  constructor(tokens: readonly Token[]) {
    this.tokens = tokens;
  }

  public parse(): number | null {
    const value = this.parseAdditive();
    if (value === null || this.pos !== this.tokens.length) {
      return null;
    }
    if (!Number.isFinite(value) || Number.isNaN(value)) {
      return null;
    }
    return Object.is(value, -0) ? 0 : value;
  }

  private peek(): Token | undefined {
    return this.tokens[this.pos];
  }

  private consume(): Token | undefined {
    const token = this.tokens[this.pos];
    this.pos += 1;
    return token;
  }

  private parseAdditive(): number | null {
    let left = this.parseMultiplicative();
    if (left === null) {
      return null;
    }

    while (true) {
      const next = this.peek();
      if (next?.kind === "op" && (next.op === "+" || next.op === "-")) {
        this.consume();
        const right = this.parseMultiplicative();
        if (right === null) {
          return null;
        }
        this.operationCount += 1;
        left = next.op === "+" ? left + right : left - right;
        if (!Number.isFinite(left)) {
          return null;
        }
      } else {
        break;
      }
    }

    return left;
  }

  private parseMultiplicative(): number | null {
    let left = this.parseUnary();
    if (left === null) {
      return null;
    }

    while (true) {
      const next = this.peek();
      if (
        next?.kind === "op" &&
        (next.op === "*" || next.op === "/" || next.op === "%")
      ) {
        this.consume();
        const right = this.parseUnary();
        if (right === null) {
          return null;
        }
        this.operationCount += 1;
        if (next.op === "*") {
          left *= right;
        } else {
          if (right === 0) {
            return null;
          }
          left = next.op === "/" ? left / right : left % right;
        }
        if (!Number.isFinite(left)) {
          return null;
        }
      } else {
        break;
      }
    }

    return left;
  }

  private parseUnary(): number | null {
    if (this.depth >= MAX_PARSE_DEPTH) {
      return null;
    }

    const next = this.peek();
    if (next?.kind === "op" && (next.op === "+" || next.op === "-")) {
      this.consume();
      const targetAfterSign = this.peek();
      if (targetAfterSign && targetAfterSign.kind !== "number") {
        this.operationCount += 1;
      }
      this.depth += 1;
      const operand = this.parseUnary();
      this.depth -= 1;
      if (operand === null) {
        return null;
      }
      return next.op === "-" ? -operand : operand;
    }

    return this.parsePower();
  }

  private parsePower(): number | null {
    const base = this.parsePrimary();
    if (base === null) {
      return null;
    }

    const next = this.peek();
    if (next?.kind === "op" && next.op === "^") {
      this.consume();
      this.depth += 1;
      const exponent = this.parseUnary();
      this.depth -= 1;
      if (exponent === null) {
        return null;
      }
      this.operationCount += 1;
      const result = Math.pow(base, exponent);
      if (!Number.isFinite(result) || Number.isNaN(result)) {
        return null;
      }
      return result;
    }

    return base;
  }

  private parsePrimary(): number | null {
    if (this.depth >= MAX_PARSE_DEPTH) {
      return null;
    }

    const token = this.peek();
    if (!token) {
      return null;
    }

    if (token.kind === "number") {
      this.consume();
      return token.value;
    }

    if (token.kind === "lparen") {
      this.consume();
      this.depth += 1;
      const inner = this.parseAdditive();
      this.depth -= 1;
      if (inner === null) {
        return null;
      }
      const closing = this.consume();
      if (closing?.kind !== "rparen") {
        return null;
      }
      return inner;
    }

    if (token.kind === "ident") {
      this.consume();
      if (isConstantName(token.name)) {
        return CONSTANTS[token.name];
      }

      const fn = FUNCTIONS[token.name];
      if (!fn) {
        return null;
      }

      const openParen = this.consume();
      if (openParen?.kind !== "lparen") {
        return null;
      }

      this.depth += 1;
      const arg = this.parseAdditive();
      this.depth -= 1;
      if (arg === null) {
        return null;
      }

      const closeParen = this.consume();
      if (closeParen?.kind !== "rparen") {
        return null;
      }

      this.operationCount += 1;
      const result = fn(arg);
      if (result === null || !Number.isFinite(result) || Number.isNaN(result)) {
        return null;
      }
      return result;
    }

    return null;
  }
}

export function formatCalculatorNumber(value: number): string {
  const normalized = Object.is(value, -0) ? 0 : value;
  return String(Number(normalized.toFixed(8)));
}

export function matchCurrencyConversion(query: string): CalcPreview | null {
  const match = /^([\d.,]+)\s*([a-z]{3})\s+(?:to|in)\s+([a-z]{3})$/i.exec(
    query.trim(),
  );
  if (!match) {
    return null;
  }

  const amountText = match[1].replace(",", ".");
  if ((amountText.match(/\./g) ?? []).length > 1) {
    return null;
  }

  const amount = Number(amountText);
  const from = match[2].toUpperCase();
  const to = match[3].toUpperCase();
  const fromRate = CURRENCY_RATES[from];
  const toRate = CURRENCY_RATES[to];
  if (!Number.isFinite(amount) || !fromRate || !toRate) {
    return null;
  }

  const converted = (amount / fromRate) * toRate;
  return {
    value: new Intl.NumberFormat("en", {
      style: "currency",
      currency: to,
    }).format(converted),
    expression: `${amount} ${from}`,
    label: `${from} to ${to} · rate ${(toRate / fromRate).toFixed(4)}`,
    raw: converted.toFixed(2),
  };
}

export function evaluateMathExpression(rawQuery: string): CalcPreview | null {
  const trimmed = rawQuery.trim();
  if (!trimmed) {
    return null;
  }

  const currencyPreview = matchCurrencyConversion(trimmed);
  if (currencyPreview) {
    return currencyPreview;
  }

  const cleaned = trimmed.replace(/\s*=\s*\??$/, "").trim();
  if (!cleaned) {
    return null;
  }

  const tokens = tokenize(cleaned);
  if (!tokens) {
    return null;
  }

  const parser = new ExpressionParser(tokens);
  const value = parser.parse();
  if (value === null || parser.operationCount === 0) {
    return null;
  }

  const formatted = formatCalculatorNumber(value);
  return {
    value: formatted,
    expression: cleaned,
    label: "Result",
    raw: formatted,
  };
}
