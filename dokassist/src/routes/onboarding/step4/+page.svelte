<script lang="ts">
  import { goto } from '$app/navigation';
  import { completeOnboarding, parseError } from '$lib/api';
  import {
    ChevronLeft,
    Users,
    Mic,
    Search,
    FileText,
    Calendar,
    CheckCircle,
  } from 'lucide-svelte';

  let isCompleting = $state(false);
  let error = $state<string | null>(null);

  const colorClasses: Record<string, { bg: string; text: string }> = {
    blue: { bg: 'bg-blue-900/20', text: 'text-blue-500' },
    green: { bg: 'bg-green-900/20', text: 'text-green-500' },
    purple: { bg: 'bg-purple-900/20', text: 'text-purple-500' },
    yellow: { bg: 'bg-yellow-900/20', text: 'text-yellow-500' },
    red: { bg: 'bg-red-900/20', text: 'text-red-500' },
  };

  async function handleComplete() {
    isCompleting = true;
    error = null;

    try {
      await completeOnboarding();
      goto('/dashboard');
    } catch (err) {
      error = parseError(err).message;
      isCompleting = false;
    }
  }

  function handleBack() {
    goto('/onboarding/step3');
  }

  const features = [
    {
      icon: Users,
      title: 'Patient Management',
      description:
        'Create and manage patient records with comprehensive demographic information, diagnoses, medications, and treatment plans.',
      color: 'blue',
    },
    {
      icon: Calendar,
      title: 'Session Scheduling',
      description:
        'Schedule therapy sessions with calendar integration. Track session notes, AMDP data, and clinical observations.',
      color: 'green',
    },
    {
      icon: Mic,
      title: 'Session Recording',
      description:
        'Record therapy sessions and generate AI-powered summaries. All processing happens locally for maximum privacy.',
      color: 'purple',
    },
    {
      icon: Search,
      title: 'Global Search',
      description:
        'Quickly find patients, sessions, and notes using powerful full-text search. Press Cmd+K to open search from anywhere.',
      color: 'yellow',
    },
    {
      icon: FileText,
      title: 'Report Generation',
      description:
        'Generate clinical reports, letters, and documentation with AI assistance. All data stays on your device.',
      color: 'red',
    },
  ];
</script>

<div class="min-h-screen bg-gray-950 flex items-center justify-center p-8">
  <div class="max-w-4xl w-full">
    <div class="mb-8 text-center">
      <h1 class="text-3xl font-bold text-gray-100 mb-2">Welcome to RamDoc!</h1>
      <p class="text-gray-400">
        Here's a quick overview of the key features to help you get started.
      </p>
      <div class="flex items-center justify-center gap-2 mt-4">
        <div class="h-2 w-16 bg-blue-600 rounded-full"></div>
        <div class="h-2 w-16 bg-blue-600 rounded-full"></div>
        <div class="h-2 w-16 bg-blue-600 rounded-full"></div>
        <div class="h-2 w-16 bg-blue-600 rounded-full"></div>
      </div>
    </div>

    {#if error}
      <div class="bg-red-900/20 border border-red-500 rounded-lg p-4 mb-6">
        <p class="text-red-500 text-sm">{error}</p>
      </div>
    {/if}

    <div class="bg-gray-900 border border-gray-800 rounded-lg p-8 space-y-6">
      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        {#each features as feature}
          <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
            <div
              class="inline-block p-3 {colorClasses[feature.color].bg} rounded-lg mb-4"
            >
              <svelte:component this={feature.icon} size={28} class={colorClasses[feature.color].text} />
            </div>
            <h3 class="text-gray-100 font-semibold mb-2">{feature.title}</h3>
            <p class="text-gray-400 text-sm">{feature.description}</p>
          </div>
        {/each}
      </div>

      <div class="bg-blue-900/20 border border-blue-800 rounded-lg p-6">
        <div class="flex items-start gap-4">
          <div class="flex-shrink-0">
            <CheckCircle size={24} class="text-blue-500" />
          </div>
          <div>
            <h3 class="text-blue-400 font-semibold mb-2">Privacy & Security</h3>
            <p class="text-gray-400 text-sm leading-relaxed">
              All your data is encrypted at rest using SQLCipher with AES-256 encryption.
              Patient files are stored in an encrypted vault. The AI model runs locally on
              your machine, so patient data never leaves your device. Audit logs track all
              data access for nDSG compliance.
            </p>
          </div>
        </div>
      </div>

      <div class="bg-gray-800 rounded-lg p-6 border border-gray-700">
        <h3 class="text-gray-100 font-semibold mb-3">Quick Keyboard Shortcuts</h3>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
          <div class="flex items-center justify-between">
            <span class="text-gray-400 text-sm">Open command palette</span>
            <kbd class="px-2 py-1 bg-gray-900 border border-gray-600 rounded text-gray-300 text-xs font-mono">
              Cmd+K
            </kbd>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-gray-400 text-sm">Create new patient</span>
            <kbd class="px-2 py-1 bg-gray-900 border border-gray-600 rounded text-gray-300 text-xs font-mono">
              Cmd+N
            </kbd>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-gray-400 text-sm">Create new session</span>
            <kbd class="px-2 py-1 bg-gray-900 border border-gray-600 rounded text-gray-300 text-xs font-mono">
              Cmd+Shift+S
            </kbd>
          </div>
          <div class="flex items-center justify-between">
            <span class="text-gray-400 text-sm">Close command palette</span>
            <kbd class="px-2 py-1 bg-gray-900 border border-gray-600 rounded text-gray-300 text-xs font-mono">
              Esc
            </kbd>
          </div>
        </div>
      </div>
    </div>

    <div class="flex justify-between items-center mt-8">
      <button
        onclick={handleBack}
        disabled={isCompleting}
        class="px-6 py-3 bg-gray-700 hover:bg-gray-600 disabled:bg-gray-800 disabled:cursor-not-allowed text-white font-medium rounded-lg transition-colors flex items-center gap-2"
      >
        <ChevronLeft size={20} />
        Back
      </button>

      <button
        onclick={handleComplete}
        disabled={isCompleting}
        class="px-8 py-3 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 disabled:cursor-not-allowed text-white font-bold rounded-lg transition-colors flex items-center gap-2 text-lg"
      >
        {#if isCompleting}
          Completing...
        {:else}
          Get Started
          <CheckCircle size={24} />
        {/if}
      </button>
    </div>
  </div>
</div>
