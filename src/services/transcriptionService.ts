import { invoke } from "@tauri-apps/api/core";
import type SpeechDto from "../dtos/SpeechDto";

const transcriptionService = {
  async transcribe(audioPath: string): Promise<string> {
    const speeches = await invoke<SpeechDto[]>("transcribe_audio", {
      audioPath,
    });
    return speeches
      .map((speech) => `${speech.speaker}: ${speech.fala}`)
      .join("\n\n");
  },
};
export default transcriptionService;
