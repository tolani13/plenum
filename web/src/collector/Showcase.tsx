import { useState } from 'react'
import { Layers } from 'lucide-react'
import CollectorScene from "./CollectorScene";
import { useSim } from "./simulator";

export default function Showcase() {
  const [exploded, setExploded] = useState(false)
  const { live } = useSim()

  return (
    <div className="relative h-[min(72vh,820px)] min-h-[26rem] w-full overflow-hidden rounded-lg border border-seam">
      <CollectorScene exploded={exploded} />

      {/* spec plate — the nameplate riveted to the corner of the view */}
      <div className="pointer-events-none absolute left-4 top-4 max-w-[min(320px,calc(100%-2rem))]">
        <div className="rounded-lg border border-seam bg-surface pointer-events-auto p-4">
          <div className="nameplate-strong text-2xs text-text-dim mb-1">Cartridge dust collector</div>
          <h1 className="nameplate-strong text-3xl font-semibold uppercase leading-none tracking-wide">
            GSX-class · 8-cartridge
          </h1>
          <dl className="mt-3 grid grid-cols-2 gap-x-4 gap-y-1 text-xs text-text-dim">
            <dt>Design airflow</dt>
            <dd className="text-right tabular text-text">8,200 CFM</dd>
            <dt>Live dP</dt>
            <dd className="text-right tabular text-text">{live.dp.toFixed(2)}"</dd>
            <dt>Cleaning</dt>
            <dd className="text-right text-text">Pulse-jet</dd>
            <dt>Telemetry</dt>
            <dd className="text-right text-ok">Streaming</dd>
          </dl>
          <p className="mt-3 text-xs leading-relaxed text-text-faint">
            Drag to orbit. Tap a <span className="text-data">+</span> marker for the story at each
            subsystem. When the pulse header flashes, the dashboard sawtooth just reset — same
            event, both surfaces.
          </p>
        </div>
      </div>

      <div className="absolute bottom-4 left-1/2 -translate-x-1/2">
        <button
          onClick={() => setExploded((e) => !e)}
          className="flex items-center gap-2 rounded-full border border-seam bg-surface/90 px-4 py-2 text-sm text-text backdrop-blur transition-colors hover:border-data/60"
        >
          <Layers className="h-4 w-4 text-data" />
          {exploded ? 'Assemble unit' : 'Exploded view'}
        </button>
      </div>
    </div>
  )
}
