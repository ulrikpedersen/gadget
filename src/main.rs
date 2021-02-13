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

use cortex_m_semihosting::{hprintln, debug};

#[entry]
fn main() -> ! {
    let peripherals = stm32::Peripherals::take().unwrap();
    // rcc: reset clock control
    let mut rcc = peripherals.RCC.constrain();
    let core_peripherals = cortex_m::Peripherals::take().unwrap();
    let mut flash = peripherals.FLASH.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut delay = Delay::new(core_peripherals.SYST, clocks);
    let mut gpiob = peripherals.GPIOB.split(&mut rcc.ahb);

    // Configure the I2C pins and bus
    //    "moder": ??
    //    "afrl": alternate function low...?
    let scl = gpiob.pb6.into_af4(&mut gpiob.moder, &mut gpiob.afrl);
    let sda = gpiob.pb7.into_af4(&mut gpiob.moder, &mut gpiob.afrl);
    let i2c = i2c::I2c::new(peripherals.I2C1, (scl, sda), 400.khz(), clocks, &mut rcc.apb1);


    loop {
        // your code goes here
        hprintln!(".").unwrap();
        delay.delay_ms(1000_u16);
    }
}
