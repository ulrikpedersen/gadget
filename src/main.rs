#![no_std]
#![no_main]

// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
// use panic_itm as _; // logs messages over ITM; requires ITM support
// use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

//use cortex_m::asm;
use cortex_m_rt::entry;
use stm32f3xx_hal::{prelude::*, stm32, i2c};
//use stm32f3xx_hal::gpio::gpiob;
use stm32f3xx_hal::delay::Delay;
use bme280::BME280;

use cortex_m_semihosting::hprintln;

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
    let mut _measurements = bme280.measure().unwrap();
    
    loop {
        // measure temperature, pressure, and humidity
        _measurements = bme280.measure().unwrap();
        // hprintln!("{}% {} deg C {} pascals", 
        //     _measurements.humidity, 
        //     _measurements.temperature, 
        //     _measurements.pressure)
        //     .unwrap();
    }
}
