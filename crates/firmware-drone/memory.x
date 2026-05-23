/* Memory layout for the BBC micro:bit v2 (Nordic nRF52833, QFN aQFN73).
 *
 * - 512 KiB flash @ 0x0000_0000
 * - 128 KiB RAM   @ 0x2000_0000
 *
 * No SoftDevice is in use, so flash starts at 0x0000_0000. If/when BLE is
 * adopted (it is not on the roadmap — radio is plain Nordic ESB or 802.15.4),
 * the SoftDevice will sit at the base of flash and this file must move FLASH
 * ORIGIN up accordingly.
 *
 * `flip-link` (configured in .cargo/config.toml) takes care of stack overflow
 * protection by relocating the stack to the bottom of RAM and trapping on
 * underflow, so no manual stack split is needed here.
 */

MEMORY
{
  FLASH : ORIGIN = 0x00000000, LENGTH = 512K
  RAM   : ORIGIN = 0x20000000, LENGTH = 128K
}
