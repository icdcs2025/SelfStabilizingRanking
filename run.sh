#!/bin/sh

cargo run --release -- dupe 256 > run_256.csv
cargo run --release -- geom > const_fraction.csv
python3 plot.py run_256.csv 256
python3 plot_geom.py const_fraction.csv
