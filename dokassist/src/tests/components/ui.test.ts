import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import Badge from '$lib/components/ui/Badge.svelte';
import Button from '$lib/components/ui/Button.svelte';
import Card from '$lib/components/ui/Card.svelte';
import IconButton from '$lib/components/ui/IconButton.svelte';
import Input from '$lib/components/ui/Input.svelte';
import Spinner from '$lib/components/ui/Spinner.svelte';
import ThinkingIndicator from '$lib/components/ui/ThinkingIndicator.svelte';
import ListSkeleton from '$lib/components/ui/ListSkeleton.svelte';

/**
 * These guard the contract the rest of the app relies on: primitives must emit
 * design tokens, never raw palette classes, and must not silently drop the
 * accessibility affordances views depend on.
 */
const RAW_PALETTE =
  /\b(?:bg|text|border|ring|divide)-(?:gray|slate|zinc|neutral|blue|red|green|amber|yellow|purple|emerald|orange|indigo|sky)-[0-9]{2,3}\b/;

describe('Button', () => {
  it('renders a button element by default', () => {
    render(Button);
    expect(screen.getByRole('button')).toBeInTheDocument();
  });

  it('renders an anchor when href is given', () => {
    render(Button, { href: '/patients' });
    expect(screen.getByRole('link')).toHaveAttribute('href', '/patients');
  });

  it('applies the accent fill only for the primary variant', () => {
    const { unmount } = render(Button, { variant: 'primary' });
    expect(screen.getByRole('button')).toHaveClass('bg-accent');
    unmount();

    render(Button, { variant: 'secondary' });
    expect(screen.getByRole('button')).not.toHaveClass('bg-accent');
  });

  it('uses the danger token for destructive actions', () => {
    render(Button, { variant: 'danger' });
    expect(screen.getByRole('button')).toHaveClass('bg-danger');
  });

  it('disables itself while loading so the action cannot double-fire', () => {
    render(Button, { loading: true });
    expect(screen.getByRole('button')).toBeDisabled();
  });

  it('blocks pointer events when disabled', () => {
    // jsdom dispatches clicks to disabled elements, so assert the guard itself
    // rather than the event behaviour a real browser provides.
    render(Button, { disabled: true });
    const button = screen.getByRole('button');
    expect(button).toBeDisabled();
    expect(button).toHaveClass('disabled:pointer-events-none');
  });

  it('fires onclick when enabled', async () => {
    let clicks = 0;
    render(Button, { onclick: () => (clicks += 1) });
    await fireEvent.click(screen.getByRole('button'));
    expect(clicks).toBe(1);
  });

  // An <a> is never :disabled, so Tailwind's disabled: variants silently do
  // nothing on the href branch. Without an explicit guard an inert link stays
  // clickable and navigates.
  it('makes a disabled link inert while keeping its link semantics', () => {
    render(Button, { href: '/patients', disabled: true });
    const link = screen.getByRole('link');
    expect(link).toHaveClass('pointer-events-none');
    expect(link).toHaveAttribute('aria-disabled', 'true');
    expect(link).toHaveAttribute('tabindex', '-1');
    expect(link).toHaveAttribute('href');
  });

  it('treats a loading link as inert too', () => {
    render(Button, { href: '/patients', loading: true });
    const link = screen.getByRole('link');
    expect(link).toHaveClass('pointer-events-none');
    expect(link).toHaveAttribute('aria-disabled', 'true');
    expect(link).toHaveAttribute('href');
  });

  it('leaves an enabled link navigable', () => {
    render(Button, { href: '/patients' });
    const link = screen.getByRole('link');
    expect(link).toHaveAttribute('href', '/patients');
    expect(link).not.toHaveClass('pointer-events-none');
    expect(link).not.toHaveAttribute('aria-disabled');
  });

  it('keeps both control sizes on the shared 28/32px rhythm', () => {
    const { unmount } = render(Button, { size: 'sm' });
    expect(screen.getByRole('button')).toHaveClass('h-7');
    unmount();

    render(Button, { size: 'md' });
    expect(screen.getByRole('button')).toHaveClass('h-8');
  });

  it('emits no raw palette utilities', () => {
    render(Button, { variant: 'primary' });
    expect(screen.getByRole('button').className).not.toMatch(RAW_PALETTE);
  });
});

describe('Input', () => {
  it('marks itself invalid for assistive tech', () => {
    render(Input, { invalid: true });
    expect(screen.getByRole('textbox')).toHaveAttribute('aria-invalid', 'true');
  });

  it('carries no aria-invalid when valid', () => {
    render(Input);
    expect(screen.getByRole('textbox')).not.toHaveAttribute('aria-invalid');
  });

  it('signals invalidity with the danger border token', () => {
    render(Input, { invalid: true });
    expect(screen.getByRole('textbox')).toHaveClass('border-danger');
  });

  it('emits no raw palette utilities', () => {
    render(Input, { invalid: true });
    expect(screen.getByRole('textbox').className).not.toMatch(RAW_PALETTE);
  });
});

describe('IconButton', () => {
  it('always exposes an accessible name', () => {
    render(IconButton, { label: 'Delete medication' });
    expect(screen.getByRole('button')).toHaveAccessibleName('Delete medication');
  });

  it('makes a disabled link inert', () => {
    render(IconButton, { label: 'Edit', href: '/patients/1/edit', disabled: true });
    const link = screen.getByRole('link');
    expect(link).toHaveClass('pointer-events-none');
    expect(link).toHaveAttribute('aria-disabled', 'true');
    expect(link).toHaveAttribute('href');
  });
});

describe('Badge and Card', () => {
  it('maps each badge tone to its own token trio', () => {
    render(Badge, { tone: 'success' });
    const badge = screen.getByText((_, el) => el?.tagName === 'SPAN');
    expect(badge.className).toContain('bg-success-subtle');
    expect(badge.className).toContain('text-success-fg');
    expect(badge.className).not.toMatch(RAW_PALETTE);
  });

  it('separates cards with a hairline rather than a shadow', () => {
    const { container } = render(Card);
    const card = container.querySelector('div');
    expect(card).toHaveClass('border-line');
    expect(card?.className).not.toMatch(/shadow-/);
  });

  // List rows are clickable cards, so Card renders a real button rather than
  // leaving callers to hand-roll one with card classes.
  it('renders a button when given onclick, and fires it', async () => {
    let clicks = 0;
    render(Card, { onclick: () => (clicks += 1) });
    const card = screen.getByRole('button');
    expect(card).toHaveClass('rounded-card');
    await fireEvent.click(card);
    expect(clicks).toBe(1);
  });

  it('renders a link when given href', () => {
    render(Card, { href: '/patients/1' });
    expect(screen.getByRole('link')).toHaveAttribute('href', '/patients/1');
  });

  it('gains the hover affordance implicitly when actionable', () => {
    const { unmount } = render(Card, { onclick: () => {} });
    expect(screen.getByRole('button')).toHaveClass('hover:bg-surface-hover');
    unmount();

    const { container } = render(Card);
    expect(container.querySelector('div')?.className).not.toMatch(/hover:/);
  });
});

describe('Spinner', () => {
  it('exposes a status role with an accessible name', () => {
    render(Spinner);
    expect(screen.getByRole('status')).toHaveAccessibleName('Loading...');
  });

  it('uses the provided label as the accessible name', () => {
    render(Spinner, { label: 'Generating report' });
    expect(screen.getByRole('status')).toHaveTextContent('Generating report');
  });
});

describe('ListSkeleton', () => {
  it('exposes a status role named with the loading label', () => {
    render(ListSkeleton, { count: 3 });
    expect(screen.getByRole('status')).toHaveAccessibleName('Loading...');
  });

  it('renders the requested number of placeholder rows', () => {
    const { container } = render(ListSkeleton, { count: 3, variant: 'row' });
    expect(container.querySelectorAll('.skeleton').length).toBeGreaterThanOrEqual(3);
  });
});

describe('ThinkingIndicator', () => {
  it('exposes a live status with the current stage label', () => {
    render(ThinkingIndicator, { stage: 'thinking', startedAt: Date.now() });
    const status = screen.getByRole('status');
    expect(status).toHaveAttribute('aria-live', 'polite');
    expect(status).toHaveTextContent('Thinking');
  });

  it('names a completed tool lookup', () => {
    render(ThinkingIndicator, {
      stage: 'looking_up',
      startedAt: Date.now(),
      toolName: 'list_medications',
    });
    expect(screen.getByRole('status')).toHaveTextContent('Looked up medications');
  });

  it('uses an explicit label for non-chat activity such as model loading', () => {
    render(ThinkingIndicator, {
      startedAt: Date.now(),
      label: 'Loading phi4 into memory',
    });
    expect(screen.getByRole('status')).toHaveTextContent('Loading phi4 into memory');
  });

  it('keeps the elapsed clock visible but out of the live region', () => {
    render(ThinkingIndicator, { stage: 'thinking', startedAt: Date.now() - 3000 });
    const status = screen.getByRole('status');
    expect(status).toHaveAttribute('aria-live', 'polite');
    expect(status).toHaveTextContent('Thinking');
    expect(status.textContent).not.toMatch(/\d:\d{2}/);
    expect(screen.getByText('0:03')).toBeInTheDocument();
  });
});
