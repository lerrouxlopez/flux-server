import { useState } from "react";
import { Button } from "../components/Button";
import { Input } from "../components/Input";
import { useNavigate, Link } from "react-router-dom";
import { useAuthStore } from "../state/auth";
import { apiFetch } from "../api/client";
import { BrandLogo } from "../components/BrandLogo";

type AuthResponse = { access_token: string; refresh_token: string };

export function LoginPage() {
  const nav = useNavigate();
  const setTokens = useAuthStore((s) => s.setTokens);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setErr(null);
    setLoading(true);
    try {
      const res = await apiFetch<AuthResponse>("/auth/login", {
        method: "POST",
        body: JSON.stringify({ email, password }),
      });
      setTokens(res.access_token, res.refresh_token);
      await useAuthStore.getState().loadMe();
      nav("/orgs");
    } catch (e) {
      setErr((e as Error).message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="mx-auto max-w-md rounded-xl border border-slate-800 bg-slate-900/40 p-6">
      <div className="mb-4 flex justify-center">
        <BrandLogo height={40} />
      </div>
      <h1 className="text-xl font-semibold">Login</h1>
      <p className="mt-1 text-sm text-slate-300">
        Don&apos;t have an account?{" "}
        <Link className="text-indigo-400 hover:underline" to="/register">
          Register
        </Link>
      </p>
      <form className="mt-5 space-y-3" onSubmit={onSubmit}>
        <div>
          <label className="mb-1 block text-sm text-slate-300">Email</label>
          <Input value={email} onChange={(e) => setEmail(e.target.value)} autoComplete="email" />
        </div>
        <div>
          <label className="mb-1 block text-sm text-slate-300">Password</label>
          <Input
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            type="password"
            autoComplete="current-password"
          />
        </div>
        {err ? <div className="text-sm text-red-400">{err}</div> : null}
        <Button disabled={loading} className="w-full">
          {loading ? "Signing in..." : "Sign in"}
        </Button>
      </form>
    </div>
  );
}
