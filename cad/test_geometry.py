"""CAD-01 · Dimensional assertions on the built solids.

    cad/.venv/Scripts/python -m pytest cad/test_geometry.py -v

These tests measure the geometry that was actually built — bounding boxes of
labelled leaves in the assembly compound — rather than re-deriving the
arithmetic they are meant to be checking. The two orientation tests
(`test_downflow_inlet_is_above_all_media`,
`test_crossflow_inlet_is_below_all_media`) are the ones that matter: they
assert the defining feature of each geometry instead of assuming it.
"""

from __future__ import annotations

from dataclasses import replace
from pathlib import Path

import pytest
from build123d import Compound

import build as build_cli
import crossflow
import downflow
from common import LABEL_CARTRIDGE_PREFIX, LABEL_INLET
from params import PARAMS, CollectorParams

#: Tolerance for a dimension that should land exactly on a parameter value.
#: OCCT reports these to machine precision; 0.1 mm is the unit's contract.
TOL_MM = 0.1


# ── fixtures ─────────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def params() -> CollectorParams:
    return PARAMS


@pytest.fixture(scope="module")
def downflow_asm(params: CollectorParams) -> Compound:
    return downflow.build(params)


@pytest.fixture(scope="module")
def crossflow_asm(params: CollectorParams) -> Compound:
    return crossflow.build(params)


# ── helpers ──────────────────────────────────────────────────────────────


def leaf(assembly: Compound, label: str):
    """The one leaf carrying `label`."""
    found = [item for item in assembly.leaves if item.label == label]
    assert len(found) == 1, f"expected exactly one {label!r}, found {len(found)}"
    return found[0]


def cartridges(assembly: Compound) -> list:
    return [
        item
        for item in assembly.leaves
        if str(item.label).startswith(LABEL_CARTRIDGE_PREFIX)
    ]


# ── 1 · both geometries build ────────────────────────────────────────────


def test_downflow_builds(downflow_asm: Compound) -> None:
    assert downflow_asm.volume > 0


def test_crossflow_builds(crossflow_asm: Compound) -> None:
    assert crossflow_asm.volume > 0


# ── 2 · overall bounding box matches the parameter-derived expectation ───


@pytest.mark.parametrize("name", ["downflow", "crossflow"])
def test_bounding_box_matches_parameters(
    name: str, params: CollectorParams, downflow_asm: Compound, crossflow_asm: Compound
) -> None:
    assembly = downflow_asm if name == "downflow" else crossflow_asm
    bbox = assembly.bounding_box()
    (x0, x1), (y0, y1), (z0, z1) = params.expected_bounds()

    assert bbox.min.X == pytest.approx(x0, abs=TOL_MM)
    assert bbox.max.X == pytest.approx(x1, abs=TOL_MM)
    assert bbox.min.Y == pytest.approx(y0, abs=TOL_MM)
    assert bbox.max.Y == pytest.approx(y1, abs=TOL_MM)
    assert bbox.min.Z == pytest.approx(z0, abs=TOL_MM)
    assert bbox.max.Z == pytest.approx(z1, abs=TOL_MM)


@pytest.mark.parametrize("name", ["downflow", "crossflow"])
def test_overall_height_is_housing_plus_hopper(
    name: str, params: CollectorParams, downflow_asm: Compound, crossflow_asm: Compound
) -> None:
    """D.'s acceptance check 2, as an assertion."""
    assembly = downflow_asm if name == "downflow" else crossflow_asm
    bbox = assembly.bounding_box()
    measured = bbox.max.Z - bbox.min.Z
    assert measured == pytest.approx(
        params.housing_height + params.hopper_height, abs=TOL_MM
    )


# ── 3 · cartridge count ──────────────────────────────────────────────────


@pytest.mark.parametrize("name", ["downflow", "crossflow"])
def test_cartridge_count(
    name: str, params: CollectorParams, downflow_asm: Compound, crossflow_asm: Compound
) -> None:
    assembly = downflow_asm if name == "downflow" else crossflow_asm
    assert len(cartridges(assembly)) == params.cartridge_count


@pytest.mark.parametrize("name", ["downflow", "crossflow"])
def test_cartridge_dimensions(
    name: str, params: CollectorParams, downflow_asm: Compound, crossflow_asm: Compound
) -> None:
    """A cartridge measures diameter x diameter x length on its own axes."""
    assembly = downflow_asm if name == "downflow" else crossflow_asm
    horizontal = name == "downflow"
    for item in cartridges(assembly):
        bbox = item.bounding_box()
        along = bbox.max.Y - bbox.min.Y if horizontal else bbox.max.Z - bbox.min.Z
        across = bbox.max.Z - bbox.min.Z if horizontal else bbox.max.Y - bbox.min.Y
        assert along == pytest.approx(params.cartridge_length, abs=TOL_MM)
        assert across == pytest.approx(params.cartridge_diameter, abs=TOL_MM)
        assert bbox.max.X - bbox.min.X == pytest.approx(
            params.cartridge_diameter, abs=TOL_MM
        )


# ── 4 · Geometry A is downflow: the inlet is above ALL media ─────────────


def test_downflow_inlet_is_above_all_media(
    downflow_asm: Compound, params: CollectorParams
) -> None:
    """The defining feature of Geometry A — asserted on the built solids.

    The inlet duct's bore is `inlet_wall_thickness` inside its outer skin, so
    the opening's lower edge is the duct's own minimum Z plus that skin.
    """
    duct = leaf(downflow_asm, LABEL_INLET).bounding_box()
    opening_lower_z = duct.min.Z + params.inlet_wall_thickness
    highest_media_top = max(c.bounding_box().max.Z for c in cartridges(downflow_asm))

    assert opening_lower_z > highest_media_top, (
        f"inlet opening lower edge {opening_lower_z:.3f} is not above the "
        f"topmost cartridge surface {highest_media_top:.3f}"
    )


# ── 5 · Geometry B is crossflow: the inlet is below ALL media ───────────


def test_crossflow_inlet_is_below_all_media(
    crossflow_asm: Compound, params: CollectorParams
) -> None:
    """The defining feature of Geometry B — asserted on the built solids."""
    duct = leaf(crossflow_asm, LABEL_INLET).bounding_box()
    opening_upper_z = duct.max.Z - params.inlet_wall_thickness
    lowest_media_bottom = min(
        c.bounding_box().min.Z for c in cartridges(crossflow_asm)
    )

    assert opening_upper_z < lowest_media_bottom, (
        f"inlet opening upper edge {opening_upper_z:.3f} is not below the "
        f"lowest cartridge surface {lowest_media_bottom:.3f}"
    )


def test_the_two_geometries_differ_in_cartridge_axis(
    downflow_asm: Compound, crossflow_asm: Compound, params: CollectorParams
) -> None:
    """A is horizontal (long in Y), B is vertical (long in Z). Not the same model."""
    a = cartridges(downflow_asm)[0].bounding_box()
    b = cartridges(crossflow_asm)[0].bounding_box()
    assert a.max.Y - a.min.Y == pytest.approx(params.cartridge_length, abs=TOL_MM)
    assert a.max.Z - a.min.Z == pytest.approx(params.cartridge_diameter, abs=TOL_MM)
    assert b.max.Z - b.min.Z == pytest.approx(params.cartridge_length, abs=TOL_MM)
    assert b.max.Y - b.min.Y == pytest.approx(params.cartridge_diameter, abs=TOL_MM)


# ── 6 · the parameter set actually drives the solids ────────────────────


@pytest.mark.parametrize("module", [downflow, crossflow])
def test_halving_cartridge_count_changes_volume_and_count(module) -> None:
    """D.'s acceptance check 5, as an assertion."""
    full = PARAMS
    half = replace(full, cartridge_count=full.cartridge_count // 2)

    full_asm = module.build(full)
    half_asm = module.build(half)

    assert len(cartridges(half_asm)) == half.cartridge_count
    assert len(cartridges(full_asm)) == full.cartridge_count
    assert half_asm.volume != pytest.approx(full_asm.volume, rel=1e-9)
    assert half_asm.volume < full_asm.volume


@pytest.mark.parametrize("module", [downflow, crossflow])
def test_changing_housing_height_changes_overall_height(module) -> None:
    """One number in params.py, both geometries move."""
    taller = replace(PARAMS, housing_height=PARAMS.housing_height + 500.0)
    base_bbox = module.build(PARAMS).bounding_box()
    tall_bbox = module.build(taller).bounding_box()
    grew = (tall_bbox.max.Z - tall_bbox.min.Z) - (base_bbox.max.Z - base_bbox.min.Z)
    assert grew == pytest.approx(500.0, abs=TOL_MM)


def test_parameter_validation_rejects_an_overfull_bank() -> None:
    with pytest.raises(ValueError):
        replace(PARAMS, cartridge_count=PARAMS.cartridge_rows * PARAMS.cartridge_columns + 1)


# ── 7 · every exported file exists and is non-zero ──────────────────────


def test_exports_are_written(tmp_path: Path) -> None:
    assert build_cli.main(["--out", str(tmp_path)]) == 0

    expected = [
        tmp_path / "downflow.step",
        tmp_path / "downflow.glb",
        tmp_path / "crossflow.step",
        tmp_path / "crossflow.glb",
    ]
    for path in expected:
        assert path.exists(), f"{path.name} was not written"
        assert path.stat().st_size > 0, f"{path.name} is empty"

    # STEP is text and self-identifying; GLB is binary and starts with 'glTF'.
    assert (tmp_path / "downflow.step").read_text(errors="ignore").startswith("ISO-10303-21")
    assert (tmp_path / "downflow.glb").read_bytes()[:4] == b"glTF"
    assert (tmp_path / "crossflow.glb").read_bytes()[:4] == b"glTF"


def test_committed_artifacts_are_present_and_non_zero() -> None:
    """The four artifacts in cad/out/ are the committed deliverable."""
    for name in ("downflow", "crossflow"):
        for suffix in (".step", ".glb"):
            path = build_cli.DEFAULT_OUT / f"{name}{suffix}"
            assert path.exists(), f"{path} is missing — run python cad/build.py"
            assert path.stat().st_size > 0
