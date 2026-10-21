// Mode démo : décodeur ffmpeg simulé (l'installation « télécharge » en 2 s, rien n'est réellement installé).
import type { FfmpegProgress, FfmpegStatus } from "./types.ts";

const state = { installed: false, phase: "idle" as FfmpegProgress["phase"], done: 0 };
const TOTAL = 114_768_076;

export const ffmpegMock = {
  /** Vidéos que « Windows » ne sait pas lire en démo, tant que ffmpeg n'est pas « installé ». */
  needsDecoder: (name: string): boolean => !state.installed && /\.(wmv|mkv)$/i.test(name),
  status: async (): Promise<FfmpegStatus> => ({
    installed: state.installed,
    version: state.installed ? "9.0.2" : null,
    path: state.installed ? "C:\Program Files\PureRGB\tools\ffmpeg.exe" : null,
  }),
  progress: async (): Promise<FfmpegProgress> => ({ phase: state.phase, done: state.done, total: TOTAL, error: null }),
  install: async (): Promise<FfmpegStatus> => {
    state.phase = "download";
    for (let i = 1; i <= 10; i++) {
      await new Promise((r) => setTimeout(r, 200));
      state.done = (TOTAL / 10) * i;
    }
    state.installed = true;
    state.phase = "done";
    return ffmpegMock.status();
  },
};
