<script lang="ts">
  import {
    uploadFile,
    processFile,
    audioMimeType,
    isAudioMime,
    AUDIO_EXTENSIONS,
    type FileRecord,
  } from '$lib/api';
  import { Upload } from 'lucide-svelte';
  import { t } from '$lib/translations';
  import { errorText } from '$lib/translations/labels';
  import { addToast } from '$lib/stores/toast';
  import { Alert, Button, ProgressBar } from '$lib/components/ui';

  const accept = ['.pdf', '.png', '.jpg', '.jpeg', '.docx']
    .concat(AUDIO_EXTENSIONS.map((ext) => `.${ext}`))
    .join(',');

  interface Props {
    patientId: string;
    onUpload?: (file: FileRecord) => void;
    /** `compact` is a single-line drop strip for list pages. */
    variant?: 'dropzone' | 'compact';
  }

  let { patientId, onUpload, variant = 'dropzone' }: Props = $props();

  let input = $state<HTMLInputElement | null>(null);
  let isDragging = $state(false);
  let isUploading = $state(false);
  let uploadProgress = $state(0);
  let errorMessage = $state('');

  const supportedTypes = [
    'application/pdf',
    'image/png',
    'image/jpeg',
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  ];

  /** Opens the native file picker (used by the Documents "New ▾ Upload file" item). */
  export function browse() {
    input?.click();
  }

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
    isDragging = true;
  }

  function handleDragLeave() {
    isDragging = false;
  }

  async function handleDrop(event: DragEvent) {
    event.preventDefault();
    isDragging = false;

    const files = event.dataTransfer?.files;
    if (!files || files.length === 0) return;

    await uploadFiles(files);
  }

  async function handleFileInput(event: Event) {
    const input = event.target as HTMLInputElement;
    const files = input.files;
    if (!files || files.length === 0) return;

    await uploadFiles(files);
    input.value = '';
  }

  async function uploadFiles(files: FileList) {
    for (const file of Array.from(files)) {
      // Recorded dictations are accepted too; they are transcribed locally.
      const mimeType = supportedTypes.includes(file.type)
        ? file.type
        : audioMimeType(file.name, file.type);
      if (!mimeType) {
        errorMessage = $t('files.unsupportedType').replace('{type}', file.type);
        setTimeout(() => (errorMessage = ''), 5000);
        continue;
      }

      try {
        isUploading = true;
        uploadProgress = 0;
        errorMessage = '';

        const buffer = await file.arrayBuffer();
        const data = Array.from(new Uint8Array(buffer));

        uploadProgress = 50;

        const record = await uploadFile(patientId, file.name, data, mimeType);

        uploadProgress = 100;

        if (onUpload) {
          onUpload(record);
        }

        processFile(record.id).catch((err) => {
          console.warn('process_file failed (non-fatal):', err);
          // A failed transcription (e.g. no speech model installed yet) is
          // worth telling the user about; the recording itself is stored.
          if (isAudioMime(mimeType)) {
            addToast(
              $t('dictation.transcriptionFailed').replace('{error}', $errorText(err)),
              'error'
            );
          }
        });

        isUploading = false;
        uploadProgress = 0;
      } catch (error) {
        console.error('Upload failed:', error);
        errorMessage = $t('files.uploadFailed')
          .replace('{name}', file.name)
          .replace('{error}', String(error));
        isUploading = false;
        uploadProgress = 0;
        setTimeout(() => (errorMessage = ''), 5000);
      }
    }
  }
</script>

<div class="space-y-2">
  <!-- The whole area is a drop target; the visible "Browse" control is the
       keyboard path, so the native input itself stays hidden. -->
  <div
    class="relative flex items-center rounded-card border border-dashed transition-colors duration-150 ease-standard {isDragging
      ? 'border-line-strong bg-surface-selected'
      : 'border-line bg-surface-raised'} {variant === 'compact'
      ? 'min-h-row flex-wrap gap-x-2 gap-y-1 px-3 py-2'
      : 'flex-col justify-center px-6 py-8 text-center'}"
    role="region"
    aria-label={$t('files.uploadArea')}
    ondragover={handleDragOver}
    ondragleave={handleDragLeave}
    ondrop={handleDrop}
  >
    <input
      bind:this={input}
      type="file"
      class="sr-only"
      tabindex="-1"
      aria-hidden="true"
      multiple
      {accept}
      onchange={handleFileInput}
    />

    {#if variant === 'compact'}
      <Upload size={16} class="shrink-0 text-fg-subtle" aria-hidden="true" />
      <span class="text-label text-fg-muted">
        {isDragging ? $t('files.dropHere') : $t('files.dropCompact')}
      </span>
      <Button variant="ghost" size="sm" class="-ml-1" onclick={browse}>
        {$t('files.browse')}
      </Button>
      <span class="min-w-0 basis-full truncate text-caption text-fg-subtle sm:basis-auto sm:pl-1">
        {$t('files.supportedTypes')}
      </span>
    {:else}
      <Upload size={20} class="mb-3 text-fg-subtle" aria-hidden="true" />
      <p class="text-heading text-fg">
        {isDragging ? $t('files.dropHere') : $t('files.dropOrBrowse')}
      </p>
      <p class="mt-1 max-w-measure text-label text-fg-muted">{$t('files.supportedTypes')}</p>
      <Button class="mt-3" onclick={browse}>{$t('files.browse')}</Button>
    {/if}
  </div>

  {#if isUploading}
    <ProgressBar value={uploadProgress} label={$t('files.uploading')} showValue size="sm" />
  {/if}

  {#if errorMessage}
    <Alert tone="danger">{errorMessage}</Alert>
  {/if}
</div>
