# Fm Radio Sim

FMラジオの音を再現するシミュレーションプログラム。


## ビルド方法

現状、**Windowsでのみ**ビルド可能。  
`msvc`と`rust`が導入されている環境であれば、ビルドできる。

### VSTPlugin

クローンしたディレクトリにて、`cargo build -r`を実行

生成物は`target/release/`に`frequency_modulation.dll`と`fm_sim.exe`ができる。

`frequency_modulation.dll`がプラグインである。  
`fm_sim.exe`はwavファイルを入力し、シミュレーション結果のファイルを生成するプログラムである。

### テストプログラム

クローンしたディレクトリにて、以下のコマンドを実行

1. `cd gui_test`
2. `cargo build -r`

`target/release/`に`gui_test.exe`が生成される。

## プラグインの利用

ビルドした`frequency_modulation.dll`をVSTを読み込めるプログラム(VST　Hostや各種DAWアプリ)で読み込み。  
現状、設定できるパラメータはなく、79.5MHzのFMラジオをシミュレートする。

また、バッファサイズが700サンプル固定なので、Hostアプリケーション側で入力バッファサイズを700に変更すること。  
または、ソースコード(`src/lib.rs`)にある`DEFAULT_BUFFER_SIZE`を変更。

