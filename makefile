.PHONY: all cad cadquery clean gcode code pcb-review mainboard ledboard

all: cad pcb code

cad: cadquery build/pcb/main/pcb.step build/pcb/ledboard/ledboard.step
pcb: mainboard ledboard
code: build/code/oasis build/code/firmware build/code/demoserver

cadquery: build/cadquery/top.step build/cadquery/bottom.step build/cadquery/underplate.step build/cadquery/top_plug.step build/cadquery/mister_mount_cover.step build/cadquery/sensor_basket.step build/cadquery/oasis.glb
gcode: build/cadquery/top.gcode build/cadquery/bottom.gcode build/cadquery/underplate.gcode build/cadquery/top_plug.gcode build/cadquery/sensor_basket.gcode
mainboard: build/pcb/main/schematic.pdf build/pcb/main/pcb.pdf build/pcb/main/pcb.step build/pcb/main/drc_results.txt build/pcb/main/gerbers.zip build/pcb/main/bom.csv build/pcb/main/pos.csv build/pcb/main/pcb_back.svg build/pcb/main/pcb_front.svg build/pcb/main/pcb_in1.svg build/pcb/main/pcb_in2.svg build/pcb/main/pcb_frontback.svg build/pcb/main/pcb_front3d.png build/pcb/main/pcb_back3d.png
ledboard: build/pcb/ledboard/schematic.pdf build/pcb/ledboard/ledboard.pdf build/pcb/ledboard/ledboard.step build/pcb/ledboard/drc_results.txt build/pcb/ledboard/gerbers.zip build/pcb/ledboard/bom.csv build/pcb/ledboard/pos.csv build/pcb/ledboard/ledboard_front.svg build/pcb/ledboard/ledboard_frontback.svg build/pcb/ledboard/ledboard_front3d.png

clean:
	rm -r build || true
	cd code/client && cargo clean || true
	cd code/demoserver && cargo clean || true
	cd code/esp32 && cargo clean || true
	cd code/terralib && cargo clean || true
	rm -r pcb/main/production || true
	rm -r pcb/ledboard/production || true


# CadQuery
################################################################################

build/cadquery/top.step: cadquery/oasis.py cadquery/util.py oasis_constants.py
	@mkdir -p $(@D)
	PYTHONPATH=./ cq-cli --infile cadquery/oasis.py --expression="Top(TopProfile(),vent_holes=True).shape" --outfile $@

build/cadquery/underplate.step: cadquery/oasis.py cadquery/util.py oasis_constants.py
	@mkdir -p $(@D)
	PYTHONPATH=./ cq-cli --infile cadquery/oasis.py --expression="Underplate(Top(TopProfile())).shape" --outfile $@

build/cadquery/bottom.step: cadquery/oasis.py cadquery/util.py oasis_constants.py
	@mkdir -p $(@D)
	PYTHONPATH=./ cq-cli --infile cadquery/oasis.py --expression="Bottom(BottomProfile()).shape" --outfile $@

build/cadquery/top_plug.step: cadquery/oasis.py cadquery/util.py oasis_constants.py
	@mkdir -p $(@D)
	PYTHONPATH=./ cq-cli --infile cadquery/oasis.py --expression="TopPlug().shape" --outfile $@

build/cadquery/mister_mount_cover.step: cadquery/oasis.py cadquery/util.py oasis_constants.py
	@mkdir -p $(@D)
	PYTHONPATH=./ cq-cli --infile cadquery/oasis.py --expression="mister_mount_cover().rotate_x(180)" --outfile $@

build/cadquery/sensor_basket.step: cadquery/oasis.py cadquery/util.py oasis_constants.py
	@mkdir -p $(@D)
	PYTHONPATH=./ cq-cli --infile cadquery/oasis.py --expression="SensorBasket(Sht30Board()).shape" --outfile $@

# build/cadquery/oasis.gltf: cadquery/oasis.py cadquery/util.py oasis_constants.py build/pcb/main/pcb.step build/pcb/ledboard/ledboard.step
# 	@mkdir -p $(@D)
# 	PYTHONPATH=./ cq-cli --infile cadquery/oasis.py --expression="terrarium()" --outfile $@

build/cadquery/oasis.glb: cadquery/oasis.py cadquery/util.py oasis_constants.py build/pcb/main/pcb.step build/pcb/ledboard/ledboard.step
	@mkdir -p $(@D)
	PYTHONPATH=./ cq-cli --infile cadquery/oasis.py --expression="terrarium(explode=0, vent_holes=True)" --outfile $@


# GCode for 3d printing
# NOTE: I don't actually use these gcode files directly - I use a gui slicer
# (PrusaSlicer), but these are useful for getting estimates on filament usage,
# print times, etc.
################################################################################

SLICER_FLAGS = --export-gcode --filament-type PLA --brim-type outer_only

build/cadquery/top.gcode: build/cadquery/top.step
	prusa-slicer $(SLICER_FLAGS) $^ --output $@

build/cadquery/bottom.gcode: build/cadquery/bottom.step
	prusa-slicer $(SLICER_FLAGS) $^ --output $@

build/cadquery/underplate.gcode: build/cadquery/underplate.step
	prusa-slicer $(SLICER_FLAGS) $^ --output $@

build/cadquery/top_plug.gcode: build/cadquery/top_plug.step
	prusa-slicer $(SLICER_FLAGS) $^ --output $@

build/cadquery/mister_mount_cover.gcode: build/cadquery/mister_mount_cover.step
	prusa-slicer $(SLICER_FLAGS) $^ --output $@

build/cadquery/sensor_basket.gcode: build/cadquery/sensor_basket.step
	prusa-slicer $(SLICER_FLAGS) $^ --output $@


.PHONY: printtime
printtime: build/cadquery/bottom.gcode build/cadquery/top.gcode build/cadquery/underplate.gcode build/cadquery/top_plug.gcode build/cadquery/mister_mount_cover.gcode build/cadquery/sensor_basket.gcode
	grep "estimated printing time (normal" $^


# KiCad main board
################################################################################

build/pcb/main/schematic.pdf: pcb/main/pcb.kicad_sch
	@mkdir -p $(@D)
	kicad-cli sch export pdf --output $@ $^

build/pcb/main/pcb.pdf: pcb/main/pcb.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli pcb export pdf -l F.Cu,F.Mask,F.Silkscreen,F.Courtyard,Edge.Cuts,B.Cu,B.Mask,B.Silkscreen,B.Courtyard --output $@ $^

# TODO: include courtyard and mask?
# TODO: not happy with any of these svg outputs

build/pcb/main/pcb_front.svg: pcb/main/pcb.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli pcb export svg --exclude-drawing-sheet -l F.Mask,F.Silkscreen,F.Cu,F.Courtyard,Edge.Cuts --output $@ $^

build/pcb/main/pcb_in1.svg: pcb/main/pcb.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli pcb export svg --exclude-drawing-sheet -l In1.Cu,Edge.Cuts --output $@ $^

build/pcb/main/pcb_in2.svg: pcb/main/pcb.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli pcb export svg --exclude-drawing-sheet -l In2.Cu,Edge.Cuts --output $@ $^

build/pcb/main/pcb_back.svg: pcb/main/pcb.kicad_pcb
	@mkdir -p $(@D)
	# TODO: --mirror
	kicad-cli pcb export svg --exclude-drawing-sheet -l B.Mask,B.Silkscreen,B.Cu,B.Courtyard,Edge.Cuts --output $@ $^

build/pcb/main/pcb_frontback.svg: pcb/main/pcb.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli pcb export svg --exclude-drawing-sheet -l B.Cu,B.Courtyard,B.Silkscreen,F.Cu,F.Courtyard,F.Silkscreen,Edge.Cuts --output $@ $^

PCB_RENDER_ARGS=--quality high --perspective --floor --preset=follow_pcb_editor

# Consider adding --include-tracks --include-zones
# It looks cooler with them, but it takes longer to process and results in a larger / more complicated output
build/pcb/main/pcb.step: pcb/main/pcb.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli pcb export step --subst-models --output $@ $^

build/pcb/main/pcb.glb: pcb/main/pcb.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli pcb export glb --include-pads --include-silkscreen --include-soldermask --include-tracks --include-zones --subst-models -o $@ $^

build/pcb/main/pcb_front3d.png: pcb/main/pcb.kicad_pcb
	kicad-cli pcb render --side top --rotate ' -10,0,-90' $(PCB_RENDER_ARGS) --output $@ $^

build/pcb/main/pcb_back3d.png: pcb/main/pcb.kicad_pcb
	kicad-cli pcb render --side bottom --rotate ' -10,0,-90' $(PCB_RENDER_ARGS) --output $@ $^


# TODO: don't hardcode PYTHONPATH
build/pcb/main/gerbers.zip build/pcb/main/bom.csv build/pcb/main/pos.csv &: pcb/main/pcb.kicad_pcb
	mkdir -p build/pcb/main
	PYTHONPATH="/usr/lib/python3.13/site-packages:pcb/Fabrication-Toolkit:${PYTHONPATH}" python -m plugins.cli --path $^ --autoTranslate --autoFill
	cp pcb/main/production/bom.csv build/pcb/main/bom.csv
	cp pcb/main/production/positions.csv build/pcb/main/pos.csv
	cp pcb/main/production/Terrarium_Control_Board*.zip build/pcb/main/gerbers.zip

# design rules check
build/pcb/main/drc_results.txt: pcb/main/pcb.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli pcb drc --output $@ $^

.PHONY: main_drc
main_drc: build/pcb/main/drc_results.txt
	cat build/pcb/main/drc_results.txt


# KiCad LED board
################################################################################

build/pcb/ledboard/schematic.pdf: pcb/ledboard/ledboard.kicad_sch
	@mkdir -p $(@D)
	kicad-cli sch export pdf --output $@ $^

build/pcb/ledboard/ledboard.pdf: pcb/ledboard/ledboard.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli  pcb export pdf -l F.Cu,F.Mask,F.Silkscreen,F.Courtyard,Edge.Cuts,B.Cu,B.Mask,B.Silkscreen,B.Courtyard --output $@ $^

# TODO: include courtyard and mask?
# TODO: not happy with any of these svg outputs

build/pcb/ledboard/ledboard_front.svg: pcb/ledboard/ledboard.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli pcb export svg --exclude-drawing-sheet -l F.Silkscreen,F.Cu,F.Courtyard,Edge.Cuts --output $@ $^

build/pcb/ledboard/ledboard_frontback.svg: pcb/ledboard/ledboard.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli pcb export svg --exclude-drawing-sheet -l B.Cu,B.Courtyard,B.Silkscreen,F.Cu,F.Courtyard,F.Silkscreen,Edge.Cuts --output $@ $^

# Consider adding --include-tracks --include-zones
# It looks cooler with them, but it takes longer to process and results in a larger / more complicated output
build/pcb/ledboard/ledboard.step: pcb/ledboard/ledboard.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli pcb export step --subst-models --output $@ $^

build/pcb/ledboard/ledboard.glb: pcb/ledboard/ledboard.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli pcb export glb --include-tracks --include-zones --subst-models -o $@ $^

build/pcb/ledboard/ledboard_front3d.png: pcb/ledboard/ledboard.kicad_pcb
	kicad-cli pcb render --zoom 0.9 --side top --rotate ' -10,0,-90' $(PCB_RENDER_ARGS) --output $@ $^


# TODO: don't hardcode PYTHONPATH
build/pcb/ledboard/gerbers.zip build/pcb/ledboard/bom.csv build/pcb/ledboard/pos.csv &: pcb/ledboard/ledboard.kicad_pcb
	mkdir -p build/pcb/ledboard
	PYTHONPATH="/usr/lib/python3.13/site-packages:pcb/Fabrication-Toolkit:${PYTHONPATH}" python -m plugins.cli --path $^ --autoTranslate --autoFill
	cp pcb/ledboard/production/bom.csv build/pcb/ledboard/bom.csv
	cp pcb/ledboard/production/positions.csv build/pcb/ledboard/pos.csv
	cp pcb/ledboard/production/Terrarium_LED_Board*.zip build/pcb/ledboard/gerbers.zip

# design rules check
build/pcb/ledboard/drc_results.txt: pcb/ledboard/ledboard.kicad_pcb
	@mkdir -p $(@D)
	kicad-cli pcb drc --output $@ $^

.PHONY: drc_ledboard
drc_ledboard: build/pcb/ledboard/drc_results.txt
	cat build/pcb/ledboard/drc_results.txt


# Rust code
################################################################################

# code that runs on esp32
# TODO: none of the below rust targets have their inputs fully specified, so you might change something relevant and make will not rebuild the affected output.
build/code/firmware: code/esp32 code/terralib
	@mkdir -p $(@D)
	cd code/esp32 && cargo build --release --bin oasis
	cp code/esp32/target/riscv32imc-esp-espidf/release/oasis $@

build/code/binsize_info.txt: build/code/oasis
	@mkdir -p $(@D)
	cd code/esp32 && cargo espflash save-image --partition-table partitions.csv --chip esp32c3 --bin oasis /tmp/out.bin > /tmp/info.txt
	cp /tmp/info.txt $@

# build esp32 code, flash it to the mcu connected over usb, reboot it, watch its output.
.PHONY: deploy-to-esp32
deploy-to-esp32:
	cd code/esp32 && cargo run --release --bin oasis

# command-line client
build/code/oasis: code/client code/terralib
	@mkdir -p $(@D)
	cd code/client && cargo build --release
	cp code/client/target/release/client $@

build/code/demoserver: code/demoserver code/terralib
	@mkdir -p $(@D)
	cd code/demoserver && cargo build --release
	cp code/demoserver/target/release/demoserver $@

# note: esp32 directory is omitted below because `cargo test` would attempt to
# run the code on an actual esp32, not on the host computer.
# TODO: this should at least build the esp32 code
.PHONY: test-code
test-code: code
	cd code/client && cargo test
	cd code/demoserver && cargo test
	cd code/terralib && cargo test

.PHONY: cargo-clippy
cargo-clippy:
	cd code/client && cargo clippy
	cd code/demoserver && cargo clippy
	cd code/terralib && cargo clippy
	cd code/esp32 && cargo clippy

.PHONY: cargo-clippy-fix
cargo-clippy-fix:
	cd code/client && cargo clippy --fix --allow-dirty
	cd code/demoserver && cargo clippy --fix --allow-dirty
	cd code/terralib && cargo clippy --fix --allow-dirty
	cd code/esp32 && cargo clippy --fix --allow-dirty

# Other
################################################################################

# check for typos
# Install with `cargo install typos-cli`
.PHONY: typos
typos:
	typos README.md code/README.md pcb/README.md cadquery/* $(shell find code/**/src/ -name "*.rs") website/content/


.PHONY: update-website-photos
update-website-photos: pcb
	convert -density 600 build/pcb/ledboard/schematic.pdf website/content/docs/electronics/ledboard_schematic.png 
	convert -density 600 build/pcb/main/schematic.pdf website/content/docs/electronics/mainboard_schematic.png 
	cp build/pcb/main/pcb_front3d.png website/content/docs/electronics/mainboard_front3d.png
	cp build/pcb/main/pcb_back3d.png website/content/docs/electronics/mainboard_back3d.png
	cp build/pcb/ledboard/ledboard_front3d.png website/content/docs/electronics/ledboard_front3d.png
