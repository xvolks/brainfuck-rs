#!/usr/bin/env bash

usage() {
    reason=$1
    if [[ $reason != "" ]]; then
	echo "ERROR: $reason"
	echo
    fi
    
    echo "Usage: $0 <ops.bin>"
    echo "Transform binary code to executable"
    echo "\tops.bin is a bunch of binary op codes"
    exit 69
}

INPUT_BIN="$1"

if [[ $INPUT_BIN == "" ]]; then
    INPUT_BIN=ops.bin
fi

[[ -f $INPUT_BIN ]] || usage "$INPUT_BIN: File Not Found"

INPUT_BIN=$(readlink -f "$INPUT_BIN")
[[ -f $INPUT_BIN ]] || usage "$INPUT_BIN: File Not Found"

HERE=$(pwd)
cd "$(dirname "$0")"

# Convert raw binary to ELF relocatable object
aarch64-linux-gnu-objcopy \
    -I binary \
    -O elf64-littleaarch64 \
    -B aarch64 \
    --rename-section .data=.text,alloc,load,readonly,code,contents \
    $INPUT_BIN ops.o || exit 1


# Create a linker script
echo "ENTRY(_start)

SECTIONS
{
    . = 0x400000;

    .text : ALIGN(16)
    {
        KEEP(*(.text))
    }

    /DISCARD/ :
    {
        *(.comment)
        *(.note*)
    }
}
" > layout.ld


# Link this shit
aarch64-linux-gnu-ld \
    -Ttext=0x400000 \
    --entry=0x400000 \
    -T layout.ld \
    -o ops \
    ops.o || exit 2

# Cleanup
rm ops.o layout.ld || exit 3

mv ops $HERE/ops
echo "$HERE/ops ELF program created from $INPUT_BIN, enjoy!"
echo "Check the proc mapping with gdb command \`info proc mappings\`"
echo "Before running set the x0 register to the stack (rw) 0x7ffffdf000" 
echo "- set breakpoint \`b *0x400000\`"
echo '- set $x0=0x7ffffdf000'
