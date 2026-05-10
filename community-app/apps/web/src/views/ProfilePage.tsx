import { useMemo, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { apiFetch } from "../api/client";
import { useAuthStore } from "../state/auth";
import { Input } from "../components/Input";
import { Button } from "../components/Button";
import { Avatar } from "../components/Avatar";

export function ProfilePage() {
  const user = useAuthStore((s) => s.user);
  const loadMe = useAuthStore((s) => s.loadMe);

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
          <Avatar
            name={displayName.trim() || user.display_name}
            size={56}
            src={avatarDataUrl ?? user.avatar_url ?? null}
          />
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
          {avatarDataUrl ? (
            <div className="mt-2 flex gap-2">
              <Button className="bg-slate-800 hover:bg-slate-700" onClick={() => setAvatarDataUrl(null)} type="button">
                Clear
              </Button>
            </div>
          ) : null}
        </div>

        <div className="mt-4 flex items-center gap-3">
          <Button
            className="bg-indigo-600 hover:bg-indigo-500"
            disabled={!canSave || saveProfile.isPending}
            onClick={() => saveProfile.mutate()}
            type="button"
          >
            {saveProfile.isPending ? "Saving..." : "Save"}
          </Button>
          {err ? <div className="text-sm text-red-400">{err}</div> : null}
          {!err && saveProfile.isSuccess ? <div className="text-sm text-emerald-400">Saved.</div> : null}
        </div>
      </div>
    </div>
  );
}
