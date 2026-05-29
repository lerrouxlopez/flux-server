import { useBrandingStore } from "../state/branding";
import defaultLogo from "../assets/fluxNexus.png";

function FluxMark(props: { height: number }) {
  const h = props.height;
  return (
    <svg
      aria-hidden="true"
      height={h}
      viewBox="0 0 48 48"
      width={h}
      style={{ display: "block" }}
    >
      <defs>
        <linearGradient id="fluxAccent" x1="0" x2="1" y1="0" y2="1">
          <stop offset="0" stopColor="var(--flux-color-accent, #4f46e5)" stopOpacity="1" />
          <stop offset="1" stopColor="var(--flux-color-accent, #4f46e5)" stopOpacity="0.6" />
        </linearGradient>
      </defs>
      <rect
        x="2"
        y="2"
        width="44"
        height="44"
        rx="14"
        fill="color-mix(in srgb, var(--flux-surface-1, #0b1220) 88%, #ffffff 12%)"
        stroke="var(--flux-color-border, #1f2937)"
      />
      <path
        d="M28.5 10.5 16.5 27.1h8.2l-3.2 10.4 12-16.6h-8.2l3.2-10.4Z"
        fill="url(#fluxAccent)"
      />
      <circle cx="36.2" cy="12.2" r="2.2" fill="var(--flux-color-accent, #4f46e5)" opacity="0.9" />
    </svg>
  );
}

export function BrandLogo(props: {
  className?: string;
  height?: number;
  width?: number;
  showText?: boolean;
  square?: boolean;
}) {
  const branding = useBrandingStore((s) => s.branding);
  const height = props.height ?? 36;
  const width = props.width;
  const square = props.square ?? false;

  const alt = branding?.app_name ?? "Flux";

  const imgStyle = square
    ? { height, width: height, objectFit: "contain" as const }
    : { height, width: width ?? "auto", objectFit: "contain" as const };

  return (
    <div className={props.className ?? ""} style={{ display: "flex", alignItems: "center", gap: 10 }}>
      {branding?.logo_url ? (
        <img src={branding.logo_url} alt={alt} style={imgStyle} />
      ) : (
        <img src={defaultLogo} alt={alt} style={imgStyle} />
      )}
      {props.showText ? <span className="font-semibold tracking-tight">{branding?.app_name ?? "Flux"}</span> : null}
    </div>
  );
}

