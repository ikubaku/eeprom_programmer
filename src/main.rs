//#![deny(warnings)]
#![no_main]
#![no_std]

use core::fmt::Write as Core_Write;

use cortex_m_rt as rt;
use panic_halt as _;
use rt::entry;
use stm32g0xx_hal as hal;

use arrayvec::ArrayVec;

use nb::block;

use hal::delay::DelayExt;
use hal::gpio::GpioExt;
use hal::hal::digital::v2::OutputPin;
use hal::hal::serial::Read;
use hal::hal::serial::Write;
use hal::i2c;
use hal::i2c::I2cExt;
use hal::rcc::RccExt;
use hal::serial::FullConfig;
use hal::serial::SerialExt;
use hal::stm32;
use hal::time::U32Ext;

use eeprom24x::{Eeprom24x, SlaveAddr};

use eeprom_programmer_command::parser::{Command, Parser};
use eeprom_programmer_command::reader::BufferReader;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().unwrap();

    // HACK: enable the clock of syscfg before the RCC is borrowed to the main thread
    dp.RCC.apbenr2.write(|w| w.syscfgen().set_bit());

    let mut rcc = dp.RCC.constrain();

    // Pin remappings for the UART functionality
    let cfgr1 = dp.SYSCFG.cfgr1.read().bits();
    unsafe {
        dp.SYSCFG
            .cfgr1
            .write(|w| w.bits(cfgr1 | 0b0000_0000_0000_0000_0000_0000_0001_1000));
    }

    // Configure a timer for delay
    let mut delay = dp.TIM1.delay(&mut rcc);

    // Configure GPIO A
    let gpioa = dp.GPIOA.split(&mut rcc);
    let txd = gpioa.pa9;
    let rxd = gpioa.pa10;

    // Enable UART
    let uart_config = FullConfig::default().baudrate(9600.bps());
    let uart = dp.USART1.usart(txd, rxd, uart_config, &mut rcc).unwrap();
    let (mut tx, mut rx) = uart.split();

    // Create a readline buffer
    let mut read_buf = ArrayVec::<[u8; 32]>::new();

    // Interactive interface
    writeln!(tx, "ikeeprom EEPROM Reader & Writer v0.1.0\r").unwrap();

    // Wait for firmware download via SWD for 2 seconds
    delay.delay(2000.ms());

    // Initialize the I2C bus
    let mut pulldown = gpioa.pa13.into_open_drain_output();
    pulldown.set_low().unwrap();
    let gpiob = dp.GPIOB.split(&mut rcc);
    let sda = gpiob.pb7.into_open_drain_output();
    let scl = gpiob.pb6.into_open_drain_output();
    let mut i2c = dp
        .I2C1
        .i2c(sda, scl, i2c::Config::new(100_000.hz()), &mut rcc);

    // Create EEPROM device
    let eeprom_addr = SlaveAddr::Default;
    // 24x64 as a default device
    let mut eeprom = Eeprom24x::new_24x64(i2c, eeprom_addr);

    loop {
        write!(tx, "> ").unwrap();
        read_buf.clear();
        loop {
            let c = block!(rx.read()).unwrap_or(b' ');
            if c == 0x08 {
                if read_buf.pop().is_some() {
                    block!(tx.write(c)).unwrap();
                }
            } else if (0x20 <= c && c <= 0x7E) || c == b'\r' || c == b'\n' {
                if read_buf.try_push(c).is_ok() {
                    block!(tx.write(c)).unwrap();
                }
            }
            if c == b'\n' {
                let reader = BufferReader::try_new(read_buf.as_slice()).unwrap();
                let mut parser = Parser::new(reader);
                match parser.parse_command() {
                    Ok(cmd) => match cmd {
                        Command::ReadByte(addr) => match eeprom.read_byte(addr) {
                            Ok(b) => writeln!(tx, "data = {}\r", b).unwrap(),
                            Err(_) => writeln!(tx, "Could not read data!\r").unwrap(),
                        },
                        Command::WriteByte(addr, data) => match eeprom.write_byte(addr, data) {
                            Ok(_) => writeln!(tx, "Ok\r").unwrap(),
                            Err(_) => writeln!(tx, "Could not write data!\r").unwrap(),
                        },
                        Command::ReadData(_, _) => writeln!(tx, "ReadData\r").unwrap(),
                        Command::WritePage(_) => writeln!(tx, "WritePage\r").unwrap(),
                        Command::SetDevice(_) => writeln!(tx, "SetDevice\r").unwrap(),
                    },
                    Err(_) => writeln!(tx, "An error occured!\r").unwrap(),
                }

                break;
            }
        }
    }
}
