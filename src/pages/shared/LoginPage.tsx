import { FormEvent, useState } from 'react';

interface LoginPageProps {
  loading: boolean;
  error: string | null;
  onLogin: (email: string, password: string) => Promise<void>;
}

export function LoginPage({ loading, error, onLogin }: LoginPageProps) {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');

  function submit(event: FormEvent) {
    event.preventDefault();
    void onLogin(email.trim(), password);
  }

  return (
    <main className="login-page">
      <form className="login-card page-card" onSubmit={submit}>
        <div>
          <p className="login-kicker">Chef Admin</p>
          <h1>Вход администратора</h1>
          <p className="page-muted">Авторизация через backend Super Admin API.</p>
        </div>

        <label className="field">
          <span className="field-label">Email</span>
          <input type="email" required value={email} onChange={(event) => setEmail(event.target.value)} />
        </label>

        <label className="field">
          <span className="field-label">Пароль</span>
          <input type="password" required value={password} onChange={(event) => setPassword(event.target.value)} />
        </label>

        {error && <p className="form-error">{error}</p>}
        <button className="btn btn-primary" type="submit" disabled={loading}>
          {loading ? 'Проверяем...' : 'Войти'}
        </button>
      </form>
    </main>
  );
}
