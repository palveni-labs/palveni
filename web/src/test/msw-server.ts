import { setupServer } from "msw/node";
import { M0_FEATURES, MockDb, type MockDbOptions } from "@/mocks/db";
import { createHandlers } from "@/mocks/handlers";
import { createSkinHandlers } from "@/mocks/skin-handlers";

export const server = setupServer();
export const ALL_FEATURES = [...M0_FEATURES];
let current: MockDb | null = null;

export function installMockDb(options: MockDbOptions = {}): MockDb {
  disposeMockDb();
  const db = new MockDb({ scale: 0.02, scanDurationMs: 300, ...options });
  current = db;
  server.resetHandlers(...createHandlers(db), ...createSkinHandlers(db));
  return db;
}

export function disposeMockDb() {
  current?.dispose();
  current = null;
}
