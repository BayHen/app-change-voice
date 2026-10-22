export type ToastTone = "info" | "warning" | "success";

export interface ToastMessage {
  id: number;
  title: string;
  message: string;
  tone: ToastTone;
}
