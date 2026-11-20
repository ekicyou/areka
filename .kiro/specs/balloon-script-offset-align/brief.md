# Brief: balloon-script-offset-align

> 2026-11-01 `shell-scale-linkage` の完了時の棚卸で `/kiro-discovery` で起票（対話なし・完了した spec からの引き渡し）。出どころは同 spec の実機の確かめの段 4（`completed/shell-scale-linkage/real-machine-verification.md`・`tasks.md` の Implementation Notes の 9.2）と、2026-11-01 の開発者の裁定（案 A＝段 4 を「今の areka では起きない形」として閉じ、離して置ける仕組みができたら確かめ直す）。起票の条件: 規模が spec 1 本に相当し、新しい仕組み（台本からバルーンの置き方を変える受け口）が要る。区分 **C**（ukadoc の拾い残し）。

## Problem

- 台本の `\![set,balloonoffset,x,y]`（バルーンの取り付け位置のずらし）と `\![set,balloonalign,ID]`（バルーンをキャラのどちら側へ付けるか）を、areka は扱わない。網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の 2 行はどちらも `status = "absent"`・宛先が空・優先度 A2。`check_script` は受け手の無い命令（`unknown_command`）と答え、利用者から見るとタグが消えたのと同じになる（バルーンは既定の位置・既定の側に出たまま）。
- そのため、本体とそのバルーンを DPI の違う画面に分けて置く形を作れない。`shell-scale-linkage` の実機の確かめの段 4（ぱすた開発との約束＝分かれたときに「＊バルーン倍率のずれ」を出さないこと・`currentghost.balloon.scope(0).currentmonitor.dpi` がバルーンの居る画面の DPI を答えること）は確かめられないまま閉じている。

## Current State

- 本体を画面の境にまたがせても、バルーンは本体の居る画面の作業領域の中へ押し戻される。記録は `[balloon-limit] Clamp runtime 関門 … route=Some(BalloonFollow) … how="pushed"`（2026-11-01 の走行 ssl1・`target\pasta-verify\logs\ssl1.stdout.log` の 00:00:03〜10）。押し戻しは `crates/areka/src/placement/balloon_limit.rs` の `place_balloon_in_work_area`、呼ぶのは起動の関門 `apply_balloon_limit_one` と書き込みの関門 `apply_balloon_limit_gate`（`placement/follow/window_move.rs`）。
- descript の `balloon.offsetx`／`balloon.offsety`・`balloon.alignment` は読んでいる（`placement/resolver.rs` の P5・完了 spec `descript-window-keys`）。台本から同じ値を変える道が無い。
- 倍率の表・`currentmonitor.dpi` の読み手（`shell-scale-linkage`）は窓ごとに値を持つので、バルーンが別の画面に居れば別の DPI を答える作りになっている（実機ではまだ見ていない）。

## Desired Outcome

- `\![set,balloonoffset,x,y]` と `\![set,balloonalign,ID]` を ukadoc の意味どおりに受け、そのスコープのバルーンの置き方が変わる。台帳の 2 行が実装済みになり、`check_script` が受け手の無い命令と報告しない。
- 作者が台本でバルーンを本体から離して置けば、本体と別の画面にバルーンを出せる（押し戻しとの関係は要件の段で決める）。
- 出せた形で `shell-scale-linkage` の段 4 を実機で確かめ直し、結果をぱすた開発のセッションへ知らせる。

## Approach

- 受け口は `\!` の汎用キャリアの受け手の表（`consumer_ledger.rs`）に 2 行を足し、置き方の値の持ち主（`placement/` の P5 の入力）へ届ける。typed の個別の命令は作らない。
- 値は起動の配置・追従・押し戻しの全部が同じ所から読む（descript の値に台本の値を上書きする層を 1 つ足す）。保存するか・切り替えで消えるかは ukadoc と SSP で決める。
- 押し戻しの関門は、バルーンの作業領域を「本体の画面」でなく「バルーン自身が居る画面」で見るのか、本体の画面の中に留めるのかを要件の段で決める（ここが段 4 を作れるかどうかの分かれ目）。

## Scope

- **In**: 2 つの命令の受け口・置き方への反映（起動・追従・押し戻しの関門）・値の寿命・記録の欄・決定論のテスト・台帳 2 行と §8 の行・SSP の測り・`shell-scale-linkage` の段 4 の実機の確かめ直し。
- **Out**: descript の鍵の読み（`descript-window-keys` の持ち物）・並べ替えの判定と右に出すときのずらしの符号（`balloon-flip-side-rules`）・キャラ窓の寄せ（`work_area_clamp.rs`）・3 体以上の窓（`extra-character-windows`）。

## Boundary Candidates

- 命令の受け口（キャリアの受け手の表から置き方の値への橋）
- 置き方の値の層（descript の既定に台本の値を重ねる・寿命）
- 押し戻しの関門がどの画面の作業領域を見るか

## Out of Boundary

- 倍率・DPI の値そのもの（`shell-scale-linkage` の表）。この spec は窓の置き場所だけを変える。
- バルーンの大きさ・面の選び。

## Upstream / Downstream

- **Upstream**: 完了 spec `shell-scale-linkage`（倍率の表と段 4 の約束）・`descript-window-keys`（`balloon.offsetx`・`balloon.alignment` の読み）・`window-work-area-clamp`（押し戻しの決まり）・`areka-P0-balloon-offset-dpi`。
- **Downstream**: ぱすたの倍率の確かめ（ぱすた開発のセッション・離して置けるようになったら声をかける約束）。

## Existing Spec Touchpoints

- **Extends**: なし（完了 spec からの引き渡し）。
- **Adjacent**: `balloon-flip-side-rules`（`balloon_limit.rs` の並べ替えと P5 の右の式を触る＝同じウェーブに置かない）・`balloon-canon-residue`（`resolver.rs` のバルーンの左右の判定）・`placement-crate-split`（`placement/` を移す）・`audit-placement-core`・`audit-placement-follow`。

## Constraints

- 失敗の道は必ず記録（`error!`＋`Err`）。受け取った値が壊れていれば送らず・置かずに `warn!`（未検証の字は `?` で残す）。押し戻しの記録は今の `[balloon-limit] Clamp` の行の欄で足す。
- テストは兄弟のファイル `*_tests.rs`。`window_move.rs` は前から 1,000 行を超えている＝足す分は別のファイルに置く。
- SSP を測るときは毎回開発者の許しを取る。試験の根・検体はワークツリーの `target\` の下だけ。実機の確かめは排他テストの机を持って回す。

- **優先度の区分**: C（ukadoc の拾い残し・台帳 2 行は A2）。
- **版**: 要件の段で決める（ぱすたの v0.1.0 の検体は本体とバルーンを重ねる形で、この命令を使わない）。
- **要件定義のモデル**: Opus。受け口と値の層は既存の型どおりで、押し戻しの画面の選びも 1 つの決めで済む。
- **規模**: M（8〜12 タスクの見込み・見積もりは要件の段）。
- **分割の案**: なし（2 つの命令は同じ値の層に入り、実機の確かめも 1 回の机でまとめて取れる）。
- **要件の段の議題**（答えで作業が変わるもの）:
  1. 押し戻しの関門の画面（バルーン自身が居る画面の作業領域で見る／本体の画面の中に留める）。SSP で測ってから決める。本体の画面に留めるなら段 4 は作れないので、確かめ直しの扱いを決め直す。
  2. 台本の値の寿命（保存する・シェルやバルーンの切り替えで消す・ゴーストの終わりで消す）。ukadoc と SSP で決める。

## 2026-11-20 棚卸㊵の再測定（main 67e0c70e）
- **前提の変化**: 見直しの 4 本が着地した＝`consumer_ledger.rs` の `audit-emo2-boot-spine`（11-19・受け手の表を `CANONICAL_ROWS` の 1 表にまとめ、手書きの数を外した＝行を足すのは表の 2 行と `consumer_ledger_agreement_tests.rs` の見本だけ）と、配置の `audit-placement-core`・`audit-placement-follow`・`audit-placement-follow-residue`。`balloon-flip-side-rules` も着地した（`completed/`）。crate 分けで本文のパスは古い（`crates/areka/src/placement/` → `crates/areka-placement/src/`・`emo2_boot/` → `crates/areka-runtime/src/`）。版は v0.2.0（棚卸㉝）＝本文の「版: 要件の段で決める」は古い。
- **触るファイル**: `crates/areka-runtime/src/consumer_ledger.rs`（412 行・表に 2 行）・`consumer_ledger_agreement_tests.rs`・`crates/areka-placement/src/resolver.rs`（564・P5）・`balloon_limit.rs`（350）・`follow/window_move.rs`（1,234・上限の例外表に載る＝足す分は別のファイル）・値の層の新しいファイル（`areka-placement` の中の見込み）・網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の 3043・3088 行目の 2 項目（別々の行は許す重なり）。今の型（`\![lock,balloonmove]`・`\![set,alignmenttodesktop]` は `MoveSink`＝`move_cue.rs`）に倣えば `crates/areka-runtime/src/move_cue.rs`（373）・`move_cue_args.rs`・`frame/move_hold.rs`（271）・`frame/drain_resnap.rs`（759）も触る見込み（設計で決める）。値の寿命を切り替えで消すなら `crates/areka/src/session/ghost_switch.rs`・`shell_balloon_switch.rs` にも及ぶ。
- **規模**: M（8〜12 タスク）のまま。
- **先に要るもの**: 配置と `consumer_ledger.rs` の見直しは済んだ。残るのは `move_cue*` の持ち主 `audit-emo2-boot-switch`（未完了・`session-crate-split` の後＝見込み C26）と、`frame/` の持ち主 `audit-emo2-boot-frame`（見込み C23）＝版の段の「v0.1.0 の後の隙間」の取り方 ⑶。`move_cue*` に触らない設計（受け手を新しいファイルに置く）にできなければ、早くて C27。
- **同じウェーブに置けない相手**: `window-resetwindowpos-hold`（`frame/move_hold.rs`・`frame/drain_resnap.rs`）＝見込みの C25 で並べているのは誤り。`balloon-text-visual-tree`（`drain_resnap.rs`）・`balloon-flip-ssp-review`（案 ⑵・⑶ で `balloon_limit.rs`・`follow/window_move.rs`）・`audit-emo2-boot-frame`・`audit-emo2-boot-switch`（待つ）・`install-live-target-hazards`（`drain_resnap.rs`）・`consumer_ledger.rs` に行を足すほかの spec（1 本ずつ）・`balloon-canon-residue`・`window-snap`（`resolver.rs`）。
- **優先度の区分**: C（ukadoc の拾い残し・台帳の 2 行は A2）。
- **要件定義のモデル**: Opus（受け口は `MoveSink` の型どおり・値の層は descript の P5 に重ねる 1 層・押し戻しの画面は 1 つの決め）。
- **分割の案**: 無し。
- **見つけた穴**: ⑴ 見込みの C25 の並べ方（`window-resetwindowpos-hold` と同じウェーブ・`audit-emo2-boot-switch` より前）。⑵ roadmap の「直列の列」の `consumer_ledger.rs` の鎖の「手書きの数も直す」は、手書きの数が消えて古い。⑶ 本文の Current State・Approach の `crates/areka/src/placement/…`・`placement/` のパスは古い（上の読み替え）。
