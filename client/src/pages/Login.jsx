import { useEffect, useMemo, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import LanguageSwitcher from '../components/LanguageSwitcher';
import ThemeSwitcher from '../components/ThemeSwitcher';
import { useAuth } from '../contexts/AuthContext';
import { useI18n } from '../contexts/I18nContext';
import { registerWithEmail, requestRegistrationCode } from '../utils/adminApi';

export default function Login() {
  const navigate = useNavigate();
  const location = useLocation();
  const { login, isAuthenticated } = useAuth();
  const { t } = useI18n();
  const [mode, setMode] = useState('login');
  const [form, setForm] = useState({ username: '', password: '' });
  const [registerForm, setRegisterForm] = useState({
    email: '',
    code: '',
    password: '',
    confirmPassword: '',
  });
  const [loading, setLoading] = useState(false);
  const [codeLoading, setCodeLoading] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');

  const metrics = useMemo(
    () => [
      { value: '5', label: t('login.metricsCoreSurfaces') },
      { value: '1', label: t('login.metricsApiLayer') },
      { value: '401 / 403', label: t('login.metricsAuthHandling') },
    ],
    [t],
  );

  useEffect(() => {
    if (isAuthenticated) {
      navigate('/dashboard', { replace: true });
    }
  }, [isAuthenticated, navigate]);

  async function handleSubmit(event) {
    event.preventDefault();
    setLoading(true);
    setError('');
    setSuccess('');
    try {
      await login(form.username, form.password);
      const next = location.state?.from?.pathname || '/dashboard';
      navigate(next, { replace: true });
    } catch (err) {
      setError(err?.message || t('login.failed'));
    } finally {
      setLoading(false);
    }
  }

  async function handleSendCode() {
    setCodeLoading(true);
    setError('');
    setSuccess('');
    try {
      await requestRegistrationCode({ email: registerForm.email });
      setSuccess(t('login.codeSent'));
    } catch (err) {
      setError(err?.message || t('login.registerFailed'));
    } finally {
      setCodeLoading(false);
    }
  }

  async function handleRegister(event) {
    event.preventDefault();
    setLoading(true);
    setError('');
    setSuccess('');
    try {
      await registerWithEmail({
        email: registerForm.email,
        code: registerForm.code,
        password: registerForm.password,
        confirm_password: registerForm.confirmPassword,
      });
      setForm((prev) => ({ ...prev, username: registerForm.email }));
      setMode('login');
      setSuccess(t('login.registerSuccess'));
      setRegisterForm((prev) => ({ ...prev, code: '', password: '', confirmPassword: '' }));
    } catch (err) {
      setError(err?.message || t('login.registerFailed'));
    } finally {
      setLoading(false);
    }
  }

  function switchMode(nextMode) {
    setMode(nextMode);
    setError('');
    setSuccess('');
  }

  return (
    <div className="login-page">
      <div className="login-page__locale">
        <LanguageSwitcher inverse />
        <ThemeSwitcher inverse />
      </div>

      <section className="login-page__visual">
        <div className="login-page__copy">
          <span className="login-page__eyebrow">{t('login.eyebrow')}</span>
          <h1>{t('login.title')}</h1>
          <p>{t('login.description')}</p>
          <div className="login-page__metrics">
            {metrics.map((item) => (
              <article key={item.label}>
                <strong>{item.value}</strong>
                <span>{item.label}</span>
              </article>
            ))}
          </div>
        </div>
      </section>

      <section className="login-page__form">
        <div className="login-card">
          <div className="login-card__tabs" role="tablist" aria-label={t('login.authMode')}>
            <button
              type="button"
              className={mode === 'login' ? 'is-active' : ''}
              onClick={() => switchMode('login')}
            >
              {t('login.signInTab')}
            </button>
            <button
              type="button"
              className={mode === 'register' ? 'is-active' : ''}
              onClick={() => switchMode('register')}
            >
              {t('login.registerTab')}
            </button>
          </div>

          <div className="login-card__head">
            <h2>{mode === 'login' ? t('login.signIn') : t('login.registerTitle')}</h2>
            <p>{mode === 'login' ? t('login.help') : t('login.registerHelp')}</p>
          </div>

          {mode === 'login' ? (
            <form className="admin-form" onSubmit={handleSubmit}>
              <label>
                <span>{t('login.username')}</span>
                <input
                  value={form.username}
                  onChange={(event) => setForm((prev) => ({ ...prev, username: event.target.value }))}
                  placeholder={t('login.usernamePlaceholder')}
                  required
                />
              </label>
              <label>
                <span>{t('login.password')}</span>
                <input
                  type="password"
                  value={form.password}
                  onChange={(event) => setForm((prev) => ({ ...prev, password: event.target.value }))}
                  placeholder={t('login.passwordPlaceholder')}
                  required
                />
              </label>

              {success ? <div className="admin-inline-banner is-success">{success}</div> : null}
              {error ? <div className="admin-inline-banner is-danger">{error}</div> : null}

              <button type="submit" className="admin-primary-button admin-primary-button--full" disabled={loading}>
                {loading ? t('login.loadingSubmit') : t('login.submit')}
              </button>
            </form>
          ) : (
            <form className="admin-form" onSubmit={handleRegister}>
              <label>
                <span>{t('login.email')}</span>
                <input
                  type="email"
                  value={registerForm.email}
                  onChange={(event) => setRegisterForm((prev) => ({ ...prev, email: event.target.value }))}
                  placeholder={t('login.emailPlaceholder')}
                  required
                />
              </label>
              <label>
                <span>{t('login.code')}</span>
                <div className="login-code-row">
                  <input
                    inputMode="numeric"
                    value={registerForm.code}
                    onChange={(event) => setRegisterForm((prev) => ({ ...prev, code: event.target.value }))}
                    placeholder={t('login.codePlaceholder')}
                    required
                  />
                  <button
                    type="button"
                    className="admin-secondary-button"
                    disabled={codeLoading || !registerForm.email}
                    onClick={handleSendCode}
                  >
                    {codeLoading ? t('login.sendingCode') : t('login.sendCode')}
                  </button>
                </div>
              </label>
              <label>
                <span>{t('login.password')}</span>
                <input
                  type="password"
                  value={registerForm.password}
                  onChange={(event) => setRegisterForm((prev) => ({ ...prev, password: event.target.value }))}
                  placeholder={t('login.passwordPlaceholder')}
                  required
                />
              </label>
              <label>
                <span>{t('login.confirmPassword')}</span>
                <input
                  type="password"
                  value={registerForm.confirmPassword}
                  onChange={(event) => setRegisterForm((prev) => ({ ...prev, confirmPassword: event.target.value }))}
                  placeholder={t('login.confirmPasswordPlaceholder')}
                  required
                />
              </label>

              {success ? <div className="admin-inline-banner is-success">{success}</div> : null}
              {error ? <div className="admin-inline-banner is-danger">{error}</div> : null}

              <button type="submit" className="admin-primary-button admin-primary-button--full" disabled={loading}>
                {loading ? t('login.creatingAccount') : t('login.createAccount')}
              </button>
            </form>
          )}
        </div>
      </section>
    </div>
  );
}
