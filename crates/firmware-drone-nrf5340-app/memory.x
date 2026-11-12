/* Memory layout for the custom NRF5340 application core
 *
 * - 1024 KiB application-core flash @ 0x0000_0000
 * - 512 KiB application-core RAM @ 0x2000_0000
 * - 64 KiB shared IPC region @ 0x2007_0000 (within application-core RAM)
 *
 * The stack stays below SHARED_RAM. Both cores reserve the same IPC region.
 */

MEMORY
{
  FLASH         : ORIGIN = 0x00000000, LENGTH = 1012K
  RUNTIMECONFIG : ORIGIN = 0x000FD000, LENGTH = 8K
  BUILDCONFIG   : ORIGIN = 0x000FF000, LENGTH = 4K
  RAM           : ORIGIN = 0x20000000, LENGTH = 448K
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
