<script lang="ts">
  import { uploadFile, processFile, type FileRecord } from '$lib/api';
  import { Paperclip } from 'lucide-svelte';

  interface Props {
    patientId: string;
    onUpload?: (file: FileRecord) => void;
  }

  let { patientId, onUpload }: Props = $props();

  let isDragging = $state(false);
  let isUploading = $state(false);
  let uploadProgress = $state(0);
  let errorMessage = $state('');

  const supportedTypes = ['application/pdf', 'image/png', 'image/jpeg', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'];

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
    input.value = ''; // Reset input
  }

  async function uploadFiles(files: FileList) {
    for (const file of Array.from(files)) {
      if (!supportedTypes.includes(file.type)) {
        errorMessage = `File type ${file.type} not supported. Supported types: PDF, PNG, JPG, DOCX`;
        setTimeout(() => errorMessage = '', 5000);
        continue;
      }

      try {
        isUploading = true;
        uploadProgress = 0;
        errorMessage = '';

        const buffer = await file.arrayBuffer();
        const data = Array.from(new Uint8Array(buffer));

        uploadProgress = 50; // Simulate progress

        const record = await uploadFile(patientId, file.name, data, file.type);

        uploadProgress = 100;

        if (onUpload) {
          onUpload(record);
        }

        // Fire-and-forget: extract text + embed in the background.
        // The backend emits "file-processed" when done; do not block the upload UI.
        processFile(record.id).catch((err) => {
          console.warn('process_file failed (non-fatal):', err);
        });

        isUploading = false;
        uploadProgress = 0;
      } catch (error) {
        console.error('Upload failed:', error);
        errorMessage = `Failed to upload ${file.name}: ${error}`;
        isUploading = false;
        uploadProgress = 0;
        setTimeout(() => errorMessage = '', 5000);
      }
    }
  }
</script>

<div class="space-y-4">
  <div
    class="relative border-2 border-dashed rounded-lg p-8 transition-colors {isDragging
      ? 'border-blue-500 bg-blue-500/10'
      : 'border-gray-700 bg-gray-900'}"
    ondragover={handleDragOver}
    ondragleave={handleDragLeave}
    ondrop={handleDrop}
  >
    <input
      type="file"
      class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
      multiple
      accept=".pdf,.png,.jpg,.jpeg,.docx"
      onchange={handleFileInput}
    />

    <div class="text-center pointer-events-none">
      <div class="mb-4 flex justify-center text-gray-400">
        <Paperclip size={48} />
      </div>
      <p class="text-gray-300 font-medium mb-2">
        Drop files here or click to browse
      </p>
      <p class="text-sm text-gray-500">
        Supports PDF, PNG, JPG, DOCX
      </p>
    </div>
  </div>

  {#if isUploading}
    <div class="bg-gray-900 rounded-lg p-4">
      <div class="flex items-center justify-between mb-2">
        <span class="text-sm text-gray-300">Uploading...</span>
        <span class="text-sm text-gray-400">{uploadProgress}%</span>
      </div>
      <div class="w-full bg-gray-800 rounded-full h-2">
        <div
          class="bg-blue-600 h-2 rounded-full transition-all duration-300"
          style="width: {uploadProgress}%"
        ></div>
      </div>
    </div>
  {/if}

  {#if errorMessage}
    <div class="bg-red-900/20 border border-red-800 rounded-lg p-4">
      <p class="text-sm text-red-400">{errorMessage}</p>
    </div>
  {/if}
</div>
