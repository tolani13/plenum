import type { ReactNode } from 'react'
import { AlertTriangle, Brain, Gauge, PiggyBank, Wind, Wrench, Zap } from 'lucide-react'
import { fmtDate, fmtTime, SERVICE_LIMIT, useSim } from "./simulator";

export function Card({ title, children, right }: { title: string; children: ReactNode; right?: ReactNode }) {
  return (
    <section className="rounded-lg border border-seam bg-surface flex min-w-0 flex-col p-4">
      <header className="mb-3 flex items-center justify-between gap-2">
        <h2 className="nameplate-strong text-2xs text-text-dim">{title}</h2>
        {right}
      </header>
      {children}
    </section>
  )
}

export function KpiRow() {
  const { live, prediction } = useSim()
  const dpWarn = live.dp > 4.2
  const life = Math.max(0, Math.min(100, Math.round((1 - (live.dp - 2) / (SERVICE_LIMIT - 2)) * 100)))
  const items = [
    {
      icon: <Gauge className="h-4 w-4" />,
      label: 'Differential pressure',
      value: live.dp.toFixed(2),
      unit: 'in. w.g.',
      tone: dpWarn ? 'text-warn' : 'text-text',
    },
    {
      icon: <Wind className="h-4 w-4" />,
      label: 'Airflow',
      value: live.cfm.toLocaleString(),
      unit: 'CFM',
      tone: 'text-text',
    },
    {
      icon: <Zap className="h-4 w-4" />,
      label: 'Fan power',
      value: live.kw.toFixed(1),
      unit: 'kW',
      tone: 'text-text',
    },
    {
      icon: <Wrench className="h-4 w-4" />,
      label: 'Predicted service',
      value: fmtDate(prediction.serviceDate),
      unit: `${Math.round(prediction.daysOut)} days`,
      tone: 'text-ok',
      sub: `filter life ~${life}%`,
    },
  ]
  return (
    <div className="grid grid-cols-2 gap-3 xl:grid-cols-4">
      {items.map((k) => (
        <div key={k.label} className="rounded-lg border border-seam bg-surface p-4">
          <div className="mb-2 flex items-center gap-2 text-text-dim">
            {k.icon}
            <span className="nameplate-strong text-2xs text-text-dim">{k.label}</span>
          </div>
          <div className="flex flex-wrap items-baseline gap-x-2">
            <span className={`nameplate-strong text-3xl font-semibold tabular ${k.tone}`}>{k.value}</span>
            <span className="text-xs text-text-faint">{k.unit}</span>
          </div>
          {k.sub && <div className="mt-1 text-xs text-text-faint">{k.sub}</div>}
        </div>
      ))}
    </div>
  )
}

export function FilterBank() {
  const { cartridges } = useSim()
  const color = (h: number, flagged: boolean) =>
    flagged ? 'border-alarm/60 text-alarm' : h > 55 ? 'border-ok/40 text-ok' : 'border-warn/50 text-warn'
  return (
    <div className="grid grid-cols-4 gap-2">
      {cartridges.map((c) => (
        <div
          key={c.id}
          className={`rounded-md border bg-surface-2 px-2 py-2.5 text-center ${color(c.health, c.flagged)}`}
        >
          <div className="font-mono text-[11px] text-text-dim">{c.id}</div>
          <div className="nameplate-strong text-lg font-semibold tabular">{c.health}%</div>
          {c.flagged && <div className="text-[10px] leading-tight">leak sig.</div>}
        </div>
      ))}
    </div>
  )
}

export function Insights() {
  const { prediction } = useSim()
  const items = [
    {
      icon: <Brain className="h-4 w-4 text-ok" />,
      title: `Service window ${fmtDate(prediction.serviceDate)} · ${(prediction.confidence * 100).toFixed(0)}% confidence`,
      body: `Baseline drift ${prediction.slopePerDay.toFixed(3)}"/day. Ordering cartridges now lands them a week ahead of the ${SERVICE_LIMIT.toFixed(1)}" limit — planned change-out, not a Friday-night emergency.`,
    },
    {
      icon: <AlertTriangle className="h-4 w-4 text-alarm" />,
      title: 'C-06 emission signature anomaly',
      body: 'Post-pulse recovery on C-06 diverges from the bank median — consistent with a seat leak, not media life. Inspect the seat before condemning the cartridge.',
    },
    {
      icon: <PiggyBank className="h-4 w-4 text-warn" />,
      title: 'Compressed air: raise clean setpoint 0.2"',
      body: 'Pulse frequency is 18% above what this drift rate needs. Raising the setpoint saves compressed air without moving the service date.',
    },
  ]
  return (
    <ul className="flex flex-col gap-3">
      {items.map((i) => (
        <li key={i.title} className="rounded-md border border-seam bg-surface-2 p-3">
          <div className="mb-1 flex items-center gap-2 text-sm font-medium text-text">
            {i.icon}
            <span className="min-w-0">{i.title}</span>
          </div>
          <p className="text-xs leading-relaxed text-text-dim">{i.body}</p>
        </li>
      ))}
    </ul>
  )
}

export function AlertFeed() {
  const { alerts } = useSim()
  const dot = { pulse: 'bg-data', warn: 'bg-alarm', info: 'bg-text-faint' } as const
  return (
    <ul className="flex flex-col gap-2.5">
      {alerts.map((a) => (
        <li key={a.t + a.text} className="flex items-start gap-2.5 text-xs">
          <span className={`mt-1 h-1.5 w-1.5 shrink-0 rounded-full ${dot[a.kind]}`} />
          <span className="min-w-0 text-text-dim">
            <span className="mr-2 font-mono text-text-faint">{fmtTime(a.t)}</span>
            {a.text}
          </span>
        </li>
      ))}
    </ul>
  )
}
