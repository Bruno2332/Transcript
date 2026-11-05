import type AppError from "../dtos/AppError";

export default function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object") {
    const appError = error as AppError;
    if (appError.type === "validation" && Array.isArray(appError.errors))
      return appError.errors.map((item) => item.message).join(" ");
    if ("message" in appError && typeof appError.message === "string")
      return appError.message;
  }
  return "Não foi possível concluir a operação. Tente novamente.";
}
