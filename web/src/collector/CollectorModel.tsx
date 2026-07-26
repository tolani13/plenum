import { useMemo, useRef, useState } from 'react'
import { useFrame } from '@react-three/fiber'
import { Html } from '@react-three/drei'
import * as THREE from 'three'
import { sim } from "./simulator";
import { materials, token } from "./materials";

// A cartridge dust collector built from primitives:
// clean-air plenum + outlet on top, cartridge housing with access doors,
// side inlet, pulse header across the front, pyramid hopper + discharge below.

export interface Hotspot {
  id: string
  title: string
  body: string
  position: [number, number, number]
}

export const HOTSPOTS: Hotspot[] = [
  {
    id: 'inlet',
    title: 'High-entry inlet',
    body: 'Dust-laden air enters above the cartridges; heavy particulate drops straight to the hopper, cutting the load the filters ever see.',
    position: [-2.35, 1.5, 0],
  },
  {
    id: 'cartridges',
    title: 'Pleated cartridge bank',
    body: '8 vertical cartridges. Open-pleat media loads in depth — the sawtooth floor on the dP chart is this bank slowly aging.',
    position: [0, 0.8, 1.55],
  },
  {
    id: 'pulse',
    title: 'Pulse-jet cleaning',
    body: 'Compressed-air header fires row by row when dP hits the clean setpoint. Watch it flash here when the dashboard sawtooth resets.',
    position: [0, 2.35, 1.35],
  },
  {
    id: 'hopper',
    title: 'Hopper + discharge',
    body: 'Collected dust exits through the rotary discharge into the drum — the point where NFPA-driven housekeeping either works or does not.',
    position: [0, -2.1, 0.9],
  },
]

// Values live in tokens.css; materials.ts resolves them (palette law).
const steel = materials.steel;
const steelDark = materials.steelDark;
const drum = materials.drum;

function Cartridges({ open }: { open: boolean }) {
  const group = useRef<THREE.Group>(null)
  useFrame((state) => {
    if (!group.current) return
    // pulse shake shortly after a pulse event
    const since = Date.now() - sim.getState().pulseAt
    const amp = since < 600 ? (1 - since / 600) * 0.03 : 0
    group.current.position.x = amp * Math.sin(state.clock.elapsedTime * 60)
  })
  const items = useMemo(() => {
    const arr: { pos: [number, number, number]; key: string }[] = []
    for (let r = 0; r < 2; r++)
      for (let c = 0; c < 4; c++)
        arr.push({ key: `c${r}${c}`, pos: [-1.05 + c * 0.7, 0.75, r === 0 ? -0.45 : 0.45] })
    return arr
  }, [])
  return (
    <group ref={group}>
      {items.map(({ key, pos }) => (
        <group key={key} position={[pos[0], pos[1], pos[2] + (open ? (pos[2] > 0 ? 1.5 : 0.9) : 0)]}>
          <mesh castShadow>
            <cylinderGeometry args={[0.24, 0.24, 1.35, 24]} />
            <meshStandardMaterial
              color={token("coll-media")}
              roughness={0.85}
              metalness={0.05}
            />
          </mesh>
          {/* pleat ribs */}
          {[0.45, 0.15, -0.15, -0.45].map((y) => (
            <mesh key={y} position={[0, y, 0]}>
              <torusGeometry args={[0.245, 0.012, 8, 24]} />
              <meshStandardMaterial color={token("coll-media-rib")} roughness={0.9} />
            </mesh>
          ))}
          <mesh position={[0, 0.72, 0]}>
            <cylinderGeometry args={[0.27, 0.27, 0.06, 24]} />
            <meshStandardMaterial {...steelDark()} />
          </mesh>
        </group>
      ))}
    </group>
  )
}

function PulseHeader() {
  const mat = useRef<THREE.MeshStandardMaterial>(null)
  useFrame(() => {
    if (!mat.current) return
    const since = Date.now() - sim.getState().pulseAt
    const glow = since < 900 ? (1 - since / 900) * 2.2 : 0
    mat.current.emissiveIntensity = glow
  })
  return (
    <group position={[0, 2.28, 1.18]}>
      <mesh castShadow rotation={[0, 0, Math.PI / 2]}>
        <cylinderGeometry args={[0.11, 0.11, 3.1, 20]} />
        <meshStandardMaterial
          ref={mat}
          color={token("coll-steel")}
          metalness={0.7}
          roughness={0.35}
          emissive={token("data")}
          emissiveIntensity={0}
        />
      </mesh>
      {[-1.05, -0.35, 0.35, 1.05].map((x) => (
        <mesh key={x} position={[x, 0.16, 0]} castShadow>
          <boxGeometry args={[0.18, 0.16, 0.18]} />
          <meshStandardMaterial {...steelDark()} />
        </mesh>
      ))}
    </group>
  )
}

function Door({ x, y }: { x: number; y: number }) {
  return (
    <group position={[x, y, 1.06]}>
      <mesh castShadow>
        <boxGeometry args={[1.28, 1.28, 0.08]} />
        <meshStandardMaterial {...steel()} />
      </mesh>
      <mesh position={[0.5, 0, 0.07]}>
        <cylinderGeometry args={[0.045, 0.045, 0.22, 12]} />
        <meshStandardMaterial
          color={token("coll-fitting")}
          metalness={0.4}
          roughness={0.4}
        />
      </mesh>
    </group>
  )
}

export default function CollectorModel({ exploded }: { exploded: boolean }) {
  const [active, setActive] = useState<string | null>(null)
  const plenum = useRef<THREE.Group>(null)
  const doors = useRef<THREE.Group>(null)
  const hopper = useRef<THREE.Group>(null)
  const cartsOpen = useRef(false)

  useFrame((_, dt) => {
    const k = 1 - Math.pow(0.001, dt) // frame-rate independent lerp
    const target = exploded ? 1 : 0
    if (plenum.current)
      plenum.current.position.y = THREE.MathUtils.lerp(plenum.current.position.y, target * 1.2, k)
    if (doors.current)
      doors.current.position.z = THREE.MathUtils.lerp(doors.current.position.z, target * 1.6, k)
    if (hopper.current)
      hopper.current.position.y = THREE.MathUtils.lerp(hopper.current.position.y, target * -1.0, k)
    cartsOpen.current = exploded
  })

  return (
    <group position={[0, 0.4, 0]}>
      {/* legs */}
      {[
        [-1.5, -0.85],
        [1.5, -0.85],
        [-1.5, 0.85],
        [1.5, 0.85],
      ].map(([x, z]) => (
        <mesh key={`${x}${z}`} position={[x, -2.55, z]} castShadow>
          <boxGeometry args={[0.16, 2.4, 0.16]} />
          <meshStandardMaterial {...steelDark()} />
        </mesh>
      ))}

      {/* hopper + discharge */}
      <group ref={hopper}>
        <mesh position={[0, -1.55, 0]} rotation={[0, Math.PI / 4, 0]} castShadow>
          <coneGeometry args={[2.05, 1.5, 4]} />
          <meshStandardMaterial {...steel()} side={THREE.DoubleSide} />
        </mesh>
        <mesh position={[0, -2.45, 0]} castShadow>
          <cylinderGeometry args={[0.24, 0.24, 0.35, 16]} />
          <meshStandardMaterial {...steelDark()} />
        </mesh>
        <mesh position={[0, -3.0, 0]} castShadow>
          <cylinderGeometry args={[0.5, 0.5, 0.8, 24]} />
          <meshStandardMaterial {...drum()} />
        </mesh>
      </group>

      {/* main housing */}
      <mesh position={[0, 0.6, 0]} castShadow receiveShadow>
        <boxGeometry args={[2.9, 2.9, 2.0]} />
        <meshStandardMaterial {...steel()} />
      </mesh>
      {/* panel seams */}
      {[-0.72, 0.72].map((x) => (
        <mesh key={x} position={[x, 0.6, 1.005]}>
          <boxGeometry args={[0.02, 2.9, 0.02]} />
          <meshStandardMaterial color={token("coll-seam-line")} />
        </mesh>
      ))}

      {/* access doors */}
      <group ref={doors}>
        <Door x={-0.72} y={1.28} />
        <Door x={0.72} y={1.28} />
        <Door x={-0.72} y={-0.08} />
        <Door x={0.72} y={-0.08} />
      </group>

      {/* cartridges (visible through the exploded doors) */}
      <Cartridges open={exploded} />

      {/* pulse header */}
      <PulseHeader />

      {/* clean-air plenum + outlet */}
      <group ref={plenum}>
        <mesh position={[0, 2.35, 0]} castShadow>
          <boxGeometry args={[2.9, 0.6, 2.0]} />
          <meshStandardMaterial {...steelDark()} />
        </mesh>
        <mesh position={[1.9, 2.35, 0]} rotation={[0, 0, Math.PI / 2]} castShadow>
          <cylinderGeometry args={[0.42, 0.42, 1.1, 24]} />
          <meshStandardMaterial {...steel()} />
        </mesh>
        <mesh position={[2.5, 2.35, 0]} rotation={[0, 0, Math.PI / 2]}>
          <cylinderGeometry args={[0.5, 0.5, 0.1, 24]} />
          <meshStandardMaterial {...steelDark()} />
        </mesh>
      </group>

      {/* inlet duct */}
      <group position={[-1.85, 1.5, 0]}>
        <mesh castShadow>
          <boxGeometry args={[0.9, 0.75, 0.75]} />
          <meshStandardMaterial {...steel()} />
        </mesh>
        <mesh position={[-0.5, 0, 0]}>
          <boxGeometry args={[0.1, 0.95, 0.95]} />
          <meshStandardMaterial {...steelDark()} />
        </mesh>
      </group>

      {/* nameplate */}
      <mesh position={[1.05, 0.25, 1.07]}>
        <boxGeometry args={[0.55, 0.32, 0.02]} />
        <meshStandardMaterial
          color={token("coll-plate")}
          metalness={0.8}
          roughness={0.3}
        />
      </mesh>

      {/* hotspots */}
      {HOTSPOTS.map((h) => (
        <Html key={h.id} position={h.position} center zIndexRange={[20, 0]}>
          <button
            onClick={() => setActive(active === h.id ? null : h.id)}
            aria-label={h.title}
            className="relative flex h-6 w-6 items-center justify-center rounded-full border border-data/70 bg-bg/80 text-data backdrop-blur-sm transition-transform hover:scale-110"
          >
            <span className="text-xs leading-none">+</span>
          </button>
          {active === h.id && (
            <div className="absolute left-8 top-1/2 w-56 -translate-y-1/2 rounded-md border border-seam bg-surface/95 p-3 text-left shadow-xl backdrop-blur">
              <div className="nameplate-strong text-2xs text-text-dim mb-1 text-data!">{h.title}</div>
              <p className="text-xs leading-relaxed text-text-dim">{h.body}</p>
            </div>
          )}
        </Html>
      ))}
    </group>
  )
}
