<script lang="ts">
  import type { LauncherItem } from "./types";

  interface Props {
    item: LauncherItem;
    active: boolean;
    onToggle: (item: LauncherItem) => void;
  }

  let { item, active, onToggle }: Props = $props();
</script>

<button
  class="fav-toggle"
  class:active
  type="button"
  aria-label={active
    ? `Remove ${item.name} from Favorites`
    : `Add ${item.name} to Favorites`}
  aria-pressed={active}
  title={active ? "Remove from Favorites" : "Add to Favorites"}
  onmousedown={(event) => {
    event.preventDefault();
    event.stopPropagation();
  }}
  onclick={(event) => {
    event.stopPropagation();
    onToggle(item);
  }}
>
  <svg
    width="15"
    height="15"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.7"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="m12 3 2.8 5.7 6.2.9-4.5 4.4 1.1 6.2-5.6-2.9-5.6 2.9 1.1-6.2L3 9.6l6.2-.9z" />
  </svg>
</button>

<style>
  .fav-toggle {
    display: grid;
    width: 26px;
    height: 26px;
    flex: none;
    padding: 0;
    place-items: center;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--dim);
    cursor: pointer;
  }

  .fav-toggle:hover {
    background: var(--chip);
    color: var(--fg);
  }

  .fav-toggle:focus-visible {
    outline: 2px solid var(--fg);
    outline-offset: 1px;
    color: var(--fg);
  }

  .fav-toggle.active {
    color: var(--fg);
  }

  .fav-toggle.active svg {
    fill: currentColor;
  }
</style>
