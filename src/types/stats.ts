// Mirror of stats structs in `src-tauri/src/storage/stats.rs`.
export interface Totals {
  wordCount: number;
  sessionCount: number;
  totalDurationMs: number;
}

export interface StreakInfo {
  current: number;
  longest: number;
}

export interface DailyStats {
  day: string; // ISO YYYY-MM-DD
  wordCount: number;
  sessionCount: number;
  totalDurationMs: number;
  avgWpm: number | null;
}
