"""CAD-01 · The parameter set — the single source of truth for collector geometry.

Every dimension that appears in a solid comes from this file. A bare number
inside a geometry function is a defect (unit constraint 9); the only numbers
allowed downstream are indices, counts already declared here, and the epsilon
used to guarantee clean boolean cuts.

UNITS
    Millimetres and degrees throughout. STEP is written with Unit.MM, glTF
    likewise, so a viewer that honours units measures the same numbers you
    read here.

COORDINATE SYSTEM (shared by both geometries)
    +X  width      — the inlet duct leaves the -X wall, the clean-air outlet
                     the +X wall.
    +Y  depth      — for the downflow build the clean plenum sits at +Y,
                     behind a vertical tube sheet; the cartridges cantilever
                     toward -Y.
    +Z  height     — z = 0 is the hopper discharge face (the bottom of the
                     machine). The housing floor — where the hopper's wide rim
                     meets the housing — is z = hopper_height. The housing top
                     is z = hopper_height + housing_height, which is therefore
                     the overall height of the collector.

WHAT IS NOT MODELLED (deliberate, see also the module docstring in build.py)
    Pleats. A cartridge is a plain cylinder. Modelling pleat geometry
    multiplies face count enormously and buys nothing until a media-area
    calculation exists to consume it; the media area is carried here as the
    parameter `cartridge_media_area_m2` instead, so the number is present in
    the source of truth without the faces.
    Also absent: support legs, ladders/platforms, the pulse-jet header and
    blowpipes, gaskets, door hardware, and any weldment detail.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class CollectorParams:
    """One parameter set; both cartridge orientations are built from it.

    Fields are grouped by the subassembly they drive. Where a dimension is
    meaningful for only one of the two geometries, the field name says so.
    """

    # ── housing ──────────────────────────────────────────────────────────
    housing_width: float = 1800.0
    housing_depth: float = 1500.0
    housing_height: float = 2200.0
    housing_wall_thickness: float = 6.0
    # The -Y wall is omitted so the cartridge bank is visible in a viewer.
    # Set False for a fully enclosed machine; nothing else changes, including
    # the overall bounding box (the roof and the hopper rim still span the
    # full depth).
    housing_cutaway: bool = True

    # ── hopper ───────────────────────────────────────────────────────────
    hopper_height: float = 1100.0
    hopper_outlet_size: float = 300.0
    hopper_wall_thickness: float = 6.0

    # ── tube sheet ───────────────────────────────────────────────────────
    # Vertical (normal = Y) in the downflow build, horizontal (normal = Z) in
    # the crossflow build. One thickness serves both.
    tube_sheet_thickness: float = 6.0

    # ── clean plenum ─────────────────────────────────────────────────────
    # Crossflow: the vertical extent of the clean plenum above the horizontal
    # tube sheet, measured down from the housing top.
    clean_plenum_height: float = 500.0
    # Downflow: the same chamber lying on its side — the extent from the +Y
    # wall inward to the vertical tube sheet.
    clean_plenum_depth: float = 500.0

    # ── cartridges ───────────────────────────────────────────────────────
    cartridge_diameter: float = 325.0
    cartridge_length: float = 660.0
    # Grid capacity. `cartridge_count` is how many of those positions are
    # actually populated, filled in row-major order starting at the first row
    # (lowest row in the downflow build, most -Y row in the crossflow build).
    cartridge_rows: int = 2
    cartridge_columns: int = 4
    cartridge_count: int = 8
    # Column-to-column spacing, along X in both geometries.
    cartridge_pitch_columns: float = 400.0
    # Row-to-row spacing: along Z (downflow), along Y (crossflow).
    cartridge_pitch_rows: float = 420.0
    # Downflow only: centreline elevation of the first (lowest) cartridge row
    # above the housing floor.
    downflow_bank_offset: float = 700.0
    # Filtration media area per cartridge. Carried as a number, NOT as pleat
    # geometry — see the module docstring.
    cartridge_media_area_m2: float = 22.0

    # ── dirty-air inlet ──────────────────────────────────────────────────
    # A rectangular duct leaving the -X wall in both geometries.
    inlet_width: float = 600.0  # along Y
    inlet_height: float = 600.0  # along Z
    inlet_duct_length: float = 250.0  # protrusion along -X
    inlet_wall_thickness: float = 5.0
    # Elevation of the inlet opening's LOWER edge, in global Z. The two
    # geometries are defined by where this sits relative to the media, so the
    # two values are independent parameters, not one value reused:
    #   downflow  — above every cartridge (dirty air enters over the media)
    #   crossflow — below every cartridge (dirty air enters under the media)
    inlet_elevation: float = 2450.0  # downflow (Geometry A)
    crossflow_inlet_elevation: float = 1300.0  # crossflow (Geometry B)

    # ── clean-air outlet ─────────────────────────────────────────────────
    # A round duct leaving the +X wall at the clean plenum, in both geometries.
    outlet_diameter: float = 400.0
    outlet_duct_length: float = 250.0
    outlet_wall_thickness: float = 5.0
    # Outlet centreline measured down from the housing top.
    outlet_top_clearance: float = 250.0

    # ── boolean hygiene ──────────────────────────────────────────────────
    # Overshoot applied to cutting tools so a subtraction never leaves a
    # zero-thickness coincident face. Not a dimension of the machine.
    cut_epsilon: float = 1.0

    # ── validation ───────────────────────────────────────────────────────

    def __post_init__(self) -> None:
        if self.cartridge_rows < 1 or self.cartridge_columns < 1:
            raise ValueError("cartridge_rows and cartridge_columns must be >= 1")
        capacity = self.cartridge_rows * self.cartridge_columns
        if not 1 <= self.cartridge_count <= capacity:
            raise ValueError(
                f"cartridge_count must be between 1 and "
                f"cartridge_rows * cartridge_columns ({capacity}); "
                f"got {self.cartridge_count}"
            )
        if self.cartridge_pitch_columns <= self.cartridge_diameter:
            raise ValueError("cartridge_pitch_columns must exceed cartridge_diameter")
        if self.cartridge_pitch_rows <= self.cartridge_diameter:
            raise ValueError("cartridge_pitch_rows must exceed cartridge_diameter")
        if self.hopper_outlet_size <= 2 * self.hopper_wall_thickness:
            raise ValueError("hopper_outlet_size must exceed twice the hopper wall")
        if self.clean_plenum_height >= self.housing_height:
            raise ValueError("clean_plenum_height must be less than housing_height")
        if self.clean_plenum_depth >= self.housing_depth:
            raise ValueError("clean_plenum_depth must be less than housing_depth")
        if self.outlet_top_clearance + self.outlet_diameter / 2 > self.clean_plenum_height:
            raise ValueError(
                "the outlet must sit wholly inside the crossflow clean plenum: "
                "outlet_top_clearance + outlet_diameter/2 <= clean_plenum_height"
            )
        if self.inlet_elevation + self.inlet_height > self.housing_top_z:
            raise ValueError("the downflow inlet opening runs past the housing top")
        if self.crossflow_inlet_elevation < self.housing_floor_z:
            raise ValueError("the crossflow inlet opening starts below the housing floor")

    # ── derived geometry (read-only; no new dimensions introduced) ───────

    @property
    def housing_floor_z(self) -> float:
        """Global Z where the hopper rim meets the housing."""
        return self.hopper_height

    @property
    def housing_top_z(self) -> float:
        """Global Z of the housing roof — the overall height of the machine."""
        return self.hopper_height + self.housing_height

    @property
    def total_height(self) -> float:
        return self.housing_height + self.hopper_height

    @property
    def cartridge_radius(self) -> float:
        return self.cartridge_diameter / 2.0

    @property
    def outlet_radius(self) -> float:
        return self.outlet_diameter / 2.0

    @property
    def outlet_elevation(self) -> float:
        """Global Z of the clean-air outlet centreline (both geometries)."""
        return self.housing_top_z - self.outlet_top_clearance

    # -- downflow (Geometry A) --------------------------------------------

    @property
    def downflow_tube_sheet_y(self) -> float:
        """Y of the vertical tube sheet's mid-plane."""
        return self.housing_depth / 2.0 - self.clean_plenum_depth

    @property
    def downflow_cartridge_center_y(self) -> float:
        """Y of a cartridge's mid-length: cantilevered off the dirty face."""
        return (
            self.downflow_tube_sheet_y
            - self.tube_sheet_thickness / 2.0
            - self.cartridge_length / 2.0
        )

    @property
    def downflow_dirty_center_y(self) -> float:
        """Y midpoint of the dirty-air side — where the inlet duct centres."""
        return (-self.housing_depth / 2.0 + self.downflow_tube_sheet_y) / 2.0

    @property
    def downflow_outlet_center_y(self) -> float:
        """Y of the outlet centreline: mid-depth of the clean plenum."""
        return self.housing_depth / 2.0 - self.clean_plenum_depth / 2.0

    # -- crossflow (Geometry B) -------------------------------------------

    @property
    def crossflow_tube_sheet_z(self) -> float:
        """Global Z of the horizontal tube sheet's mid-plane."""
        return self.housing_top_z - self.clean_plenum_height

    @property
    def crossflow_cartridge_center_z(self) -> float:
        """Global Z of a hanging cartridge's mid-length."""
        return (
            self.crossflow_tube_sheet_z
            - self.tube_sheet_thickness / 2.0
            - self.cartridge_length / 2.0
        )

    # -- expected extents (the contract the dimensional tests assert) ------

    def expected_bounds(self) -> tuple[tuple[float, float], ...]:
        """((xmin, xmax), (ymin, ymax), (zmin, zmax)) of either assembly.

        Derived purely from parameters. The two geometries share these
        extents: the inlet duct is the only thing reaching past -X, the
        outlet duct the only thing past +X, nothing protrudes in Y, and the
        machine stands from the hopper discharge face to the housing roof.
        """
        return (
            (
                -(self.housing_width / 2.0 + self.inlet_duct_length),
                self.housing_width / 2.0 + self.outlet_duct_length,
            ),
            (-self.housing_depth / 2.0, self.housing_depth / 2.0),
            (0.0, self.total_height),
        )


#: The parameter set the CLI builds. Change a number here, re-run
#: `python cad/build.py`, and every exported file changes with it.
PARAMS = CollectorParams()
