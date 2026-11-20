<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import {
    getLetter,
    updateLetter,
    deleteLetter,
    markLetterAsFinalized,
    markLetterAsSent,
    type Letter,
  } from '$lib/api';
  import { t } from '$lib/translations';
  import { errorText } from '$lib/translations/labels';
  import { fmt } from '$lib/format';
  import { addToast } from '$lib/stores/toast';
  import { ArrowLeft, Copy, Trash2 } from 'lucide-svelte';
  import {
    Alert,
    Badge,
    Button,
    ConfirmDialog,
    Field,
    ListSkeleton,
    Menu,
    PageShell,
    Textarea,
    type MenuItem,
  } from '$lib/components/ui';
  import StickyActionBar from '$lib/components/documents/StickyActionBar.svelte';
  import { documentsUrl, letterStatus, parseTimestamp } from '$lib/components/documents/documents';

  /*
   * A saved letter. Drafts stay editable and can be finalized; finalized
   * letters are read-only and can be marked as sent. Uses the existing
   * letter commands only.
   */
  let patientId = $derived(page.params.id!);
  let letterId = $derived(page.params.letterId!);

  let letter = $state<Letter | null>(null);
  let content = $state('');
  let loading = $state(true);
  let busy = $state(false);
  let error = $state('');
  let confirmDeleteOpen = $state(false);

  let isDraft = $derived(letter?.status === 'draft');
  let dirty = $derived(Boolean(letter && content !== letter.content));
  let status = $derived(letter ? letterStatus(letter.status) : null);
  const statusTone = { draft: 'warning', final: 'info', sent: 'success' } as const;

  async function load() {
    try {
      loading = true;
      error = '';
      letter = await getLetter(letterId);
      content = letter.content;
    } catch (e) {
      error = $errorText(e);
    } finally {
      loading = false;
    }
  }

  async function run(action: () => Promise<Letter>) {
    busy = true;
    error = '';
    try {
      letter = await action();
      content = letter.content;
    } catch (e) {
      error = $errorText(e);
    } finally {
      busy = false;
    }
  }

  async function save() {
    await run(() => updateLetter(letterId, { content }));
    if (!error) addToast($t('letters.saved'));
  }

  async function finalize() {
    // Never finalize stale text: save pending edits first.
    if (dirty) {
      await run(() => updateLetter(letterId, { content }));
      if (error) return;
    }
    await run(() => markLetterAsFinalized(letterId));
  }

  async function markSent() {
    await run(() => markLetterAsSent(letterId));
  }

  async function copyText() {
    try {
      await navigator.clipboard.writeText(content);
      addToast($t('letters.copied'));
    } catch (e) {
      error = $errorText(e);
    }
  }

  async function handleDelete() {
    try {
      await deleteLetter(letterId);
      await goto(documentsUrl(patientId, 'letters'));
    } catch (e) {
      error = $errorText(e);
    }
  }

  let recipient = $derived(
    letter ? [letter.recipient_name, letter.recipient_address].filter(Boolean).join('\n') : ''
  );

  let description = $derived.by(() => {
    if (!letter) return undefined;
    const type = $t(`letters.types.${letter.letter_type}`);
    const when =
      letter.status === 'sent' && letter.sent_at
        ? $t('letters.sentOn').replace('{date}', $fmt.dateTime(parseTimestamp(letter.sent_at)))
        : letter.status === 'finalized' && letter.finalized_at
          ? $t('letters.finalizedOn').replace(
              '{date}',
              $fmt.dateTime(parseTimestamp(letter.finalized_at))
            )
          : $t('letters.createdOn').replace(
              '{date}',
              $fmt.dateTime(parseTimestamp(letter.created_at))
            );
    return `${type} · ${when}`;
  });

  let moreItems = $derived<MenuItem[]>([
    { label: $t('letters.copyText'), icon: Copy, onSelect: () => void copyText() },
    {
      label: $t('common.delete'),
      icon: Trash2,
      danger: true,
      onSelect: () => (confirmDeleteOpen = true),
    },
  ]);

  onMount(() => {
    void load();
  });
</script>

<PageShell
  width="prose"
  title={letter ? letter.subject || $t(`letters.types.${letter.letter_type}`) : undefined}
  {description}
>
  {#snippet breadcrumb()}
    <a
      href={documentsUrl(patientId, 'letters')}
      class="inline-flex items-center gap-1 text-fg-subtle hover:text-fg"
    >
      <ArrowLeft size={12} aria-hidden="true" />
      {$t('letters.backToDocuments')}
    </a>
  {/snippet}

  {#if loading}
    <p class="sr-only">{$t('letters.loadingLetter')}</p>
    <ListSkeleton count={3} />
  {:else if error && !letter}
    <Alert tone="danger">{error}</Alert>
  {:else if letter}
    {#if error}
      <Alert tone="danger" class="mb-4">{error}</Alert>
    {/if}

    {#if status}
      <div class="mb-4">
        <Badge tone={statusTone[status]}>{$t(`documents.status.${status}`)}</Badge>
      </div>
    {/if}

    {#if recipient}
      <dl class="mb-4 text-body">
        <dt class="field-label">{$t('letters.recipient')}</dt>
        <dd>
          <pre class="whitespace-pre-wrap font-sans text-body text-fg">{recipient}</pre>
        </dd>
      </dl>
    {/if}

    {#if isDraft}
      <Field label={$t('letters.content')} for="letter-content">
        <Textarea id="letter-content" bind:value={content} rows={22} class="leading-relaxed" />
      </Field>
    {:else}
      <article
        class="card-surface rounded-panel px-5 py-4 sm:px-8 sm:py-6"
        aria-label={$t('letters.content')}
      >
        <pre class="whitespace-pre-wrap font-sans text-body text-fg">{letter.content}</pre>
      </article>
      <p class="mt-2 text-caption text-fg-subtle">{$t('letters.readOnlyHint')}</p>
    {/if}

    <StickyActionBar>
      {#snippet start()}
        <Menu items={moreItems} label={$t('common.moreActions')} align="start" />
      {/snippet}
      {#if isDraft}
        <Button onclick={save} disabled={!dirty || busy}>{$t('letters.saveChanges')}</Button>
        <Button variant="primary" onclick={finalize} loading={busy}>
          {$t('letters.finalize')}
        </Button>
      {:else if letter.status === 'finalized'}
        <Button variant="primary" onclick={markSent} loading={busy}>
          {$t('letters.markAsSent')}
        </Button>
      {/if}
    </StickyActionBar>
  {/if}
</PageShell>

{#if letter}
  <ConfirmDialog
    bind:open={confirmDeleteOpen}
    tone="danger"
    title={$t('letters.deleteTitle')}
    message={$t('letters.confirmDeleteNamed').replace(
      '{subject}',
      letter.subject || $t(`letters.types.${letter.letter_type}`)
    )}
    onConfirm={handleDelete}
  />
{/if}
