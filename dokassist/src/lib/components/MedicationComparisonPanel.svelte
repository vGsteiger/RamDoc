<script lang="ts">
  import type { SubstanceDetail } from '$lib/api';

  interface Props {
    current: SubstanceDetail;
    replacement: SubstanceDetail;
  }

  let { current, replacement }: Props = $props();
</script>

<div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6">
  <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100 mb-4">
    Medikamentenvergleich
  </h3>

  <div class="grid grid-cols-2 gap-6">
    <!-- Current Medication Column -->
    <div class="border-r border-gray-200 dark:border-gray-700 pr-4">
      <div class="mb-4">
        <h4 class="text-sm font-medium text-gray-500 dark:text-gray-400 mb-1">
          Aktuelles Medikament
        </h4>
        <p class="text-lg font-semibold text-gray-900 dark:text-gray-100">
          {current.name_de}
        </p>
        {#if current.atc_code}
          <p class="text-sm text-gray-600 dark:text-gray-300">ATC: {current.atc_code}</p>
        {/if}
      </div>

      {#if current.trade_names && current.trade_names.length > 0}
        <div class="mb-4">
          <h5 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Handelsnamen</h5>
          <p class="text-sm text-gray-600 dark:text-gray-400">
            {current.trade_names.join(', ')}
          </p>
        </div>
      {/if}

      {#if current.indication}
        <div class="mb-4">
          <h5 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Indikation</h5>
          <p class="text-sm text-gray-600 dark:text-gray-400 line-clamp-4">
            {current.indication}
          </p>
        </div>
      {/if}

      {#if current.side_effects}
        <div class="mb-4">
          <h5 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Nebenwirkungen</h5>
          <p class="text-sm text-gray-600 dark:text-gray-400 line-clamp-4">
            {current.side_effects}
          </p>
        </div>
      {/if}

      {#if current.contraindications}
        <div class="mb-4">
          <h5 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Kontraindikationen
          </h5>
          <p class="text-sm text-gray-600 dark:text-gray-400 line-clamp-4">
            {current.contraindications}
          </p>
        </div>
      {/if}
    </div>

    <!-- Replacement Medication Column -->
    <div class="pl-4">
      <div class="mb-4">
        <h4 class="text-sm font-medium text-gray-500 dark:text-gray-400 mb-1">
          Neues Medikament
        </h4>
        <p class="text-lg font-semibold text-gray-900 dark:text-gray-100">
          {replacement.name_de}
        </p>
        {#if replacement.atc_code}
          <p class="text-sm text-gray-600 dark:text-gray-300">ATC: {replacement.atc_code}</p>
        {/if}
      </div>

      {#if replacement.trade_names && replacement.trade_names.length > 0}
        <div class="mb-4">
          <h5 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Handelsnamen</h5>
          <p class="text-sm text-gray-600 dark:text-gray-400">
            {replacement.trade_names.join(', ')}
          </p>
        </div>
      {/if}

      {#if replacement.indication}
        <div class="mb-4">
          <h5 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Indikation</h5>
          <p class="text-sm text-gray-600 dark:text-gray-400 line-clamp-4">
            {replacement.indication}
          </p>
        </div>
      {/if}

      {#if replacement.side_effects}
        <div class="mb-4">
          <h5 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Nebenwirkungen</h5>
          <p class="text-sm text-gray-600 dark:text-gray-400 line-clamp-4">
            {replacement.side_effects}
          </p>
        </div>
      {/if}

      {#if replacement.contraindications}
        <div class="mb-4">
          <h5 class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Kontraindikationen
          </h5>
          <p class="text-sm text-gray-600 dark:text-gray-400 line-clamp-4">
            {replacement.contraindications}
          </p>
        </div>
      {/if}
    </div>
  </div>

  <!-- Comparison Notes -->
  {#if current.atc_code === replacement.atc_code && current.atc_code}
    <div class="mt-4 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded">
      <p class="text-sm text-blue-800 dark:text-blue-300">
        ℹ️ Beide Medikamente haben denselben ATC-Code und gehören zur gleichen pharmakologischen
        Klasse.
      </p>
    </div>
  {/if}
</div>
