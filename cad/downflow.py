"""CAD-01 · Geometry A — downflow, horizontal cartridges.

The defining arrangement: the dirty-air inlet enters ABOVE all filtration
media, so heavy particulate drops toward the hopper before it ever reaches a
cartridge, and the air that does reach the media travels downward with
gravity rather than against it. The cartridges cantilever horizontally off a
VERTICAL tube sheet; the clean plenum is the chamber behind that tube sheet,
at +Y, and the clean-air outlet leaves it through the +X wall.

Every dimension below arrives from `CollectorParams`.
"""

from __future__ import annotations

from build123d import Compound

from common import (
    cartridge_parts,
    housing_shell,
    hopper,
    inlet_duct,
    inlet_opening,
    outlet_duct,
    outlet_opening,
    vertical_tube_sheet,
)
from params import CollectorParams

NAME = "downflow"


def build(p: CollectorParams) -> Compound:
    """The Geometry A assembly, as a compound of labelled solids."""
    inlet_y = p.downflow_dirty_center_y
    inlet_z = p.inlet_elevation
    outlet_y = p.downflow_outlet_center_y
    outlet_z = p.outlet_elevation

    shell = housing_shell(p)
    shell = shell - inlet_opening(p, inlet_y, inlet_z)
    shell = shell - outlet_opening(p, outlet_y, outlet_z)
    shell.label = "housing"

    parts = [
        shell,
        hopper(p),
        vertical_tube_sheet(p),
        inlet_duct(p, inlet_y, inlet_z),
        outlet_duct(p, outlet_y, outlet_z),
        *cartridge_parts(p, horizontal=True),
    ]

    assembly = Compound(children=parts)
    assembly.label = NAME
    return assembly
