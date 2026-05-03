import { describe, expect, it } from "vitest";
import { looksLikeModelRefusal } from "./email-output-quality";

describe("looksLikeModelRefusal", () => {
  it("detects Portuguese refusal text", () => {
    expect(
      looksLikeModelRefusal(
        "Nao posso criar um e-mail que contenha linguagem explicita. Posso ajudar com outra coisa?",
      ),
    ).toBe(true);
  });

  it("detects common English assist refusals", () => {
    expect(looksLikeModelRefusal("I'm sorry, but I can't assist with that request.")).toBe(true);
    expect(looksLikeModelRefusal("I cannot assist with that request.")).toBe(true);
    expect(looksLikeModelRefusal("I’m sorry, but I can’t help with that.")).toBe(true);
  });

  it("allows normal email drafts", () => {
    expect(
      looksLikeModelRefusal(
        "Ola Sr. Bruno Rodrigues,\n\nEscrevo para confirmar que hoje passarei ai as 17h.\n\nCumprimentos,",
      ),
    ).toBe(false);
  });
});
