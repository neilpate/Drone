/* Memory layout for the custom NRF5340 board
 *
 * - 256 KiB network-core flash @ 0x0100_0000
 * - 64 KiB network-core RAM   @ 0x2100_0000
 * - 64 KiB shared IPC region  @ 0x2007_0000 (in app-core RAM)
 *
 * The stack uses network-core RAM; SHARED_RAM is owned by the IPC transport.
 */

MEMORY
{
  FLASH         : ORIGIN = 0x01000000, LENGTH = 244K
  RUNTIMECONFIG : ORIGIN = 0x0103D000, LENGTH = 8K
  BUILDCONFIG   : ORIGIN = 0x0103F000, LENGTH = 4K
  RAM           : ORIGIN = 0x21000000, LENGTH = 64K
  SHARED_RAM    : ORIGIN = 0x20070000, LENGTH = 64K
}

SECTIONS
{
  .buildconfig :
  {
    KEEP(*(.buildconfig .buildconfig.*));
  } > BUILDCONFIG

  .shared_ram (NOLOAD) :
  {
    . = ALIGN(4);
    __radio_shared_start = .;
    KEEP(*(.shared_ram .shared_ram.*));
    . = ORIGIN(SHARED_RAM) + LENGTH(SHARED_RAM);
    __radio_shared_end = .;
  } > SHARED_RAM
} INSERT AFTER .text;
