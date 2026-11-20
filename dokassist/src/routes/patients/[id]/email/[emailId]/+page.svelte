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
  import { fmt } from '$lib/format';
  import { ArrowLeft } from 'lucide-svelte';
  import {
    Badge,
    Button,
    Field,
    Input,
    ListSkeleton,
    PageShell,
    Textarea,
  } from '$lib/components/ui';
  import StickyActionBar from '$lib/components/documents/StickyActionBar.svelte';
  import { documentsUrl, parseTimestamp } from '$lib/components/documents/documents';

  let patientId = $derived(page.params.id!);
  let emailId = $derived(page.params.emailId!);

  let email = $state<Email | null>(null);
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
      await goto(documentsUrl(patientId, 'emails'));
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
        goto(documentsUrl(patientId, 'emails'));
      }, 500);
    } catch (e) {
      error = parseError(e);
    } finally {
      isSaving = false;
    }
  }

  onMount(() => {
    loadEmail();
  });

  function openMailClient() {
    const mailtoLink = encodeURI(
      `mailto:${recipientEmail}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`
    );
    window.location.href = mailtoLink;
  }

  let dateLine = $derived(
    email
      ? email.status === 'sent' && email.sent_at
        ? `${$t('email.sent')} ${$fmt.dateTime(parseTimestamp(email.sent_at))}`
        : `${$t('email.created')} ${$fmt.dateTime(parseTimestamp(email.created_at))}`
      : undefined
  );
</script>

<PageShell
  width="prose"
  title={email
    ? email.status === 'draft'
      ? $t('email.editDraft')
      : $t('email.viewEmail')
    : undefined}
  description={dateLine}
>
  {#snippet breadcrumb()}
    <a
      href={documentsUrl(patientId, 'emails')}
      class="inline-flex items-center gap-1 text-fg-subtle hover:text-fg"
    >
      <ArrowLeft size={12} aria-hidden="true" />
      {$t('email.backToDocuments')}
    </a>
  {/snippet}

  {#if isLoading}
    <p class="sr-only">{$t('email.loadingEmail')}</p>
    <ListSkeleton count={3} />
  {:else if error && !email}
    <ErrorDisplay {error} showDetails={true} />
  {:else if email}
    <div class="mb-4">
      <Badge tone={email.status === 'sent' ? 'success' : 'warning'}>
        {email.status === 'draft' ? $t('email.draft') : $t('email.sentStatus')}
      </Badge>
    </div>

    {#if error}
      <div class="mb-4">
        <ErrorDisplay {error} showDetails={true} />
      </div>
    {/if}

    <div class="space-y-4">
      <Field label={$t('email.to')} for="recipient">
        <Input id="recipient" type="email" bind:value={recipientEmail} disabled={!isEditing} />
      </Field>

      <Field label={$t('email.subject')} for="subject">
        <Input id="subject" bind:value={subject} disabled={!isEditing} />
      </Field>

      <Field label={$t('email.message')} for="body">
        <Textarea
          id="body"
          bind:value={body}
          disabled={!isEditing}
          rows={15}
          class="leading-relaxed"
        />
      </Field>

      {#if isEditing}
        <p class="text-label text-fg-subtle">{$t('email.mailClientEditHint')}</p>
      {/if}
    </div>

    <StickyActionBar>
      {#snippet start()}
        <Button variant="ghost" href={documentsUrl(patientId, 'emails')}>
          {$t('email.backToEmails')}
        </Button>
      {/snippet}
      {#if isEditing}
        <Button onclick={handleSaveChanges} disabled={isSaving}>
          {isSaving ? $t('email.saving') : $t('email.saveChanges')}
        </Button>
        <Button variant="primary" onclick={handleSendEmail} disabled={isSaving}>
          {isSaving ? $t('email.opening') : $t('email.openMailClient')}
        </Button>
      {:else}
        <Button variant="primary" onclick={openMailClient}>
          {$t('email.openMailClient')}
        </Button>
      {/if}
    </StickyActionBar>
  {/if}
</PageShell>
