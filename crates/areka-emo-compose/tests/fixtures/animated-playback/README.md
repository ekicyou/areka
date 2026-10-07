# 動く絵の再生の検体

spec `areka-P0-animated-image-playback`（要件 9.4）の試験用のシェル（`shell/`）とバルーン（`balloon/`）。第三者の著作物は 0 件。

## 絵の出どころ

動く絵は読み込みの側の検体 `crates/areka-emo-atlas/src/testdata/animated/`（作り手 `crates/areka-emo-atlas/examples/gen_animated_samples.rs`・単色の矩形だけ）のバイト列をそのまま写した。コマ・待ち時間・決め手の画素はあちらの `README.md` が正本。

| ファイル | 写し元 | 形式 | コマ | 待ち時間（ms） | 繰り返し |
|---|---|---|---|---|---|
| `shell/rgb.apng`・`shell/surface2.png`・`balloon/balloons0.png` | `rgb.apng` | APNG・α なし | 2 | 100・100 | 終わりなし |
| `shell/surface1.png` | `basic.apng` | APNG | 4（3 番は全透明） | 333・0・70・1 | 合計 2 回 |
| `shell/alpha.webp`・`balloon/balloons1.png` | `alpha.webp` | 動く WebP | 3 | 100・0・70 | 合計 3 回 |
| `shell/rgb.webp` | `rgb.webp` | 動く WebP・α なし | 2 | 100・100 | 終わりなし |

名前が `.png` でも中身で見分けて読む（`surface1.png`・`surface2.png` は APNG、`balloons1.png` は動く WebP）。

動かない絵は全面が 1 色の RGBA 8 ビットの PNG で、Python の標準ライブラリ（`zlib`・`struct`）で `IHDR`・`IDAT`・`IEND` を書いて作った。

| ファイル | 寸法 | 色 (R,G,B,A) |
|---|---|---|
| `shell/body.png` | 16×16 | (128,128,128,255) 灰 |
| `shell/part.png` | 8×8 | (255,0,255,255) 赤紫 |
| `shell/yellow.png` | 4×4 | (255,255,0,255) 黄 |
| `shell/cyan.png` | 4×4 | (0,255,255,255) 水色 |

`shell/descript.txt` は透過の宣言 `seriko.use_self_alpha,1` だけを置く（`areka-P0-self-alpha-declaration` の後、宣言の無いシェルは読めない旨の `warn!` を 1 行出すため）。

## シェルのサーフェス（`shell/surfaces.txt`）

| 番号 | 役 |
|---|---|
| 0 | 一番上。element定義に置いた動く絵（`rgb.apng` を (0,0) と (8,0) の 2 か所・`alpha.webp` を (0,8)）・部品 20 を (8,8)・手書きの `always`（30・31 を 100 ms ずつ） |
| 1 | `surface1.png`（動く APNG・合計 2 回）だけで絵が建つ。element定義は無い |
| 2 | `element0` が在るので、動く `surface2.png` は土台に使われない |
| 3 | 0 と同じ絵 `rgb.apng` を置くもう 1 つのサーフェス |
| 10 | 待ち時間の合計が 0 の手書きの `always` |
| 11 | `bind+always`（本 spec では駆動しない組み合わせ） |
| 20 | 部品（0 が置く）。動く絵 `rgb.webp` と手書きの `always`（30・31 を 50 ms ずつ） |
| 30・31 | 手書きの `always` のコマが指すサーフェス（黄・水色） |

## バルーン（`balloon/`）

面の絵だけ。`balloons0.png`（動く APNG・終わりなし）と `balloons1.png`（動く WebP・合計 3 回）。
