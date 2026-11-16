<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import {
    getEmail,
    updateEmail,
    markEmailAsSent,
    parseError,
    type Email,
    type UpdateEmail,
    type AppError,
  } from '$lib/api';
  import ErrorDisplay from '$lib/components/ErrorDisplay.svelte';
  import { t } from '$lib/translations';

  let patientId = $derived(page.params.id!);
  let emailId = $derived(page.params.emailId!);

  let email: Email | null = $state(null);
  let recipientEmail = $state('');
  let subject = $state('');
  let body = $state('');
  let error: AppError | null = $state(null);
  let isLoading = $state(true);
  let isSaving = $state(false);
  let isEditing = $state(false);

  async function loadEmail() {
    try {
      isLoading = true;
      error = null;
      email = await getEmail(emailId);
      recipientEmail = email.recipient_email;
      subject = email.subject;
      body = email.body;
      isEditing = email.status === 'draft';
    } catch (e) {
      error = parseError(e);
    } finally {
      isLoading = false;
    }
  }

  async function handleSaveChanges() {
    if (!recipientEmail.trim() || !subject.trim() || !body.trim()) {
      error = {
        code: 'VALIDATION_ERROR',
        message: $t('email.validationError'),
        ref: 'VALIDATION',
      };
      return;
    }

    try {
      isSaving = true;
      error = null;

      const input: UpdateEmail = {
        recipient_email: recipientEmail,
        subject: subject,
        body: body,
      };

      email = await updateEmail(emailId, input);
      await goto(`/patients/${patientId}/email`);
    } catch (e) {
      error = parseError(e);
    } finally {
      isSaving = false;
    }
  }

  async function handleSendEmail() {
    if (!email) return;

    try {
      isSaving = true;
      error = null;

      if (
        recipientEmail !== email.recipient_email ||
        subject !== email.subject ||
        body !== email.body
      ) {
        const input: UpdateEmail = {
          recipient_email: recipientEmail,
          subject: subject,
          body: body,
        };
        email = await updateEmail(emailId, input);
      }

      await markEmailAsSent(emailId);

      const mailtoLink = encodeURI(
        `mailto:${recipientEmail}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`
      );
      window.location.href = mailtoLink;

      setTimeout(() => {
        goto(`/patients/${patientId}/email`);
      }, 500);
    } catch (e) {
      error = parseError(e);
    } finally {
      isSaving = false;
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

  onMount(() => {
    loadEmail();
  });
</script>

<div class="p-8 max-w-4xl mx-auto">
  {#if isLoading}
    <div class="text-fg-muted">{$t('email.loadingEmail')}</div>
  {:else if error}
    <ErrorDisplay {error} showDetails={true} />
  {:else if email}
    <div class="mb-6">
      <div class="flex justify-between items-start mb-2">
        <h2 class="text-display font-semibold text-fg">
          {email.status === 'draft' ? $t('email.editDraft') : $t('email.viewEmail')}
        </h2>
        <span
          class="px-3 py-1 text-body rounded-card {email.status === 'sent'
            ? 'bg-success-subtle text-success-fg '
            : 'bg-warning-subtle text-warning-fg'}"
        >
          {email.status === 'draft' ? $t('email.draft') : $t('email.sentStatus')}
        </span>
      </div>
      <p class="text-fg-muted text-body">
        {#if email.status === 'sent' && email.sent_at}
          {$t('email.sent')} {formatDate(email.sent_at)}
        {:else}
          {$t('email.created')} {formatDate(email.created_at)}
        {/if}
      </p>
    </div>

    {#if error}
      <div class="mb-6">
        <ErrorDisplay {error} showDetails={true} />
      </div>
    {/if}

    <div class="bg-surface-sunken rounded-card p-6 border border-line space-y-4">
      <div>
        <label for="recipient" class="block text-body font-medium text-fg-muted mb-2">
          {$t('email.to')}
        </label>
        <input
          id="recipient"
          type="email"
          bind:value={recipientEmail}
          disabled={!isEditing}
          class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30 disabled:opacity-60 disabled:cursor-not-allowed"
        />
      </div>

      <div>
        <label for="subject" class="block text-body font-medium text-fg-muted mb-2">
          {$t('email.subject')}
        </label>
        <input
          id="subject"
          type="text"
          bind:value={subject}
          disabled={!isEditing}
          class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30 disabled:opacity-60 disabled:cursor-not-allowed"
        />
      </div>

      <div>
        <label for="body" class="block text-body font-medium text-fg-muted mb-2">
          {$t('email.message')}
        </label>
        <textarea
          id="body"
          bind:value={body}
          disabled={!isEditing}
          rows="15"
          class="w-full px-3 py-2 bg-surface-raised border border-line rounded-control text-fg focus:outline-none focus:ring-2 focus:ring-accent/30 font-mono disabled:opacity-60 disabled:cursor-not-allowed"
        ></textarea>
      </div>

      <div class="flex justify-between items-center pt-4 border-t border-line">
        <a
          href={`/patients/${patientId}/email`}
          class="px-4 py-2 text-fg-muted hover:text-fg transition-colors"
        >
          {$t('email.backToEmails')}
        </a>
        {#if isEditing}
          <div class="flex space-x-3">
            <button
              onclick={handleSaveChanges}
              disabled={isSaving}
              class="h-8 px-3 bg-surface-selected text-fg-muted rounded-control hover:bg-surface-selected transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSaving ? $t('email.saving') : $t('email.saveChanges')}
            </button>
            <button
              onclick={handleSendEmail}
              disabled={isSaving}
              class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSaving ? $t('email.opening') : $t('email.openMailClient')}
            </button>
          </div>
        {:else}
          <button
            onclick={() => {
              const mailtoLink = encodeURI(
                `mailto:${recipientEmail}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`
              );
              window.location.href = mailtoLink;
            }}
            class="h-8 px-3 bg-accent text-on-accent rounded-control hover:bg-accent-hover transition-colors"
          >
            {$t('email.openMailClient')}
          </button>
        {/if}
      </div>
    </div>

    {#if isEditing}
      <div class="mt-4 text-body text-fg-subtle">
        <p>{$t('email.mailClientEditHint')}</p>
      </div>
    {/if}
  {/if}
</div>
