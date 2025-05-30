import cadquery as cq
from util import INCH, AXIS_X, AXIS_Z


# model for a 30x30x10mm blower fan
# https://www.amazon.com/gp/product/B08MKLDGQR
class BlowerFan:

    def __init__(self):
        self.h = 10.4
        self.size = 30
        self.hole_r = 2.5 / 2
        self.hole_off_lat = self.size / 2 - 1.75 - self.hole_r
        self.inlet_r = 13

        self.bbox = cq.Workplane("XY").rect(self.size, self.size).extrude(self.h)

        # main box shape
        fan = self.bbox

        # cutout in the middle showing where the fan rotor goes
        fan = fan.faces(">Z").circle(self.inlet_r).cutBlind(-self.h + 1)
        # add the main fan rotor bit
        fan = fan.faces(">Z").circle(8.5).extrude(-self.h)

        # airflow outlet cutout
        cutout_h = 8.3
        cutout_w = 21
        cutout_y_offset = 2.5
        cutout = (
            cq.Workplane("YZ")
            .rect(cutout_w, cutout_h)
            .extrude(30)
            .translate(
                (9, -self.size / 2 + cutout_w / 2 + cutout_y_offset, cutout_h / 2 + 1)
            )
        )
        fan = fan.cut(cutout)

        # corner cutouts for screw placement
        cutout_z_offset = 2.8
        corner_cutout_sz = 5.1
        cutout_lat_offset = self.size / 2 - corner_cutout_sz / 2
        corner_cutout = (
            cq.Workplane("XY").rect(corner_cutout_sz, corner_cutout_sz).extrude(30)
        )
        fan = fan.cut(
            corner_cutout.translate(
                (cutout_lat_offset, cutout_lat_offset, cutout_z_offset)
            )
        )
        fan = fan.cut(
            corner_cutout.translate(
                (-cutout_lat_offset, -cutout_lat_offset, cutout_z_offset)
            )
        )

        # mounting holes
        self.hole_locs = [
            (self.hole_off_lat, self.hole_off_lat),
            (-self.hole_off_lat, -self.hole_off_lat),
        ]
        fan = (
            fan.faces(">Z").pushPoints(self.hole_locs).circle(self.hole_r).cutThruAll()
        )

        self.shape = fan


class BlowerFanCutOut:

    def __init__(
        self,
        bf,
        tol=0.2,
        inlet_cutout_ht=20,
        wire_slot_cutout_ht=20,
        mount_screw_hole_depth=5,
        corner_cutouts=False,
    ):
        self.bf = bf

        # main exterior shape, increased by `tol` in each direction
        self.h = bf.h + tol
        cutout = (
            cq.Workplane("XY")
            .rect(bf.size + 2 * tol, bf.size + 2 * tol)
            .extrude(self.h)
        )

        # cut out slot for wire
        # TODO: this could be a lot better
        slot_x_dim = 12
        slot_y_dim = 3
        wireslot = (
            cq.Workplane("XY")
            .moveTo(bf.size / 2 + tol - slot_x_dim, bf.size / 2 + tol - slot_y_dim)
            .rect(slot_x_dim, slot_y_dim, centered=False)
            .extrude(bf.h + wire_slot_cutout_ht)
        )
        cutout = cutout.union(wireslot)

        # cutouts for mounting screws
        # TODO: should screw holes be smaller?
        if mount_screw_hole_depth > 0:
            cutout = (
                cutout.pushPoints(bf.hole_locs)
                .circle(bf.hole_r)
                .extrude(-mount_screw_hole_depth)
            )

        # TODO: simplify this - don't duplicate it from the main fan model

        # corner cutouts for screw placement
        if corner_cutouts:
            cutout_z_offset = 2.8 + tol
            corner_cutout_sz = 5.1 - tol
            cutout_lat_offset = (bf.size + 2 * tol) / 2 - corner_cutout_sz / 2
            corner_cutout = (
                cq.Workplane("XY").rect(corner_cutout_sz, corner_cutout_sz).extrude(30)
            )
            cutout = cutout.cut(
                corner_cutout.translate(
                    (cutout_lat_offset, cutout_lat_offset, cutout_z_offset)
                )
            )
            cutout = cutout.cut(
                corner_cutout.translate(
                    (-cutout_lat_offset, -cutout_lat_offset, cutout_z_offset)
                )
            )

        # TODO: air outlet cutout?

        self.shape = cutout


def fan_inlet_path(bf, h=5):
    ip = cq.Workplane("XY").circle(bf.inlet_r).extrude(h)
    ip = ip.union(
        cq.Workplane("XZ")
        .rect(bf.inlet_r * 2, h, centered=False)
        .extrude(-50)
        .translate((-bf.inlet_r, 0, 0))
    )
    ip = ip.edges(">Z").fillet(2)
    ip = ip.translate((0, 0, bf.h))
    ip = ip.rotate_z(-90)
    return ip


# same as fan_inlet_path(), but the end tapers out to a rectangle rather than a rect with two corners rounded.
# TODO: refactor. this works, but the code is awful
def fan_inlet_path2(bf, h=5):
    ip = cq.Workplane("XY").circle(bf.inlet_r).extrude(h)
    flow_len = 50
    flow_curved_segment = bf.inlet_r - 5
    flow_transition_segment = 15
    ip = ip.union(
        cq.Workplane("XZ")
        .rect(bf.inlet_r * 2, h, centered=False)
        .extrude(-flow_curved_segment)
        .translate((-bf.inlet_r, 0, 0))
    )
    ip = ip.edges(">Z and |Y").fillet(2)

    # transition from rounded to a straight corner rect
    ip = ip.union(
        ip.faces(">Y")
        .wires()
        .toPending()
        .workplane()
        .transformed(offset=(-bf.inlet_r, 0, flow_transition_segment))
        .rect(bf.inlet_r * 2, h, centered=False)
        .loft()
    )

    # straight rectangle segment
    ip = ip.union(
        ip.faces(">Y")
        .wires()
        .toPending()
        .workplane()
        .transformed(
            offset=(
                -bf.inlet_r,
                0,
                flow_len - flow_transition_segment - flow_curved_segment,
            )
        )
        .rect(bf.inlet_r * 2, h, centered=False)
        .loft()
    )

    ip = ip.translate((0, 0, bf.h))
    ip = ip.rotate_z(-90)
    return ip


def fan_outlet_path(bf):
    # TODO: pull these params from bf, don't redefine here
    cutout_h = 8.3
    cutout_w = 21
    cutout_y_offset = 2.5
    spacing = -0.2
    return (
        cq.Workplane("YZ")
        .rect(cutout_w, cutout_h, centered=False)
        .extrude(30)
        .translate(
            (
                bf.size / 2 + spacing,
                -bf.size / 2 + cutout_y_offset,
                (bf.h - cutout_h) / 2,
            )
        )
    )


if __name__ == "__cq_main__":
    bf = BlowerFan().shape
    co = BlowerFanCutOut(BlowerFan()).shape.translate([40, 0, 0])
