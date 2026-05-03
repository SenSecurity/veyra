import { describe, expect, it } from "vitest";
import { INITIAL_VOICE_ACTIVITY, nextVoiceActivity } from "./voice-activity";

function feed(levels: number[], deltaMs = 50) {
  return levels.reduce(
    (state, level) => nextVoiceActivity(state, level, deltaMs),
    INITIAL_VOICE_ACTIVITY,
  );
}

describe("nextVoiceActivity", () => {
  it("does not mark constant elevated mic noise as speech", () => {
    const state = feed(Array.from({ length: 20 }, () => 0.08));

    expect(state.energy).toBe(0);
    expect(state.noiseFloor).toBeGreaterThan(0.07);
  });

  it("does not mark fluctuating background noise as speech", () => {
    const noise = Array.from({ length: 40 }, (_, index) =>
      0.078 + ((index * 7) % 9) / 1000,
    );

    const state = feed(noise);

    expect(state.energy).toBe(0);
    expect(state.noiseFloor).toBeGreaterThan(0.075);
  });

  it("marks speech spikes above the learned noise floor as active", () => {
    const calibrated = feed(Array.from({ length: 10 }, () => 0.035));
    const speaking = feed([0.16, 0.18, 0.17], 50);
    const fromCalibrated = [0.16, 0.18, 0.17].reduce(
      (state, level) => nextVoiceActivity(state, level, 50),
      calibrated,
    );

    expect(speaking.energy).toBe(0);
    expect(fromCalibrated.energy).toBeGreaterThan(0.2);
  });

  it("marks speech spikes after a noisy calibration floor", () => {
    const calibrated = feed(Array.from({ length: 12 }, () => 0.085));
    const active = [0.19, 0.21, 0.2].reduce(
      (state, level) => nextVoiceActivity(state, level, 50),
      calibrated,
    );

    expect(active.energy).toBeGreaterThan(0.25);
  });

  it("settles back to inactive after speech returns to floor", () => {
    const calibrated = feed(Array.from({ length: 10 }, () => 0.03));
    const active = [0.16, 0.17, 0.15].reduce(
      (state, level) => nextVoiceActivity(state, level, 50),
      calibrated,
    );
    const settled = Array.from({ length: 16 }, () => 0.03).reduce(
      (state, level) => nextVoiceActivity(state, level, 50),
      active,
    );

    expect(active.energy).toBeGreaterThan(0);
    expect(settled.energy).toBe(0);
  });
});
