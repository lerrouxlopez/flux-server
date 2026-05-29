import { useMemo, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import { useAuthStore } from "../state/auth";
import { Input } from "../components/Input";
import { Button } from "../components/Button";
import { Avatar } from "../components/Avatar";
import { useExperience } from "../features/experience/useExperience";
import { THEME_PRESETS } from "../branding/presets";

export function ProfilePage() {
  const user = useAuthStore((s) => s.user);
  const loadMe = useAuthStore((s) => s.loadMe);
  const experience = useExperience();

  const [name, setName] = useState(user?.name ?? "");
  const [displayName, setDisplayName] = useState(user?.display_name ?? "");
  const [avatarDataUrl, setAvatarDataUrl] = useState<string | null>(null);
  const [err, setErr] = useState<string | null>(null);

  const canSave = useMemo(() => {
    if (!user) return false;
    const nameTrim = name.trim();
    const displayTrim = displayName.trim();
    const profileChanged = nameTrim !== (user.name ?? "") || displayTrim !== (user.display_name ?? "");
    return profileChanged || !!avatarDataUrl;
  }, [user, name, displayName, avatarDataUrl]);

  const saveProfile = useMutation({
    mutationFn: async () => {
      setErr(null);
      if (!user) throw new Error("Not signed in");
      if (avatarDataUrl) {
        await apiFetch<{ status: string }>("/auth/me/avatar", {
          method: "POST",
          body: JSON.stringify({ data_url: avatarDataUrl }),
        });
      }
      await apiFetch<{ status: string }>("/auth/me", {
        method: "PATCH",
        body: JSON.stringify({ name, display_name: displayName }),
      });
      await loadMe();
    },
    onSuccess: () => setAvatarDataUrl(null),
    onError: (e) => setErr((e as Error).message),
  });

  if (!user) return <div className="text-slate-300">Please sign in.</div>;

  return (
    <div className="mx-auto max-w-2xl">
      <div className="text-xl font-semibold">Profile</div>
      <div className="mt-4 rounded-xl border border-slate-800 bg-slate-900/30 p-4">
        <div className="flex items-center gap-4">
          <Avatar name={displayName.trim() || user.display_name} size={56} src={avatarDataUrl ?? user.avatar_url ?? null} />
          <div className="min-w-0">
            <div className="text-sm font-semibold text-slate-100">{user.email}</div>
            <div className="mt-1 text-xs text-slate-400">Update your name, display name, and avatar.</div>
          </div>
        </div>

        <div className="mt-4 grid gap-3 sm:grid-cols-2">
          <div>
            <label className="mb-1 block text-sm text-slate-300">Name</label>
            <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="Your name" />
          </div>
          <div>
            <label className="mb-1 block text-sm text-slate-300">Display name</label>
            <Input value={displayName} onChange={(e) => setDisplayName(e.target.value)} placeholder="Shown in chat" />
          </div>
        </div>

        <div className="mt-4">
          <label className="mb-1 block text-sm text-slate-300">Avatar</label>
          <input
            accept="image/*"
            className="block w-full text-sm text-slate-300 file:mr-3 file:rounded-md file:border-0 file:bg-slate-800 file:px-3 file:py-2 file:text-sm file:text-slate-200 hover:file:bg-slate-700"
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (!f) return;
              if (f.size > 1_000_000) {
                setErr("Avatar too large (max 1MB).");
                return;
              }
              const fr = new FileReader();
              fr.onload = () => setAvatarDataUrl(typeof fr.result === "string" ? fr.result : null);
              fr.onerror = () => setErr("Failed to read image.");
              fr.readAsDataURL(f);
            }}
            type="file"
          />
          <div className="mt-2">
            <Button
              className="bg-indigo-600 hover:bg-indigo-500"
              disabled={!canSave || saveProfile.isPending}
              onClick={() => saveProfile.mutate()}
              type="button"
            >
              {saveProfile.isPending ? "Saving…" : "Save profile"}
            </Button>
          </div>
          {err ? <div className="mt-2 text-sm text-red-400">{err}</div> : null}
        </div>
      </div>

      <div className="mt-4 rounded-xl border border-slate-800 bg-slate-900/30 p-4">
        <div className="text-sm font-semibold text-slate-100">Experience Mode</div>
        <div className="mt-1 text-xs text-slate-400">
          Mode follows your account across organizations. Current:{" "}
          <span className="font-semibold text-slate-200">{experience.label}</span>
        </div>

        <div className="mt-3 flex flex-wrap gap-2">
          <button
            aria-pressed={experience.rawMode === "work"}
            className={`rounded-md border px-3 py-2 text-sm ${
              experience.rawMode === "work"
                ? "flux-chip-active border-slate-800"
                : "border-slate-800 bg-slate-950/20 text-slate-200 hover:bg-slate-800/60"
            }`}
            disabled={experience.isLoading}
            onClick={() => experience.setMode("work")}
            type="button"
          >
            Work Mode
          </button>
          <button
            aria-pressed={experience.rawMode === "play"}
            className={`rounded-md border px-3 py-2 text-sm ${
              experience.rawMode === "play"
                ? "flux-chip-active border-slate-800"
                : "border-slate-800 bg-slate-950/20 text-slate-200 hover:bg-slate-800/60"
            }`}
            disabled={experience.isLoading}
            onClick={() => experience.setMode("play")}
            type="button"
          >
            Game Mode
          </button>
          <Button
            className="bg-slate-800 hover:bg-slate-700"
            disabled={experience.isLoading}
            onClick={() => experience.clearModePreference()}
            type="button"
          >
            Clear preference
          </Button>
        </div>

        {experience.error ? <div className="mt-3 text-sm text-red-400">{experience.error}</div> : null}
        {experience.isLoading ? <div className="mt-2 text-xs text-slate-400">Updating…</div> : null}
        {!experience.error ? (
          <div className="mt-2 text-xs text-slate-500">
            Resolution source: <span className="font-mono">{experience.source}</span>
          </div>
        ) : null}
      </div>

      <div className="mt-4 rounded-xl border border-slate-800 bg-slate-900/30 p-4">
        <div className="text-sm font-semibold text-slate-100">Personal Theme</div>
        <div className="mt-1 text-xs text-slate-400">
          Applies on global pages (org gallery, login, profile). Each organization controls its own theme when you're active in it.
        </div>

        <div className="mt-3 grid grid-cols-2 gap-2 sm:grid-cols-3">
          {THEME_PRESETS.map((preset) => {
            const active = experience.userThemeId === preset.id;
            return (
              <button
                key={preset.id}
                type="button"
                aria-pressed={active}
                onClick={() => experience.setUserTheme(preset.id)}
                className={`flex items-center gap-2 rounded-lg border px-3 py-2 text-left transition-colors ${
                  active
                    ? "border-indigo-500 bg-indigo-950/40 text-slate-100"
                    : "border-slate-800 bg-slate-950/20 text-slate-300 hover:border-slate-700 hover:bg-slate-900/40"
                }`}
              >
                <span className="flex shrink-0 gap-0.5">
                  <span
                    className="block h-4 w-2 rounded-l-sm"
                    style={{ backgroundColor: preset.vars.appBg }}
                  />
                  <span
                    className="block h-4 w-2 rounded-r-sm"
                    style={{ backgroundColor: preset.vars.brandPrimary }}
                  />
                </span>
                <span className="min-w-0">
                  <span className="block truncate text-xs font-semibold leading-tight">{preset.label}</span>
                  <span className="block text-[10px] leading-tight text-slate-500">{preset.colorScheme}</span>
                </span>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}
