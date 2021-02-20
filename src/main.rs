#![no_std]
#![no_main]

// pick a panicking behavior
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
use panic_itm as _; // logs messages over ITM; requires ITM support
// use panic_semihosting as _; // logs messages to the host stderr; requires a debugger. Dont work with openocd - use only with qemu.

//use cortex_m::asm;
use cortex_m_rt::entry;

use stm32f3xx_hal::{prelude::*, stm32, i2c};
//use stm32f3xx_hal::gpio::gpiob;
use stm32f3xx_hal::delay::Delay;

// For logging
use lazy_static::lazy_static;
use log::LevelFilter;
pub use cortex_m_log::log::Logger;
use cortex_m_log::{
    destination::Itm as ItmDest,
    printer::itm::InterruptSync,
    modes::InterruptFree,
    printer::itm::ItmSync
};
use log::{info, warn};

// Our application specific hardware drivers
use bme280::BME280;  // The temperature, humidity and pressure sensor

#[entry]
fn main() -> ! {
    let peripherals = stm32::Peripherals::take().unwrap();
    // rcc: reset clock control
    let mut rcc = peripherals.RCC.constrain();
    let core_peripherals = cortex_m::Peripherals::take().unwrap();
    let mut flash = peripherals.FLASH.constrain();  // why do we need this flash?
    let clocks = rcc.cfgr.freeze(&mut flash.acr);   // what does the 'freeze' function do?
    let delay = Delay::new(core_peripherals.SYST, clocks);
    let mut gpiob = peripherals.GPIOB.split(&mut rcc.ahb);

    // Setting up the logging framework. Steals the ITM so it can't be used directly!
    lazy_static! {
        static ref LOGGER: Logger<ItmSync<InterruptFree>> = Logger {
            level: LevelFilter::Info,
            inner: unsafe {
                InterruptSync::new( ItmDest::new(cortex_m::Peripherals::steal().ITM) )
            },
        };
    }
    cortex_m_log::log::init(&LOGGER).unwrap();
    warn!("Board basically setup now. Let's go!");

    info!("Configure i2c bus on PB6 and PB7");
    // Configure the I2C pins and bus
    //    "moder": ??
    //    "afrl": alternate function low...?
    let scl = gpiob.pb6.into_af4(&mut gpiob.moder, &mut gpiob.afrl);
    let sda = gpiob.pb7.into_af4(&mut gpiob.moder, &mut gpiob.afrl);
    let i2c = i2c::I2c::new(peripherals.I2C1, (scl, sda), 400.khz(), clocks, &mut rcc.apb1);

    // Temperature, humidity and pressure sensor on i2c bus...
    let mut bme280 = BME280::new_primary(i2c, delay);
    // initialize the sensor
    bme280.init().unwrap();

    info!("Let's take the temperature!");
    let mut _measurements = bme280.measure().unwrap();
    
    loop {
        // measure temperature, pressure, and humidity
        _measurements = bme280.measure().unwrap();
        info!("{}% {} deg C {} pa", 
            _measurements.humidity, 
            _measurements.temperature, 
            _measurements.pressure);
    }
    
    #[allow(unreachable_code)] {
        panic!("We should never get here - we've fallen out of an infinite loop!");
    }
}
