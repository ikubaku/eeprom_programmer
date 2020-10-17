//#![deny(warnings)]
#![no_main]
#![no_std]

use core::fmt::Write as Core_Write;

use cortex_m_rt as rt;
use panic_halt as _;
use rt::entry;
use stm32g0xx_hal as hal;

use hal::delay::DelayExt;
use hal::gpio::GpioExt;
use hal::hal::digital::v2::OutputPin;
use hal::i2c;
use hal::i2c::I2cExt;
use hal::rcc::RccExt;
use hal::serial::FullConfig;
use hal::serial::SerialExt;
use hal::stm32;
use hal::time::U32Ext;

mod app;

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
    let (tx, rx) = uart.split();

    let mut tx = tx;

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
    let i2c = dp
        .I2C1
        .i2c(sda, scl, i2c::Config::new(100_000.hz()), &mut rcc);

    app::main(tx, rx, i2c, delay);
}
