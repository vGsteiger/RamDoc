import { describe, expect, it } from 'vitest';
import type { ChatMessageRow } from '$lib/api';
import {
  evidenceForTurn,
  provenanceForAnswer,
  unresolvedClaims,
  unsupportedDraftClaims,
} from '$lib/chat-provenance';

function message(overrides: Partial<ChatMessageRow>): ChatMessageRow {
  return {
    id: crypto.randomUUID(),
    session_id: 's1',
    role: 'tool_result',
    content: '[]',
    tool_name: null,
    tool_args_json: null,
    tool_result_for: null,
    created_at: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

describe('chat provenance', () => {
  it('uses only declared read-tool results as evidence, never assistant citation text', () => {
    const messages = [
      message({ role: 'user', content: 'Summarize the medication' }),
      message({ tool_name: 'list_medications', content: '[{"substance":"A"}]' }),
      message({ role: 'assistant', content: 'The medication is A [E999].' }),
    ];

    const provenance = provenanceForAnswer(messages, 2, true);
    expect(evidenceForTurn(messages, 2)).toEqual([
      { toolName: 'list_medications', status: 'supported' },
    ]);
    expect(provenance.status).toBe('inferred');
    expect(provenance.unverifiedCitations).toEqual(['[E999]']);
  });

  it('does not treat failed or write-tool output as a patient evidence source', () => {
    const messages = [
      message({ role: 'user' }),
      message({
        tool_name: 'write_report',
        content: '{"status":"pending_clinician_confirmation"}',
      }),
      message({ tool_name: 'list_medications', content: '{"error":"offline"}' }),
      message({ role: 'assistant', content: 'Answer' }),
    ];
    expect(provenanceForAnswer(messages, 3, true).status).toBe('unsupported');
  });

  it('requires explicit resolutions for missing typed sources and citation-looking draft claims', () => {
    const claims = unsupportedDraftClaims('Draft with [E7]', []);
    expect(claims.map((claim) => claim.id)).toEqual(['missing-typed-source', 'citation:[E7]']);
    expect(unresolvedClaims(claims, { 'missing-typed-source': 'uncertain' })).toHaveLength(1);
    expect(
      unresolvedClaims(claims, {
        'missing-typed-source': 'uncertain',
        'citation:[E7]': 'removed',
      })
    ).toHaveLength(0);
  });
});
