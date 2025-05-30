# Oasis PCB Design

## KiCad Plugins

This project uses a plugin for laying out components. Some values are defined in /oasis_constants.py, then used in both the CAD model and the PCB layout.

### Component layout plugin

The plugin is located at https://github.com/justbuchanan/kicad_component_layout/blob/versionfix/component_layout_plugin.py and must be copied into Kicad's plugin directory in order to use it.

Usage:

- run pcb/{main, ledboard}/layout.py, which generates a layout.yaml output.

- In Kicad's pcb editor, click "Tools" -> "External Plugins" -> "Layout footprints from layout.yaml".

To edit placement of items, modify `oasis_constants.py` and/or `pcb/{main, ledboard}/layout.py`.
