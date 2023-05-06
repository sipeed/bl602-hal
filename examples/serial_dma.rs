#![no_std]
#![no_main]

use bl602_hal as hal;
use embedded_hal::delay::blocking::DelayMs;
use hal::{
    clock::{Strict, SysclkFreq, UART_PLL_FREQ},
    dma::single_buffer,
    dma::{DMAExt, DMA},
    pac,
    prelude::*,
    serial::*,
};

use panic_halt as _;

#[riscv_rt::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let mut parts = dp.GLB.split();

    // Set up all the clocks we need
    let clocks = Strict::new()
        .use_pll(40_000_000u32.Hz())
        .sys_clk(SysclkFreq::Pll160Mhz)
        .uart_clk(UART_PLL_FREQ.Hz())
        .freeze(&mut parts.clk_cfg);

    // Set up uart output. Since this microcontroller has a pin matrix,
    // we need to set up both the pins and the muxs
    let pin16 = parts.pin16.into_uart_sig0();
    let pin7 = parts.pin7.into_uart_sig7();
    let mux0 = parts.uart_mux0.into_uart0_tx();
    let mux7 = parts.uart_mux7.into_uart0_rx();

    // Configure our UART to 115200Baud, and use the pins we configured above
    let mut serial = Serial::new(
        dp.UART0,
        Config::default().baudrate(115_200.Bd()),
        ((pin16, mux0), (pin7, mux7)),
        clocks,
    );
    serial.link_dma(false, true);

    let dma = DMA::new(dp.DMA);
    let channels = dma.split();
    let mut channel = channels.ch0;

    let mut tx_buf = include_bytes!("serial_dma.txt");

    // Create a blocking delay function based on the current cpu frequency
    let mut d = bl602_hal::delay::McycleDelay::new(clocks.sysclk().0);

    loop {
        let dma_config = single_buffer::Config::new(channel, tx_buf, serial);
        let dma_transfer = dma_config.start();

        // Blocking wait, this can also be done by listening for the *transfer complete* interrupt.
        (channel, tx_buf, serial) = dma_transfer.wait();

        d.delay_ms(1000).unwrap();
    }
}

