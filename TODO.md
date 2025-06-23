- [ ] stop hardcoding values in the gpu
- [ ] figure out why performance is not the same after resetting simulation
- [ ] make dispatches out of math instead of sending an array
- [ ] improve frame timings
- [ ] SoA to promote SIMD
- [ ] allow bins bigger than a particle
- [ ] allow rectangular display, not just squared
- [ ] allow saving particle positions to file and loading them from file
- [ ] switch template using UI
- [ ] check if instance data (colors) are being updated every frame (they should not be)
- [ ] gif loading. send instance data as u8 and use vertex shader to convert to f32
- [ ] save particle UV for each template to a file

IMPROVEMENTS
- [ ] improve dispatching, see report
- [ ] parallel reduce to count number of particles per bin on the gpu

<!-- - [ ] keep spawners in two arrays, to remove them if they went over their particle limit -->
