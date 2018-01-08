# プロジェクトテンプレートをコピー

```
$ cargo clone cortex-m-quickstart
$ cd cortex-m-quickstart
```

`cargo clone`サブコマンドが無い場合は`cargo install cargo-clone`でインストールする。

## 設定の編集
必要に応じて`Cargo.toml`を編集する。手元で動作させるならコレぐらいで。
```
[package]
name = "cortex-m-quickstart"
version = "0.1.0"
```

## ターゲットの設定
`.cargo/config`に、実際のビルドターゲットを書いておくと、毎回、オプションで指定しなくても良い。
```
[build]
target="thumbv7m-none-eabi"
```
* `thumbv6m-none-eabi`: Cortex-M0+
* `thumbv7m-none-eabi`: Cortex-M3, STM32F103はコレ
* `thumbv7em-none-eabi`: Cortex-M4
* `thumbv7em-none-eabihf`: Cortex-M7

## メモリマップの設定
このテンプレートでは`memory.x`がリンカスクリプトになっているので、メモリマップを設定する。

STM32F103RBの場合は次のとおり。
```
MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 128K
  RAM   : ORIGIN = 0x20000000, LENGTH = 20K
}
```

# ビルト＆実行

## example/hello.rs
いきなりだが `example/hello.rs`をビルドしよう。
```
$ xargo build --example hello
```
エラーが出た。
```
Undefined reference to `rust_begin_unwind'
```

`example/hello.rs`に次を加えて応急処置。
```
#[no_mangle]
pub fn rust_begin_unwind() {
    asm::nop();
}
```

## 逐行解説

```
#![feature(used)]
```
このクレート内で `#[used]` feature を使うことを宣言する。
```
#![no_std]
```
このクレート内で libstd を使わないことを宣言する。

```
extern crate cortex_m;
extern crate cortex_m_rt;
extern crate cortex_m_semihosting;
```
外部クレートとしてこれらを使う。参照先は`Cargo.toml`に記述する。
* `cortex_m`: Cortex-Mへの Low Level API。`asm`,`exception`,`interrupt`,`itm`, `peripheral`, `register`のサブクレートがある。 https://crates.io/crates/cortex-m
* `cortex_m_rt`: https://crates.io/crates/cortex-m-rt
* `cortex_m_semihosting`: 標準入出力をデバッガに表示する。https://crates.io/crates/cortex-m-semihosting

```
use core::fmt::Write;
```
文字列整形する。
```
use cortex_m::asm;
use cortex_m_semihosting::hio;
```
上でインポートしたクレートに短縮名を付けて使う。例えば`cortex_m::asm`ではなく`asm`と書ける。`hio`はHost Standard I/O の略。`stdin`のかわりに`hstdin`が使える。
```
fn main() {
    let mut stdout = hio::hstdout().unwrap();
    writeln!(stdout, "Hello, world!").unwrap();
}
```
`hstdout`に"Hello, world!"と表示する。
```
// As we are not using interrupts, we just register a dummy catch all handler
#[link_section = ".vector_table.interrupts"]
#[used]
static INTERRUPTS: [extern "C" fn(); 240] = [default_handler; 240];
```
* `#[used]`は、指定した static 変数が、最適化で消えてしまわないようにする指示。Cの`volatile`のようなもの。
* `#[linker_section=...]`は、リンカに対する指示。次に配列を割り込みベクタに配置する。サンプルのリンカファイル(`memory.x`)には `.vector_table.interrupts`は定義されていないが`cortex-m-rt`の`link.x`でセクションが定義されている(このような相互参照の多さが気にかかっている点ではある)。
* `INTERRUPT`という`extern "C" fn()`型の 240要素の `static`(でimmutable) な配列を定義し、`default_handler`×240個で初期化する。
```
extern "C" fn default_handler() {
    asm::bkpt();
}
```
デフォルトハンドラを定義する。中身は、break 命令。
```
#[no_mangle]
pub fn rust_begin_unwind() {
    asm::nop();
}
```
リンクエラーとなった`rust_begin_unwind`のダミーを定義する。

## gdb で実行

### GDBの設定とgdb-dashboard のインストール

* https://github.com/cyrus-and/gdb-dashboard から `.gdbinit`をダウンロードして`~/.gdbinit`におく。
* `~/.gdbinit`の`syntax_highlighting`で、青色になっている部分が非常に見にくいので、`vim`となっている部分を``に修正する。
* 末尾付近に `set auto-load safe-path /`を追加する。コレによって、`./.gdbinit`をロードしてくれる。そして、そこには`target remote :3333`などが書かれている。

## gdbで実行

OpenOCDサーバを実行した状態で、gdbを起動する。
```
$ arm-none-eabi-gdb target/thumbv7m-none-eabi/debug/examples/hello
```
`c(ontinue)`で実行した時に、OpenOCDの方に、`Hello, world!`と表示されればOK。

# Lチカ

## デバイスサポートクレートの追加

デバイスクレートは、メーカーから配布されている SVDファイルから `svd2rust`で作成するのが本筋だが、その後の手直しが多く求められるのが実情である。今回は調整済みで配布されているのを使う。

* `Cargo.toml`に次を追加。
```
[dependencies.stm32f103xx]
features = ["rt"]
version = "0.7.*"
```

* `src/bin/blinky1.rs`に次を写経。
```
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
```
* ビルド
```
$ xargo build --bin blinky
```
`src/bin/blinky.rs`をバイナリークレートとしてビルドする。
* 実行
```
$ xargo run --bin blinky
```
`.cargo/config`に`runner = 'arm-none-eabi-gdb'`が設定されていると、
`target/thumbv7m-none-eabi/debug/blinky1`に生成されたバイナリを gdb にロードするところまでやってくれる(別途OpenOCDサーバが起動している必要がある)。
* `c(ontinue)`で、gdb上で実行される。

## 単体での実行
* OpenOCDを使って書き込み、単体で動かすにはこうすれば良い。
```
$ openocd -f board/st_nucleo_f103rb.cfg -c "init" -c "reset init" -c "stm32f1x mass_erase 0" -c "flash write_image target/thumbv7m-none-eabi/debug/blinky1" -c "reset halt" -c "reset run" -c "exit"
```

## 逐行解説

```
extern crate cortex_m;
extern crate stm32f103xx;
use stm32f103xx::{GPIOA, RCC};
```
今回の範囲では`cortex_m`のみで良い。`GPIOA`と`RCC`を使う。

ターゲットボードのマニュアルを参照すると、Nucleo boardでは LED(LD2)はPA5に、ボタン(B1)はPC13に接続されていることがわかる。

```
fn main() {
    cortex_m::interrupt::free(
        |cs| {
```
`cortex-m`クレートの`interrupt`モジュールの機能で、freeというのがある。CriticalSectionを作り、その中での割り込みを禁止する。各レジスタは、CSごとに借用を行い、専用のアクセス関数(`stm32f103xx`で定義される)で、安全にアクセスする。

```
            let rcc = RCC.borrow(cs);
            let gpioa = GPIOA.borrow(cs);
```
`cs`に対応した `RCC`と`GPIOA`の借用を得る。
```
            rcc.apb2enr.modify(|_, w| w.iopaen().enabled());
            gpioa.crl.modify(|_, w| w.cnf5().push());
            gpioa.crl.modify(|_, w| w.mode5().output());
```
* `RCC`の`APB2ENR`レジスタを修正して、GPIOAにクロックを供給する。
* 修正するときは `modify(r,w)`を使う。今回は`r(ead)`は使わないので`_`としてある。
* `APB2ENR`レジスタの`IOPAEN`ビットを`ENABLE(1)`にする。
    + どのレジスタのどのビットを操作しなければならないのかは、リファレンスマニュアルを参照する。
    + レジスタへのアクセス関数については `stm32f103xx`クレートのマニュアル(https://docs.rs/stm32f103xx/0.7.5/stm32f103xx/)を参照する。
* `GPIOA`の`CRL`レジスタを設定して、PIN_5をプッシュプル・出力に設定する。一々インタフェースの関数名を調べなければならないが、可読性が高い記述ができる。

```
            loop {
                gpioa.bsrr.write(|w| w.bs5().set());
                for _ in 1..4000 { unsafe { asm!(""); } }

                gpioa.bsrr.write(|w| w.br5().reset());
                for _ in 1..4000 { unsafe { asm!(""); } }

```
* BSRRにアクセスしてGPIOのピンを操作する。
    + セット側のビットセット(`BS5`)のアクセサは`set()`で、リセット側のビットセットは`reset()`になっていることに注意(`BSRR`は`BSn`のビットをセットすれば対応するI/Oがセットされ、*`BRn`をセット*すれば対応するI/Oが*リセット*される)。
* ビジーループ。

# RTFM Framework

japaricさんの cortex-m-rtfm フレームワークを使ってみる。これはライブラリではなくフレームワークなので、完全にその世界のルールに従う必要がある。マクロを使いまくっているのて、見た目もかなり異なるDSLみたいなものだ。

`cargo-edit` サブコマンド群がインストールされていれば、次で、`Cargo.toml`に`cortex-m-rtfm`のエントリーが追加される。
```
$ cargo add cortex-m-rtfm
```
`cargo-edit`を追加するには次のようにすれば良い。
```
$ cargo install cargo-edit
```

## ボタンを押したら点滅周期が変わるようにしよう

## 写経
```
#![no_std]
#![feature(asm)]
#![feature(proc_macro)]

extern crate cortex_m;
extern crate cortex_m_rtfm as rtfm;  // 必ずリネームすること
extern crate stm32f103xx;

use cortex_m::asm;
use cortex_m::peripheral::SystClkSource;
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
            resources: [LED, GPIOA, COUNT, INTERVAL],
        },
        EXTI15_10 : {
            path: exti13,
            priority: 1,
            resources: [GPIOC, EXTI, LED, INTERVAL],
        },
    },
}

fn init(p: init::Peripherals, r: init::Resources) {
    // PA5(LD2)を Output, Pushpullにする
    p.RCC.apb2enr.modify(|_, w| w.iopaen().enabled());
    p.GPIOA.crl.modify(|_, w| w.mode5().output().cnf5().push());

    // PC13(B1)を Input, EXTI13(falling edgh)にする
    p.RCC.apb2enr.modify(|_, w| w.iopcen().enabled().afioen().enabled());
    p.EXTI.imr.modify(|_, w| w.mr13().set_bit());
    p.EXTI.ftsr.modify(|_, w| w.tr13().set_bit());
    unsafe {p.AFIO.exticr4.modify(|_, w| w.exti13().bits(0b0000_0010));}

    // SysTickを設定し 10ms毎に割り込みがかかるようにする
    p.SYST.set_clock_source(SystClkSource::Core);
    p.SYST.set_reload(8_000*10);
    p.SYST.enable_interrupt();
    p.SYST.enable_counter();

    **r.INTERVAL = 100; // * 10ms
}

fn idle() -> !{
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
```

## 逐行解説

```
#![no_std]
#![feature(asm)]
#![feature(proc_macro)]

extern crate cortex_m;
extern crate cortex_m_rtfm as rtfm;  // 必ずリネームすること
extern crate stm32f103xx;
```
`cortex_m_rtfm`は`rtfm`と名前を変えて取り込む
```
use cortex_m::asm;
use cortex_m::peripheral::SystClkSource;
use stm32f103xx::Interrupt;
use rtfm::{app, Threshold, Resource};
```
* `cortex_m`
    + `asm`: `nop()`などで使うので。
    + `SystClkSource`: `SysTick`の設定のため。
* `stm32f103xx`
    + `GPIOA`などのペリフェラルは`app!`の`device:`で指定することで、自動的に取り込まれる。
    + しかし、`Interrupt`は手動で取り込まなければならない。
* `rtfm`
    + コンパイラの指定に従って。
    + `Peripherals`がなぜ不要かよくわからない。
```
pub struct Led {
    on: bool,
}
```
* LEDの状態を管理するクラスを作ってみる。
* クラス名はスネークケース。
* `pub`にしておかなれけばならない。
```
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
```
* blink の引数として`GPIOA`をとる。
    + `app!`→割り込みハンドラ経由で借りてこなければならないのて、引数として与えなければならない。
    + 上手いこと定義時に関連付けられたら良いのだが。
```
app!{
    device: stm32f103xx,
```
`device:`でデバイスクレートを指定する。
```
    resources: {
        static LED: Led = Led{on: false};
        static COUNT: u32 = 0;
        static INTERVAL: u32 = 0;
    },
```
* `resource:`では、ユーザ定義の共有リソースを定義する。
    + ペリフェラルなどのハードウェアリソースは、フレームワークで定義される。
    + `cargo expand`すると、マクロ展開後の状態が見れる。
```
    tasks: {
        SYS_TICK: {
            path: sys_tick,
            priority: 2,
            resources: [LED, GPIOA, COUNT, INTERVAL],
        },
        EXTI15_10 : {
            path: exti13,
            priority: 1,
            resources: [GPIOC, EXTI, LED, INTERVAL],
        },
    },
}
```
* 割り込みハンドラに対応したタスクを宣言する。
    + タスク名は、割り込みハンドラの名前。
    + `path:`は、ハンドラの関数名。
    + `priority:`は、数字が大きいほど優先度が高い。
    + `recource:`は、ユーザ定義のリソースと、ペリフェラルの両方を指定する。
```
fn init(p: init::Peripherals, r: init::Resources) {
```
* 初期化関数の名前は`init`固定。
    + このように、ペリフェラルとユーザ定義の2つの引数を取る。
    + `init()`は、排他状態で実行される。
```
    // PA5(LD2)を Output, Pushpullにする
    p.RCC.apb2enr.modify(|_, w| w.iopaen().enabled());
    p.GPIOA.crl.modify(|_, w| w.mode5().output().cnf5().push());
```
* `stm32f103xx`クレートの機能を使って、ペリフェラルを初期化する。
    + GPIOを使うには、まずRCCを操作して、GPIOにクロックを供給する。
        - GPIOAは APB2 バスにつながっているので、`APB2ENR`レジスタの対応するビット(`IOPAEN`)をONにする。
        - レジスタの一部のビットを修正するので`modify`を使う。
        - クロージャのひとつ目の引数は `r`だが、今回は使わないのて`_`としてある。
    + `GPIOA`のコントロールレジスタを操作して、LEDがつながっているポート(PA5)を Output Pushpullにする。
        - コントロールレジスタは、High/Lowに分かれており、Pin5は`CRL`(Low側)である。`MODE5`ビットと`CNF5`ビットをセットするが、`output()`、`push()`という可読性の高いアクセサが定義されている。
        - このようにチェイン表記が可能である。
```
    // PC13(B1)を Input, EXTI13(falling edgh)にする
    p.RCC.apb2enr.modify(|_, w| w.iopcen().enabled().afioen().enabled());
    p.EXTI.imr.modify(|_, w| w.mr13().set_bit());
    p.EXTI.ftsr.modify(|_, w| w.tr13().set_bit());
    unsafe {p.AFIO.exticr4.modify(|_, w| w.exti13().bits(0b0000_0010));}
```
* ボタンの対応するPC13を、割り込み入力に初期化する。
    + 端子は外部プルアップされた、アクティブLowなので、Input, floating, Falling Edgeに設定する。
    + まずは`GPIOC`にクロックを供給(同上、上で同時にやっても良い)。
    + 入力、フローティングの設定は`p.GPIOC.crh.modify(|_, w| w.mode13().input());`だがデフォルトでこうなので省略可能。
    + `EXTI`の`IMR`(Interrupt Mask Register)のビットをセットして割り込みマスクを解除する。
    + `EXTI`の`FTSR`(Falling Trigger Set Register)をセットして、立ち下がりエッジで割り込みがかかるようにする。
    + `AFIO`の`EXTI13`ビット(`AFIO_EXTICR4`にある)の`PCx`に対応するビット('0b0010')をセットする。ここはアクセサが無いので`bits`でセットしなければならないが、`bit`はunsafeなので`unsafe{}`で囲む。この辺、まだ未整備である。
    + これらの設定をして、初めて EXTIが正常に動作する。CubeMXでは裏側でやってくれていたが、ここではデータシートを理解して手動設定が必要である。うまくフレームワークで隠してくれると便利なのだが。
    + 本来は`p.NVIC.enable(Interrupt::EXTI15_10);`のように、`NVIC`を操作して、割り込みを有効化しなければならないが、これはフレームワークがやってくれる(`xargo expand ...`)。 
```
    // SysTickを設定し 10ms毎に割り込みがかかるようにする
    p.SYST.set_clock_source(SystClkSource::Core);
    p.SYST.set_reload(8_000*10);
    p.SYST.enable_interrupt();
    p.SYST.enable_counter();
```
* `cortex_m::SYST`を使ってSysTickを設定する。
    + SysTickは、チップ固有ペリフェラルではなく、Cortex-Mの機能であることに注意。
    + `SYST`のドキュメントを見ると`set_reload`の引数に`get_tics_per_10ms()`を与えれば良いと思うかもしれないが、STM32F103では、これは9000を返すだけである。設定したクロックツリーを元に正しいカウンタ値を手書きしなければならない。
```
    **r.INTERVAL = 100; // * 10ms
```
* ユーザ定義リソースを初期化するには、`resource:`で初期化するのが普通でと思うが、こうしても良い。
    + `**r.INTERVAL`のように`**r`と2回 De-referenceする。
```
fn idle() -> !{
    loop {
        rtfm::wfi();
    }
}
```
* `idle()`は、通常このようにすれば`WFI`でスタンバイ状態に落ちる。
```
fn sys_tick(_t: &mut Threshold, r: SYS_TICK::Resources) {
    **r.COUNT += 1;
    if **r.COUNT >= **r.INTERVAL {  // LEDが点灯中は INTERVAL が勝手に変わって欲しくない
        **r.COUNT = 0;
        r.LED.blink(r.GPIOA); // 反転
    } else {
        return;
    }
}
```
* `SYS_TICK`の割り込みハンドラ。
    + 10ms周期で呼ばれるたびに`COUNT`をインクリメントし、INTERVALを超えていたら、LEDを`blink()`させる。
    + `Threshold`と`Resources`を引数に取るが、`Threshold`は、今回は使わない。
    + `Resources`は`SYS_TICK::Resouces`となっており、定義時に`resource:`に指定したリソースが渡される。
    + 中身を参照するには、2回 De-referenceする。
    + `INTERVAL`や`LED`は`exti13`とリソースを共用しているが、こちらのほうが`priority:`が高いので、こちらではあまり気にせず使える。
    + LEDを`blink()`させるときに、引数で渡されてきた`GPIOA`を渡す。
```
fn exti13(t: &mut Threshold, mut r: EXTI15_10::Resources) {
```
* exti13の方は `Threshold` も使うし、`Resources` は `mut`である。
```
    rtfm::set_pending(Interrupt::EXTI15_10);
```
* 割り込みをペンディングする。コレも定形処理なのでフレームワークで面倒を見てほしいものだ。
```
    if r.GPIOC.idr.read().idr13().bit_is_clear() {
```
* STM32の場合、EXTI15〜EXTI10までが1つの割り込みハンドラに飛んでくるので、GPIOの該当するピンを見て、本当に欲しい入力かを確認しなければならない。
```
        r.EXTI.pr.modify(|_, w| w.pr13().clear_bit());
```
* `EXTI`の方のペンディングビットをクリアする。
```
        loop {
            let mut is_break = false;
            r.LED.claim_mut(t, |led, _t| {
                if !led.is_on() {
                    is_break = true;
                }
            });
            if is_break { break; }
        }
```
* LEDが点いていたら待つ処理を、(わざとナイーブに)書いてみた。
    + クロージャの中からは直線`break`出来ないのて、`is_break`というフラグ(`bool`と推定される)を使う。
    + `r.LED`は`SYS_TICK`でも使われているのて、優先度が低いこちらでは、`claim_mut`でローカルクリティカルセクションが許可されている時のみ変更できる。
```
        r.INTERVAL.claim_mut(t, |interval, _t| {
            if **interval == 100 { // ココでINTERVAL を変更する。
                **interval = 20;
            } else {
                **interval = 100;
            }
        });
```
* こちらも同様。
    + LEDはオブジェクトだが、`INTERVAL`は変数なので2回 De-referenceされている(ここのところよくわからないが、コンパイルエラーのいうがまま)。
```
// --debug 時はコレが必要
// --release 時は不要
#[no_mangle]
pub fn rust_begin_unwind() {
    asm::nop();
}
```
* コメントのとおりなのだが、謎。



# 感想
* 初期化、定形処理をリファレンスマニュアル見ながら書くのは(復讐にはなるが)めんどくさい。CubeMXの力を感じる。
* cortex-m: きちんと借用を処理していること、SVDから、半自動でデバイスクレートを生成して、しかも可読性に優れていることは大きな魅力である。
* cortex-m: semihosting はデバッグに便利。
* RTFM: リソースの定義やタスクの定義がトップダウンでわかりやすい。
* RTFM: ハンドラの書き方など、まだまだドキュメント不足。引数のリファレンスのほどき方など、サンプルコード頼みだ。コンパイラのエラーメッセージが親切なことに助けられている。
* RTFM: 結局、Non-OSの割り込みイベントハンドラのラッパフレームワークであり、RTOSではない。つまり、複数のアプリケーションタスクを並行して走らせることへの助けは無い。手書きや、拙作の STM32Cube ラッパーと、そう労力は変わらないと感じた。CMSIS-RTOS(及びその一実装であるFreeRTOS)のラッパを待つ、または、作る方が将来らいはあるだろう。
