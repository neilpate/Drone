/* Memory layout for the custom NRF5340 application core
 *
 * - 1024 KiB application-core flash @ 0x0000_0000
 * - 512 KiB application-core RAM @ 0x2000_0000
 * - 64 KiB shared IPC region @ 0x2007_0000 (within application-core RAM)
 *
 * The smoke test uses cortex-m-rt's standard stack at the top of RAM.
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
    KEEP(*(.shared_ram .shared_ram.*));
  } > SHARED_RAM
} INSERT AFTER .text;
