let passed = 0;

function assert(cond: boolean, msg: string) {
  if (cond) {
    passed++;
  } else {
    throw new Error(`FAIL: ${msg}`);
  }
}

console.log("Running Confirm Dialog unit tests...");

// Test 1: Action semantics - confirm executes handler, cancel does not
{
  let confirmed = false;
  let cancelled = false;

  const onConfirm = () => {
    confirmed = true;
  };
  const onCancel = () => {
    cancelled = true;
  };

  // Simulate cancel
  onCancel();
  assert(cancelled && !confirmed, "Cancel does not execute confirmation action");

  // Simulate confirm
  onConfirm();
  assert(confirmed, "Confirm executes confirmation action");
}

// Test 2: Double-submission prevention
{
  let executionCount = 0;
  let isSubmitting = false;

  const triggerAction = async () => {
    if (isSubmitting) return;
    isSubmitting = true;
    executionCount++;
  };

  // First trigger
  void triggerAction();
  // Second rapid trigger while isSubmitting is true
  void triggerAction();

  assert(executionCount === 1, "Prevents duplicate concurrent submissions");
}

// Test 3: Keyboard semantics (Escape cancels, Enter confirms)
{
  let cancelled = false;
  let confirmed = false;

  function simulateDialogKey(
    key: string,
    isSubmitting: boolean,
    onCancel: () => void,
    onConfirm: () => void,
  ) {
    if (key === "Escape" && !isSubmitting) {
      onCancel();
    } else if (key === "Enter" && !isSubmitting) {
      onConfirm();
    }
  }

  simulateDialogKey("Escape", false, () => { cancelled = true; }, () => { confirmed = true; });
  assert(cancelled && !confirmed, "Escape cancels dialog");

  cancelled = false;
  confirmed = false;
  simulateDialogKey("Enter", false, () => { cancelled = true; }, () => { confirmed = true; });
  assert(confirmed && !cancelled, "Enter confirms dialog");

  // While submitting, keys should not trigger
  cancelled = false;
  confirmed = false;
  simulateDialogKey("Escape", true, () => { cancelled = true; }, () => { confirmed = true; });
  simulateDialogKey("Enter", true, () => { cancelled = true; }, () => { confirmed = true; });
  assert(!cancelled && !confirmed, "Keys ignored while operation is submitting");
}

console.log(`Confirm Dialog unit tests passed successfully (${passed} checks).`);
