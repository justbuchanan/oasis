+++
title = 'Hacking'
weight = 6
+++

# Hacking

This is an open-source project built on open-source tools. It uses [KiCad](https://www.kicad.org/) for electronics design, [CadQuery](https://github.com/CadQuery/cadquery) for 3d-modeling, and [rust](https://www.rust-lang.org/) for the software.

## Code

See the readme for more info: https://github.com/justbuchanan/oasis/blob/main/code/README.md

## CAD models

The CAD models for this project are designed in [CadQuery](https://github.com/cadquery/cadquery), which uses python for part design. This project's [requirements.txt file](https://github.com/justbuchanan/oasis/blob/main/requirements.txt) contains the relevant python dependencies, but you may need to consult the CadQuery documentation for further installation instructions.

Note that while the python environment is _mostly_ for working with the CAD models, there are some optional python dependencies needed for working with the KiCad electronics designs. These include the [Fabrication-Toolkit plugin](https://github.com/bennymeg/Fabrication-Toolkit) for exporting manufacturing files for JLC PCB and the [component layout plugin](https://github.com/justbuchanan/kicad_component_layout).

### Python Environment Setup

#### Initial setup (do once)

```sh
cd <this project>
mkdir -p pyvenv
python -m venv ./pyvenv --system-site-packages
source ./pyvenv/bin/activate
pip install -r requirements.txt
```

#### Environment (do every time)

Run this in your shell before you do anything related to python, including running `make`.

```sh
cd <this project>
source ./pyvenv/bin/activate
```

## Command-Line Client

This project comes with a command-line client that can:

- configure wifi credentials
- set the terrarium's schedule (lights, fans, mister)
- query the terrarium's state (sensors, light level, etc)
- control lights, fans, mister
- scan the network for online terrariums

Source code is at https://github.com/justbuchanan/oasis/tree/main/code/client.

For full documentation, build the client and run `oasis --help`.
