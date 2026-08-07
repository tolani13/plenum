"""CAD-01 · Build both collector geometries and export STEP + GLB.

    python cad/build.py            # writes cad/out/
    python cad/build.py --out DIR  # writes elsewhere

Change a number in `cad/params.py`, re-run, and all four artifacts change.

WHY B-REP AT ALL
    The Three.js collector at /collector is authored directly in primitives.
    A mesh is an approximation with no dimensional authority — you cannot
    measure it to a tolerance, and nothing outside the browser can consume
    it. This module makes the geometry a dimensioned source of truth: STEP
    carries exact mathematical surfaces that any CAD package can open and
    measure, and GLB is the tessellation of that same solid for the browser.
    The Three.js geometry is presentation; `cad/params.py` is the authority.

THIS UNIT IS AUTHORING-ONLY
    Python does not enter the ship path. `Dockerfile`, `render.yaml` and
    `scripts/check.sh` are untouched, the Render image gains no Python layer,
    and CI gains no Python step. The exported files are committed artifacts,
    exactly like `web/src/map/blank-us-map-states-only.svg`. Wiring them into
    the app is CAD-02.

WHAT IS DELIBERATELY NOT MODELLED
    Pleat geometry. Pleats multiply face count enormously and buy nothing
    until a media-area calculation exists to consume them, so a cartridge is
    a plain cylinder and the media area is carried as the parameter
    `cartridge_media_area_m2` instead. Also absent: support legs, platforms,
    the pulse-jet header and blowpipes, gaskets, door hardware, weldment
    detail, and any material or colour assignment (colour belongs to the app
    tokens, and is CAD-02's problem).

DEPENDENCIES AND LICENSES (attribution follows the precedent the committed
map asset set — see README "Territory map" and cad/README.md)
    build123d 0.11.1              Apache-2.0
        https://github.com/gumyr/build123d — the modelling API used here.
    cadquery-ocp-novtk 7.9.3.1.1  Apache-2.0 (package metadata)
    cadquery-ocp-proxy 7.9.3.1.1  Apache-2.0 (package metadata)
        Python bindings to Open CASCADE Technology 7.9.3, the B-rep kernel.
        OCCT itself is distributed by Open Cascade under LGPL-2.1 with the
        Open CASCADE Exception; the wheels ship no license file, so that is
        recorded from the upstream project rather than verified from disk.
    pytest 9.1.1                  MIT — test runner, authoring-only.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from build123d import Compound, Unit, export_gltf, export_step

import crossflow
import downflow
from params import PARAMS, CollectorParams

DEFAULT_OUT = Path(__file__).resolve().parent / "out"

#: (module name, builder) for every geometry this unit exports.
GEOMETRIES = (
    (downflow.NAME, downflow.build),
    (crossflow.NAME, crossflow.build),
)


def export_one(assembly: Compound, name: str, out_dir: Path) -> list[Path]:
    """Write `<name>.step` and `<name>.glb`; return the paths written."""
    step_path = out_dir / f"{name}.step"
    glb_path = out_dir / f"{name}.glb"

    if not export_step(assembly, step_path, unit=Unit.MM):
        raise RuntimeError(f"export_step reported failure for {step_path}")
    if not export_gltf(assembly, glb_path, unit=Unit.MM, binary=True):
        raise RuntimeError(f"export_gltf reported failure for {glb_path}")

    return [step_path, glb_path]


def describe(p: CollectorParams) -> str:
    (x0, x1), (y0, y1), (z0, z1) = p.expected_bounds()
    return (
        f"  parameters: {p.cartridge_count} cartridge(s) "
        f"of {p.cartridge_rows}x{p.cartridge_columns} capacity, "
        f"{p.cartridge_diameter:g} dia x {p.cartridge_length:g} long\n"
        f"  overall:    {p.housing_width:g} W x {p.housing_depth:g} D x "
        f"{p.total_height:g} H (housing {p.housing_height:g} + hopper "
        f"{p.hopper_height:g})\n"
        f"  extents:    X {x0:g}..{x1:g}  Y {y0:g}..{y1:g}  Z {z0:g}..{z1:g}"
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--out",
        type=Path,
        default=DEFAULT_OUT,
        help=f"output directory (default: {DEFAULT_OUT})",
    )
    args = parser.parse_args(argv)

    out_dir: Path = args.out
    out_dir.mkdir(parents=True, exist_ok=True)

    p = PARAMS
    # ASCII only: this prints to a Windows console under cp1252.
    print("PLENUM CAD-01 - parametric collector geometry")
    print(describe(p))
    print(f"  output:     {out_dir}")
    print()

    written: list[Path] = []
    for name, builder in GEOMETRIES:
        assembly = builder(p)
        cartridges = sum(
            1 for leaf in assembly.leaves if str(leaf.label).startswith("cartridge_")
        )
        bbox = assembly.bounding_box()
        print(
            f"{name:<10} built: {len(assembly.children)} solids, "
            f"{cartridges} cartridges, volume {assembly.volume / 1e9:.6f} m^3"
        )
        print(
            f"{'':<10} bbox:  X {bbox.min.X:.3f}..{bbox.max.X:.3f}  "
            f"Y {bbox.min.Y:.3f}..{bbox.max.Y:.3f}  "
            f"Z {bbox.min.Z:.3f}..{bbox.max.Z:.3f}"
        )
        for path in export_one(assembly, name, out_dir):
            written.append(path)
            print(f"{'':<10} wrote: {path.name:<16} {path.stat().st_size:>10,} bytes")
        print()

    print(f"{len(written)} files written to {out_dir}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
