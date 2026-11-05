type AppError =
  | { type: "validation"; errors: { field: string; message: string }[] }
  | { type: "database" | "not_found" | "transcription"; message: string };
export default AppError;
