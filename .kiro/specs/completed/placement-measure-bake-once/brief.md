# Brief: placement-measure-bake-once

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

## 2026-10-31 棚卸㉛の再測定（main 4068422f）
- **前提の変化**: ✅ `balloon-scroll-arrows`（10-31）が起動の資産組み立て `crates/areka/src/emo2_boot/assets.rs` を 489→540 行にした（バルーンは scope ごとに全部の面の定義と矢印の絵を `load_scope_balloon_definitions`〔`crates/areka-emo-present/src/balloon_definitions.rs`・新・133 行〕で読み、シェルは箱の印の絵も読む）。✅ `shell-mouse-cursor`（10-30）がシェル読み込みの入口 `load_shell_target`（`crates/areka-emo-present/src/shell_target.rs` 855→874）でカーソルの指定も読むようにし、資産組み立てでカーソルを作る。採寸 `crates/areka/src/placement/measure.rs`（465・`measure_native_scope_sizes` 153 行目・`load_shell_target` 343 行目・`measure_balloon_surface0` 377 行目）は動いていない＝採寸と資産組み立てが別々にシェルを焼く形は変わらない（読む物が増えたのは資産組み立ての側だけ）。`build_boot_assets` は 216 行目・`build_boot_assets_with_shell` は 241 行目。`crates/areka-emo-present/src/balloon.rs`（701）は動かず、✅ `animated-surface-log-flood`（10-31）が触ったのは emo-present の `presenter/` だけ。行数: `placement/mod.rs` 969→**970**（上限まで 30）・`crates/areka/src/ghost_session.rs` 935→**940**（上限まで 60・✅ `window-work-area-clamp`・✅ `balloon-scroll-arrows`）・`emo2_boot/mod.rs` 795→828。
- **触るファイル**: 案 A＝`placement/{mod,measure}.rs`・`ghost_session.rs`（配置の呼び出しは 214 行目の `prepare_ghost_windows_for_shell`）・`emo2_boot/{mod,assets}.rs` と兄弟のテスト（上限の近い `placement/mod.rs`・`ghost_session.rs` には足さず新しいファイルへ。ゴーストの切り替えと読み直しも同じ道なので一緒に直る）。案 B＝`measure.rs`・`shell_target.rs`（874）・要れば `balloon.rs`・試験の支え `crates/areka/src/mcp/dump_surface_gpu_test_support.rs`（461）。案 B は `load_shell_target` の形を変えず、上限を渡す別の入口を足すほうが当たりが小さい（呼び手 `offline_dump/mod.rs` と手本 `crates/areka/examples/` を直さずに済む）。シェルの着替え（`switch_assets.rs`）は採寸をしない＝範囲外（棚卸㉒の答え）。
- **規模**: 案 A 10〜14・案 B 5〜8 タスクのまま。上限 20 の内。
- **先に要るもの**: 働きの上ではなし。v0.1.0 の相手: 案 A＝`shell-scale-linkage`（`placement/mod.rs`・`emo2_boot/mod.rs`・プルの口に載せるなら `ghost_session.rs`）・`balloon-communicatebox`（`emo2_boot/mod.rs` の `mod` の 1 行）。`script-small-tags` とは重なり 0（あちらが印を描いて `assets.rs` に触れるときだけ重なる）。案 B＝v0.1.0 の残りと重なり 0（相手だった `shell-mouse-cursor`・`balloon-scroll-arrows` は着地した）＝`script-small-tags`・`shell-scale-linkage`・`balloon-communicatebox` のどれとも同じウェーブに置ける。前の相手 ✅ `startup-install-option`・✅ `window-work-area-clamp`・✅ `stay-on-top-directives` は着地した。余力の枠の `multi-ghost-runtime`（案 A）。依存を足さない・emo-text の席を使わない。
- **待ち**: 最後に静かな机で、release の焼く時間の前後の比べを 1 回（この spec の中の静かな机はこれだけ）。焼く回数・読んだコマの数は決定論のテストで固定し、時間は参考の記録に回す＝ウェーブに並べてよい。窓の配置の確かめは本物のゴースト（ぱすたの面 1000 の動く WebP 3 つ）と 96 以外の拡大率。
- **優先度の区分**: C（性能の持ち越し）。v0.1.0 では試験の速さとぱすたの起動の速さに効く項目として扱う（棚卸㉗）。
- **要件定義のモデル**: Opus のまま（案 A も 1 本の道の中の順の入れ替えで、並行の新しい設計ではない）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**:
  - 本文 Problem の「今は踏まない（検体に動く絵は 0 枚）」は、ぱすたを検体にした今は外れた。本文の議題 2 は答えが出た（上の `switch_assets.rs`）。
  - `window-link-directives` の実機の申し送り ⅳ（MCP の返事が遅れたあいだ、動く絵の玉が合成し直された）は再生の重さで、起動の焼く回数とは別の件。この spec では引き取らない。
