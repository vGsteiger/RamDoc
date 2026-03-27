<script lang="ts">
  import { goto } from '$app/navigation';
  import { ChevronRight, ChevronLeft, Upload, FileText, Database } from 'lucide-svelte';

  function handleContinue() {
    goto('/onboarding/step4');
  }

  function handleBack() {
    goto('/onboarding/step2');
  }

  function handleImportLater() {
    goto('/onboarding/step4');
  }
</script>

<div class="min-h-screen bg-gray-950 flex items-center justify-center p-8">
  <div class="max-w-3xl w-full">
    <div class="mb-8 text-center">
      <h1 class="text-3xl font-bold text-gray-100 mb-2">Import Existing Data</h1>
      <p class="text-gray-400">
        Optionally import patient data from CSV files or a DokAssist backup. You can also do
        this later from Settings.
      </p>
      <div class="flex items-center justify-center gap-2 mt-4">
        <div class="h-2 w-16 bg-blue-600 rounded-full"></div>
        <div class="h-2 w-16 bg-blue-600 rounded-full"></div>
        <div class="h-2 w-16 bg-blue-600 rounded-full"></div>
        <div class="h-2 w-16 bg-gray-700 rounded-full"></div>
      </div>
    </div>

    <div class="bg-gray-900 border border-gray-800 rounded-lg p-8 space-y-6">
      <div class="text-center">
        <div class="inline-block p-4 bg-gray-800 rounded-full mb-4">
          <Database size={48} class="text-gray-400" />
        </div>
        <h2 class="text-xl font-bold text-gray-100 mb-2">Data Import Options</h2>
        <p class="text-gray-400">Choose how you'd like to import your data</p>
      </div>

      <div class="space-y-4">
        <button
          onclick={() => goto('/settings')}
          class="w-full bg-gray-800 hover:bg-gray-750 border border-gray-700 rounded-lg p-6 text-left transition-colors group"
        >
          <div class="flex items-start gap-4">
            <div class="flex-shrink-0 w-12 h-12 bg-blue-900/20 rounded-lg flex items-center justify-center group-hover:bg-blue-900/30 transition-colors">
              <FileText size={24} class="text-blue-500" />
            </div>
            <div class="flex-1">
              <h3 class="text-gray-100 font-semibold mb-1">Import from CSV</h3>
              <p class="text-gray-400 text-sm">
                Import patient records from a CSV file with automatic column detection for
                German, French, and English headers.
              </p>
            </div>
            <ChevronRight size={20} class="text-gray-600 group-hover:text-gray-400 transition-colors" />
          </div>
        </button>

        <button
          onclick={() => goto('/settings')}
          class="w-full bg-gray-800 hover:bg-gray-750 border border-gray-700 rounded-lg p-6 text-left transition-colors group"
        >
          <div class="flex items-start gap-4">
            <div class="flex-shrink-0 w-12 h-12 bg-green-900/20 rounded-lg flex items-center justify-center group-hover:bg-green-900/30 transition-colors">
              <Database size={24} class="text-green-500" />
            </div>
            <div class="flex-1">
              <h3 class="text-gray-100 font-semibold mb-1">Restore from Backup</h3>
              <p class="text-gray-400 text-sm">
                Restore a complete DokAssist backup archive including patients, sessions,
                and encrypted files.
              </p>
            </div>
            <ChevronRight size={20} class="text-gray-600 group-hover:text-gray-400 transition-colors" />
          </div>
        </button>
      </div>

      <div class="bg-yellow-900/20 border border-yellow-800 rounded-lg p-4">
        <p class="text-yellow-400 text-sm">
          <strong>Note:</strong> You can skip this step and import data later from the Settings
          page. The import features will always be available.
        </p>
      </div>
    </div>

    <div class="flex justify-between items-center mt-8">
      <button
        onclick={handleBack}
        class="px-6 py-3 bg-gray-700 hover:bg-gray-600 text-white font-medium rounded-lg transition-colors flex items-center gap-2"
      >
        <ChevronLeft size={20} />
        Back
      </button>

      <button
        onclick={handleImportLater}
        class="px-6 py-3 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors flex items-center gap-2"
      >
        Skip for now
        <ChevronRight size={20} />
      </button>
    </div>
  </div>
</div>
