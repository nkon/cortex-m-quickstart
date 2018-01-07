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
* `#[linker_section=...]`は、リンカに対する指示。次に配列を割り込みベクタに配置する。サンプルのリンカファイル(`memory.x`)には `.vector_table.interrupts`は定義されていないが`cortex-m-rt`の`link.x`でセクションが定義されている。(このような相互参照の多さが気にかかっている点ではある)
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

デバイスクレートは、メーカから配布されている SVDファイルから `svd2rust`で作成するのが本筋だが、その後の手直しが多く求められるのが実情であり、今回は調整済みで配布されているのを使う。

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
`target/thumbv7m-none-eabi/debug/blinky1`にバイナリが生成されるが、それを gdb にロードする。
* `c(ontinue)`で、gdb上で実行される。

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
* `GPIOA`の`CRL`レジスタを設定して、PIN_5をプッシュプル・出力に設定する。一々インターフェイスの関数名を調べなければならないが、可読性が高い記述ができる。

```
            loop {
                gpioa.bsrr.write(|w| w.bs5().set());
                for _ in 1..4000 { unsafe { asm!(""); } }

                gpioa.bsrr.write(|w| w.br5().reset());
                for _ in 1..4000 { unsafe { asm!(""); } }

```
* BSRRにアクセスしてGPIOのピンを操作する。
    + セット側のビットセット(`BS5`)のアクセサは`set()`で、リセット側のビットセットは`reset()`になっていることに注意(`BSRR`は`BSn`のビットをセットすれば対応するI/Oがセットされ、*`BRn`をセット*すれば対応するI/Oが*リセット*される)。