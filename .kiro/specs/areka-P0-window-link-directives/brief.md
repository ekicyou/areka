# Brief: areka-P0-window-link-directives

> 2026-10-17 `/kiro-discovery` で起票（開発者「新しい move 形式を含む周辺仕様を ukadoc から取り込み」→「とりあえず 1 本、2 本目は 0.1.0 に回す。アニメーションに絡むのは、dola の改造後」）。1 本目は `move-canon`（v0.0.3）。着手の区分は **A（開発者が起票）**。版は **v0.1.0**。

## Problem

ukadoc の窓の位置の台本の指令のうち、窓どうしをつなぐもの（`\![set,sticky-window]`）と、窓を画面の端へ吸着させるもの（`\![set,alignmenttodesktop]`）を areka は受けない。どちらも運ばれるだけで誰も受け取らず、黙って何も起きない。`\![move]` の新しい形の `--option=ignore-sticky-window` も、つなぐ仕組みが無いので意味を持たない。

## Current State

- `\![set,alignmenttodesktop,方向]`: 台本の指令は未実装。descript の 4 段の優先（`seriko.alignmenttodesktop` と `sakura.`／`kero.`／`char*.`）は実装済み＝`placement/config.rs:120-223` → `Anchor` → 部品 `Anchored`（`placement/follow/anchor.rs:180`）。受ける側の system `anchor_changed_system`（`placement/follow/window_move.rs:412-423`）は定義だけあり「cue の振り分けは非所有」と書いてある。
- `\![set,sticky-window]`／`\![reset,sticky-window]`: 未実装。descript の `seriko.sticky-window` は生のまま書き写すだけ（`placement/config.rs:106,135`）。プロパティ `currentghost.seriko.sticky-window` は sylphya の語彙表に名前だけ（`areka-sylphya/src/vocab/dotted.rs:132`）。
- 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml`: `set,alignmenttodesktop` と `,free`（:3151）・`set,sticky-window`（:3266）・`reset,sticky-window`（:2948）は absent・持ち主なし。
- 受け手の登録は `emo2_boot/consumer_ledger.rs`（943 行）。

### ukadoc の正典（2026-10-17 に読んだ）

- `\![set,alignmenttodesktop,方向]`: 方向は top・bottom・left・right・free・default。上下に吸着すると上下へドラッグできず、左右も同様。「ゴースト終了まで有効」。`alignmentondesktop` の綴りでも効く。兄弟の項目 `\![set,alignmenttodesktop,free]`。
- `\![set,sticky-window,スコープID,スコープID,...]`（2.3.77）: 「ドラッグ中やmoveタグを実行中にセットで動く」。組を複数作れて、組んだ ID を含むタグをもう一度打つとエラー。解除は `\![reset,sticky-window]`（2.3.77・descript の値へ戻る）。descript の `seriko.sticky-window`（2.4）・プロパティ `currentghost.seriko.sticky-window`（2.8.78）。
- `\![move]` の `--option=ignore-sticky-window`: 「\![set,sticky-window]での連携設定を無視して単独で動かす」。

## Desired Outcome

- `\![set,alignmenttodesktop,方向]` で、台本からキャラクターの窓の吸着を変えられる（`default` で descript の値へ戻る）。吸着した向きにはドラッグで動かない。
- `\![set,sticky-window,…]` で組んだ窓が、ドラッグと `\![move]` で一緒に動く。`\![reset,sticky-window]` で descript の値へ戻る。descript の `seriko.sticky-window` も効く。
- `\![move]` の `--option=ignore-sticky-window` で、組を無視して 1 つだけ動く。
- 台帳の 3 行と COMPAT §8 が新しい実装に合う。

## Approach

`move-canon` の後に、受け手を足して既存の部品へ流す。吸着は `Anchored` と `anchor_changed_system` が既に在るので、台本の指令を振り分けるだけ。つなぐ組は新しい部品（組の持ち主）を置き、ドラッグの道と `\![move]` の適用（`apply_move_directive`）の両方から組の相手を同じだけ動かす。時間つきの移動（dola の改造の後）では組の相手も同じ時間で動く必要がある＝そのときは時間の spec が引き取る。

## Scope
- **In**: `\![set,alignmenttodesktop]`（`alignmentondesktop` の綴りと `free`・`default` を含む）・`\![set,sticky-window]`・`\![reset,sticky-window]`・descript の `seriko.sticky-window` を効かせる・`--option=ignore-sticky-window` の効き目・ドラッグと瞬間の `\![move]` での組の連れ動き・台帳と COMPAT の行
- **Out**: 時間つきの移動・`\![moveasync]` での連れ動き（dola の改造の後）・ほかのアプリの窓への吸着（`window-snap`・夢）・3 人目以降の窓（`extra-character-windows`）・プロパティ `currentghost.seriko.sticky-window` の値（`currentghost-property-tree`）

## Boundary Candidates
- 吸着の台本の指令（受け手 → `Anchored` の書き換え → `anchor_changed_system`）
- つなぐ組の持ち主（部品）と、ドラッグ・`\![move]` からの連れ動き

## Out of Boundary
- 吸着の相手を作業領域の辺から広げること（`window-snap`）
- 時間つきの移動での連れ動き

## Upstream / Downstream
- **Upstream**: `move-canon`（v0.0.3・`--option` の語彙と参照点の計算・`apply_move_directive`）・✅ `window-placement`（`Anchored`・吸着の 4 段の優先）・✅ `scope-zorder-pinning`（組を扱う受け手の型）
- **Downstream**: `window-snap`（吸着の土台の持ち主の議題 1）・`extra-character-windows`（3 人目以降の窓での組）・`currentghost-property-tree`（`currentghost.seriko.sticky-window`）・時間つきの移動（dola の改造の後）

## Existing Spec Touchpoints
- **Extends**: なし
- **Adjacent**: `move-canon`（同じ `move_cue.rs`・`consumer_ledger.rs`＝この spec が後）・`window-snap`（同 brief の議題 1「土台の持ち主」にこの spec が答える）・`tasktray-*`（`consumer_ledger.rs` に行を足す＝1 本ずつ）

## Constraints
- 台本で動いた窓は位置の記憶に書かない（`char-position-save-on-exit`）。組の相手も同じ。
- `consumer_ledger.rs`（943 行）は 1,000 行の決まりに近い＝受け手の中身は兄弟のファイルへ。
- 「ゴースト終了まで有効」は台本の指令の効き目の寿命＝ゴーストの切り替えで descript の値へ戻る。

## 申し送り（2026-10-17・`areka-P0-move-canon` の要件ディスカッションから）

- `\![execute,resetballoonpos]`（ukadoc 2.7.41「各バルーンの位置を初期位置に戻す。メニューの「バルーン位置初期化」と同じ働きをする」）・メニューの「バルーン位置初期化」（`resetballoonpositionbutton.caption`）・SSP ヘルプ（「よくある質問」「ショートカット一覧」）の「シェル位置初期化」「バルーン位置初期化」のキーボードの近道を、本 spec で引き取る。`move-canon` が入れるのは「シェル位置初期化」の項目・`\![execute,resetwindowpos]`・`OnResetWindowPos` だけ（並べ直しの関数とメニューの枠は `move-canon` の設計を下敷きにできる）。
- `--option=ignore-sticky-window` は `move-canon` で語彙として受け取るだけで、動きは変えない。効き目を持たせるのは本 spec。
- SSP 2.9.18 の実測（`move-canon` の `research.md` §8）: `\![move]` の基準 `screen`・`primaryscreen` は作業領域。`\![set,position]`／`\![reset,position]` は SSP も実装していない（「unknown subcommand」）。
- `move-canon` で `\![move]` は今の吸い付き（`Anchored`・`project_anchor`）に従うようになる（下端に吸い付いている間は Y を無視・SSP 2.9.18 の実測と同じ）。本 spec の `\![set,alignmenttodesktop,方向]` で向きを変えれば、`\![move]` もその向きに従う＝本 spec で `\![move]` 側に足すものは無い見込み。

## 2026-10-18 棚卸㉗の再測定（main 2d86dbff）
- 前提の変化: 上流の `move-canon` が着地した（10-18・PR#332）。`--option=ignore-sticky-window` は語として読まれて欄 `ignore_sticky_window` に入るが、誰も読まない（`crates/areka/src/emo2_boot/move_cue.rs` 63 行「今は誰も読まない（window-link-directives が意味を持たせる）」・台本の移動の適用 `emo2_boot/move_cue_place.rs` の `apply_move_directive` も 103 行で「読まない」）。シェルの `seriko.sticky-window` は今も生のまま写すだけ（`placement/config.rs` 107・135 行）。
- 棚卸㉗で足した範囲（理由と証拠）: 範囲は変えないが、v0.1.0 のゴール「ぱすたが SSP と同じレベルで動く」が待つ相手になった。ぱすたはシェルの descript に `seriko.sticky-window,0,1`（窓の枠＝相方と、ぱすた＝本体を同じ位置に重ねて一緒にドラッグ）を書き、辞書の「窓あわせ」で `\![move,…,--option=ignore-sticky-window]` を出して相方だけを動かす（ゴースト「窓際のぱすたさん」の `shell/master/descript.txt`・`dic/tategaki.pasta`）。
  - ぱすたが使うのは組と `ignore-sticky-window` だけ。`\![set,alignmenttodesktop]` は使わない（descript の `seriko.alignmenttodesktop,free` は実装済み）。
  - 組の窓のバルーンも一緒に追従することを確かめる項目に入れる（ぱすたの縦書きのバルーンは窓の枠の絵に重ねてある）。
- 触るファイル: `emo2_boot/move_cue_place.rs`（206）・`move_cue.rs`（308）・`placement/config.rs`（687）・`placement/follow/drag_follow.rs`（937・1,000 行の近く＝足す分は兄弟へ）・`placement/follow/anchor.rs`（181）・`emo2_boot/consumer_ledger.rs`（960）・新しい組の持ち主のファイル・申し送りの「バルーン位置初期化」を足すなら `menu/reset_pos_frame.rs`（139・`move-canon` が足した枠）・網羅台帳の 3 行・COMPAT §8。`placement/follow/window_move.rs`（1,221・行数の番人の例外）は触らずに済ませたい。
- 規模: 10〜14 タスク（吸着の指令 2〜3・組 5〜6・バルーン位置初期化 2〜3・記録 1）。
- 先に要るもの: 無し（`move-canon` は着地済み）。
- 重なり: `descript-window-keys`（新・`placement/config.rs`・`drag_follow.rs` のバルーンのドラッグの入口）・`dpi-realign-remembered-chain`（下の穴）・受け手の表 `consumer_ledger.rs` に行を足す v0.1.0 の spec（`stay-on-top-directives`・`script-small-tags`・`descript-window-keys`・`mcp-reload`・`mcp-strict-errors`・`script-security-level`＝行を足すだけなので 1 本ずつ）・`extra-character-windows`・`window-snap`（いつか）。
- 優先度の区分: A（開発者が起票・v0.1.0）。要件定義のモデル: Opus（先例＝`move-canon` の参照点と `Anchored`）。
- 分割の案: 20 を超えそうなら「吸着の指令」と「組」に分ける。ぱすたに要るのは「組」の側。
- 見つけた穴・古くなった記述:
  - 本文 Upstream の「`move-canon`（v0.0.3・…）」は着地済み。本文に規模と要件定義のモデルが無かった（上に書いた）。
  - 拡大率が変わった後の詰め直し（`placement/chain_realign.rs`）が、同じ位置に重ねた組の窓を横に並べ直さないこと。組の側と詰め直しの側のどちらが持つかを要件で決める（`dpi-realign-remembered-chain` にも書いた）。
  - SSP で測る: 組の片方をドラッグしたとき、相手のバルーンも一緒に動くか。

## 2026-10-18 棚卸㉗の申し送り（持ち主が居なかった残り物）

- 完了 `move-canon` の tasks.md の Implementation Notes 2.2: 既定の位置の基準（`DefaultPlacementBasis`）が、起動のときとシェルの切り替えのときで違う設定を持つ。`\![execute,resetballoonpos]`（本 spec の範囲）でバルーンの位置をこの基準から出すなら、先に 2 つをそろえること（そろえないと、起動の直後とシェルを替えた後で並べ直しの結果が変わる）。

## 2026-10-22 棚卸㉘の再測定（main 52b014de）
- 前提の変化: 触る所の中身は棚卸㉗から変わらない（`--option=ignore-sticky-window` は `crates/areka/src/emo2_boot/move_cue.rs` 64 行目の欄に入るだけで、台本の移動の適用 `move_cue_place.rs` は 103 行目の注記どおり読まない。シェルの `seriko.sticky-window` は `placement/config.rs` 107・135 行目で生のまま写すだけ）。✅ `mcp-reload`（10-22）で受け手の表 `emo2_boot/consumer_ledger.rs` が 472 行になった。同じ spec で、ゴーストの読み直しは窓を閉じて作り直すようになった＝台本でつないだ組と吸着が、読み直しの後に残るかを決める必要が出た。
- 触るファイル: `emo2_boot/move_cue_place.rs`（206）・`move_cue.rs`（308）・`placement/config.rs`（687）・`placement/follow/drag_follow.rs`（937・足す分は兄弟へ）・`placement/follow/anchor.rs`（181）・`consumer_ledger.rs`（472）・新しい組の持ち主のファイル（新）・バルーン位置初期化を足すなら `menu/reset_pos_frame.rs`（139）・詰め直しで組を外す側を持つなら `placement/chain_realign.rs`（245）・網羅台帳 `sakura-script.toml` の 4 行（2821・2939・2955・3273 行目）・`doc/COMPAT_ARCHITECTURE.md` §8。`placement/follow/window_move.rs`（1,221・行数の番人の例外）は触らずに済ませたい。
- 規模: 10〜14 タスクのまま。
- 先に要るもの: 働きの上ではなし。同じウェーブに置けない v0.1.0 の相手: `descript-window-keys`（`config.rs`・`drag_follow.rs`）・`dpi-realign-remembered-chain`（`chain_realign.rs` をこちらが持つ場合・働きでもこちらが先）・受け手の表に行を足す `stay-on-top-directives`・`script-small-tags`・`mcp-strict-errors`（案 2）・`script-security-level`（1 本ずつ）。後の版: `window-snap`・`extra-character-windows`・`shell-balloon-scale-setting`（`move_cue*.rs`）・`check-script-arg-checks`・`element-animation-easing`・`surfaces-basepos`（`move_cue.rs`）・`resident-ghost-rotation`（`reset_pos_frame.rs`）。依存を足さない。実機（ぱすたの 2 つの窓を一緒にドラッグ）は排他テストの机。静かな机・前後の全体テストの比べは要らない。SSP の測りは開発者の機械。
- 優先度の区分: A（開発者が起票・v0.1.0）。
- 要件定義のモデル: Opus（先例＝`move-canon` の参照点・`Anchored`・今のドラッグの道に乗る＝新しい境目や並行の保証は無い）。
- 分割の案: 20 を超えそうなら「吸着の指令」と「組」に分ける（変わらず）。今の見立て 10〜14 では分けない。
- 見つけた穴・古くなった記述:
  - 本文 Current State の網羅台帳の行番号（3151・3266・2948）と「`consumer_ledger.rs`（943 行）」は古い（上）。Constraints の「1,000 行に近い」も今は当たらない（受け手の中身を兄弟に置くのは変わらない）。
  - ゴーストの読み直しの後に、台本でつないだ組・台本で変えた吸着を保つか descript の値へ戻すかを議題に足す（正典は「ゴースト終了まで有効」・読み直しは終了ではない）。
  - `dpi-realign-remembered-chain` が要る「組の窓は詰め直しから外す」の目印を、この spec の組の持ち主に持たせるかを要件で決める（持たせれば、向こうは `chain_realign.rs` の判定だけで済み、このファイルの重なりも消える）。

## 本 spec からの申し送り（2026-10-23・`areka-P0-pasta-in-windows-verification` のタスク 6）

- 出どころ: `areka-P0-pasta-in-windows-verification` の検証の記録 `verification.md`（5 節の崩れた所の表。件の番号は表の番号）と、SSP の測りの記録 `ssp-measurement.md`（SSP 2.9.23・書庫の sha256 `ad473d5d…a43a6`＝`pasta.toml` 1.0.0）。撮った物と記録は git の外の `<w>\target\pasta-verify\`（本走行 run1。`logs\`・`shots\run1\`・SSP の撮った物は `ssp\`）・`<w>\target\pasta-verify-alt\`（補いの走り alt2。縦書きのバルーンを根の `balloon\` へ写した置き方）・`<w>\target\pasta-verify-rerun\`（直しの後の回し直し rerun1。alt2 と同じ置き方）に在る（`<w>` は `C:\home\maz\git\areka\.claude\worktrees\areka-p0-pasta-windows-d4ed7a`）。記録の時刻は世界標準時（`Z`）のまま。日本時間は 9 時間足す。
- 開発者の決め（2026-10-23）「原則として、すべての申し送りが0.1.0に解決するのであれば承認する。申し送り漏れはダメですよ。」。この spec は v0.1.0 なので、下の件はこの spec の中で片づける。
- 着地した後の確かめは、ぱすたの手順書 `doc/pasta-real-run.md`（5 節の 7 段の表）で回す。
- **件 6（起動の並べ直しで窓の枠が別の画面へ送られる）は本 spec で直した**（タスク 5.4・コミット `97186487`・2026-10-23 開発者の承認）: 起動の最後の並べ直し `finalize_chain`（`crates/areka/src/placement/chain_finalize.rs`。直す前は 121 行目）が、組の窓が既定の位置のままかを x だけで比べていた（`default_x != Some(current_x)`）。ぱすたの台本の `\![move,--X=0,--Y=0,--base=0,…]` は `\1` を `\0` の左上（0,864）へ動かしたが、x が既定の 0 のままだったので「動かされていない」と読まれ、`\0` の左隣（x＝-1108・副画面）へ並べ直されていた（run1 の `2026-10-22T23:17:58.830231Z` `chain_finalize: 実表示寸で連鎖を再解決 … scope=1 from_x=0 to_x=-1108`）。いまは x と y の両方で比べる。既定の位置を集めて渡す所は `crates/areka/src/emo2_boot/frame/drain_resnap.rs` の `collect_chain_states`。赤から緑のテストは `chain_finalize_tests::vertically_moved_scope_is_not_pulled_back`。拡大率の後の詰め直し（`placement/chain_realign.rs`）も同じ判定を使う。rerun1 では `scope=1` の `chain_finalize` の行が 0 件で、`\1` は台本どおり (0,864) へ動いたまま、ぱすたの奥に出た（`<w>\target\pasta-verify-rerun\shots\rerun1\1-boot-primary-ghost.png`）。この spec が `chain_finalize.rs`・`chain_realign.rs` を触るときは、この直しの後の形の上で考える。
- **件 7（ドラッグで窓の枠が付いてこない）はこの spec の持ち分**: 同じ画面の中のドラッグで、ぱすた（`\0`）だけが動き、窓の枠（`\1`）が付いてこない。拡大率の違う画面へのドラッグでも同じ（run1 の段 5 でも窓の枠は動かず、ぱすたと離れたまま）。
  - 根拠: run1 の `2026-10-22T23:23:03.468835Z` `char DragEnd 保存 scope=0 char_x=800 char_y=464`（`\1` の保存の行は無い・`logs\run1.stage4-5-excerpt.log`）・`shots\run1\4-before-drag-full.png`・`4-after-drag-full.png`・`5-after-move-full.png`。rerun1 でも、ぱすたのドラッグで保存されたのは `scope=0` だけで、窓の枠は別に手でドラッグされた（`<w>\target\pasta-verify-rerun\shots\rerun1\notes.md` の「開発者の手の操作が入った」）。
  - 原因: `crates/areka/src/placement/config.rs:107`・`135`。シェルの `seriko.sticky-window,0,1` を生のまま写すだけで、ドラッグに組の相手を連れて動かす所が無い。
- **SSP の基準・測りの答え**（`ssp-measurement.md` 2 節の段 4・3 節の 5 行目）: 身体をつかんで同じ 150% の画面の中で左へ 700 動かすと、`scope(0)` と `scope(1)` がどちらも `-782,201,49,1227` から `-1482,201,-651,1227` へ揃って動いた（`ssp\stage4-before-shot.png`・`stage4-after-shot.png`・`stage5-rects.txt` の 1 行目）。ドラッグに伴うイベントは出なかった。200% の画面へ移したときも、2 つは同じ rect（`818,261,1649,1287`）のまま。起動の台本の `\![move,…,--option=ignore-sticky-window]` で `\0` だけを動かしたときに `\1` が付いてくるかは測っていない。
- 網羅台帳の宛先: 2026-10-23 に 1 行の `owner` をこの spec にした（`verification.md` 10 節の「直したことと残り」）。
- **着地で変わるはずの所**（手順書の段 1・段 4）: 窓の枠がぱすたに重なって付き（起動のときの重なりは件 6 の直しで出るようになった＝着地の後も崩れないことを確かめる）、ドラッグで窓の枠がぱすたについて動き、落とした後に付き直る。
