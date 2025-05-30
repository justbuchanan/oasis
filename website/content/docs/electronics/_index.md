+++
title = 'Electronics'
weight = 4
+++

# Electronics

See below for details on the custom-designed electronics for this terrarium.

There are essentially three options for obtaining the electronics:

- Order bare PCBs (from OSH Park, JLCPCB, PCBWay) and parts (digikey, mouser, etc) and solder them yourself. This can be a good option if you have experience with SMD soldering.
- Order fully-assembled PCBs from JLCPCB. This can be a good option if you'd prefer not to solder the boards yourself, but it can be expensive for small order quantities.
- [Sign up to buy a kit](/docs/contact/#buying-a-kit) if/when they become available. I don't have any immediate or concrete plans to sell these, but will consider it if there's enough interest.

## Mainboard

The mainboard contains:

- a microcontroller with builtin wifi ([esp32-c3](https://www.espressif.com/en/products/socs/esp32-c3))
- led driver
- 18v to 3.3v voltage converter
- ultrasonic mister driver
- fan driver
- usb connector for updating firmware

Find the KiCad design files here: https://github.com/justbuchanan/oasis/tree/main/pcb/main.

### Schematic

![schematic](mainboard_schematic.png)

### 3D Renders

<!-- Show the front and back renders side-by-side -->
<div class="img-row">
  <div class="img-column2">
    <img src="mainboard_front3d.png"/>
    <p>Top View</p>
  </div>
  <div class="img-column2">
    <img src="mainboard_back3d.png"/>
    <p>Bottom View</p>
  </div>
</div>

## Ledboard

The LED board is very simple - it's 5 LEDs wired in series mounted on an aluminum board to act as a heatsink.

Find the KiCad design files here: https://github.com/justbuchanan/oasis/tree/main/pcb/ledboard.

TODO: add info on JST connector connector/cable.

### Schematic

![schematic](ledboard_schematic.png)

### 3D Render

![front](ledboard_front3d.png)

## SHT30 Sensor Board Wiring

The sht30 sensor board needs comes without a cable.

TODO

![sht30_wiring](sht30_wiring.jpg)

## PCB Order History

### v0.1 all-in-one board - Feb 11, 2025

Single-board design - esp32, leds, sht30 temp/humid sensor, and everything else all on one board.

A lot of the board was assembled by JLC, but the following parts were purchased and assembled separately (in total, probably $17 worth of parts per board):

- led driver
- leds
- sht30 temp/humid sensor
- sht30 bypass capacitor

Files: https://github.com/justbuchanan/oasis/tree/main/pcb/fabricated-boards/v0.2

#### Pricing

note: this price info is what I was charged by JLC, it does not include the components I sourced and assembled myself (mentioned above).

- total: $140
    - merchandise total: $89
    - shipping: $38
    - sales tax: $13

### v0.2 ledboard - March 16, 2025

Based on the poor thermal performance of the all-in-one boards, I decided to do a major redesign and put the leds on their own board, made out of solid aluminum. The electronics are on their own board.

This order is for 5x 1.6mm thick boards for the leds. Price/board = $6.84. This price doesn't include the 5 Cree XT-E LEDs or the connector cable soldered on.

Files: https://github.com/justbuchanan/oasis/tree/main/pcb/fabricated-boards/ledboard-v0.3

#### Pricing

- total: $35
    - merchandise total: $14
    - shipping: $18
    - sales tax: $3

### v0.2 mainboard - March 21, 2025

Files: https://github.com/justbuchanan/oasis/tree/main/pcb/fabricated-boards/mainboard-v0.3

5 Control boards. Fully assembled except the 2 4-pin headers. Price/board = $40.6.

- total: $203
    - merchandise total: $153
    - shipping: $29
    - sales tax: $21

## Power Consumption

Power consumption of the electronics was measured using a bench power supply set to 18V. Here are the results:

| Configuration                                                   | Current Draw (amps) |
| --------------------------------------------------------------- | ------------------- |
| Microcontroller plugged in, everything (mist, fans, lights) off | 0.01-0.03           |
| LEDs 100% on                                                    | 0.61                |
| Fans on                                                         | 0.09                |
| Mister on, water present                                        | 0.05                |
| Mister on, no water present                                     | 0.03                |
| Everything on (mister, fans, LEDs @ 100%), water present        | 0.72                |
