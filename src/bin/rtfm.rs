#![no_std]
#![feature(asm)]
#![feature(proc_macro)]

extern crate cortex_m;
extern crate cortex_m_rtfm as rtfm;  // 必ずリネームすること
extern crate stm32f103xx;

use cortex_m::asm;
use cortex_m::peripheral::SystClkSource;
// use core::fmt::Write;
// use semihosting::hio;
use stm32f103xx::Interrupt;
use rtfm::{app, Threshold, Resource};

pub struct Led {
    on: bool,
}

impl Led {
    pub fn is_on(&self) -> bool {
        self.on
    }

    pub fn blink(&mut self, gpio: &mut ::stm32f103xx::GPIOA) {
        self.on = !self.on;
        if self.on {
            gpio.bsrr.write(|w| w.bs5().set());
        } else {
            gpio.bsrr.write(|w| w.br5().reset());
        }
    }
}

app!{
    device: stm32f103xx,

    resources: {
        static LED: Led = Led{on: false};
        static COUNT: u32 = 0;
        static INTERVAL: u32 = 0;
    },
    tasks: {
        SYS_TICK: {
            path: sys_tick,
            priority: 2,
            resources: [
                LED,
                GPIOA,
                COUNT,
                INTERVAL,
            ],
        },
        EXTI15_10 : {
            path: exti13,
            priority: 1,
            resources: [
                GPIOC,
                EXTI,
                LED,
                INTERVAL,
            ],
        },
    },
}

fn init(p: init::Peripherals, r: init::Resources) {
//    writeln!(hio::hstdout().unwrap(), "Hello, ").unwrap();

    // PA5(LD2)を Output, Pushpullにする
    p.RCC.apb2enr.modify(|_, w| w.iopaen().enabled()); // GPIOAにバスクロックを供給する
    p.GPIOA.crl.modify(|_, w| w.mode5().output().cnf5().push()); // チェーン記法も可

    // PC13(B1)を Input, EXTI13(falling edgh)にする
    p.RCC.apb2enr.modify(|_, w| w.iopcen().enabled().afioen().enabled()); // GPIOCとAFIOにバスクロックを供給する
    p.GPIOC.crh.modify(|_, w| w.mode13().input());  // default=input なので省略可
    p.EXTI.imr.modify(|_, w| w.mr13().set_bit());
    p.EXTI.ftsr.modify(|_, w| w.tr13().set_bit());
    unsafe {p.AFIO.exticr4.modify(|_, w| w.exti13().bits(0b0000_0010));} //bits() は unsafe
//    p.NVIC.enable(Interrupt::EXTI15_10); // framework が EXIT15_10をenableしてくれる

    // SysTickを設定し 10ms毎に割り込みがかかるようにする
    p.SYST.set_clock_source(SystClkSource::Core);
//    p.SYST.set_reload(p.SYST.get_ticks_per_10ms());
//    データシートによると carib=9000 が get_ticks_per_10ms()から帰ってくる。
//    デフォルトの設定では HSI=8MHzがそのままSYSCLKになるので8_000*10が正しい10ms。
    p.SYST.set_reload(8_000*10);
    p.SYST.enable_interrupt();
    p.SYST.enable_counter();

    **r.INTERVAL = 100; // * 10ms
}

fn idle() -> !{
//    writeln!(hio::hstdout().unwrap(), "world!").unwrap();

    loop {
        rtfm::wfi();
    }
}

fn sys_tick(_t: &mut Threshold, r: SYS_TICK::Resources) {
    **r.COUNT += 1;
    if **r.COUNT >= **r.INTERVAL {  // LEDが点灯中は INTERVAL が勝手に変わって欲しくない
        **r.COUNT = 0;
        r.LED.blink(r.GPIOA); // 反転
    } else {
        return;
    }
}

fn exti13(t: &mut Threshold, mut r: EXTI15_10::Resources) {
    rtfm::set_pending(Interrupt::EXTI15_10);
    if r.GPIOC.idr.read().idr13().bit_is_clear() {
        r.EXTI.pr.modify(|_, w| w.pr13().clear_bit());

        loop {
            let mut is_break = false;
            r.LED.claim_mut(t, |led, _t| {
                if !led.is_on() {
                    is_break = true;
                }
            });
            if is_break { break; }
        }

        r.INTERVAL.claim_mut(t, |interval, _t| {
            if **interval == 100 { // ココでINTERVAL を変更する。
                **interval = 20;
            } else {
                **interval = 100;
            }
        });
    }
}

// --debug 時はコレが必要
// --release 時は不要
#[no_mangle]
pub fn rust_begin_unwind() {
    asm::nop();
}
