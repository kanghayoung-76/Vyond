package chipyard.fpga.vcu118

import sys.process._

import org.chipsalliance.cde.config.{Config, Parameters}
import freechips.rocketchip.subsystem.{SystemBusKey, PeripheryBusKey, ControlBusKey, ExtMem}
import freechips.rocketchip.devices.debug.{DebugModuleKey, ExportDebug, JTAG}
import freechips.rocketchip.devices.tilelink.{DevNullParams, BootROMLocated}
import freechips.rocketchip.diplomacy.{DTSModel, DTSTimebase, RegionType, AddressSet}
import freechips.rocketchip.tile.{XLen}

import sifive.blocks.devices.spi.{PeripherySPIKey, SPIParams}
import sifive.blocks.devices.uart.{PeripheryUARTKey, UARTParams}

import sifive.fpgashells.shell.{DesignKey}
import sifive.fpgashells.shell.xilinx.{VCU118ShellPMOD, VCU118DDRSize}

import testchipip.serdes.{SerialTLKey}

import chipyard._
import chipyard.harness._

class WithDefaultPeripherals extends Config((site, here, up) => {
  case PeripheryUARTKey => List(UARTParams(address = BigInt(0x64000000L)))
  case PeripherySPIKey => List(SPIParams(rAddress = BigInt(0x64001000L)))
  case VCU118ShellPMOD => "SDIO"
})

class WithSystemModifications extends Config((site, here, up) => {
  case DTSTimebase => BigInt((1e6).toLong)
  case BootROMLocated(x) => up(BootROMLocated(x), site).map { p =>
    // invoke makefile for sdboot
    val freqMHz = (site(SystemBusKey).dtsFrequency.get / (1000 * 1000)).toLong
    val make = s"make -C fpga/src/main/resources/vcu118/sdboot PBUS_CLK=${freqMHz} bin"
    require (make.! == 0, "Failed to build bootrom")
    p.copy(hang = 0x10000, contentFileName = s"./fpga/src/main/resources/vcu118/sdboot/build/sdboot.bin")
  }
  case ExtMem => up(ExtMem, site).map(x => x.copy(master = x.master.copy(size = site(VCU118DDRSize)))) // set extmem to DDR size
  case SerialTLKey => Nil // remove serialized tl port
})

// DOC include start: AbstractVCU118 and Rocket
class WithVCU118Tweaks extends Config(
  // clocking
  new chipyard.harness.WithAllClocksFromHarnessClockInstantiator ++
  new chipyard.clocking.WithPassthroughClockGenerator ++
  new chipyard.config.WithMemoryBusFrequency(75) ++
  new chipyard.config.WithSystemBusFrequency(75) ++
  new chipyard.config.WithControlBusFrequency(75) ++
  new chipyard.config.WithPeripheryBusFrequency(75) ++
  new chipyard.config.WithControlBusFrequency(75) ++
  new WithFPGAFrequency(75) ++ // default 75MHz freq
  // harness binders
  new WithUART ++
  new WithSPISDCard ++
  new WithDDRMem ++
  new WithJTAG ++
  // other configuration
  new WithDefaultPeripherals ++
  new chipyard.config.WithTLBackingMemory ++ // use TL backing memory
  new WithSystemModifications ++ // setup busses, use sdboot bootrom, setup ext. mem. size
  //new chipyard.config.WithNoDebug ++ // remove debug module
  new freechips.rocketchip.subsystem.WithoutTLMonitors ++
  new freechips.rocketchip.subsystem.WithNMemoryChannels(1)
)

class RocketVCU118Config extends Config(
  new WithVCU118Tweaks ++
  new chipyard.RocketConfig
)

class WGRocketVCU118Config extends Config(
  new WithVCU118Tweaks ++
  new chipyard.WGRocketConfig
)

// Phase 2: 8-world (nWorlds=8, widWidth=3) variant of WGRocketVCU118Config.
// Pinned to 50 MHz to match the verified 4-world bitstream's known-good timing
// point (WithVCU118Tweaks defaults to 75 MHz). Differs from WGRocketVCU118Config
// only in nWorlds.
class WGRocket8VCU118Config extends Config(
  new WithFPGAFreq50MHz ++
  new WithVCU118Tweaks ++
  new chipyard.WGRocket8Config
)

// 16-world (widWidth=4) VCU118 variant, 50 MHz. Smaller than 32-world so it
// should close timing at 50 MHz (faster than the 32-world @25MHz fallback).
class WGRocket16VCU118Config extends Config(
  new WithFPGAFreq50MHz ++
  new WithVCU118Tweaks ++
  new chipyard.WGRocket16Config
)

// Architectural-maximum 32-world (widWidth=5) VCU118 variant, 50 MHz.
// Larger design (5-bit WID tags, perm fully uses 64 bits) — timing closure
// not guaranteed; treat the build as exploratory.
class WGRocket32VCU118Config extends Config(
  new WithFPGAFreq50MHz ++
  new WithVCU118Tweaks ++
  new chipyard.WGRocket32Config
)

// 32-world at 25 MHz. The 50 MHz variant closes timing with ~0ns margin on the
// main system clock (harnessSysPLL), which hangs under sustained load; 25 MHz
// gives a comfortable margin for functional validation.
class WGRocket32VCU118Config25 extends Config(
  new WithFPGAFreq25MHz ++
  new WithVCU118Tweaks ++
  new chipyard.WGRocket32Config
)
// DOC include end: AbstractVCU118 and Rocket

class BoomVCU118Config extends Config(
  new WithFPGAFrequency(50) ++
  new WithVCU118Tweaks ++
  new chipyard.MegaBoomConfig
)

class WithFPGAFrequency(fMHz: Double) extends Config(
  new chipyard.harness.WithHarnessBinderClockFreqMHz(fMHz) ++
  new chipyard.config.WithSystemBusFrequency(fMHz) ++
  new chipyard.config.WithPeripheryBusFrequency(fMHz) ++
  new chipyard.config.WithControlBusFrequency(fMHz) ++
  new chipyard.config.WithFrontBusFrequency(fMHz) ++
  new chipyard.config.WithMemoryBusFrequency(fMHz)
)

class WithFPGAFreq25MHz extends WithFPGAFrequency(25)
class WithFPGAFreq50MHz extends WithFPGAFrequency(50)
class WithFPGAFreq75MHz extends WithFPGAFrequency(75)
class WithFPGAFreq100MHz extends WithFPGAFrequency(100)
