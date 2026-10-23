# 実機確認の記録（areka-P0-window-link-directives・タスク 9.2・要件 9.7・9.8）

検体「窓際のぱすたさん」（`pasta-in-windows`）で、窓の組の連れ動き・`ignore-sticky-window`・次の起動での重なり・位置初期化の 2 つの近道を確かめた。実機の走りは 4 回で、**判定は 4 回目の結果による。4 回目は全項目が合格した。** 操作はすべてハーネス（`SendInput` と MCP）が行い、判定は窓の矩形と記録で機械的に出した。開発者の目視は使っていない。

## 共通

- 日時（4 回目）: 2026-10-23 10:53〜10:55 JST（記録は UTC の 01:53〜01:55）
- ブランチ: `claude/areka-p0-window-link-directives-1f6eee`（HEAD `1674245b` に、下の「直したもの」の未コミットの変更を足した作業ツリー）
- 環境: Windows 11 Pro 10.0.26300。排他テストの机を持ち、入力の机が `Default`（画面のロックもスクリーンセーバーも無い）ことを `OpenInputDesktop` で確かめてから走らせた
- モニタ:
  - 1〜3 回目: 2 台。主 2880x1800（作業領域 0,0〜2880,1704・DPI 192＝200%）と副 2560x1600（-2560,195〜0,1795・DPI 144＝150%）
  - 4 回目: 主の 1 台だけ（`[diag.monitor_snapshot] count=1`・DPI 192）。ドラッグは右上へ引くので、2 台でも副のモニタへは渡らない
  - 確かめの間に拡大率は変えていない。4 回目の窓の DPI は全部 192 のまま
- ハーネス: `crates/areka/tests/window_link_real_run.rs`（`real-run-kit` で起こす・既定 OFF）。走らせたコマンドは `AREKA_WINDOW_LINK_REAL_RUN=1 cargo test -p areka --test window_link_real_run -- --nocapture`（1 回）。結果は `test result: ok. 1 passed`、91.15 秒
- 検体: `tools/sample-fetch.ps1` で書庫を取り寄せ、`cargo run -p sample-ghost-kit --bin nar-sample-path -- pasta-in-windows` で `target\nar-samples\manual\pasta-in-windows\` へ展開したもの
- 根と記憶の置き場（要件 9.8）: すべて作業ツリーの `target\wl-signoff\` の下
  - 根 `target\wl-signoff\root\` には、展開した `ghost\` の写しと、ゴーストの中のバルーン `pasta_tategaki` を根の `balloon\pasta_tategaki\` へ写したものを置いた
  - アプリの記憶は `target\wl-signoff\profile\`（開発者の記憶は読まず・書かない）
  - 根は絶対パスで渡し、`\\?\` は付けない
- `RUST_LOG=info,kanade=debug,areka::placement=debug,areka::input_events=debug,areka::menu=debug,wintf::ecs::window_proc::keyboard=debug,wintf::ecs::clickthrough=debug`。`sticky_groups`・`sticky_follow`・`sticky_follow_ignored`・`balloon_pos_reset`・`reset_pos_menu_outcome`・`move_applied`・`merge_scope restore`・`[diag.window_move]`・クリックの透けの切り替えが出る水準
- 走りは 2 本: 走り 1 は記憶なしで起こし、組・ドラッグ・`ignore-sticky-window` を見る。走り 2 は記憶を残したまま起こし直し、重なりと近道を見る。どちらもハーネスが閉じた（`Ended { status: 0, by: UserClose }`）
- 記録: `target\wl-signoff\judgments.txt`（判定）・`steps.txt`（手順と矩形）・`run1.log`・`run2.log`

## 判定（4 回目）

| 判定 | 結果 | 数字・記録の抜粋 |
|---|---|---|
| 1 起動で `sticky_groups`（descript_seed・`{0,1}`） | PASS | `event="sticky_groups" action="descript_seed" groups=[StickyGroup { members: {0, 1}, source: Descript }]` |
| 1b 起動の連鎖の後で 2 つのキャラクターの窓が重なる | PASS | ぱすた (0,864)・窓 (0,864)。どちらも矩形 [0,864,1108,2232] |
| 2 ドラッグがぱすたのキャラクターの窓から始まる | PASS | 押した点 580,1564（胴の論理 (290,350)×2）。`[DragStartEvent] Dispatching entity=26v0 x=580 y=1564`、ぱすたの窓の entity も 26v0 |
| 2a ドラッグの途中で両キャラクターの窓が同じ量動く | PASS | 途中（20 歩のうち 10 歩目）: ぱすた (+120,−60)・窓 (+120,−60) |
| 2b ドラッグの後で両キャラクターの窓が同じ量動いた | PASS | ぱすた (+240,−120)・窓 (+240,−120)。ぱすたは [240,744,1348,2112]、窓は [240,744,1348,2112] |
| 2c `sticky_follow`（drag）が出る | PASS | `event="sticky_follow" trigger="drag" scope=0 partners=[1] dx=240 dy=-120` |
| 2d ドラッグの間に窓のバルーンが付いて動く | PASS | `[diag.window_move] route=BalloonFollow … scope=1` が 34 行（先頭 `entity=27v0 kind=balloon scope=1 x=0 y=804`）。バルーンは作業領域の端で止まるので、矩形の量でなく記録で見た（窓のバルーンは作業領域の左端 x=0 で止まり、y だけ −60 動いた） |
| 3a `ignore-sticky-window` の move で窓だけが動き、ぱすたは動かない | PASS | 窓 (+400,0)、ぱすたの 2 枚は (0,0) |
| 3b `sticky_follow_ignored` が出る | PASS | `event="sticky_follow_ignored" scope=1 partners=[0]` |
| 3c 窓あわせで窓だけがぱすたへ戻り重なる | PASS | 窓 (−400,0)、ぱすた (0,0)。重なった位置はぱすた (240,744)・窓 (240,744)。`sticky_follow_ignored scope=1 partners=[0]` |
| 4 起こし直すと 2 つの窓が重なったまま前回の位置で出る | PASS | 走り 2: ぱすた (240,744)・窓 (240,744)。走り 1 の終わり: ぱすた (240,744) |
| 5a Ctrl+Shift+F10 で `balloon_pos_reset`（trigger=shortcut） | PASS | クリックした点 820,1444 の下はぱすたの窓。入力先は areka（pid 30284）。`event="balloon_pos_reset" scope=0 to_x=320 to_y=804 offset_x=80 offset_y=116 trigger="shortcut"` |
| 5b Ctrl+Shift+F12 で `reset_pos_menu_outcome` | PASS | `event="reset_pos_menu_outcome" outcome="no_reply"`（`OnResetWindowPos` に返事が無いので既定の位置へ並べ直した） |

- 近道を押す前に、点の下がぱすたの窓（`WindowFromPoint`）であることと、前に出ている窓が areka であること（`GetForegroundWindow` の pid）を確かめた。areka でなければキーを送らずに赤にする作りで、4 回目はどちらも areka だった。

## F10 の届き方

Ctrl+Shift+F10 は areka に届き、`balloon_pos_reset` が trigger=shortcut で出た（5a）。F10 は Windows ではメニューの活性化（`SC_KEYMENU`）に使われるキーだが、それに吸われて近道が消えることは無かった（tasks.md の Implementation Notes 2.4 の確かめ）。

## 追加の確かめ

- 起動の連鎖の確定は、台本が置いた窓を引き戻さない（4 回目）
  - 走り 1 の起動では、ぱすたの辞書の「窓あわせ」が窓を動かした: `move_applied scope=1 … from_x=0 from_y=336 script_x=0 script_y=864 to_x=0 to_y=864`
  - その後、`chain_finalize: 実表示寸で連鎖を再解決` は走り 1・走り 2 とも 0 行、scope 1 の `DpiReproject` も 0 行だった
- 記憶の寄せ直し（`merge_scope restore`）の行
  - 走り 1（記憶なし）: scope 0 は `saved_win_x=None saved_win_y=None default_char_x=0 default_char_y=864 char_w=640 char_h=840`、scope 1 は `saved_win_x=None saved_win_y=None default_char_x=0 default_char_y=336 char_w=1108 char_h=1368`
  - 走り 2:
    - `merge_scope restore scope=0 anchor=Free saved_win_x=Some(240) saved_win_y=Some(744) default_char_x=0 default_char_y=864 char_x=240 char_y=744 char_w=640 char_h=840 …`
    - `merge_scope restore scope=1 anchor=Free saved_win_x=Some(240) saved_win_y=Some(744) default_char_x=0 default_char_y=336 char_x=240 char_y=744 char_w=1108 char_h=1368 …`
  - 走り 2 で両方の窓に記憶があるのは、ドラッグの確定（本体と組の相手の位置を書く・要件 4.5）による。台本の移動で置いた窓は記憶に書かない（境界の節・要件 5.4・`areka-P0-char-position-save-on-exit` 要件 1.7）
- ドラッグを始める前に、ぱすたの胴の点の下では `clickthrough: ex-style トグル適用 window=26v0 desired=Opaque` が出た（透けない点を押した）

## 4 回の走りの経緯

| 回 | 結果 | 見つかったこと | 直したもの |
|---|---|---|---|
| 1 | 走りにならない | ハーネスが `canonicalize` で作った `\\?\` 付きの根では、pasta が辞書を探せず LOAD に失敗した | ハーネス: 根を `std::path::absolute` で作り、`\\?\` が付かないことを assert する |
| 2 | PASS 4・FAIL 5（1b・2・4・5a・5b） | ⑴ 起動の連鎖の確定が、窓あわせで縦にだけ動いた窓（窓 (0,336)→(0,864)、x は既定の 0 のまま）を「動かされていない」と見て、同じフレームで x=−1108 へ並べ直した。150% のモニタへ渡り、−831 へ詰め直された（1b・4 が赤）。⑵ ハーネスが押した点がバルーンか透けた点で、クリックがほかのアプリへ抜けた。点 3 は作業領域の下（y=1753 > 1704）にあった（2・5 が赤） | ⑴ 製品（下の「直したもの」）: 台本が置いた窓の印 `ScriptPlaced` を使い、完了済み `areka-P0-scope-chain-gap` 要件 7.3 を守る。⑵ ハーネス: 押す点をぱすたの胴に決め、乗せてから `WindowFromPoint` と作業領域で確かめる。右上へ引き、`DragStartEvent` の entity を照合し、バルーンは記録で見る。入力先が areka のときだけキーを送る |
| 3 | PASS 7（1b・4 が緑）・FAIL 3（2・5a・5b） | 起動の挨拶（約 11 秒）の最中に `\0\s[1000]\e` を送ったので、kanade に断られた（`NG:the ghost cannot accept a script now`・起動のトークの間は受けない作り）。ぱすたは胴が透ける面 1002 のままで、点の下はほかのアプリの窓だった | ハーネス: 最初の `steady_talk_done` を待ってから送る。断られたら 1 秒ごとに最大 10 回送り直し、閉じた行は 20 秒待つ。ぱすたの窓を `HWND_TOP`（`SWP_NOACTIVATE`）で前へ上げ、点を最大 5 秒確かめ直す。使えないときは見分けの材料（カーソルの位置・点の下の pid・`WS_EX_TRANSPARENT`・覆われているか）を返す。`wintf::ecs::clickthrough=debug` を開ける |
| 4 | 全 PASS | 走り 1 の点の確かめで、前へ上げる前は点の下がほかのアプリの窓だった（`covered=false hit=0x1509ea hit_pid=22540`）。前へ上げたあとはぱすたの窓になった。閉じた行（`trigger="clear"`）は 20 秒の内に出なかったが、点はぱすたの窓だった。走り 2 では閉じた行が出て、点の下は初めからぱすたの窓だった | — |

### 直したもの（タスク 9.2 の中）

- 製品（scg 7.3・設計の「実機」の節）
  - `placement/chain_finalize.rs`: 台本の `\![move]` が置いたキャラクターの窓の印 `ScriptPlaced`（Component）を足した
  - `emo2_boot/move_cue_place.rs` の `apply_move_directive`: 窓へ書けたら、動かした窓にその印を付ける
  - `placement/follow/sticky_follow.rs` の `follow_sticky_move`: 連れ動かした組の相手にも付ける
  - `emo2_boot/frame/drain_resnap.rs` の `finalize_chain_once_with`: 印の付いた窓の既定を `None`（既定配置ではない）として判定へ渡す。x が既定のまま（縦にだけ動いた・ちょうど既定へ置いた）でも引き戻さない
  - 走査（`collect_chain_states`・DPI 遷移の解き直しと共有）と `chain_realign.rs` は変えていない
- 決定論のテスト
  - `frame_chain_finalize_persist_tests.rs` のテスト 11c: `\![move]` で縦にだけ動かした相方が、並べ直しで動かず、記憶にも書かれない
  - `frame_chain_finalize_tests.rs`: ちょうど既定へ置いたスコープと、組の相手の `\![move]` に縦にだけ連れ動いたスコープを引き戻さない、の 2 本
  - 3 本とも直す前に赤を確かめた。直した後、`cargo test -p areka --bin areka` は 3445 passed
- ハーネス: 上の表の 1〜3 回目の欄

## 申し送り（本 spec の範囲外・起票の材料）

- ⅰ `finalize_chain` は並べ直した位置を作業領域の中へ寄せない。2 回目の走りでは、窓が x=−1108 へ置かれて隣のモニタへ渡った。持ち主の `areka-P0-scope-chain-gap` は完了済みなので、`/kiro-discovery` で起票する
- ⅱ ぱすたの窓が作業領域の下へはみ出す（y 864〜2232・作業領域の下端は 1704）。起動の配置は面 0 の大きさで解かれ、554×684 の面へ切り替わったときに、吸い付きの無い窓は左上を保つため
- ⅲ 記憶から戻すときの別々の寄せ（research 1.7 の調べ 5）は確かめられていない。窓あわせが起動のたびに窓を重ね直すので、その振る舞いが隠れる
- ⅳ MCP の `sakurascript` の返事が遅れる。3 回目の走り 1 で 4.5 秒と 10 秒超を記録した。そのあいだ、面 1001・1002 の動く webp の玉が毎秒 10〜48 回 `cache_hit=false` で合成し直され、`loop ticker catch-up` が 01:22:29 に 20 回出ていた。判定 2・5 の赤の原因ではなかった
- ⅴ 並べ終えた時点の保存 `persist_unremembered_char_positions` は位置（x・y）だけを比べる。台本の `\![move]` がちょうど既定の位置へ置いた窓は「動かされていない」と見られ、記憶に書かれる（`areka-P0-char-position-save-on-exit` 要件 1.7 の外れ方）。印 `ScriptPlaced` を読めば外せる
