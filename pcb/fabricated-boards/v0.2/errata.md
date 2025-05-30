# Rfbb1 wrong size

shortly after submitting the design through the jlcpcb website, they reached out to let me know that there was an issue with Rfbb1. I had accidentally selected an 0402 resistor, but the footprint on the board design was for an 0603. They gave me the option to select the correct part.

old PN: C844504
new PN: C844759

note: the bom and such in this directory reflect the old part number.

# mister1 connector wrong rotation / wrong pin numbers

pin numbers are right. pin 1 is vcc (sort of), pin 2 is gnd.

the issue is that the part on the jlcpcb order is rotated 180 from what I have in my pcb layout in kicad. this might be a mistake in jlcpcb 3d viewer or a translation issue between kicad and jlc. could possibly be fixed with fabrication toolkit by adding a "FT Rotation Offset: 180" to the part attributes in kicad.

I contacted jlcpcb support and had them fix it before assembly.

TODO: adjust the kicad design so that this doesn't happen next time.

# Gate and source are swapped on the footprint for the fan and mister mosfets

see datasheet here: https://wmsc.lcsc.com/wmsc/upload/file/pdf/v2/lcsc/2307110943_VBsemi-Elec-SI2328DS-T1-GE3_C7420571.pdf

I was able to fix this by desoldering both mosfets, flipping them upside down and rotating a bit, and commiting crimes against soldering. The board is fully working now!

# power input connection/holes are too big

Not causing any issues at all, but make them a bit smaller next time.

# buttons aren't actually necessary

I'm not sure if the en and boot boot buttons are actually useful at all. Since we're using the builtin usb support of the c3, all programming can be done over usb without any button presses. It's possible we can re-use the buttons for something else and it's possible the en (reset?) button is still useful, but I haven't used either of them once yet. Consider removing the buttons on the next iteration or re-purposing them.
