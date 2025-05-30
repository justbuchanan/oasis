import cadquery as cq
import math

# The default unit is millimeters. An inch is 25.4mm.
INCH = 25.4

# define a constant for each axis for convenience
AXIS_X = (1, 0, 0)
AXIS_Y = (0, 1, 0)
AXIS_Z = (0, 0, 1)


# # evenly space points around a circle
# def radial_spaced_pts(r, n, theta0rad=0):
#     return list(
#         [
#             (
#                 math.cos(i * 2 * math.pi / n + theta0rad) * r,
#                 math.sin(i * 2 * math.pi / n + theta0rad) * r,
#             )
#             for i in range(0, n)
#         ]
#     )


# useful for cutaway views to see what's inside an object
def remove_pos_y(obj):
    l = 1000
    h = 1000
    a = obj.cut(cq.Workplane("XY").rect(l, l).extrude(h).translate((0, l / 2, -h / 2)))
    return a

    # sel = cq.Workplane("XY").rect(l, l).extrude(h).translate((0, -l / 2, -h / 2))
    # return obj.intersect(sel)


# useful for cutaway views to see what's inside an object
def remove_neg_y(obj):
    l = 1000
    h = 1000
    a = obj.cut(cq.Workplane("XY").rect(l, l).extrude(h).translate((0, -l / 2, -h / 2)))
    return a


# useful for cutaway views to see what's inside an object
def remove_pos_x(obj):
    l = 1000
    h = 1000
    a = obj.cut(cq.Workplane("XY").rect(l, l).extrude(h).translate((l / 2, 0, -h / 2)))
    return a


# useful for cutaway views to see what's inside an object
def remove_neg_x(obj):
    l = 1000
    h = 1000
    a = obj.cut(cq.Workplane("XY").rect(l, l).extrude(h).translate((-l / 2, 0, -h / 2)))
    return a


# # vent holes for fan
# def hole_grid_pts(pos, w, h, hole_r=1.5, hole_gap=1.3):

#     def d1(s, l, hole_r, hole_gap):
#         nholes = math.floor((l - hole_gap) / (hole_gap + hole_r * 2))
#         off = (l - (nholes * (hole_gap + hole_r * 2) - hole_gap)) / 2
#         return [s + off + hole_r + (i * (2 * hole_r + hole_gap)) for i in range(nholes)]

#     return [
#         (x, y)
#         for x in d1(pos[0], w, hole_r, hole_gap)
#         for y in d1(pos[1], h, hole_r, hole_gap)
#     ]


# borrowed from https://github.com/CadQuery/cadquery/issues/746
def rounded_rect(self, xlen, ylen, fillet_radius):
    rect = cq.Workplane().rect(xlen, ylen).val()
    pts = rect.Vertices()
    rect = rect.fillet2D(fillet_radius, pts)
    return self.eachpoint(lambda loc: rect.moved(loc), True)


def rotate_x(self, angle_deg):
    return self.rotate((0, 0, 0), AXIS_X, angle_deg)


def rotate_y(self, angle_deg):
    return self.rotate((0, 0, 0), AXIS_Y, angle_deg)


def rotate_z(self, angle_deg):
    return self.rotate((0, 0, 0), AXIS_Z, angle_deg)


cq.Workplane.rounded_rect = rounded_rect

# new defs
cq.Workplane.rotate_z = rotate_z
cq.Workplane.rotate_x = rotate_x
cq.Workplane.rotate_y = rotate_y
