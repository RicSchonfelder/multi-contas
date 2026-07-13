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
