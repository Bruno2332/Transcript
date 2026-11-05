import { invoke } from "@tauri-apps/api/core";
import type Transcript from "../models/Transcript";
import type TranscriptForm from "../models/TranscriptForm";
import type TranscriptDto from "../dtos/TranscriptDto";
import type PageResponse from "../dtos/PageResponse";

function fromDto(dto: TranscriptDto): Transcript {
  return {
    id: dto.id,
    name: dto.name,
    dateOfService: dto.date_of_service,
    transcription: dto.transcription,
    createdAt: dto.created_at,
    updatedAt: dto.updated_at,
  };
}

const transcriptService = {
  async list(
    page: number,
    size: number,
    name: string,
  ): Promise<PageResponse<Transcript>> {
    const filter = name.trim() || null;
    const [rows, total] = await Promise.all([
      invoke<TranscriptDto[]>("find_all_transcripts_paginated", {
        page,
        size,
        name: filter,
      }),
      invoke<number>("count_all_transcripts", { name: filter }),
    ]);
    return { items: rows.map(fromDto), total, page, size };
  },
  async save(
    values: TranscriptForm,
    original?: Transcript,
  ): Promise<Transcript> {
    const now = new Date().toISOString();
    const dto: TranscriptDto = {
      id: original?.id ?? null,
      name: values.name.trim(),
      date_of_service: values.dateOfService,
      transcription: values.transcription,
      created_at: original?.createdAt ?? now,
      updated_at: now,
    };
    return fromDto(
      await invoke<TranscriptDto>(
        original ? "update_transcript" : "insert_transcript",
        original ? { id: original.id, dto } : { dto },
      ),
    );
  },
  async remove(id: number): Promise<void> {
    await invoke("delete_transcript", { id });
  },
};
export default transcriptService;
