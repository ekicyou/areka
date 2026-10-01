# Brief: areka-P0-animated-image-playback

> 2026-10-01 `/kiro-discovery` で起票（開発者指示は `animated-image-decode` の brief と同じ）。roadmap「動く画像」節の 2 本目＝**再生の側**。開発者確定「2 本で分割、always も含めて進めて」。
> 本文の file:line は起票時（main `5e37745e`）の実測＝**着手時に引き直すこと**。**同日の議論で `surface-element-nesting`（element でサーフェスを置く）の上に載せると決めた**（開発者「手書きで定義できるようになる→画像を自動に定義分解する。ながれできれい」）。

## Problem

- 正典の 3 つの口が areka で動かない:
  1. **自動アニメーション**（ukadoc の element定義の項「surface*.pngまたはelement定義にアニメGIF/APNG/WebPアニメを指定すると、SERIKO定義を書かなくても自動的にアニメーションする(SSP 2.7.38～)」）。
  2. **`import` メソッド**（`animation*.pattern*,import,ファイル名,ウエイトmsec,X,Y`・2.7.50）＝動く絵を、既存のアニメーションと並んで動く 1 本として取り込む。合成は overlay・元のファイルの繰り返し回数は無視。網羅台帳は `absent`（`areka-emo-compose` の `method::from_name` が名前を知らず `warn!` で描かない）。
  3. **interval `always`**（ずっと繰り返す）＝seriko の表が記録しない。

## Current State（起票時の実測）

- **seriko の表**（`crates/areka-seriko/src/table.rs` の `AnimationTable::from_world`）が記録するのは `random`・`bindrandom`・`sometimes`・`rarely` だけ。`always`・`runonce`・`bind` などは駆動されない。時計は 2 層（1000 ms の抽選の境目と、`frame_at` の累積の待ち時間＝`timeline.rs`）で、**繰り返しの概念が無い**（1 回再生したら次の抽選を待つ）。刻みは `spawn_loop_ticker`（`crates/areka-ghost/src/ticker.rs`・16 ms）→ `SerikoMsg::Tick`。
- **出力は `PatternState`**（アニメーションの id → 1 つの `PatternFrame`・`crates/areka-emo-compose/src/pattern.rs`）→ `DisplayCommand::Show{pattern}` → 合成のキャッシュの鍵 `ComposeKey{surface_id, binds, pattern}`（`crates/areka-emo-present/src/cache.rs`・`CAPACITY = 3`）。**コマが替わるたびに CPU で合成し直す**。
- **合成器**: `ComposeMethod::is_implemented()` は `Overlay` だけ（`method.rs`）。`flatten_surface`（`plan.rs`）は pattern が指すサーフェスへ再帰する（位置のずれの加算・循環の停止）が、**入れ子の内側は `PatternState` を見ない**（`is_top_level=false`）。
- **element は画像専用**: `Element{layer, path}`（`crates/areka-parsers/src/shell/model.rs`）に描画メソッドの欄が無く、`overlay` 以外の行は読み捨て。
- バルーンの面は合成した `surfaces.txt` を同じパーサと `bake` へ通す（`crates/areka-emo-present/src/balloon.rs`）＝読み込みの変更は届くが、面ごとのアニメーションの表（`SerikoLoopConfig.balloon_tables`）は emo2 では空。

## Desired Outcome

- 動く絵を element定義・`surface*.png` に置けば、SERIKO 定義なしでずっと動く（待ち時間はファイルのコマごとの値）。
- `import` の pattern で取り込んだ動く絵が、他のアニメーションと並んで動く。
- interval `always` のアニメーションがずっと繰り返す。
- バルーンの面に置いた動く絵も動く。
- 動かないシェル（emo2 を含む）の見た目・合成の回数・1 コマの時間は変わらない。

## Approach（2026-10-01 discovery の開発者確定事項）

1. **分解して子サーフェスにする**（開発者の案「読み込み時にサブエレメント分解して、アニメーション表示を回す」）。読み込み（`animated-image-decode`）が出したコマから、「コマを 1 枚ずつ持ち、`always` でコマを順に指す」合成のサーフェスを作り、動く絵を指す element定義・`surface*.png` を、その子サーフェスを指す element（`surface-element-nesting` の入口）に置き換える。時計は `surface-element-nesting` の子の時計＝**親の面の切り替えで巻き戻らない**（`surface1` と `surface2` が同じ GIF を置いていれば、切り替えても途切れない）。**アニメのエンジンは sakura と seriko の 2 つのまま**（3 つ目の時計は作らない）。合成のサーフェスの番号は作者の番号と衝突しない空間に置く（設計で決める）。
2. **`always` を本 spec で入れる**（開発者確定）。自動アニメーションと同じ「ずっと繰り返す」仕組みで、正典の interval の語として表に記録する。
3. `import` は 2 と同じ仕組みの上に、pattern の描画メソッドとして足す（合成は overlay・繰り返し回数は無視）。

## Scope

- **In**: 自動アニメーション（element定義・`surface*.png`）・`import` メソッド・interval `always`・繰り返しの仕組み（コマの終わりで頭へ戻る）・ファイルの繰り返し回数の扱い（自動アニメーションで守るかは要件で正典を確かめる）・バルーンの面の動く絵・合成のし直しの回数の確かめ（性能）・網羅台帳の行（element・`import`・`always`）の更新・決定論テスト・実機の確かめ（3 形式の検体）。
- **Out**: 読み込みとアトラス（`animated-image-decode`）。element でサーフェスを置く入口と子の時計（`surface-element-nesting`）。`runonce`・`bind` などの他の interval（「サーフェスアニメーション」の束＝別途）。`overlay` 以外の描画メソッド全般。element のオプション（`--clipping` など）。

## Boundary Candidates

- seriko の表と時計（`areka-seriko` の `table.rs`・`timeline.rs`・`looper.rs`）。
- 合成器の描画メソッドと再帰（`areka-emo-compose` の `method.rs`・`plan.rs`・`fold.rs`）。
- 自動アニメーションの合成（読み込み結果から表の行を作る場所＝設計で決める）。

## Out of Boundary

- 台本の再生（sakura）と dola の台本には触らない。

## Upstream / Downstream

- **Upstream**: `animated-image-decode`・`surface-element-nesting`（入口・子の時計）。
- **Downstream**: `currentghost-property-tree`（`seriko.*` のプロパティでアニメーションが読めるようになる＝読むだけの隣接）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）。
- **Adjacent**: `balloon-element-order`（合成の分割＝同じ `areka-emo-compose`・`areka-emo-present`）・`shell-balloon`（element定義の描画メソッドの欄を足す＝同じ `areka-parsers` の shell）・`mcp-dump-images`（合成の結果の読み戻し）。

## Constraints

- アニメのエンジンは 2 つ（sakura・seriko）のまま。1 ファイル 1,000 行。決定論テスト網羅は必達。合成のキャッシュ `CAPACITY` は統制された定数＝変えるなら根拠を残す。動かないシェルの 1 コマの時間を落とさない。
