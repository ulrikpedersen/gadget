#![no_std]
#![no_main]

// pick a panicking behavior
// use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
// use panic_abort as _; // requires nightly
use panic_itm as _; // logs messages over ITM; requires ITM support
// use panic_semihosting as _; // logs messages to the host stderr; requires a debugger. Dont work with openocd - use only with qemu.

//use cortex_m::asm;
use cortex_m_rt::entry;
use core::fmt::Write;   // Required by the ssd1306 TerminalMode to use write! macro and string formatting

use stm32f3xx_hal::{prelude::*, stm32, i2c, pac};
use stm32f3xx_hal::spi::{Mode, Phase, Polarity, Spi};
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

// LEDs on the discovery board
use stm32f3_discovery::leds::Leds;
use stm32f3_discovery::switch_hal::OutputSwitch;

// Our application specific hardware drivers
use shared_bus::BusManagerSimple;
use bme280::BME280;  // The temperature, humidity and pressure sensor
use ssd1306::{prelude::*, Builder, I2CDIBuilder};  // oled display

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
    let mut gpioa = peripherals.GPIOA.split(&mut rcc.ahb);

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
    let i2c_bus_manager = BusManagerSimple::new(i2c);

    info!("Configuring the bme280 sensor driver");
    // Temperature, humidity and pressure sensor on i2c bus...
    let mut bme280 = BME280::new_primary(i2c_bus_manager.acquire_i2c(), delay);
    // initialize the sensor
    bme280.init().unwrap();

    info!("Configuring ssd1306 oled display driver");
    let interface = I2CDIBuilder::new().init(i2c_bus_manager.acquire_i2c());
    let mut disp: TerminalMode<_, _> = Builder::new().connect(interface).into();
    disp.init().unwrap();
    let _ = disp.clear();
    let _ = disp.write_str("Display is live!");

    info!("Configuring the Discovery board user LEDs");
    let mut gpioe = peripherals.GPIOE.split(&mut rcc.ahb);
    let leds = Leds::new(
        gpioe.pe8,  gpioe.pe9,  gpioe.pe10, gpioe.pe11,
        gpioe.pe12, gpioe.pe13, gpioe.pe14, gpioe.pe15,
        &mut gpioe.moder, &mut gpioe.otyper,
    );
    let mut led_circle = leds.into_array();

    info!("Configuring SPI on PA5, PA5 and PA7");
    let sck = gpioa.pa5.into_af5(&mut gpioa.moder, &mut gpioa.afrl);
    let miso = gpioa.pa6.into_af5(&mut gpioa.moder, &mut gpioa.afrl);
    let mosi = gpioa.pa7.into_af5(&mut gpioa.moder, &mut gpioa.afrl);

    let spi_mode = Mode {
        polarity: Polarity::IdleLow,
        phase: Phase::CaptureOnFirstTransition,
    };

    // NOTE: have to explicitly tell the compiler the type here. 
    //       It doesn't know what WORD is (can be u8 or maybe u16 I guess...)
    let mut spi : Spi<_, _, u8> = Spi::spi1(
                peripherals.SPI1,
                (sck, miso, mosi),
                spi_mode,
                3.mhz(),
                clocks,
                &mut rcc.apb2,
            );

    info!("sending 0xDEADBEEF via spi..");
    // Create an `u8` array, which can be transfered via SPI.
    let msg_send: [u8; 8] = [0xD, 0xE, 0xA, 0xD, 0xB, 0xE, 0xE, 0xF];
    // Copy the array, as it would be mutually shared in `transfer` while simultaneously would be
    // immutable shared in `assert_eq`.
    let mut msg_sending = msg_send;
    // Transfer the content of the array via SPI and receive it's output.
    // When MOSI and MISO pins are connected together, `msg_received` should receive the content.
    // from `msg_sending`
    let msg_received = spi.transfer(&mut msg_sending).unwrap();
    info!("msg_received: {:#?}", msg_received);

    
    info!("Let's take the temperature!");
    let mut _measurements = bme280.measure().unwrap();
    let _ = disp.clear();
    loop {
        for led in led_circle.iter_mut() {
            // Light up the next LED (spinnning the circle)
            led.on().ok();

            // Reset the text position to top left to over-write from the previous iteration.
            // Works better than disp.clear which makes the display flicker.
            let _ = disp.set_position(0, 0);
            // measure temperature, pressure, and humidity
            _measurements = bme280.measure().unwrap();
            info!("{}% {} deg C {} pa", 
                _measurements.humidity, 
                _measurements.temperature, 
                _measurements.pressure);
            // Write the data to oled display. Set fixed width fields in formatting.
            let _ = write!(disp, 
                "{humidity:>9.2}%\n\n{temp:>9.2} degC\n\n{pressure:>8.1} pa", 
                humidity=_measurements.humidity, 
                temp=_measurements.temperature, 
                pressure=_measurements.pressure);

            led.off().ok();
        }
    }

    #[allow(unreachable_code)] {
        panic!("We should never get here - we've fallen out of an infinite loop!");
    }
}
