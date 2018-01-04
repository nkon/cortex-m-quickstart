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

## gdb で実行

### GDBの設定

### gdb-dashboard のインストール

* https://github.com/cyrus-and/gdb-dashboard から `.gdbinit`をダウンロードして`~/.gdbinit`におく。
* `~/.gdbinit`の`syntax_highlighting`で、青色になっている部分が非常に見にくいので、`vim`となっている部分を``に修正する。
* 末尾付近に `set auto-load safe-path /`を追加する。コレによって、`./.gdbinit`をロードしてくれる。そして、そこには`target remote :3333`などが書かれている。

OpenOCDサーバを実行した状態で、gdbを起動する。
```
$ arm-none-eabi-gdb target/thumbv7m-none-eabi/debug/examples/hello
```
`c(ontinue)`で実行した時に、OpenOCDの方に、`Hello, world!`と表示されればOK。

