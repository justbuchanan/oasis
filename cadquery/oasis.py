# 3D model design for Oasis terrarium.
#
# This model is designed using CadQuery - see README.md for more info.
#
# Before diving into the code below, it may be helpful to check out the 3d model
# view on the website to visually see how things come together:
# https://oasis-terrarium.com/docs/3dmodel/
#
# This code is in need of some major refactoring. I'm very happy with the
# 3d-printable models that this code generates, but the code itself has grown
# into a sprawling mess.
#
# This file defines a number of different parts, test pieces, and
# sub-assemblies. At the very bottom of the file, we instantiate something to
# show. By default, this is the full terrarium assembly, but this can be
# changed to any of the individual parts you want to explore. Note that the
# full assembly can take a long time to render.
#
#
# NOTE: most individual parts are represented as python classes. All of them set
# the `self.shape` property during initialization (i.e. the `__init__
# ()` method), which is the actual resultant cadquery object. The benefit of
# using python objects/classes is that the params are accessible as properties
# of each object.
#
# Example:
#     top = TopProfile()
#     print(f"top h: {top.h}, wall_th: {top.wall_th}, outer_r: {top.outer_r}")
#     show_object(top.shape)
#
#
# NOTE: a lot of the code below uses the following abbreviations:
#
# - l=length
# - w=width
# - h=height
# - th=thickness
# - r=radius
#
#
# ------------ TODO -----------
# - fitment of oring in TopPlugInlet is ok, but could be better.
# - consider moving the side vent holes down lower to increase ventilation
# - make it impossible to attach the underplate in the wrong direction. Right now it's possible to do it right and to do it 180deg off from right.
# - similar to the vertical wall/ring around the entire underplate, consider doing the same around the inside of the entire top.
#
# - add text labels for:
#   - micro usb
#   - 18v dc

import cadquery as cq
import math
from util import (
    remove_neg_y,
    remove_pos_y,
    remove_pos_x,
    INCH,
    AXIS_X,
    AXIS_Y,
    AXIS_Z,
)
import sys
import os
from blower_fan import (
    BlowerFan,
    BlowerFanCutOut,
    fan_inlet_path,
    fan_outlet_path,
)
from math import degrees, radians
from honeycomb import gen_honeycombed_circle, gen_tiled_hexagon_locations

from cq_warehouse.thread import IsoThread

# add parent dir to path so we can import oasis_constants
try:
    sys.path.append(os.path.join(os.path.dirname(__file__), os.pardir))
except:
    print(
        "oasis.py failed to add parent directory to the path. May fail to load oasis_constants.py"
    )
# oasis_constants contains values shared between the cad model and the circuit board.
from oasis_constants import (
    ledboard_inner_r,
    ledboard_outer_r,
    led_count,
    led0_angle_deg,
    hole0_angle_deg,
    mainboard_usb_port_angle_deg,
)

# Color definitions for the total assembly view
glass_color = cq.Color(1, 1, 1, 0.2)
light_color = cq.Color(1, 0.9, 0.9, 0.2)
print_color = cq.Color("blue")
ledboard_color = cq.Color("white")
mainboard_color = cq.Color(0, 0.55, 0.29)

# LED board relevant dimensions
ledboard_th = 1.6
ledboard_center_r = (ledboard_outer_r + ledboard_inner_r) / 2
# rotate the ledboard a bit so that the lights are out of the way of where we
# want the humidity sensor to go.
# TODO: calculate this from the values in oasis_constants.py and the location of the sht30 sensor.
ledboard_rotation_angle = 360 / 5 * 0.5 + 5

# screw holes in the underplate should be wide enough for the M3 screws to easily
# slide through.
m3_passthrough_hole_r = 3.15 / 2
# holes in the mount pieces of the Top() are slightly smaller than an M3 so the
# screws can self-tap into the plastic.
m3_selftap_hole_r = 2.75 / 2

# Mainboard relevant parameters
# TODO: some of this should move to oasis_constants.py
mainboard_th = 1.6
mainboard_w = 65
mainboard_button_offset_from_edge = 15.08
mainboard_hole_offset_from_edge = 2.96
mainboard_arc_to_back_dist = 35.96
mainboard_barreljack_overhang = 0.9
mainboard_barreljack_h = 10.95
mainboard_barreljack_w = 8.95

# distance from the bottom-most piece of the Top to the bottom-most bit of the
# underplate. The underplate should sit a bit higher so that if the whole top
# assembly is set down on a table, the underplate doesn't touch the table
# surface.
underplate_top_bottom_offset = 3

# 2 "mating plugs" that connect between Underplate and Top. Underplate has pegs,
# Top has holes.
top_mate_plug_r = 4
top_mate_plug_depth = 7
top_mate_plug_x_offset = 83
top_mate_plug_angle_offset = 20  # degrees clockwise from the +x axis

# specs for the two screws that hold the underplate to the top
underplate_screw_x_offset = 83
underplate_screw_angle_offset = -22  # degrees clockwise from the +x axis


# assuming we're using a 12"x24" acrylic sheet to make the tube
def calculate_outer_r(wall_th, tube_gap):
    acryl_sheet_len = 24 * INCH
    # when bending the flat sheet into a tube, this is how much the sheet
    # overlaps itself.
    acryl_tube_overlap = 10
    # using C = 2*pi*r, we get r = C / (2*pi)
    acryl_tube_r = (acryl_sheet_len - acryl_tube_overlap) / (2 * math.pi)
    # the radius of the tube should fall in the center of the tube gap (hence
    # tube_gap/2).
    outer_r = acryl_tube_r + tube_gap / 2 + wall_th
    return outer_r


# 2d profile, which we later revolve to create a 3d shape. Used for both Bottom and Top.
class CapProfile:

    def __init__(self, tube_offset, h):
        self.tube_offset = tube_offset
        self.tube_gap = 4
        self.wall_th = 3
        self.outer_r = calculate_outer_r(wall_th=self.wall_th, tube_gap=self.tube_gap)
        # prusa i3 mk3 build area is 210mm x 210mm, so max printable radius is
        # 105mm.
        assert self.outer_r < 104, "Too big to print on a prusa i3"
        self.inner_r = self.outer_r - self.wall_th * 2 - self.tube_gap
        self.h = h
        self.th = 4
        # indent slants edges for easier insertion of acrylic tube
        indent = 1.2
        self.shape = (
            cq.Workplane("XZ")
            .hLineTo(self.outer_r)
            .vLineTo(h)
            .hLine(-self.wall_th + indent)
            .line(-indent, -indent)
            .vLineTo(self.tube_offset)
            .hLine(-self.tube_gap)
            .vLineTo(h - indent)
            .line(-indent, indent)
            .hLine(-self.wall_th + indent)
            .vLineTo(self.th)
            .hLineTo(0)
            .close()
        )


class TopProfile(CapProfile):

    def __init__(self):
        super().__init__(tube_offset=31, h=53)


class BottomProfile(CapProfile):

    def __init__(self):
        super().__init__(tube_offset=4, h=42)


# `Tube` is the circular wall of the terrarium made out of a sheet of acrylic.
class Tube:

    def __init__(self, bottom_profile):
        self.h = 12 * INCH
        # note that part of the tube is 2x+ this thickness because of the
        # overlap where it is glued together.
        self.th = 1 / 32 * INCH
        self.middle_r = (
            bottom_profile.outer_r
            - bottom_profile.wall_th
            - bottom_profile.tube_gap / 2
        )
        self.outer_r = self.middle_r + self.th / 2

        self.acrylic_sheet_glueing_overlap = 10
        self.acrylic_sheet_length = (
            math.pi * 2 * self.middle_r + self.acrylic_sheet_glueing_overlap
        )
        assert (
            self.acrylic_sheet_length <= 24 * INCH
        ), "Required acrylic sheet length is greater than 24 inches"

        self.shape = (
            cq.Workplane("XY")
            .circle(self.outer_r)
            .circle(self.outer_r - self.th)
            .extrude(self.h)
        )


assert ledboard_outer_r < TopProfile().inner_r


# Rough model of sht30 temperature/humidity sensor.
# see page 17 of https://mm.digikey.com/Volume0/opasdata/d220001/medias/docus/1067/HT_DS_SHT3x_DIS.pdf
def sht30():
    return (
        cq.Workplane("XY")
        .rect(2.4, 2.4, centered=True)
        .extrude(0.9)
        .faces(">Z")
        .workplane()
        .hole(1.5, 0.2)
    )


def header_pin():
    # metal pin
    p = cq.Workplane("XY").rect(0.62, 0.62).extrude(11.23).translate((0, 0, -3))
    # black plastic box
    b = cq.Workplane("XY").rect(2.44, 2.44).extrude(2.5)
    return p.union(b)


# Rough model of https://www.amazon.com/dp/B0CN32WXJY.
# No datasheet found - I measured all the dimensions so this is an approximation
class Sht30Board:

    def __init__(self):
        self.size = (12.66, 10.56, 1.6)
        self.outline = cq.Workplane("XY").rounded_rect(self.size[0], self.size[1], 1)
        self.hole_loc = (
            -self.size[0] / 2 + 1 + m3_passthrough_hole_r,
            self.size[1] / 2 - 1.1 - m3_passthrough_hole_r,
        )
        self.th = 1.6

        # these numbers are approximations
        pin_hole_r = 0.68
        self.pin_hole_locs = [
            # left
            (
                -self.size[0] / 2 + 0.65 + pin_hole_r,
                self.size[1] / 2 - 6.05 - pin_hole_r,
            ),
            (-self.size[0] / 2 + 0.65 + pin_hole_r, -self.size[1] / 2 + 1 + pin_hole_r),
            # right
            (self.size[0] / 2 - 0.65 - pin_hole_r, self.size[1] / 2 - 0.8 - pin_hole_r),
            (
                self.size[0] / 2 - 0.65 - pin_hole_r,
                self.size[1] / 2 - 0.8 - pin_hole_r - 2.5,
            ),
            (
                self.size[0] / 2 - 0.65 - pin_hole_r,
                self.size[1] / 2 - 0.8 - pin_hole_r - 2.5 * 2,
            ),
            (
                self.size[0] / 2 - 0.65 - pin_hole_r,
                self.size[1] / 2 - 0.8 - pin_hole_r - 2.5 * 3,
            ),
        ]

        sb = self.outline.extrude(self.size[2])
        sb = (
            sb.faces(">Z")
            .workplane()
            .moveTo(self.hole_loc[0], self.hole_loc[1])
            .hole(m3_passthrough_hole_r * 2)
        )

        # header pin holes
        sb = (
            sb.faces(">Z")
            .workplane()
            .pushPoints(self.pin_hole_locs)
            .hole(pin_hole_r * 2)
        )

        # header pins on right
        for i in range(4):
            hole_loc = self.pin_hole_locs[2 + i]
            sb = sb.add(
                header_pin().translate((hole_loc[0], hole_loc[1], self.size[2]))
            )

        # add sht30 chip
        self.sht30_loc = (
            -self.size[0] / 2 + 2.7 + 2.4 / 2,
            -self.size[1] / 2 + 1.85 + 2.4 / 2,
        )
        sb = sb.union(sht30().rotate_x(180).translate(self.sht30_loc))

        self.shape = sb


class MisterDisc:

    def __init__(self):
        th = 0.7
        r = 8
        # main disc
        disc = cq.Workplane("XY").circle(r).extrude(th)
        # rough stand-in for connecting wires
        disc = disc.union(cq.Workplane("XY").moveTo(r, 0).rect(4, 2.5).extrude(th * 2))
        # outdent on the bottom of the disc
        disc = disc.union(cq.Workplane("XY").circle(2).extrude(-0.1))
        self.shape = disc


# Model of the LED we're using.
# https://downloads.cree-led.com/files/ds/x/XLamp-XTE.pdf
class CreeXTE:

    def __init__(self):
        sz = 3.45
        rad = 1.53
        h = 0.83
        led = cq.Workplane("XY").rect(sz, sz, centered=True).extrude(h)
        lens = cq.Workplane().sphere(rad)
        lens = lens.cut(cq.Solid.makeBox(10, 10, 10, pnt=(-5, -5, -10)))
        lens = lens.translate((0, 0, h))
        led = led.union(lens)
        self.shape = led


# Carclo 20mm LED optic for cree X-Lamp LEDs
#
# might try a couple different options, but leaning towards carclo 10140. carclo
# 20mm optics are all the same size/shape (AFAIK?), but differ in how wide the
# output spot of light is, whether they're frosted or not, etc.
#
# Note: this is an approximation, not an exact model. See datasheet:
# https://www.carclo-optics.com/sites/default/files/images/optics/10140_iss2_230508.pdf
class LedOptic:

    # rect_tab: some optic models have a rectangular bit that sticks out at the bottom
    # cutout_tol: how much bigger the cutout template is (in terms of radius) than the actual optic.
    def __init__(self, rect_tab=True, cutout_tol=0.06, cutout_h=10):
        self.r = 19.6 / 2
        self.h = 9.9
        self.r_top = 3.1

        lo = cq.CQ(cq.Solid.makeCone(self.r, self.r_top, self.h))

        cutout = cq.Workplane("XY").circle(self.r + cutout_tol).extrude(cutout_h)

        if rect_tab:
            tab_w = 2.8
            tab_h = 3
            # note: this differs from the datasheet (which says 0.2), but
            # measurement and testing has shown this to be a reasonable value.
            tab_l = 1.2

            # make tab stick out on the +x side
            tab = (
                cq.Workplane("XY")
                .moveTo(0, -tab_w / 2)
                .rect(self.r + tab_l, tab_w, centered=False)
                .extrude(tab_h)
            )
            lo = lo.union(tab)

            tabcutout = (
                cq.Workplane("XY")
                .moveTo(0, -(tab_w + 2 * cutout_tol) / 2)
                .rect(
                    self.r + tab_l + cutout_tol, tab_w + 2 * cutout_tol, centered=False
                )
                .extrude(cutout_h)
            )
            cutout = cutout.union(tabcutout)

        self.shape = lo
        # cutout provides a 3d object for creating a cutout for the optic to
        # fit into.
        self.cutout = cutout


# An approximate model of the led board.
class LedboardSimple:

    def __init__(self):
        self.outer_r = ledboard_outer_r
        self.inner_r = ledboard_inner_r
        self.th = ledboard_th

        led_angle_spacing = 360 / led_count

        # main pcb shape
        pcb = (
            cq.Workplane("XY")
            .circle(self.outer_r)
            .circle(self.inner_r)
            .extrude(self.th)
        )

        # add leds
        led_obj = CreeXTE()
        for i in range(led_count):
            # flip upside down
            led = led_obj.shape.rotate_x(180)
            # position based on angle
            led = led.translate((ledboard_center_r, 0))
            theta = led0_angle_deg + i * led_angle_spacing
            led = led.rotate_z(theta)
            pcb = pcb.union(led)

        self.shape = pcb


# Loads the "real" ledboard from a STEP file generated by KiCad.
def kicad_ledboard():
    # TODO: do better - this is here because when using cq-cli, __file__ is not
    # set. see https://github.com/CadQuery/cq-cli/pull/37
    try:
        pardir = os.path.join(os.path.dirname(__file__), os.pardir)
    except:
        pardir = "./"

    path = os.path.join(pardir, "build/pcb/ledboard/ledboard.step")
    board = cq.importers.importStep(path)
    bbc = board.val().BoundingBox().center
    board = board.translate((-bbc.x, -bbc.y, 0))
    # flip it over so leds face down
    board = board.rotate_x(180)
    # shift it up by the board thickness so z=0 is the bottom of the board
    # (the side with leds)
    board = board.translate((0, 0, ledboard_th))
    # rotate about Z so the leds and such are where we expect
    board = board.rotate_z(-9.5)  # TODO
    return board


# Load the "real" ledboard if available, otherwise use the simple/approximate model.
def ledboard(prefer_kicad=True):
    # try to load the kicad-exported step pcb model. If it's not available, use the simple model.
    if prefer_kicad:
        try:
            return kicad_ledboard()
        except ValueError:
            print(
                "Unable to load pcb model exported from kicad; using simple cadquery model instead..."
            )
    return LedboardSimple().shape


# Load the "real" mainboard from a STEP file generated by KiCad
def kicad_mainboard():
    # TODO: do better - this is here because when using cq-cli, __file__ is not
    # set. see https://github.com/CadQuery/cq-cli/pull/37
    try:
        pardir = os.path.join(os.path.dirname(__file__), os.pardir)
    except:
        pardir = "./"
    fp = os.path.join(pardir, "build/pcb/main/pcb.step")
    p = cq.importers.importStep(fp)
    bbc = p.val().BoundingBox().center
    return p.translate((-bbc.x, -bbc.y, 0))


# Rough approximation of the mainboard.
class MainboardSimple:

    def __init__(self, tp: TopProfile):
        self.arc_r = tp.outer_r - tp.wall_th * 2 - tp.tube_gap
        self.w = 65
        self.offset = self.arc_r - 36

        # c1 and c2 are on a circle with radius arc_r. x values are w/2
        # and -w/2. Using circle equation of x^2 + y^2 = r^2 and plugging in
        # x=w/2 r=arc_r, we can solve for y.
        y = math.sqrt(self.arc_r**2 - self.w**2 / 4)

        self.side_len = y - self.offset

        # calculate the arc that defines one side of the board.
        c1 = (self.w / 2, y)
        c2 = (-self.w / 2, y)
        arcmid = (0, self.arc_r)
        c3 = (-self.w / 2, self.offset)
        c4 = (self.w / 2, self.offset)

        # print(f"c1: {c1}")
        # print(f"c2: {c2}")
        # print(f"c3: {c3}")
        # print(f"c4: {c4}")
        # print(f"arcmid: {arcmid}")

        b = (
            cq.Workplane("XY")
            .moveTo(*c1)
            .threePointArc(arcmid, c2)
            .lineTo(*c3)
            .lineTo(*c4)
            .close()
        )
        b = b.extrude(mainboard_th)
        # move and rotate so the center "back" of the board is at the origin and
        # the board front points towards +x
        b = b.translate((0, -self.offset, 0)).rotate_z(-90)

        self.shape = b


# Load the "real" mainboard if available, otherwise use the simple/approximate model.
def mainboard(prefer_kicad=True):
    if prefer_kicad:
        try:
            return kicad_mainboard()
        except ValueError:
            print(
                "Unable to load pcb model exported from kicad; using simple cadquery model instead..."
            )
    return MainboardSimple(TopProfile()).shape


class MisterMount:

    def __init__(self, th=4, hole_r=m3_selftap_hole_r, oring_cut=True):
        self.th = th
        self.hole_dist_from_center = 14
        self.center_hole_r = 10
        # main circular shape of mount
        p = cq.Workplane("XY").circle(12).extrude(self.th)

        # add screw mount points
        for i in range(3):
            m = (
                cq.Workplane("XY")
                .moveTo(self.hole_dist_from_center)
                .circle(4)
                .extrude(self.th)
                .rotate_z(360 / 3 * i)
            )
            # screw hole
            m = m.faces(">Z").hole(hole_r * 2)
            p = p.union(m)

        # blend screw mount points with main circular body
        p = p.edges("|Z").fillet(3)
        # center hole
        p = p.faces(">Z").hole(self.center_hole_r)
        # cut out "well"/slot for o ring
        if oring_cut:
            p = p.cut(oring12x3(tol=0.1).translate((0, 0, self.th)))

        self.shape = p


# Rough approximation of an M3 screw
def m3_screw(length):
    s = (
        cq.Workplane("XY")
        .circle(5.43 / 2)
        .extrude(2.95)
        .faces(">Z")
        .circle(2.92 / 2)
        .extrude(length)
    )
    s = s.cut(cq.Workplane("XY").polygon(6, 3).extrude(2.4))
    return s


# Small piece that holds the mister disc to the water tank. Has holes for three M3 screws.
def mister_mount_cover():
    mm = MisterMount(th=3.5, oring_cut=False, hole_r=m3_passthrough_hole_r)
    cover = mm.shape
    # cutout for mister wires
    cover = cover.cut(
        cq.Workplane("XY")
        .moveTo(0, -1.5)
        .rect(13, 3, centered=False)
        .extrude(1.7)
        .rotate_z(-60)
    )
    # cutout to "embed" disc a bit
    cover = cover.cut(cq.Workplane("XY").circle(8.1).extrude(0.7))
    return cover


# Assembly for visualizing the mister assembly. Not used in the final design.
def mister_mount_asm():
    mmth = 4

    asm = cq.Assembly()
    asm.add(MisterMount(th=mmth).shape, color=print_color, name="bottom")

    asm.add(
        oring12x3(),
        loc=cq.Location((0, 0, mmth + 0.5)),
        color=cq.Color("black"),
        name="oring",
    )

    fd = MisterDisc().shape.rotate_z(-60)
    asm.add(
        fd,
        loc=cq.Location((0, 0, mmth + 0.5 + 2)),
        color=cq.Color("white"),
        name="disc",
    )

    # screws
    screw = m3_screw(length=8).rotate_x(180)
    for i in range(3):
        asm.add(
            screw.translate((14, 0, 17)).rotate_z(i * 120),
            name="screw%d" % i,
            color=cq.Color("gray"),
        )

    asm.add(
        mister_mount_cover(), loc=cq.Location((0, 0, 10)), name="top", color=print_color
    )

    return asm


# Plug for the water tank in the top piece. This screws into the Top and
# prevents water from spilling if the Top or terrarium as a whole is moved.
class TopPlug:

    def __init__(self):
        self.knob_r = 13
        self.knob_h = 5
        self.thread_pitch = 2
        self.thread_major_diameter = 19
        self.shaft_l = 5
        self.end_slant_d = 3

        threads = IsoThread(
            major_diameter=self.thread_major_diameter,
            pitch=self.thread_pitch,
            length=self.shaft_l,
            external=True,
            end_finishes=("square", "fade"),
        )

        tp = (
            cq.Workplane("XZ")
            .hLine(self.knob_r)
            .vLine(self.knob_h)
            .hLineTo(threads.min_radius)
            .vLine(self.shaft_l)
            .line(-self.end_slant_d, self.end_slant_d)
            .hLineTo(0)
            .close()
            .revolve()
        )

        tp = tp.union(threads.cq_object.translate((0, 0, self.knob_h)))

        # knurling on knob to make it easier to turn with your hand
        angle_spacing = 10
        for i in range(int(360 / angle_spacing)):
            cutout = (
                cq.Workplane("XZ")
                .moveTo(self.knob_r - 0.5, 0)
                .rect(2, self.knob_h, centered=False)
                .revolve(angleDegrees=angle_spacing / 2)
                .rotate_z(i * angle_spacing)
            )
            tp = tp.cut(cutout)

        self.shape = tp


# Mates with the TopPlug. The inlet, although designed as a standalone piece
# here, is merged in as a part of Top.
class TopPlugInlet:

    def __init__(self, plug: TopPlug, test=False):
        self.h = plug.shaft_l + plug.end_slant_d + 1
        self.oring_z_off = 2.8
        self.bottom_hole_r = 7.5

        unthreaded_bottom_dist = 4.5
        # note: if the internal threads and external threads have the same major
        # diameter, they will *exactly* mesh. This means that we want
        # the "female"/internal threads to have a slightly larger diameter so
        # that they actually fit together in real life. The tolerance value
        # below was determined experimentally but could be increased or
        # decreased if the the threads are too tight or loose.
        thread_major_diameter = plug.thread_major_diameter + 0.4
        threads = IsoThread(
            major_diameter=thread_major_diameter,
            pitch=plug.thread_pitch,
            length=self.h - unthreaded_bottom_dist,
            external=False,
            end_finishes=("fade", "fade"),
        )

        self.bottom_outer_r = thread_major_diameter / 2 + 2
        self.top_outer_r = self.bottom_outer_r if test else self.bottom_outer_r + 7
        self.hole_r = thread_major_diameter / 2
        inl = (
            cq.Workplane("XZ")
            .moveTo(self.bottom_outer_r, 0)
            .lineTo(self.top_outer_r, self.h)
            .hLineTo(self.hole_r)
            .vLineTo(unthreaded_bottom_dist)
            .lineTo(self.bottom_hole_r, 1)
            .vLineTo(0)
            .close()
            .revolve()
        )
        inl = inl.union(threads.cq_object.translate((0, 0, unthreaded_bottom_dist)))
        oringcut = oring12x3(tol=0.1).translate((0, 0, self.oring_z_off))
        inl = inl.cut(oringcut)

        self.shape = inl


# Demo assembly to help visualize how the top plug and inlet fit together. Not
# used in the actual 3d-printed model.
def top_plug_demo_asm(explode=0, halve=True):

    def maybeHalve(obj):
        return remove_pos_y(obj) if halve else obj

    asm = cq.Assembly()
    plug = TopPlug()
    inlet = TopPlugInlet(plug)
    # TODO: why doesn't maybeHalve work on the plug? It works if i remove the
    # threads just a bit...
    asm.add(
        maybeHalve(plug.shape.rotate_x(180).rotate_z(50)),
        name="plug",
        loc=cq.Location((0, 0, inlet.h + 1 + plug.knob_h + explode)),
        color=print_color,
    )
    asm.add(maybeHalve(inlet.shape), name="inlet", color=print_color)
    asm.add(
        maybeHalve(oring12x3()),
        name="oring",
        color=cq.Color("black"),
        loc=cq.Location((0, 0, inlet.oring_z_off + explode * 2 / 3)),
    )
    return asm


# Bottom piece of the terrarium. All of the plants and hardscape sit in here.
# Has a slot for the side walls / acrylic tube to fit into.
class Bottom:

    def __init__(self, profile):
        self.profile = profile
        self.shape = profile.shape.revolve()


# Top piece of the terrarium, which contains most of the complexity. Features:
# - mounting point for the mainboard as well as holes for accessing the mainboard's power and usb plugs.
# - hole for accessing the hard reset button on the mainboard
# - mates to the "underplate" using pegs and screws.
# - vent holes to let heat from the ledboard escape
# - water tank
# - mister mount on the water tank
# - inlet for the top plug to fit into
# - slots for mounting the two fans
class Top:

    def __init__(self, profile, vent_holes=False):
        self.profile = profile

        self.fan_rotate_angle = 150
        # NOTE: fan_offset_x was 89 for the first print. it was too tight to the
        # edge, so making it smaller.
        # NOTE: was 85 (never printed) until 4/25. need to push the fan back out again.
        self.fan_offset_x = 87
        self.fan_offset_z = 22 + self.profile.th

        # how much the acrylic cylinder "slides into" the top, vertically
        self.tube_inset = self.profile.h - self.profile.tube_offset

        t = self.profile.shape.revolve()

        # tag face for later use
        t = t.faces("<Z").workplane().tag("backZ")

        # add hexagon vent holes through the outside/side wall/perimeter.
        # note: the hexagons are oriented "long-ways" vertical so that there is no flat overhang to print.
        # TODO: refactor this into its own function
        hexdiam = 8
        hex_spacing = 2.04
        if vent_holes:
            hex_placement_inner_r = self.profile.inner_r - 1
            hex_placement_outer_r = self.profile.outer_r + 1
            hex_l = hex_placement_outer_r - hex_placement_inner_r
            h = (
                cq.Workplane("XY")
                .polygon(
                    nSides=6,
                    diameter=hexdiam * hex_placement_inner_r / hex_placement_outer_r,
                )
                .workplane(offset=hex_l)
                .polygon(nSides=6, diameter=hexdiam)  # .extrude(5)
                .loft()
            )

            # note: holes should stay below self.profile.tube_offset=31

            outer_circumf = 2 * math.pi * hex_placement_outer_r
            for loc in gen_tiled_hexagon_locations(
                start=(13, 0),
                rows=71,
                cols=2,
                spacing=hex_spacing,
                hex_diameter=hexdiam,
            ):
                angle = loc[1] * 360 / outer_circumf
                if angle < 40 or angle > 360 - 40:
                    continue
                if angle > 180 - 40 and angle < 180 + 40:
                    continue
                if angle > 90 - 20 and angle < 90 + 20:
                    continue
                h2 = (
                    h.rotate_y(90)
                    .translate((hex_placement_inner_r, 0, loc[0]))
                    .rotate_z(angle)
                )
                t = t.cut(h2)

        # water tank dimensions
        tank_cyl_ht = 20
        tank_cyl_r = ledboard_inner_r + 5

        # put holes in the top surface for venting out heat.
        # note: adding vent holes *significantly* increases the amount of time
        # cadquery takes to render the model.
        if vent_holes:
            h = cq.Workplane("XY").polygon(nSides=6, diameter=hexdiam).extrude(10)
            for loc in gen_tiled_hexagon_locations(
                start=(-self.profile.outer_r, -self.profile.outer_r),
                rows=23,
                cols=23,
                spacing=hex_spacing,
                hex_diameter=hexdiam,
            ):
                r_sq = loc[0] ** 2 + loc[1] ** 2
                # don't put holes too close to the edge
                if r_sq > (self.profile.inner_r - 8) ** 2:
                    continue
                # don't put holes in the water tank
                if r_sq < (tank_cyl_r + 5) ** 2:
                    continue

                # avoid holes by the reset button.
                if abs(loc[0]) < 10 and loc[1] > 50:
                    continue
                # avoid holes by the fan mounts
                if abs(loc[0]) > self.profile.inner_r - 20:
                    continue

                t = t.cut(h.translate(loc))

        # tank to hold water for misting
        tank = WaterTank(
            tank_cyl_ht=tank_cyl_ht,
            tank_cyl_r=tank_cyl_r,
            max_h=self.profile.h - underplate_top_bottom_offset,
        )
        # tank = tank.rotate_z(30)
        self.underplate_mate_radius = tank.underplate_mate_radius
        # hole for filling with water
        t = t.workplaneFromTagged("backZ").hole(tank.inlet.hole_r * 2)
        # rotate the tank 30 degrees so that the mister wire points directly to
        # the back towards the mainboard, which it plugs into.
        t = t.union(tank.shape.rotate_z(30))

        mainboard_barreljack_cut_tol = 0.2

        # mount post for mainboard screw
        self.mainboard_screw_y_offset = (
            self.profile.inner_r
            - mainboard_arc_to_back_dist
            + mainboard_hole_offset_from_edge
        )
        mainboard_offset_from_top = (
            mainboard_barreljack_h + mainboard_barreljack_cut_tol
        )  # TODO: ?
        mp = (
            cq.Workplane("XY")
            .moveTo(0, self.mainboard_screw_y_offset)
            .circle(3.9)
            .extrude(self.profile.th + mainboard_offset_from_top)
        )
        # add cone for support
        cn = cq.CQ(cq.Solid.makeCone(3.9 * 3, 3.9, self.profile.th + 6)).translate(
            (0, self.mainboard_screw_y_offset, 0)
        )
        mp = mp.union(cn)
        mp = mp.faces(">Z").hole(m3_selftap_hole_r * 2, mainboard_offset_from_top - 1)
        t = t.union(mp)

        # cutout for rectangular face of power plug
        pplug_cut_w = mainboard_barreljack_w + mainboard_barreljack_cut_tol
        pplug_cut_h = mainboard_barreljack_h + mainboard_barreljack_cut_tol
        ppcut_l = 10
        ppcut = (
            cq.Workplane("XY")
            .move(-pplug_cut_w / 2, 0)
            .rect(pplug_cut_w, ppcut_l, centered=False)
            .extrude(pplug_cut_h)
            .translate(
                (
                    0,
                    self.profile.inner_r
                    - ppcut_l
                    + mainboard_barreljack_overhang
                    + mainboard_barreljack_cut_tol,
                    self.profile.th
                    + mainboard_offset_from_top
                    - mainboard_barreljack_h
                    - mainboard_barreljack_cut_tol / 2,
                )
            )
        )
        t = t.cut(ppcut)

        # circular cutout for plug
        # TODO: re-evaluate the y offset of this cutout
        barreljack_center_offset_from_board = 6.75
        barreljack_plug_cut_radius = 10 / 2
        plugcut = (
            cq.Workplane("XZ")
            .circle(barreljack_plug_cut_radius)
            .extrude(-30)
            .translate(
                (
                    0,
                    self.profile.inner_r
                    + mainboard_barreljack_overhang
                    + mainboard_barreljack_cut_tol,
                    self.profile.th
                    + mainboard_offset_from_top
                    - barreljack_center_offset_from_board,
                )
            )
        )
        t = t.cut(plugcut)

        # cutout for micro usb port
        usb_cut_w = 15
        usb_cut_h = 9
        usb_cut_l = 40
        usb_port_middle_offset_from_mainboard = 1.5
        usbcut = (
            cq.Workplane("XY")
            .moveTo(-usb_cut_w / 2, 0)
            .rect(usb_cut_w, usb_cut_l, centered=False)
            .extrude(usb_cut_h)
            .edges("|Y")
            .fillet(3)
            .translate(
                (
                    0,
                    self.profile.inner_r - 1,
                    self.profile.th
                    + mainboard_offset_from_top
                    - usb_port_middle_offset_from_mainboard
                    - usb_cut_h / 2,
                )
            )
            .rotate_z(-mainboard_usb_port_angle_deg)
        )
        t = t.cut(usbcut)

        # hole for pressing the reset button with a paperclip
        button_ht = 2
        paperclip_tube_y_offset = (
            self.profile.inner_r
            - mainboard_arc_to_back_dist
            + mainboard_button_offset_from_edge
        )
        ppclip_tube = (
            cq.Workplane("XY")
            .circle(4.5)
            .extrude(self.profile.th + mainboard_offset_from_top - button_ht)
        )
        ppclip_tube = ppclip_tube.translate((0, paperclip_tube_y_offset, 0))
        ppclip_tube_hole_r = 1.75 / 2
        cn2 = cq.CQ(cq.Solid.makeCone(3.9 * 3, 3.9, self.profile.th + 6)).translate(
            (0, paperclip_tube_y_offset, 0)
        )
        ppclip_tube = ppclip_tube.union(cn2)
        t = t.union(ppclip_tube)
        # cut a hole through the entire top and paperclip tube.
        # note: if the hole is placed exactly at the right offset, it
        # disappears... so I've added a tiny additional offset. wtf.
        t = (
            t.workplaneFromTagged("backZ")
            .transformed(offset=(0, -paperclip_tube_y_offset + 0.00000001, 0))
            .hole(ppclip_tube_hole_r * 2)
        )

        # add a "reset" label near the paperclip hole
        t = (
            t.workplaneFromTagged("backZ")
            .transformed(offset=(0, -paperclip_tube_y_offset - 5, 0))
            .text("reset", 6, -1)
        )

        # TODO: what text do we want on top?
        t = (
            t.workplaneFromTagged("backZ")
            .transformed(offset=(0, 30, 0), rotate=(0, 0, 180))
            .text("Oasis", 14, -1)
        )

        # Mounts for fans

        bf = BlowerFan()
        # TODO: take a look at inlet_cutout_ht again
        bfc = BlowerFanCutOut(bf, inlet_cutout_ht=5, mount_screw_hole_depth=0)

        mount_side_th = 1.5
        fan_mount = (
            cq.Workplane("XY")
            .moveTo(profile.inner_r)
            .rect((profile.inner_r - ledboard_outer_r - 0.5) * 2, 200)
            .extrude(profile.h)
        )

        # make the mount block as high as we can without entrapping a corner of the fan.
        self.fan_mount_block_h = profile.h - 22
        bound = (
            cq.Workplane("XY")
            .circle(profile.inner_r + 1)
            .extrude(self.fan_mount_block_h)
        )
        fan_mount = bound.intersect(fan_mount)

        # apply the translates and rotates to move the fan into place
        self.fan_transform = (
            lambda shape: shape.rotate_x(180)
            .rotate_z(self.fan_rotate_angle)
            .rotate_y(90)
            .translate((self.fan_offset_x, 0, self.fan_offset_z))
        )

        fan_mount = fan_mount.cut(self.fan_transform(bfc.shape))

        # add mating plug hole that underplate plugs slot into
        plug_hole = (
            cq.Workplane("XY")
            .circle(top_mate_plug_r + 0.2)
            .extrude(top_mate_plug_depth)
            .translate(
                (
                    top_mate_plug_x_offset,
                    0,
                    self.fan_mount_block_h - top_mate_plug_depth,
                )
            )
            .rotate_z(-top_mate_plug_angle_offset)
        )
        fan_mount = fan_mount.cut(plug_hole)

        # add screw holes for attaching underplate
        # TODO: could add a slight countersink so these go in easier
        screw_depth = self.fan_mount_block_h - self.profile.th - 2
        screw_cut = cq.Workplane("XY").circle(m3_selftap_hole_r).extrude(screw_depth)
        screw_cut = screw_cut.translate(
            (underplate_screw_x_offset, 0, self.fan_mount_block_h - screw_depth)
        ).rotate_z(-underplate_screw_angle_offset)
        fan_mount = fan_mount.cut(screw_cut)

        # add fan mount
        t = t.union(fan_mount)

        ip = self.fan_transform(fan_inlet_path(bf).rotate_z(-60))
        op = self.fan_transform(fan_outlet_path(bf))

        # make a copy of the fan mount and put it on the opposite side
        t = t.union(fan_mount.rotate_z(180))

        # apply airflow cutouts on both sides
        t = t.cut(ip)
        t = t.cut(op)
        t = t.cut(ip.rotate_z(180))
        t = t.cut(op.rotate_z(180))

        self.shape = t


# Clips that hold the ledboard in place on the underplate.
#
# TODO: this could use a lot of improvement. the coordinate systems are a mess
# and it's overall a bit rough.
def underplate_ledboard_mount_clips():
    optic = LedOptic()
    clip_overhang = 1.0
    clip_underhang = 2
    clip_extra_h = 2
    clip_w = 4
    assert clip_w > clip_underhang
    pcb_w = ledboard_outer_r - ledboard_inner_r

    # gap between top of pcb and bottom of clip "overhang"
    pcb_v_tol = 0.3

    def make_clip(xoff=0, mirror=False):
        # start at lower-left corner and define points "counter-clock-wise"
        # note: the use of "left" and "right" below are for when mirror=False.
        # When mirror=True, the "left" and "right" are obviously mirrored.
        clip2d = (
            cq.Workplane("XZ", origin=(xoff, pcb_w / 2, 0))
            .transformed(rotate=(0, 180 if mirror else 0, 0))
            # start at lower-left corner
            .moveTo(-pcb_w / 2, 0)
            .hLine(clip_underhang)
            # move up to the where the bottom of the pcb is
            .vLine(optic.h)
            # move right to the edge of the side of the pcb
            .hLine(-clip_underhang)
            # move up to just above the top of the pcb
            .vLine(ledboard_th + pcb_v_tol)
            # move left to create the clip overhang
            .hLine(clip_overhang)
            # move up and right
            .line(-(clip_w - clip_overhang), clip_extra_h)
            # move down to the bottom of the clip
            .vLine(-(optic.h + ledboard_th + clip_extra_h + pcb_v_tol))
            .close()
        )
        return clip2d

    # clip_inner = make_clip(ledboard_center_r).revolve(
    #     axisStart=(-ledboard_center_r, 0, 0), axisEnd=(-ledboard_center_r, -1, 0)
    # )
    clip_outer = make_clip(ledboard_center_r, mirror=True).revolve(
        axisStart=(ledboard_center_r, 0, 0), axisEnd=(ledboard_center_r, -1, 0)
    )
    # clips = clip_outer.add(clip_inner)
    clips = clip_outer

    return clips.translate((0, -pcb_w / 2, 0))


# The water tank holds water for the mister. It is a part of the Top piece.
class WaterTank:

    def __init__(
        self, top_r=18, tank_cyl_ht=20, tank_cyl_r=ledboard_inner_r + 5, max_h=40
    ):
        self.top_r = top_r
        self.max_h = max_h
        self.shell_th = 2.2
        self.underplate_mate_radius = self.top_r + 4

        # note: the screws protrude 7.8mm from the top of the mount base

        # note: we don't actually use the MisterMount shape for the WaterTank.
        # Instead, we create our own o-ring placement and screw holes.
        mmobj = MisterMount(th=5)

        # main tank shape
        tank_envelope = (
            cq.Workplane("XZ")
            .hLine(tank_cyl_r)
            .vLine(tank_cyl_ht)
            .lineTo(self.top_r, max_h)
            .hLineTo(0)
            .close()
            .revolve()
            .edges()
            .fillet(3)
        )
        # tank envelope is a fully solid thing. `shell()` makes it hollow so it
        # can actually store water.
        tank = tank_envelope.shell(-self.shell_th)

        # adjust the shape to have a straight cylindrical bit at the top so it
        # fits into the underplate
        # TODO: refactor this into the main shape definition above?
        underplate_th = 2  # TODO: source this from underplate
        fit_th = underplate_th + 1
        fit = (
            cq.Workplane("XY")
            .circle(self.underplate_mate_radius)
            .extrude(fit_th)
            .translate((0, 0, max_h - fit_th))
        )
        tank.add(fit)

        # hole/inlet for filling tank
        self.inlet = TopPlugInlet(TopPlug())
        # cut hole
        tank = (
            tank.faces("<Z").workplane().hole(diameter=self.inlet.hole_r * 2, depth=10)
        )
        # merge the inlet (has threads that mate with the plug and a spot for an
        # oring)
        tank = tank.union(
            self.inlet.shape.rotate_x(180).translate((0, 0, self.inlet.h))
        )

        # add a center post so that the top of the tank isn't a problematic overhang for 3d printing
        post_z_off = self.inlet.h
        post = (
            cq.Workplane("XY")
            .circle(self.inlet.bottom_outer_r - 2)
            .extrude(max_h - self.inlet.h)
            .translate((0, 0, self.inlet.h))
        )

        # cone creates a smoother interface between the inlet and the post
        bottom_cone = (
            cq.Workplane("XZ")
            .moveTo(0, self.inlet.h)
            .hLine(self.inlet.bottom_outer_r)
            .lineTo(0, 23)
            .close()
            .revolve()
        )
        post = post.union(bottom_cone)

        # this cone cutout removes some horizontal overhangs that would be problematic to print.
        cone_cut = (
            cq.Workplane("XZ")
            .moveTo(0, self.inlet.h - 1)
            .hLine(self.inlet.bottom_hole_r + 0.7)
            .lineTo(0, 20)
            .close()
            .revolve()
        )
        post = post.cut(cone_cut)

        # connect post to top with a cone for easier printing
        cone = (
            cq.Workplane("XZ")
            .moveTo(0, max_h - 25)
            .lineTo(self.top_r + 3, max_h)
            .hLineTo(0)
            .close()
            .revolve()
        )
        post = post.union(cone.intersect(tank_envelope))

        topmost_hole_r = 3
        topmost_hole = (
            cq.Workplane("XY").circle(topmost_hole_r).extrude(100).rotate_y(90)
        )

        post_hole_rotate_z = 19
        # 3 holes near the mister - rotation is determined so that the holes don't intersect the screw holes
        for i in range(3):
            post = post.cut(
                topmost_hole.translate(
                    (0, 0, max_h - self.shell_th - topmost_hole_r)
                ).rotate_z(60 + 120 * i)
            )

        # 4 holes near the water entry point
        for i in range(4):
            post = post.cut(
                topmost_hole.translate(
                    (0, 0, self.inlet.h + topmost_hole_r + 0.5)
                ).rotate_z(i * 90)
            )

        # even more holes
        for i in range(4):
            post = post.cut(
                topmost_hole.translate((0, 0, self.max_h / 2 + 6)).rotate_z(i * 90)
            )
        for i in range(4):
            post = post.cut(
                topmost_hole.translate((0, 0, self.max_h / 2 - 1)).rotate_z(45 + i * 90)
            )

        tank = tank.union(post)

        # o-ring slot
        tank = tank.cut(oring12x3(tol=0.1).translate((0, 0, max_h)))

        # screw holes
        for i in range(3):
            # depth should be 4mm minimum for 8mm M3 screws
            screw_hole_depth = 5
            hole = (
                cq.Workplane("XY")
                .circle(m3_selftap_hole_r)
                .extrude(screw_hole_depth)
                .translate((mmobj.hole_dist_from_center, 0, max_h - screw_hole_depth))
                .rotate_z(i * 120)
            )
            tank = tank.cut(hole)

        # TODO: calculate and print out tank volume

        # hole in bottom of tank where fogger disc attaches
        tank = tank.faces(">Z").workplane().hole(mmobj.center_hole_r)

        self.shape = tank


# 12x3 o-ring from assorted metric set
# https://www.amazon.com/dp/B08QM9L9J5
# rubber has diameter=3mm; o ring has diameter 12+1.5*2
# @tol is the increase in base radius of the rubber for tolerance purposes
def oring12x3(tol=0.0):
    return cq.Workplane("XZ").moveTo(12 / 2 + 3 / 2, 0).circle(3 / 2 + tol).revolve()


# Test print for checking that the mainboard mounting/positioning works.
def test_mainboard_mount_and_cutouts():
    t = Top(TopProfile()).shape
    select = cq.Workplane("XY").rect(23, 65).extrude(17).translate((0, 80, 0))
    t = t.intersect(select)
    return t


# Test print for checking that the ledboard clips fit.
def underplate_clips_test(
    # parameters for selection.
    min_select_x=40,
    max_select_x=ledboard_outer_r + 10,
    select_y=60,
    revolve_deg=20,
):
    t = underplate_ledboard_mount_clips()
    t = t.union(cq.Workplane("XY").circle(ledboard_outer_r + 3).extrude(2))

    select = (
        cq.Workplane("XY")
        .move(min_select_x, 0)
        .rect(max_select_x - min_select_x, select_y, centered=False)
        .revolve(angleDegrees=revolve_deg)
        .rotate_x(90)
    )
    return t.intersect(select)


# A small cylindrical part that sits below the underplate and holds the sht30
# sensor board. It has two tabs on top that slot into the underplate to hold it
# in place.
class SensorBasket:

    def __init__(self, sb: Sht30Board):
        self.th = 2
        self.inner_r = 10
        self.outer_r = self.inner_r + self.th
        underplate_th = 2  # TODO: source this elsewhere
        self.base_h = 15
        tab_h = 4
        h = self.base_h + underplate_th + tab_h
        basket = cq.Workplane("XY").circle(self.outer_r).extrude(h)
        basket = basket.cut(
            cq.Workplane("XY").workplane(offset=self.th).circle(self.inner_r).extrude(h)
        )

        # put vent holes around the perimeter
        # TODO: most of this code was copied from Top - this should be refactored out into its own function
        hexdiam = 4
        hex_spacing = 1.1
        if True:
            hex_placement_inner_r = self.inner_r - 1
            hex_placement_outer_r = self.outer_r + 1
            hex_l = hex_placement_outer_r - hex_placement_inner_r
            hex = (
                cq.Workplane("XY")
                .polygon(
                    nSides=6,
                    diameter=hexdiam * hex_placement_inner_r / hex_placement_outer_r,
                )
                .workplane(offset=hex_l)
                .polygon(nSides=6, diameter=hexdiam)  # .extrude(5)
                .loft()
            )

            outer_circumf = 2 * math.pi * hex_placement_outer_r
            for loc in gen_tiled_hexagon_locations(
                start=(hexdiam / 2 + self.th, 0),
                rows=18,
                cols=3,
                spacing=hex_spacing,
                hex_diameter=hexdiam,
            ):
                angle = loc[1] * 360 / outer_circumf
                hex2 = (
                    hex.rotate_y(90)
                    .translate((hex_placement_inner_r, 0, loc[0]))
                    .rotate_z(angle)
                )
                basket = basket.cut(hex2)

        # put vent holes in the bottom
        for loc, hex in gen_honeycombed_circle(
            hexdiam,
            hex_thickness=2,
            hex_spacing=hex_spacing,
            circle_center=(0, 0),
            circle_r=self.inner_r,
        ):
            basket = basket.cut(hex)

        self.tab_angle = 35
        tab_support_angle = 20

        sidecut = (
            cq.Workplane("XZ")
            .moveTo(0, self.base_h)
            .rect(100, 100, centered=False)
            .revolve(180 - self.tab_angle)
        )
        basket = basket.cut(sidecut)
        basket = basket.cut(sidecut.rotate_z(180))

        slotcut = (
            cq.Workplane("XZ")
            .moveTo(0, self.base_h)
            .rect(100, underplate_th + 0.5, centered=False)
            .revolve(180 - tab_support_angle)
        )
        basket = basket.cut(slotcut)
        basket = basket.cut(slotcut.rotate_z(180))

        board_z_off = 6
        cutout_board = (
            cq.Workplane("XY")
            .rounded_rect(sb.size[0] + 0.2, sb.size[1] + 0.2, 1.15)
            .extrude(sb.th + 10)
        ).translate((0, 0, board_z_off))

        header_bottom_cutout = (
            cq.Workplane("XY")
            .rounded_rect(3, sb.size[1], 1.5)
            .extrude(2)
            .translate(
                (
                    sb.size[0] / 2 - 3 / 2,
                    0,
                    board_z_off - 2,
                )
            )
        )
        bottom_cutout = (
            cq.Workplane("XY")
            .rounded_rect(sb.size[0] - 1, sb.size[1] - 1, 1.1)
            .extrude(10)
        )

        holder = (
            cq.Workplane("XY")
            .rounded_rect(sb.size[0] + 2, sb.size[1] + 2, 1.25)
            .extrude(board_z_off + sb.th + 2)
        )
        holder = holder.cut(cutout_board)
        holder = holder.cut(header_bottom_cutout)
        holder = holder.cut(bottom_cutout)

        basket = basket.union(holder.translate((0, 0, 0)))

        self.shape = basket


# Mounting slots for the sensor basket. A hole in the center allows the sensor
# cable to pass through and two side slots mate with the tabs on the sensor
# basket to hold it in place.
def underplate_cutout_for_sensor_basket(undermount: SensorBasket, th):
    # sized for a 4-pin female header.
    wire_cutout = cq.Workplane("XY").rect(3.5, 11.5).extrude(th)
    tol = 0.12
    angle_tol = 2
    tab_cutout = (
        cq.Workplane("XZ")
        .moveTo(undermount.inner_r - tol, 0)
        .rect(undermount.th + tol * 2, th, centered=False)
        .revolve(undermount.tab_angle + angle_tol)
        .rotate_z(-undermount.tab_angle / 2 - angle_tol / 2)
    )

    return wire_cutout + tab_cutout + tab_cutout.rotate_z(180)


# Test print for verifying the fit of the underplate mounting slots/cutouts.
def underplate_cutout_test(undermount: SensorBasket, th=2):
    return (
        cq.Workplane("XY")
        .circle(undermount.outer_r + 1.5)
        .extrude(th)
        .cut(underplate_cutout_for_sensor_basket(undermount, th))
    )


# The underplate attaches to the underside of the Top piece. Features include:
# - mounts for the five led optics
# - clips to attach the ledboard above the led optics
# - a cutout / slots for attaching the sensor basket on the underside
# - a large hole in the middle to allow the water tank / mister assembly to pass through
# - air inlet/outlet paths for the two fans
#
# TODO: come up with a better name for this thing - what the hell is an "underplate"?
class Underplate:

    def __init__(self, top: Top, th=2):
        self.th = th
        self.outer_r = top.profile.inner_r - 0.05

        optic = LedOptic()

        # overall shape
        up = cq.Workplane("XY").circle(self.outer_r).extrude(self.th)

        # cutout for mister/tank in the center
        up = (
            up.faces(">Z")
            .workplane()
            .hole(diameter=top.underplate_mate_radius * 2 + 0.6)
        )

        # small cutout near the water tank that the mister wire can pass through
        mister_wire_cutout = (
            cq.Workplane("XY")
            .rect(3, 3)
            .extrude(self.th)
            .translate((0, -top.underplate_mate_radius - 3 / 2, 0))
        )
        up = up.cut(mister_wire_cutout)

        # create a clip slice object which we'll copy and place once for each led
        full_clip_ring = underplate_ledboard_mount_clips().translate((0, 0, self.th))
        # on first full print of underplate, slice angle was 10. this was a bit too big, so we're going smaller.
        clip_slice_angle = 7
        clip_slice_select = (
            cq.Workplane("XY")
            .rect(self.outer_r, 100, centered=False)
            .revolve(angleDegrees=clip_slice_angle)
            .rotate_x(90)
        )
        clip_slice = full_clip_ring.intersect(clip_slice_select)

        # for each led optic, add a circular wall, then cutout the optic profile
        # and a hole through the underplate for the light to shine through
        for i in range(led_count):
            angle = led0_angle_deg + i * 360 / led_count + ledboard_rotation_angle

            # add short circular walls to hold led optics in place
            circ_wall = cq.Workplane("XY").circle(optic.r + 3).extrude(2)
            circ_wall = circ_wall.translate((ledboard_center_r, 0, self.th)).rotate_z(
                angle
            )
            up = up.union(circ_wall)

            # add underplate clip
            up = up.union(clip_slice.rotate_z(angle - clip_slice_angle / 2))

            # main optic cutout shape
            cutout = optic.cutout.translate((0, 0, self.th))
            # rotate optic cutout so rectangular clip bit doesn't intersect the
            # clip for the ledboard.
            cutout = cutout.rotate_z(90)
            # add a hole for the light to shine through the underplate
            cutout = cutout.union(
                cq.Workplane("XY").circle(optic.r - 1.5 / 2).extrude(self.th)
            )
            cutout = cutout.translate((ledboard_center_r, 0, 0))
            cutout = cutout.rotate_z(angle)
            up = up.cut(cutout)

        # sht30 mount
        sht30_mnt_y_off = 70
        sens_cutout = underplate_cutout_for_sensor_basket(
            SensorBasket(Sht30Board()), self.th
        ).translate((0, -sht30_mnt_y_off, 0))
        up = up.cut(sens_cutout)

        fan_mount_block_h = (
            top.profile.h - underplate_top_bottom_offset - top.fan_mount_block_h
        )

        # add a ring around the perimeter for structural support - this helps
        # keep the underplate from being able to flex too much.
        outer_ring = (
            cq.Workplane("XY")
            .circle(self.outer_r)
            .circle(self.outer_r - 5)
            .extrude(fan_mount_block_h)
        )
        up = up.union(outer_ring)

        # fan mounting and inflow/outflow paths

        bf = BlowerFan()

        bound = cq.Workplane("XY").circle(self.outer_r).extrude(fan_mount_block_h)

        fm1 = (
            cq.Workplane("XY")
            .rect(100, self.outer_r * 2)
            .extrude(20)
            .translate((50 + ledboard_outer_r + 0.5, 0, 0))
            .intersect(bound)
        )

        fan_z_adjust = top.profile.h - underplate_top_bottom_offset - top.fan_offset_z
        # when we rotate_x(180) below, we effectively move the fan placement
        # from +fan_offset_z to -fan_offset_z. Adding fan_offset_z cancels that
        # out.
        fan_z_adjust += top.fan_offset_z
        fan_transform = (
            lambda shape: top.fan_transform(shape)
            .rotate_x(180)
            .translate((0, 0, fan_z_adjust))
        )

        up = up.union(fm1)

        ip = fan_transform(fan_inlet_path(bf).rotate_z(-60))
        up = up.cut(ip)

        op = fan_transform(fan_outlet_path(bf))
        up = up.cut(op)

        # copy it on the other side
        # up = up.union(fm1.rotate_z(180)) # TODO: this causes rendering to hang
        fm2 = (
            cq.Workplane("XY")
            .rect(100, self.outer_r * 2)
            .extrude(20)
            .translate((-50 - ledboard_outer_r - 0.5, 0, 0))
            .intersect(bound)
        )
        up = up.union(fm2)

        up = up.cut(ip.rotate_z(180))
        up = up.cut(op.rotate_z(180))

        # note: bfc needs to be cut out *after* outer ring is added. in other words, apply it to the whole underplate, not just the fan mount.
        bfc = BlowerFanCutOut(
            bf, inlet_cutout_ht=0, wire_slot_cutout_ht=0, mount_screw_hole_depth=0
        )
        up = up.cut(fan_transform(bfc.shape))
        up = up.cut(fan_transform(bfc.shape).rotate_z(180))

        # mating plugs that connect between Underplate and Top. Underplate has pegs, Top
        # has holes.
        plug = (
            cq.Workplane("XY")
            .circle(top_mate_plug_r)
            .extrude(top_mate_plug_depth - top_mate_plug_r - 1)
        )
        plug = plug.union(
            cq.Workplane("XY")
            .sphere(top_mate_plug_r)
            .translate((0, 0, top_mate_plug_depth - 1 - top_mate_plug_r))
        )
        plug = plug.translate((top_mate_plug_x_offset, 0, fan_mount_block_h)).rotate_z(
            top_mate_plug_angle_offset
        )
        # add one plug on each side
        up = up.union(plug)
        up = up.union(plug.rotate_z(180))

        # mounting screw holes
        screw_cut = (
            cq.Workplane("XY")
            .circle(6.5 / 2)
            .extrude(3.5)
            .faces(">Z")
            .circle(m3_passthrough_hole_r)
            .extrude(50)
        )
        screw_cut = screw_cut.translate((underplate_screw_x_offset, 0, 0)).rotate_z(
            underplate_screw_angle_offset
        )
        up = up.cut(screw_cut)
        up = up.cut(screw_cut.rotate_z(180))

        self.shape = up


# test print for checking if the led optic fits ok
def underplate_optic_fitment_demo():
    optic = LedOptic()
    upobj = Underplate(Top(TopProfile()))
    bound = (
        cq.Workplane("XY")
        .circle(optic.r + 2)
        .extrude(50)
        .translate((ledboard_center_r, 0, 0))
        .rotate_z(led0_angle_deg)
    )
    return upobj.shape.intersect(bound)


# The underplate (sub)assembly contains:
# - underplate
# - led optics
# - ledboard
# - sensor board
# - sensor basket
# - mounting screws
def underplate_assembly(top: Top, explode=0, show_light_cones=False):
    asm = cq.Assembly()

    underplate = Underplate(top)
    asm.add(underplate.shape, color=print_color, name="underplate")

    optic = LedOptic()

    # TODO: fix rotation angle
    lb = ledboard(prefer_kicad=True).rotate_z(ledboard_rotation_angle)
    asm.add(
        lb,
        loc=cq.Location((0, 0, optic.h + underplate.th + explode)),
        name="ledboard",
        color=ledboard_color,
    )

    sht30_board = Sht30Board()
    asm.add(
        sht30_board.shape,
        name="sht30_board",
        loc=cq.Location(
            (
                0,
                -70,
                -10 - explode / 3,
            )
        ),
        color=cq.Color("purple"),
    )

    for i in range(led_count):
        angle = led0_angle_deg + i * 360 / led_count
        o = (
            optic.shape.rotate_z(90)
            .translate(
                (
                    ledboard_center_r,
                    0,
                    underplate.th + explode / 2,
                )
            )
            .rotate_z(angle + ledboard_rotation_angle)
        )
        asm.add(o, color=glass_color, name=f"optic{i}")

        if show_light_cones:
            # angle in degrees of light cone coming out of optic
            optic_angle = 31.1  # carclo 10140 optic
            # length of cone from tip to center of bottom circle
            cone_l = 300
            cone_r = math.tan(radians(optic_angle / 2)) * cone_l
            cone = cq.Solid.makeCone(cone_r, 0, cone_l)
            asm.add(
                cone,
                color=light_color,
                loc=cq.Location(
                    (
                        ledboard_center_r * math.cos(radians(angle)),
                        ledboard_center_r * math.sin(radians(angle)),
                        top_off_z
                        - top_profile.h
                        + underplate.th
                        + underplate_top_bottom_offset
                        - explode * 0.75
                        - cone_l
                        + optic.h,
                    )
                ),
            )

    sb = SensorBasket(sht30_board)
    asm.add(
        sb.shape,
        loc=cq.Location((0, -70, -sb.base_h - explode)),
        color=print_color,
        name="sensor_basket",
    )

    # mounting screws to attach the underplate to the top
    screw = m3_screw(20)
    screw = screw.translate((underplate_screw_x_offset, 0, 0)).rotate_z(
        underplate_screw_angle_offset
    )
    asm.add(
        screw,
        loc=cq.Location((0, 0, -explode)),
        color=cq.Color("gray"),
        name="mount_screw_1",
    )
    asm.add(
        screw.rotate_z(180),
        loc=cq.Location((0, 0, -explode)),
        color=cq.Color("gray"),
        name="mount_screw_2",
    )

    return asm


# Full assembly of the terrarium used to visually see how it all comes together.
def terrarium(explode=50, show_light_cones=False, vent_holes=False):
    """
    Assembly that pulls in all the individual components together to form the
    final terrarium

    Note: unfortunately I couldn't figure out how to use constraints and still
    show an exploded view, so I'm manually positioning every component.
    """

    bottom_profile = BottomProfile()

    asm = cq.Assembly()
    asm.add(Bottom(bottom_profile).shape, color=print_color, name="bottom")
    tube = Tube(bottom_profile)
    asm.add(
        tube.shape,
        loc=cq.Location((0, 0, bottom_profile.th + explode)),
        color=glass_color,
        name="tube",
    )
    top_profile = TopProfile()
    top = Top(top_profile, vent_holes=vent_holes)
    top_offset_z = (
        bottom_profile.th + tube.h + top_profile.h - top.tube_inset + explode * 4
    )
    asm.add(
        top.shape.rotate_x(180),
        loc=cq.Location((0, 0, top_offset_z)),
        color=print_color,
        name="top",
    )

    top_plug = TopPlug()
    asm.add(
        top_plug.shape.rotate_x(180),
        loc=cq.Location((0, 0, top_offset_z + top_plug.knob_h + 0.5 + explode)),
        color=print_color,
        name="top_plug",
    )

    # oring for top plug
    asm.add(
        oring12x3(),
        loc=cq.Location((0, 0, top_offset_z - 4 + explode / 2)),
        color=cq.Color("black"),
        name="top_oring",
    )

    # mister disc
    mister_offset_z = (
        top_offset_z - top_profile.h + underplate_top_bottom_offset - explode
    )
    asm.add(
        MisterDisc().shape.rotate_x(180),
        loc=cq.Location((0, 0, mister_offset_z)),
        color=cq.Color("gray"),
        name="mister",
    )

    # TODO: check z offset
    mm_cover = mister_mount_cover()
    mm_cover_offset_z = mister_offset_z - 4 - explode / 2
    asm.add(
        mm_cover.rotate_x(180).rotate_z(-30),
        loc=cq.Location((0, 0, mm_cover_offset_z)),
        color=print_color,
        name="mister_mount_cover",
    )

    # add 3 m3 screws for holding down mister mount cover
    mister_screw = m3_screw(8).translate((MisterMount().hole_dist_from_center, 0, 0))
    for i in range(3):
        asm.add(
            mister_screw.rotate_z(i * 120 - 30),
            loc=cq.Location((0, 0, mm_cover_offset_z - 5 - explode / 3)),
            color=cq.Color("gray"),
            name=f"mister_screw_{i}",
        )

    # oring for mister mount
    asm.add(
        oring12x3(),
        loc=cq.Location((0, 0, mister_offset_z + 4 + explode / 2)),
        color=cq.Color("black"),
        name="mister_oring",
    )

    # TODO: deduplicate fan code
    # TODO: use top.fan_transform()?

    fan1 = (
        BlowerFan()
        .shape.rotate_x(180)
        .rotate_z(top.fan_rotate_angle + 180)
        .rotate_y(90)
    )
    asm.add(
        fan1,
        loc=cq.Location(
            (
                top.fan_offset_x,
                0,
                top_offset_z - top.fan_offset_z - explode,
            )
        ),
        color=cq.Color("black"),
        name="fan1",
    )

    # rotate fan 90deg so intake is on x-axis
    fan2 = BlowerFan().shape.rotate_x(180).rotate_z(top.fan_rotate_angle).rotate_y(-90)
    asm.add(
        fan2,
        loc=cq.Location(
            (
                -top.fan_offset_x,
                0,
                top_offset_z - top.fan_offset_z - explode,
            )
        ),
        color=cq.Color("black"),
        name="fan2",
    )

    mainboard = kicad_mainboard().rotate_z(-90)
    mainboard_bb = mainboard.val().BoundingBox()
    mainboard_top_offset_z = (
        top_offset_z - top.profile.th - mainboard_bb.zlen / 2 - explode
    )
    # TODO: z offset is not right
    mainboard_top_offset_z -= 2
    mainboard_offset_y = (
        top.profile.inner_r - mainboard_bb.ylen / 2 + mainboard_barreljack_overhang
    )
    asm.add(
        mainboard,
        loc=cq.Location((0, -mainboard_offset_y, mainboard_top_offset_z)),
        name="mainboard",
        color=mainboard_color,
    )

    mainboard_screw = m3_screw(8)
    asm.add(
        mainboard_screw,
        loc=cq.Location(
            (0, -top.mainboard_screw_y_offset, mainboard_top_offset_z - 2 - explode / 4)
        ),
        color=cq.Color("gray"),
        name="mainboard_screw",
    )

    upasm = underplate_assembly(
        top=top, explode=explode, show_light_cones=show_light_cones
    )

    asm.add(
        upasm,
        loc=cq.Location(
            (
                0,
                0,
                top_offset_z
                - top_profile.h
                + underplate_top_bottom_offset
                - explode * 2,
            )
        ),
    )

    return asm


# Print out values of key terrarium parameters.
#
# WARNING: this function instantiates all of the main models. If you separately
# build the model and call this function, you will be rendering everything
# twice, which can be slow.
def print_params():

    def print_object_params(obj, objname, param_names):
        print(objname + ":")
        for name in param_names:
            print(f"- {name}: {getattr(obj, name)}")
        print()

    print()
    print()
    print()
    print("Terrarium parameters:")
    print("--------------------------------------------------------------")
    pcb = LedboardSimple()
    print_object_params(pcb, "PCB", ["inner_r", "outer_r", "th"])
    tp = TopProfile()
    print_object_params(tp, "Top", ["h", "inner_r", "outer_r"])
    bp = BottomProfile()
    print_object_params(bp, "Bottom", ["h"])
    mb = MainboardSimple(TopProfile())
    print_object_params(mb, "Mainboard", ["offset", "arc_r", "side_len", "w"])
    tube = Tube(BottomProfile())
    print_object_params(tube, "Tube", ["th", "outer_r", "acrylic_sheet_length"])
    print()


# A test piece with honeycomb vent holes the same hexagon dimensions as the Top
# piece. Printed out as a test
def honeycomb_demo():
    th = 4
    shape = cq.Workplane("XY").circle(32).extrude(th)
    for loc, hex_cutout in gen_honeycombed_circle(
        hex_diameter=8,
        hex_thickness=th,
        hex_spacing=2,
        circle_r=30,
        circle_center=(0, 0),
    ):
        shape = shape.cut(hex_cutout)
    return shape


# Insert for Bottom that can prevent dirt and such from getting into the
# tube/wall slot while planting and scaping the terrarium.
class DirtBlockerTool:

    def __init__(self, profile: BottomProfile):
        th = 2
        tol = 0.3
        depth = 3
        self.shape = (
            cq.Workplane("XZ")
            .moveTo(profile.outer_r, 0)
            .vLine(th)
            .hLine(-profile.wall_th - tol)
            .vLine(depth)
            .hLine(-profile.tube_gap + tol * 2)
            .vLine(-depth)
            .hLineTo(profile.inner_r)
            .vLineTo(0)
            .close()
            .revolve()
        )


if __name__ == "__cq_main__":
    # print_params()
    t = terrarium(explode=50)
