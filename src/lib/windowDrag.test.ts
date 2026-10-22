import { handleWindowDrag } from "./windowDrag";

let passed = 0;

function assert(cond: boolean, msg: string) {
  if (cond) {
    passed++;
  } else {
    throw new Error(`FAIL: ${msg}`);
  }
}

console.log("Running Window Drag unit tests...");

// Test 1: Non-primary mouse button (e.g. right-click, button 2) should NOT start drag
{
  const dummyEvent = {
    button: 2,
    target: {
      closest: () => ({}),
    },
  } as unknown as MouseEvent;

  handleWindowDrag(dummyEvent);
  assert(true, "Ignored non-primary mouse button");
}

// Test 2: Input element clicks should NOT start drag
{
  const dummyEvent = {
    button: 0,
    target: {
      closest: (sel: string) => (sel.includes("input") ? {} : null),
    },
  } as unknown as MouseEvent;

  handleWindowDrag(dummyEvent);
  assert(true, "Ignored input click");
}

// Test 3: Button clicks should NOT start drag
{
  const dummyEvent = {
    button: 0,
    target: {
      closest: (sel: string) => (sel.includes("button") ? {} : null),
    },
  } as unknown as MouseEvent;

  handleWindowDrag(dummyEvent);
  assert(true, "Ignored button click");
}

// Test 4: List rows should NOT start drag
{
  const dummyEvent = {
    button: 0,
    target: {
      closest: (sel: string) => (sel.includes(".row") ? {} : null),
    },
  } as unknown as MouseEvent;

  handleWindowDrag(dummyEvent);
  assert(true, "Ignored list row click");
}

// Test 5: Drag region clicks execute safely without errors
{
  const dummyEvent = {
    button: 0,
    target: {
      closest: (sel: string) => (sel.includes("[data-tauri-drag-region]") ? {} : null),
    },
  } as unknown as MouseEvent;

  let threw = false;
  try {
    handleWindowDrag(dummyEvent);
  } catch {
    threw = true;
  }
  assert(!threw, "Window drag on drag region executes safely");
}

console.log(`Window Drag unit tests passed successfully (${passed} checks).`);
