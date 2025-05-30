## make usb traces same length

Probably doesn't matter at the speeds we're doing, but the traces between the esp32 and the usb connector should be the same length. Look into using the "differential pair" routing tool in kicad.

## Arc radius changed

Since the design of this board, the overall radius of the terrarium shrunk a couple millimeters. The arc radius of the mainboard should be changed to account for this, ideally without changing the distance from the hole and buttons to the outer edge of the board. Note that the change in radius is small enough that the fabricated board fits in the smaller terrarium design without issue.

## better mounting hole

consider using a plated hole with vias for better strength
