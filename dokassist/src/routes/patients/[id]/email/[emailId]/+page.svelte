<script lang="ts">
  import { page } from '$app/stores';
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

  $: patientId = $page.params.id;
  $: emailId = $page.params.emailId;

  let email: Email | null = null;
  let recipientEmail = '';
  let subject = '';
  let body = '';
  let error: AppError | null = null;
  let isLoading = true;
  let isSaving = false;
  let isEditing = false;

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
    <div class="text-gray-500 dark:text-gray-400">{$t('email.loadingEmail')}</div>
  {:else if error}
    <ErrorDisplay {error} showDetails={true} />
  {:else if email}
    <div class="mb-6">
      <div class="flex justify-between items-start mb-2">
        <h2 class="text-2xl font-bold text-gray-900 dark:text-gray-100">
          {email.status === 'draft' ? $t('email.editDraft') : $t('email.viewEmail')}
        </h2>
        <span
          class="px-3 py-1 text-sm rounded {email.status === 'sent'
            ? 'bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400'
            : 'bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-400'}"
        >
          {email.status === 'draft' ? $t('email.draft') : $t('email.sentStatus')}
        </span>
      </div>
      <p class="text-gray-500 dark:text-gray-400 text-sm">
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

    <div
      class="bg-gray-50 dark:bg-gray-800 rounded-lg p-6 border border-gray-200 dark:border-gray-700 space-y-4"
    >
      <div>
        <label
          for="recipient"
          class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
        >
          {$t('email.to')}
        </label>
        <input
          id="recipient"
          type="email"
          bind:value={recipientEmail}
          disabled={!isEditing}
          class="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-60 disabled:cursor-not-allowed"
        />
      </div>

      <div>
        <label
          for="subject"
          class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
        >
          {$t('email.subject')}
        </label>
        <input
          id="subject"
          type="text"
          bind:value={subject}
          disabled={!isEditing}
          class="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-60 disabled:cursor-not-allowed"
        />
      </div>

      <div>
        <label for="body" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          {$t('email.message')}
        </label>
        <textarea
          id="body"
          bind:value={body}
          disabled={!isEditing}
          rows="15"
          class="w-full px-3 py-2 bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono disabled:opacity-60 disabled:cursor-not-allowed"
        ></textarea>
      </div>

      <div
        class="flex justify-between items-center pt-4 border-t border-gray-200 dark:border-gray-700"
      >
        <a
          href={`/patients/${patientId}/email`}
          class="px-4 py-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 transition-colors"
        >
          {$t('email.backToEmails')}
        </a>
        {#if isEditing}
          <div class="flex space-x-3">
            <button
              on:click={handleSaveChanges}
              disabled={isSaving}
              class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded hover:bg-gray-300 dark:hover:bg-gray-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSaving ? $t('email.saving') : $t('email.saveChanges')}
            </button>
            <button
              on:click={handleSendEmail}
              disabled={isSaving}
              class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSaving ? $t('email.opening') : $t('email.openMailClient')}
            </button>
          </div>
        {:else}
          <button
            on:click={() => {
              const mailtoLink = encodeURI(
                `mailto:${recipientEmail}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`
              );
              window.location.href = mailtoLink;
            }}
            class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
          >
            {$t('email.openMailClient')}
          </button>
        {/if}
      </div>
    </div>

    {#if isEditing}
      <div class="mt-4 text-sm text-gray-400 dark:text-gray-500">
        <p>{$t('email.mailClientEditHint')}</p>
      </div>
    {/if}
  {/if}
</div>
