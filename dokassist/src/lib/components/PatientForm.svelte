<script lang="ts">
  import { createEventDispatcher, untrack } from 'svelte';
  import type { Patient, CreatePatient, UpdatePatient } from '$lib/api';
  import AhvInput from './AhvInput.svelte';
  import { t } from '$lib/translations';

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

  $effect(() => {
    if (patient) {
      untrack(() => {
        formData = {
          ahv_number: patient!.ahv_number || '',
          first_name: patient!.first_name || '',
          last_name: patient!.last_name || '',
          date_of_birth: patient!.date_of_birth || '',
          gender: patient!.gender || '',
          address: patient!.address || '',
          phone: patient!.phone || '',
          email: patient!.email || '',
          insurance: patient!.insurance || '',
          gp_name: patient!.gp_name || '',
          gp_address: patient!.gp_address || '',
          notes: patient!.notes || ''
        };
      });
    }
  });

  let errors = $state<Record<string, string>>({});

  function validate(): boolean {
    errors = {};

    if (!formData.ahv_number) {
      errors.ahv_number = $t('patients.validation.ahvRequired');
    }

    if (!formData.first_name.trim()) {
      errors.first_name = $t('patients.validation.firstNameRequired');
    }

    if (!formData.last_name.trim()) {
      errors.last_name = $t('patients.validation.lastNameRequired');
    }

    if (!formData.date_of_birth) {
      errors.date_of_birth = $t('patients.validation.dateOfBirthRequired');
    } else {
      const date = new Date(formData.date_of_birth);
      const today = new Date();
      if (date > today) {
        errors.date_of_birth = $t('patients.validation.dateOfBirthFuture');
      }
    }

    return Object.keys(errors).length === 0;
  }

  function handleSubmit() {
    if (!validate()) {
      return;
    }

    if (patient) {
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
    <label for="ahv_number" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
      {$t('patients.ahvNumber')} <span class="text-red-400">*</span>
    </label>
    <AhvInput bind:value={formData.ahv_number} error={errors.ahv_number} />
  </div>

  <!-- Name Fields -->
  <div class="grid grid-cols-2 gap-4">
    <div>
      <label for="first_name" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        {$t('patients.firstName')} <span class="text-red-400">*</span>
      </label>
      <input
        type="text"
        id="first_name"
        bind:value={formData.first_name}
        class="w-full px-4 py-2 bg-white dark:bg-gray-800 border rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500 {errors.first_name
          ? 'border-red-500'
          : 'border-gray-700'}"
      />
      {#if errors.first_name}
        <p class="mt-1 text-sm text-red-400">{errors.first_name}</p>
      {/if}
    </div>

    <div>
      <label for="last_name" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        {$t('patients.lastName')} <span class="text-red-400">*</span>
      </label>
      <input
        type="text"
        id="last_name"
        bind:value={formData.last_name}
        class="w-full px-4 py-2 bg-white dark:bg-gray-800 border rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500 {errors.last_name
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
      <label for="date_of_birth" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        {$t('patients.dateOfBirth')} <span class="text-red-400">*</span>
      </label>
      <input
        type="date"
        id="date_of_birth"
        bind:value={formData.date_of_birth}
        class="w-full px-4 py-2 bg-white dark:bg-gray-800 border rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500 {errors.date_of_birth
          ? 'border-red-500'
          : 'border-gray-700'}"
      />
      {#if errors.date_of_birth}
        <p class="mt-1 text-sm text-red-400">{errors.date_of_birth}</p>
      {/if}
    </div>

    <div>
      <label for="gender" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{$t('patients.gender')}</label>
      <select
        id="gender"
        bind:value={formData.gender}
        class="w-full px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
      >
        <option value="">{$t('patients.genderSelect')}</option>
        <option value="male">{$t('patients.male')}</option>
        <option value="female">{$t('patients.female')}</option>
        <option value="other">{$t('patients.other')}</option>
      </select>
    </div>
  </div>

  <!-- Contact Information -->
  <div class="grid grid-cols-2 gap-4">
    <div>
      <label for="phone" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{$t('patients.phone')}</label>
      <input
        type="tel"
        id="phone"
        bind:value={formData.phone}
        class="w-full px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
      />
    </div>

    <div>
      <label for="email" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{$t('patients.email')}</label>
      <input
        type="email"
        id="email"
        bind:value={formData.email}
        class="w-full px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
      />
    </div>
  </div>

  <!-- Address -->
  <div>
    <label for="address" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{$t('patients.address')}</label>
    <textarea
      id="address"
      bind:value={formData.address}
      rows="2"
      class="w-full px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
    ></textarea>
  </div>

  <!-- Insurance -->
  <div>
    <label for="insurance" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{$t('patients.insurance')}</label>
    <input
      type="text"
      id="insurance"
      bind:value={formData.insurance}
      class="w-full px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
    />
  </div>

  <!-- GP Information -->
  <div class="grid grid-cols-2 gap-4">
    <div>
      <label for="gp_name" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{$t('patients.gpName')}</label>
      <input
        type="text"
        id="gp_name"
        bind:value={formData.gp_name}
        class="w-full px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
      />
    </div>

    <div>
      <label for="gp_address" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
        {$t('patients.gpAddress')}
      </label>
      <input
        type="text"
        id="gp_address"
        bind:value={formData.gp_address}
        class="w-full px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
      />
    </div>
  </div>

  <!-- Notes -->
  <div>
    <label for="notes" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">{$t('patients.notes')}</label>
    <textarea
      id="notes"
      bind:value={formData.notes}
      rows="4"
      class="w-full px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-700 rounded-lg text-gray-900 dark:text-gray-100 focus:outline-none focus:border-blue-500"
    ></textarea>
  </div>

  <!-- Actions -->
  <div class="flex gap-4 justify-end">
    <button
      type="button"
      onclick={handleCancel}
      disabled={isSubmitting}
      class="px-6 py-2 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
    >
      {$t('common.cancel')}
    </button>
    <button
      type="submit"
      disabled={isSubmitting}
      class="px-6 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
    >
      {isSubmitting ? $t('patients.saving') : patient ? $t('patients.updatePatient') : $t('patients.createPatient')}
    </button>
  </div>
</form>
