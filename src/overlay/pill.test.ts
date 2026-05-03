import { describe, expect, it } from "vitest";
import { calculateWaveBarHeights } from "./pill";

describe("calculateWaveBarHeights", () => {
  it("changes bar heights while recording with speech energy", () => {
    const first = calculateWaveBarHeights({
      state: "recording",
      voiceLevel: 0.2,
      phase: 0,
    });
    const next = calculateWaveBarHeights({
      state: "recording",
      voiceLevel: 0.2,
      phase: 1.2,
    });

    expect(next).not.toEqual(first);
  });

  it("keeps bars settled while recording silence", () => {
    const first = calculateWaveBarHeights({
      state: "recording",
      voiceLevel: 0.01,
      phase: 0,
    });
    const next = calculateWaveBarHeights({
      state: "recording",
      voiceLevel: 0.01,
      phase: 1.2,
    });

    expect(next).toEqual(first);
  });

  it("uses the idle waveform outside recording", () => {
    expect(
      calculateWaveBarHeights({
        state: "idle",
        voiceLevel: 0.5,
        phase: 1.2,
      }),
    ).toEqual([5, 8, 11, 7, 14, 9, 6, 12, 8, 5]);
  });
});
