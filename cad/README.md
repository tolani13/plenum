# `cad/` — parametric collector geometry

The dimensional source of truth for the collector. Two cartridge orientations
are built from one parameter set and exported twice each: **STEP** (B-rep
solid, for CAD and desktop use) and **GLB** (binary glTF mesh, for the
browser).

- **Geometry A — downflow / horizontal cartridges.** Dirty-air inlet above all
  filtration media; cartridges cantilevered horizontally off a vertical tube
  sheet; clean plenum behind that tube sheet.
- **Geometry B — crossflow / vertical cartridges.** Cartridges hung vertically
  from a horizontal tube sheet; inlet at the side, below the media; clean
  plenum above the tube sheet.

The Three.js collector at `/collector` is **presentation only**. It must not
diverge from `cad/params.py`.

## Python is an authoring tool, not part of the ship path

`Dockerfile`, `render.yaml` and `scripts/check.sh` are untouched by this
module. The Render image gains no Python layer and CI gains no Python step.
The exported files in `out/` are **committed artifacts**, exactly like
`web/src/map/blank-us-map-states-only.svg` — the repo's existing precedent for
committed static geometry. Wiring the `.glb` files into the app is CAD-02.

## Run it

```
py -3.13 -m venv cad/.venv
cad/.venv/Scripts/python -m pip install -r cad/requirements.txt
cad/.venv/Scripts/python cad/build.py
cad/.venv/Scripts/python -m pytest cad/test_geometry.py -v
```

`cad/.venv/` is git-ignored. On a machine where `python` already resolves to
3.11–3.13 with build123d installed, `python cad/build.py` is enough.

Change a number in `cad/params.py`, re-run `build.py`, and all four artifacts
in `out/` change accordingly.

A rebuild that changed no dimension is **byte-identical** — the STEP header
timestamp is pinned from `params.step_timestamp` rather than the wall clock,
so a dirty `cad/out/` in `git status` means the artifacts are genuinely stale,
never that you re-ran the build.

## Files

| File | What it is |
|---|---|
| `params.py` | The dataclass. Every dimension in the model, plus derived geometry and the expected overall extents. |
| `common.py` | Shared solids — housing shell, hopper, tube sheets, ducts, cartridges, the cartridge grid. |
| `downflow.py` | Geometry A assembly. |
| `crossflow.py` | Geometry B assembly. |
| `build.py` | CLI. Builds both, exports STEP + GLB to `out/`. |
| `test_geometry.py` | Dimensional assertions, including the two that assert each geometry's defining feature. |
| `out/` | Generated artifacts, committed. |

## Deliberately not modelled

**Pleats.** A cartridge is a plain cylinder. Pleat geometry multiplies face
count enormously and buys nothing until a media-area calculation exists to
consume it, so media area is carried as the parameter
`cartridge_media_area_m2` instead — the number is present in the source of
truth without the faces.

Also absent: support legs, platforms, the pulse-jet header and blowpipes,
gaskets, door hardware, weldment detail, and any material or colour
assignment. Colour belongs to `web/src/styles/tokens.css` under the palette
law and is CAD-02's problem, not this module's.

The `-Y` housing wall is omitted (`housing_cutaway`) so the cartridge bank is
visible in a mesh viewer. The roof and hopper rim still span the full depth,
so the omission does not change the assembly's extents.

## Dependencies and licenses

Attribution follows the precedent the committed map asset set (see the root
README, "Territory map").

| Package | Version | License | Why |
|---|---|---|---|
| [build123d](https://github.com/gumyr/build123d) | 0.11.1 | Apache-2.0 | The modelling API. Actively-developed successor to CadQuery over the same kernel, cleaner API, git-diffable Python source, headless. |
| cadquery-ocp-novtk | 7.9.3.1.1 | Apache-2.0 (package metadata) | Python bindings to the Open CASCADE B-rep kernel — where the dimensional precision actually lives. Pulled in by build123d. |
| cadquery-ocp-proxy | 7.9.3.1.1 | Apache-2.0 (package metadata) | Selector shim that resolves the OCP wheel. Pulled in by build123d. |
| pytest | 9.1.1 | MIT | Test runner. Authoring-only. |

The OCP wheels bind **Open CASCADE Technology 7.9.3**. OCCT is distributed by
Open Cascade under **LGPL-2.1 with the Open CASCADE Exception**; the wheels
ship no license file, so that is recorded from the upstream project rather
than verified from disk. The wheel's own metadata declares Apache-2.0.

Transitive dependencies of build123d (numpy, scipy, scikit-learn, ezdxf,
svgpathtools, ipython and others) are installed into the local venv and never
reach the repo or the deployed image.

## Why B-rep rather than more Three.js primitives

A mesh is an approximation with no dimensional authority: it cannot be
measured to a tolerance, and nothing outside the browser can consume it. A
STEP file carries exact mathematical surfaces that any CAD package can open
and measure. The GLB is the tessellation of that same solid, for the browser —
so the two surfaces cannot drift, because they are exported from one source.
