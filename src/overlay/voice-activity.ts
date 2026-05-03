export interface VoiceActivityState {
  elapsedMs: number;
  noiseFloor: number | null;
  energy: number;
}

const CALIBRATION_MS = 300;
const NOISE_MARGIN = 0.035;
const ENERGY_SCALE = 0.18;
const ACTIVE_ENERGY = 0.06;

export const INITIAL_VOICE_ACTIVITY: VoiceActivityState = {
  elapsedMs: 0,
  noiseFloor: null,
  energy: 0,
};

export function nextVoiceActivity(
  previous: VoiceActivityState,
  rawLevel: number,
  deltaMs = 50,
): VoiceActivityState {
  const level = Math.max(0, Math.min(1, rawLevel));
  const elapsedMs = previous.elapsedMs + Math.max(0, deltaMs);
  let noiseFloor = previous.noiseFloor ?? level;

  if (elapsedMs <= CALIBRATION_MS) {
    noiseFloor = noiseFloor * 0.55 + level * 0.45;
    return { elapsedMs, noiseFloor, energy: 0 };
  }

  const nearFloor = level <= noiseFloor + NOISE_MARGIN;
  if (nearFloor) {
    noiseFloor =
      level < noiseFloor
        ? noiseFloor * 0.7 + level * 0.3
        : noiseFloor * 0.96 + level * 0.04;
  }

  const excess = Math.max(0, level - (noiseFloor + NOISE_MARGIN));
  const targetEnergy = excess > 0
    ? Math.min(1, Math.pow(excess / ENERGY_SCALE, 0.6))
    : 0;
  const energy =
    targetEnergy > previous.energy
      ? previous.energy * 0.25 + targetEnergy * 0.75
      : previous.energy * 0.82 + targetEnergy * 0.18;

  return {
    elapsedMs,
    noiseFloor,
    energy: energy > ACTIVE_ENERGY ? energy : 0,
  };
}
