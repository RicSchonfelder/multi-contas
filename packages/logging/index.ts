import winston from 'winston';

const SENSITIVE_KEYS = [
  'password',
  'token',
  'cookie',
  'secret',
  'authorization',
  'proxy_pass',
  'proxypass',
];

/**
 * Recursively redacts sensitive keys and values from objects, arrays, and strings.
 */
export function redact(obj: any): any {
  if (obj === null || obj === undefined) {
    return obj;
  }

  if (Array.isArray(obj)) {
    return obj.map((item) => redact(item));
  }

  if (typeof obj === 'object') {
    // If it's an error object, serialize it first
    if (obj instanceof Error) {
      const errorObj: any = {
        name: obj.name,
        message: redact(obj.message),
        stack: redact(obj.stack),
      };
      // Copy any additional custom properties on the error object
      for (const key of Object.keys(obj)) {
        if (key !== 'name' && key !== 'message' && key !== 'stack') {
          errorObj[key] = redact((obj as any)[key]);
        }
      }
      return errorObj;
    }

    const copy = Object.create(Object.getPrototypeOf(obj));
    
    // Copy Symbol properties (important for winston metadata and formatting symbols)
    const symbols = Object.getOwnPropertySymbols(obj);
    for (const sym of symbols) {
      copy[sym] = (obj as any)[sym];
    }

    const keys = Object.keys(obj);
    for (const key of keys) {
      const lowerKey = key.toLowerCase();
      const isSensitive = SENSITIVE_KEYS.some((sensitiveKey) => {
        return lowerKey === sensitiveKey || lowerKey.includes(sensitiveKey);
      });

      if (isSensitive) {
        const isExact = SENSITIVE_KEYS.some(
          (sk) => lowerKey === sk || lowerKey.replace('_', '') === sk.replace('_', '')
        );
        if (isExact) {
          copy[key] = '[REDACTED]';
        } else if (typeof obj[key] === 'object' && obj[key] !== null) {
          copy[key] = redact(obj[key]);
        } else {
          copy[key] = '[REDACTED]';
        }
      } else {
        copy[key] = redact(obj[key]);
      }
    }
    return copy;
  }

  if (typeof obj === 'string') {
    let redactedString = obj;
    for (const key of SENSITIVE_KEYS) {
      // Redact JSON string values like "password": "value"
      const jsonRegex = new RegExp(`("${key}"\\s*:\\s*")[^"]+(")`, 'gi');
      redactedString = redactedString.replace(jsonRegex, `$1[REDACTED]$2`);

      // Redact query parameter patterns like password=value
      const queryRegex = new RegExp(`(${key}=)[^&\\s]+`, 'gi');
      redactedString = redactedString.replace(queryRegex, `$1[REDACTED]`);
    }
    return redactedString;
  }

  return obj;
}

/**
 * Winston format that applies the redaction logic to log metadata.
 */
export const redactFormat = winston.format((info) => {
  return redact(info);
});

export interface LoggerConfig {
  logFilePath?: string;
  level?: string;
}

/**
 * Creates a configured winston logger.
 */
export function createLogger(config: LoggerConfig = {}) {
  const transports: winston.transport[] = [
    new winston.transports.Console({
      format: winston.format.combine(
        redactFormat(),
        winston.format.colorize(),
        winston.format.timestamp(),
        winston.format.printf(({ timestamp, level, message, ...meta }) => {
          // Remove internal symbols and level/message properties from output metadata
          const cleanMeta = { ...meta };
          delete cleanMeta.timestamp;
          
          const metaStr = Object.keys(cleanMeta).length
            ? ` ${JSON.stringify(cleanMeta)}`
            : '';
          return `[${timestamp}] ${level}: ${message}${metaStr}`;
        })
      ),
    }),
  ];

  if (config.logFilePath) {
    transports.push(
      new winston.transports.File({
        filename: config.logFilePath,
        format: winston.format.combine(
          redactFormat(),
          winston.format.timestamp(),
          winston.format.json()
        ),
      })
    );
  }

  return winston.createLogger({
    level: config.level || process.env.LOG_LEVEL || 'info',
    transports,
  });
}

// Default logger
export const logger = createLogger({
  logFilePath: process.env.LOG_FILE_PATH,
});
