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


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 規模 M〜L（15〜20 タスク）。brief の記述は実物と一致（seriko の表は `Random`・`BindRandom`・`sometimes`・`rarely` だけを記録・`is_implemented` は Overlay だけ・`CAPACITY = 3`）。軽い違い 1 つ＝`balloon_tables` は `assets.rs` が面ごとに作っている（emo2 では中身が空なだけ）。
- **触るファイル**: `crates/areka-seriko/src/{table.rs, timeline.rs, looper.rs}`・`crates/areka-emo-compose/src/{method.rs, plan.rs}`・自動アニメーションの合成サーフェスを作る場所（設計で決める）・`doc/ukadoc-coverage/ledger/assets.toml`・検体。
- **議題**: 合成サーフェスの番号の空間／ファイルの繰り返し回数を守るか／`CAPACITY` を変えるか。


## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M〜L（15〜20 タスク）。変わらず。前提の 2 本の設計しだいで上下する（子の時計を `surface-element-nesting` が作り切れば下限側）。
- 前提の状態: 未（`animated-image-decode`・`surface-element-nesting` のどちらも C3 で未着手）。
- 崩れた前提／古くなった位置:
  - `shell-balloon` の着地で seriko に入ったのは `crates/areka-seriko/src/actor.rs` の `BalloonResolve::NameForm` の腕を `debug!` へ下げた 1 か所だけ。`table.rs`（`Random`・`BindRandom`・`sometimes`・`rarely` だけを記録）・`timeline.rs`・`looper.rs`（`on_surface_changed`）・`areka-emo-compose` の `method.rs`（`is_implemented` は `Overlay` だけ）・`plan.rs`（`is_top_level`）・`areka-emo-present/src/cache.rs`（`CAPACITY = 3`）は棚卸⑳のまま。
  - `shell-balloon` が入れた箱の表（`areka-emo-compose/src/boxes.rs` の `BoxLayout`）はサーフェス番号を鍵に持つ。本 spec が作る「合成のサーフェス」は surfaces.txt の文面に現れないので `parse_boxes` には載らず衝突しないが、番号の空間を決めるときは `BoxLayout` と seriko の表の両方で作者の番号と分かれていることを確かめる。
  - 前提の 2 本が「`AtlasKey`・`manifest.rs`・`AtlasTable::new` を変えない」設計で着地すると、コマの引き方は atlas の `table.rs` に足される別の口になる（`animated-image-decode` の棚卸㉑の節）。本 spec の合成のサーフェスの element はその口でコマを引く＝`areka-emo-compose/src/atlas_bind.rs` を触る見込みが新たに出た。
- 触るファイル（並走の照合用）:
  - `crates/areka-seriko/src/{table.rs, timeline.rs, looper.rs}`（`always` と繰り返し）
  - `crates/areka-emo-compose/src/{method.rs, plan.rs, atlas_bind.rs}`（`import`・コマの引き方）
  - 合成のサーフェスを作る場所（設計で決める。候補は `areka-emo-present/src/shell_target.rs` の読み込みの直後か compose の畳み込み）
  - `crates/areka/src/emo2_boot/assets.rs`（バルーンの面の `balloon_tables`）
  - `doc/ukadoc-coverage/ledger/assets.toml`・新規の検体
- 議題（答えで作業が変わるものだけ）: 棚卸⑳のまま（合成サーフェスの番号の空間／ファイルの繰り返し回数を守るか／`CAPACITY` を変えるか）。
- 見つけた穴: 無し。


## 2026-10-04 `animated-image-decode` からの申し送り（同 spec のタスク 6.2）

読み込みの側（`areka-P0-animated-image-decode`）が着地した形。コードの引用は「何の定義か」で指す。着手時に引き直すこと。

### 渡すもの

- **コマの引き方**: `areka-emo-atlas` の `AtlasTable::animation(id)`。`id` が動く絵の親（今までの鍵で引く `ElementId`）のときだけ `Some(&Animation)` を返し、静止画と 2 枚目以降のコマの `id` では `None`。`AtlasKey`・`manifest.rs`・`AtlasTable::new` の署名は変えていない（動く絵つきの表は別の組み立て口 `AtlasTable::with_frames` で組む）。
- **`Animation` の 3 つの欄**: `frames: Vec<ElementId>`（`frames[0]` は親自身＝0 番のコマ＝今までの鍵で引ける絵）・`delays_ms: Vec<u32>`（`frames` と同じ長さ）・`loop_count: LoopCount`。コマは 2 枚以上。
- **コマの番号の並び**: 2 枚目以降のコマは、鍵のエントリが全部並んだ後ろに、親の番号の昇順・コマの番号の昇順で続けて並ぶ。動く絵が無いシェルでは静止画の番号は今までと同じ。2 枚目以降のコマの鍵（`AtlasTable::key`）は親と同じ鍵で、鍵からの逆引き（`AtlasTable::resolve`）はいちばん小さい番号＝親を返す。各コマは普通のエントリとして `AtlasTable::entry` で引ける。全透明のコマも、位置の無い（`placement` が `None` の）エントリとして番号と待ち時間が残る。
- **コマの中身**: ファイルの重ね方（背景へ戻す・前へ戻す・重ねる）を解いた後の、絵の全体の寸法の 1 枚ずつ。乗算済み BGRA。透明な縁の切り詰めはコマごとに静止画と同じに行う（ずれは各エントリの `trim_offset`）。透明度を持たない動く絵は、1 枚目のコマの左上の色が全コマから抜かれている。
- **繰り返し回数**: `LoopCount::Infinite`（終わりなし）か `LoopCount::Finite(n)`（全体を合計 n 回・n は 1 以上）。APNG・WebP で同じ意味。
- **待ち時間**: ミリ秒へ四捨五入した値。0 は 0 のまま。
- **1 枚へ縮んだ動く絵**: 上限を超えた絵・読み込みに失敗した絵は、動きの 1 枚目だけの普通の静止画になり、`animation` は `None`。再生の側から見ると静止画と区別が付かない。
- **3 つの上限**: `AREKA_ANIMATED_IMAGE_MAX_FRAMES`（既定 1,024 枚）・`AREKA_ANIMATED_IMAGE_MAX_PIXELS`（既定 67,108,864 画素）・`AREKA_ANIMATED_IMAGE_MAX_TOTAL_PIXELS`（既定 268,435,456 画素）。定義は `areka-emo-atlas` の `limits.rs`、利用者向けの説明は `dist/README.txt`。合計は `bake` 1 回ごとに 0 から数える（起動ではシェルが採寸と資産の組み立てで 2 回焼かれるが、それぞれ別に数える）。設定画面と設定ファイルへの引き取りは `roadmap.md` の予約に載せた。
- **検体**: `crates/areka-emo-atlas/src/testdata/animated/` の 12 個（APNG 7・動く WebP 4〔中身が WebP の `.png` を含む〕・GIF 1）と、その中身を書いた `README.md`。作り手は `crates/areka-emo-atlas/examples/gen_animated_samples.rs`（単色の矩形だけ・第三者の著作物 0 件）。本物の読み手の結合テスト（`samples_e2e_tests.rs`）は、`single.webp`（1 枚の WebP は WIC の WebP の拡張機能が要る）と `deep16.apng`（16 ビットは新しい読み手で読めず、今までの WIC の 1 枚読みへ落ちる）を焼く検体から外している。実機の確かめにも使える。
- **見た目の変化の記録**: `doc/COMPAT_ARCHITECTURE.md` の §8 の 1 節「動く絵（APNG・動く WebP）の読み込み」。

### 渡さずに残したもの（本 spec では決めていない）

- **待ち時間 0 の丸め方**: 読み込みは 0 を 0 のまま渡す。0 や極端に小さい値をどう再生するかは、再生の側で正典を確かめて決める。
- **繰り返し回数の使い方**: 読み込みは回数を渡すだけ。自動アニメーションで守るか・`import` で無視する（正典）かは再生の側。
- **C2 の `--clipping` の決まり**: element定義に `--clipping` を付けると、動く絵としての読み込みが無効になる（正典）。areka は element定義のオプションをまだ読まないので、今の読み込みは `--clipping` を見ずに全コマを読む。引き受ける spec は、読み込みの側のタスク 6.3 で `/kiro-discovery` から `areka-P0-element-clipping-option` として起票した（2026-10-05・優先・シェルの element の列で本 spec の後）。本 spec は、`--clipping` 付きの element定義では動く絵として読まれない前提で再生を組めばよい（読まない判定はあちらが持つ）。
- **再生そのもの**（自動アニメーション・interval `always`・`import`）。
- **動く GIF は非対応**（開発者裁定 2026-10-04・理由: 古い形式）。GIF は今までどおり 1 枚の絵として読み、`animation` は常に `None`（`image` の機能 `gif` は入れていない）。上の Problem 1 が引く正典の文は GIF も挙げているが、areka は動く GIF を動かさない。Scope の「実機の確かめ（3 形式の検体）」は、APNG と動く WebP の 2 形式に読み替える。網羅台帳の `element*` の項にも同じ注記がある。
- **焼く時間と起動のメモリの山の数字**: 読み込みの側のタスク 7.1 で測って、この brief に追記する。採寸の側で全コマを読まずに済ませる直しが要るかも、その数字で判断する（要るなら読み込みの側の完了時に `/kiro-discovery` で起票）。
