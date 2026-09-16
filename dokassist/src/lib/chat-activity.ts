export type ChatActivityStage = 'thinking' | 'reasoning' | 'looking_up' | 'writing';

export const THINK_START = '<think>';
export const THINK_END = '</think>';

/** Show the elapsed clock once the wait is long enough to notice. */
export const ELAPSED_AFTER_SECONDS = 2;

/** Swap "Thinking" for "Still thinking" so a long wait does not look frozen. */
export const STILL_THINKING_AFTER_SECONDS = 8;

const WRITE_TOOLS = new Set(['draft_email', 'write_report']);

/** Proposal/write tools; everything else is treated as a lookup. */
export function isWriteTool(toolName: string): boolean {
  return toolName.startsWith('create_') || WRITE_TOOLS.has(toolName);
}

export function chatActivityStage(content: string, toolName?: string | null): ChatActivityStage {
  if (toolName) return 'looking_up';
  if (!content.trim()) return 'thinking';
  if (content.startsWith(THINK_START)) {
    const end = content.indexOf(THINK_END);
    if (end === -1) return 'reasoning';
    const main = content.slice(end + THINK_END.length).trim();
    return main ? 'writing' : 'reasoning';
  }
  return 'writing';
}

export function formatElapsed(seconds: number): string {
  const clamped = Math.max(0, Math.floor(seconds));
  const m = Math.floor(clamped / 60);
  const s = clamped % 60;
  return `${m}:${String(s).padStart(2, '0')}`;
}

export function chatToolLabel(toolName: string, translate: (key: string) => string): string {
  const key = `chat.toolNames.${toolName}`;
  const translated = translate(key);
  return translated === key ? toolName.replaceAll('_', ' ') : translated;
}

export function chatToolActivityLabel(
  toolName: string,
  translate: (key: string) => string,
  inProgress = false
): string {
  const tool = chatToolLabel(toolName, translate);
  const key = inProgress
    ? isWriteTool(toolName)
      ? 'chat.activity.preparing'
      : 'chat.activity.lookingUp'
    : isWriteTool(toolName)
      ? 'chat.activity.prepared'
      : 'chat.activity.lookedUp';
  return translate(key).replace('{tool}', tool);
}

export function chatActivityLabel(
  stage: ChatActivityStage,
  elapsedSeconds: number,
  translate: (key: string) => string,
  toolName?: string | null
): string {
  if (stage === 'looking_up' && toolName) {
    return chatToolActivityLabel(toolName, translate);
  }
  if (stage === 'thinking' && elapsedSeconds >= STILL_THINKING_AFTER_SECONDS) {
    return translate('chat.activity.stillThinking');
  }
  return translate(`chat.activity.${stage}`);
}
