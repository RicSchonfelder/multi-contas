import { describe, it, expect, vi } from 'vitest';
import { redact, createLogger, redactFormat } from './index';
import winston from 'winston';

describe('Secrets Redaction', () => {
  it('should redact sensitive keys in simple objects', () => {
    const raw = {
      username: 'testuser',
      password: 'supersecretpassword',
      token: '12345-token',
      cookie: 'sessionid=abc',
      secret: 'my-secret',
      authorization: 'Bearer xyz',
      proxy_pass: 'proxy_password_123',
      normal_field: 'hello',
    };

    const expected = {
      username: 'testuser',
      password: '[REDACTED]',
      token: '[REDACTED]',
      cookie: '[REDACTED]',
      secret: '[REDACTED]',
      authorization: '[REDACTED]',
      proxy_pass: '[REDACTED]',
      normal_field: 'hello',
    };

    expect(redact(raw)).toEqual(expected);
  });

  it('should redact sensitive keys case-insensitively and partial matches', () => {
    const raw = {
      PASSWORD: '123',
      accessToken: 'jwt-token',
      user_cookie: 'somecookie',
      my_secret_key: 'topsecret',
      proxy_pass_phrase: 'pass',
    };

    expect(redact(raw).PASSWORD).toBe('[REDACTED]');
    expect(redact(raw).accessToken).toBe('[REDACTED]');
    expect(redact(raw).user_cookie).toBe('[REDACTED]');
    expect(redact(raw).my_secret_key).toBe('[REDACTED]');
    expect(redact(raw).proxy_pass_phrase).toBe('[REDACTED]');
  });

  it('should redact sensitive values inside nested objects and arrays', () => {
    const raw = {
      profiles: [
        {
          name: 'Profile 1',
          proxyConfig: {
            host: '1.1.1.1',
            password: 'secretproxyport',
          },
        },
      ],
      meta: {
        cookiesList: ['auth=123', 'session=456'],
        nestedSecret: {
          token: 'nested-token-val',
        },
      },
    };

    const redacted = redact(raw);
    expect(redacted.profiles[0].proxyConfig.password).toBe('[REDACTED]');
    expect(redacted.meta.nestedSecret.token).toBe('[REDACTED]');
  });

  it('should redact secrets in raw strings (JSON and query format)', () => {
    const jsonStr = '{"username":"admin","password":"password123"}';
    const queryStr = 'user=admin&password=password123&token=mytoken';

    expect(redact(jsonStr)).toBe('{"username":"admin","password":"[REDACTED]"}');
    expect(redact(queryStr)).toBe('user=admin&password=[REDACTED]&token=[REDACTED]');
  });

  it('should not mutate original object', () => {
    const raw = {
      name: 'John',
      password: '123',
    };
    const redacted = redact(raw);
    expect(redacted.password).toBe('[REDACTED]');
    expect(raw.password).toBe('123');
  });
});

describe('Winston Logger Redaction Integration', () => {
  it('should redact sensitive data before printing to console/transports', () => {
    const logEntries: any[] = [];
    const memoryTransport = new winston.transports.Console({
      log(info, callback) {
        logEntries.push(info);
        if (callback) callback();
      }
    });

    const tempLogger = winston.createLogger({
      format: redactFormat(),
      transports: [memoryTransport],
    });

    tempLogger.info('Testing redact format', {
      username: 'user',
      password: 'password123',
    });

    expect(logEntries.length).toBe(1);
    expect(logEntries[0].password).toBe('[REDACTED]');
    expect(logEntries[0].username).toBe('user');
  });
});
