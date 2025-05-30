+++
title = '3D Model'
weight = 5
+++

<script type="module" src="https://ajax.googleapis.com/ajax/libs/model-viewer/4.0.0/model-viewer.min.js"></script>

<model-viewer id="model"
              src="oasis.glb"
              alt="Oasis Terrarium 3D Model"
              field-of-view="8deg"
              camera-orbit="90deg"
              camera-controls
              touch-action='pan-y'
              ar
              style="width: 100%; height: 1000px;">
</model-viewer>

Controls:

- Scroll to zoom
- left-click drag to rotate
- right-click drag to pan

<script>
const modelViewer = document.getElementById('model');

modelViewer.addEventListener('load', () => {
    console.log("played");
  modelViewer.play({repetitions: 1});
});
</script>

<!-- Creating the glb file:
1. export from cadquery by running `make build/cadquery/oasis.glb`
2. open blender
3. import oasis.glb
4. Follow this youtube video to create the exploding effect: https://www.youtube.com/watch?v=IC0-9b0Rv2g&ab_channel=GeorgeKayesi
    4.1. create an empty shape (cube or axes) and move it off to the side of the terrarium model. Scale up the cube a bit (maybe 50x) to make it easily visible. Set its z position to zero - that way it's easy to get it back to its start position later.
    4.2. select the tube/walls, then add a "copy position" constraint.
        - only z axis should be applied (uncheck x and 7)
        - check the "offset" box
    4.3. select every shape that isn't the tube/walls. then select the walls (this must be selected last). then copy the "copy position" constraint to selected objects.
        - note: don't select visually and don't select them direct in the tree view on the right side. you need to individually select each part (an extra level deep in the tree, has an upside-down triangle next to it)
5. drag the empty cube upwards until the top-most item in the assembly is where you want it to be.
6. for every part, adjust the copy position influence down so the part is where you want it.
6. TODO: figure out keyframe 1 and keyframe 100
7. export the finished product. this is mostly just the same glb as before, but with an added animation.
    - animation checkbox: checked
    - animation mode: active actions merged
        - note that the default options export a separate animation for each individual object in the assembly. model-viewer works best (or at all?) with one single animation.
        - bake animations box: checked


Glass:
On the right side, click the "Render Properties" Menu
    - Render Engine: set to "Cycles"

Click on acrylic walls, then go to material menu
- Use Principled BSDF
- color should be white
- set Roughness to 0
- IOR set to 1.45
- Transmission: set to 1.0 (or maybe just a hair less?)


Plastic material:
    - Roughness factor: 1.0 -> 0.75
    - metallic factor: 1.0 -> 0.5
 -->
