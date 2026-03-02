<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { Patient, CreatePatient, UpdatePatient } from '$lib/api';
  import AhvInput from './AhvInput.svelte';

  interface Props {
    patient?: Patient | null;
    isSubmitting?: boolean;
  }

  let { patient = null, isSubmitting = false }: Props = $props();

  const dispatch = createEventDispatcher<{
    submit: CreatePatient | { id: string; data: UpdatePatient };
    cancel: void;
  }>();

  let formData = $state({
    ahv_number: patient?.ahv_number || '',
    first_name: patient?.first_name || '',
    last_name: patient?.last_name || '',
    date_of_birth: patient?.date_of_birth || '',
    gender: patient?.gender || '',
    address: patient?.address || '',
    phone: patient?.phone || '',
    email: patient?.email || '',
    insurance: patient?.insurance || '',
    gp_name: patient?.gp_name || '',
    gp_address: patient?.gp_address || '',
    notes: patient?.notes || ''
  });

  let errors = $state<Record<string, string>>({});

  function validate(): boolean {
    errors = {};

    if (!formData.ahv_number) {
      errors.ahv_number = 'AHV number is required';
    }

    if (!formData.first_name.trim()) {
      errors.first_name = 'First name is required';
    }

    if (!formData.last_name.trim()) {
      errors.last_name = 'Last name is required';
    }

    if (!formData.date_of_birth) {
      errors.date_of_birth = 'Date of birth is required';
    } else {
      const date = new Date(formData.date_of_birth);
      const today = new Date();
      if (date > today) {
        errors.date_of_birth = 'Date of birth cannot be in the future';
      }
    }

    return Object.keys(errors).length === 0;
  }

  function handleSubmit() {
    if (!validate()) {
      return;
    }

    if (patient) {
      // Update mode - only send changed fields
      const updates: UpdatePatient = {};
      if (formData.ahv_number !== patient.ahv_number) updates.ahv_number = formData.ahv_number;
      if (formData.first_name !== patient.first_name) updates.first_name = formData.first_name;
      if (formData.last_name !== patient.last_name) updates.last_name = formData.last_name;
      if (formData.date_of_birth !== patient.date_of_birth)
        updates.date_of_birth = formData.date_of_birth;
      if (formData.gender !== patient.gender) updates.gender = formData.gender || null;
      if (formData.address !== patient.address) updates.address = formData.address || null;
      if (formData.phone !== patient.phone) updates.phone = formData.phone || null;
      if (formData.email !== patient.email) updates.email = formData.email || null;
      if (formData.insurance !== patient.insurance) updates.insurance = formData.insurance || null;
      if (formData.gp_name !== patient.gp_name) updates.gp_name = formData.gp_name || null;
      if (formData.gp_address !== patient.gp_address)
        updates.gp_address = formData.gp_address || null;
      if (formData.notes !== patient.notes) updates.notes = formData.notes || null;

      dispatch('submit', { id: patient.id, data: updates });
    } else {
      // Create mode
      const createData: CreatePatient = {
        ahv_number: formData.ahv_number,
        first_name: formData.first_name,
        last_name: formData.last_name,
        date_of_birth: formData.date_of_birth,
        gender: formData.gender || null,
        address: formData.address || null,
        phone: formData.phone || null,
        email: formData.email || null,
        insurance: formData.insurance || null,
        gp_name: formData.gp_name || null,
        gp_address: formData.gp_address || null,
        notes: formData.notes || null
      };

      dispatch('submit', createData);
    }
  }

  function handleCancel() {
    dispatch('cancel');
  }
</script>

<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }} class="space-y-6">
  <!-- AHV Number -->
  <div>
    <label for="ahv_number" class="block text-sm font-medium text-gray-300 mb-2">
      AHV Number <span class="text-red-400">*</span>
    </label>
    <AhvInput bind:value={formData.ahv_number} error={errors.ahv_number} />
  </div>

  <!-- Name Fields -->
  <div class="grid grid-cols-2 gap-4">
    <div>
      <label for="first_name" class="block text-sm font-medium text-gray-300 mb-2">
        First Name <span class="text-red-400">*</span>
      </label>
      <input
        type="text"
        id="first_name"
        bind:value={formData.first_name}
        class="w-full px-4 py-2 bg-gray-800 border rounded-lg text-gray-100 focus:outline-none focus:border-blue-500 {errors.first_name
          ? 'border-red-500'
          : 'border-gray-700'}"
      />
      {#if errors.first_name}
        <p class="mt-1 text-sm text-red-400">{errors.first_name}</p>
      {/if}
    </div>

    <div>
      <label for="last_name" class="block text-sm font-medium text-gray-300 mb-2">
        Last Name <span class="text-red-400">*</span>
      </label>
      <input
        type="text"
        id="last_name"
        bind:value={formData.last_name}
        class="w-full px-4 py-2 bg-gray-800 border rounded-lg text-gray-100 focus:outline-none focus:border-blue-500 {errors.last_name
          ? 'border-red-500'
          : 'border-gray-700'}"
      />
      {#if errors.last_name}
        <p class="mt-1 text-sm text-red-400">{errors.last_name}</p>
      {/if}
    </div>
  </div>

  <!-- Date of Birth and Gender -->
  <div class="grid grid-cols-2 gap-4">
    <div>
      <label for="date_of_birth" class="block text-sm font-medium text-gray-300 mb-2">
        Date of Birth <span class="text-red-400">*</span>
      </label>
      <input
        type="date"
        id="date_of_birth"
        bind:value={formData.date_of_birth}
        class="w-full px-4 py-2 bg-gray-800 border rounded-lg text-gray-100 focus:outline-none focus:border-blue-500 {errors.date_of_birth
          ? 'border-red-500'
          : 'border-gray-700'}"
      />
      {#if errors.date_of_birth}
        <p class="mt-1 text-sm text-red-400">{errors.date_of_birth}</p>
      {/if}
    </div>

    <div>
      <label for="gender" class="block text-sm font-medium text-gray-300 mb-2">Gender</label>
      <select
        id="gender"
        bind:value={formData.gender}
        class="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:border-blue-500"
      >
        <option value="">Select...</option>
        <option value="male">Male</option>
        <option value="female">Female</option>
        <option value="other">Other</option>
      </select>
    </div>
  </div>

  <!-- Contact Information -->
  <div class="grid grid-cols-2 gap-4">
    <div>
      <label for="phone" class="block text-sm font-medium text-gray-300 mb-2">Phone</label>
      <input
        type="tel"
        id="phone"
        bind:value={formData.phone}
        class="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:border-blue-500"
      />
    </div>

    <div>
      <label for="email" class="block text-sm font-medium text-gray-300 mb-2">Email</label>
      <input
        type="email"
        id="email"
        bind:value={formData.email}
        class="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:border-blue-500"
      />
    </div>
  </div>

  <!-- Address -->
  <div>
    <label for="address" class="block text-sm font-medium text-gray-300 mb-2">Address</label>
    <textarea
      id="address"
      bind:value={formData.address}
      rows="2"
      class="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:border-blue-500"
    ></textarea>
  </div>

  <!-- Insurance -->
  <div>
    <label for="insurance" class="block text-sm font-medium text-gray-300 mb-2">Insurance</label>
    <input
      type="text"
      id="insurance"
      bind:value={formData.insurance}
      class="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:border-blue-500"
    />
  </div>

  <!-- GP Information -->
  <div class="grid grid-cols-2 gap-4">
    <div>
      <label for="gp_name" class="block text-sm font-medium text-gray-300 mb-2">GP Name</label>
      <input
        type="text"
        id="gp_name"
        bind:value={formData.gp_name}
        class="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:border-blue-500"
      />
    </div>

    <div>
      <label for="gp_address" class="block text-sm font-medium text-gray-300 mb-2">
        GP Address
      </label>
      <input
        type="text"
        id="gp_address"
        bind:value={formData.gp_address}
        class="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:border-blue-500"
      />
    </div>
  </div>

  <!-- Notes -->
  <div>
    <label for="notes" class="block text-sm font-medium text-gray-300 mb-2">Notes</label>
    <textarea
      id="notes"
      bind:value={formData.notes}
      rows="4"
      class="w-full px-4 py-2 bg-gray-800 border border-gray-700 rounded-lg text-gray-100 focus:outline-none focus:border-blue-500"
    ></textarea>
  </div>

  <!-- Actions -->
  <div class="flex gap-4 justify-end">
    <button
      type="button"
      onclick={handleCancel}
      disabled={isSubmitting}
      class="px-6 py-2 border border-gray-600 rounded-lg text-gray-300 hover:bg-gray-800 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
    >
      Cancel
    </button>
    <button
      type="submit"
      disabled={isSubmitting}
      class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
    >
      {isSubmitting ? 'Saving...' : patient ? 'Update Patient' : 'Create Patient'}
    </button>
  </div>
</form>
