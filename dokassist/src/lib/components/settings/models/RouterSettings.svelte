<script lang="ts">
  import type { ModelInfo } from '$lib/api';
  import { Button, Select, SettingsRow, StatusIndicator } from '$lib/components/ui';
  import { router, setRouterOverride, unloadRouter } from '$lib/stores/router';
  import { t } from '$lib/translations';

  /*
   * The tool-routing model: managed (smallest compatible installed model) or
   * an expert override, plus what is resident in memory right now. The
   * router store records failures, so the calls here don't rethrow.
   */
  let { installedModels }: { installedModels: ModelInfo[] } = $props();

  let onDisk = $derived(installedModels.filter((model) => model.exists_on_disk));
  let activeFilename = $derived($router.diagnostics?.active_model_filename ?? null);

  function onChange(event: Event & { currentTarget: HTMLSelectElement }) {
    const filename = event.currentTarget.value;
    const modelId = filename
      ? (installedModels.find((model) => model.filename === filename)?.id ?? null)
      : null;
    setRouterOverride(modelId).catch(() => {});
  }
</script>

<SettingsRow
  label={$t('settings.routerModel')}
  description={$t('settings.routerModelDescription')}
  for="router-model"
>
  {#snippet control()}
    <div class="w-full @min-[560px]:w-60">
      <Select
        id="router-model"
        disabled={$router.isMutating}
        value={$router.diagnostics?.advanced_override.configured_filename ?? ''}
        onchange={onChange}
      >
        <option value="">{$t('settings.routerManaged')}</option>
        {#each onDisk as model (model.id)}
          <option value={model.filename}>{model.name}</option>
        {/each}
      </Select>
    </div>
  {/snippet}
  {#if $router.diagnostics?.fallback_reason}
    <p class="mt-2 text-caption text-warning-fg">{$router.diagnostics.fallback_reason}</p>
  {/if}
</SettingsRow>

<SettingsRow
  label={$t('settings.models.routerResident')}
  description={$t('settings.models.routerResidentDesc')}
>
  {#snippet control()}
    <Button
      size="sm"
      disabled={$router.isMutating || !activeFilename}
      onclick={() => unloadRouter().catch(() => {})}
    >
      {$t('settings.unloadRouter')}
    </Button>
  {/snippet}
  <div class="mt-1.5">
    {#if activeFilename}
      <StatusIndicator tone="ok" label={activeFilename} />
    {:else}
      <StatusIndicator tone="idle" label={$t('settings.models.routerNotLoaded')} />
    {/if}
  </div>
  {#if $router.error}
    <p class="mt-2 text-caption text-danger-fg">{$router.error}</p>
  {/if}
</SettingsRow>
