.global main
.align 2
.intel_syntax noprefix

.text
main:

  // 00. Set buffer address in x0
  // Since BrainFuck has a memory space

  lea  rdi, [rip + buffer]      // Load the buffer address

  // Just for fun and useful to backport the assembled instructions
  // Create the basic instruction blocks

  // A. INC
  inc  byte ptr [rdi]	// increment at rsi
	nop
  add  byte ptr [rdi], 0x42
	nop

  // B. DEC
  dec  byte ptr [rdi]	// decrement at rsi
	nop
  sub byte ptr [rdi], 69
	nop

  // C. LEFT
  dec  rdi		// decrement rsi
	nop
  sub  rdi, 0x42
  sub  rdi, 0x1337
	nop

  // D. RIGHT
  inc  rdi		// increment rsi
	nop
  add  rdi, 0x42
  add  rdi, 0x1337
	nop

  // E. JZ
  movzx  eax, byte ptr [rdi]
  test   ax, ax
  jz     remote1
  nop

next1:
  // F. JNZ
  movzx  eax, byte ptr [rdi]
  test   ax, ax
  jnz    remote2
  nop

next2:

  // 0. write the INPUT (,) brainfuck block
  mov  rsi, rdi		// Save the head pointer, and set the buffer address
  xor  rdi, rdi		// Set fd to Stdin to read from
  mov  rdx, 1		// Set the number of chars to read
  xor  rax, rax		// Set the syscall #0 (read)
  syscall
  test rax, rax	// Check if we've read anything
  js failed		// RAX < 0: Error occured
  jz eof			// RAX = 0: EOF reached
read_ok:
  jmp done
failed:	
  mov byte ptr [rsi], 0	// Store 0 at [rdi] (rsi)
  mov  rdi, rsi		// Restore RDI	
  ret
eof:	
  mov byte ptr [rsi], 0	// Store 0 at [rdi] (rsi)
done:	
  mov  rdi, rsi		// Restore RDI	

	nop
	
  // 1. is the end of string reached?
  movzx  eax, byte ptr [rdi]
  test ax, ax
  jz exit

	nop
loop:
  // 2. output character if any OUPUT (.) brainfuck block
  mov  rsi, rdi       // Set buffer
  // mov  r8,  rdi       // Save rdi
  mov  rdi, 0x1       // Write to stdout
  mov  rdx, 0x1       // Write one char
  mov  rax, 0x1       // Syscall sys_write
  syscall             // Kernel trap
  mov  rdi, rsi       // Restore rdi

	nop
	
  // 3. INPUT (,) brainfuck block
  mov  rsi, rdi		// Save the head pointer, and set the buffer address
  xor  rdi, rdi		// Set fd to Stdin to read from
  mov  rdx, 1		// Set the number of chars to read
  xor  rax, rax		// Set the syscall #0 (read)
  syscall
  test rax, rax 	// Check if we've read anything
  js _failed		// RAX < 0: Error occured
  jz _eof			// RAX = 0: EOF reached
_read_ok:
  jmp _done
_failed:	
  mov byte ptr [rsi], 0	// Store 0 at [rdi] (rsi)
  mov  rdi, rsi		// Restore RDI	
  ret
_eof:	
  mov byte ptr [rsi], 0	// Store 0 at [rdi] (rsi)
_done:	
  mov  rdi, rsi		// Restore RDI	

	nop
  // 4. loop if last read char was not \x00
  movzx  eax, byte ptr [rdi]
  test ax, ax
  jnz loop

exit:
  mov  rdi, 0              // Success exit status 0

exit_with_status:
  mov  rax, 60		// Syscall number 1: sys_exit
  syscall

remote1:
  jmp  next1

remote2:
  jmp  next2

.data
.align 4
buffer:
    .space 16               // Allocate 16 bytes of zero-filled memory
