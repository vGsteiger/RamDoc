<script lang="ts">
  import type { Patient } from '$lib/api';
  import { ListRow } from '$lib/components/ui';
  import { ageInYears, fmt } from '$lib/format';
  import { t } from '$lib/translations';

  /*
   * One dense row of the patient list: name, date of birth with age, and the
   * AHV number (hidden below the md breakpoint so 700px windows never scroll
   * sideways). The whole row links to the patient workspace.
   */
  interface Props {
    patient: Patient;
    href?: string;
  }

  let { patient, href = undefined }: Props = $props();

  let age = $derived(ageInYears(patient.date_of_birth));
</script>

<ListRow href={href ?? `/patients/${patient.id}`}>
  <span class="min-w-0 flex-1 truncate text-body text-fg">
    <span class="font-medium">{patient.last_name}</span>, {patient.first_name}
  </span>
  <span class="w-36 shrink-0 truncate text-label text-fg-muted" data-numeric>
    {$fmt.date(patient.date_of_birth)}
    {#if age !== null}
      <span class="text-fg-subtle">({$t('patients.ageYears').replace('{age}', String(age))})</span>
    {/if}
  </span>
  <span class="hidden w-40 shrink-0 truncate text-label text-fg-muted md:block" data-numeric>
    {patient.ahv_number ?? ''}
  </span>
</ListRow>
