import { DEFAULT_LOCALE, messages } from './messages';

export function readMessage(locale, key) {
  return key.split('.').reduce((value, segment) => value?.[segment], messages[locale]);
}

export function interpolate(template, values = {}) {
  if (typeof template !== 'string') {
    return template;
  }
  return template.replace(/\{(\w+)\}/g, (_, name) => {
    const value = values[name];
    return value == null ? '' : String(value);
  });
}

export function hasMessage(locale, key) {
  return readMessage(locale, key) != null || readMessage(DEFAULT_LOCALE, key) != null;
}

export function translateMessage(locale, key, values) {
  const template = readMessage(locale, key) ?? readMessage(DEFAULT_LOCALE, key) ?? key;
  return interpolate(template, values);
}
