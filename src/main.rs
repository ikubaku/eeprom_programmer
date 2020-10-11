#![deny(warnings)]
//#![deny(unsafe_code)]
#![no_main]
#![no_std]

use cortex_m_rt as rt;
use panic_halt as _;
use stm32g0xx_hal as hal;

//use hal::prelude::*;
use hal::stm32;
use rt::entry;

use core::fmt::Write;

use hal::rcc::RccExt;
use hal::gpio::GpioExt;
use hal::serial::SerialExt;
use hal::serial::FullConfig;
use stm32g0xx_hal::time::U32Ext;

use eeprom_programmer_command::parser::Command;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();

    // HACK: enable the clock of syscfg before the RCC is borrowed to the main thread
    dp.RCC.apbenr2.write(|w| w.syscfgen().set_bit());

    let mut rcc = dp.RCC.constrain();

    // Pin remappings for the UART functionality
    let cfgr1 = dp.SYSCFG.cfgr1.read().bits();
    unsafe { dp.SYSCFG.cfgr1.write(|w| w.bits(cfgr1 | 0b0000_0000_0000_0000_0000_0000_0001_1000)); }

    // Configure GPIO A
    let gpioa = dp.GPIOA.split(&mut rcc);
    let txd = gpioa.pa9;
    let rxd = gpioa.pa10;

    // Enable UART
    let uart_config = FullConfig::default();
    let uart_config = uart_config.baudrate(9600.bps());
    let uart = dp.USART1.usart(txd, rxd, uart_config, &mut rcc).unwrap();
    let (mut tx, rx) = uart.split();

    // Create the Command Parser
    let reader = eeprom_programmer_command::reader::SerialReader::new(rx);
    let mut parser = eeprom_programmer_command::parser::Parser::new(reader);

    // Interactive interface
    writeln!(tx, "ikeeprom EEPROM Reader & Writer v0.1.0\r").unwrap();

    loop {
        write!(tx, "> ").unwrap();
        loop {
            match parser.parse_command() {
                Ok(cmd) => {
                    match cmd {
                        Command::ReadByte(_) => writeln!(tx, "ReadByte\r").unwrap(),
                        Command::WriteByte(_, _) => writeln!(tx, "WriteByte\r").unwrap(),
                        Command::ReadData(_, _) => writeln!(tx, "ReadData\r").unwrap(),
                        Command::WritePage(_) => writeln!(tx, "WritePage\r").unwrap(),
                        Command::SetDevice(_) => writeln!(tx, "SetDevice\r").unwrap(),
                    }
                },
                Err(_) => writeln!(tx, "An error occured!\r").unwrap(),
            }
        }
    }
}
