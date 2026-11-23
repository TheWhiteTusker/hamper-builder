/**
 * What the combo box lists. While typing: matches from every option, the
 * current group's first. Otherwise (or with nothing typed): the current group.
 */
export function filterComboOptions(options: string[], allOptions: string[] | undefined, query: string, isTyping: boolean) {
  const q = query.trim().toLowerCase();
  if (!isTyping || !q) return options;
  const source = allOptions && allOptions.length > 0 ? allOptions : options;
  const current = new Set(options);
  const matches = source.filter((opt) => opt.toLowerCase().includes(q));
  return [...matches.filter((opt) => current.has(opt)), ...matches.filter((opt) => !current.has(opt))];
}
