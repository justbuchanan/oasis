+++
title = 'Electronics'
weight = 4
+++

# Electronics

## How to get them?

There are essentially three options for obtaining the electronics:

- Order bare PCBs (from OSH Park, JLCPCB, PCBWay) and parts (digikey, mouser, etc) and solder them yourself. This can be a good option if you have experience with SMD soldering.
- Order fully-assembled PCBs from JLCPCB. This can be a good option if you'd prefer not to solder the boards yourself, but it can be expensive for small order quantities.
- [Sign up to buy a kit](/docs/contact/#buying-a-kit) if/when they become available. I don't have any immediate or concrete plans to sell these, but will consider it if there's enough interest.

See the [PCB Ordering Guide](#pcb-ordering-guide) below for more info.

## Design

### Overview

The electronics have a few main tasks:

- power the lights and allow for brightness control ("dimming")
- turn the fans on/off
- drive the mister (this is a somewhat complicated resonant circuit)
- communicate with the temperature+humidity sensor
- wifi
- usb port for programming/debugging

### Microcontroller

Requirements:

- i2c for sensors ([sht30](https://www.digikey.com/en/products/detail/sensirion-ag/SHT30-DIS-B2-5KS/5872250))
- 1 pwm for LED driver
- 1 pwm for mist
- 1 gpio for fans
- wifi

While there are a quite a few microcontrollers that fit the requirements, they vary widely in size and cost. I selected the esp32 due its small size (smaller than a postage stamp), SMD footprint (can be soldered directly to a board), and low cost (~$3).

### LEDs + Driver

I chose [Cree XT-E LEDs](https://downloads.cree-led.com/files/ds/x/XLamp-XTE.pdf) as these are common in the aquarium and terrarium hobbies for building high power lighting and because I have prior experience with them.

A string of LEDs is best powered by a constant-current driver. There are a lot of options here, but I selected the [Meanwell LDD-700LS](https://www.digikey.com/en/products/detail/mean-well-usa-inc/LDD-700LS/7704762) due to its small SMD footprint, dimming control via pwm, and ease of use (no external components required). This driver can provide up to 700mA (hence the 700 in the name), but there are several variants that can provide more or less current.

I chose to go with five LEDs laid out in a circle at the top of the terrarium. The circular spacing gives a more even lighting to the enclosure than we'd get with one LED in the center or several bunched closely together. The choice of five LEDs was somewhat arbitrary.

Cree XT-E LEDs have a forward voltage a little under 3V when current is 700mA (see page 21 of the [datasheet](https://downloads.cree-led.com/files/ds/x/XLamp-XTE.pdf)). With five LEDs in series, this means we need a total voltage of ~15V across the LEDs. The driver input voltage must be around 2v higher than its output voltage, giving us a minimum of ~17v to power the driver.

### Mister

I'm using an ultrasonic mister to provide watering in the terrarium. These are small discs made of a metal+piezo material sandwich with very tiny (~4uM) holes in the center. By applying an alternating voltage across the terminals of the disc, we can cause it to vibrate and "chop up" water into tiny particles that descend into the enclosure. The closer we can drive the disc to its natural resonant frequency (commonly ~110kHz), the faster and more efficiently we can drive it.

I heavily relied on these articles when designing the resonant circuit for driving the mister:

- https://www.edn.com/ultrasonic-mist-maker/.
- https://www.instructables.com/Make-Your-Own-Super-Simple-Ultrasonic-Mist-Maker/

Essentially a mosfet is switched on and off very quickly (~110kHz) causing it to alternate between "charging up" an inductor and dumping that charge across the misting disc. The mosfet is connected to a mosfet gate driver, which receives the 110kHz signal from a pwm output on the microcontroller.

Some mist driver circuits use a dedicated square wave generating IC like a 555 timer to provide the 110kHz signal, however this design uses a pwm output from the microcontroller. This has a couple advantages:

- less components
- the signal frequency can be changed in software to accommodate different misting discs

And one key disadvantage: it is possible to write firmware that outputs an "always on" signal rather than the expected 110kHz square wave, which will cause the misting circuit to let out the magic smoke.

### Voltage Requirements

- LED driver: >17v
- sensors: 2.2v - 5.5v
- esp32: 3.3v
- fans: nominally 24v, but can run at 18v
- mist: >12v

I chose to use an input voltage supply of 18v in order to satisfy the requirements of the higher-voltage devices, then include a voltage regulator to provide 3.3v for the lower-voltage devices. I selected a buck converter from TI and used their WeBench tool to design a regulator circuit. See details [here](https://github.com/justbuchanan/oasis/blob/main/pcb/reference/voltage-circuit-export-from-ti-webench.pdf).

### Thermal Considerations

It turns out that LEDs, even fairly efficient ones like we're using here, produce a lot of heat.

The initial design of the electronics placed all components, including the LEDs and the temperature+humidity sensor on one board. I anticipated that this might have issues with overheating, but the elegance of having everything on one board was too hard to resist, so I stubbornly pushed forward. Below are renders of the initial design:

<!-- Show the front and back renders side-by-side -->
<div class="img-row">
  <div class="img-column2">
    <img src="old_pcb_front3d.png"/>
    <p>Top View</p>
  </div>
  <div class="img-column2">
    <img src="old_pcb_back3d.png"/>
    <p>Bottom View</p>
  </div>
</div>

Predictably, the LEDs quickly caused the board to heat up, throwing off the temperature sensor readings and bringing the esp32 close to its operating limits. I attempted to mitigate the overheating by drilling lots of ventilation holes in the 3d-printed top of the enclosure and sticking a dozen small heat sinks to the top of the circuit board. This helped a bit, but not enough. The only way to make this work would be to significantly limit the max LED brightness in firmware, leaving the lighting levels significantly below what I had hoped for the terrarium.

Back to the drawing board! The final design opts to place the LEDs on their own board made out of aluminum to act as a heat sink. Almost everything else is placed on a separate mainboard. The temperature+humidity sensor is on its own board that sits inside the enclosure, thermally isolating it from the heat of the LEDs.

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

In order to connect to the mainboard, the ledboard needs a 2-pin JST PH connector (for example https://www.amazon.com/dp/B01DUC1O68) soldered onto it.

### Schematic

![schematic](ledboard_schematic.png)

### 3D Render

![front](ledboard_front3d.png)

## SHT30 Sensor Board Wiring

The sht30 sensor board comes without a cable. You'll need to solder a standard 4-pin connector cable in order to plug it into the mainboard.

![sht30_wiring](sht30_wiring.jpg)

## PCB Order History

Below are all three circuit board orders I've placed for this project (as of June 2025). All boards were ordered from JLCPCB and include varying levels of assembly (components sourced and soldered by JLCPCB).

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

This order was for five boards, so price/board was $28 + ~$17 = ~$45.

### v0.2 ledboard - March 16, 2025

Based on the poor thermal performance of the all-in-one boards, I decided to do a major redesign and put the leds on their own board, made out of solid aluminum. The rest of the electronics are on their own board.

This order is for 5x 1.6mm thick aluminum boards for the leds. Price/board = $6.84. This price doesn't include the 5 Cree XT-E LEDs or the connector cable soldered on.

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

## PCB Ordering Guide

There are a lot of options for custom PCB manufacturing. I chose to go with JLCPCB due to low cost, support for board assembly, and extensive component catalog. PCBWay is another good option that also provides assembly services. OSH Park is a good choice for US-based manufacturing, but they don't offer assembly services.

### PCB Production Files

To order custom PCBs, you'll need the production files to send to the manufacturer. If you're ordering bare/un-assembled boards, you'll just need the [gerber files](https://en.wikipedia.org/wiki/Gerber_format) (typically bundled into a `gerbers.zip`), which are essentially vector image files that specify things like where copper traces are located on either side of the board, silkscreen text and images on either side of the board, etc.

If you're looking to order assembled boards, you'll additionally need two more files:

- `bom.csv`: a bill of materials file, which maps from component ids in the board design to specific part numbers in the manufacturer's part catalog
- `pos.csv`/`positions.csv`: file which records the location and orientation of each part

All three production files are generated from the KiCad design files using the [Fabrication-Toolkit plugin](https://github.com/bennymeg/Fabrication-Toolkit). This plugin can be invoked from the KiCad user interface or from the command line. The command line invocation for the plugin is contained in this project's [makefile](https://github.com/justbuchanan/oasis/blob/fbd0d2fceae36f7a195ebd945a816dcebf819d9c/makefile#L142-L147), which you can run with `make pcb`. This will generate the production files for both boards in `build/pcb/main` and `build/pcb/ledboard`. If you make modifications to the board in the KiCad Schematic Editor, run "Tools -> Update PCB From Schematic", then rerun `make pcb` to regenerate the files.

While you can check out the `main` branch of the project in git and use the KiCad files directly, it is recommended to use a tagged release which has been vetted for correctness. It's possible that as the project evolves, changes will be made to the KiCad files that haven't yet been tested.

Tagged releases can be found on the project's [Releases Page on GitHub](https://github.com/justbuchanan/oasis/releases). Alternatively, you can use the design files in the [pcb/fabricated-boards](https://github.com/justbuchanan/oasis/tree/main/pcb/fabricated-boards) directory, which is an archive of all boards ordered to-date.

### Notes On Assembly and Component Selection

JLCPCB has an extensive [catalog of parts](https://jlcpcb.com/parts) they stock in their warehouse(s) and many of the components selected for this project were specifically chosen based on their availability at JLCPCB. There are a couple parts in this design that JLCPCB doesn't stock (including the LED Driver and LEDs) and need to be handled separately. Furthermore, the parts inventory is constantly in flux and it's possible that certain parts that were in stock previously are not in stock when you're placing an order. There are a few options here:

- Skip assembly of those parts, order them separately from somewhere like digikey, then solder them yourself at home.
- Use JLCPCB's pre-order service, which allows you to order almost any part from another supplier. It will take a week or two for the parts to arrive at JLCPCB, after which point you can submit your board order.
- Select a different/equivalent part from JLCPCB's catalog that they do have in stock. This will require you updating the "LCSC Part #" field of the relevant component in KiCad, then regenerating and re-uploading the `bom.csv` file.

#### Overhead Costs and "Basic" vs "Extended" Parts

JLCPCB uses large [pick-and-place machines](https://en.wikipedia.org/wiki/Pick-and-place_machine) for board assembly. These machines have a number of component "reels" attached to them, which hold things like resistors, capacitors, connectors, etc. For commonly-used parts, such as a 1k 0805 resistor, JLCPCB has selected a specific model of the part that they keep loaded up in each of their machines. JLCPCB calls these "Basic Parts" and charges no assembly fee for them since they're already loaded into every machine. When selecting parts, try and find a "Basic" part that fits the bill in order to save money. Any non-"Basic" (i.e. "Extended") parts that require assembly will cost you a $3 setup fee per part. If you're ordering a large number of boards, this is a small cost. If you're only ordering a few, these assembly costs add up.

### JLCPCB Ordering Step-by-Step

Below is a step-by-step guide to placing a board order through JLCPCB. This will focus on the main board, but the steps are largely the same for the LED board. The main differences with the LED board are that the board should be made of aluminum (to act as a heat sink) instead of the standard FR4 and that only one side of the board requires assembly.

1. Go to https://jlcpcb.com and login
1. Upload gerbers.zip
1. (optional) select quantity - default is 5
1. (optional) "Surface Finish": select "LeadFree HASL". Costs like $4 extra on a 5-board order, so why not skip the lead.
1. If you're just ordering bare PCBs, you're done! Finish up the checkout process and place your order.
1. Toggle "PCB Assembly" ON
1. To assembly both sides of the mainboard, change "PCBA Type" from "Economic" to "Standard", then change "Assembly Side" to "Both Sides"
1. (optional) Enable "Confirm Parts Placement" and "Confirm Production file" - it's cheap, so why not have them double-check things?
1. (optional) Adjust assembly quantity. Default is 5, but you could for example order 5 total boards and have only 2 of them assembled.
1. Click the "Next" button
1. Take a quick look over the board images, then click "Next" again
1. Upload "bom.csv" and "positions.csv", then click the "Process" button
1. You're now presented with the bill of materials. If any of the parts say "shortfall" in the right-most column, it means JLCPCB doesn't have those parts. See [Notes on Assembly and Component Selection](#notes-on-assembly-and-component-selection) above.
1. Click "Next" to move on to the assembly review page
1. Click "Next" to move on to the "Quote & Order" Page.
1. Select a "Product Description" from the dropdown. This has tax/tariff/legal implications, but probably doesn't matter much for us. I've been selecting "Other" and typing in "Terrarium for Plants".
1. "Save to Cart" and continue on to checkout
1. See if you can find any relevant coupons: https://jlcpcb.com/coupon-center. There's likely a $15 off coupon for SMT orders over $150. Maybe also a new customer coupon.
1. Finish the checkout process

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
