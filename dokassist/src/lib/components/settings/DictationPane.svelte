<script lang="ts">
  import { PageShell } from '$lib/components/ui';
  import DictationSettings from '$lib/components/DictationSettings.svelte';
  import { t } from '$lib/translations';
</script>

<PageShell width="form" title={$t('dictation.section')} description={$t('dictation.sectionDesc')}>
  <DictationSettings />
</PageShell>
