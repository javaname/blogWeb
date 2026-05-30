import { useI18n } from '../contexts/I18nContext';
import { useTheme } from '../contexts/ThemeContext';
import AdminIcon from './AdminIcon';

export default function ThemeSwitcher({ inverse = false }) {
  const { isDark, toggleTheme } = useTheme();
  const { t } = useI18n();
  const label = isDark ? t('theme.switchToLight') : t('theme.switchToDark');

  return (
    <button
      type="button"
      className={`admin-icon-button theme-switcher ${inverse ? 'is-inverse' : ''}`}
      aria-label={label}
      title={label}
      onClick={toggleTheme}
    >
      <AdminIcon name={isDark ? 'sun' : 'moon'} />
    </button>
  );
}
