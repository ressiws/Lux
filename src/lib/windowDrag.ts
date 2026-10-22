import { getCurrentWindow } from "@tauri-apps/api/window";

const DRAG_THRESHOLD_PX = 4;

/**
 * Starts window dragging once the pointer moves past a small threshold while the primary button
 * is held. A plain click therefore still focuses the input instead of entering a window move.
 */
function startDraggingOnMove(e: MouseEvent): void {
  const startX = e.clientX;
  const startY = e.clientY;
  const stop = (): void => {
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mouseup", stop);
  };
  const onMove = (m: MouseEvent): void => {
    if ((m.buttons & 1) === 0) {
      stop();
      return;
    }
    if (Math.hypot(m.clientX - startX, m.clientY - startY) >= DRAG_THRESHOLD_PX) {
      stop();
      try {
        void getCurrentWindow().startDragging();
      } catch {
        // Graceful fallback outside Tauri or in test environment
      }
    }
  };
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mouseup", stop);
}

/**
 * Handles mousedown on draggable window surfaces (title bar, footer, non-interactive padding).
 * Initiates window dragging if the left mouse button was pressed, the target is within a designated
 * drag surface, and the click target is not an interactive element (input, button, link, selectable list item, etc.).
 */
export function handleWindowDrag(e: MouseEvent): void {
  // Only respond to primary (left) mouse button
  if (e.button !== 0) return;

  const target = e.target as HTMLElement | null;
  if (!target) return;

  // An empty search input has no text to select, so dragging from it moves the window.
  const searchInput = target.closest("#q") as HTMLInputElement | null;
  if (searchInput && searchInput.value === "" && typeof window !== "undefined") {
    startDraggingOnMove(e);
    return;
  }

  // Never drag when interacting with interactive controls, text inputs, or list items
  if (
    target.closest(
      "input, textarea, button, select, a, [role='option'], [role='button'], [role='tab'], .btn, .interactive, .row, #list",
    )
  ) {
    return;
  }

  // Ensure click is within a designated draggable surface (.bar, .foot, or data-tauri-drag-region)
  if (target.closest("[data-tauri-drag-region], .bar, .foot")) {
    try {
      void getCurrentWindow().startDragging();
    } catch {
      // Graceful fallback outside Tauri or in test environment
    }
  }
}

/**
 * Attaches the window drag listener to window and returns an unregister cleanup function.
 */
export function registerWindowDrag(): () => void {
  if (typeof window === "undefined") {
    return () => {};
  }
  window.addEventListener("mousedown", handleWindowDrag);
  return () => {
    window.removeEventListener("mousedown", handleWindowDrag);
  };
}
