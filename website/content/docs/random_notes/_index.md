+++
title = 'Random Notes'
weight = 7
+++

# Random Notes

## Light Intensity

I used the [Photone App for iOS](https://apps.apple.com/us/app/photone-grow-light-meter/id1450079523) to take some lighting measurements. An actual PAR meter would give more accurate measurements, but those are expensive and I already spent all my money on tariffs importing custom electronics from China.

These measurements were taken with an iPhone sitting at the _bottom_ of the terrarium. You will get higher PAR values higher up in the enclosure.

The LEDs are powerful and, due to thermal considerations, the LED driver is limited to 80% power in the firmware (see code [here](https://github.com/justbuchanan/oasis/blob/main/code/esp32/src/terrarium/real_terrarium.rs)). The below percentage values are what you would set in the terrarium's control interface, so for example, the 80% value below actually means 80%\*80%=64% of what the LEDs/driver are capable of.

This test was done with CREE XT-E 5700k LEDs. Results may vary for different color temperatures.

| Light % | PAR (umol/m^2/s) |
| ------- | ---------------- |
| 80%     | 325              |
| 100%    | 414              |

### How much light do plants want?

Here are a couple pages I found helpful:

- https://www.neherpetoculture.com/vivariumlighting101
- https://herebutnot.com/light-recommendations-ppfd-par-for-orchids-and-houseplants/

tl;dr: these LEDs are very bright, don't set them too high.

## Ideas for Future Improvement

The terrarium in its current state works well, but there's room for improvement. Below are a few ideas for a future version of the design:

- The terrarium should be able to detect when the water tank is empty. There are a few options for how to implement this:
    - Add a sensor inside the tank. A float sensor or capacitive sensor could work well.
    - Measure mister current draw. The mister circuit draws almost 2x as much current when water is present.
    - Measure humidity before and after running the mister and see if it increased. It should be possible to do this in software using the current hardware, but can be tricky.
- Increase airflow from fans. While the fans are not particularly powerful to begin with, they are somewhat limited in the current design by the size of the inflow and outflow paths built into the 3d printed parts. This is in turn limited by the size of the led board. If the led board is made smaller in a future version, there could be room to make the air inlet larger.
- The mister/mount/screws stick out below the bottom of the Top. Ideally, the top should be able to set down (face down) on a table and not have anything touch.
- The cord running down the back is ugly. I don't really see a way around this, but adding a cable clip to the bottom piece might help at least tidy it up a bit.
- Make the water tank bigger. Right now there's some "wasted" space that could be used to have the terrarium hold more water.
