#!/bin/bash

set -euxo pipefail

crate=bl602-hal

# remove existing blobs because otherwise this will append object files to the old blobs
rm -f bin/*.a

riscv64-unknown-elf-gcc -ggdb3 -fdebug-prefix-map=$(pwd)=/$crate -c -mabi=ilp32 -march=rv32i trap.S -o bin/$crate.o
riscv64-unknown-elf-ar crs bin/trap_riscv32i-unknown-none-elf.a bin/$crate.o

riscv64-unknown-elf-gcc -ggdb3 -fdebug-prefix-map=$(pwd)=/$crate -c -mabi=ilp32f -march=rv32if trap.S -o bin/$crate.o
riscv64-unknown-elf-ar crs bin/trap_riscv32if-unknown-none-elf.a bin/$crate.o

rm bin/$crate.o
