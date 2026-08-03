// auto-generated, do not edit

pub const DIRECTIVES: [(&str, &str); 15] = [(".define ( PUBLIC ) <symbol> <value>", "Define an integer symbol named <symbol> with the value <value>. If this .define appears before the first program in the input file, then this
define is global to all programs, otherwise it is local to the program in which it
occurs. If PUBLIC is specified the symbol will be emitted into the assembled
output for use by user code. For the SDK this takes the form of:
#define <program_name>_<symbol> value for program symbols or #define <symbol>
value for global symbols"), (".clock_div <divider>", "If this directive is present, <divider> is the state machine clock divider for the
program. Note, that divider is a floating point value, but may not currently use
arithmetic expressions or defined values. This directive affects the default
state machine configuration for a program. This directive is only valid within a
program before the first instruction"), (".fifo <fifo_config>", "If this directive is present, it is used to specify the FIFO configuration for the
program. It affects the default state machine configuration for a program, but
also restricts what instructions may be used (for example PUSH makes no
sense if there is no IN FIFO configrued).
The following values are supported:
txrx: 4 FIFO entries for each of TX and RX; this is the default. tx - All 8 FIFO
entries for TX.
rx - All 8 FIFO entries for RX.
txput - 4 FIFO entries for TX, and 4 FIFO entries for mov rxfifo[index], isr aka
put. This value is not supported on PIO version 0.
txget - 4 FIFO entries for TX, and 4 FIFO entries for mov osr, rxfifo[index] aka
get. This value is not supported on PIO version 0.
putget - 4 FIFO entries for mov rxfifo[index], isr aka put, and 4 FIFO entries for
mov osr, rxfifo[index] aka get. This value is not supported on PIO version 0.
This directive is only valid within a program before the first instruction"), (".mov_status rxfifo < <n>
.mov_status txfifo < <n>
.mov_status irq _( _prev | next )_ set
<n>_", "This directive configures the source for the mov , STATUS . One of the three
syntaxes can be used to set the status based on the RXFIFO level being below
a value N, the TXFIFO level being below a value N, or an IRQ flag N being set
on this PIO instance (or the next lower numbered, or higher numbered PIO
instance if prev or next or specified). Note, that the IRQ option requires PIO
version 1.
This directive affects the default state machine configuration for a program.
This directive is only valid within a program before the first instruction"), (".in <count> (left|right) (auto)
(<threshold>)", "If this directive is present, <count> indicates the number of IN bits to be used.
'left' or 'right' if specified, control the ISR shift direction; 'auto', if present,
enables \"auto-push\"; <threshold>, if present, specifies the \"auto-push\"
threshold. This directive affects the default state machine configuration for a
program. This directive is only valid within a program before the first
instruction
When assembling for PIO version 0, count must be 32."), (".program <name>", "Start a new program with the name <name>. Note that that name is used in
code so should be alphanumeric/underscore not starting with a digit. The
program lasts until another .program directive or the end of the source file. PIO
instructions are only allowed within a program"), (".origin <offset>", "Optional directive to specify the PIO instruction memory offset at which the
program must load. Most commonly this is used for programs that must load
at offset 0, because they use data based JMPs with the (absolute) jmp target
being stored in only a few bits. This directive is invalid outside a program"), (".out <count> (left|right) (auto)
(<threshold>)", "If this directive is present, <count> indicates the number of OUT bits to be
used. 'left' or 'right' if specified control the OSR shift direction; 'auto', if present,
enables \"auto-pull\"; <threshold>, if present, specifies the \"auto-pull\" threshold.
This directive affects the default state machine configuration for a program.
This directive is only valid within a program before the first instruction"), (".pio_version <version>", "This directive sets the target PIO hardware version. The version for RP2350 is
1 or RP2350, and is also the default version number. For backwards
compatibility with RP2040, 0 or RP2040 may be used.
If this directive appears before the first program in the input file, then this
define is the default for all programs, otherwise it specifies the version for the
program in which it occurs. If specified for a program, it must occur before the
first instruction."), (".set <count>", "If this directive is present, <count> indicates the number of SET bits to be
used. This directive affects the default state machine configuration for a
program. This directive is only valid within a program before the first
instruction"), (".side_set <count> (opt) (pindirs)", "If this directive is present, <count> indicates the number of side-set bits to be
used. Additionally opt may be specified to indicate that a side <value> is
optional for instructions (note this requires stealing an extra bit — in addition
to the <count> bits — from those available for the instruction delay). Finally,
pindirs may be specified to indicate that the side set values should be applied
to the PINDIRs and not the PINs. This directive is only valid within a program
before the first instruction"), (".wrap_target", "Place prior to an instruction, this directive specifies the instruction where
execution continues due to program wrapping. This directive is invalid outside
of a program, may only be used once within a program, and if not specified
defaults to the start of the program"), (".wrap", "Placed after an instruction, this directive specifies the instruction after which,
in normal control flow (i.e. jmp with false condition, or no jmp), the program
wraps (to .wrap_target instruction). This directive is invalid outside of a
program, may only be used once within a program, and if not specified
defaults to after the last program instruction."), (".lang_opt <lang> <name> <option>", "Specifies an option for the program related to a particular language generator.. This directive is invalid outside of a program"), (".word <value>", "Stores a raw 16-bit value as an instruction in the program. This directive is
invalid outside of a program.")];
pub const JMP: [(&str, &str); 2] = [("<cond>", "Is an optional condition listed above (e.g. !x for scratch X zero). If a condition code is not specified,
the branch is always taken"), ("<target>", "Is a program label or value representing instruction offset within the program (the
first instruction being offset 0). Note that because the PIO JMP instruction uses absolute addresses
in the PIO instruction memory, JMPs need to be adjusted based on the program load offset at
runtime. This is handled for you when loading a program with the SDK, but care should be taken when
encoding JMP instructions for use by OUT EXEC")];
pub const WAIT: [(&str, &str); 7] = [("<polarity>", "Is a value specifying the polarity (either 0 or 1)"), ("<pin_num>", "Is a value specifying the input pin number (as mapped by the SM input pin
mapping)"), ("<gpio_num>", "Is a value specifying the actual GPIO pin number"), ("<irq_num> ( rel )", "Is a value specifying The irq number to wait on (0-7). If rel is present, then the
actual irq number used is calculating by replacing the low two bits of the irq number (irq_num )
10
with the low two bits of the sum (irq_num + sm_num ) where sm_num is the state machine
10 10 10
number"), ("prev", "(version 1 and above) To wait on the IRQ on the next lower numbered PIO block instead of on the
current PIO block"), ("next", "(version 1 and above) To wait on the IRQ on the next higher numbered PIO block instead of on the
current PIO block"), ("<pin_offset>", "(version 1 and above) A value added to the jmp_pin to get the actual pin number.")];
pub const IN: [(&str, &str); 2] = [
    ("<source>", "Is one of the sources specified above."),
    (
        "<bit_count>",
        "Is a value specifying the number of bits to shift (valid range 1-32)",
    ),
];
pub const OUT: [(&str, &str); 2] = [
    (
        "<destination>",
        "Is one of the destinations specified above.",
    ),
    (
        "<bit_count>",
        "Is a value specifying the number of bits to shift (valid range 1-32)",
    ),
];
pub const PUSH: [(&str, &str); 3] = [("iffull", "Is equivalent to IfFull == 1 above. i.e. the default if this is not specified is IfFull == 0"), ("block", "Is equivalent to Block == 1 above. This is the default if neither block nor noblock are specified"), ("noblock", "Is equivalent to Block == 0 above.")];
pub const PULL: [(&str, &str); 3] = [("ifempty", "Is equivalent to IfEmpty == 1 above. i.e. the default if this is not specified is IfEmpty == 0"), ("block", "Is equivalent to Block == 1 above. This is the default if neither block nor noblock are specified"), ("noblock", "Is equivalent to Block == 0 above.")];
pub const MOV: [(&str, &str); 3] = [
    (
        "<destination>",
        "Is one of the destinations specified above.",
    ),
    (
        "<op>",
        "If present, is:
! or ~ for NOT (Note: this is always a bitwise NOT)
:: for bit reverse",
    ),
    ("<source>", "Is one of the sources specified above."),
];
pub const MOV_TO_RX: [(&str, &str); 2] = [
    (
        "y",
        "Is the literal token \"y\", indicating the RX FIFO entry is indexed by the Y register",
    ),
    (
        "<index>",
        "Is a value specifying the RX FIFO entry to write (valid range 0-3)",
    ),
];
pub const MOV_FROM_RX: [(&str, &str); 2] = [
    (
        "y",
        "Is the literal token \"y\", indicating the RX FIFO entry is indexed by the Y register",
    ),
    (
        "<index>",
        "Is a value specifying the RX FIFO entry to read (valid range 0-3)",
    ),
];
pub const IRQ: [(&str, &str); 8] = [("<irq_num> ( rel )", "Is a value specifying The irq number to wait on (0-7). If rel is present, then the
actual irq number used is calculating by replacing the low two bits of the irq number (irq_num )
10
with the low two bits of the sum (irq_num + sm_num ) where sm_num is the state machine
10 10 10
number"), ("irq", "Means set the IRQ without waiting"), ("irq set", "Also means set the IRQ without waiting"), ("irq nowait", "Again, means set the IRQ without waiting"), ("irq wait", "Means set the IRQ and wait for it to be cleared before proceeding"), ("irq clear", "Means clear the IRQ"), ("prev", "(version 1 and above) To target the IRQ on the next lower numbered PIO block instead of the
current PIO block"), ("next", "(version 1 and above) To target the IRQ on the next higher numbered PIO block instead of the
current PIO block")];
pub const SET: [(&str, &str); 2] = [
    (
        "<destination>",
        "Is one of the destinations specified above.",
    ),
    ("<value>", "The value to set (valid range 0-31)"),
];
pub const INSTRUCTIONS: [(&str, &[(&str, &str)]); 11] = [
    ("JMP", &JMP),
    ("WAIT", &WAIT),
    ("IN", &IN),
    ("OUT", &OUT),
    ("PUSH", &PUSH),
    ("PULL", &PULL),
    ("MOV", &MOV),
    ("MOV_TO_RX", &MOV_TO_RX),
    ("MOV_FROM_RX", &MOV_FROM_RX),
    ("IRQ", &IRQ),
    ("SET", &SET),
];
pub const INSTRUCTION_DOC: [(&str, &str); 11] = [("JMP", "
## Operation

Set program counter to Address if Condition is true, otherwise no operation.

Delay cycles on a JMP always take effect, whether Condition is true or false, and they take place after Condition is
evaluated and the program counter is updated.
* Condition:

  * 000: (no condition): Always

  * 001: !X: scratch X zero

  * 010: X--: scratch X non-zero, prior to decrement

  * 011: !Y: scratch Y zero

  * 100: Y--: scratch Y non-zero, prior to decrement

  * 101: X!=Y: scratch X not equal scratch Y

  * 110: PIN: branch on input pin

  * 111: !OSRE: output shift register not empty

* Address: Instruction address to jump to. In the instruction encoding this is an absolute address within the PIO
instruction memory.

JMP PIN branches on the GPIO selected by EXECCTRL_JMP_PIN, a configuration field which selects one out of the maximum
of 32 GPIO inputs visible to a state machine, independently of the state machine’s other input mapping. The branch is
taken if the GPIO is high.

!OSRE compares the bits shifted out since the last PULL with the shift count threshold configured by SHIFTCTRL_PULL_THRESH.
This is the same threshold used by autopull.

JMP X-- and JMP Y-- always decrement scratch register X or Y, respectively. The decrement is not conditional on the
current value of the scratch register. The branch is conditioned on the initial value of the register, i.e. before the
decrement took place: if the register is initially nonzero, the branch is taken.

## Parameters 
- **&lt;cond&gt;**  : Is an optional condition listed above (e.g. !x for scratch X zero). If a condition code is not specified, the branch is always taken
- **&lt;target&gt;**: Is a program label or value representing instruction offset within the program (the first instruction being offset 0). Note that because the PIO JMP instruction uses absolute addresses in the PIO instruction memory, JMPs need to be adjusted based on the program load offset at runtime. This is handled for you when loading a program with the SDK, but care should be taken when encoding JMP instructions for use by OUT EXEC"), ("WAIT", "
## Operation

Stall until some condition is met.

Like all stalling instructions, delay cycles begin after the instruction completes. That is, if any delay cycles are present,
they do not begin counting until after the wait condition is met.
* Polarity:

  * 1: wait for a 1.

  * 0: wait for a 0.

* Source: what to wait on. Values are:

  * 00: GPIO: System GPIO input selected by Index. This is an absolute GPIO index, and is not affected by the state
machine’s input IO mapping.
  * 01: PIN: Input pin selected by Index. This state machine’s input IO mapping is applied first, and then Index
selects which of the mapped bits to wait on. In other words, the pin is selected by adding Index to the
PINCTRL_IN_BASE configuration, modulo 32.
  * 10: IRQ: PIO IRQ flag selected by Index

  * 11: (version 1 and above) JMPPIN: wait on the pin indexed by the PINCTRL_JMP_PIN configuration, plus an Index in
the range 0-3, all modulo 32. Other values of Index are reserved.
* Index: which pin or bit to check.

WAIT x IRQ behaves slightly differently from other WAIT sources:
* If Polarity is 1, the selected IRQ flag is cleared by the state machine upon the wait condition being met.

* The flag index is decoded in the same way as the IRQ index field, decoding down from the two MSBs (aligning with
the IRQ instruction IdxMode field):
  * 00: the three LSBs are used directly to index the IRQ flags in this PIO block.

  * 01 (version 1 and above) (PREV), the instruction references an IRQ from the next-lower-numbered PIO in the
system, wrapping to the highest-numbered PIO if this is PIO0.
  * 10 (REL), the state machine ID (0…3) is added to the IRQ index, by way of modulo-4 addition on the two LSBs.
For example, state machine 2 with a flag value of '0x11' will wait on flag 3, and a flag value of '0x13' will wait
on flag 1. This allows multiple state machines running the same program to synchronise with each other.
  * 11 (version 1 and above) (NEXT), the instruction references an IRQ from the next-higher-numbered PIO in the
system, wrapping to PIO0 if this is the highest-numbered PIO.

---
> **_CAUTION_**
> 
> WAIT 1 IRQ x should not be used with IRQ flags presented to the interrupt controller, to avoid a race condition with a
> system interrupt handler
> 
---


## Parameters 
- **&lt;polarity&gt;**       : Is a value specifying the polarity (either 0 or 1)
- **&lt;pin_num&gt;**        : Is a value specifying the input pin number (as mapped by the SM input pin mapping)
- **&lt;gpio_num&gt;**       : Is a value specifying the actual GPIO pin number
- **&lt;irq_num&gt; ( rel )**: Is a value specifying The irq number to wait on (0-7). If rel is present, then the actual irq number used is calculating by replacing the low two bits of the irq number (irq_num ) 10 with the low two bits of the sum (irq_num + sm_num ) where sm_num is the state machine 10 10 10 number
- **prev**             : (version 1 and above) To wait on the IRQ on the next lower numbered PIO block instead of on the current PIO block
- **next**             : (version 1 and above) To wait on the IRQ on the next higher numbered PIO block instead of on the current PIO block
- **&lt;pin_offset&gt;**     : (version 1 and above) A value added to the jmp_pin to get the actual pin number."), ("IN", "
## Operation

Shift Bit count bits from Source into the Input Shift Register (ISR). Shift direction is configured for each state machine by
SHIFTCTRL_IN_SHIFTDIR. Additionally, increase the input shift count by Bit count, saturating at 32.
* Source:

  * 000: PINS

  * 001: X (scratch register X)

  * 010: Y (scratch register Y)
  * 011: NULL (all zeroes)

  * 100: Reserved

  * 101: Reserved

  * 110: ISR

  * 111: OSR

* Bit count: How many bits to shift into the ISR. 1…32 bits, 32 is encoded as 00000.

If automatic push is enabled, IN will also push the ISR contents to the RX FIFO if the push threshold is reached
(SHIFTCTRL_PUSH_THRESH). IN still executes in one cycle, whether an automatic push takes place or not. The state machine
will stall if the RX FIFO is full when an automatic push occurs. An automatic push clears the ISR contents to all-zeroes,
and clears the input shift count.

IN always uses the least significant Bit count bits of the source data. For example, if PINCTRL_IN_BASE is set to 5, the
instruction IN PINS, 3 will take the values of pins 5, 6 and 7, and shift these into the ISR. First the ISR is shifted to the left
or right to make room for the new input data, then the input data is copied into the gap this leaves. The bit order of the
input data is not dependent on the shift direction.

NULL can be used for shifting the ISR’s contents. For example, UARTs receive the LSB first, so must shift to the right.
After 8 IN PINS, 1 instructions, the input serial data will occupy bits 31…24 of the ISR. An IN NULL, 24 instruction will shift
in 24 zero bits, aligning the input data at ISR bits 7…0. Alternatively, the processor or DMA could perform a byte read
from FIFO address + 3, which would take bits 31…24 of the FIFO contents.

## Parameters 
- **&lt;source&gt;**   : Is one of the sources specified above.
- **&lt;bit_count&gt;**: Is a value specifying the number of bits to shift (valid range 1-32)"), ("OUT", "
## Operation

Shift Bit count bits out of the Output Shift Register (OSR), and write those bits to Destination. Additionally, increase the
output shift count by Bit count, saturating at 32.
* Destination:

  * 000: PINS

  * 001: X (scratch register X)

  * 010: Y (scratch register Y)

  * 011: NULL (discard data)
  * 100: PINDIRS

  * 101: PC

  * 110: ISR (also sets ISR shift counter to Bit count)

  * 111: EXEC (Execute OSR shift data as instruction)

* Bit count: how many bits to shift out of the OSR. 1…32 bits, 32 is encoded as 00000.

A 32-bit value is written to Destination: the lower Bit count bits come from the OSR, and the remainder are zeroes. This
value is the least significant Bit count bits of the OSR if SHIFTCTRL_OUT_SHIFTDIR is to the right, otherwise it is the most
significant bits.

PINS and PINDIRS use the OUT pin mapping.

If automatic pull is enabled, the OSR is automatically refilled from the TX FIFO if the pull threshold, SHIFTCTRL_PULL_THRESH,
is reached. The output shift count is simultaneously cleared to 0. In this case, the OUT will stall if the TX FIFO is empty,
but otherwise still executes in one cycle.

OUT EXEC allows instructions to be included inline in the FIFO datastream. The OUT itself executes on one cycle, and the
instruction from the OSR is executed on the next cycle. There are no restrictions on the types of instructions which can
be executed by this mechanism. Delay cycles on the initial OUT are ignored, but the executee may insert delay cycles as
normal.

OUT PC behaves as an unconditional jump to an address shifted out from the OSR.

## Parameters 
- **&lt;destination&gt;**: Is one of the destinations specified above.
- **&lt;bit_count&gt;**  : Is a value specifying the number of bits to shift (valid range 1-32)"), ("PUSH", "
## Operation

Push the contents of the ISR into the RX FIFO, as a single 32-bit word. Clear ISR to all-zeroes.
* IfFull: If 1, do nothing unless the total input shift count has reached its threshold, SHIFTCTRL_PUSH_THRESH (the same
as for autopush).
* Block: If 1, stall execution if RX FIFO is full.

PUSH IFFULL helps to make programs more compact, like autopush. It is useful in cases where the IN would stall at an
inappropriate time if autopush were enabled, e.g. if the state machine is asserting some external control signal at this
point.

The PIO assembler sets the Block bit by default. If the Block bit is not set, the PUSH does not stall on a full RX FIFO, instead
continuing immediately to the next instruction. The FIFO state and contents are unchanged when this happens. The ISR
is still cleared to all-zeroes, and the FDEBUG_RXSTALL flag is set (the same as a blocking PUSH or autopush to a full RX FIFO)
to indicate data was lost.

## Parameters 
- **iffull** : Is equivalent to IfFull == 1 above. i.e. the default if this is not specified is IfFull == 0
- **block**  : Is equivalent to Block == 1 above. This is the default if neither block nor noblock are specified
- **noblock**: Is equivalent to Block == 0 above."), ("PULL", "
## Operation

Load a 32-bit word from the TX FIFO into the OSR.
* IfEmpty: If 1, do nothing unless the total output shift count has reached its threshold, SHIFTCTRL_PULL_THRESH (the
same as for autopull).
* Block: If 1, stall if TX FIFO is empty. If 0, pulling from an empty FIFO copies scratch X to OSR.

Some peripherals (UART, SPI…) should halt when no data is available, and pick it up as it comes in; others (I2S) should
clock continuously, and it is better to output placeholder or repeated data than to stop clocking. This can be achieved
with the Block parameter.

A nonblocking PULL on an empty FIFO has the same effect as MOV OSR, X. The program can either preload scratch register
X with a suitable default, or execute a MOV X, OSR after each PULL NOBLOCK, so that the last valid FIFO word will be recycled
until new data is available.

PULL IFEMPTY is useful if an OUT with autopull would stall in an inappropriate location when the TX FIFO is empty. For
example, a UART transmitter should not stall immediately after asserting the start bit. IfEmpty permits some of the same
program simplifications as autopull, but the stall occurs at a controlled point in the program.

---
> **_NOTE_**
> 
> When autopull is enabled, any PULL instruction is a no-op when the OSR is full, so that the PULL instruction behaves as
> a barrier. OUT NULL, 32 can be used to explicitly discard the OSR contents. See the RP2350 Datasheet for more detail
> on autopull.
> 
---


## Parameters 
- **ifempty**: Is equivalent to IfEmpty == 1 above. i.e. the default if this is not specified is IfEmpty == 0
- **block**  : Is equivalent to Block == 1 above. This is the default if neither block nor noblock are specified
- **noblock**: Is equivalent to Block == 0 above."), ("MOV", "
## Operation

Copy data from Source to Destination.
* Destination:

  * 000: PINS (Uses same pin mapping as OUT)

  * 001: X (Scratch register X)

  * 010: Y (Scratch register Y)

  * 011: (version 1 and above) PINDIRS (Uses same pin mapping as OUT)

  * 100: EXEC (Execute data as instruction)

  * 101: PC

  * 110: ISR (Input shift counter is reset to 0 by this operation, i.e. empty)

  * 111: OSR (Output shift counter is reset to 0 by this operation, i.e. full)

* Operation:

  * 00: None

  * 01: Invert (bitwise complement)

  * 10: Bit-reverse

  * 11: Reserved

* Source:

  * 000: PINS (Uses same pin mapping as IN)
  * 001: X

  * 010: Y

  * 011: NULL

  * 100: Reserved

  * 101: STATUS

  * 110: ISR

  * 111: OSR

MOV PC causes an unconditional jump. MOV EXEC has the same behaviour as OUT EXEC (Section 3.4.7), and allows register
contents to be executed as an instruction. The MOV itself executes in 1 cycle, and the instruction in Source on the next
cycle. Delay cycles on MOV EXEC are ignored, but the executee may insert delay cycles as normal.

The STATUS source has a value of all-ones or all-zeroes, depending on some state machine status such as FIFO
full/empty, configured by EXECCTRL_STATUS_SEL.

MOV can manipulate the transferred data in limited ways, specified by the Operation argument. Invert sets each bit in
Destination to the logical NOT of the corresponding bit in Source, i.e. 1 bits become 0 bits, and vice versa. Bit reverse sets
each bit n in Destination to bit 31 - n in Source, assuming the bits are numbered 0 to 31.

MOV dst, PINS reads pins using the IN pin mapping, and writes the full 32-bit value to the destination without masking.
The LSB of the read value is the pin indicated by PINCTRL_IN_BASE, and each successive bit comes from a higher-
numbered pin, wrapping after 31.

## Parameters 
- **&lt;destination&gt;**: Is one of the destinations specified above.
- **&lt;op&gt;**         : If present, is: ! or ~ for NOT (Note: this is always a bitwise NOT) :: for bit reverse
- **&lt;source&gt;**     : Is one of the sources specified above."), ("MOV_TO_RX", "
## Operation

Write the ISR to a selected RX FIFO entry. The state machine can write the RX FIFO entries in any order, indexed either
by the Y register, or an immediate Index in the instruction. Requires the SHIFTCTRL_FJOIN_RX_PUT configuration field to be
set, otherwise its operation is undefined. The FIFO configuration can be specified for the program via the .fifo directive
(see pioasm_fifo).

If IdxI (index by immediate) is set, the RX FIFO’s registers are indexed by the two least-significant bits of the Index
operand. Otherwise, they are indexed by the two least-significant bits of the Y register. When IdxI is clear, all nonzero
values of Index are reserved encodings, and their operation is undefined.

When only SHIFTCTRL_FJOIN_RX_PUT is set (in SM0_SHIFTCTRL through SM3_SHIFTCTRL), the system can also read the RX FIFO
registers with random access via RXF0_PUTGET0 through RXF0_PUTGET3 (where RXFx indicates which state machine’s FIFO is
being accessed). In this state, the FIFO register storage is repurposed as status registers, which the state machine can
update at any time and the system can read at any time. For example, a quadrature decoder program could maintain the
current step count in a status register at all times, rather than pushing to the RX FIFO and potentially blocking.

When both SHIFTCTRL_FJOIN_RX_PUT and SHIFTCTRL_FJOIN_RX_GET are set, the system can no longer access the RX FIFO
storage registers, but the state machine can now put/get the registers in arbitrary order, allowing them to be used as
additional scratch storage.

---
> **_NOTE_**
> 
> The RX FIFO storage registers have only a single read port and write port, and access through each port is assigned
> to only one of (system, state machine) at any time.
> 
---


## Parameters 
- **y**      : Is the literal token \"y\", indicating the RX FIFO entry is indexed by the Y register
- **&lt;index&gt;**: Is a value specifying the RX FIFO entry to write (valid range 0-3)"), ("MOV_FROM_RX", "
## Operation

Read the selected RX FIFO entry into the OSR. The PIO state machine can read the FIFO entries in any order, indexed
either by the Y register, or an immediate Index in the instruction. Requires the SHIFTCTRL_FJOIN_RX_GET configuration field
to be set, otherwise its operation is undefined.

If IdxI (index by immediate) is set, the RX FIFO’s registers are indexed by the two least-significant bits of the Index
operand. Otherwise, they are indexed by the two least-significant bits of the Y register. When IdxI is clear, all nonzero
values of Index are reserved encodings, and their operation is undefined.

When only SHIFTCTRL_FJOIN_RX_GET is set, the system can also write the RX FIFO registers with random access via
RXF0_PUTGET0 through RXF0_PUTGET3 (where RXFx indicates which state machine’s FIFO is being accessed). In this state, the
RX FIFO register storage is repurposed as additional configuration registers, which the system can update at any time
and the state machine can read at any time. For example, a UART TX program might use these registers to configure
the number of data bits, or the presence of an additional stop bit.

When both SHIFTCTRL_FJOIN_RX_PUT and SHIFTCTRL_FJOIN_RX_GET are set, the system can no longer access the RX FIFO
storage registers, but the state machine can now put/get the registers in arbitrary order, allowing them to be used as
additional scratch storage.

---
> **_NOTE_**
> 
> The RX FIFO storage registers have only a single read port and write port, and access through each port is assigned
> to only one of (system, state machine) at any time.
> 
---


## Parameters 
- **y**      : Is the literal token \"y\", indicating the RX FIFO entry is indexed by the Y register
- **&lt;index&gt;**: Is a value specifying the RX FIFO entry to read (valid range 0-3)"), ("IRQ", "
## Operation

Set or clear the IRQ flag selected by Index argument. * Clear: if 1, clear the flag selected by Index, instead of raising it. If
Clear is set, the Wait bit has no effect. * Wait: if 1, halt until the raised flag is lowered again, e.g. if a system interrupt
handler has acknowledged the flag. * Index: specifies an IRQ index from 0-7. This IRQ flag will be set/cleared depending
on the Clear bit. * IdxMode: modify the behaviour if the Index field, either modifying the index, or indexing IRQ flags from
a different PIO block: 00: the three LSBs are used directly to index the IRQ flags in this PIO block. 01 (version 1 and
above) (PREV): the instruction references an IRQ flag from the next-lower-numbered PIO in the system, wrapping to the
highest-numbered PIO if this is PIO0. 10 (REL): the state machine ID (0…3) is added to the IRQ flag index, by way of
modulo-4 addition on the two LSBs. For example, state machine 2 with a flag value of '0x11' will wait on flag 3, and a
flag value of '0x13' will wait on flag 1. This allows multiple state machines running the same program to synchronise
with each other. 11 (version 1 and above) (NEXT): the instruction references an IRQ flag from the next-higher-numbered
PIO in the system, wrapping to PIO0 if this is the highest-numbered PIO.

On PIO version 0, IRQ flags 4-7 are visible only to the state machines; IRQ flags 0-3 can be routed out to system level
interrupts, on either of the PIO’s two external interrupt request lines, configured by IRQ0_INTE and IRQ1_INTE. PIO version 1
lifts this limitation and allows all eight flags to assert system interrupts.

The modulo addition mode allows relative addressing of 'IRQ' and 'WAIT' instructions, for synchronising state machines
which are running the same program. Bit 2 (the third LSB) is unaffected by this addition.

The modulo addition mode (REL) allows relative addressing of 'IRQ' and 'WAIT' instructions, for synchronising state
machines which are running the same program. Bit 2 (the third LSB) is unaffected by this addition.

The NEXT/PREV modes (version 1 and above) can be used to synchronise between state machines in different PIO blocks.
If these state machines' clocks are divided, their clock dividers must be the same, and must have been synchronised by
writing CTRL.NEXTPREV_CLKDIV_RESTART in addition to the relevant NEXT_PIO_MASK/PREV_PIO_MASK bits. Note that the
cross-PIO connection is severed between PIOs with different accessibility to Non-secure code, as per ACCESSCTRL.

If Wait is set, Delay cycles do not begin until after the wait period elapses.

## Parameters 
- **&lt;irq_num&gt; ( rel )**: Is a value specifying The irq number to wait on (0-7). If rel is present, then the actual irq number used is calculating by replacing the low two bits of the irq number (irq_num ) 10 with the low two bits of the sum (irq_num + sm_num ) where sm_num is the state machine 10 10 10 number
- **irq**              : Means set the IRQ without waiting
- **irq set**          : Also means set the IRQ without waiting
- **irq nowait**       : Again, means set the IRQ without waiting
- **irq wait**         : Means set the IRQ and wait for it to be cleared before proceeding
- **irq clear**        : Means clear the IRQ
- **prev**             : (version 1 and above) To target the IRQ on the next lower numbered PIO block instead of the current PIO block
- **next**             : (version 1 and above) To target the IRQ on the next higher numbered PIO block instead of the current PIO block"), ("SET", "
## Operation

Write immediate value Data to Destination.
* Destination:

  * 000: PINS

  * 001: X (scratch register X) 5 LSBs are set to Data, all others cleared to 0.

  * 010: Y (scratch register Y) 5 LSBs are set to Data, all others cleared to 0.

  * 011: Reserved

  * 100: PINDIRS

  * 101: Reserved

  * 110: Reserved

  * 111: Reserved

* Data: 5-bit immediate value to drive to pins or register.

This can be used to assert control signals such as a clock or chip select, or to initialise loop counters. As Data is 5 bits in
size, scratch registers can be SET to values from 0-31, which is sufficient for a 32-iteration loop.

The mapping of SET and OUT onto pins is configured independently. They may be mapped to distinct locations, for
example if one pin is to be used as a clock signal, and another for data. They may also be overlapping ranges of pins: a
UART transmitter might use SET to assert start and stop bits, and OUT instructions to shift out FIFO data to the same pins.

## Parameters 
- **&lt;destination&gt;**: Is one of the destinations specified above.
- **&lt;value&gt;**      : The value to set (valid range 0-31)")];
