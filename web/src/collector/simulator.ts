// Simulated telemetry for a cartridge dust collector.
// Physics kept honest enough for a domain audience:
//  - Differential pressure (dP) rises as dust cake builds on cartridges.
//  - Pulse-jet cleaning fires when dP crosses the clean setpoint, knocking
//    cake off — the classic sawtooth. A little cake never comes back
//    (depth loading), so the sawtooth floor drifts upward over filter life.
//  - Fan power tracks dP at roughly constant airflow.
// The "AI" layer fits the baseline drift and projects the date the unit
// crosses the 6.0 in. w.g. service threshold.

import { useSyncExternalStore } from 'react'

export interface Sample {
  t: number // epoch ms
  dp: number // differential pressure, in. w.g.
  cfm: number // airflow
  kw: number // fan power
}

export interface Cartridge {
  id: string
  row: number
  col: number
  health: number // 0..100
  flagged: boolean
}

export interface Prediction {
  serviceDate: number
  daysOut: number
  confidence: number // 0..1
  slopePerDay: number // in. w.g. per day of baseline drift
}

export interface AlertItem {
  t: number
  kind: 'pulse' | 'warn' | 'info'
  text: string
}

export interface SimState {
  history: Sample[] // hourly, trailing 14 days
  live: Sample
  baseline: number
  prediction: Prediction
  cartridges: Cartridge[]
  alerts: AlertItem[]
  pulseAt: number // epoch ms of last pulse event (drives the 3D flash)
}

export const SERVICE_LIMIT = 6.0
export const CLEAN_SETPOINT = 4.6
const HOURS = 24 * 14
const NOMINAL_CFM = 8200

let rngState = 20260717
function rand(): number {
  rngState = (rngState * 1664525 + 1013904223) >>> 0
  return rngState / 0xffffffff
}
const jitter = (amt: number) => (rand() - 0.5) * 2 * amt
const round2 = (x: number) => Math.round(x * 100) / 100

function fanKw(dp: number, cfm: number): number {
  return round2(9 + dp * 1.9 + (cfm - NOMINAL_CFM) * 0.001 + jitter(0.25))
}

function buildHistory(now: number): { history: Sample[]; baseline: number; cake: number } {
  const history: Sample[] = []
  let baseline = 2.15
  let cake = 0.3
  for (let i = HOURS; i >= 1; i--) {
    const t = now - i * 3600_000
    baseline += 0.0048 + jitter(0.0012)
    cake += 0.09 + jitter(0.03)
    let dp = baseline + cake
    if (dp > CLEAN_SETPOINT + jitter(0.15)) {
      cake = 0.12 + rand() * 0.1
      dp = baseline + cake
    }
    const cfm = Math.round(NOMINAL_CFM - dp * 55 + jitter(60))
    history.push({ t, dp: round2(dp), cfm, kw: fanKw(dp, cfm) })
  }
  return { history, baseline, cake }
}

function fitPrediction(history: Sample[], now: number): Prediction {
  // Baseline per day = daily minimum dP (the sawtooth floor). Linear fit → crossing date.
  const days = new Map<number, number>()
  for (const s of history) {
    const d = Math.floor(s.t / 86400_000)
    days.set(d, Math.min(days.get(d) ?? Infinity, s.dp))
  }
  const xs = [...days.keys()].sort((a, b) => a - b)
  const ys = xs.map((d) => days.get(d) as number)
  const n = xs.length
  const x0 = xs[0]
  const xn = xs.map((x) => x - x0)
  const sx = xn.reduce((a, b) => a + b, 0)
  const sy = ys.reduce((a, b) => a + b, 0)
  const sxy = xn.reduce((a, b, i) => a + b * ys[i], 0)
  const sxx = xn.reduce((a, b) => a + b * b, 0)
  const slope = (n * sxy - sx * sy) / (n * sxx - sx * sx)
  const intercept = (sy - slope * sx) / n
  const yhat = xn.map((x) => intercept + slope * x)
  const my = sy / n
  const ssRes = ys.reduce((a, y, i) => a + (y - yhat[i]) ** 2, 0)
  const ssTot = ys.reduce((a, y) => a + (y - my) ** 2, 0)
  const r2 = ssTot === 0 ? 0 : 1 - ssRes / ssTot
  const nowDay = now / 86400_000 - x0
  const currentBase = intercept + slope * nowDay
  const daysOut = Math.max(0, (SERVICE_LIMIT - currentBase) / Math.max(slope, 1e-6))
  return {
    serviceDate: now + daysOut * 86400_000,
    daysOut,
    confidence: Math.min(0.97, Math.max(0.5, r2)),
    slopePerDay: slope,
  }
}

function buildCartridges(): Cartridge[] {
  const out: Cartridge[] = []
  for (let r = 0; r < 2; r++) {
    for (let c = 0; c < 4; c++) {
      const n = r * 4 + c + 1
      const flagged = n === 6 // C-06 carries the anomaly story
      out.push({
        id: `C-0${n}`,
        row: r,
        col: c,
        health: flagged ? 34 : Math.round(58 + rand() * 20),
        flagged,
      })
    }
  }
  return out
}

type Listener = () => void

function createSim() {
  const now = Date.now()
  const built = buildHistory(now)
  let cake = built.history[built.history.length - 1].dp - built.baseline
  let baseline = built.baseline

  let state: SimState = {
    history: built.history,
    live: built.history[built.history.length - 1],
    baseline,
    prediction: fitPrediction(built.history, now),
    cartridges: buildCartridges(),
    alerts: [
      { t: now - 40 * 60_000, kind: 'warn', text: 'C-06 emission signature above bank median — seat inspection recommended' },
      { t: now - 3 * 3600_000, kind: 'pulse', text: 'Pulse cycle complete · row 2 · header 92 psi' },
      { t: now - 9 * 3600_000, kind: 'info', text: 'Baseline drift model refit · confidence 0.91' },
    ],
    pulseAt: 0,
  }

  const listeners = new Set<Listener>()
  const emit = () => listeners.forEach((l) => l())

  function tick() {
    const t = Date.now()
    baseline += 0.00002
    cake += 0.028 + jitter(0.012)
    let dp = baseline + cake
    let pulseAt = state.pulseAt
    let alerts = state.alerts
    if (dp > CLEAN_SETPOINT) {
      cake = 0.14 + rand() * 0.1
      dp = baseline + cake
      pulseAt = t
      alerts = [{ t, kind: 'pulse' as const, text: 'Pulse cycle fired · dP reset to floor' }, ...alerts].slice(0, 8)
    }
    const cfm = Math.round(NOMINAL_CFM - dp * 55 + jitter(50))
    // replace the snapshot object so useSyncExternalStore sees a change
    state = {
      ...state,
      baseline,
      live: { t, dp: round2(dp), cfm, kw: fanKw(dp, cfm) },
      pulseAt,
      alerts,
    }
    emit()
  }

  const interval = setInterval(tick, 2500)
  if (typeof window !== 'undefined') {
    window.addEventListener('beforeunload', () => clearInterval(interval))
  }

  return {
    getState: () => state,
    subscribe: (l: Listener) => {
      listeners.add(l)
      return () => {
        listeners.delete(l)
      }
    },
  }
}

export const sim = createSim()

export function useSim(): SimState {
  return useSyncExternalStore(sim.subscribe, sim.getState)
}

export const fmtDate = (t: number) =>
  new Date(t).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
export const fmtTime = (t: number) =>
  new Date(t).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })
