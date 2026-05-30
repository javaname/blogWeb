import { useI18n } from '../contexts/I18nContext';

export default function LanguageSwitcher({ compact = false, inverse = false }) {
  const { locale, setLocale, languageOptions, t } = useI18n();

  return (
    <label className={`language-switcher ${compact ? 'is-compact' : ''} ${inverse ? 'is-inverse' : ''}`}>
      {!compact && (
        <span className="language-switcher__label">{t('common.language')}</span>
      )}
      <select
        aria-label={t('common.language')}
        value={locale}
        onChange={(event) => setLocale(event.target.value)}
      >
        {languageOptions.map((option) => (
          <option key={option.value} value={option.value}>
            {t(`languages.${option.value}`)}
          </option>
        ))}
      </select>
    </label>
  );
}
