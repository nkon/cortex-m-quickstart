#![no_std]
#![feature(asm)]

extern crate cortex_m;
extern crate stm32f103xx;

use stm32f103xx::{GPIOA, RCC};

// Nucleo boardでは LED(LD2)はPA5に、ボタン(B1)はPC13に接続されている。

fn main() {
    cortex_m::interrupt::free(
        |cs| {
            let rcc = RCC.borrow(cs);
            let gpioa = GPIOA.borrow(cs);

            rcc.apb2enr.modify(|_, w| w.iopaen().enabled());
            gpioa.crl.modify(|_, w| w.cnf5().push());
            gpioa.crl.modify(|_, w| w.mode5().output());

            loop {
                gpioa.bsrr.write(|w| w.bs5().set());
                for _ in 1..4000 { unsafe { asm!(""); } }

                gpioa.bsrr.write(|w| w.br5().reset());
                for _ in 1..4000 { unsafe { asm!(""); } }
            }
        }
    )
}
