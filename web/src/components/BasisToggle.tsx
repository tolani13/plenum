// The GROSS / NET toggle — the Thesis-2 control, shared by Command and
// Leaderboards. It only ever changes a display choice: every payload is
// dual-basis, so flipping it re-renders in place (no refetch, no flash).

import { Segmented } from "./Segmented";
import type { Basis } from "../lib/params";

const OPTIONS = [
  { value: "gross" as const, label: "Gross", title: "Rank + figures by list price" },
  { value: "net" as const, label: "Net", title: "Rank + figures after discount" },
];

export function BasisToggle({
  value,
  onChange,
}: {
  value: Basis;
  onChange: (b: Basis) => void;
}) {
  return (
    <Segmented
      options={OPTIONS}
      value={value}
      onChange={onChange}
      ariaLabel="Revenue basis"
      testid="basis-toggle"
    />
  );
}
