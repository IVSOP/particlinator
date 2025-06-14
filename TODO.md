- [ ] stop hardcoding values in the gpu
- [ ] gpu should do everything except the binning (gravity, update, constraint, collisions)
- [ ] add some frinction or change the restitution to stop particles from moving too much under pressure
- [ ] figure out why performance is not the same after resetting simulation
- [ ] keep spawners in two arrays, to remove them if they went over their particle limit
- [ ] figure out why functions in common.rs are working without padding, if the rows and cols passed in all rely on padding. compute shader should have the same issue too

IMPROVEMENTS
- [ ] improve dispatching, see drawing
