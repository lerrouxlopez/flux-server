import { Input } from "../../../components/Input";

export function OrgSearchBar(props: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
}) {
  return (
    <div className="flex items-center gap-2">
      <label className="sr-only" htmlFor="org-search">
        Search organizations
      </label>
      <Input
        id="org-search"
        value={props.value}
        onChange={(e) => props.onChange(e.target.value)}
        placeholder={props.placeholder ?? "Search organizations"}
      />
    </div>
  );
}

