<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  interface Props {
    value?: string;
    error?: string;
  }

  let { value = $bindable(''), error = '' }: Props = $props();

  const dispatch = createEventDispatcher<{ input: string; blur: void }>();

  let displayValue = $state('');
  let isValid = $state(true);
  let validationError = $state('');

  // Initialize display value from prop
  $effect(() => {
    if (value && !displayValue) {
      displayValue = formatAhv(value);
    }
  });

  function formatAhv(ahv: string): string {
    // Remove all non-digit characters
    const digits = ahv.replace(/\D/g, '');

    // Format as 756.XXXX.XXXX.XX
    if (digits.length >= 13) {
      return `${digits.slice(0, 3)}.${digits.slice(3, 7)}.${digits.slice(7, 11)}.${digits.slice(11, 13)}`;
    } else if (digits.length >= 11) {
      return `${digits.slice(0, 3)}.${digits.slice(3, 7)}.${digits.slice(7, 11)}.${digits.slice(11)}`;
    } else if (digits.length >= 7) {
      return `${digits.slice(0, 3)}.${digits.slice(3, 7)}.${digits.slice(7)}`;
    } else if (digits.length >= 3) {
      return `${digits.slice(0, 3)}.${digits.slice(3)}`;
    }
    return digits;
  }

  function validateAhvChecksum(ahv: string): boolean {
    // Remove dots and validate format
    const digits = ahv.replace(/\D/g, '');

    if (digits.length !== 13) {
      return false;
    }

    if (!digits.startsWith('756')) {
      return false;
    }

    // EAN-13 checksum validation
    const sum = digits
      .slice(0, 12)
      .split('')
      .reduce((acc, digit, idx) => {
        const d = parseInt(digit, 10);
        return acc + (idx % 2 === 0 ? d : d * 3);
      }, 0);

    const checksum = (10 - (sum % 10)) % 10;
    return checksum === parseInt(digits[12], 10);
  }

  function handleInput(e: Event) {
    const input = e.target as HTMLInputElement;
    const raw = input.value;

    // Format as user types
    displayValue = formatAhv(raw);

    // Get plain digits for validation and binding
    const digits = raw.replace(/\D/g, '');

    // Always update bound value with plain digits and dispatch input
    value = digits;
    dispatch('input', digits);

    // Now validate based on the current digits
    if (digits.length === 0) {
      validationError = '';
      isValid = true;
    } else if (digits.length < 13) {
      validationError = `Enter 13 digits (${digits.length}/13)`;
      isValid = false;
    } else if (digits.length === 13) {
      isValid = validateAhvChecksum(digits);
      if (!isValid) {
        validationError = 'Invalid AHV checksum';
      } else {
        validationError = '';
      }
    } else {
      // More than 13 digits is invalid
      validationError = 'AHV must contain exactly 13 digits';
      isValid = false;
    }
  }

  function handleBlur() {
    dispatch('blur');
  }
</script>

<div class="w-full">
  <input
    type="text"
    bind:value={displayValue}
    oninput={handleInput}
    onblur={handleBlur}
    placeholder="756.____.____.__ "
    class="w-full px-4 py-2 bg-white dark:bg-gray-800 border rounded-lg text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:border-blue-500 {!isValid &&
    displayValue
      ? 'border-red-500'
      : error
        ? 'border-red-500'
        : 'border-gray-300 dark:border-gray-700'}"
    maxlength="16"
  />
  {#if validationError && displayValue}
    <p class="mt-1 text-sm text-red-400">{validationError}</p>
  {:else if error}
    <p class="mt-1 text-sm text-red-400">{error}</p>
  {/if}
</div>
