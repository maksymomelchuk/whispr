import { useState } from "react";

import { PageShell } from "@/components/PageShell";

import { PeriodToggle, StatsTab, type Period } from "../components/StatsTab";

export function StatsPage() {
  const [period, setPeriod] = useState<Period>("week");
  return (
    <PageShell
      title="Stats"
      trailing={<PeriodToggle value={period} onChange={setPeriod} />}
    >
      <StatsTab period={period} />
    </PageShell>
  );
}
