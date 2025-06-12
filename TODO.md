- [ ] keep spawners in two arrays, to remove them if they went over their particle limit
- [ ] figure out why functions in common.rs are working without padding, if the rows and cols passed in all rely on padding. compute shader should have the same issue too
- [ ] implement gpu binning

- [x] see common.rs. fix not using the padding for calculations


IMPROVEMENTS
- [ ] gpu can do everything except the binning (gravity, update, etc)
- [ ] improve dispatching, see drawing
