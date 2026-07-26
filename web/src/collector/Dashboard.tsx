import { AlertFeed, Card, FilterBank, Insights, KpiRow } from "./Panels";
import { DpChart, EnergyChart } from "./Charts";
import { useSim } from "./simulator";

export default function Dashboard() {
  const { live } = useSim()
  const warn = live.dp > 4.2

  return (
    <div className="flex w-full min-w-0 flex-col gap-3">
      <KpiRow />

      <div className="grid grid-cols-1 gap-3 lg:grid-cols-3">
        <div className="lg:col-span-2">
          <Card
            title="Differential pressure · 14 days + AI projection"
            right={
              <span className="flex items-center gap-2 text-xs text-text-faint">
                <span className={warn ? "led-warn" : "led"} />
                {warn ? 'approaching clean setpoint' : 'nominal'}
              </span>
            }
          >
            <div className="h-56 min-h-0 sm:h-64">
              <DpChart />
            </div>
          </Card>
        </div>
        <Card title="AI insights">
          <Insights />
        </Card>
      </div>

      <div className="grid grid-cols-1 gap-3 md:grid-cols-2 lg:grid-cols-3">
        <Card title="Fan energy · daily kWh">
          <div className="h-44 min-h-0">
            <EnergyChart />
          </div>
        </Card>
        <Card title="Cartridge bank · media health">
          <FilterBank />
        </Card>
        <Card title="Event feed">
          <AlertFeed />
        </Card>
      </div>
    </div>
  )
}
