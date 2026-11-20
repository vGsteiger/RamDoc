<script lang="ts">
  import { get } from 'svelte/store';
  import { goto } from '$app/navigation';
  import { FileDown, Mail, MessageSquarePlus, Pencil, Trash2 } from 'lucide-svelte';
  import { deletePatient, type Diagnosis, type Medication, type Patient } from '$lib/api';
  import { Button, ConfirmDialog, Menu, type MenuItem } from '$lib/components/ui';
  import { ageInYears, fmt } from '$lib/format';
  import { t } from '$lib/translations';
  import { errorText } from '$lib/translations/labels';
  import { addToast } from '$lib/stores/toast';
  import { exportPatientFhir, exportPatientSummaryPdf } from './export';
  import {
    activeDiagnoses,
    activeMedications,
    maskAhv,
    newPatientChatHref,
    patientPath,
  } from './workspace';

  /*
   * The patient identification strip shown on every patient route (D4):
   * name, age, date of birth and masked AHV on one line, the active
   * diagnoses and current medications as chips on the next, and the actions
   * on the right: "New chat" and a ⋯ menu (Edit, email, exports, Delete).
   * Kept to two short rows so header + tabs stay under ~110px at 700×600.
   */
  let {
    patient,
    diagnoses = [],
    medications = [],
    newChatPrimary = true,
  }: {
    patient: Patient;
    diagnoses?: Diagnosis[];
    medications?: Medication[];
    /** Steps down to secondary on routes that have their own primary (Save). */
    newChatPrimary?: boolean;
  } = $props();

  const MAX_CHIPS = 3;

  let showDelete = $state(false);
  let exporting = $state(false);

  let fullName = $derived(`${patient.first_name} ${patient.last_name}`.trim());
  let age = $derived(ageInYears(patient.date_of_birth));
  let dx = $derived(activeDiagnoses(diagnoses));
  let meds = $derived(activeMedications(medications));
  let chips = $derived([
    ...dx.slice(0, MAX_CHIPS).map((d) => ({
      id: d.id,
      label: d.icd10_code,
      title: `${d.icd10_code} ${d.description}`,
      href: patientPath(patient.id, 'diagnoses'),
      kind: 'dx' as const,
    })),
    ...meds.slice(0, MAX_CHIPS).map((m) => ({
      id: m.id,
      label: [m.substance, m.dosage].filter(Boolean).join(' '),
      title: [m.substance, m.dosage, m.frequency].filter(Boolean).join(' · '),
      href: patientPath(patient.id, 'medications'),
      kind: 'med' as const,
    })),
  ]);
  let hiddenCount = $derived(
    Math.max(0, dx.length - MAX_CHIPS) + Math.max(0, meds.length - MAX_CHIPS)
  );

  async function runExport(kind: 'fhir' | 'pdf') {
    if (exporting) return;
    exporting = true;
    try {
      if (kind === 'fhir') await exportPatientFhir(patient);
      else await exportPatientSummaryPdf(patient);
    } catch (e) {
      const fallback = get(t)(
        kind === 'fhir' ? 'patients.exportFhirFailed' : 'patients.exportSummaryFailed'
      );
      addToast(get(errorText)(e, fallback), 'error');
      console.error('Patient export failed:', e);
    } finally {
      exporting = false;
    }
  }

  let menuItems = $derived<MenuItem[]>([
    {
      label: $t('patients.editPatient'),
      icon: Pencil,
      onSelect: () => goto(patientPath(patient.id, 'edit')),
    },
    {
      label: $t('patients.sendEmail'),
      icon: Mail,
      onSelect: () => goto(patientPath(patient.id, 'email/new')),
    },
    {
      label: $t('patients.exportFhir'),
      icon: FileDown,
      disabled: exporting,
      separator: true,
      onSelect: () => runExport('fhir'),
    },
    {
      label: $t('patients.exportPdf'),
      icon: FileDown,
      disabled: exporting,
      onSelect: () => runExport('pdf'),
    },
    {
      label: $t('patients.deletePatient'),
      icon: Trash2,
      danger: true,
      onSelect: () => (showDelete = true),
    },
  ]);

  async function confirmDelete() {
    try {
      await deletePatient(patient.id);
      await goto('/patients');
    } catch (e) {
      addToast(get(errorText)(e, get(t)('patients.deleteFailed')), 'error');
      console.error('Error deleting patient:', e);
      throw e;
    }
  }
</script>

<div class="flex min-w-0 flex-col gap-1.5" data-patient-header>
  <div class="flex min-w-0 items-center gap-3">
    <div class="flex min-w-0 flex-1 items-baseline gap-x-3 overflow-hidden">
      <h1 class="min-w-0 shrink truncate text-title font-semibold text-fg" title={fullName}>
        {fullName}
      </h1>
      <p class="min-w-0 truncate text-label text-fg-muted" data-numeric>
        {#if age !== null}
          <span>{$t('patients.ageYears').replace('{age}', String(age))}</span>
          <span aria-hidden="true" class="px-1 text-fg-subtle">·</span>
        {/if}
        <span title={$t('patients.dateOfBirth')}>{$fmt.date(patient.date_of_birth)}</span>
        {#if patient.ahv_number}
          <span aria-hidden="true" class="px-1 text-fg-subtle">·</span>
          <span title={$t('patients.ahvNumber')}>
            {$t('patients.ahvShort')}
            {maskAhv(patient.ahv_number)}
          </span>
        {/if}
      </p>
    </div>
    <div class="flex shrink-0 items-center gap-1.5">
      <Button
        variant={newChatPrimary ? 'primary' : 'secondary'}
        size="sm"
        href={newPatientChatHref(patient.id)}
      >
        <MessageSquarePlus size={14} aria-hidden="true" />
        {$t('patients.newChat')}
      </Button>
      <Menu items={menuItems} size="sm" label={$t('patients.patientActions')} />
    </div>
  </div>

  {#if chips.length > 0}
    <ul
      class="flex h-5 min-w-0 items-center gap-1.5 overflow-hidden"
      aria-label={$t('patients.keyFacts')}
    >
      {#each chips as chip (chip.kind + chip.id)}
        <li class="shrink-0">
          <a
            href={chip.href}
            title={chip.title}
            class="inline-flex h-5 max-w-56 items-center gap-1 rounded-control border px-1.5 text-caption transition-colors duration-150 ease-standard hover:bg-surface-hover {chip.kind ===
            'dx'
              ? 'border-line font-mono text-fg'
              : 'border-line-subtle text-fg-muted'}"
          >
            <span class="truncate">{chip.label}</span>
          </a>
        </li>
      {/each}
      {#if hiddenCount > 0}
        <li class="shrink-0 text-caption text-fg-subtle" data-numeric>
          {$t('patients.moreCount').replace('{count}', String(hiddenCount))}
        </li>
      {/if}
    </ul>
  {/if}
</div>

<ConfirmDialog
  bind:open={showDelete}
  tone="danger"
  title={$t('patients.deletePatient')}
  message={$t('patients.confirmDeleteText').replace('{name}', fullName)}
  confirmLabel={$t('patients.deletePatient')}
  onConfirm={confirmDelete}
/>
