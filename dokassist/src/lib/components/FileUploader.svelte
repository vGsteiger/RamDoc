<script lang="ts">
  import {
    uploadFile,
    processFile,
    audioMimeType,
    isAudioMime,
    AUDIO_EXTENSIONS,
    type FileRecord,
  } from '$lib/api';
  import { Paperclip } from 'lucide-svelte';
  import { t } from '$lib/translations';
  import { errorText } from '$lib/translations/labels';
  import { addToast } from '$lib/stores/toast';

  const accept = ['.pdf', '.png', '.jpg', '.jpeg', '.docx']
    .concat(AUDIO_EXTENSIONS.map((ext) => `.${ext}`))
    .join(',');

  interface Props {
    patientId: string;
    onUpload?: (file: FileRecord) => void;
  }

  let { patientId, onUpload }: Props = $props();

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

<div class="space-y-4">
  <div
    class="relative border-2 border-dashed rounded-card p-8 transition-colors {isDragging
      ? 'border-accent bg-accent-subtle'
      : 'border-line bg-surface-sunken'}"
    role="region"
    aria-label={$t('files.uploadArea')}
    ondragover={handleDragOver}
    ondragleave={handleDragLeave}
    ondrop={handleDrop}
  >
    <input
      type="file"
      class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
      multiple
      {accept}
      onchange={handleFileInput}
    />

    <div class="text-center pointer-events-none">
      <div class="mb-4 flex justify-center text-fg-muted">
        <Paperclip size={48} />
      </div>
      <p class="text-fg-muted font-medium mb-2">
        {$t('files.dropOrBrowse')}
      </p>
      <p class="text-body text-fg-subtle">
        {$t('files.supportedTypes')}
      </p>
    </div>
  </div>

  {#if isUploading}
    <div class="bg-surface-sunken rounded-card p-4">
      <div class="flex items-center justify-between mb-2">
        <span class="text-body text-fg-muted">{$t('files.uploading')}</span>
        <span class="text-body text-fg-muted">{uploadProgress}%</span>
      </div>
      <div class="w-full bg-surface-selected rounded-full h-2">
        <div
          class="bg-accent h-2 rounded-full transition-colors duration-300"
          style="width: {uploadProgress}%"
        ></div>
      </div>
    </div>
  {/if}

  {#if errorMessage}
    <div class="bg-danger-subtle border border-danger-line rounded-card p-4">
      <p class="text-body text-danger-fg">{errorMessage}</p>
    </div>
  {/if}
</div>
