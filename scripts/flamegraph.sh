#! /usr/bin/env sh

cargo flamegraph --bin noxious-server --dev -o target/flamegraph.svg

####
# SETUP
# cargo install flamegraph
# echo 0 | sudo tee /proc/sys/kernel/kptr_restrict
# echo 0 | sudo tee /proc/sys/kernel/perf_event_paranoid