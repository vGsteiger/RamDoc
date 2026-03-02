<script lang="ts">
  import type { Patient } from '$lib/api';

  interface Props {
    patient: Patient;
    onclick?: () => void;
  }

  let { patient, onclick }: Props = $props();

  function formatDate(dateStr: string): string {
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString('de-CH', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit'
      });
    } catch {
      return dateStr;
    }
  }

  function calculateAge(dateOfBirth: string): number {
    const birth = new Date(dateOfBirth);
    const today = new Date();
    let age = today.getFullYear() - birth.getFullYear();
    const monthDiff = today.getMonth() - birth.getMonth();
    if (monthDiff < 0 || (monthDiff === 0 && today.getDate() < birth.getDate())) {
      age--;
    }
    return age;
  }
</script>

<button
  onclick={onclick}
  class="w-full text-left p-4 bg-gray-800 border border-gray-700 rounded-lg hover:bg-gray-750 hover:border-gray-600 transition-colors"
>
  <div class="flex justify-between items-start mb-2">
    <div>
      <h3 class="text-lg font-semibold text-gray-100">
        {patient.last_name}, {patient.first_name}
      </h3>
      <p class="text-sm text-gray-400">
        AHV: {patient.ahv_number}
      </p>
    </div>
    <div class="text-right">
      <p class="text-sm text-gray-400">
        {formatDate(patient.date_of_birth)}
      </p>
      <p class="text-xs text-gray-500">
        Age: {calculateAge(patient.date_of_birth)}
      </p>
    </div>
  </div>

  {#if patient.gender}
    <div class="flex gap-2 items-center">
      <span class="text-xs px-2 py-1 bg-gray-700 text-gray-300 rounded">
        {patient.gender}
      </span>
    </div>
  {/if}

  {#if patient.insurance}
    <div class="mt-2 text-sm text-gray-400">
      Insurance: {patient.insurance}
    </div>
  {/if}
</button>
