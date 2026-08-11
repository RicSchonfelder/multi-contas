export type ProfileStatus =
  | 'available'
  | 'in_use'
  | 'syncing'
  | 'locked'
  | 'error'
  | 'archived'
  | 'deleted'
  | 'pending_update';

export type ProfileSyncStatus =
  | 'never_synced'
  | 'in_sync'
  | 'pending'
  | 'conflict'
  | 'failed';

export type SyncStatus = ProfileSyncStatus;

export interface LeaseInfo {
  holderId: string;
  deviceId: string;
  fencingToken: number;
  acquiredAt: string;
  expiresAt: string;
}

export interface SyncManifest {
  version: number;
  checksum: string;
  parentVersion: number | null;
  fileCount: number;
  totalSize: number;
  status: ProfileSyncStatus;
}

export type ProxyType = 'direct' | 'http' | 'https' | 'socks5' | 'pac';

export interface ProxyConfig {
  id?: string;
  name?: string;
  type: ProxyType;
  host?: string;
  port?: number;
  username?: string;
  password?: string;
  pacUrl?: string;
  declaredCountry?: string;
  lastCheckedAt?: string;
  lastLatencyMs?: number;
  status?: 'unverified' | 'ok' | 'failed' | 'expired' | 'disabled';
}

export interface Profile {
  id: string; // UUID v4
  organizationId: string; // UUID v4
  ownerUserId: string; // UUID v4
  name: string;
  description?: string;
  status: ProfileStatus;
  color?: string; // hex format e.g. #FF5733
  avatarUrl?: string;
  browserKind: 'chromium';
  browserVersion?: string;
  declaredOs?: 'windows' | 'macos' | 'linux';
  policies?: Record<string, any>;
  notes?: string;
  customFields?: Record<string, any>;
  checksum?: string;
  syncStatus: ProfileSyncStatus;
  lastOpenedAt?: string;
  lastOpenedBy?: string;
  lastOpenedDeviceId?: string;
  retentionPolicy?: Record<string, any>;
  archivedAt?: string;
  deletedAt?: string;
  version: number;
  createdAt: string;
  updatedAt: string;

  // Local/Desktop properties
  localDir?: string;
  diskUsageBytes?: number;
  lastSyncedAt?: string;
  proxyConfig?: ProxyConfig;
  tags?: string[];
  folderId?: string;
  proxyId?: string;
}

// ===== Smart Orchestrator types =====

export type ReasonCode =
  | 'limit_exceeded'
  | 'resource_exhausted'
  | 'circuit_breaker_open'
  | 'profile_not_found'
  | 'profile_already_open'
  | 'auth_failed'
  | 'invalid_request'
  | 'profile_cooldown';

export type JobType = 'navigate' | 'comment' | 'close' | 'custom_script';

export type JobState = 'pending' | 'running' | 'done' | 'failed';

export interface Job {
  id: string;
  profileId: string;
  type: JobType;
  payload: Record<string, unknown>;
  priority: number;
  state: JobState;
  createdAt: string;
  startedAt?: string;
  error?: string;
}

export interface OpenBatchRequest {
  profileIds: string[];
  navigateTo?: string;
  priority?: number;
}

export interface OpenBatchEntry {
  id: string;
  cdpPort?: number;
  wsUrl?: string;
}

export interface QueuedEntry {
  id: string;
  position: number;
  etaMs: number;
}

export interface RejectedEntry {
  id: string;
  error: string;
  reason: ReasonCode;
}

export interface OpenBatchResponse {
  opened: OpenBatchEntry[];
  queued: QueuedEntry[];
  rejected: RejectedEntry[];
  resourceWarnings: string[];
}

export interface ResourceSnapshot {
  freeRamMb: number;
  load1m: number;
  load5m: number;
  chromeProcesses: number;
  healthy: boolean;
}

export interface CircuitBreakerStatus {
  profileId: string;
  state: 'open' | 'closed';
  cooldownUntil?: string;
}

export interface OrchestratorStatus {
  activeProfiles: number;
  maxActiveProfiles: number;
  queueDepth: number;
  resources: ResourceSnapshot;
  circuitBreakers: CircuitBreakerStatus[];
}

export interface HermesHealthResponse {
  ok: boolean;
  version: string;
  orchestrator: {
    activeProfiles: number;
    maxProfiles: number;
    queueDepth: number;
  };
  resources: ResourceSnapshot;
}
