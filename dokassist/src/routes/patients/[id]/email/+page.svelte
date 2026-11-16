<script lang="ts">
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import { listEmails, deleteEmail, parseError, type Email, type AppError } from '$lib/api';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import { t } from '$lib/translations';

  let patientId = $derived(page.params.id!);
  let emails: Email[] = $state([]);
  let loading = $state(true);
  let error: AppError | null = $state(null);

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
    <h2 class="text-display font-semibold text-fg">{$t('email.title')}</h2>
    <a
      href={`/patients/${patientId}/email/new`}
      class="inline-flex items-center h-8 px-3 bg-accent text-on-accent rounded-card hover:bg-accent-hover transition-colors"
    >
      {$t('email.composeNew')}
    </a>
  </div>

  {#if loading}
    <div class="text-fg-muted">{$t('email.loading')}</div>
  {:else if error}
    <ErrorDisplay {error} showDetails={true} />
  {:else if emails.length === 0}
    <div class="text-center py-12">
      <p class="text-fg-muted mb-4">{$t('email.noEmails')}</p>
      <a
        href={`/patients/${patientId}/email/new`}
        class="inline-flex items-center inline-block h-8 px-3 bg-accent text-on-accent rounded-card hover:bg-accent-hover transition-colors"
      >
        {$t('email.composeFirst')}
      </a>
    </div>
  {:else}
    <div class="space-y-4">
      {#each emails as email}
        <div class="bg-surface-raised rounded-card p-6 border border-line">
          <div class="flex justify-between items-start mb-3">
            <div class="flex-1">
              <div class="flex items-center gap-3 mb-2">
                <h3 class="text-heading font-semibold text-fg">
                  {email.subject}
                </h3>
                <span
                  class="px-2 py-1 text-caption rounded-card {email.status === 'sent'
                    ? 'bg-success-subtle text-success-fg'
                    : 'bg-warning-subtle text-warning-fg'}"
                >
                  {formatStatus(email.status)}
                </span>
              </div>
              <p class="text-body text-fg-muted">
                {$t('email.to')}
                {email.recipient_email}
              </p>
              <p class="text-caption text-fg-muted mt-1">
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
                class="inline-flex items-center h-7 px-2.5 text-body bg-surface-hover text-fg-muted rounded-card hover:bg-surface-selected transition-colors"
              >
                {email.status === 'draft' ? $t('common.edit') : $t('email.view')}
              </a>
              {#if email.status === 'draft'}
                <button
                  onclick={() => handleDeleteEmail(email.id, email.status)}
                  class="h-7 px-2.5 text-body bg-danger-subtle text-danger-fg rounded-control hover:bg-danger-subtle/40 transition-colors"
                >
                  {$t('common.delete')}
                </button>
              {/if}
            </div>
          </div>
          <div class="text-body text-fg-muted line-clamp-3">
            {email.body.substring(0, 300)}{email.body.length > 300 ? '...' : ''}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
