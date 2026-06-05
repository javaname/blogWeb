import config from '../vite.config.js';

const expectedTarget = 'http://127.0.0.1:3000';
const actualTarget = config?.server?.proxy?.['/api']?.target;

if (actualTarget !== expectedTarget) {
  console.error(
    `Expected Vite /api proxy target to be ${expectedTarget}, got ${actualTarget || '<missing>'}.`,
  );
  process.exit(1);
}
