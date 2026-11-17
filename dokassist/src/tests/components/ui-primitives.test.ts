import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent, within } from '@testing-library/svelte';
import { createRawSnippet, tick } from 'svelte';
import BrandMark from '$lib/components/ui/BrandMark.svelte';
import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
import HighlightedSnippet from '$lib/components/ui/HighlightedSnippet.svelte';
import ListRow from '$lib/components/ui/ListRow.svelte';
import Markdown from '$lib/components/ui/Markdown.svelte';
import Menu from '$lib/components/ui/Menu.svelte';
import PageShell from '$lib/components/ui/PageShell.svelte';
import ProgressBar from '$lib/components/ui/ProgressBar.svelte';
import SectionHeader from '$lib/components/ui/SectionHeader.svelte';
import SegmentedControl from '$lib/components/ui/SegmentedControl.svelte';
import SettingsRow from '$lib/components/ui/SettingsRow.svelte';
import StatusIndicator from '$lib/components/ui/StatusIndicator.svelte';
import Tabs from '$lib/components/ui/Tabs.svelte';
import Tooltip from '$lib/components/ui/Tooltip.svelte';

const html = (markup: string) => createRawSnippet(() => ({ render: () => markup }));

describe('PageShell', () => {
  it('renders one h1, actions, and the chosen width token', () => {
    const { container } = render(PageShell, {
      title: 'Patients',
      description: 'All active records.',
      width: 'form',
      actions: html('<button>New patient</button>'),
      children: html('<p>body</p>'),
    });
    expect(screen.getAllByRole('heading', { level: 1 })).toHaveLength(1);
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent('Patients');
    expect(screen.getByRole('button', { name: 'New patient' })).toBeInTheDocument();
    const shell = container.querySelector('[data-page-shell]');
    expect(shell).toHaveClass('max-w-form', 'px-page-x', 'mx-auto');
  });

  it('defaults to the wide page width', () => {
    const { container } = render(PageShell, { children: html('<p>x</p>') });
    expect(container.querySelector('[data-page-shell]')).toHaveClass('max-w-page');
    expect(screen.queryByRole('heading')).toBeNull();
  });
});

describe('SectionHeader', () => {
  it('renders the requested heading level in text-heading', () => {
    render(SectionHeader, { title: 'Diagnoses', level: 3 });
    const heading = screen.getByRole('heading', { level: 3 });
    expect(heading).toHaveTextContent('Diagnoses');
    expect(heading).toHaveClass('text-heading');
  });
});

describe('SettingsRow', () => {
  it('passes label and description ids to the control', () => {
    render(SettingsRow, {
      label: 'Theme',
      description: 'Light, dark or system',
      control: createRawSnippet((ids: () => { labelId: string; descriptionId?: string }) => ({
        render: () =>
          `<select aria-labelledby="${ids().labelId}" aria-describedby="${ids().descriptionId}"><option>x</option></select>`,
      })),
    });
    const select = screen.getByRole('combobox');
    expect(select).toHaveAccessibleName('Theme');
    expect(select).toHaveAccessibleDescription('Light, dark or system');
  });

  it('renders a real <label for> when given an input id', () => {
    const { container } = render(SettingsRow, { label: 'Practice name', for: 'practice-name' });
    expect(container.querySelector('label')).toHaveAttribute('for', 'practice-name');
  });

  it('keeps the control from shrinking or wrapping', () => {
    const { container } = render(SettingsRow, {
      label: 'Updates',
      control: html('<button>Check for Updates</button>'),
    });
    const control = screen.getByRole('button').parentElement;
    expect(control).toHaveClass('shrink-0', 'whitespace-nowrap');
    expect(container.querySelector('.\\@container')).not.toBeNull();
  });
});

describe('ListRow', () => {
  it('renders a stretched link with trailing actions outside it', () => {
    render(ListRow, {
      title: 'Anna Müller',
      secondary: '12.03.1984',
      meta: 'Today',
      href: '/patients/p1',
      selected: true,
      trailing: html('<button>More</button>'),
    });
    const link = screen.getByRole('link');
    expect(link).toHaveAttribute('href', '/patients/p1');
    expect(link).toHaveAttribute('aria-current', 'page');
    expect(link).toHaveTextContent('Anna Müller');
    // The action button is not nested inside the link.
    expect(within(link).queryByRole('button')).toBeNull();
    expect(screen.getByRole('button', { name: 'More' })).toBeInTheDocument();
  });

  it('fires onclick as a button row', async () => {
    const onclick = vi.fn();
    render(ListRow, { title: 'Chat', onclick });
    await fireEvent.click(screen.getByRole('button'));
    expect(onclick).toHaveBeenCalledOnce();
  });
});

describe('Tabs', () => {
  const items = [
    { id: 'overview', label: 'Overview' },
    { id: 'sessions', label: 'Sessions', count: 4 },
    { id: 'documents', label: 'Documents', disabled: true },
    { id: 'diagnoses', label: 'Diagnoses' },
  ];

  it('exposes a tablist with one selected, tabbable tab', () => {
    render(Tabs, { items, value: 'sessions', label: 'Patient sections' });
    expect(screen.getByRole('tablist')).toHaveAccessibleName('Patient sections');
    const selected = screen.getByRole('tab', { selected: true });
    expect(selected).toHaveTextContent('Sessions');
    expect(selected).toHaveAttribute('tabindex', '0');
    expect(selected).toHaveClass('border-accent');
    expect(screen.getByRole('tab', { name: 'Overview' })).toHaveAttribute('tabindex', '-1');
  });

  it('moves selection with arrow keys, skipping disabled tabs, and wraps', async () => {
    const onchange = vi.fn();
    render(Tabs, { items, value: 'sessions', label: 'Sections', onchange });
    const list = screen.getByRole('tablist');
    await fireEvent.keyDown(list, { key: 'ArrowRight' });
    await tick();
    expect(onchange).toHaveBeenLastCalledWith('diagnoses');
    await fireEvent.keyDown(list, { key: 'ArrowRight' });
    expect(onchange).toHaveBeenLastCalledWith('overview');
    await fireEvent.keyDown(list, { key: 'End' });
    expect(onchange).toHaveBeenLastCalledWith('diagnoses');
    await fireEvent.keyDown(list, { key: 'Home' });
    expect(onchange).toHaveBeenLastCalledWith('overview');
  });

  it('scrolls inside its own box instead of widening the page', () => {
    const { container } = render(Tabs, { items, value: 'overview', label: 'Sections' });
    expect(container.firstElementChild).toHaveClass('overflow-x-auto', 'max-w-full', 'min-w-0');
  });

  it('renders route tabs as links with aria-current', () => {
    render(Tabs, {
      items: [
        { id: 'a', label: 'Overview', href: '/p/1' },
        { id: 'b', label: 'Sessions', href: '/p/1/sessions' },
      ],
      value: 'b',
      label: 'Patient',
    });
    expect(screen.getByRole('navigation')).toHaveAccessibleName('Patient');
    expect(screen.getByRole('link', { name: 'Sessions' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('link', { name: 'Overview' })).not.toHaveAttribute('aria-current');
  });
});

describe('SegmentedControl', () => {
  const options = [
    { value: 'week', label: 'Week' },
    { value: 'month', label: 'Month' },
  ];

  it('is a radiogroup with the selected option checked', () => {
    render(SegmentedControl, { options, value: 'week', label: 'View' });
    expect(screen.getByRole('radiogroup')).toHaveAccessibleName('View');
    expect(screen.getByRole('radio', { name: 'Week' })).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByRole('radio', { name: 'Month' })).toHaveAttribute('aria-checked', 'false');
  });

  it('changes on click and arrow keys', async () => {
    const onchange = vi.fn();
    render(SegmentedControl, { options, value: 'week', label: 'View', onchange });
    await fireEvent.click(screen.getByRole('radio', { name: 'Month' }));
    expect(onchange).toHaveBeenLastCalledWith('month');
    await fireEvent.keyDown(screen.getByRole('radiogroup'), { key: 'ArrowRight' });
    expect(onchange).toHaveBeenLastCalledWith('week');
  });
});

describe('Menu', () => {
  it('opens, navigates with arrows, selects, and closes', async () => {
    const edit = vi.fn();
    const remove = vi.fn();
    render(Menu, {
      label: 'Patient actions',
      items: [
        { label: 'Edit', onSelect: edit },
        { label: 'Export', onSelect: () => {}, disabled: true },
        { label: 'Delete', onSelect: remove, danger: true },
      ],
    });
    const trigger = screen.getByRole('button', { name: 'Patient actions' });
    expect(trigger).toHaveAttribute('aria-haspopup', 'menu');
    expect(trigger).toHaveAttribute('aria-expanded', 'false');

    await fireEvent.click(trigger);
    await tick();
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    const menu = screen.getByRole('menu');
    const items = within(menu).getAllByRole('menuitem');
    expect(items).toHaveLength(3);
    expect(document.activeElement).toBe(items[0]);
    // The danger item is separated from the rest.
    expect(within(menu).getByRole('separator')).toBeInTheDocument();
    expect(items[2]).toHaveClass('text-danger-fg');

    await fireEvent.keyDown(menu, { key: 'ArrowDown' });
    // Disabled items are skipped.
    expect(document.activeElement).toBe(items[2]);

    await fireEvent.click(items[2]);
    expect(remove).toHaveBeenCalledOnce();
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it('closes on Escape and returns focus to the trigger', async () => {
    render(Menu, { items: [{ label: 'Rename', onSelect: () => {} }] });
    const trigger = screen.getByRole('button', { name: 'More actions' });
    await fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    await tick();
    await fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' });
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });
});

describe('ConfirmDialog', () => {
  it('names the record, focuses Cancel first for danger, and confirms', async () => {
    const onConfirm = vi.fn();
    render(ConfirmDialog, {
      open: true,
      title: 'Delete diagnosis?',
      message: 'F33.1 will be removed from this record.',
      tone: 'danger',
      onConfirm,
    });
    await tick();
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAccessibleName('Delete diagnosis?');
    expect(dialog).toHaveTextContent('F33.1 will be removed from this record.');
    expect(dialog).toHaveTextContent('This cannot be undone.');
    expect(document.activeElement).toHaveTextContent('Cancel');

    const confirm = screen.getByRole('button', { name: 'Delete' });
    expect(confirm).toHaveClass('bg-danger');
    await fireEvent.click(confirm);
    await tick();
    expect(onConfirm).toHaveBeenCalledOnce();
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('cancels on Escape without confirming', async () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    render(ConfirmDialog, { open: true, title: 'Discard draft?', onConfirm, onCancel });
    await tick();
    await fireEvent.keyDown(window, { key: 'Escape' });
    await tick();
    expect(onConfirm).not.toHaveBeenCalled();
    expect(onCancel).toHaveBeenCalledOnce();
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('stays open when onConfirm throws', async () => {
    render(ConfirmDialog, {
      open: true,
      title: 'Delete?',
      tone: 'danger',
      onConfirm: () => Promise.reject(new Error('locked')),
    });
    await tick();
    await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
    await tick();
    await tick();
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('uses the ink primary for non-destructive confirmations', async () => {
    render(ConfirmDialog, { open: true, title: 'Send letter?', onConfirm: () => {} });
    await tick();
    expect(screen.getByRole('button', { name: 'Confirm' })).toHaveClass('bg-accent');
  });
});

describe('StatusIndicator', () => {
  it('always shows a text label next to an icon', () => {
    const { container } = render(StatusIndicator, { tone: 'ok', label: 'Model ready' });
    expect(screen.getByText('Model ready')).toBeInTheDocument();
    expect(container.querySelector('svg')).not.toBeNull();
  });

  it('becomes a >=24px control when interactive', async () => {
    const onclick = vi.fn();
    render(StatusIndicator, { tone: 'warn', label: 'Load model', onclick });
    const button = screen.getByRole('button', { name: 'Load model' });
    expect(button).toHaveClass('min-h-6');
    await fireEvent.click(button);
    expect(onclick).toHaveBeenCalledOnce();
  });
});

describe('ProgressBar', () => {
  it('exposes determinate progress', () => {
    render(ProgressBar, { value: 42, label: 'Downloading model', showValue: true });
    const bar = screen.getByRole('progressbar');
    expect(bar).toHaveAccessibleName('Downloading model');
    expect(bar).toHaveAttribute('aria-valuenow', '42');
    expect(screen.getByText('42%')).toBeInTheDocument();
  });

  it('clamps and supports indeterminate progress', () => {
    const { unmount } = render(ProgressBar, { value: 250 });
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '100');
    unmount();
    render(ProgressBar, { value: null });
    const bar = screen.getByRole('progressbar');
    expect(bar).not.toHaveAttribute('aria-valuenow');
    expect(bar).toHaveAccessibleName('Progress');
  });
});

describe('BrandMark', () => {
  it('is decorative by default and tinted with the brand token', () => {
    const { container } = render(BrandMark, { size: 32 });
    const mark = container.querySelector('[data-brand-mark]');
    expect(mark).toHaveAttribute('aria-hidden', 'true');
    expect(mark).toHaveClass('text-brand');
    expect(mark).toHaveStyle({ width: '32px', height: '32px' });
    expect(mark?.querySelector('svg')).not.toBeNull();
  });

  it('becomes a named image when given a title', () => {
    render(BrandMark, { title: 'RamDoc' });
    expect(screen.getByRole('img', { name: 'RamDoc' })).toBeInTheDocument();
  });
});

describe('Tooltip', () => {
  it('renders a tooltip and hands its id to the trigger', () => {
    render(Tooltip, {
      text: 'Patients',
      children: createRawSnippet((id: () => string) => ({
        render: () => `<button aria-label="Patients" aria-describedby="${id()}">P</button>`,
      })),
    });
    const tooltip = screen.getByRole('tooltip', { hidden: true });
    expect(tooltip).toHaveTextContent('Patients');
    expect(screen.getByRole('button')).toHaveAttribute('aria-describedby', tooltip.id);
  });
});

describe('HighlightedSnippet', () => {
  it('highlights backend <mark> matches without injecting HTML', () => {
    const { container } = render(HighlightedSnippet, {
      snippet: 'Exploration der <mark>Belastung</mark> <img src=x onerror="alert(1)">',
    });
    const mark = container.querySelector('mark');
    expect(mark).toHaveTextContent('Belastung');
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('<img src=x onerror="alert(1)">');
  });

  it('never creates elements from a hostile match', () => {
    const { container } = render(HighlightedSnippet, {
      snippet: '<mark><script>alert(1)</script></mark>',
    });
    expect(container.querySelector('script')).toBeNull();
    expect(container.querySelector('mark')).toHaveTextContent('<script>alert(1)</script>');
  });

  it('highlights client-side query matches', () => {
    const { container } = render(HighlightedSnippet, { text: 'Anna Müller', query: 'mül' });
    expect(container.querySelector('mark')).toHaveTextContent('Mül');
  });
});

describe('Markdown', () => {
  it('renders the formatting chat replies use', () => {
    const { container } = render(Markdown, {
      source:
        'In den letzten Sitzungen:\n\n- **Schlaf**: besser\n- *Antrieb*: reduziert\n\n`PHQ-9`',
    });
    expect(container.querySelector('ul')?.children).toHaveLength(2);
    expect(container.querySelector('strong')).toHaveTextContent('Schlaf');
    expect(container.querySelector('em')).toHaveTextContent('Antrieb');
    expect(container.querySelector('code')).toHaveTextContent('PHQ-9');
    expect(container.textContent).not.toContain('**');
  });

  it.each([
    ['<script>alert(1)</script>', 'script'],
    ['<img src=x onerror=alert(1)>', 'img'],
    ['<iframe src="https://evil.example"></iframe>', 'iframe'],
    ['Text <svg onload=alert(1)></svg>', 'svg'],
    ['<a href="javascript:alert(1)">x</a>', 'a'],
    ['<div onclick="alert(1)">raw</div>', 'div[onclick]'],
  ])('shows raw HTML %j as text', (source, selector) => {
    const { container } = render(Markdown, { source });
    expect(container.querySelector(selector)).toBeNull();
    expect(container.textContent).toContain(source.replace(/^Text /, ''));
  });

  it('never renders an unsafe link target', () => {
    const { container } = render(Markdown, {
      source: '[a](javascript:alert(1)) [b](data:text/html,x) [c](https://ok.example)',
    });
    const links = container.querySelectorAll('a');
    expect(links).toHaveLength(1);
    expect(links[0]).toHaveAttribute('href', 'https://ok.example/');
    expect(links[0]).toHaveAttribute('rel', 'noopener noreferrer');
    expect(container.textContent).toContain('a');
  });

  it('never loads images', () => {
    const { container } = render(Markdown, { source: '![Befund](https://tracker.example/p.png)' });
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('Befund');
  });

  it('decodes entities into text, not markup', () => {
    const { container } = render(Markdown, { source: '&lt;b&gt;bold&lt;/b&gt; &amp; more' });
    expect(container.querySelector('b')).toBeNull();
    expect(container.textContent).toContain('<b>bold</b> & more');
  });

  it('shifts headings below the page h1', () => {
    render(Markdown, { source: '# Verlaufsbericht' });
    expect(screen.queryByRole('heading', { level: 1 })).toBeNull();
    expect(screen.getByRole('heading', { level: 3 })).toHaveTextContent('Verlaufsbericht');
  });

  it('renders tables and task lists', () => {
    const { container } = render(Markdown, {
      source: '| Skala | Wert |\n|---|---:|\n| PHQ-9 | 11 |\n\n- [x] erledigt',
    });
    expect(container.querySelector('th')).toHaveTextContent('Skala');
    expect(container.querySelectorAll('td')[1]).toHaveClass('text-right');
    expect(container.querySelector('input[type="checkbox"]')).toBeChecked();
  });
});
