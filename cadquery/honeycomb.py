# This file contains functions for generating tiled grids of hexagons. They are
# used to create ventilation holes in the terrarium.

import math
import mpmath
import cadquery as cq


def gen_tiled_hexagon_locations(
    start=(0, 0), hex_diameter=10, spacing=1, rows=5, cols=5
):
    """
    Generates center locations for a tiled grid of hexagons.

    Odd columns are offset vertically by half a row spacing.

    Yields:
        (x, y) coordinates for the center of each hexagon.
    """

    # Calculate side length
    side_length = hex_diameter / 2.0
    # Height of the hexagon
    h = math.sqrt(3) * side_length
    # horizontal distance from right-most of one hexagon to the left corner of
    # the *bottom* of the one up and to the right of it.
    q = spacing * mpmath.sec(math.radians(30))

    row_spacing = h + spacing
    col_spacing = hex_diameter + 2 * q + side_length

    # i is columns, j is rows
    for i in range(cols):
        for j in range(rows):
            odd_col = i % 2 == 1
            loc = (
                start[0]
                + math.floor(i / 2) * col_spacing
                + (col_spacing / 2 if odd_col else 0),
                start[1] + j * row_spacing + (row_spacing / 2 if odd_col else 0),
            )
            yield loc


def gen_honeycombed_circle(
    hex_diameter,
    hex_thickness,
    hex_spacing,
    circle_center,
    circle_r,
):
    """
    Generates a circular area filled with tiled hexagons, including partial hexagons that intersect with the bounds of the circle.

    Yields:
        ((x, y), hexagon) - note that `hexagon` here may be only a partial hexagon if the rest of it is "cut off" by the bounding circle.
    """

    boundcirc = (
        cq.Workplane("XY")
        .circle(circle_r)
        .extrude(hex_thickness)
        .translate(circle_center)
    )

    h = (
        cq.Workplane("XY")
        .polygon(nSides=6, diameter=hex_diameter)
        .extrude(hex_thickness)
    )
    # TODO: rows,cols calculation could be a lot better
    for loc in gen_tiled_hexagon_locations(
        start=(circle_center[0] - circle_r, circle_center[1] - circle_r),
        hex_diameter=hex_diameter,
        spacing=hex_spacing,
        rows=math.ceil(circle_r * 2.3 / (hex_diameter + hex_spacing)) + 1,
        cols=math.ceil(1.4 * circle_r * 2 / (hex_diameter + hex_spacing)) + 1,
    ):
        dst_sq_from_center = (loc[0] - circle_center[0]) ** 2 + (
            loc[1] - circle_center[1]
        ) ** 2
        if dst_sq_from_center > (circle_r + hex_diameter) ** 2:
            # print("skipping")
            continue
        h2 = h.translate(loc)
        if dst_sq_from_center >= (circle_r - hex_diameter) ** 2:
            # print("intersecting")
            h2 = h2.intersect(boundcirc)
        else:
            pass
            # print("no intersect necessary")
        yield (loc, h2)


def demo():
    osth = 10
    hexdiam = 10
    boundcirc_r = 30
    boundcirc_loc = (30, 30)
    outer_shape = cq.Workplane("XY").rect(200, 200).extrude(osth)
    boundcirc = (
        cq.Workplane("XY").circle(boundcirc_r).extrude(osth).translate(boundcirc_loc)
    )

    h = cq.Workplane("XY").polygon(nSides=6, diameter=hexdiam).extrude(osth)
    for loc in gen_tiled_hexagon_locations(rows=20, cols=20):
        h2 = h.translate(loc).intersect(boundcirc)
        outer_shape = outer_shape.cut(h2)
    return outer_shape


def demo2():
    """
    Creates the same result as demo, but much more efficient. Instead of always
    generating the 3d hexagon and `intersect()`ing it with the bounding circle
    (as in `demo()`), only do so for hexagons that actually do intersect the
    bounds. Skip those outside the bounds and use hexagons without intersecting
    if they're fully within the circle.
    """
    osth = 10
    outer_shape = cq.Workplane("XY").rect(200, 200).extrude(osth)
    for loc, cutout in gen_honeycombed_circle(
        hex_diameter=10,
        hex_thickness=osth,
        hex_spacing=1,
        circle_r=30,
        circle_center=(30, 30),
    ):
        outer_shape = outer_shape.cut(cutout)
    return outer_shape


if __name__ == "__cq_main__":
    show_object(demo2())
