- [x] button for simulation reset
- [x] button to set colors
- [ ] fix bottom left acting as a black hole
- [ ] make a new function that appends particles
- [ ] fix cpu binning to have an extra layer at the edges, see the FIXME:
- [ ] implement gpu binning







no particles are colliding with particles that are on the bottom left
sometimes NaNs appear on bin 300, wich is the 4th row and last column, due to distance being 0
but I have checked and they are different particles, it's just that they are on the exact same position, so I'll just add a check for nan
