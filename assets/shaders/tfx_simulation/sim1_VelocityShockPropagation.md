Propagate velocity shock resulted by attached based mesh. One thread computes one strand.

1. Calculate (rotation + translation) that root strand vertex was subjected to
2. Propagate (rotation + translation) from parent to child vertices in same strand, using `vspCoeff` (set as uniform in simulation settings) as a weight
3. Write values to `g_HairVertexPositions`, `g_HairVertexPositionsPrev` for child vertices

# Why it's not implemented

Since VSP is used to propagate base mesh (head) movement, and this demo does not have any, it's pointless to implement it here.


# Original code

* My TressFX-OpenGL: https://github.com/Scthe/TressFX-OpenGL/blob/master/src/shaders/gl-tfx/sim1_VelocityShockPropagation.comp.glsl
* AMD: https://github.com/GPUOpen-Effects/TressFX/blob/ba0bdacdfb964e38522fda812bf23169bc5fa603/src/Shaders/TressFXSimulation.hlsl#L740
  * Not sure why the AMD version looks like this it seems **CLEARLY** wrong (code is per vertex, comment says per strand, code does not compute what is says it does etc.).
