import { useBrandingStore } from "../state/branding";
import logoDark from "../assets/logodark.png";
import logoLight from "../assets/logolight.png";

export function BrandLogo(props: { className?: string; height?: number; showText?: boolean }) {
  const branding = useBrandingStore((s) => s.branding);
  const height = props.height ?? 36;

  const src = branding?.logo_url ? branding.logo_url : branding?.theme === "light" ? logoLight : logoDark;
  const alt = branding?.app_name ?? "Flux";

  return (
    <div className={props.className ?? ""} style={{ display: "flex", alignItems: "center", gap: 10 }}>
      <img src={src} alt={alt} style={{ height, width: "auto" }} />
      {props.showText ? <span className="font-semibold tracking-tight">{branding?.app_name ?? "Flux"}</span> : null}
    </div>
  );
}

