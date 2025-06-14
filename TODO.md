- [ ] stop hardcoding values in the gpu
- [ ] keep spawners in two arrays, to remove them if they went over their particle limit
- [ ] figure out why functions in common.rs are working without padding, if the rows and cols passed in all rely on padding. compute shader should have the same issue too
- [ ] gpu should do everything except the binning (gravity, update, constraint, collisions)

IMPROVEMENTS
- [ ] improve dispatching, see drawing
