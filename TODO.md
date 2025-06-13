- [ ] keep spawners in two arrays, to remove them if they went over their particle limit
- [ ] figure out why functions in common.rs are working without padding, if the rows and cols passed in all rely on padding. compute shader should have the same issue too
- [ ] implement gpu binning
    - [ ] there are still issues in the bottom left
        the position goes 30 29 27 25 23 21 ......, meaning it is going down twice
        - [x] bins are repeated in the dispatches
        - [x] the particles are in more than one bin at once
        - [x] basic cpu test
        - [x] advanced cpu test, using the id from the workgroup, the number of dispatches, etc
        - [ ] thread goes over its bins more than once
        - [ ] data race when copying the data over?
        - [ ] accessing dispatch data on the gpu



IMPROVEMENTS
- [ ] gpu can do everything except the binning (gravity, update, etc)
- [ ] improve dispatching, see drawing
