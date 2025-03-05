import winston from 'winston';
import path from 'path';

const customFormat = winston.format.printf(({ timestamp, level, message, ...metadata }) => {
  const ts = timestamp as string;
  const paddedLevel = level.toUpperCase().padEnd(5);
  const meta = Object.keys(metadata).length ? ` | ${JSON.stringify(metadata)}` : '';
  return `${ts} | ${paddedLevel} | [${message}]${meta}`;
});

const createLogger = (testId: string) => {
  return winston.createLogger({
    level: 'info',
    format: winston.format.combine(winston.format.timestamp(), customFormat),
    transports: [
      new winston.transports.File({
        filename: path.join('logs', `loadtest-${testId}.log`),
        level: 'info',
      }),
    ],
  });
};

export default createLogger;
