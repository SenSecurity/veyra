export interface HighlightPart {
  text: string;
  match: boolean;
}

export function tokenizeQuery(query: string): string[] {
  return Array.from(
    new Set(
      query
        .toLowerCase()
        .split(/\s+/)
        .map((token) => token.trim())
        .filter(Boolean),
    ),
  );
}

export function highlightParts(text: string, query: string): HighlightPart[] {
  const tokens = tokenizeQuery(query);
  if (tokens.length === 0) return [{ text, match: false }];

  const escaped = tokens.map((token) =>
    token.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
  );
  const re = new RegExp(`(${escaped.join("|")})`, "gi");
  const parts = text.split(re).filter((part) => part.length > 0);
  return parts.map((part) => ({
    text: part,
    match: tokens.includes(part.toLowerCase()),
  }));
}

