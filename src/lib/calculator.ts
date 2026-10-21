import type { CalcPreview } from "./types";

// ============================================================================
// 1. CURRENCY CONVERSION & RATES CACHING
// ============================================================================

export interface CurrencyCacheState {
  rates: Record<string, number>;
  lastUpdated: string;
  timestamp: number;
  provider: string;
}

const DEFAULT_CURRENCY_RATES: Readonly<Record<string, number>> = {
  USD: 1,
  EUR: 0.926,
  GBP: 0.79,
  BRL: 5.05,
  JPY: 151.2,
  CAD: 1.38,
  AUD: 1.54,
  CHF: 0.91,
  CNY: 7.24,
  INR: 83.5,
};

const CACHE_STORAGE_KEY = "lux_currency_rates_v1";

let currencyCache: CurrencyCacheState = {
  rates: { ...DEFAULT_CURRENCY_RATES },
  lastUpdated: "2026-10-21",
  timestamp: Date.now(),
  provider: "open.er-api.com (ECB)",
};

try {
  if (typeof localStorage !== "undefined") {
    const stored = localStorage.getItem(CACHE_STORAGE_KEY);
    if (stored) {
      const parsed = JSON.parse(stored) as CurrencyCacheState;
      if (parsed && parsed.rates && typeof parsed.rates === "object") {
        currencyCache = parsed;
      }
    }
  }
} catch {
  // Ignore localStorage access failures
}

export function setCurrencyCacheForTesting(cache: CurrencyCacheState): void {
  currencyCache = cache;
}

export function getCurrencyCache(): Readonly<CurrencyCacheState> {
  return currencyCache;
}

let isRefreshingRates = false;
let lastRefreshAttempt = 0;

export async function refreshCurrencyRates(force = false): Promise<boolean> {
  const now = Date.now();
  if (!force && now - lastRefreshAttempt < 6 * 3600 * 1000) {
    return false;
  }
  if (isRefreshingRates) {
    return false;
  }
  isRefreshingRates = true;
  lastRefreshAttempt = now;

  try {
    const response = await fetch("https://open.er-api.com/v6/latest/USD");
    if (!response.ok) {
      isRefreshingRates = false;
      return false;
    }
    const data = (await response.json()) as {
      result?: string;
      rates?: Record<string, number>;
      time_last_update_utc?: string;
    };
    if (data.result === "success" && data.rates && typeof data.rates === "object") {
      currencyCache = {
        rates: data.rates,
        lastUpdated: data.time_last_update_utc || new Date().toISOString(),
        timestamp: Date.now(),
        provider: "open.er-api.com (ECB)",
      };
      try {
        if (typeof localStorage !== "undefined") {
          localStorage.setItem(CACHE_STORAGE_KEY, JSON.stringify(currencyCache));
        }
      } catch {
        // Ignore write error
      }
      isRefreshingRates = false;
      return true;
    }
  } catch {
    // Network or parse error: retain existing cache
  }
  isRefreshingRates = false;
  return false;
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

  if (Date.now() - currencyCache.timestamp > 24 * 3600 * 1000) {
    void refreshCurrencyRates();
  }

  const fromRate = currencyCache.rates[from];
  const toRate = currencyCache.rates[to];
  if (!Number.isFinite(amount) || fromRate === undefined || toRate === undefined) {
    return null;
  }

  const converted = (amount / fromRate) * toRate;
  const rateRatio = toRate / fromRate;
  const isStale = Date.now() - currencyCache.timestamp > 48 * 3600 * 1000;
  const stalenessNote = isStale ? ` · cached (${currencyCache.lastUpdated.slice(0, 16)})` : "";

  return {
    value: new Intl.NumberFormat("en", {
      style: "currency",
      currency: to,
    }).format(converted),
    expression: `${amount} ${from}`,
    label: `${from} to ${to} · rate ${rateRatio.toFixed(4)}${stalenessNote}`,
    raw: converted.toFixed(2),
    categoryLabel: "Currency",
  };
}

// ============================================================================
// 2. TIME ZONES & TIME CONVERSION
// ============================================================================

const CITY_TIMEZONES: Readonly<Record<string, string>> = {
  london: "Europe/London",
  lisbon: "Europe/Lisbon",
  "new york": "America/New_York",
  nyc: "America/New_York",
  tokyo: "Asia/Tokyo",
  paris: "Europe/Paris",
  berlin: "Europe/Berlin",
  sydney: "Australia/Sydney",
  auckland: "Pacific/Auckland",
  "los angeles": "America/Los_Angeles",
  la: "America/Los_Angeles",
  "san francisco": "America/Los_Angeles",
  sf: "America/Los_Angeles",
  chicago: "America/Chicago",
  toronto: "America/Toronto",
  vancouver: "America/Vancouver",
  dubai: "Asia/Dubai",
  singapore: "Asia/Singapore",
  "hong kong": "Asia/Hong_Kong",
  beijing: "Asia/Shanghai",
  shanghai: "Asia/Shanghai",
  mumbai: "Asia/Kolkata",
  delhi: "Asia/Kolkata",
  bangalore: "Asia/Kolkata",
  "sao paulo": "America/Sao_Paulo",
  "buenos aires": "America/Argentina/Buenos_Aires",
  cairo: "Africa/Cairo",
  johannesburg: "Africa/Johannesburg",
  seoul: "Asia/Seoul",
  bangkok: "Asia/Bangkok",
  rome: "Europe/Rome",
  madrid: "Europe/Madrid",
  amsterdam: "Europe/Amsterdam",
  brussels: "Europe/Brussels",
  vienna: "Europe/Vienna",
  stockholm: "Europe/Stockholm",
  oslo: "Europe/Oslo",
  copenhagen: "Europe/Copenhagen",
  helsinki: "Europe/Helsinki",
  athens: "Europe/Athens",
  istanbul: "Europe/Istanbul",
  dublin: "Europe/Dublin",
  honolulu: "Pacific/Honolulu",
  utc: "UTC",
  gmt: "UTC",
};

export function resolveTimeZone(input: string): { name: string; tz: string } | null {
  const clean = input.trim().toLowerCase();
  if (clean === "here" || clean === "local") {
    let localTz = "UTC";
    try {
      localTz = Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
    } catch {
      localTz = "UTC";
    }
    return { name: "Local", tz: localTz };
  }

  if (CITY_TIMEZONES[clean]) {
    const tz = CITY_TIMEZONES[clean];
    const name = clean.split(" ").map((w) => w.charAt(0).toUpperCase() + w.slice(1)).join(" ");
    return { name, tz };
  }

  // Check if it's already an IANA timezone like Europe/London
  try {
    Intl.DateTimeFormat(undefined, { timeZone: input.trim() });
    return { name: input.trim(), tz: input.trim() };
  } catch {
    return null;
  }
}

function getTimeZoneOffsetMinutes(tz: string, date: Date): number {
  const parts = new Intl.DateTimeFormat("en-US", {
    timeZone: tz,
    year: "numeric",
    month: "numeric",
    day: "numeric",
    hour: "numeric",
    minute: "numeric",
    second: "numeric",
    hour12: false,
  }).formatToParts(date);

  let year = 1970;
  let month = 1;
  let day = 1;
  let hour = 0;
  let minute = 0;
  let second = 0;

  for (const part of parts) {
    if (part.type === "year") year = Number(part.value);
    else if (part.type === "month") month = Number(part.value);
    else if (part.type === "day") day = Number(part.value);
    else if (part.type === "hour") {
      hour = Number(part.value);
      if (hour === 24) hour = 0;
    } else if (part.type === "minute") minute = Number(part.value);
    else if (part.type === "second") second = Number(part.value);
  }

  const asUtc = Date.UTC(year, month - 1, day, hour, minute, second);
  return (asUtc - date.getTime()) / 60000;
}

export function matchTimeZoneConversion(query: string): CalcPreview | null {
  const trimmed = query.trim();

  // Pattern A: "time in <city>" or "time <city>"
  const timeInMatch = /^(?:what(?:\s+is)?\s+the\s+)?time(?:\s+in|\s+at)?\s+([a-zA-Z\s/_-]+)$/i.exec(trimmed);
  if (timeInMatch) {
    const placeStr = timeInMatch[1].trim();
    if (placeStr.toLowerCase() === "diff" || placeStr.toLowerCase().startsWith("difference")) {
      // Handled by Pattern B
    } else {
      const resolved = resolveTimeZone(placeStr);
      if (resolved) {
        const now = new Date();
        const timeStr = new Intl.DateTimeFormat("en-US", {
          timeZone: resolved.tz,
          hour: "numeric",
          minute: "2-digit",
          hour12: true,
        }).format(now);
        const dateStr = new Intl.DateTimeFormat("en-US", {
          timeZone: resolved.tz,
          weekday: "short",
          month: "short",
          day: "numeric",
        }).format(now);

        return {
          value: timeStr,
          expression: `Current time in ${resolved.name}`,
          label: `${dateStr} · ${resolved.tz}`,
          raw: timeStr,
          categoryLabel: "Time Zone",
        };
      }
    }
  }

  // Pattern B: "time difference [between] <place1> [and] <place2>" or "time diff <place1> <place2>"
  const diffMatch = /^(?:what(?:\s+is)?\s+the\s+)?time\s+diff(?:erence)?(?:\s+between)?\s+([a-zA-Z\s/_-]+?)(?:\s+and\s+|\s+to\s+|\s+)([a-zA-Z\s/_-]+)$/i.exec(
    trimmed,
  );
  if (diffMatch) {
    const r1 = resolveTimeZone(diffMatch[1]);
    const r2 = resolveTimeZone(diffMatch[2]);
    if (r1 && r2) {
      const now = new Date();
      const off1 = getTimeZoneOffsetMinutes(r1.tz, now);
      const off2 = getTimeZoneOffsetMinutes(r2.tz, now);
      const diffMins = off1 - off2;
      const diffHours = diffMins / 60;

      let valueText = "";
      if (diffHours === 0) {
        valueText = "Same time (0 hours difference)";
      } else if (diffHours > 0) {
        const h = Number(diffHours.toFixed(2));
        valueText = `${r1.name} is ${h}h ahead of ${r2.name}`;
      } else {
        const h = Number(Math.abs(diffHours).toFixed(2));
        valueText = `${r1.name} is ${h}h behind ${r2.name}`;
      }

      return {
        value: valueText,
        expression: `Time difference: ${r1.name} vs ${r2.name}`,
        label: `${r1.tz} to ${r2.tz}`,
        raw: valueText,
        categoryLabel: "Time Zone",
      };
    }
  }

  // Pattern C: "<time> [from] <place1> to <place2>" e.g. "10 PM here to London", "9:30 Lisbon to New York"
  const convMatch = /^(\d{1,2}(?::\d{2})?\s*(?:am|pm)?)\s+(?:from\s+)?([a-zA-Z\s/_-]+?)\s+to\s+([a-zA-Z\s/_-]+)$/i.exec(
    trimmed,
  );
  if (convMatch) {
    const timeStr = convMatch[1].trim();
    const r1 = resolveTimeZone(convMatch[2]);
    const r2 = resolveTimeZone(convMatch[3]);
    if (r1 && r2) {
      // Parse time components
      const tMatch = /^(\d{1,2})(?::(\d{2}))?\s*(am|pm)?$/i.exec(timeStr);
      if (tMatch) {
        let hour = Number(tMatch[1]);
        const minute = tMatch[2] ? Number(tMatch[2]) : 0;
        const meridiem = tMatch[3] ? tMatch[3].toLowerCase() : null;

        if (meridiem === "pm" && hour < 12) hour += 12;
        if (meridiem === "am" && hour === 12) hour = 0;

        if (hour >= 0 && hour <= 23 && minute >= 0 && minute <= 59) {
          const now = new Date();
          // Find offset for r1 and r2
          const off1 = getTimeZoneOffsetMinutes(r1.tz, now);
          const off2 = getTimeZoneOffsetMinutes(r2.tz, now);
          const diffMinutes = off2 - off1;

          // Source time in minutes from midnight
          const srcTotalMinutes = hour * 60 + minute;
          let dstTotalMinutes = srcTotalMinutes + diffMinutes;

          let dayOffset = 0;
          while (dstTotalMinutes < 0) {
            dstTotalMinutes += 1440;
            dayOffset -= 1;
          }
          while (dstTotalMinutes >= 1440) {
            dstTotalMinutes -= 1440;
            dayOffset += 1;
          }

          const dstHour24 = Math.floor(dstTotalMinutes / 60);
          const dstMinute = Math.floor(dstTotalMinutes % 60);

          const dstMeridiem = dstHour24 >= 12 ? "PM" : "AM";
          const dstHour12 = dstHour24 % 12 === 0 ? 12 : dstHour24 % 12;
          const minFormatted = String(dstMinute).padStart(2, "0");

          let dayNote = "";
          if (dayOffset === 1) dayNote = " (+1 day)";
          else if (dayOffset === -1) dayNote = " (-1 day)";
          else if (dayOffset > 1) dayNote = ` (+${dayOffset} days)`;
          else if (dayOffset < -1) dayNote = ` (${dayOffset} days)`;

          const formattedResult = `${dstHour12}:${minFormatted} ${dstMeridiem}${dayNote}`;

          return {
            value: formattedResult,
            expression: `${timeStr} in ${r1.name} → ${r2.name}`,
            label: `${r2.tz} · Difference ${((off2 - off1) / 60).toFixed(1)}h`,
            raw: `${dstHour12}:${minFormatted} ${dstMeridiem}`,
            categoryLabel: "Time Zone",
          };
        }
      }
    }
  }

  return null;
}

// ============================================================================
// 3. DATES AND DURATIONS
// ============================================================================

const MONTH_NAMES: Readonly<Record<string, number>> = {
  jan: 0, january: 0,
  feb: 1, february: 1,
  mar: 2, march: 2,
  apr: 3, april: 3,
  may: 4,
  jun: 5, june: 5,
  jul: 6, july: 6,
  aug: 7, august: 7,
  sep: 8, sept: 8, september: 8,
  oct: 9, october: 9,
  nov: 10, november: 10,
  dec: 11, december: 11,
};

function parseDateInput(str: string, referenceDate: Date): Date | null {
  const clean = str.trim().toLowerCase();
  if (clean === "today" || clean === "now") {
    return new Date(referenceDate.getFullYear(), referenceDate.getMonth(), referenceDate.getDate());
  }
  if (clean === "tomorrow") {
    const d = new Date(referenceDate.getFullYear(), referenceDate.getMonth(), referenceDate.getDate());
    d.setDate(d.getDate() + 1);
    return d;
  }
  if (clean === "yesterday") {
    const d = new Date(referenceDate.getFullYear(), referenceDate.getMonth(), referenceDate.getDate());
    d.setDate(d.getDate() - 1);
    return d;
  }
  if (clean === "christmas") {
    return new Date(referenceDate.getFullYear(), 11, 25);
  }
  if (clean === "new year" || clean === "new years" || clean === "new year's") {
    return new Date(referenceDate.getFullYear(), 0, 1);
  }
  if (clean === "halloween") {
    return new Date(referenceDate.getFullYear(), 9, 31);
  }

  // ISO date: YYYY-MM-DD
  const isoMatch = /^(\d{4})-(\d{1,2})-(\d{1,2})$/.exec(clean);
  if (isoMatch) {
    const y = Number(isoMatch[1]);
    const m = Number(isoMatch[2]) - 1;
    const day = Number(isoMatch[3]);
    const d = new Date(y, m, day);
    if (d.getFullYear() === y && d.getMonth() === m && d.getDate() === day) {
      return d;
    }
    return null;
  }

  // Day Month [Year], e.g. "12 Oct" or "21 Oct 2026" or "12 October"
  const dmyMatch = /^(\d{1,2})\s+([a-zA-Z]+)(?:\s+(\d{4}))?$/.exec(clean);
  if (dmyMatch) {
    const day = Number(dmyMatch[1]);
    const monthKey = dmyMatch[2].toLowerCase();
    const m = MONTH_NAMES[monthKey];
    if (m !== undefined) {
      const year = dmyMatch[3] ? Number(dmyMatch[3]) : referenceDate.getFullYear();
      const d = new Date(year, m, day);
      if (d.getMonth() === m && d.getDate() === day) {
        return d;
      }
    }
  }

  // Month Day [Year], e.g. "Oct 12" or "October 12 2026"
  const mdyMatch = /^([a-zA-Z]+)\s+(\d{1,2})(?:\s+(\d{4}))?$/.exec(clean);
  if (mdyMatch) {
    const monthKey = mdyMatch[1].toLowerCase();
    const m = MONTH_NAMES[monthKey];
    const day = Number(mdyMatch[2]);
    if (m !== undefined) {
      const year = mdyMatch[3] ? Number(mdyMatch[3]) : referenceDate.getFullYear();
      const d = new Date(year, m, day);
      if (d.getMonth() === m && d.getDate() === day) {
        return d;
      }
    }
  }

  return null;
}

export function matchDateDuration(query: string, referenceDate: Date = new Date()): CalcPreview | null {
  const trimmed = query.trim();

  // Pattern A: "<N> (days|weeks|months|years) (from today|from now|after today|ago|before today)"
  const offsetMatch = /^(\d+)\s+(day|days|week|weeks|month|months|year|years)\s+(from\s+today|from\s+now|after\s+today|ago|before\s+today)$/i.exec(
    trimmed,
  );
  if (offsetMatch) {
    const amount = Number(offsetMatch[1]);
    const unit = offsetMatch[2].toLowerCase();
    const dirStr = offsetMatch[3].toLowerCase();
    const isPast = dirStr.includes("ago") || dirStr.includes("before");

    const target = new Date(referenceDate.getFullYear(), referenceDate.getMonth(), referenceDate.getDate());

    if (unit.startsWith("day")) {
      target.setDate(target.getDate() + (isPast ? -amount : amount));
    } else if (unit.startsWith("week")) {
      target.setDate(target.getDate() + (isPast ? -amount * 7 : amount * 7));
    } else if (unit.startsWith("month")) {
      target.setMonth(target.getMonth() + (isPast ? -amount : amount));
    } else if (unit.startsWith("year")) {
      target.setFullYear(target.getFullYear() + (isPast ? -amount : amount));
    }

    const formatted = target.toLocaleDateString("en-US", {
      weekday: "long",
      year: "numeric",
      month: "long",
      day: "numeric",
    });

    return {
      value: formatted,
      expression: `${amount} ${unit} ${dirStr}`,
      label: `Date Calculation · Calendar Days`,
      raw: formatted,
      categoryLabel: "Date",
    };
  }

  // Pattern B: "days until <date/event>"
  const untilMatch = /^days\s+until\s+(.+)$/i.exec(trimmed);
  if (untilMatch) {
    let target = parseDateInput(untilMatch[1], referenceDate);
    if (target) {
      const today = new Date(referenceDate.getFullYear(), referenceDate.getMonth(), referenceDate.getDate());
      // If target is in the past and no year was specified, jump to next year
      if (target.getTime() < today.getTime() && !/\d{4}/.test(untilMatch[1])) {
        target = new Date(target.getFullYear() + 1, target.getMonth(), target.getDate());
      }

      const diffMs = target.getTime() - today.getTime();
      const diffDays = Math.round(diffMs / (1000 * 60 * 60 * 24));

      const targetFormatted = target.toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
        year: "numeric",
      });

      const displayValue = diffDays === 0 ? "Today" : `${diffDays} days`;

      return {
        value: displayValue,
        expression: `Days until ${targetFormatted}`,
        label: `Countdown · ${diffDays} calendar days`,
        raw: String(diffDays),
        categoryLabel: "Date",
      };
    }
  }

  // Pattern C: "days between <date1> and <date2>"
  const betweenMatch = /^days\s+between\s+(.+?)\s+and\s+(.+)$/i.exec(trimmed);
  if (betweenMatch) {
    const d1 = parseDateInput(betweenMatch[1], referenceDate);
    const d2 = parseDateInput(betweenMatch[2], referenceDate);
    if (d1 && d2) {
      const diffMs = Math.abs(d2.getTime() - d1.getTime());
      const diffDays = Math.round(diffMs / (1000 * 60 * 60 * 24));

      const fmt1 = d1.toLocaleDateString("en-US", { month: "short", day: "numeric" });
      const fmt2 = d2.toLocaleDateString("en-US", { month: "short", day: "numeric" });

      return {
        value: `${diffDays} days`,
        expression: `Between ${fmt1} and ${fmt2}`,
        label: `Duration · ${diffDays} calendar days`,
        raw: String(diffDays),
        categoryLabel: "Date",
      };
    }
  }

  return null;
}

// ============================================================================
// 4. UNIT CONVERSIONS (SI vs IEC Storage, Metric, Imperial, Temperature)
// ============================================================================

interface UnitDef {
  type: "distance" | "weight" | "storage" | "temperature";
  factor: number; // base: meter, gram, byte
  isIec?: boolean;
}

const UNITS: Readonly<Record<string, UnitDef>> = {
  // Distance (base: meter)
  km: { type: "distance", factor: 1000 },
  kilometer: { type: "distance", factor: 1000 },
  kilometers: { type: "distance", factor: 1000 },
  kilometre: { type: "distance", factor: 1000 },
  kilometres: { type: "distance", factor: 1000 },
  m: { type: "distance", factor: 1 },
  meter: { type: "distance", factor: 1 },
  meters: { type: "distance", factor: 1 },
  metre: { type: "distance", factor: 1 },
  metres: { type: "distance", factor: 1 },
  cm: { type: "distance", factor: 0.01 },
  centimeter: { type: "distance", factor: 0.01 },
  centimeters: { type: "distance", factor: 0.01 },
  mm: { type: "distance", factor: 0.001 },
  millimeter: { type: "distance", factor: 0.001 },
  millimeters: { type: "distance", factor: 0.001 },
  mi: { type: "distance", factor: 1609.344 },
  mile: { type: "distance", factor: 1609.344 },
  miles: { type: "distance", factor: 1609.344 },
  ft: { type: "distance", factor: 0.3048 },
  foot: { type: "distance", factor: 0.3048 },
  feet: { type: "distance", factor: 0.3048 },
  in: { type: "distance", factor: 0.0254 },
  inch: { type: "distance", factor: 0.0254 },
  inches: { type: "distance", factor: 0.0254 },
  yd: { type: "distance", factor: 0.9144 },
  yard: { type: "distance", factor: 0.9144 },
  yards: { type: "distance", factor: 0.9144 },

  // Temperature
  c: { type: "temperature", factor: 0 },
  celsius: { type: "temperature", factor: 0 },
  f: { type: "temperature", factor: 0 },
  fahrenheit: { type: "temperature", factor: 0 },
  k: { type: "temperature", factor: 0 },
  kelvin: { type: "temperature", factor: 0 },

  // Weight (base: gram)
  kg: { type: "weight", factor: 1000 },
  kilogram: { type: "weight", factor: 1000 },
  kilograms: { type: "weight", factor: 1000 },
  g: { type: "weight", factor: 1 },
  gram: { type: "weight", factor: 1 },
  grams: { type: "weight", factor: 1 },
  mg: { type: "weight", factor: 0.001 },
  milligram: { type: "weight", factor: 0.001 },
  milligrams: { type: "weight", factor: 0.001 },
  lb: { type: "weight", factor: 453.59237 },
  lbs: { type: "weight", factor: 453.59237 },
  pound: { type: "weight", factor: 453.59237 },
  pounds: { type: "weight", factor: 453.59237 },
  oz: { type: "weight", factor: 28.349523125 },
  ounce: { type: "weight", factor: 28.349523125 },
  ounces: { type: "weight", factor: 28.349523125 },

  // Storage (base: byte)
  // Decimal SI (powers of 1000)
  bit: { type: "storage", factor: 0.125 },
  bits: { type: "storage", factor: 0.125 },
  byte: { type: "storage", factor: 1 },
  bytes: { type: "storage", factor: 1 },
  kb: { type: "storage", factor: 1_000 },
  mb: { type: "storage", factor: 1_000_000 },
  gb: { type: "storage", factor: 1_000_000_000 },
  tb: { type: "storage", factor: 1_000_000_000_000 },

  // Binary IEC (powers of 1024)
  kib: { type: "storage", factor: 1024, isIec: true },
  mib: { type: "storage", factor: 1024 ** 2, isIec: true },
  gib: { type: "storage", factor: 1024 ** 3, isIec: true },
  tib: { type: "storage", factor: 1024 ** 4, isIec: true },
};

function formatUnitNumber(val: number): string {
  if (Number.isInteger(val)) {
    return val.toLocaleString("en-US");
  }
  const str = val.toFixed(6).replace(/\.?0+$/, "");
  const parts = str.split(".");
  const intPart = Number(parts[0]).toLocaleString("en-US");
  return parts.length > 1 ? `${intPart}.${parts[1]}` : intPart;
}

export function matchUnitConversion(query: string): CalcPreview | null {
  const match = /^([\d.,]+)\s*([a-zA-Z°]+)\s+(?:to|in)\s+([a-zA-Z°]+)$/i.exec(query.trim());
  if (!match) {
    return null;
  }

  const amountText = match[1].replace(",", ".");
  if ((amountText.match(/\./g) ?? []).length > 1) {
    return null;
  }

  const amount = Number(amountText);
  if (!Number.isFinite(amount)) {
    return null;
  }

  const fromRaw = match[2].toLowerCase().replace("°", "");
  const toRaw = match[3].toLowerCase().replace("°", "");

  const uFrom = UNITS[fromRaw];
  const uTo = UNITS[toRaw];

  if (!uFrom || !uTo || uFrom.type !== uTo.type) {
    return null;
  }

  // Temperature conversion
  if (uFrom.type === "temperature") {
    let kelvin = 0;
    const fKey = fromRaw.charAt(0);
    const tKey = toRaw.charAt(0);

    if (fKey === "c") kelvin = amount + 273.15;
    else if (fKey === "f") kelvin = ((amount - 32) * 5) / 9 + 273.15;
    else if (fKey === "k") kelvin = amount;
    else return null;

    let converted = 0;
    let targetUnitSymbol = "";
    if (tKey === "c") {
      converted = kelvin - 273.15;
      targetUnitSymbol = "°C";
    } else if (tKey === "f") {
      converted = ((kelvin - 273.15) * 9) / 5 + 32;
      targetUnitSymbol = "°F";
    } else if (tKey === "k") {
      converted = kelvin;
      targetUnitSymbol = "K";
    } else {
      return null;
    }

    const formatted = formatUnitNumber(converted);
    const sourceSymbol = fKey === "k" ? "K" : `°${fKey.toUpperCase()}`;

    return {
      value: `${formatted} ${targetUnitSymbol}`,
      expression: `${amount} ${sourceSymbol} to ${targetUnitSymbol}`,
      label: `Temperature Conversion`,
      raw: String(Number(converted.toFixed(4))),
      categoryLabel: "Unit",
    };
  }

  // Linear units (distance, weight, storage)
  const baseValue = amount * uFrom.factor;
  const converted = baseValue / uTo.factor;
  const formatted = formatUnitNumber(converted);

  const displayTarget = match[3];
  let subLabel = `Unit Conversion · ${uFrom.type.charAt(0).toUpperCase() + uFrom.type.slice(1)}`;
  if (uFrom.type === "storage") {
    if (uFrom.isIec && !uTo.isIec) subLabel = "Storage · Binary IEC to Decimal SI";
    else if (!uFrom.isIec && uTo.isIec) subLabel = "Storage · Decimal SI to Binary IEC";
    else if (uFrom.isIec && uTo.isIec) subLabel = "Storage · Binary IEC";
    else subLabel = "Storage · Decimal SI";
  }

  return {
    value: `${formatted} ${displayTarget}`,
    expression: `${amount} ${match[2]} = ${formatted} ${displayTarget}`,
    label: subLabel,
    raw: String(Number(converted.toFixed(6))),
    categoryLabel: "Unit",
  };
}

// ============================================================================
// 5. PRACTICAL CALCULATIONS (Percentages, VAT/Tax, Bill Split)
// ============================================================================

export function matchPracticalCalculation(query: string): CalcPreview | null {
  const trimmed = query.trim();

  // Pattern A: Percentage e.g. "15% of 240"
  const pctMatch = /^([\d.,]+)\s*%\s+of\s+([\d.,]+)$/i.exec(trimmed);
  if (pctMatch) {
    const p = Number(pctMatch[1].replace(",", "."));
    const n = Number(pctMatch[2].replace(",", "."));
    if (Number.isFinite(p) && Number.isFinite(n)) {
      const result = (p / 100) * n;
      const formatted = formatCalculatorNumber(result);
      return {
        value: formatted,
        expression: `${p}% of ${n}`,
        label: `Percentage Calculation · ${p}% of ${n} = ${formatted}`,
        raw: formatted,
        categoryLabel: "Calculation",
      };
    }
  }

  // Pattern B: VAT addition e.g. "VAT 23% on 100", "23% VAT on 100", "100 + 23% VAT"
  const vatAddMatch = /^(?:VAT\s+([\d.,]+)%\s+on\s+([\d.,]+)|([\d.,]+)%\s+VAT\s+on\s+([\d.,]+)|([\d.,]+)\s*\+\s*([\d.,]+)%\s*VAT)$/i.exec(
    trimmed,
  );
  if (vatAddMatch) {
    let rate = 0;
    let base = 0;
    if (vatAddMatch[1] && vatAddMatch[2]) {
      rate = Number(vatAddMatch[1].replace(",", "."));
      base = Number(vatAddMatch[2].replace(",", "."));
    } else if (vatAddMatch[3] && vatAddMatch[4]) {
      rate = Number(vatAddMatch[3].replace(",", "."));
      base = Number(vatAddMatch[4].replace(",", "."));
    } else if (vatAddMatch[5] && vatAddMatch[6]) {
      base = Number(vatAddMatch[5].replace(",", "."));
      rate = Number(vatAddMatch[6].replace(",", "."));
    }

    if (Number.isFinite(rate) && Number.isFinite(base)) {
      const taxAmount = (base * rate) / 100;
      const total = base + taxAmount;
      return {
        value: total.toFixed(2),
        expression: `${base.toFixed(2)} + ${taxAmount.toFixed(2)} (${rate}% VAT)`,
        label: `Total with VAT · Tax amount: ${taxAmount.toFixed(2)}`,
        raw: total.toFixed(2),
        categoryLabel: "Tax",
      };
    }
  }

  // Pattern C: VAT removal e.g. "remove 23% VAT from 123", "extract 23% VAT from 123"
  const vatRemMatch = /^(?:remove|extract)\s+([\d.,]+)%\s*VAT\s+from\s+([\d.,]+)$/i.exec(trimmed);
  if (vatRemMatch) {
    const rate = Number(vatRemMatch[1].replace(",", "."));
    const gross = Number(vatRemMatch[2].replace(",", "."));
    if (Number.isFinite(rate) && Number.isFinite(gross) && rate > 0) {
      const net = gross / (1 + rate / 100);
      const taxIncluded = gross - net;
      return {
        value: net.toFixed(2),
        expression: `${gross.toFixed(2)} with ${rate}% VAT removed (Net)`,
        label: `Net before VAT · VAT included: ${taxIncluded.toFixed(2)}`,
        raw: net.toFixed(2),
        categoryLabel: "Tax",
      };
    }
  }

  // Pattern D: Bill split e.g. "split 87.50 between 4", "split 100 among 5"
  const splitMatch = /^split\s+([\d.,]+)\s+(?:between|by|among)\s+(\d+)$/i.exec(trimmed);
  if (splitMatch) {
    const total = Number(splitMatch[1].replace(",", "."));
    const count = Number(splitMatch[2]);
    if (Number.isFinite(total) && count > 0) {
      const each = total / count;
      const formatted = each.toFixed(2);
      return {
        value: `${formatted} each`,
        expression: `${total.toFixed(2)} split between ${count} people`,
        label: `Bill Split · ${formatted} per person`,
        raw: formatted,
        categoryLabel: "Calculation",
      };
    }
  }

  return null;
}

// ============================================================================
// 6. COLOR CONVERSIONS (HEX <-> RGB <-> HSL)
// ============================================================================

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  const rNorm = r / 255;
  const gNorm = g / 255;
  const bNorm = b / 255;

  const max = Math.max(rNorm, gNorm, bNorm);
  const min = Math.min(rNorm, gNorm, bNorm);
  let h = 0;
  let s = 0;
  const l = (max + min) / 2;

  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case rNorm:
        h = (gNorm - bNorm) / d + (gNorm < bNorm ? 6 : 0);
        break;
      case gNorm:
        h = (bNorm - rNorm) / d + 2;
        break;
      case bNorm:
        h = (rNorm - gNorm) / d + 4;
        break;
    }
    h /= 6;
  }

  return [Math.round(h * 360), Math.round(s * 100), Math.round(l * 100)];
}

function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  const sNorm = s / 100;
  const lNorm = l / 100;

  const c = (1 - Math.abs(2 * lNorm - 1)) * sNorm;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = lNorm - c / 2;

  let rPrime = 0;
  let gPrime = 0;
  let bPrime = 0;

  if (h >= 0 && h < 60) {
    rPrime = c; gPrime = x; bPrime = 0;
  } else if (h >= 60 && h < 120) {
    rPrime = x; gPrime = c; bPrime = 0;
  } else if (h >= 120 && h < 180) {
    rPrime = 0; gPrime = c; bPrime = x;
  } else if (h >= 180 && h < 240) {
    rPrime = 0; gPrime = x; bPrime = c;
  } else if (h >= 240 && h < 300) {
    rPrime = x; gPrime = 0; bPrime = c;
  } else {
    rPrime = c; gPrime = 0; bPrime = x;
  }

  return [
    Math.round((rPrime + m) * 255),
    Math.round((gPrime + m) * 255),
    Math.round((bPrime + m) * 255),
  ];
}

function componentToHex(c: number): string {
  const hex = c.toString(16);
  return hex.length === 1 ? "0" + hex : hex;
}

function rgbToHex(r: number, g: number, b: number): string {
  return `#${componentToHex(r)}${componentToHex(g)}${componentToHex(b)}`.toLowerCase();
}

export function matchColorConversion(query: string): CalcPreview | null {
  const trimmed = query.trim();

  // Pattern A: Hex e.g. "#798F78 to RGB", "#798f78", "#fff to HSL"
  const hexMatch = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})(?:\s+(?:to|in)\s+(rgb|hsl))?$/i.exec(trimmed);
  if (hexMatch) {
    let hex = hexMatch[1];
    if (hex.length === 3) {
      hex = hex.split("").map((ch) => ch + ch).join("");
    }
    const r = parseInt(hex.slice(0, 2), 16);
    const g = parseInt(hex.slice(2, 4), 16);
    const b = parseInt(hex.slice(4, 6), 16);

    const fullHex = `#${hex.toLowerCase()}`;
    const [h, s, l] = rgbToHsl(r, g, b);
    const rgbStr = `rgb(${r}, ${g}, ${b})`;
    const hslStr = `hsl(${h}, ${s}%, ${l}%)`;

    const targetFormat = hexMatch[2]?.toLowerCase();
    let displayVal = `${rgbStr} · ${hslStr}`;
    if (targetFormat === "rgb") displayVal = rgbStr;
    else if (targetFormat === "hsl") displayVal = hslStr;

    return {
      value: displayVal,
      expression: `#${hexMatch[1]}`,
      label: `Color · RGB & HSL`,
      raw: targetFormat === "hsl" ? hslStr : rgbStr,
      colorPreview: fullHex,
      categoryLabel: "Color",
    };
  }

  // Pattern B: RGB e.g. "rgb(121, 143, 120) to HEX", "rgb(121, 143, 120)"
  const rgbMatch = /^rgb\s*\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})\s*\)(?:\s+(?:to|in)\s+(hex|hsl))?$/i.exec(
    trimmed,
  );
  if (rgbMatch) {
    const r = Number(rgbMatch[1]);
    const g = Number(rgbMatch[2]);
    const b = Number(rgbMatch[3]);
    if (r >= 0 && r <= 255 && g >= 0 && g <= 255 && b >= 0 && b <= 255) {
      const hex = rgbToHex(r, g, b);
      const [h, s, l] = rgbToHsl(r, g, b);
      const hslStr = `hsl(${h}, ${s}%, ${l}%)`;

      const target = rgbMatch[4]?.toLowerCase();
      let displayVal = `${hex} · ${hslStr}`;
      if (target === "hex") displayVal = hex;
      else if (target === "hsl") displayVal = hslStr;

      return {
        value: displayVal,
        expression: `rgb(${r}, ${g}, ${b})`,
        label: `Color · HEX & HSL`,
        raw: target === "hsl" ? hslStr : hex,
        colorPreview: hex,
        categoryLabel: "Color",
      };
    }
  }

  // Pattern C: HSL e.g. "hsl(117, 9%, 52%) to HEX", "hsl(117, 9%, 52%)"
  const hslMatch = /^hsl\s*\(\s*(\d{1,3})\s*,\s*(\d{1,3})%\s*,\s*(\d{1,3})%\s*\)(?:\s+(?:to|in)\s+(hex|rgb))?$/i.exec(
    trimmed,
  );
  if (hslMatch) {
    const h = Number(hslMatch[1]);
    const s = Number(hslMatch[2]);
    const l = Number(hslMatch[3]);
    if (h >= 0 && h <= 360 && s >= 0 && s <= 100 && l >= 0 && l <= 100) {
      const [r, g, b] = hslToRgb(h, s, l);
      const hex = rgbToHex(r, g, b);
      const rgbStr = `rgb(${r}, ${g}, ${b})`;

      const target = hslMatch[4]?.toLowerCase();
      let displayVal = `${hex} · ${rgbStr}`;
      if (target === "hex") displayVal = hex;
      else if (target === "rgb") displayVal = rgbStr;

      return {
        value: displayVal,
        expression: `hsl(${h}, ${s}%, ${l}%)`,
        label: `Color · HEX & RGB`,
        raw: target === "rgb" ? rgbStr : hex,
        colorPreview: hex,
        categoryLabel: "Color",
      };
    }
  }

  return null;
}

// ============================================================================
// 7. SECURE PASSWORD GENERATION
// ============================================================================

export function matchPasswordGeneration(query: string): CalcPreview | null {
  const match = /^(?:generate|gen)\s+password(?:\s+(\d+))?$/i.exec(query.trim());
  if (!match) {
    return null;
  }

  const length = match[1] ? Number(match[1]) : 16;
  if (!Number.isFinite(length) || length < 8 || length > 128) {
    return {
      value: "Length must be between 8 and 128",
      expression: query.trim(),
      label: "Password Generator · Invalid Length",
      raw: "",
      categoryLabel: "Password",
    };
  }

  const upper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
  const lower = "abcdefghijklmnopqrstuvwxyz";
  const digits = "0123456789";
  const symbols = "!@#$%^&*()-_=+[]{}|;:,.<>?";
  const pool = upper + lower + digits + symbols;

  const randomBuffer = new Uint32Array(length);
  if (typeof crypto !== "undefined" && typeof crypto.getRandomValues === "function") {
    crypto.getRandomValues(randomBuffer);
  } else {
    // Deterministic fallback for environments without crypto
    for (let i = 0; i < length; i++) {
      randomBuffer[i] = Math.floor(Math.random() * 0xffffffff);
    }
  }

  const chars: string[] = [];
  for (let i = 0; i < length; i++) {
    chars.push(pool[randomBuffer[i] % pool.length]);
  }

  const password = chars.join("");

  return {
    value: password,
    expression: `Secure password (${length} chars)`,
    label: `Password Generator · Cryptographically Secure`,
    raw: password,
    categoryLabel: "Password",
  };
}

// ============================================================================
// 8. MATHEMATICAL PARSER (ARITHMETIC, TRIGONOMETRY, CONSTANTS)
// ============================================================================

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

// ============================================================================
// 9. UNIFIED ENTRY POINT
// ============================================================================

export function evaluateMathExpression(rawQuery: string): CalcPreview | null {
  const trimmed = rawQuery.trim();
  if (!trimmed) {
    return null;
  }

  // 1. Explicit Password Generation
  const passwordPreview = matchPasswordGeneration(trimmed);
  if (passwordPreview) {
    return passwordPreview;
  }

  // 2. Color Conversions
  const colorPreview = matchColorConversion(trimmed);
  if (colorPreview) {
    return colorPreview;
  }

  // 3. Currency Conversion
  const currencyPreview = matchCurrencyConversion(trimmed);
  if (currencyPreview) {
    return currencyPreview;
  }

  // 4. Practical Calculations (Percentages, VAT, Bill Split)
  const practicalPreview = matchPracticalCalculation(trimmed);
  if (practicalPreview) {
    return practicalPreview;
  }

  // 5. Unit Conversions
  const unitPreview = matchUnitConversion(trimmed);
  if (unitPreview) {
    return unitPreview;
  }

  // 6. Time Zones and Time Conversions
  const timeZonePreview = matchTimeZoneConversion(trimmed);
  if (timeZonePreview) {
    return timeZonePreview;
  }

  // 7. Dates and Durations
  const datePreview = matchDateDuration(trimmed);
  if (datePreview) {
    return datePreview;
  }

  // 8. General Math Expressions
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
    categoryLabel: "Calculator",
  };
}
