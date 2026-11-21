import type { Job } from "@/api/types";

export type TaskState = Job["state"];
export type Task = Omit<Job, "subject"> & {
  title: string;
  subject: { title: string } | null;
  scope: string | null;
  libraryId: string | null;
};
