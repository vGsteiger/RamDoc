<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { SubstanceDetail } from '$lib/api';

  interface Props {
    substanceId: string | null;
  }

  let { substanceId }: Props = $props();

  let detail = $state<SubstanceDetail | null>(null);
  let loading = $state(false);
  let collapsed = $state(false);

  $effect(() => {
    if (!substanceId) {
      detail = null;
      return;
    }

    loading = true;
    collapsed = false;
    const currentSubstanceId = substanceId;
    invoke<SubstanceDetail>('get_medication_reference_detail', { id: substanceId })
      .then((d) => {
        if (currentSubstanceId === substanceId) {
          detail = d;
        }
      })
      .catch(() => {
        if (currentSubstanceId === substanceId) {
          detail = null;
        }
      })
      .finally(() => {
        if (currentSubstanceId === substanceId) {
          loading = false;
        }
      });
  });
</script>

{#if loading}
  <div class="mt-2 px-3 py-2 bg-gray-700 rounded-lg text-xs text-gray-400 animate-pulse">
    Compendium-Daten werden geladen…
  </div>
{:else if detail}
  <div class="mt-2 bg-gray-700 border border-gray-600 rounded-lg overflow-hidden text-xs">
    <!-- Header -->
    <button
      type="button"
      class="w-full flex items-center justify-between px-3 py-2 text-left hover:bg-gray-600 transition-colors"
      onclick={() => (collapsed = !collapsed)}
    >
      <div class="flex items-center gap-2">
        <span class="font-semibold text-gray-200">{detail.name_de}</span>
        {#if detail.atc_code}
          <span class="font-mono bg-blue-900 text-blue-300 px-1.5 py-0.5 rounded">
            {detail.atc_code}
          </span>
        {/if}
        {#if detail.trade_names.length > 0}
          <span class="text-gray-400">{detail.trade_names.slice(0, 3).join(' · ')}</span>
        {/if}
      </div>
      <span class="text-gray-400 text-xs">{collapsed ? '▸' : '▾'}</span>
    </button>

    {#if !collapsed}
      <div class="px-3 pb-3 space-y-2 border-t border-gray-600">
        {#if detail.indication}
          <div class="pt-2">
            <p class="font-semibold text-gray-300 mb-0.5">Indikation</p>
            <p class="text-gray-400 leading-relaxed">{detail.indication}</p>
          </div>
        {/if}

        {#if detail.side_effects}
          <div>
            <p class="font-semibold text-amber-400 mb-0.5">Nebenwirkungen</p>
            <p class="text-gray-400 leading-relaxed">{detail.side_effects}</p>
          </div>
        {/if}

        {#if detail.contraindications}
          <div>
            <p class="font-semibold text-red-400 mb-0.5">Kontraindikationen</p>
            <p class="text-gray-400 leading-relaxed">{detail.contraindications}</p>
          </div>
        {/if}

        {#if detail.source_version}
          <p class="text-gray-500 pt-1">Quelle: Swissmedic AIPS {detail.source_version}</p>
        {/if}
      </div>
    {/if}
  </div>
{/if}
