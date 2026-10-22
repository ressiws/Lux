<script lang="ts">
  interface Props {
    open: boolean;
    title: string;
    message: string;
    confirmLabel?: string;
    cancelLabel?: string;
    isDanger?: boolean;
    isSubmitting?: boolean;
    onConfirm: () => void | Promise<void>;
    onCancel: () => void;
  }

  const {
    open = false,
    title,
    message,
    confirmLabel = "Confirm",
    cancelLabel = "Cancel",
    isDanger = false,
    isSubmitting = false,
    onConfirm,
    onCancel,
  }: Props = $props();

  let confirmButtonElement = $state<HTMLButtonElement | null>(null);
  let cancelButtonElement = $state<HTMLButtonElement | null>(null);

  function handleKeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      if (!isSubmitting) {
        onCancel();
      }
    } else if (e.key === "Enter" && !isSubmitting) {
      e.preventDefault();
      e.stopPropagation();
      void onConfirm();
    }
  }

  $effect(() => {
    if (open) {
      setTimeout(() => {
        if (isDanger) {
          cancelButtonElement?.focus();
        } else {
          confirmButtonElement?.focus();
        }
      }, 30);
    }
  });
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="dialog-overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget && !isSubmitting) {
        onCancel();
      }
    }}
    onkeydown={handleKeydown}
  >
    <div
      class="dialog-box"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="dialog-title"
      aria-describedby="dialog-desc"
    >
      <div class="dialog-header">
        {#if isDanger}
          <div class="dialog-icon danger" aria-hidden="true">
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path
                d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"
              />
              <line x1="12" y1="9" x2="12" y2="13" />
              <line x1="12" y1="17" x2="12.01" y2="17" />
            </svg>
          </div>
        {/if}
        <h3 id="dialog-title" class="dialog-title">{title}</h3>
      </div>
      <p id="dialog-desc" class="dialog-desc">{message}</p>
      <div class="dialog-actions">
        <button
          type="button"
          class="dialog-btn btn-secondary"
          bind:this={cancelButtonElement}
          disabled={isSubmitting}
          onclick={onCancel}
        >
          {cancelLabel}
        </button>
        <button
          type="button"
          class="dialog-btn btn-primary"
          class:danger={isDanger}
          bind:this={confirmButtonElement}
          disabled={isSubmitting}
          onclick={() => void onConfirm()}
        >
          {#if isSubmitting}
            <span class="spinner" aria-hidden="true"></span>
          {/if}
          {confirmLabel}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    animation: fadeIn 0.12s ease-out;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  .dialog-box {
    background: var(--bg, #18181b);
    border: 1px solid var(--line, rgba(255, 255, 255, 0.1));
    border-radius: 14px;
    box-shadow: var(--shadow, 0 20px 40px rgba(0, 0, 0, 0.4));
    width: 360px;
    max-width: calc(100% - 32px);
    padding: 20px 22px;
    color: var(--fg, #f4f4f5);
    animation: scaleIn 0.14s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes scaleIn {
    from {
      transform: scale(0.96);
      opacity: 0;
    }
    to {
      transform: scale(1);
      opacity: 1;
    }
  }

  .dialog-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
  }

  .dialog-icon.danger {
    color: #e5484d;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .dialog-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--fg, #f4f4f5);
    letter-spacing: -0.01em;
  }

  .dialog-desc {
    margin: 0 0 18px;
    font-size: 13px;
    line-height: 1.45;
    color: var(--mut, #a1a1aa);
  }

  .dialog-actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 10px;
  }

  .dialog-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 32px;
    padding: 0 14px;
    border-radius: 8px;
    font-size: 12.5px;
    font-weight: 500;
    cursor: default;
    border: none;
    transition: background 0.12s, opacity 0.12s;
  }

  .dialog-btn:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .btn-secondary {
    background: var(--chip, rgba(255, 255, 255, 0.08));
    border: 1px solid var(--line, rgba(255, 255, 255, 0.1));
    color: var(--fg, #f4f4f5);
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--sel, rgba(255, 255, 255, 0.12));
  }

  .btn-primary {
    background: var(--fg, #f4f4f5);
    color: var(--bg, #18181b);
    font-weight: 600;
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.92;
  }

  .btn-primary.danger {
    background: #e5484d;
    color: #ffffff;
  }

  .btn-primary.danger:hover:not(:disabled) {
    background: #d43b40;
  }

  .spinner {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #ffffff;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
