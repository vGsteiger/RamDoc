<script lang="ts">
  import type { Snippet } from 'svelte';
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { Select } from '$lib/components/ui';
  import {
    SETTINGS_SECTIONS,
    sectionFromPath,
    type SettingsPath,
  } from '$lib/components/settings/sections';
  import { t } from '$lib/translations';

  /*
   * Settings as sub-pages with their own section list (D6).
   *
   * The layout responds to the width of the content area, not the window
   * (a container query), because the app sidebar's width varies: with at
   * least 720px of content the section list sits beside the pane; below
   * that it collapses to a Select above the pane. A Select keeps the current
   * section visible and switches in one step, where a separate list page
   * would cost a back-navigation for every switch.
   */
  let { children }: { children: Snippet } = $props();

  let current = $derived(sectionFromPath(page.url.pathname));

  function onSelect(event: Event & { currentTarget: HTMLSelectElement }) {
    const path = event.currentTarget.value as SettingsPath;
    void goto(resolve(path));
  }
</script>

<div class="@container min-h-full">
  <div class="flex min-h-full w-full min-w-0 flex-col @min-[720px]:flex-row">
    <nav
      aria-label={$t('settings.nav.label')}
      class="hidden shrink-0 border-line-subtle @min-[720px]:block @min-[720px]:w-60 @min-[720px]:border-r"
    >
      <div class="sticky top-0 px-3 py-page-y">
        <p class="mb-2 px-2.5 text-title text-fg">{$t('settings.title')}</p>
        <ul class="space-y-0.5">
          {#each SETTINGS_SECTIONS as section (section.path)}
            {@const active = current?.path === section.path}
            {@const Icon = section.icon}
            <li>
              <a
                href={resolve(section.path)}
                aria-current={active ? 'page' : undefined}
                class="flex h-control min-w-0 items-center gap-2 rounded-control px-2.5 text-body transition-colors duration-150 ease-standard {active
                  ? 'bg-fill-strong font-medium text-fg'
                  : 'text-fg-muted hover:bg-fill hover:text-fg'}"
              >
                <Icon size={16} strokeWidth={1.5} class="shrink-0" aria-hidden="true" />
                <span class="truncate">{$t(section.labelKey)}</span>
              </a>
            </li>
          {/each}
        </ul>
      </div>
    </nav>

    <div class="px-page-x pt-page-y @min-[720px]:hidden">
      <label for="settings-section" class="field-label">{$t('settings.title')}</label>
      <div class="max-w-xs">
        <Select id="settings-section" value={current?.path ?? ''} onchange={onSelect}>
          {#each SETTINGS_SECTIONS as section (section.path)}
            <option value={section.path}>{$t(section.labelKey)}</option>
          {/each}
        </Select>
      </div>
    </div>

    <div class="min-w-0 flex-1">
      {@render children()}
    </div>
  </div>
</div>
