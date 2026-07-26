import { useMemo } from 'react'
import {
  Area,
  Bar,
  BarChart,
  CartesianGrid,
  ComposedChart,
  Line,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'
import { fmtDate, SERVICE_LIMIT, useSim } from "./simulator";

const axis = {
  stroke: "var(--color-seam)",
  fontSize: 11,
  tickLine: false as const,
};

function ChartTip({ active, payload, label }: { active?: boolean; payload?: { name: string; value: number; unit?: string }[]; label?: string | number }) {
  if (!active || !payload?.length) return null
  return (
    <div className="rounded border border-seam bg-surface-2 px-2.5 py-1.5 text-xs shadow-lg">
      <div className="text-text-faint">{typeof label === 'number' ? fmtDate(label) : label}</div>
      {payload.map((p) => (
        <div key={p.name} className="text-text">
          {p.name}: <span className="font-medium">{p.value?.toFixed?.(2) ?? p.value}</span>
          {p.unit ?? ''}
        </div>
      ))}
    </div>
  )
}

export function DpChart() {
  const { history, prediction, baseline } = useSim()

  const data = useMemo(() => {
    const hist = history
      .filter((_, i) => i % 2 === 0) // hourly → 2-hourly keeps the sawtooth crisp but light
      .map((s) => ({ t: s.t, dp: s.dp, proj: null as number | null }))
    // projection: baseline now → threshold crossing
    const now = history[history.length - 1].t
    const steps = 10
    const proj: { t: number; dp: null; proj: number }[] = []
    for (let i = 0; i <= steps; i++) {
      const t = now + (prediction.daysOut * 86400_000 * i) / steps
      proj.push({ t, dp: null, proj: baseline + prediction.slopePerDay * prediction.daysOut * (i / steps) })
    }
    return [...hist, ...proj]
  }, [history, prediction, baseline])

  return (
    <ResponsiveContainer width="100%" height="100%">
      <ComposedChart data={data} margin={{ top: 8, right: 8, bottom: 0, left: -14 }}>
        <defs>
          <linearGradient id="dpFill" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--color-data)" stopOpacity={0.28} />
            <stop offset="100%" stopColor="var(--color-data)" stopOpacity={0.02} />
          </linearGradient>
        </defs>
        <CartesianGrid stroke="var(--color-seam)" vertical={false} />
        <XAxis
          dataKey="t"
          type="number"
          scale="time"
          domain={['dataMin', 'dataMax']}
          tickFormatter={fmtDate}
          {...axis}
        />
        <YAxis domain={[1.5, 6.6]} width={44} tickFormatter={(v: number) => v.toFixed(1)} {...axis} />
        <Tooltip content={<ChartTip />} />
        <ReferenceLine
          y={SERVICE_LIMIT}
          stroke="var(--color-alarm)"
          strokeDasharray="4 4"
          label={{
            value: 'service limit 6.0"',
            fill: "var(--color-alarm)",
            fontSize: 10,
            position: "insideTopRight",
          }}
        />
        <Area
          name="dP"
          dataKey="dp"
          stroke="var(--color-data)"
          strokeWidth={1.4}
          fill="url(#dpFill)"
          isAnimationActive={false}
          connectNulls={false}
          dot={false}
        />
        <Line
          name="AI projection"
          dataKey="proj"
          stroke="var(--color-warn)"
          strokeWidth={1.6}
          strokeDasharray="6 5"
          dot={false}
          isAnimationActive={false}
          connectNulls={false}
        />
      </ComposedChart>
    </ResponsiveContainer>
  )
}

export function EnergyChart() {
  const { history } = useSim()
  const data = useMemo(() => {
    const byDay = new Map<string, { kwh: number; n: number; t: number }>()
    for (const s of history) {
      const key = fmtDate(s.t)
      const cur = byDay.get(key) ?? { kwh: 0, n: 0, t: s.t }
      cur.kwh += s.kw // hourly samples → kW ≈ kWh per sample
      cur.n++
      byDay.set(key, cur)
    }
    return [...byDay.entries()].map(([day, v]) => ({ day, kwh: Math.round(v.kwh) }))
  }, [history])

  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart data={data} margin={{ top: 8, right: 8, bottom: 0, left: -18 }}>
        <CartesianGrid stroke="var(--color-seam)" vertical={false} />
        <XAxis dataKey="day" interval={2} {...axis} />
        <YAxis width={44} {...axis} />
        <Tooltip content={<ChartTip />} cursor={{ fill: "var(--color-surface-2)" }} />
        <Bar name="kWh" dataKey="kwh" fill="var(--color-ok)" radius={[3, 3, 0, 0]} isAnimationActive={false} />
      </BarChart>
    </ResponsiveContainer>
  )
}
