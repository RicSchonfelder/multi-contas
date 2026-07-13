import { describe, it, expect } from 'vitest';
import { ProfileSchema, ProxyConfigSchema } from './index';

describe('ProxyConfigSchema', () => {
  it('should validate a valid direct proxy config', () => {
    const result = ProxyConfigSchema.safeParse({
      type: 'direct',
    });
    expect(result.success).toBe(true);
  });

  it('should validate a valid http proxy config', () => {
    const result = ProxyConfigSchema.safeParse({
      type: 'http',
      host: '12.34.56.78',
      port: 8080,
      username: 'user',
      password: 'password',
    });
    expect(result.success).toBe(true);
  });

  it('should fail http proxy config if host or port is missing', () => {
    const result = ProxyConfigSchema.safeParse({
      type: 'http',
      host: '12.34.56.78',
    });
    expect(result.success).toBe(false);
  });

  it('should validate a valid pac proxy config', () => {
    const result = ProxyConfigSchema.safeParse({
      type: 'pac',
      pacUrl: 'https://example.com/proxy.pac',
    });
    expect(result.success).toBe(true);
  });

  it('should fail pac proxy config if pacUrl is missing', () => {
    const result = ProxyConfigSchema.safeParse({
      type: 'pac',
    });
    expect(result.success).toBe(false);
  });
});

describe('ProfileSchema', () => {
  const validProfile = {
    id: 'a3d6f461-8b06-4444-9f05-df8538740b2f',
    organizationId: 'e29788d4-5ea7-4228-a3be-5120a17387cc',
    ownerUserId: '4337b51b-1934-4b5b-9d41-38a4dfbe635d',
    name: 'Profile 1',
    description: 'Test profile',
    status: 'available',
    color: '#FF5733',
    browserKind: 'chromium',
    syncStatus: 'never_synced',
    version: 1,
    createdAt: '2026-07-02T12:00:00Z',
    updatedAt: '2026-07-02T12:00:00Z',
  };

  it('should validate a valid profile', () => {
    const result = ProfileSchema.safeParse(validProfile);
    expect(result.success).toBe(true);
  });

  it('should fail if profile status is invalid', () => {
    const result = ProfileSchema.safeParse({
      ...validProfile,
      status: 'invalid_status',
    });
    expect(result.success).toBe(false);
  });

  it('should fail if color is not a hex code', () => {
    const result = ProfileSchema.safeParse({
      ...validProfile,
      color: 'red',
    });
    expect(result.success).toBe(false);
  });

  it('should fail if name contains invalid characters', () => {
    const result = ProfileSchema.safeParse({
      ...validProfile,
      name: 'Profile #1!',
    });
    expect(result.success).toBe(false);
  });
});
