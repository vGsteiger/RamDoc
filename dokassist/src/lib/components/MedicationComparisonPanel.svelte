<script lang="ts">
  import type { SubstanceDetail } from '$lib/api';
  import { t } from '$lib/translations';
  import { Alert } from '$lib/components/ui';

  interface Props {
    current: SubstanceDetail;
    replacement: SubstanceDetail;
  }

  let { current, replacement }: Props = $props();
</script>

<div class="bg-surface-raised border border-line rounded-card p-4">
  <h3 class="text-heading font-semibold text-fg mb-4">{$t('medications.comparison')}</h3>

  <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
    <!-- Current Medication Column -->
    <div class="border-r border-line pr-4">
      <div class="mb-4">
        <h4 class="text-body font-medium text-fg-muted mb-1">
          {$t('medications.currentMedication')}
        </h4>
        <p class="text-heading font-semibold text-fg">
          {current.name_de}
        </p>
        {#if current.atc_code}
          <p class="text-body text-fg-muted">ATC: {current.atc_code}</p>
        {/if}
      </div>

      {#if current.trade_names && current.trade_names.length > 0}
        <div class="mb-4">
          <h5 class="text-body font-medium text-fg-muted mb-1">{$t('medications.tradeNames')}</h5>
          <p class="text-body text-fg-muted">
            {current.trade_names.join(', ')}
          </p>
        </div>
      {/if}

      {#if current.indication}
        <div class="mb-4">
          <h5 class="text-body font-medium text-fg-muted mb-1">{$t('medications.indication')}</h5>
          <p class="text-body text-fg-muted line-clamp-4">
            {current.indication}
          </p>
        </div>
      {/if}

      {#if current.side_effects}
        <div class="mb-4">
          <h5 class="text-body font-medium text-fg-muted mb-1">{$t('medications.sideEffects')}</h5>
          <p class="text-body text-fg-muted line-clamp-4">
            {current.side_effects}
          </p>
        </div>
      {/if}

      {#if current.contraindications}
        <div class="mb-4">
          <h5 class="text-body font-medium text-fg-muted mb-1">
            {$t('medications.contraindications')}
          </h5>
          <p class="text-body text-fg-muted line-clamp-4">
            {current.contraindications}
          </p>
        </div>
      {/if}
    </div>

    <!-- Replacement Medication Column -->
    <div class="pl-4">
      <div class="mb-4">
        <h4 class="text-body font-medium text-fg-muted mb-1">
          {$t('medications.replacementMedication')}
        </h4>
        <p class="text-heading font-semibold text-fg">
          {replacement.name_de}
        </p>
        {#if replacement.atc_code}
          <p class="text-body text-fg-muted">ATC: {replacement.atc_code}</p>
        {/if}
      </div>

      {#if replacement.trade_names && replacement.trade_names.length > 0}
        <div class="mb-4">
          <h5 class="text-body font-medium text-fg-muted mb-1">{$t('medications.tradeNames')}</h5>
          <p class="text-body text-fg-muted">
            {replacement.trade_names.join(', ')}
          </p>
        </div>
      {/if}

      {#if replacement.indication}
        <div class="mb-4">
          <h5 class="text-body font-medium text-fg-muted mb-1">{$t('medications.indication')}</h5>
          <p class="text-body text-fg-muted line-clamp-4">
            {replacement.indication}
          </p>
        </div>
      {/if}

      {#if replacement.side_effects}
        <div class="mb-4">
          <h5 class="text-body font-medium text-fg-muted mb-1">{$t('medications.sideEffects')}</h5>
          <p class="text-body text-fg-muted line-clamp-4">
            {replacement.side_effects}
          </p>
        </div>
      {/if}

      {#if replacement.contraindications}
        <div class="mb-4">
          <h5 class="text-body font-medium text-fg-muted mb-1">
            {$t('medications.contraindications')}
          </h5>
          <p class="text-body text-fg-muted line-clamp-4">
            {replacement.contraindications}
          </p>
        </div>
      {/if}
    </div>
  </div>

  <!-- Comparison Notes -->
  {#if current.atc_code === replacement.atc_code && current.atc_code}
    <Alert tone="info" class="mt-4">{$t('medications.sameAtcNote')}</Alert>
  {/if}
</div>
