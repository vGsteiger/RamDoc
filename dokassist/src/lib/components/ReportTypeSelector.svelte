<script lang="ts">
  export let selectedType: string = '';

  interface ReportTypeInfo {
    value: string;
    label: string;
    description: string;
  }

  const reportTypes: ReportTypeInfo[] = [
    {
      value: 'Befundbericht',
      label: 'Befundbericht',
      description: 'Comprehensive diagnostic report with findings and assessment'
    },
    {
      value: 'Verlaufsbericht',
      label: 'Verlaufsbericht',
      description: 'Progress report documenting treatment course over time'
    },
    {
      value: 'Ueberweisungsschreiben',
      label: 'Überweisungsschreiben',
      description: 'Referral letter to transfer care to another provider'
    }
  ];

  function selectType(type: string) {
    selectedType = type;
  }
</script>

<div class="space-y-4">
  <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">Select Report Type</h3>
  <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
    {#each reportTypes as type}
      <button
        on:click={() => selectType(type.value)}
        class="p-4 rounded-lg border-2 transition-all text-left {selectedType === type.value
          ? 'border-blue-500 bg-blue-900/20'
          : 'border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-800 hover:border-gray-500 dark:hover:border-gray-600'}"
      >
        <div class="font-semibold text-gray-900 dark:text-gray-100 mb-2">{type.label}</div>
        <div class="text-sm text-gray-500 dark:text-gray-400">{type.description}</div>
      </button>
    {/each}
  </div>
</div>
