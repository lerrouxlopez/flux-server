export type OrgGalleryTab = "my_orgs" | "discover" | "requests" | "invites";

function TabButton(props: {
  active: boolean;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      aria-pressed={props.active}
      className={`rounded-md border px-3 py-2 text-sm ${
        props.active
          ? "flux-chip-active border-slate-800"
          : "border-slate-800 bg-slate-950/20 text-slate-200 hover:bg-slate-800/60"
      }`}
      onClick={props.onClick}
      type="button"
    >
      {props.label}
    </button>
  );
}

export function OrgGalleryTabs(props: { tab: OrgGalleryTab; onTabChange: (t: OrgGalleryTab) => void }) {
  return (
    <div className="flex flex-wrap gap-2" role="tablist" aria-label="Organization gallery tabs">
      <TabButton active={props.tab === "my_orgs"} label="My Organizations" onClick={() => props.onTabChange("my_orgs")} />
      <TabButton active={props.tab === "discover"} label="Discover" onClick={() => props.onTabChange("discover")} />
      <TabButton active={props.tab === "requests"} label="Requests" onClick={() => props.onTabChange("requests")} />
      <TabButton active={props.tab === "invites"} label="Invites" onClick={() => props.onTabChange("invites")} />
    </div>
  );
}

