use embedded_hal::blocking::delay::{DelayMs, DelayUs};
use embedded_hal::blocking::i2c;
use embedded_hal::serial;

use nb::block;

use arrayvec::ArrayVec;

use eeprom24x::{Eeprom24x, SlaveAddr};

use eeprom_programmer_command::parser::{Command, Parser};
use eeprom_programmer_command::reader::BufferReader;

pub fn main<TXD, RXD, I2C, DELAY, E>(
    mut uart_txd: TXD,
    mut uart_rxd: RXD,
    i2c: I2C,
    delay: DELAY,
) -> !
where
    TXD: serial::Write<u8> + core::fmt::Write,
    <TXD>::Error: core::fmt::Debug,
    RXD: serial::Read<u8>,
    <RXD>::Error: core::fmt::Debug,
    I2C: i2c::Write<Error = E> + i2c::Read<Error = E> + i2c::WriteRead<Error = E>,
    DELAY: DelayMs<u32> + DelayUs<u32>,
{
    // Create a readline buffer
    let mut read_buf = ArrayVec::<[u8; 32]>::new();

    // Create EEPROM device
    let eeprom_addr = SlaveAddr::Default;
    // 24x64 as a default device
    let mut eeprom = Eeprom24x::new_24x64(i2c, eeprom_addr);

    loop {
        write!(uart_txd, "> ").unwrap();
        read_buf.clear();
        loop {
            let c = block!(uart_rxd.read()).unwrap_or(b' ');
            if c == 0x08 {
                if read_buf.pop().is_some() {
                    block!(uart_txd.write(c)).unwrap();
                }
            } else if (0x20 <= c && c <= 0x7E) || c == b'\r' || c == b'\n' {
                if read_buf.try_push(c).is_ok() {
                    block!(uart_txd.write(c)).unwrap();
                }
            }
            if c == b'\n' {
                let reader = BufferReader::try_new(read_buf.as_slice()).unwrap();
                let mut parser = Parser::new(reader);
                match parser.parse_command() {
                    Ok(cmd) => match cmd {
                        Command::ReadByte(addr) => match eeprom.read_byte(addr) {
                            Ok(b) => writeln!(uart_txd, "data = {}\r", b).unwrap(),
                            Err(_) => writeln!(uart_txd, "Could not read data!\r").unwrap(),
                        },
                        Command::WriteByte(addr, data) => match eeprom.write_byte(addr, data) {
                            Ok(_) => writeln!(uart_txd, "Ok\r").unwrap(),
                            Err(_) => writeln!(uart_txd, "Could not write data!\r").unwrap(),
                        },
                        Command::ReadData(_, _) => writeln!(uart_txd, "ReadData\r").unwrap(),
                        Command::WritePage(_) => writeln!(uart_txd, "WritePage\r").unwrap(),
                        Command::SetDevice(_) => writeln!(uart_txd, "SetDevice\r").unwrap(),
                    },
                    Err(_) => writeln!(uart_txd, "An error occured!\r").unwrap(),
                }

                break;
            }
        }
    }
}
