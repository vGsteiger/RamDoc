<script lang="ts">
  import { createEventDispatcher, untrack } from 'svelte';
  import type { Patient, CreatePatient, UpdatePatient } from '$lib/api';
  import AhvInput from './AhvInput.svelte';
  import { Button, Field, Input, Select, Textarea } from '$lib/components/ui';
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

  function toFormData(source: Patient | null) {
    return {
      ahv_number: source?.ahv_number || '',
      first_name: source?.first_name || '',
      last_name: source?.last_name || '',
      date_of_birth: source?.date_of_birth || '',
      gender: source?.gender || '',
      address: source?.address || '',
      phone: source?.phone || '',
      email: source?.email || '',
      insurance: source?.insurance || '',
      gp_name: source?.gp_name || '',
      gp_address: source?.gp_address || '',
      notes: source?.notes || '',
    };
  }

  // Snapshot the incoming patient for the initial render; the effect below keeps
  // the form in sync when a different patient is passed in later.
  let formData = $state(untrack(() => toFormData(patient)));

  $effect(() => {
    if (patient) {
      untrack(() => {
        formData = toFormData(patient);
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
        notes: formData.notes || null,
      };

      dispatch('submit', createData);
    }
  }

  function handleCancel() {
    dispatch('cancel');
  }
</script>

<form
  onsubmit={(e) => {
    e.preventDefault();
    handleSubmit();
  }}
  class="space-y-4"
>
  <Field label={$t('patients.ahvNumber')} for="ahv_number" required>
    <AhvInput id="ahv_number" bind:value={formData.ahv_number} error={errors.ahv_number} />
  </Field>

  <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
    <Field label={$t('patients.firstName')} for="first_name" required error={errors.first_name}>
      <Input id="first_name" bind:value={formData.first_name} invalid={!!errors.first_name} />
    </Field>

    <Field label={$t('patients.lastName')} for="last_name" required error={errors.last_name}>
      <Input id="last_name" bind:value={formData.last_name} invalid={!!errors.last_name} />
    </Field>
  </div>

  <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
    <Field
      label={$t('patients.dateOfBirth')}
      for="date_of_birth"
      required
      error={errors.date_of_birth}
    >
      <Input
        id="date_of_birth"
        type="date"
        bind:value={formData.date_of_birth}
        invalid={!!errors.date_of_birth}
      />
    </Field>

    <Field label={$t('patients.gender')} for="gender">
      <Select id="gender" bind:value={formData.gender}>
        <option value="">{$t('patients.genderSelect')}</option>
        <option value="male">{$t('patients.male')}</option>
        <option value="female">{$t('patients.female')}</option>
        <option value="other">{$t('patients.other')}</option>
      </Select>
    </Field>
  </div>

  <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
    <Field label={$t('patients.phone')} for="phone">
      <Input id="phone" type="tel" bind:value={formData.phone} />
    </Field>

    <Field label={$t('patients.email')} for="email">
      <Input id="email" type="email" bind:value={formData.email} />
    </Field>
  </div>

  <Field label={$t('patients.address')} for="address">
    <Textarea id="address" rows={2} bind:value={formData.address} />
  </Field>

  <Field label={$t('patients.insurance')} for="insurance">
    <Input id="insurance" bind:value={formData.insurance} />
  </Field>

  <div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
    <Field label={$t('patients.gpName')} for="gp_name">
      <Input id="gp_name" bind:value={formData.gp_name} />
    </Field>

    <Field label={$t('patients.gpAddress')} for="gp_address">
      <Input id="gp_address" bind:value={formData.gp_address} />
    </Field>
  </div>

  <Field label={$t('patients.notes')} for="notes">
    <Textarea id="notes" rows={4} bind:value={formData.notes} />
  </Field>

  <div class="flex justify-end gap-2 pt-2">
    <Button onclick={handleCancel} disabled={isSubmitting}>{$t('common.cancel')}</Button>
    <Button type="submit" variant="primary" loading={isSubmitting}>
      {isSubmitting
        ? $t('patients.saving')
        : patient
          ? $t('patients.updatePatient')
          : $t('patients.createPatient')}
    </Button>
  </div>
</form>
