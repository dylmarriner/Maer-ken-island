# Deviation register

Known departures from reality. Rules and statuses are in [`REALISM.md`](REALISM.md) §4. Every row
names its owning phase (`0`, `0b`, `0c`, `1`, `2`, `3`, `4`, `4b`, `5`); the test
`crates/mk_core/tests/deviation_register.rs` enforces the format.

| ID | Reality | Model | Expected error | Why | Phase | Status |
|---|---|---|---|---|---|---|
| D1 | A rocky planet of 19,113 km radius would have roughly 6–7 g | Earth gravity (9.81 m/s²) at that radius; bulk density 1,835 kg/m³ | The planet's mass, density and interior are not self-consistent; surface physics is Earth-like | Owner decision: keep Marr'Kena's size with gravity real humans can live in. Crust and mantle use Earth-like rock | 0c | Accepted |
| D2 | Upstream canon year and moon orbits follow Kepler's law | Upstream canon year is 24% off Kepler; both moons' distances disagree with their periods | — (island canon is consistent) | Inherited upstream data; island uses its own canon (`fixtures/island/canon.json`) | 0c | Resolved |
| D3 | Temperatures swing between day and night | Upstream climate is a daily mean | No diurnal temperature range, sea breezes or valley inversions | Inherited model; Phase 2 Task 3 adds the diurnal cycle | 2 | Open |
| D4 | Mid-latitude weather is travelling fronts, highs and lows | Upstream weather is diagnostic from climate | No storms, frontal rain bands or spells of settled weather | Phase 2 Task 3b adds synoptic systems | 2 | Open |
| D5 | Convection, thunderstorms and sea breezes have scales of 1–10 km | 12 km atmosphere grid with parameterised convection | Local convective rain and small-scale winds are averaged | Resolution limit chosen for performance | 2 | Accepted |
| D6 | Small streams, hillslopes and gullies are tens to hundreds of metres | 2 km land grid (5 m inside high-detail patches) | Small catchments and slope processes are averaged outside patches | Resolution limit chosen for performance | 5 | Accepted |
| D7 | Earth's species | Species evolved for Marr'Kena by the upstream biosphere | Species identities differ; functional traits are validated against Earth allometry and metabolism instead | Marr'Kena is not Earth; realism is judged on biophysics, not names | 3 | Accepted |
| D8 | The human body clock free-runs at ~24.2 h and cannot entrain to a 36 h day | Upstream circadian hormone follows daylight directly | Humans adapt perfectly to the 36 h day | Inherited model; fixed upstream in Phase 0c Task 4 | 0c | Open |
| D9 | Work takes hours and burns energy | Upstream actions are instant and cost nothing | Productivity and hunger are wrong | Phase 3 Task 3b adds timed, MET-costed work | 3 | Open |
| D10 | Harvesting, burning and eating move carbon, oxygen and water | Upstream economy moves no carbon, oxygen or water; nodes regenerate from nothing | Carbon and water budgets do not close around human activity | Phase 3 Tasks 2–3 make materials physical | 3 | Open |
| D11 | Active margins have earthquakes | No earthquakes in the engine | No seismic hazard or co-seismic uplift | Phase 1 Task 3b adds fault stress and earthquakes | 1 | Open |
| D12 | A person holds one conversation at a time | Upstream lets every nearby pair converse simultaneously | Crowds produce thousands of simultaneous conversations (and O(n²) cost) | Phase 0 Task 4c fixes pairing upstream | 0 | Resolved |
| D13 | Machines need power and fuel | Upstream vehicles, power tools and computers need neither | Free energy for property use | Phase 3 Task 4b and Phase 4b add fuel and electricity | 3 | Open |
| D14 | Methane leaks warm the climate | No methane reservoir or forcing | Venting and leaks have no climate effect | Phase 4b Task 4 adds `AtmosCH4` and its forcing | 4b | Open |
| D15 | Turbines, generators, rigs and refinery units are manufactured in industrial economies | They exist at scenario start, imported before the simulation began, with finite spares | The island could not build them itself | No industrial base exists on the island | 4b | Accepted |
| D16 | Fuel for an isolated community must be produced or imported | Until Phase 4b, estate fuel is a finite store with no production route | Fuel eventually runs out | Phase 4b adds the oil field and refinery | 4b | Open |
| D17 | An isolated island has no internet | The opt-in computer service bridges humans to the real internet | Humans can search and email the real world | Owner's existing upstream feature, kept opt-in and outside determinism | 5 | Accepted |
| D18 | Money in a real economy is issued and managed over time | A fixed money supply is issued once at scenario start | No inflation, credit creation or monetary policy | Scope; recorded so it is not mistaken for realism | 4b | Accepted |
| D19 | Oil and gas production needs drilling rigs to develop new wells | Until Phase 4b no recipe makes a rig; oil and gas deposits exist but cannot be extracted | Fossil fuels are unavailable before Phase 4b | Phase 4b provides the field development | 4b | Open |
