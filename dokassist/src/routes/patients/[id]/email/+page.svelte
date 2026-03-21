<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { listEmails, deleteEmail, parseError, type Email, type AppError } from '$lib/api';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import { t } from '$lib/translations';

  $: patientId = $page.params.id;
  let emails: Email[] = [];
  let loading = true;
  let error: AppError | null = null;

  async function loadEmails() {
    try {
      loading = true;
      error = null;
      emails = await listEmails(patientId);
    } catch (e) {
      error = parseError(e);
    } finally {
      loading = false;
    }
  }

  async function handleDeleteEmail(emailId: string, status: string) {
    const confirmMessage =
      status === 'draft' ? $t('email.confirmDeleteDraft') : $t('email.confirmDelete');

    if (!confirm(confirmMessage)) {
      return;
    }
    try {
      await deleteEmail(emailId);
      await loadEmails();
    } catch (e) {
      error = parseError(e);
    }
  }

  function formatDate(dateStr: string): string {
    return new Date(dateStr).toLocaleDateString('de-DE', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function formatStatus(status: string): string {
    return status === 'draft' ? $t('email.draft') : $t('email.sentStatus');
  }

  onMount(() => {
    loadEmails();
  });
</script>

<div class="p-8">
  <div class="flex justify-between items-center mb-6">
    <h2 class="text-2xl font-bold text-gray-900 dark:text-gray-100">{$t('email.title')}</h2>
    <a
      href={`/patients/${patientId}/email/new`}
      class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
    >
      {$t('email.composeNew')}
    </a>
  </div>

  {#if loading}
    <div class="text-gray-500 dark:text-gray-400">{$t('email.loading')}</div>
  {:else if error}
    <ErrorDisplay {error} showDetails={true} />
  {:else if emails.length === 0}
    <div class="text-center py-12">
      <p class="text-gray-500 dark:text-gray-400 mb-4">{$t('email.noEmails')}</p>
      <a
        href={`/patients/${patientId}/email/new`}
        class="inline-block px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
      >
        {$t('email.composeFirst')}
      </a>
    </div>
  {:else}
    <div class="space-y-4">
      {#each emails as email}
        <div
          class="bg-white dark:bg-gray-800 rounded-lg p-6 border border-gray-200 dark:border-gray-700"
        >
          <div class="flex justify-between items-start mb-3">
            <div class="flex-1">
              <div class="flex items-center gap-3 mb-2">
                <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                  {email.subject}
                </h3>
                <span
                  class="px-2 py-1 text-xs rounded {email.status === 'sent'
                    ? 'bg-green-900/30 text-green-400'
                    : 'bg-yellow-900/30 text-yellow-400'}"
                >
                  {formatStatus(email.status)}
                </span>
              </div>
              <p class="text-sm text-gray-500 dark:text-gray-400">
                {$t('email.to')}
                {email.recipient_email}
              </p>
              <p class="text-xs text-gray-500 mt-1">
                {#if email.status === 'sent' && email.sent_at}
                  {$t('email.sent')} {formatDate(email.sent_at)}
                {:else}
                  {$t('email.created')} {formatDate(email.created_at)}
                {/if}
              </p>
            </div>
            <div class="flex space-x-2">
              <a
                href={`/patients/${patientId}/email/${email.id}`}
                class="px-3 py-1 text-sm bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 rounded hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
              >
                {email.status === 'draft' ? $t('common.edit') : $t('email.view')}
              </a>
              {#if email.status === 'draft'}
                <button
                  on:click={() => handleDeleteEmail(email.id, email.status)}
                  class="px-3 py-1 text-sm bg-red-900/20 text-red-400 rounded hover:bg-red-900/40 transition-colors"
                >
                  {$t('common.delete')}
                </button>
              {/if}
            </div>
          </div>
          <div class="text-sm text-gray-500 dark:text-gray-400 line-clamp-3">
            {email.body.substring(0, 300)}{email.body.length > 300 ? '...' : ''}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
