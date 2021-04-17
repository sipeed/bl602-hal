set crate=bl602-hal

riscv64-unknown-elf-gcc -ggdb3 -fdebug-prefix-map=%cd%=/%crate% -c -mabi=ilp32 -march=rv32i asm.S -o bin/%crate%.o
riscv64-unknown-elf-ar crs bin/riscv32i-unknown-none-elf.a bin/%crate%.o

riscv64-unknown-elf-gcc -ggdb3 -fdebug-prefix-map=%cd%=/%crate% -c -mabi=ilp32f -march=rv32if asm.S -o bin/%crate%.o
riscv64-unknown-elf-ar crs bin/riscv32if-unknown-none-elf.a bin/%crate%.o

del bin\%crate%.o
