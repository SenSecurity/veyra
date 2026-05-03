/**
 * Shared formatters for engine names surfaced in the chrome (titlebar
 * EngineBadge, sidebar EngineCard, overlay capsule). Centralised so the
 * Whisper / Llama / Groq display strings stay in lock-step across
 * surfaces.
 */

export function formatWhisperName(raw: string | undefined): string {
  if (!raw) return "Whisper · Turbo";
  const lower = raw.toLowerCase();
  if (lower.includes("large-v3-turbo") || lower === "turbo") {
    return "Whisper · Turbo";
  }
  if (lower.includes("medium")) return "Whisper · Medium";
  if (lower.includes("small")) return "Whisper · Small";
  if (lower.includes("base")) return "Whisper · Base";
  return `Whisper · ${raw.replace(/^ggml-|\.bin$/g, "")}`;
}

export function formatDrafterName(
  engine: "ollama" | "groq" | undefined,
  model: string | undefined,
): string {
  if (engine === "groq") {
    return model ? `Groq · ${prettyModel(model)}` : "Groq";
  }
  return model ? prettyModel(model) : "Llama · 3.2 · 1B";
}

function prettyModel(raw: string): string {
  const segments = raw.split(/[:/\-_]/).filter(Boolean);
  const tokens = segments.flatMap((s) =>
    s.length >= 4 ? splitAtLetterDigitBoundary(s) : [s],
  );
  if (tokens.length === 0) return raw;
  return tokens
    .map((p) => (p.length <= 3 ? p.toUpperCase() : capitalize(p)))
    .join(" · ");
}

function splitAtLetterDigitBoundary(s: string): string[] {
  return s.split(/(?<=[a-zA-Z])(?=\d)|(?<=\d)(?=[a-zA-Z])/);
}

function capitalize(s: string): string {
  if (!s) return s;
  return s.charAt(0).toUpperCase() + s.slice(1);
}
