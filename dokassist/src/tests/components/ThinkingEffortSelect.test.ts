import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import { get } from 'svelte/store';
import ThinkingEffortSelect from '../../lib/components/ThinkingEffortSelect.svelte';
import { thinkingEffort } from '$lib/stores/thinking';

describe('ThinkingEffortSelect', () => {
  it('shows the Codex-style effort levels', () => {
    render(ThinkingEffortSelect);
    expect(screen.getByLabelText('Thinking')).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Low' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Medium' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'High' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'Extra high' })).toBeInTheDocument();
  });

  it('writes the selected effort to the shared store', async () => {
    thinkingEffort.set('medium');
    render(ThinkingEffortSelect);
    await fireEvent.change(screen.getByLabelText('Thinking'), { target: { value: 'high' } });
    expect(get(thinkingEffort)).toBe('high');
  });
});
