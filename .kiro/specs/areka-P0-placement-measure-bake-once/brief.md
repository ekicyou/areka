# Brief: areka-P0-placement-measure-bake-once

> 2026-10-05 起票（`/kiro-discovery`）。出どころは spec `areka-P0-animated-image-decode` の完了時の棚卸（タスク 7.1 の測定＝`.kiro/specs/completed/areka-P0-animated-image-decode/research.md` 9.14 節）。**段は優先（動く画像）**。

## Problem

- **利用者**: 動く絵（APNG・動く WebP）を持つシェルでは、起動が遅くなる。上限いっぱいの動く絵が 1 つ在るだけで、起動の焼く時間が release で 0.19 秒から 3.2 秒になる。そのうち約 1.5 秒は、窓の大きさを測る段（採寸）で全コマを読んでは捨てている分である。上限いっぱいの動く絵が 4 つ在ると 13.2 秒（うち採寸 6.7 秒）・debug では 64.5 秒。
- **今は踏まない**: リポジトリの検体のシェル・バルーンに動く絵は 0 枚（`animated-image-decode` の research.md 9.13 節）。`animated-image-playback` が着地して動く絵を使うシェルが増えると、効いてくる。

## Current State

- 採寸 `crates/areka/src/placement/measure.rs` の `measure_native_scope_sizes` は、シェル読み込みの入口 `load_shell_target`（`crates/areka-emo-present/src/shell_target.rs`）を 1 回呼んでシェルを全部焼き、`compose_size` で寸法だけを使って、焼いた結果を捨てる。
- 起動の資産組み立て `crates/areka/src/emo2_boot/assets.rs` の `build_boot_assets` が、同じシェルをもう 1 回焼く。動く絵は 2 回とも全コマを読む。
- バルーンは、採寸の `measure_balloon_surface0` が scope ごとに `build_balloon_target_from_faces` を（面 0 だけで）呼び、資産組み立ての `build_balloon_assets` も scope ごとに（全部の面で）呼ぶ。scope 2 つで 4 回焼く。emo2 のバルーンは静止画だけなので 1 回 11〜12 ms で、今は小さい。
- 採寸と資産組み立ての 2 回の `load_shell_target` は、読み込みの警告（焼くときの脱落・箱・`surface-element-nesting` の入れ子の 3 種）もそれぞれ 1 度ずつ出すので、起動 1 回で同じ警告が 2 行ずつ出る（2026-10-05 `surface-element-nesting` の実機で N=2 を確認・完了 spec の `real-machine-check.md`）。焼くのを 1 回にすれば 1 行ずつになる。
- メモリの山は `bake` 1 回の中で決まり、2 回焼いても重ならない（9.14 節の測定）。重なるのは時間と読む量だけ。

## Desired Outcome

- 起動の焼く回数が、シェル・バルーンともに必要な分だけになる（採寸と資産組み立てで同じ絵を 2 回焼かない）。または、採寸が動く絵の全コマを読まずに済む。
- 動く絵を持たないシェル（emo2）の起動の結果（窓の大きさ・位置・描かれる絵）は今と同じ。
- 上限いっぱいの動く絵を持つシェルの起動の焼く時間を測り直し、採寸の分が消えたことを記録する。

## Approach

要件の段で次の 2 つから選ぶ（議題 1）。
- **案 A（焼くのを 1 回に）**: 資産組み立てが焼いた結果から採寸する、または採寸が焼いた結果を資産組み立てへ渡す。読む量が半分になり、バルーンの 4 回も 2 回へ減る。起動の段取り（採寸 → 窓の配置 → 資産組み立て）の順番に触れる。
- **案 B（採寸は全コマ無しで）**: 採寸のときだけ動く絵を 1 枚目で焼く（上限 0 枚相当の `AnimationLimits` を渡す、など）。触る所が小さいが、焼く回数は 2 回のまま。

## Scope

- **In**: 起動の採寸と資産組み立ての焼く回数・読む量。採寸の結果が今と同じであることの固定。測り直し。
- **Out**: 動く絵の読み込み（`animated-image-decode`・完了）と再生（`animated-image-playback`）。`bake` の中身の速さ（`Packer::pack` など）。シェルの着替え（`switch_assets.rs`）の焼く回数は、要件の段で同じ形かを確かめ、同じなら含める。

## Boundary Candidates

- 起動の段取り（`emo2_boot`）: 焼いた結果の持ち回り。
- 採寸（`placement/measure.rs`）: 寸法の取り方。

## Out of Boundary

- `areka-emo-atlas` の `bake` の中身。
- 窓の配置の決まり（DPI・原点）。

## Upstream / Downstream

- **Upstream**: `animated-image-decode`（`bake_with_limits` と上限）。
- **Downstream**: `animated-image-playback`（動く絵の検体が増えると効いてくる）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）。
- **Adjacent**: `emo2_boot` の結線の列（`roadmap.md` の直列の列）。`animated-image-decode` が「差分 0」で守った呼び手 5 ファイル（`emo2_boot/assets.rs`・`emo2_boot/switch_assets.rs`・`placement/measure.rs`・`areka-emo-present` の `balloon.rs`・`shell_target.rs`）に触る。

## Constraints

- 窓の配置は本物のゴースト（emo2）と 96 以外の DPI で確かめる（steering の観測条件）。
- 起動の段取りの順番を変えるなら、DPI の切り替え・シェルの着替えの道を同じ形で直す。

## 要件の段で決める議題

1. 案 A と案 B のどちらを取るか（焼く回数を減らすか、採寸だけ全コマを読まないか）。
2. シェルの着替え（`switch_assets.rs`）とバルーンの着替えも同じ直しに含めるか。


## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: 案 A なら 10〜14、案 B なら 5〜8 タスク。8〜12 の幅で見積もりのまま。切らない。
- 前提の状態: `animated-image-decode` は着地済み＝今すぐ着手できる。ただし外形（採寸の元）を変える `extent-element-offset` が先に着地すると、本 spec の「採寸の結果が今と同じ」の固定がその後の値で書ける（後になるなら、固定は「1 回焼きと 2 回焼きの答えが一致する」の形で書く）。
- 崩れた前提／古くなった位置:
  - Current State は今も正しい。採寸の `measure_native_scope_sizes`（`placement/measure.rs`）が自前の `build_shell_assets` から `load_shell_target` を 1 回、`measure_balloon_surface0` が scope ごとに `build_balloon_target_from_faces`。資産組み立ては `emo2_boot/assets.rs` の `build_shell_assets`・`build_balloon_assets`。C3 で `placement/` に入ったのは `#[allow(dead_code)]` の注記の外しだけ。
  - 2 回の焼きは呼ばれる場所が離れている。採寸は `placement/mod.rs` の `prepare_stages_for_shell`（`ghost_session.rs` の窓を作る準備から）、資産は `emo2_boot/mod.rs` の `build_boot_assets_for`（`boot_ghost` の手順 1）。placement の結果は窓を作る閉包へ移されて同期では読めない（`derive_scopes` の doc）＝案 A は `ghost_session.rs` の段取りに触れる。
  - 議題 2 の答えが出た: シェルの着替え（`switch_assets.rs`）は `build_shell_assets` だけを呼び、採寸をしない＝「同じ形」ではないので範囲外でよい。ゴーストの切替は起動と同じ道（`ghost_session.rs` の準備 → 資産組み立て）を通るので、案 A は起動と切替を一緒に直す。
- 触るファイル（並走の照合用）:
  - 案 A: `crates/areka/src/placement/{mod.rs, measure.rs}`・`crates/areka/src/ghost_session.rs`・`crates/areka/src/emo2_boot/{mod.rs, assets.rs}` と兄弟のテスト
  - 案 B: `crates/areka/src/placement/measure.rs`・`crates/areka-emo-present/src/shell_target.rs`（`load_shell_target` に上限を渡す口）・必要なら `balloon.rs`。署名を変えると試験の支え `crates/areka/src/mcp/dump_surface_gpu_test_support.rs` も
  - 共有しうる相手: 案 A は `extra-character-windows`（`placement/mod.rs`・`measure.rs`・`emo2_boot/mod.rs`）と `balloon-font-file`（`assets.rs`）。案 B は `self-alpha-declaration`・`surfaces-basepos`（`shell_target.rs`）。
- 議題（答えで作業が変わるものだけ）: 1（案 A か案 B か）だけが残る。2 は上のとおり答えが出た。
- すぐ直せる軽微な修正: この brief の「要件の段で決める議題」の直後に、起票のときの書き損じの 2 行（`</content>`・`</invoke>`）が紛れ込んでいる。消すだけ。


## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化: `animated-image-playback`（10-07）が着地し、採寸の 1 回目も束縛の直後に動く絵を子へ分解するようになった（読んで捨てる仕事が増えた）。起動で 2 行ずつ出る警告の種類も増えた＝`element-base-method` の「描けない描画メソッドの行」・`self-alpha-declaration` の「descript.txt が読めない」「添えてあった `.pna` の数」と透過の扱いの知らせ。`self-alpha-declaration` は採寸と資産組み立ての両方に透過の宣言を渡す引数を足した（`placement/measure.rs`・`emo2_boot/assets.rs`・`balloon.rs` の `build_balloon_target_from_faces`）。採寸と資産組み立てが別々に焼く形は変わっていない。
- 触るファイル: 案 A＝`crates/areka/src/placement/{mod.rs, measure.rs}`・`crates/areka/src/ghost_session.rs`・`crates/areka/src/emo2_boot/{mod.rs, assets.rs}` と兄弟のテスト。案 B＝`placement/measure.rs`・`crates/areka-emo-present/src/shell_target.rs`・必要なら `balloon.rs`。
- 規模: 案 A なら 10〜14、案 B なら 5〜8 タスクのまま。切らない。
- 先に要るもの: 無し（今すぐ着手できる）。`extent-element-offset`・`collisionex-regions`・`seriko-trigger-intervals`・`animated-image-import`・`present-emit-tail-latency` とは重なり 0。列の外では、案 A が `emo2_boot` の結線の列（`mcp-reload`・`balloon-font-file`・`extra-character-windows`）と、案 B が `balloon-canon-residue`（`balloon.rs`）と重なる。`extent-element-offset` が先だと「採寸の結果が同じ」の固定が書きやすい。
- 優先度の区分: C（性能の持ち越し・同梱の検体では踏まない）。
- 要件定義のモデル: Opus。
- 分割の案: 無し。
- 見つけた穴・古くなった記述:
  - roadmap の台帳の行の「`animated-image-playback` の前が望ましい」は過ぎた。
  - 棚卸㉒の節の「書き損じの 2 行」は棚卸㉒の即時修正で消えた（もう無い）。
  - 案 A が足す先の `placement/mod.rs`（902 行）・`emo2_boot/mod.rs`（912 行）・`ghost_session.rs`（873 行）は上限の近く。足す分は新しいファイルへ出す。

## 2026-10-16 棚卸㉕の再測定（main `12801f6d`）
- 前提の変化: 版の段で v0.1.0（「手前が持ち込んだ重さ」）。棚卸㉓から触る所の変更は 0。採寸 `crates/areka/src/placement/measure.rs`（450 行）の `measure_native_scope_sizes` と、起動の資産組み立て `crates/areka/src/emo2_boot/assets.rs`（547 行）の `build_boot_assets` が別々に焼く形はそのまま。
- 触るファイル: 案 A＝`placement/mod.rs`（902）・`placement/measure.rs`・`crates/areka/src/ghost_session.rs`（873）・`emo2_boot/mod.rs`（911）・`emo2_boot/assets.rs` と兄弟のテスト（上限の近い 3 本には足さず新しいファイルへ）。案 B＝`placement/measure.rs`・`crates/areka-emo-present/src/shell_target.rs`（757）・要れば `crates/areka-emo-present/src/balloon.rs`（701）・試験の支え `crates/areka/src/mcp/dump_surface_gpu_test_support.rs`。
- 規模: 案 A なら 10〜14、案 B なら 5〜8 タスク（変わらず）。
- 先に要るもの: 無し。`extent-element-offset` が先だと「採寸の結果が同じ」の固定が書きやすい（ファイルの重なりは 0）。
- 同じウェーブに置けない相手: 案 A＝`multi-ghost-runtime`（`placement/mod.rs`・`ghost_session.rs`・`emo2_boot/mod.rs`）・`user-data-root`（`ghost_session.rs` の記憶の置き場の呼び手を直す場合）。案 B＝`balloon-webp-names`（`balloon.rs`）。`extent-element-offset`・`animated-image-import`（合成と読み手）とは 0。`dpi-realign-remembered-chain` とも 0（あちらは `placement/` の `chain_realign.rs`・`chain_finalize.rs`・`spawn.rs`・`persist.rs` と `emo2_boot/frame/`＝`mod.rs`・`measure.rs` に触らない）。
- 優先度の区分: C（性能の持ち越し・同梱の検体では踏まない）。
- 要件定義のモデル: Opus（変わらず）。案 A は起動の段取りの順に触れるが、1 本の道の中の順で、並行の新しい設計ではない。
- 分割の案: 無し。
- 見つけた穴・古くなった記述: 案 A は `multi-ghost-runtime` がゴーストごとに分け直す起動の道と同じ場所を作り直す＝v0.1.0 の中では `multi-ghost-runtime` の後に置くか、当たりの小さい案 B を選ぶ。棚卸㉓の節の `emo2_boot/mod.rs`「912 行」は今 911 行（差し支え無し）。

## 2026-10-18 棚卸㉗の再測定（main 2d86dbff）
- 版: **v0.1.0（棚卸㉗でゴールを組み直した後も残る）**。理由が 2 つになった: ⑴ **試験の速さ**: 動く絵の多いシェルは起動の焼き込みが release で 0.19 秒から 3.2 秒（上限いっぱいの動く絵 1 つ・4 つで 13.2 秒・debug では 64.5 秒）になり、うち約半分は採寸で読んで捨てる分。実機の試験は起動を何度もくり返すので、この分がそのまま試験の時間に乗る。MCP の `reload`（`mcp-reload`）の返事の上限 10 秒にも近づく。⑵ **ぱすたが SSP と同じに動く**: ぱすた（窓際のぱすたさん）のシェルは surface1000 に動く WebP の element を 3 つ持つ（`shell/master/window/ball/` の `eback.webp` 40 コマ・`base_rings.webp` 48 コマ・`efront.webp` 40 コマ・ghost_dev の `surfaces.txt` 120〜122 行目＝2026-10-18 に読み取りだけで確かめた）。起票のときの「今は踏まない（検体に動く絵は 0 枚）」は、ぱすたを検体にした今は外れた。
- 前提の変化: 触る所の中身は棚卸㉕から変わらない（採寸 `placement/measure.rs` 450 行・資産組み立て `emo2_boot/assets.rs` 547 行が別々に焼く）。行数: `placement/mod.rs` 902→**915**・`crates/areka/src/ghost_session.rs` 873→**885**・`emo2_boot/mod.rs` 911→**916**（`move-canon`・`write-failure-notice` ほか）。`crates/areka-emo-present/src/shell_target.rs` 757・`balloon.rs` 701 は変わらず。
- 触るファイル: 案 A＝`placement/{mod,measure}.rs`・`ghost_session.rs`・`emo2_boot/{mod,assets}.rs`（上限の近い 3 本には足さず新しいファイルへ）。案 B＝`placement/measure.rs`・`shell_target.rs`・要れば `balloon.rs`・`crates/areka/src/mcp/dump_surface_gpu_test_support.rs`。
- 規模: 案 A 10〜14・案 B 5〜8 のまま。
- 先に要るもの: なし。測り直しにぱすたの絵を使うなら `sample-url-fetch`（ぱすたを検体として取り寄せる）の後が楽。測りは上限いっぱいの合成の検体でもできる。
- 同じウェーブに置けない相手（v0.1.0）: 案 A＝`mcp-reload`（`emo2_boot/mod.rs`）・`offline-surface-dump` は `load_shell_target` を読むだけ＝0。案 B＝`offline-surface-dump`（同じ `load_shell_target` の口を変えるなら照合）。後回しの段: `multi-ghost-runtime`（案 A）・`balloon-webp-names`（案 B の `balloon.rs`）。
- 優先度の区分: C のまま（性能の持ち越し）。ただし v0.1.0 では試験の速さとぱすたの起動の速さに効く項目として扱う。
- 要件定義のモデル: Opus のまま。分割の案: なし。
- 見つけた穴・古くなった記述: Problem の「今は踏まない」はぱすたで外れた（上）。行数（902・873・911）は古い。
