import { describe, expect, it } from "vitest";
import { highlightParts, tokenizeQuery } from "./fts-highlight";

describe("fts-highlight", () => {
  it("tokenizes unique query terms", () => {
    expect(tokenizeQuery("hello hello world")).toEqual(["hello", "world"]);
  });

  it("marks matching text parts", () => {
    expect(highlightParts("hello world", "world")).toEqual([
      { text: "hello ", match: false },
      { text: "world", match: true },
    ]);
  });
});

