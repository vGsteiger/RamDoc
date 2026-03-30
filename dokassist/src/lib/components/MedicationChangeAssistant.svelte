<script lang="ts">
  import { onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { getOrCreatePatientChatSession, runAgentTurn, type SubstanceDetail } from '$lib/api';
  import MedicationComparisonPanel from './MedicationComparisonPanel.svelte';

  interface Props {
    patientId: string;
    currentSubstance: SubstanceDetail;
    replacementSubstance: SubstanceDetail;
  }

  let { patientId, currentSubstance, replacementSubstance }: Props = $props();

  let aiGuidance = $state('');
  let isGenerating = $state(false);
  let error = $state<string | null>(null);
  let sessionId = $state<string | null>(null);

  // Track active listeners so they can be cleaned up on unmount.
  let activeUnlisteners: UnlistenFn[] = [];

  function cleanupListeners() {
    activeUnlisteners.forEach((fn) => fn());
    activeUnlisteners = [];
  }

  onDestroy(cleanupListeners);

  async function generateGuidance() {
    try {
      isGenerating = true;
      error = null;
      aiGuidance = '';

      // Clean up any listeners from a previous run.
      cleanupListeners();

      // Create or get the chat session for this patient
      const session = await getOrCreatePatientChatSession(patientId);
      sessionId = session.id;

      // Set up event listeners for streaming
      const chunkUnlisten = await listen<string>('agent-chunk', (event) => {
        aiGuidance += event.payload;
      });

      const doneUnlisten = await listen<void>('agent-done', () => {
        isGenerating = false;
        cleanupListeners();
      });

      activeUnlisteners = [chunkUnlisten, doneUnlisten];

      // Construct a prompt that asks the agent to compare the medications
      const prompt = `Bitte vergleiche die folgenden beiden Medikamente und gib eine Entscheidungshilfe für den Medikamentenwechsel:

Aktuelles Medikament: ${currentSubstance.name_de} (ID: ${currentSubstance.id})
Neues Medikament: ${replacementSubstance.name_de} (ID: ${replacementSubstance.id})

Nutze das compare_medications Tool, um detaillierte Informationen zu beiden Medikamenten abzurufen, und erstelle dann eine Zusammenfassung mit folgenden Punkten:

1. Gemeinsamkeiten und Unterschiede in der Indikation
2. Vergleich der Nebenwirkungen (überlappend und unterschiedlich)
3. Kontraindikationen, die beachtet werden müssen
4. Empfehlung für den Medikamentenwechsel
5. Wichtige Punkte für das Monitoring nach dem Wechsel`;

      // Send the message to the agent
      await runAgentTurn(sessionId, prompt);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
      isGenerating = false;
      cleanupListeners();
    }
  }
</script>

<div class="space-y-6">
  <!-- Comparison Panel -->
  <MedicationComparisonPanel current={currentSubstance} replacement={replacementSubstance} />

  <!-- AI Guidance Section -->
  <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-6">
    <div class="flex items-center justify-between mb-4">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
        KI-gestützte Entscheidungshilfe
      </h3>
      <button
        onclick={generateGuidance}
        disabled={isGenerating}
        class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:bg-gray-400 disabled:cursor-not-allowed"
      >
        {isGenerating ? 'Generiert...' : 'Entscheidungshilfe generieren'}
      </button>
    </div>

    {#if error}
      <div class="mb-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded">
        <p class="text-sm text-red-800 dark:text-red-300">{error}</p>
      </div>
    {/if}

    {#if aiGuidance || isGenerating}
      <div class="prose dark:prose-invert max-w-none">
        <div
          class="whitespace-pre-wrap text-sm text-gray-700 dark:text-gray-300 bg-gray-50 dark:bg-gray-900 rounded p-4"
        >
          {aiGuidance || 'Generiere Entscheidungshilfe...'}
        </div>
      </div>
    {:else}
      <p class="text-sm text-gray-500 dark:text-gray-400">
        Klicken Sie auf "Entscheidungshilfe generieren", um eine KI-gestützte Analyse des
        Medikamentenwechsels zu erhalten. Die KI wird die Kompendiumdaten beider Medikamente
        analysieren und eine fundierte Empfehlung geben.
      </p>
    {/if}
  </div>
</div>
