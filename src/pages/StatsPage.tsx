import { useState } from "react";

import { PageHeader } from "@/components/PageHeader";

import { PeriodToggle, StatsTab, type Period } from "../components/StatsTab";

export function StatsPage() {
  const [period, setPeriod] = useState<Period>("week");
  return (
    <div className="p-6 flex flex-col gap-6">
      <PageHeader
        title="Stats"
        trailing={<PeriodToggle value={period} onChange={setPeriod} />}
      />
      <StatsTab period={period} />
    </div>
  );
}
