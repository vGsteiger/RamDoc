<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { getPatient, updatePatient, deletePatient, type Patient, type UpdatePatient } from '$lib/api';
  import PatientForm from '$lib/components/PatientForm.svelte';

  let patient = $state<Patient | null>(null);
  let isLoading = $state(true);
  let isEditing = $state(false);
  let isSubmitting = $state(false);
  let isDeleting = $state(false);
  let showDeleteConfirm = $state(false);
  let error = $state('');

  let patientId = $derived($page.params.id);

  onMount(async () => {
    await loadPatient();
  });

  async function loadPatient() {
    if (!patientId) {
      error = 'No patient ID provided';
      isLoading = false;
      return;
    }

    try {
      isLoading = true;
      error = '';
      patient = await getPatient(patientId);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load patient';
      console.error('Error loading patient:', e);
    } finally {
      isLoading = false;
    }
  }

  async function handleUpdate(event: CustomEvent<{ id: string; data: UpdatePatient }>) {
    try {
      isSubmitting = true;
      error = '';
      patient = await updatePatient(event.detail.id, event.detail.data);
      isEditing = false;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to update patient';
      console.error('Error updating patient:', e);
    } finally {
      isSubmitting = false;
    }
  }

  async function handleDelete() {
    if (!patientId) return;

    try {
      isDeleting = true;
      error = '';
      await deletePatient(patientId);
      goto('/patients');
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to delete patient';
      console.error('Error deleting patient:', e);
      isDeleting = false;
      showDeleteConfirm = false;
    }
  }

  function handleCancelEdit() {
    isEditing = false;
  }

  function formatDate(dateStr: string): string {
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString('de-CH', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit'
      });
    } catch {
      return dateStr;
    }
  }
</script>

<div class="p-8">
  <div class="max-w-4xl mx-auto">
    {#if isLoading}
      <div class="flex justify-center items-center py-12">
        <div class="text-gray-400">Loading patient details...</div>
      </div>
    {:else if error}
      <div class="bg-red-900/20 border border-red-800 rounded-lg p-4 text-red-400 mb-6">
        {error}
      </div>
    {:else if patient}
      <!-- Edit Mode -->
      {#if isEditing}
        <div class="bg-gray-800 rounded-lg p-6">
          <PatientForm {patient} on:submit={handleUpdate} on:cancel={handleCancelEdit} {isSubmitting} />
        </div>
      {:else}
        <!-- View Mode -->
        <div class="bg-gray-800 rounded-lg p-6">
          <!-- Action Buttons -->
          <div class="flex justify-between mb-6">
            <button
              onclick={() => (isEditing = true)}
              class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
            >
              Edit Patient
            </button>
            <button
              onclick={() => (showDeleteConfirm = true)}
              class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors"
            >
              Delete Patient
            </button>
          </div>

          <!-- Patient Details -->
          <div class="space-y-6">
            <!-- Basic Info -->
            <div class="grid grid-cols-2 gap-6">
              <div>
                <label class="block text-sm font-medium text-gray-400 mb-1">First Name</label>
                <p class="text-gray-100">{patient.first_name}</p>
              </div>
              <div>
                <label class="block text-sm font-medium text-gray-400 mb-1">Last Name</label>
                <p class="text-gray-100">{patient.last_name}</p>
              </div>
            </div>

            <div class="grid grid-cols-2 gap-6">
              <div>
                <label class="block text-sm font-medium text-gray-400 mb-1">AHV Number</label>
                <p class="text-gray-100">{patient.ahv_number}</p>
              </div>
              <div>
                <label class="block text-sm font-medium text-gray-400 mb-1">Date of Birth</label>
                <p class="text-gray-100">{formatDate(patient.date_of_birth)}</p>
              </div>
            </div>

            {#if patient.gender}
              <div>
                <label class="block text-sm font-medium text-gray-400 mb-1">Gender</label>
                <p class="text-gray-100 capitalize">{patient.gender}</p>
              </div>
            {/if}

            <!-- Contact Info -->
            {#if patient.phone || patient.email}
              <div class="border-t border-gray-700 pt-6">
                <h3 class="text-lg font-semibold text-gray-100 mb-4">Contact Information</h3>
                <div class="grid grid-cols-2 gap-6">
                  {#if patient.phone}
                    <div>
                      <label class="block text-sm font-medium text-gray-400 mb-1">Phone</label>
                      <p class="text-gray-100">{patient.phone}</p>
                    </div>
                  {/if}
                  {#if patient.email}
                    <div>
                      <label class="block text-sm font-medium text-gray-400 mb-1">Email</label>
                      <p class="text-gray-100">{patient.email}</p>
                    </div>
                  {/if}
                </div>
              </div>
            {/if}

            {#if patient.address}
              <div>
                <label class="block text-sm font-medium text-gray-400 mb-1">Address</label>
                <p class="text-gray-100 whitespace-pre-line">{patient.address}</p>
              </div>
            {/if}

            <!-- Insurance & GP -->
            {#if patient.insurance || patient.gp_name || patient.gp_address}
              <div class="border-t border-gray-700 pt-6">
                <h3 class="text-lg font-semibold text-gray-100 mb-4">Medical Information</h3>

                {#if patient.insurance}
                  <div class="mb-4">
                    <label class="block text-sm font-medium text-gray-400 mb-1">Insurance</label>
                    <p class="text-gray-100">{patient.insurance}</p>
                  </div>
                {/if}

                {#if patient.gp_name || patient.gp_address}
                  <div class="grid grid-cols-2 gap-6">
                    {#if patient.gp_name}
                      <div>
                        <label class="block text-sm font-medium text-gray-400 mb-1">GP Name</label>
                        <p class="text-gray-100">{patient.gp_name}</p>
                      </div>
                    {/if}
                    {#if patient.gp_address}
                      <div>
                        <label class="block text-sm font-medium text-gray-400 mb-1">GP Address</label>
                        <p class="text-gray-100">{patient.gp_address}</p>
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
            {/if}

            <!-- Notes -->
            {#if patient.notes}
              <div class="border-t border-gray-700 pt-6">
                <label class="block text-sm font-medium text-gray-400 mb-1">Notes</label>
                <p class="text-gray-100 whitespace-pre-line">{patient.notes}</p>
              </div>
            {/if}

            <!-- Metadata -->
            <div class="border-t border-gray-700 pt-6 text-sm text-gray-500">
              <div class="grid grid-cols-2 gap-4">
                <div>Created: {formatDate(patient.created_at)}</div>
                <div>Last Updated: {formatDate(patient.updated_at)}</div>
              </div>
            </div>
          </div>
        </div>
      {/if}

      <!-- Delete Confirmation Modal -->
      {#if showDeleteConfirm}
        <div
          class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
          onclick={() => (showDeleteConfirm = false)}
        >
          <div
            class="bg-gray-800 rounded-lg p-6 max-w-md w-full mx-4"
            onclick={(e) => e.stopPropagation()}
          >
            <h2 class="text-xl font-bold text-gray-100 mb-4">Delete Patient</h2>
            <p class="text-gray-300 mb-6">
              Are you sure you want to delete {patient.first_name} {patient.last_name}? This action
              cannot be undone and will also delete all associated sessions, files, diagnoses, medications,
              and reports.
            </p>
            <div class="flex gap-4 justify-end">
              <button
                onclick={() => (showDeleteConfirm = false)}
                disabled={isDeleting}
                class="px-4 py-2 border border-gray-600 rounded-lg text-gray-300 hover:bg-gray-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                Cancel
              </button>
              <button
                onclick={handleDelete}
                disabled={isDeleting}
                class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isDeleting ? 'Deleting...' : 'Delete Patient'}
              </button>
            </div>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>
