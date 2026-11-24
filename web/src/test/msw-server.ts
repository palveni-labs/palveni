import { setupServer } from "msw/node";
import { M0_FEATURES, MockDb, type MockDbOptions } from "@/mocks/db";
import { createHandlers } from "@/mocks/handlers";
import { createProvisionalHandlers } from "@/mocks/provisional/handlers";
import { ProvisionalState } from "@/mocks/provisional/state";
import { PROVISIONAL_FEATURES } from "@/provisional/types";

export const server = setupServer();

let current: { db: MockDb; provisional: ProvisionalState } | null = null;

export const ALL_FEATURES = [
  ...M0_FEATURES,
  ...PROVISIONAL_FEATURES,
  "users",
  "audit",
  "backups",
  "metadata",
  "observability",
  "sync",
  "cast",
  "mqtt",
];

/**
 * Fresh in-memory server for one test with every handler installed. Defaults to what a real M0
 * server reports; pass `features: ALL_FEATURES` to exercise the provisional modules.
 */
export function installMockDb(options: MockDbOptions = {}): MockDb {
  disposeMockDb();
  const db = new MockDb({ scale: 0.02, scanDurationMs: 300, ...options });
  const provisional = new ProvisionalState(db, { simulate: false, timeScale: 0.02 });
  current = { db, provisional };
  server.resetHandlers(...createHandlers(db), ...createProvisionalHandlers(db, provisional));
  return db;
}

export function disposeMockDb() {
  current?.provisional.dispose();
  current?.db.dispose();
  current = null;
}
