import { z } from 'zod';
import type { ProfileStatus, ProfileSyncStatus, ProxyType, ProxyConfig, Profile } from '@browser-workspace/contracts';

export const ProxyTypeSchema = z.enum(['direct', 'http', 'https', 'socks5', 'pac']) satisfies z.ZodType<ProxyType>;

export const ProxyConfigSchema = z.object({
  id: z.string().uuid().optional(),
  name: z.string().min(1).max(100).optional(),
  type: ProxyTypeSchema,
  host: z.string().min(1).max(255).optional(),
  port: z.number().int().min(1).max(65535).optional(),
  username: z.string().max(255).optional(),
  password: z.string().max(255).optional(),
  pacUrl: z.string().url().optional(),
  declaredCountry: z.string().length(2).optional(),
  lastCheckedAt: z.string().datetime().optional(),
  lastLatencyMs: z.number().int().nonnegative().optional(),
  status: z.enum(['unverified', 'ok', 'failed', 'expired', 'disabled']).optional(),
}).refine((data) => {
  if (data.type !== 'direct' && data.type !== 'pac') {
    return !!data.host && !!data.port;
  }
  return true;
}, {
  message: 'Host and port are required for proxy types other than direct and pac',
  path: ['host', 'port']
}).refine((data) => {
  if (data.type === 'pac') {
    return !!data.pacUrl;
  }
  return true;
}, {
  message: 'pacUrl is required when proxy type is pac',
  path: ['pacUrl']
}) satisfies z.ZodType<ProxyConfig>;

export const ProfileStatusSchema = z.enum([
  'available',
  'in_use',
  'syncing',
  'locked',
  'error',
  'archived',
  'deleted',
  'pending_update',
]) satisfies z.ZodType<ProfileStatus>;

export const ProfileSyncStatusSchema = z.enum([
  'never_synced',
  'in_sync',
  'pending',
  'conflict',
  'failed',
]) satisfies z.ZodType<ProfileSyncStatus>;

export const ProfileSchema = z.object({
  id: z.string().uuid(),
  organizationId: z.string().uuid(),
  ownerUserId: z.string().uuid(),
  name: z.string().min(1).max(100).regex(/^[a-zA-Z0-9\s-_]+$/, {
    message: 'Name can only contain alphanumeric characters, spaces, dashes, and underscores',
  }),
  description: z.string().max(500).optional(),
  status: ProfileStatusSchema,
  color: z.string().regex(/^#[0-9a-fA-F]{6}$/, {
    message: 'Color must be a valid hex code (e.g. #FF5733)',
  }).optional(),
  avatarUrl: z.string().url().optional().or(z.literal('')),
  browserKind: z.literal('chromium'),
  browserVersion: z.string().optional(),
  declaredOs: z.enum(['windows', 'macos', 'linux']).optional(),
  policies: z.record(z.any()).optional(),
  notes: z.string().max(2000).optional(),
  customFields: z.record(z.any()).optional(),
  checksum: z.string().optional(),
  syncStatus: ProfileSyncStatusSchema,
  lastOpenedAt: z.string().datetime().optional(),
  lastOpenedBy: z.string().uuid().optional(),
  lastOpenedDeviceId: z.string().uuid().optional(),
  retentionPolicy: z.record(z.any()).optional(),
  archivedAt: z.string().datetime().optional(),
  deletedAt: z.string().datetime().optional(),
  version: z.number().int().nonnegative(),
  createdAt: z.string().datetime(),
  updatedAt: z.string().datetime(),

  // Local/Desktop properties
  localDir: z.string().optional(),
  diskUsageBytes: z.number().int().nonnegative().optional(),
  lastSyncedAt: z.string().datetime().optional(),
  proxyConfig: ProxyConfigSchema.optional(),
}) satisfies z.ZodType<Profile>;

// Payload schemas for operations
export const CreateProfileSchema = ProfileSchema.omit({
  createdAt: true,
  updatedAt: true,
  version: true,
  status: true,
  syncStatus: true,
}).extend({
  name: z.string().min(1).max(100).regex(/^[a-zA-Z0-9\s-_]+$/, {
    message: 'Name can only contain alphanumeric characters, spaces, dashes, and underscores',
  }),
  browserKind: z.literal('chromium').default('chromium'),
});

export const UpdateProfileSchema = ProfileSchema.partial().omit({
  id: true,
  organizationId: true,
  createdAt: true,
});
