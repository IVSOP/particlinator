- [ ] keep spawners in two arrays, to remove them if they went over their particle limit
- [ ] figure out why functions in common.rs are working without padding, if the rows and cols passed in all rely on padding. compute shader should have the same issue too
- [ ] implement gpu binning

ISSUES
    - [ ] dispatch construction seems fine, but there are still issues
        it should have something to do with my binning not taking padding into account (see comment in common.rs)


IMPROVEMENTS
- [ ] gpu can do everything except the binning (gravity, update, etc)
- [ ] improve dispatching, see drawing

FUTURE TESTING
- [ ] instead of 9 arrays of indices, just send the entire bins
- [ ] instead `````, send the indices with the length already there
