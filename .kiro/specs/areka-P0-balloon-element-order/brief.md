# Brief: areka-P0-balloon-element-order

> 2026-10-01 `/kiro-discovery`（シェル内バルーン）で起票。roadmap「シェル内バルーン」節。**追跡 spec**＝`shell-balloon` が最初の版で縮めた重ね順を、element定義の並び順どおりに戻す。本文の file:line は起票時（main `35209987`）の実測＝着手時に引き直すこと。

## Problem

- シェル内バルーンは `surface*`ブレスの element定義（`elementN,balloon,名前,X,Y`）で置くと決めた（`shell-balloon`）。element定義は並び順が重ね順（後ろほど手前）。ところが最初の版では「`balloon` の element定義は常に一番上に描く」に縮める。文字の層が画像の合成とは別の層で、常に一番上にあるため。
- このままだと、たとえば参考ゴーストのセキュリティボールが文字の手前を横切る、といった演出が書けない。書いた並び順と見た目が食い違う（最初の版は警告で知らせるだけ）。

## Current State

- 窓ごとの組み立て（`crates/areka-emo-present/src/mount.rs` の attach）は、窓の子として「文字の層の差し込み口（手前）」と「合成済みの面（奥）」の 2 つを持つ。面は emo-compose がアトラスから 1 枚に合成する（`areka-emo-compose`）。文字は emo-text が自分のスワップチェーンで描き、差し込み口に貼る。
- 当たり判定は合成の層とは別で、画家のアルゴリズム（後定義が手前）。

## Desired Outcome

- `balloon` の element定義を、他の element定義と同じく並び順どおりの重ね順で描く。最初の版の警告と縮めを外す。
- アニメーション（SERIKO の pattern）で上に重なる部品があっても、並び順の約束が保たれる。

## Approach（案・要件と設計で決める）

- 合成を「`balloon` の element定義の位置」で上下 2 枚（以上）に分け、その間に文字の層を挟む。あるいは文字を合成の中へ描き込む。性能（1 コマの予算・`recompose-budget` の先例）と、文字だけが変わるときに合成をやり直さないこと（今の利点）を天秤にかける。

## Scope

- **In**: 重ね順の挟み込み、最初の版の縮めと警告の撤去、SERIKO の重なりとの合わせ、性能の確かめ、決定論テスト。
- **Out**: `surface1000` などのサーフェスを element定義で置く機能（開発者の将来の希望・未起票）。本 spec の分け方はそれを妨げないこと。

## Boundary Candidates

- 合成の分割（emo-compose・emo-present）と、文字の層の差し込み位置。

## Out of Boundary

- シェル内バルーンの定義・行き先・切り替え（`shell-balloon`）。

## Upstream / Downstream

- **Upstream**: `shell-balloon`。
- **Downstream**: 将来の「サーフェスを element定義で置く」。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `present-write-coherence` の未達（性能）・`zorder-chain-residue`（窓どうしの重なり順＝別の話）。

## Constraints

- 文字だけが変わるコマで合成をやり直さないこと（今の性能を落とさない）。1 ファイル 1,000 行。決定論テスト網羅は必達。


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- `shell-balloon` の追跡 spec。element の型とシェルのパーサ・emo-compose／emo-present の合成を触る＝`surface-element-nesting`・`animated-image-playback` と同時に走らせない（シェルの element の直列の列）。


## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M（10〜14 タスク・初めての見積り）。合成を分ける案なら上限側、文字を合成へ描き込む案は性能の確かめが増えてさらに上がりうる。
- 前提の状態: `shell-balloon` は着地済み（PR#227）。`animated-image-playback`（roadmap の台帳の前提）は未。シェルの element の直列の列の 4 本目なので、`surface-element-nesting`・`animated-image-playback` の後。
- 崩れた前提／古くなった位置（`shell-balloon` が実際に作った形）:
  - 箱の文字の面は**差し込み口の下ではなく、シェルの窓の直接の子**（完了 design の論点 C「C-2 の変形」）。窓の `Children` は `[差し込み口, 箱…, 合成済みの面]` の順で、箱は差し込み口の直後に element番号の大きい順に挿す（`crates/areka-emo-text/src/actor_box.rs` の `box_child_index`・挿すのは `surface_window_child.rs` の `TextSurface::attach_window_child`）。brief の Current State の「窓の子は差し込み口（手前）と合成済みの面（奥）の 2 つ」は古い。
  - 「最初の版の縮め」の実体は 2 か所: 描画は常に画像より手前（上の並び）、警告は `crates/areka-emo-compose/src/boxes.rs` の `place`（画像の element の最大の番号より小さい番号の箱を採ったうえで `BoxIssue` へ載せる）。撤去するのはこの 2 か所。
  - 箱の当たり判定は wintf の当たり判定に「表示されている字の矩形のマスク」で乗り（`surface_window_child.rs` の `set_hit_cells`）、操作は窓へ上がって窓のハンドラの前段（`crates/areka/src/input_events/shell_box.rs`）が `shown_boxes` の四角で箱を先に見る。
  - **wintf の兄弟の重なり順は描画と当たり判定で逆のまま**（描画: `visual_sync.rs` の `visual_hierarchy_sync_system` は先頭の子が最前面／当たり判定: `tree_iter.rs` の `DepthFirstReversePostOrder` は最後の子が最前面）。`shell-balloon` は裁定せず、この逆を前提に組んだ（`surface_hit_cells_tests.rs`「絵が不透明なら今までどおり絵が受ける」）。合成を上下 2 枚以上の窓の子へ分ける案では、「箱より手前の画像」の上のクリックが奥の画像へ当たる・奥の画像の上で箱が先に当たる、が初めて表に出る＝**この裁定は本 spec の要件に乗せるのが筋**（`surface-element-nesting` は子を 1 枚へ合成するので窓の子が増えない）。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-present/src/{mount.rs, presenter.rs, cache.rs}`（合成を分けるなら・`VisualMount` の子の構成）
  - `crates/areka-emo-compose/src/{plan.rs, boxes.rs}`（合成の分け目・警告の撤去）
  - `crates/areka-emo-text/src/{actor_box.rs, surface_window_child.rs}`（挿す位置）
  - 裁定によっては `crates/wintf/src/ecs/layout/hit_test/mod.rs` か `crates/wintf/src/ecs/graphics/systems/visual_sync.rs`
  - 兄弟のテスト・検体（`crates/areka-emo-text/tests/fixtures/shell-balloon/surfaces.txt` の並びを変えた版）
- 議題（答えで作業が変わるものだけ）:
  1. 合成を分けて文字の層を挟むか、文字を合成へ描き込むか（brief のまま）。
  2. wintf の兄弟の重なり順を描画と当たり判定のどちらへ揃えるか（分ける案を採るなら必須。wintf を直すと `shell-balloon` の `[差し込み口, 箱…, 絵]` の当たり判定の前提も一緒に変わる）。
- 見つけた穴: 無し（逆の重なり順は今は窓のハンドラの前段が吸収している）。


## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M（10〜14 タスク）。変わらず。切らない。
- 前提の状態: `shell-balloon` は着地済み。roadmap の前提 `animated-image-playback` は働きの依存ではなく（合成の分け目は `plan.rs`、pattern の上乗せは今の仕組みのまま）、`plan.rs` と emo-compose を分け合うための順番待ち。もう 1 つ、emo-text の `actor_box.rs` を `balloon-font-file`（C4・文字とバルーンの列）と分け合う＝**2 つの列にまたがる**。両方の列が空いた席でしか着手できない。
- 崩れた前提／古くなった位置:
  - `shell-balloon-frame-align` が `actor_box.rs` を書き直した（約 95 行）が、挿す位置の `box_child_index`・挿す口の `TextSurface::attach_window_child`・当たりの `set_hit_cells` は今もある。棚卸㉑の位置はそのまま使える。wintf の当たり判定の順の `DepthFirstReversePostOrder` は `crates/wintf/src/ecs/common/tree_iter.rs`、描画の順は `visual_sync.rs` の `visual_hierarchy_sync_system`。
  - `surface-element-nesting` が、子のサーフェスの中の `balloon` の element定義を警告して無視する `BoxIssue::InChildSurface`（`boxes.rs`）を足した。本 spec の並び順は一番上のサーフェスの element定義だけが相手（子の中の箱は今も無視のまま）。
  - `mcp-dump-images` が「最後に表示した合成の 1 枚」を読む口（`areka-emo-present/src/presenter/snapshot.rs` の `last_shown`）を足した。合成を上下 2 枚以上に分ける案では、`dump_surface` が返す絵も重ね直す必要が出る＝触るファイルが増える。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-present/src/{mount.rs, presenter.rs, cache.rs, presenter/snapshot.rs}`（分ける案なら）・`crates/areka/src/mcp/dump_surface.rs`（同）
  - `crates/areka-emo-compose/src/{plan.rs, boxes.rs}`
  - `crates/areka-emo-text/src/{actor_box.rs, surface_window_child.rs}`
  - 裁定によっては `crates/wintf/src/ecs/layout/hit_test/mod.rs` か `crates/wintf/src/ecs/graphics/systems/visual_sync.rs`
- 議題（答えで作業が変わるものだけ）: 棚卸㉑の 2 つのまま（分けるか描き込むか／wintf の兄弟の重なり順をどちらへ揃えるか）。1 で分ける案を採ると `dump_surface` の重ね直しも範囲に入る。
- 見つけた穴: 無し。


## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化: 待っていた `animated-image-playback`（10-07）が着地した。箱を挿す位置 `box_child_index`（`crates/areka-emo-text/src/actor_box.rs`）・挿す口 `attach_window_child` と当たりの `set_hit_cells`（`surface_window_child.rs`）・警告の出どころ `place`（`crates/areka-emo-compose/src/boxes.rs`）・最後に表示した 1 枚を読む口 `last_shown`（`crates/areka-emo-present/src/presenter/snapshot.rs`）は今もある。`wintf-tooltip`（10-08）は wintf に新しいモジュールを足しただけで、兄弟の重なり順には触れていない。
- 新しく効いてくること: 動く絵は合成の回数を約 13 倍にする。合成を上下に分ける案は 1 回の適用で 2 枚を作り直すので、性能の確かめが重くなる（`present-emit-tail-latency` の尾と同じ場所）。
- 触るファイル: 棚卸㉒のまま（emo-present の `mount.rs`・`presenter.rs`・`cache.rs`・`presenter/snapshot.rs`、emo-compose の `plan.rs`・`boxes.rs`、emo-text の `actor_box.rs`・`surface_window_child.rs`、`crates/areka/src/mcp/dump_surface.rs`、裁定しだいで wintf の当たり判定か描画の順）。
- 規模: 10〜14 タスクのまま。切らない。
- 先に要るもの: 働きの上では無し。シェルの element の列（`plan.rs`）と文字とバルーンの列（`actor_box.rs`）の両方の席が空いたとき。`present-emit-tail-latency` の後が望ましい（`presenter/` を分け合う・性能の物差しが先に要る）。
- 優先度の区分: A（「シェル内バルーン」は開発者の依頼 10-01・その追跡）。
- 要件定義のモデル: Fable（合成・表示・文字・wintf にまたがる／兄弟の重なり順の裁定）。
- 分割の案: 無し。
- 見つけた穴・古くなった記述: 無し。
