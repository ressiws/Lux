<script lang="ts">
  import type { CalcPreview, QuickAnswerDetail } from "./types";

  interface Props {
    calc: CalcPreview;
    details?: readonly QuickAnswerDetail[];
    selected?: boolean;
  }

  let { calc, details = [], selected = false }: Props = $props();

  function getTimeZoneAbbr(iana: string): string {
    try {
      const now = new Date();
      const formatter = new Intl.DateTimeFormat("en-US", {
        timeZone: iana,
        timeZoneName: "short",
        hour: "numeric",
        minute: "2-digit",
        hour12: true,
      });
      const parts = formatter.formatToParts(now);
      const tzPart = parts.find((p) => p.type === "timeZoneName");
      if (tzPart) {
        return tzPart.value;
      }
    } catch {
      // ignore
    }
    return iana;
  }

  const isTimeZone = $derived(calc.categoryLabel === "Time Zone");
  const isTimeInCity = $derived(isTimeZone && calc.expression.startsWith("Current time in "));

  const city = $derived(
    isTimeInCity
      ? calc.expression.slice("Current time in ".length).charAt(0).toUpperCase() + calc.expression.slice("Current time in ".length).slice(1)
      : "",
  );

  const timeZoneAbbr = $derived(
    isTimeInCity
      ? (() => {
          const labelParts = calc.label.split("·");
          if (labelParts.length === 2) {
            const iana = labelParts[1].trim();
            return getTimeZoneAbbr(iana);
          }
          return "";
        })()
      : "",
  );

  const leftMeta = $derived(isTimeInCity ? city : "Expression");
  const rightMeta = $derived(isTimeInCity ? timeZoneAbbr : "Result");
</script>

<div class="qa" class:selected>
  <div class="qa-main">
    <div class="qa-pane">
      {#if isTimeInCity}
        <div class="qa-title">
          <span class="qa-title-main">Time</span>
          <span class="qa-title-in">in</span>
          <span class="qa-title-city">{city}</span>
        </div>
      {:else}
        <div class="qa-value expression" title={calc.expression}>{calc.expression}</div>
      {/if}
    </div>
    <div class="qa-arrow" aria-hidden="true">
      <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="1.8"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M5 12h14M13 6l6 6-6 6" />
      </svg>
    </div>
    <div class="qa-pane">
      <div class="qa-value result" title={calc.value}>
        {#if calc.colorPreview}
          <span
            class="qa-color"
            style:background-color={calc.colorPreview}
            aria-hidden="true"
          ></span>
        {/if}
        <span class="qa-result-text">{calc.value}</span>
      </div>
    </div>
  </div>
  {#if (calc.label && calc.label !== "Result") || details.length > 0}
    <div class="qa-support">
      {#if calc.label && calc.label !== "Result"}
        <span class="qa-label">{calc.label}</span>
      {/if}
      {#if details.length > 0}
        <div class="qa-details">
          {#each details as detail (detail.label)}
            <span class="qa-detail"><span>{detail.label}</span><b>{detail.value}</b></span>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
  <div class="qa-meta">
    <span class="qa-pill">{leftMeta}</span>
    <span class="qa-pill">{rightMeta}</span>
  </div>
</div>

<style>
  .qa {
    margin: 4px 0 2px;
  }

  .qa-main {
    position: relative;
    display: flex;
    align-items: stretch;
    min-height: 112px;
    border-radius: 12px;
    background: var(--sel);
    overflow: hidden;
  }

  .qa.selected .qa-main {
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .qa-pane {
    display: flex;
    flex: 1;
    min-width: 0;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 18px 20px;
  }

  .qa-pane:first-child::after {
    content: "";
    position: absolute;
    top: 16px;
    bottom: 16px;
    left: 50%;
    width: 1px;
    background: var(--line);
  }

  .qa-value {
    max-width: 100%;
    overflow: hidden;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .qa-value.expression {
    color: var(--mut);
    font-size: 20px;
  }

  .qa-value.result {
    display: flex;
    align-items: center;
    color: var(--fg);
    font-size: 30px;
    font-weight: 500;
  }

  .qa-result-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .qa-color {
    width: 18px;
    height: 18px;
    flex: none;
    margin-right: 8px;
    border: 1px solid var(--line);
    border-radius: 4px;
  }

  .qa-title {
    display: flex;
    align-items: baseline;
    gap: 4px;
    font-size: 20px;
    line-height: 1.2;
  }

  .qa-title-main {
    color: var(--fg);
    font-weight: 500;
  }

  .qa-title-in {
    color: var(--mut);
    font-weight: 400;
  }

  .qa-title-city {
    color: var(--fg);
    font-weight: 500;
  }

  .qa-pill {
    padding: 2px 9px;
    border-radius: 6px;
    background: var(--chip);
    color: var(--mut);
    font-size: 11px;
    font-weight: 500;
  }

  .qa-arrow {
    position: absolute;
    top: 50%;
    left: 50%;
    display: grid;
    width: 26px;
    height: 26px;
    transform: translate(-50%, -50%);
    place-items: center;
    border-radius: 50%;
    background: var(--bg);
    box-shadow: 0 0 0 1px var(--line);
    color: var(--mut);
  }

  .qa-support {
    padding: 4px 10px 0;
    color: var(--dim);
    font-size: 12px;
  }

  .qa-label {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: center;
  }

  .qa-details {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 4px 14px;
    padding-top: 3px;
  }

  .qa-detail {
    display: inline-flex;
    gap: 5px;
    min-width: 0;
  }

  .qa-detail b {
    min-width: 0;
    color: var(--mut);
    font-weight: 500;
    overflow-wrap: anywhere;
  }

  .qa-meta {
    display: flex;
    justify-content: space-between;
    padding: 8px 20px 4px;
    font-size: 11px;
  }
</style>