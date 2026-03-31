<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { getSettings, updateSettings, parseError, type PracticeSettings } from '$lib/api';
  import { ChevronRight, Building, User, Mail, Phone, MapPin, Globe } from 'lucide-svelte';

  let settings = $state<PracticeSettings>({
    practice_name: null,
    practice_address: null,
    practice_phone: null,
    practice_email: null,
    therapist_name: null,
    zsr_number: null,
    canton: null,
    clinical_specialty: null,
    language_preference: 'de',
    onboarding_completed: false,
  });

  let error = $state<string | null>(null);
  let isLoading = $state(false);
  let isSaving = $state(false);

  // Swiss cantons for dropdown
  const cantons = [
    { code: 'AG', name: 'Aargau' },
    { code: 'AI', name: 'Appenzell Innerrhoden' },
    { code: 'AR', name: 'Appenzell Ausserrhoden' },
    { code: 'BE', name: 'Bern' },
    { code: 'BL', name: 'Basel-Landschaft' },
    { code: 'BS', name: 'Basel-Stadt' },
    { code: 'FR', name: 'Fribourg' },
    { code: 'GE', name: 'Geneva' },
    { code: 'GL', name: 'Glarus' },
    { code: 'GR', name: 'Graubünden' },
    { code: 'JU', name: 'Jura' },
    { code: 'LU', name: 'Lucerne' },
    { code: 'NE', name: 'Neuchâtel' },
    { code: 'NW', name: 'Nidwalden' },
    { code: 'OW', name: 'Obwalden' },
    { code: 'SG', name: 'St. Gallen' },
    { code: 'SH', name: 'Schaffhausen' },
    { code: 'SO', name: 'Solothurn' },
    { code: 'SZ', name: 'Schwyz' },
    { code: 'TG', name: 'Thurgau' },
    { code: 'TI', name: 'Ticino' },
    { code: 'UR', name: 'Uri' },
    { code: 'VD', name: 'Vaud' },
    { code: 'VS', name: 'Valais' },
    { code: 'ZG', name: 'Zug' },
    { code: 'ZH', name: 'Zürich' },
  ];

  onMount(async () => {
    isLoading = true;
    try {
      settings = await getSettings();
    } catch (err) {
      console.error('Failed to load settings:', err);
    } finally {
      isLoading = false;
    }
  });

  async function handleContinue() {
    isSaving = true;
    error = null;

    try {
      await updateSettings({ ...settings, canton: settings.canton || null });
      goto('/onboarding/step2');
    } catch (err) {
      error = parseError(err).message;
    } finally {
      isSaving = false;
    }
  }

  function handleSkip() {
    goto('/onboarding/step2');
  }
</script>

<div class="min-h-screen bg-gray-950 flex items-center justify-center p-8">
  <div class="max-w-3xl w-full">
    <div class="mb-8 text-center">
      <h1 class="text-3xl font-bold text-gray-100 mb-2">Configure Practice Details</h1>
      <p class="text-gray-400">
        Set up your practice information. This will be used for billing and patient
        documentation.
      </p>
      <div class="flex items-center justify-center gap-2 mt-4">
        <div class="h-2 w-16 bg-blue-600 rounded-full"></div>
        <div class="h-2 w-16 bg-gray-700 rounded-full"></div>
        <div class="h-2 w-16 bg-gray-700 rounded-full"></div>
        <div class="h-2 w-16 bg-gray-700 rounded-full"></div>
      </div>
    </div>

    {#if error}
      <div class="bg-red-900/20 border border-red-500 rounded-lg p-4 mb-6">
        <p class="text-red-500 text-sm">{error}</p>
      </div>
    {/if}

    {#if isLoading}
      <div class="text-center py-12">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mx-auto"></div>
        <p class="mt-4 text-gray-400">Loading settings...</p>
      </div>
    {:else}
      <div class="bg-gray-900 border border-gray-800 rounded-lg p-8 space-y-6">
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div>
            <label for="practice-name" class="flex items-center gap-2 text-gray-400 mb-2">
              <Building size={16} />
              Practice Name
            </label>
            <input
              id="practice-name"
              type="text"
              bind:value={settings.practice_name}
              placeholder="Enter practice name"
              class="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div>
            <label for="therapist-name" class="flex items-center gap-2 text-gray-400 mb-2">
              <User size={16} />
              Therapist Name
            </label>
            <input
              id="therapist-name"
              type="text"
              bind:value={settings.therapist_name}
              placeholder="Enter your name"
              class="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
        </div>

        <div>
          <label for="practice-address" class="flex items-center gap-2 text-gray-400 mb-2">
            <MapPin size={16} />
            Practice Address
          </label>
          <textarea
            id="practice-address"
            bind:value={settings.practice_address}
            placeholder="Enter practice address"
            rows="3"
            class="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
          ></textarea>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div>
            <label for="practice-phone" class="flex items-center gap-2 text-gray-400 mb-2">
              <Phone size={16} />
              Phone
            </label>
            <input
              id="practice-phone"
              type="tel"
              bind:value={settings.practice_phone}
              placeholder="+41 XX XXX XX XX"
              class="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div>
            <label for="practice-email" class="flex items-center gap-2 text-gray-400 mb-2">
              <Mail size={16} />
              Email
            </label>
            <input
              id="practice-email"
              type="email"
              bind:value={settings.practice_email}
              placeholder="practice@example.com"
              class="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div>
            <label for="zsr-number" class="text-gray-400 mb-2 block">
              ZSR Number <span class="text-gray-600">(for TARMED billing)</span>
            </label>
            <input
              id="zsr-number"
              type="text"
              bind:value={settings.zsr_number}
              placeholder="ZSR number"
              class="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
          </div>

          <div>
            <label for="canton" class="text-gray-400 mb-2 block">
              Canton <span class="text-gray-600">(for Taxpunktwert)</span>
            </label>
            <select
              id="canton"
              value={settings.canton ?? ''}
              onchange={(e) => { settings.canton = e.currentTarget.value || null; }}
              class="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
            >
              <option value="">Select canton...</option>
              {#each cantons as canton}
                <option value={canton.code}>{canton.name} ({canton.code})</option>
              {/each}
            </select>
          </div>
        </div>

        <div>
          <label for="clinical-specialty" class="flex items-center gap-2 text-gray-400 mb-2">
            <Globe size={16} />
            Clinical Specialty
          </label>
          <input
            id="clinical-specialty"
            type="text"
            bind:value={settings.clinical_specialty}
            placeholder="e.g., Clinical Psychology, Psychiatry"
            class="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>
      </div>

      <div class="flex justify-between items-center mt-8">
        <button
          onclick={handleSkip}
          class="px-6 py-3 text-gray-400 hover:text-gray-300 font-medium transition-colors"
        >
          Skip for now
        </button>

        <button
          onclick={handleContinue}
          disabled={isSaving}
          class="px-6 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 disabled:cursor-not-allowed text-white font-medium rounded-lg transition-colors flex items-center gap-2"
        >
          {#if isSaving}
            Saving...
          {:else}
            Continue
            <ChevronRight size={20} />
          {/if}
        </button>
      </div>
    {/if}
  </div>
</div>
