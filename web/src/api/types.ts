/** Public names used by the workbench for the server's generated wire types. */
import type { components, operations } from "./generated";

type Schemas = components["schemas"];
export type LibraryKind = Schemas["LibraryKind"];
export type ItemKind = Schemas["ItemKind"];
export type Role = Schemas["Role"];
export type User = Schemas["User"];
export type Library = Schemas["Library"];
export type SubsonicPassword = Schemas["SubsonicPassword"];
export type SubsonicPasswordStatus = Schemas["SubsonicPasswordStatus"];
export type MusicPage = Schemas["MusicPage"];
export type Item = Schemas["Item"];
export type JobState = Schemas["JobState"];
export type JobSubject = Schemas["JobSubject"];
export type JobStep = Schemas["JobStep"];
export type Job = Schemas["Job"];
export type PerfTier = Schemas["PerfTier"];
export type EncodeMethod = Schemas["EncodeMethod"];
export type EncoderCapability = Schemas["EncoderCapability"];
export type ToolInfo = Schemas["ToolInfo"];
export type HardwareReport = Schemas["HardwareReport"];
export type PluginRuntimeKind = "none" | "recipe" | "widget" | "wasm" | "sidecar";
export type PluginRiskLevel = "low" | "medium" | "high";

export interface PluginSettingPropertySchema {
  type?: "boolean" | "string" | "integer" | "number" | "array" | "object";
  title?: string;
  description?: string;
  enum?: (string | number | boolean)[];
  default?: unknown;
  minLength?: number;
  maxLength?: number;
  minimum?: number;
  maximum?: number;
}

export interface PluginSettingsSchema {
  type?: string;
  properties?: Record<string, PluginSettingPropertySchema>;
}

export interface Plugin {
  id: string;
  name: string;
  version: string;
  runtime: PluginRuntimeKind;
  enabled: boolean;
  provides: string[];
  previousVersion?: string | null;
  permissions?: string[] | null;
  grantedPermissions?: string[] | null;
  settingsSchema?: PluginSettingsSchema | null;
  settings?: Record<string, unknown> | null;
  source?: string | null;
  riskLevel?: PluginRiskLevel | null;
  description?: Record<string, string> | null;
  managed?: boolean | null;
  installedAt?: string | null;
  updatedAt?: string | null;
}

export interface PluginCatalogEntry {
  id: string;
  name: string;
  version: string;
  previousVersion: string | null;
  runtime: PluginRuntimeKind;
  provides: string[];
  permissions: string[];
  riskLevel: PluginRiskLevel;
  source: string;
  description: Record<string, string>;
  settingsSchema: PluginSettingsSchema;
  defaultSettings: Record<string, unknown>;
  installed: boolean;
  installedVersion: string | null;
}

export interface PluginCatalogResponse {
  plugins: PluginCatalogEntry[];
}

export interface InstallPluginRequest {
  pluginId: string;
  grantedPermissions: string[];
  settings?: Record<string, unknown>;
}

export interface UpdatePluginRequest {
  enabled?: boolean;
  settings?: Record<string, unknown>;
}
export type SystemInfo = Schemas["SystemInfo"];
export type HealthResponse = Schemas["Health"];
export type Credentials = Schemas["SetupRequest"];
export type SetupResponse = Schemas["SetupResponse"];
export type LoginRequest = Schemas["LoginRequest"];
export type LoginResponse = Schemas["LoginResponse"];
export type LibraryList = Schemas["LibraryList"];
export type UpdateLibraryRequest = Schemas["UpdateLibraryRequest"];
export type CreateLibraryRequest = Schemas["CreateLibraryRequest"];
export type ScanResponse = Schemas["ScanAccepted"];
export type ScanReview = Schemas["ScanReview"];
export type ScanReviewList = Schemas["ScanReviewList"];
export type ScanReviewPaths = Schemas["ScanReviewPaths"];
export type ItemPage = Schemas["ItemList"];
export type ItemListParams = operations["listItems"]["parameters"]["query"];
export type JobList = Schemas["JobList"];
export interface PluginList {
  plugins: Plugin[];
}
export type ApiErrorCode = Schemas["ApiErrorCode"];
export type ApiErrorDetails = Schemas["ApiErrorDetails"];
export type ApiErrorBody = Schemas["ApiErrorBody"];
export type ServerEventMap = Schemas["ServerEventMap"];
export type ServerEventName = keyof ServerEventMap;
export type SkinPackageInfo = Schemas["SkinInfo"];
export type InstalledSkin = Schemas["InstalledSkin"];
export type RegistrySkin = Schemas["RegistrySkin"];
export type RuntimeSettings = Schemas["RuntimeSettings"];
export type Capability = Schemas["Capability"];
export type UserPolicy = Schemas["UserPolicy"];
export type UserPolicyPatch = Schemas["UserPolicyPatch"];
export type ManagedUser = Schemas["ManagedUser"];
export type UserList = Schemas["UserList"];
export type CreateUserRequest = Schemas["CreateUserRequest"];
export type UpdateUserRequest = Schemas["UpdateUserRequest"];
export type ChangePasswordRequest = Schemas["ChangePasswordRequest"];
export type UserSession = Schemas["UserSession"];
export type UserSessionList = Schemas["UserSessionList"];
export type AuditLogEntry = Schemas["AuditLogEntry"];
export type AuditLogPage = Schemas["AuditLogPage"];
export type BackupSummaryCounts = Schemas["BackupSummaryCounts"];
export type BackupSummary = Schemas["BackupSummary"];
export type BackupList = Schemas["BackupList"];
export type CreateBackupRequest = Schemas["CreateBackupRequest"];
export type BackupRestoreResult = Schemas["BackupRestoreResult"];
export type ExternalImportSource = Schemas["ExternalImportSource"];
export type ExternalImportSample = Schemas["ExternalImportSample"];
export type ExternalImportRequest = Schemas["ExternalImportRequest"];
export type ExternalImportResult = Schemas["ExternalImportResult"];
export type MetadataProvider = Schemas["MetadataProvider"];
export type MetadataSearchMediaType = Schemas["MetadataSearchMediaType"];
export type MetadataConfigResponse = Schemas["MetadataConfigResponse"];
export type UpdateMetadataConfigRequest = Schemas["UpdateMetadataConfigRequest"];
export type MetadataSearchRequest = Schemas["MetadataSearchRequest"];
export type MetadataSearchCandidate = Schemas["MetadataSearchCandidate"];
export type MetadataSearchResponse = Schemas["MetadataSearchResponse"];
export type MetadataOverride = Schemas["MetadataOverride"];
export type MetadataOverrideResponse = Schemas["MetadataOverrideResponse"];
export type SaveMetadataOverrideRequest = Schemas["SaveMetadataOverrideRequest"];
export type DoctorStatus = Schemas["DoctorStatus"];
export type DoctorCheck = Schemas["DoctorCheck"];
export type SystemDoctorReport = Schemas["SystemDoctorReport"];
export type ServerLogLevel = Schemas["ServerLogLevel"];
export type ServerLogEntry = Schemas["ServerLogEntry"];
export type ServerLogList = Schemas["ServerLogList"];
