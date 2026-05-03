import type { Transcription } from "@/types/transcription";

const refusalMarkers = [
  "nao posso criar",
  "não posso criar",
  "nao consigo criar",
  "não consigo criar",
  "posso ajudar com outra coisa",
  "estou aqui para ajudar",
  "como posso ajuda",
  "linguagem explic",
  "ofensiva ou inapropriada",
  "i cannot help",
  "i can't help",
  "i can’t help",
  "cannot assist",
  "can't assist",
  "can’t assist",
  "i am unable to",
  "i'm unable to",
  "i’m unable to",
  "as an ai",
];

export function looksLikeModelRefusal(text: string) {
  const lower = text.toLowerCase();
  return refusalMarkers.some((marker) => lower.includes(marker));
}

export function isUsableTranscription(row: Transcription) {
  if (row.mode !== "command") return true;
  return !looksLikeModelRefusal(row.finalText || row.rawText);
}
