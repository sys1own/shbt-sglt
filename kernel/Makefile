# shbt-os microkernel build rules (sglt.txt §1).
#  - shbt_kernel.elf: freestanding image linked through linker.ld, enforcing
#    the exact 2,112-byte SRAM .stinespring_frame allocation (Gate-13).
#  - shbt_reference.so: hosted dynamic interface library consumed by the
#    sglt-hil-microkernel FFI surface and the Python CLI.

CC      ?= gcc
CFLAGS  ?= -O3 -mavx512f -nostdlib -ffreestanding -fno-stack-protector \
           -fPIC -Iinclude -std=c11 -DSHBT_BARE_METAL

BIN     := ../crates/sglt-hil-microkernel/bin
OBJS    := build/shbt_core_runtime.o build/shbt_ecc_avx512.o

all: $(BIN)/shbt_reference.so build/shbt_kernel.elf

build:
	mkdir -p build $(BIN)

build/%.o: src/%.c include/shbt_hardware.h | build
	$(CC) $(CFLAGS) -c $< -o $@

build/shbt_kernel.elf: $(OBJS) linker.ld
	$(CC) $(CFLAGS) -static -Wl,-T,linker.ld $(OBJS) -o $@

$(BIN)/shbt_reference.so: $(OBJS)
	$(CC) $(CFLAGS) -nostdlib -shared $(OBJS) -o $@

clean:
	rm -rf build $(BIN)/shbt_reference.so

.PHONY: all clean
