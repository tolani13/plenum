"""CAD-01 · Shared solids — the pieces both cartridge orientations are made of.

Nothing in this module holds a dimension of its own. Every number a solid is
built from arrives on the `CollectorParams` instance passed in (unit
constraint 9); the only literals here are geometric constants (halves, the
90-degree rotations that lay a cylinder on its side) and grid indices.

Each returned part carries a `label`. The assemblies are `Compound`s of
labelled leaves, which is what lets the dimensional tests measure the built
solid — the tube sheet, the inlet duct, cartridge 5 — rather than re-deriving
the arithmetic they are supposed to be checking.
"""

from __future__ import annotations

from build123d import (
    Box,
    Cylinder,
    Part,
    Plane,
    Pos,
    Rectangle,
    Rot,
    loft,
)

from params import CollectorParams

# Labels the assemblies and the tests agree on.
LABEL_HOUSING = "housing"
LABEL_HOPPER = "hopper"
LABEL_TUBE_SHEET = "tube_sheet"
LABEL_INLET = "inlet_duct"
LABEL_OUTLET = "outlet_duct"
LABEL_CARTRIDGE_PREFIX = "cartridge_"


# ── cartridge grid ───────────────────────────────────────────────────────


def cartridge_slots(p: CollectorParams) -> list[tuple[int, int]]:
    """The (row, column) indices actually populated, filled row-major.

    Capacity is `cartridge_rows * cartridge_columns`; `cartridge_count` says
    how many of those positions carry a cartridge. Halving the count empties
    the last rows and leaves a physically sensible partial bank.
    """
    slots: list[tuple[int, int]] = []
    for row in range(p.cartridge_rows):
        for column in range(p.cartridge_columns):
            if len(slots) < p.cartridge_count:
                slots.append((row, column))
    return slots


def centered_offsets(n: int, pitch: float) -> list[float]:
    """`n` offsets at `pitch` spacing, symmetric about zero."""
    return [(i - (n - 1) / 2.0) * pitch for i in range(n)]


def column_x(p: CollectorParams, column: int) -> float:
    """X centreline of a grid column — the same in both geometries."""
    return centered_offsets(p.cartridge_columns, p.cartridge_pitch_columns)[column]


# ── housing and hopper ───────────────────────────────────────────────────


def housing_shell(p: CollectorParams) -> Part:
    """Four walls plus the roof, standing on the hopper rim.

    With `housing_cutaway` set, the -Y wall is omitted so the cartridge bank
    is visible in a mesh viewer. The roof and the hopper rim still span the
    full depth, so the omission does not change the assembly's extents.
    """
    eps = p.cut_epsilon
    wall = p.housing_wall_thickness
    center_z = p.housing_floor_z + p.housing_height / 2.0

    outer = Pos(0, 0, center_z) * Box(p.housing_width, p.housing_depth, p.housing_height)
    cavity = Pos(0, 0, center_z) * Box(
        p.housing_width - 2 * wall,
        p.housing_depth - 2 * wall,
        p.housing_height + 2 * eps,
    )
    roof = Pos(0, 0, p.housing_top_z - wall / 2.0) * Box(
        p.housing_width, p.housing_depth, wall
    )
    shell = (outer - cavity) + roof

    if p.housing_cutaway:
        cut_height = p.housing_height - wall
        shell = shell - Pos(
            0,
            -p.housing_depth / 2.0 + wall / 2.0,
            p.housing_floor_z + cut_height / 2.0,
        ) * Box(p.housing_width + 2 * eps, wall + 2 * eps, cut_height)

    shell.label = LABEL_HOUSING
    return shell


def hopper(p: CollectorParams) -> Part:
    """Pyramidal hopper: housing footprint at the rim, square outlet at z = 0."""
    eps = p.cut_epsilon
    wall = p.hopper_wall_thickness

    outer = loft(
        [
            Plane.XY * Rectangle(p.hopper_outlet_size, p.hopper_outlet_size),
            Plane.XY.offset(p.hopper_height)
            * Rectangle(p.housing_width, p.housing_depth),
        ]
    )
    cavity = loft(
        [
            Plane.XY.offset(-eps)
            * Rectangle(
                p.hopper_outlet_size - 2 * wall, p.hopper_outlet_size - 2 * wall
            ),
            Plane.XY.offset(p.hopper_height + eps)
            * Rectangle(p.housing_width - 2 * wall, p.housing_depth - 2 * wall),
        ]
    )
    part = outer - cavity
    part.label = LABEL_HOPPER
    return part


# ── ducts ────────────────────────────────────────────────────────────────


def inlet_duct(p: CollectorParams, center_y: float, lower_z: float) -> Part:
    """Rectangular dirty-air duct leaving the -X wall.

    `lower_z` is the elevation of the opening's lower edge — the dimension
    that distinguishes the two geometries.
    """
    eps = p.cut_epsilon
    skin = p.inlet_wall_thickness
    center_x = -(p.housing_width / 2.0 + p.inlet_duct_length / 2.0)
    center_z = lower_z + p.inlet_height / 2.0
    at = Pos(center_x, center_y, center_z)

    outer = at * Box(
        p.inlet_duct_length, p.inlet_width + 2 * skin, p.inlet_height + 2 * skin
    )
    bore = at * Box(p.inlet_duct_length + 2 * eps, p.inlet_width, p.inlet_height)
    part = outer - bore
    part.label = LABEL_INLET
    return part


def inlet_opening(p: CollectorParams, center_y: float, lower_z: float) -> Part:
    """The cutting tool that opens the -X wall behind the inlet duct."""
    eps = p.cut_epsilon
    wall = p.housing_wall_thickness
    return Pos(
        -p.housing_width / 2.0 + wall / 2.0,
        center_y,
        lower_z + p.inlet_height / 2.0,
    ) * Box(wall + 2 * eps, p.inlet_width, p.inlet_height)


def outlet_duct(p: CollectorParams, center_y: float, center_z: float) -> Part:
    """Round clean-air duct leaving the +X wall at the clean plenum."""
    eps = p.cut_epsilon
    skin = p.outlet_wall_thickness
    at = Pos(p.housing_width / 2.0 + p.outlet_duct_length / 2.0, center_y, center_z) * Rot(
        0, 90, 0
    )

    outer = at * Cylinder(p.outlet_radius + skin, p.outlet_duct_length)
    bore = at * Cylinder(p.outlet_radius, p.outlet_duct_length + 2 * eps)
    part = outer - bore
    part.label = LABEL_OUTLET
    return part


def outlet_opening(p: CollectorParams, center_y: float, center_z: float) -> Part:
    """The cutting tool that opens the +X wall behind the outlet duct."""
    eps = p.cut_epsilon
    wall = p.housing_wall_thickness
    return Pos(
        p.housing_width / 2.0 - wall / 2.0, center_y, center_z
    ) * Rot(0, 90, 0) * Cylinder(p.outlet_radius, wall + 2 * eps)


# ── tube sheet and cartridges ────────────────────────────────────────────


def vertical_tube_sheet(p: CollectorParams) -> Part:
    """Downflow: a vertical plate (normal = Y), bored for horizontal cartridges."""
    eps = p.cut_epsilon
    wall = p.housing_wall_thickness
    height = p.housing_height - wall  # stops under the roof
    plate = Pos(0, p.downflow_tube_sheet_y, p.housing_floor_z + height / 2.0) * Box(
        p.housing_width - 2 * wall, p.tube_sheet_thickness, height
    )
    for row, column in cartridge_slots(p):
        plate = plate - Pos(
            column_x(p, column),
            p.downflow_tube_sheet_y,
            downflow_row_z(p, row),
        ) * Rot(90, 0, 0) * Cylinder(
            p.cartridge_radius, p.tube_sheet_thickness + 2 * eps
        )
    plate.label = LABEL_TUBE_SHEET
    return plate


def horizontal_tube_sheet(p: CollectorParams) -> Part:
    """Crossflow: a horizontal plate (normal = Z), bored for hanging cartridges."""
    eps = p.cut_epsilon
    wall = p.housing_wall_thickness
    plate = Pos(0, 0, p.crossflow_tube_sheet_z) * Box(
        p.housing_width - 2 * wall,
        p.housing_depth - 2 * wall,
        p.tube_sheet_thickness,
    )
    for row, column in cartridge_slots(p):
        plate = plate - Pos(
            column_x(p, column), crossflow_row_y(p, row), p.crossflow_tube_sheet_z
        ) * Cylinder(p.cartridge_radius, p.tube_sheet_thickness + 2 * eps)
    plate.label = LABEL_TUBE_SHEET
    return plate


def downflow_row_z(p: CollectorParams, row: int) -> float:
    """Global Z of a downflow cartridge row's centreline."""
    return p.housing_floor_z + p.downflow_bank_offset + row * p.cartridge_pitch_rows


def crossflow_row_y(p: CollectorParams, row: int) -> float:
    """Y of a crossflow cartridge row's centreline."""
    return centered_offsets(p.cartridge_rows, p.cartridge_pitch_rows)[row]


def cartridge(p: CollectorParams, index: int, position: Pos, horizontal: bool) -> Part:
    """One cartridge: a plain cylinder. Pleats are deliberately not modelled.

    `horizontal` lays the axis along Y (downflow, cantilevered off a vertical
    tube sheet); otherwise the axis is Z (crossflow, hung from a horizontal
    tube sheet).
    """
    body = Cylinder(p.cartridge_radius, p.cartridge_length)
    part = position * (Rot(90, 0, 0) * body if horizontal else body)
    part.label = f"{LABEL_CARTRIDGE_PREFIX}{index:02d}"
    return part


def cartridge_parts(p: CollectorParams, horizontal: bool) -> list[Part]:
    """Every populated cartridge, in slot order."""
    parts: list[Part] = []
    for index, (row, column) in enumerate(cartridge_slots(p)):
        if horizontal:
            at = Pos(
                column_x(p, column),
                p.downflow_cartridge_center_y,
                downflow_row_z(p, row),
            )
        else:
            at = Pos(
                column_x(p, column),
                crossflow_row_y(p, row),
                p.crossflow_cartridge_center_z,
            )
        parts.append(cartridge(p, index, at, horizontal))
    return parts
