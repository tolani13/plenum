"""CAD-01 · Geometry B — crossflow, vertical cartridges.

The defining arrangement: the cartridges hang VERTICALLY from a horizontal
tube sheet, and the dirty-air inlet enters at the side BELOW all filtration
media. Dirty air crosses the face of the bank rather than descending onto it;
the clean plenum is the chamber above the tube sheet, and the clean-air
outlet leaves it through the +X wall.

Every dimension below arrives from `CollectorParams`.
"""

from __future__ import annotations

from build123d import Compound

from common import (
    cartridge_parts,
    horizontal_tube_sheet,
    housing_shell,
    hopper,
    inlet_duct,
    inlet_opening,
    outlet_duct,
    outlet_opening,
)
from params import CollectorParams

NAME = "crossflow"


def build(p: CollectorParams) -> Compound:
    """The Geometry B assembly, as a compound of labelled solids."""
    inlet_y = 0.0  # the bank is centred on the housing; so is its inlet
    inlet_z = p.crossflow_inlet_elevation
    outlet_y = 0.0
    outlet_z = p.outlet_elevation

    shell = housing_shell(p)
    shell = shell - inlet_opening(p, inlet_y, inlet_z)
    shell = shell - outlet_opening(p, outlet_y, outlet_z)
    shell.label = "housing"

    parts = [
        shell,
        hopper(p),
        horizontal_tube_sheet(p),
        inlet_duct(p, inlet_y, inlet_z),
        outlet_duct(p, outlet_y, outlet_z),
        *cartridge_parts(p, horizontal=False),
    ]

    assembly = Compound(children=parts)
    assembly.label = NAME
    return assembly
