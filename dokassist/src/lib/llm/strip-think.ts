/** Drop completed or truncated `<think>` blocks from model output. */
export function stripThinkTags(content: string): string {
  return content
    .replace(/<think>[\s\S]*?<\/think>/g, '')
    .replace(/<think>[\s\S]*$/g, '')
    .trim();
}
