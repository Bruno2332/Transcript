import { invoke } from "@tauri-apps/api/core";

const recordingService = {
  start: (processId: number) =>
    invoke<string>("start_recording", { processId }),
  pause: () => invoke<void>("pause_recording"),
  resume: () => invoke<void>("resume_recording"),
  stop: () => invoke<string>("stop_recording"),
};
export default recordingService;
