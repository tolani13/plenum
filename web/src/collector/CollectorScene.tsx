import { Suspense } from 'react'
import { Canvas } from '@react-three/fiber'
import { ContactShadows, OrbitControls } from '@react-three/drei'
import CollectorModel from "./CollectorModel";
import { token } from "./materials";

// Deliberately no HDRI/Environment presets: those fetch from a CDN at runtime.
// A demo has to survive venue wifi, so the whole scene is self-contained.

export default function CollectorScene({ exploded }: { exploded: boolean }) {
  return (
    <Canvas
      shadows
      camera={{ position: [6.4, 2.6, 6.8], fov: 42 }}
      dpr={[1, 2]}
      className="touch-none"
    >
      <color attach="background" args={[token("bg")]} />
      <fog attach="fog" args={[token("bg"), 14, 26]} />

      <hemisphereLight intensity={0.5} color={token("coll-light-sky")}
        groundColor={token("coll-light-ground")} />
      <directionalLight
        position={[6, 9, 5]}
        intensity={1.6}
        castShadow
        shadow-mapSize={[1024, 1024]}
      />
      <directionalLight position={[-7, 4, -4]} intensity={0.5}
        color={token("coll-light-fill")} />
      <spotLight position={[0, 10, 0]} intensity={0.4} angle={0.5} penumbra={1} />

      <Suspense fallback={null}>
        <CollectorModel exploded={exploded} />
        <ContactShadows position={[0, -2.85, 0]} opacity={0.55} scale={16} blur={2.2} far={4} />
      </Suspense>

      {/* floor grid */}
      <gridHelper
        args={[30, 30, token("coll-grid"), token("coll-grid-2")]}
        position={[0, -2.84, 0]}
      />

      <OrbitControls
        makeDefault
        autoRotate
        autoRotateSpeed={0.55}
        enablePan={false}
        minDistance={5}
        maxDistance={14}
        maxPolarAngle={Math.PI * 0.55}
        target={[0, 0.4, 0]}
      />
    </Canvas>
  )
}
