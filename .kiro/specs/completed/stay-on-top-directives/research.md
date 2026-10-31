# ギャップ分析: stay-on-top-directives

- 実施日: 2026-10-30（main `e29a73c79` の上のワークツリー）
- 入力: `requirements.md`（確定）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`）・完了 spec（`areka-P0-ghost-window-zorder`・`areka-P0-scope-zorder-pinning`・`areka-P0-zorder-scope-inversion`・`areka-P0-window-link-directives`・`areka-P0-shell-balloon-switch`）
- 外の資料: Microsoft Learn「SetWindowPos function」の Remarks（2026-10-30 に読んだ）
- 方針: 分析と選択肢を並べる。決めるのは要件ディスカッションと設計。

## 1. 分析の要約

- **読み取りの穴は小さい**。裸の `\v` は `decode_bare` の既定の腕から `decode_passthrough_bare` へ落ち、`Raw`＋「知らないタグ」の印になる。`\![set,windowstate,…]` は汎用の運び手 `GenericCommand` で最後まで運ばれるが、受け手の表に `(set, windowstate)` の行が無い。`\+` の前例（`decode_bare` の 1 腕）と受け手の表の 1 行の型でそのまま埋まる。
- **窓を帯へ上げ下げする働きは丸ごと無い**。wintf には `ZOrder::TopMost`／`NoTopMost`（`window_pos/mod.rs`）と、帯に居るかを読む `is_window_always_on_top`（`api.rs`）だけがある。今の重なりの判断（`zorder_pair.rs` の `InsertSpec::TopOfNormalBand`）は「帯へ入らない」を型で守っている。常に手前のあいだはこの避け方に条件を付けないと、手前の窓の並べ直しが組と鎖の並びを壊す。
- **台本の重なりの組の寿命（申し送りの件 1）は 1 か所の置き換え**。シェルの切り替えの後始末 `finish_shell` → `reseed_zorder_descript_base` → `apply_descript_rebase` → `ZOrderGroupLedger::set_descript_base` が台本の組まで落としている。ゴーストの読み直し（窓と `Emo2Wiring` を作り直す）では今も捨て、SHIORI だけの読み直し（窓と `Emo2Wiring` を残す）では今も保つ。要件 5.3・5.4 は今の作りで既に満たされている見込み。
- **「先送りの語」を見張る守りが要件 7.5 の言う 2 つより多い**。語の走査 2 本（wintf と areka）のほかに、窓の拡張スタイルを固定するテスト・判断が常に手前を返さないことを固定するテスト・帯に居ても隣接が成り立つことを測るテスト・受け手の表のテストが `windowstate` を「担当なし」の例に使っている。どれも着地で書き直しが要る。
- **既往の裁定との食い違いが 1 件ある**。`doc/COMPAT_ARCHITECTURE.md` §8 の行（`areka-P0-scope-zorder-pinning` の裁量「基底を据えるとき、タグ由来のグループは残さない」）を、要件 5.1 が覆す。食い違いの記録（同 §8 の `window-link-directives` の行）もある。要件 9.3 の改めの範囲に入れる必要がある。

## 2. brief と要件の主張の照合

| 主張（出どころ） | 実物 | 判定 |
|---|---|---|
| 裸の `\v` は腕が無く `Raw` で運ばれ、台本の組み立てで記録つきで捨てられる（要件 Project Description・brief Current State） | `crates/areka-parsers/src/sakura/lexer.rs:198-267` が `\v` を `Token::Bare("v")` にする。`decode.rs:226` の `decode_bare` に `"v"` の腕は無く、`:259` の既定の腕 → `:478` の `decode_passthrough_bare` が `ReadNote::UnknownTag` と `Instruction::Raw("\\v")` を返す。`crates/areka-sakura/src/compile.rs:422-424` の catch-all が `debug!("M-boot 外タグを無視")` を出し、cue を作らない | 正しい（記録は debug の水準） |
| 裸の `\+` を `\![change,ghost,random]` へ写す前例（brief Approach） | `decode.rs:251` `"+" => decode_passthrough_bang(["change","ghost","random"])`。`:253` に `\_+` も同じ形 | 正しい。`\v` は `["set","windowstate","stayontop"]` を同じ口へ渡す 1 腕で済む |
| `\![set,windowstate,…]` は汎用の運び手で届くが、受け手の表に受け手が無い（brief・要件） | `compile.rs:302-326` が `CueCommand::command_carrier` に載せる。`crates/areka/src/emo2_boot/consumer_ledger.rs:380-495` の `canonical()` に `(set, windowstate)` は無い。`:27` の注記が `\![set,windowstate]` を「担当なし」の例に挙げている | 正しい |
| `consumer_ledger.rs` は 960 行（brief Current State・Constraints） | 504 行。登記は 38 件（`consumer_ledger_tests.rs:298` の手書きの数も 38） | 古い（brief の 10-24 の追記が既に正している） |
| `check_script` が「知らない命令」と答える（brief 10-24・要件 1.8） | `crates/areka/src/mcp/check_script_judge.rs:57` は `\v` を `unknown_tag` と答える。`:72-75` は表に無い `\![set,windowstate,…]` を `unknown_command` と答える。strict の記録（`crates/areka/src/emo2_boot/strict_judges.rs:30` が同じ表を引く・`DetectionPoint::CommandUnhandled`）も同じ表に従う | 正しい（`\v` は「知らないタグ」の側）。表に 1 行足すと、`check_script` と strict の記録の両方が黙る |
| wintf の帯を避ける判断は `zorder_pair.rs` の 368 行目・帯の中かを読むのは `zorder_pair_maintain.rs` の 488〜489 行目（brief 10-24） | `zorder_pair.rs:368` `Some(_) if obs.measured_above_of_below_is_always_on_top => InsertSpec::TopOfNormalBand`。`zorder_pair_maintain.rs:487-489` で測る。測る関数は `zorder_pair.rs:599` `measure_window_is_always_on_top`（読めないときは「帯の中」に倒す）。もとの読み取りは `crates/wintf/src/api.rs:129` `is_window_always_on_top` | 正しい |
| 帯へ上げ下げする指令の口は `window_pos/mod.rs` の `zorder_topmost`・`zorder_notopmost`（brief） | `window_pos/mod.rs:131`・`:137`。`ZOrder::TopMost`／`NoTopMost` は `:296-297` で `HWND_TOPMOST`／`HWND_NOTOPMOST` に写る。本番の呼び手は 0（テストだけ） | 正しい。口はあるが使われていない |
| 台本の組はシェルの切り替えで落ちる（brief 10-23 件 1） | `crates/areka/src/emo2_boot/frame/switch.rs:491` `wiring.reseed_zorder_descript_base(…)` → `frame/wiring.rs:248-249` → `frame/zorder_descript.rs:97-101` `apply_descript_rebase` が `ledger.set_descript_base(Vec::new())` を呼ぶ。`crates/areka/src/placement/zorder_group_ledger.rs:535-555` の `set_descript_base` は `self.groups.clear()` で台本の組も消す | 正しい。今の動きを固定するテストに `frame/zorder_descript_tests.rs:507` `t_zdb13_rebase_seats_the_new_shell_value_as_the_only_group` がある |
| 兄弟の決まり（台本の組を保ち、descript の分だけ取り替える）（brief 10-23） | 同じ `finish_shell` の中の `replace_sticky_descript`（`placement/sticky_group_ledger.rs:293`）と `replace_balloon_lock_descript`（`placement/balloon_lock.rs:56`）がその形 | 正しい。前例はすぐ隣にある |
| ゴーストの読み直しで台本の組を捨てる・SHIORI だけの読み直しで保つ（要件 5.3・5.4） | ゴーストの読み直しは `ghost_switch.rs` の `switch_to` で `take_down` → `close_windows_for_restart`（`app_exit.rs:197-214`）→ 起こし直しで `Emo2Wiring` を作り直す（`emo2_boot/mod.rs:622-639`）。台帳は `Emo2Wiring` の欄（`frame/wiring.rs:201`）なので作り直しで空になる。SHIORI だけの読み直しは `wants_runtime_only`（`reload.rs:493`）→ `rebuild_runtime_in_place`（`ghost_session_runtime.rs:89`）で、窓と `Emo2Wiring` を残す | 今の作りで満たされている見込み。テストで固定されているかは設計の段で確かめる |
| 網羅台帳 `sakura-script.toml` の 3 行は absent・持ち主なし（brief Current State） | `!stayontop`（3378 行）は `absent`・**`owner = "stay-on-top-directives"`**（10-23 に書いた）。`stayontop`（3412 行）と `\v`（5546 行・id `ukadoc:list_sakura_script:_5cv:1`）は `owner = ""`。`minimize`（3395 行）は `owner = ""`・C3 | 1 行は持ち主が既に入っている。`\v` の行は 3378・3412 の近くではなく 5546 行目 |
| `shiori.toml` の `alwaysstayontopbutton.caption` は語の表だけ（brief） | 12075 行・`status = "vocabulary-only"`・`owner = ""` | 正しい |
| 窓の作りは常に手前を含めない（完了 `ghost-window-zorder` 要件 8.1） | `crates/areka/src/placement/spawn.rs:619-625` の `WindowStyle` の拡張スタイルは `WS_EX_LAYERED`・`WS_EX_TOOLWINDOW`・`WS_EX_ACCEPTFILES` の 3 つだけ | 正しい |
| 守りは 2 つ（要件 7.5） | 下の 4 節。語の走査は 2 本だが、窓と判断の側の守りがほかに 3 か所、受け手の表のテストが 2 か所ある | **足りない**（要件 7.5 の数え方を広げるか、設計で残りを拾う） |
| 守りの説明「`crates/wintf/src/api.rs:126`」（`zorder_pair_deferred_vocabulary_tests.rs:36`） | 今は 131 行目（関数は 129 行目） | 注記の行番号が古い。着地のときに一緒に直す |

## 3. 今ある資産

### 3.1 台本の読み手と運び手

- `crates/areka-parsers/src/sakura/decode.rs`（502 行）: `decode_bare`（226 行目）に腕を 1 つ足す場所。写した先の命令は `GenericCommand { name: "set", raw_args: ["windowstate","stayontop"] }`。`script-small-tags` もこのファイルを触る（直列）。
- `crates/areka-sakura/src/compile.rs`: `GenericCommand` は 0 秒の合図として `offset` の位置に載る。中断で台本がそこへ来なければ合図は配られない（要件 1.4 は運び手の性質で満たされる見込み）。

### 3.2 受け手の表と受け手

- `crates/areka/src/emo2_boot/consumer_ledger.rs`（504 行）: 「名前＋第 1 引数」の登記。`("set", Some("windowstate"))` は `set` の他の行（`zorder`・`choicetimeout`・`balloontimeout`・`alignmenttodesktop` ほか）と同じ「選別子つき」で足せる。新しい担当の識別子（`CommandConsumer` の variant）を足すか、既存の担当に載せるかは設計の判断。
- `crates/areka/src/emo2_boot/runtime_boot.rs`（232 行）: `runtime_sinks` が 12 本の受け口を並べる。5 本目より後ろは名前で自己選別するので、末尾に足してよいと注記がある。UI へ送る線（`Sender`）は `CueSinkSources`（窓の側が 1 度作る）か `RuntimeLines`（実行系ごと）に置く。
- `crates/areka/src/emo2_boot/zorder_cue.rs`（162 行）と `frame/zorder_drain.rs`（460 行）: 重なりの指令の線（`ZOrderDirective`）と UI の取り出しの相。`ZOrderCueSink` は `(set|reset, zorder)` だけを拾う（`zorder_wiring_tests.rs:143` が `(set, windowstate, minimize)` を拾わないことを固定）。**この 2 ファイルは areka 側の語の走査の対象**（`spawn_zorder_pair_deferred_tests.rs` の `PRODUCTION_FILES`）なので、ここへ `windowstate`・`stayontop`・`topmost` を書くと守りが赤くなる。

### 3.3 wintf の重なりの機構

- `window_pos/mod.rs`（454 行）: `ZOrder::{TopMost, NoTopMost, Top, Bottom, InsertAfter}`。`SetWindowPosCommand` の指令の口。
- `zorder_pair.rs`（892 行）・`zorder_pair_maintain.rs`（557 行）・`zorder_pair_establish.rs`・`zorder_pair_sink.rs`: バルーンとキャラの組の確立（`Added<WindowHandle>` 駆動）・維持（`ReassertZOrder`）・沈降の観測（`WM_ACTIVATE` の非活性化の次の巡）。判断 `decide_pair_fix` は Win32 を呼ばない純関数で、観測 `PairObservation` に帯の所属を載せて受け取る。
- `zorder_chain.rs`（631 行）・`zorder_chain_apply.rs`（930 行・上限まで 70 行）・`zorder_chain_diag.rs`: スコープをまたぐ所有の鎖（`GWLP_HWNDPARENT`）。1 巡の順は「去る窓の切離し → 鎖の撤去 → 組の外し → 組の結び直し → 鎖の付与 → 後押し（`ZOrder::InsertAfter`・`zorder_chain.rs:467`）→ 直後の実測」。組の結び目を外す印 `PairReleased`。
- `crates/wintf/src/runtime/window_factory.rs:64-67` `compute_ex_style`: `WindowStyle.ex_style` をそのまま `CreateWindowExW` へ渡す（`WS_EX_LAYERED` だけ落とす）。作る時点で `WS_EX_TOPMOST` を立てれば、作った瞬間から帯に居る窓になる。
- `crates/wintf/src/win_style.rs:362` `apply_click_through`: `WS_EX_TRANSPARENT` のビットだけを読み書きする（`apply_click_through_toggles_only_transparent_bit` で固定）。クリックの通り抜けの切り替えが帯の印を剥がす心配は小さい（要件 2.6）。

### 3.4 窓の束と寿命

- `crates/areka/src/placement/spawn.rs`（810 行）`spawn_ghost_windows`: 全スコープのキャラ窓とバルーン窓を起動の時に 1 度だけ作る。呼び手は `ghost_session.rs:289` だけ。HWND は後の巡で付く（`Added<WindowHandle>`）。
- シェルの切り替え（`finish_shell`）・バルーンの切り替えは窓を作り直さない（`switch.rs` に spawn／despawn は無い）。ゴーストの切り替え・読み直しは窓を全部閉じて作り直す（`close_windows_for_restart`）。
- 今の areka は 1 プロセス 1 ゴースト（`multi-ghost-runtime` は余力の枠で未着手）。要件 2.7（複数ゴースト）は今の作りでは自明に満たされる。状態の置き場をゴースト単位にしておくことが要件の実質。

### 3.5 網羅台帳と互換の記録

- `doc/ukadoc-coverage/ledger/sakura-script.toml`: 状態の語は 7 つ（`doc/ukadoc-coverage/README.md:146-154`）。`implemented` は**ソース側に正典 URL の証拠の 1 行コメントが要る**（検査が集める）。3 行とも `implemented` にするなら、受け手と読み手に `\v`・`stayontop`・`!stayontop` の ukadoc の URL を書く。
- `doc/ukadoc-coverage/ledger/shiori.toml`: 「対応しない」を表す専用の語は無い。`vocabulary-only`（語彙のみ）のまま note に理由を書くか、`not-applicable`（areka の作りでは意味を持たない）にするか（下の判断 7）。
- `doc/COMPAT_ARCHITECTURE.md` §8（124 行目から）: 226 行目（基底を据えるときタグ由来の組を残さない）・151 行目（`window-link-directives` の寿命の行。「兄弟の `\![set,zorder]` の台本の組はシェルの切り替えで捨てられており…食い違う」）・332 行目（シェルの差し替えの行。`apply_descript_rebase` を定義点に挙げる）が本 spec の着地で変わる。

## 4. 着地で書き直しが要る守り（要件 7.5 の範囲）

| # | 守り | 何を固定しているか | 本 spec で崩れる理由 |
|---|---|---|---|
| G1 | `crates/wintf/src/ecs/window/zorder_pair_deferred_vocabulary_tests.rs` | wintf の `zorder_` 系の本番 8 ファイルのコードに `topmost`・`stayontop`・`windowstate`・`minimize` ほか 9 語が無い。`zorder_` で始まる本番ファイルが名簿から漏れていない。`zorder_pair.rs` のコードに `topmost` が無い（対照） | 帯の避け方に条件を付けるなら `zorder_pair.rs`／`zorder_pair_maintain.rs` のコードに語が入る。新しいファイルを `zorder_` で始めれば名簿の検査にもかかる |
| G2 | `crates/areka/src/placement/spawn_zorder_pair_deferred_tests.rs` | areka 側の 7 ファイル（`spawn.rs`・`diag.rs`・`zorder_chain_compose.rs`・`zorder_group_ledger.rs`・`emo2_boot/zorder_cue.rs`・`frame/zorder_drain.rs`・`frame/zorder_descript.rs`）に同じ 9 語が無い。窓の既定スタイルに `WS_EX_TOPMOST` が無い | 件 1 で `zorder_descript.rs`・`zorder_group_ledger.rs` を触る。受け手や後から現れる窓の処理を `spawn.rs` に置けば語が入る |
| G3 | `crates/areka/src/placement/spawn_assembly_tests.rs:412` `t_i2_no_window_has_ws_ex_topmost` と G2 の `ghost_windows_carry_no_always_on_top_minimize_or_taskbar_bits` | 作った窓の `style`・`ex_style` の完全一致 | 既定（オフ）の作りは変わらないので、オフの窓の主張としては残せる。オンのときに作る窓へ `WS_EX_TOPMOST` を足す道を選ぶなら、その分岐のテストを足す |
| G4 | `crates/wintf/src/ecs/window/zorder_pair_tests.rs` の `decide_pair_fix_covers_every_arm_without_ever_yielding_always_on_top` | 組の判断が常に手前を返さない | 判断に「常に手前のあいだ」の入力を足しても、返り値に `TopMost` を足さない形なら残せる。足すなら書き直し |
| G5 | `crates/wintf/src/ecs/window/zorder_pair_maintain_always_on_top_tests.rs` | 実際の窓で、確立系と維持系の後に帯の印が付かない。帯に居る窓どうしでも隣接が成り立つ（テストの中だけで帯を作る） | 「本番の経路へ常時最前面の指定を持ち込まない」と注記している。オフの主張は残り、オンの主張（帯に居ても隣接）が本番の経路の主張に変わる |
| G6 | `crates/areka/src/emo2_boot/consumer_ledger_tests.rs:94`・`:138-148` | `(set, windowstate)` を「担当なし」の例・「後から別の担当を足せる」の例に使う | 表に行を足すと `:94` は担当ありになり、`:138` の `try_register` は `Duplicate` で失敗する。例の選別子を別の語（表に無い語）へ替える |
| G7 | `consumer_ledger_tests.rs:298` の手書きの数 38 と `consumer_ledger.rs:41-48` の一覧 | 登記の件数 | 39 へ。並走の spec（`script-small-tags`・`shell-scale-linkage`・`balloon-communicatebox`）も同じ数を書き換えるので 1 本ずつ |
| G8 | `consumer_ledger_agreement_tests.rs`（406 行）の見本の一覧と受け口の名簿 | 表の行と受け口の自己選別の一致（10 の受け口） | 行を足すと見本が要る。受け口を新しく作れば名簿に足す |

## 5. 要件と資産の対応

| 要件 | 使える資産 | 穴 | 区分 |
|---|---|---|---|
| 1.1 `\v` | `decode_bare`・`decode_passthrough_bang` | `"v"` の腕 | Missing |
| 1.2・1.3 | 汎用の運び手・受け手の表 | `(set, windowstate)` の行と受け手 | Missing |
| 1.4 中断 | 0 秒の合図の配り方 | なし（運び手の性質） | — |
| 1.5〜1.7 知らない第 2 引数・余り・綴り | `ZOrderCueSink` の小文字ちょうどの選別の流儀 | 受け手の中の分岐と記録 | Missing |
| 1.8 `check_script` | `check_script_judge.rs`・表 | 表に 1 行。ただし第 2 引数の誤り（`minimize`・綴り違い）は表を引くだけでは診ない（下の判断 6） | Missing／Constraint |
| 2.1 オフ→オンで今在る全部の窓を上げる | `ZOrder::TopMost`・`SetWindowPosCommand`・`GhostWindows` | 上げる処理・上げる順・失敗の記録 | Missing／Unknown（6 節 R1〜R3） |
| 2.2 他のアプリが活性になっても奥へ回さない | OS の性質（帯の窓は非活性化でも帯に残る） | 沈降の観測（`zorder_pair_sink.rs`）が「前面の窓より奥に居ない」を失敗として記録しないか | Constraint |
| 2.3 新しく現れる窓 | `Added<WindowHandle>` 駆動の確立系・`compute_ex_style` | 作る前（`WindowStyle`）か作った直後か、どちらで帯へ入れるか。今の areka で後から現れるのは HWND の付くのが遅れる窓だけ（作り直しはゴーストの読み直しで、そのときはオフへ戻る） | Missing |
| 2.4・2.5 並びを保つ | 所有の鎖・組の結び目・`PairReleased` | 帯を避ける判断（`TopOfNormalBand`）がオンのあいだ並びを壊す。帯の上げ方の順 | Constraint／Unknown |
| 2.6 透過ほか | `apply_click_through` がビット単位 | なし見込み | — |
| 2.7 複数ゴースト | 1 プロセス 1 ゴースト | 状態の置き場をゴーストの単位に | Constraint |
| 2.8 重ねのオン＝出し直し | — | 出し直しの手順（並びを崩さずに帯の最上へ） | Missing／Unknown |
| 3.1 オン→オフ | `ZOrder::NoTopMost` | 外す処理。OS の性質で所有の木ごと外れる（6 節 R2） | Missing |
| 3.2 重ねのオフは動かさない | — | 状態の比べ | Missing |
| 3.3 オフのあいだは今のまま | 今の判断全部 | 条件付けで今の判断を変えないこと（G4・G5 で守る） | Constraint |
| 3.4・4.1・4.2 起動・終了・読み直しでオフ | 窓と `Emo2Wiring` の作り直し | 状態の置き場しだいで自動（下の判断 2） | — |
| 4.3 SHIORI だけの読み直しで保つ | 窓と `Emo2Wiring` が残る | 同上 | — |
| 4.4・4.5 シェル・バルーンの切り替えで保つ | 窓を作り直さない | 帯の印は窓に残る。描き直しの経路が `SetWindowPos` の `InsertAfter` で通常の窓の後ろへ置くと帯から落ちる（6 節 R2） | Constraint |
| 5.1・5.2 シェルの切り替えで台本の組を保つ | `replace_sticky_descript` の前例 | 台帳に「descript の基底だけを取り替え、重なる descript の組は作らず記録する」口 | Missing |
| 5.3・5.4 | 今の作り | テストで固定されているかの確認 | — |
| 5.5 切り替えの後の `\![reset,zorder]` | `reset_to_descript`（`has_base` で先頭を残す） | 取り替えた基底を先頭に置けば今の口で動く | — |
| 6.1〜6.5 記録 | `logging.md` の水準・`log_group_applied`／`log_group_rejected` の出口 | 新しい記録の語彙。出口を wintf の 1 本に集める流儀を守るか | Missing |
| 7.1〜7.4 決定論のテスト | 純関数＋観測の型（`decide_pair_fix`・台帳の状態の檻） | 新しい判断の純関数 | Missing |
| 7.5 守りの書き直し | 4 節 | 数え方 | Constraint |
| 8.1〜8.4 実機 | 排他テストの机・`doc/pasta-real-run.md` 5 節 | 手順の追記 | Missing |
| 9.1〜9.3 | 台帳・§8 | `implemented` の証拠のコメント・§8 の 3 行 | Missing |

## 6. 調べることと外の事実

### 6.1 外の事実（Microsoft Learn「SetWindowPos」Remarks・2026-10-30）

- 普通の窓を帯へ上げると、**それが所有する窓も帯へ上がる**。所有者は変わらない。
- 帯の窓を帯から外すと、**所有者と所有される窓も帯から外れる**。
- 帯の窓を普通の窓の後ろ（`InsertAfter` に普通の窓・`HWND_BOTTOM`）へ置くと、帯から落ちる。
- 普通の窓は帯の窓を所有できるが、逆はできない。帯の窓が所有する窓は帯へ入る（所有される窓は所有者より手前に居る決まりを保つため）。

areka の鎖は所有の関係（`GWLP_HWNDPARENT`）で組み、組は案 A（`OwnerLink`）でバルーンをキャラの所有にする。よって「鎖の根だけを上げれば、鎖でつながった窓は一緒に上がる」「どれか 1 枚を外せば、所有の木ごと外れる」が文書の上では言える。ただし `PairReleased`（結び目を外した組）と、鎖に入っていないスコープ（グループの無いスコープ）の窓は根が別なので、それぞれ上げる必要がある。

### 6.2 Research Needed（設計の段へ持ち越す）

- **R1 所有の木と帯（brief 議題 2）**: 上の文書の性質が areka の鎖と組で実際に成り立つか。鎖の 1 巡の途中（所有を付け替えた窓）で帯の印がどう動くか。実機で測る（排他テストの机・画面のロックを外す）。
- **R2 帯から落とす既存の指令**: 帯に居る窓に、普通の窓を `InsertAfter` に指す指令が出ると帯から落ちる。今の経路でそれを出しうる所 ― 組の是正 `InsertSpec::After(front_of_below)`（キャラの手前の可視の窓が普通の窓なら落ちる）・鎖の後押し `ZOrder::InsertAfter`（`zorder_chain.rs:467`）・`TopOfNormalBand` の `ZOrder::Top`（帯の窓に `HWND_TOP` は帯の最上へ上がる＝組の並びより手前へ飛ぶ）。オン・オフの切り替えの途中（一部だけ上がった状態）も含めて洗い出す。
- **R3 出し直し（要件 2.8）の手順**: 帯の中で並びを崩さずに一式を帯の最上へ出す順（奥の根から手前へ `HWND_TOPMOST` を順に当てるか、所有の根だけか）。`SWP_NOACTIVATE`・`SWP_NOOWNERZORDER` の要否。
- **R4 沈降の観測の偽の失敗**: 常に手前のあいだ、他のアプリが活性になっても沈まないのは正しい動き。`zorder_pair_sink.rs` の観測がこれを失敗（warn）として残さないか、記録の欄で区別できるか。実機のサインオフの grep の判定（`signoff-scan.ps1` ほか）に効くか。
- **R5 SSP の振る舞い（brief 議題 1・要件の段の裁定 1・3・4・5 の「SSP で未測」）**: `\1\v` の効く範囲・バルーンも上がるか・`!stayontop` の後・ゴーストの切り替え／読み直し／シェルの切り替えの後に常に手前と `\![set,zorder]` の組が残るか。開発者の機械の SSP で測る（SSP の環境を変える許可を取る）。
- **R6 ぱすたの辞書の原文**: `dic/tategaki.pasta` の「重ね順」の台本は brief の引用だけで、この分析では原文を読んでいない（外のゴースト）。実機の確かめ（要件 8.3）の前に原文を確かめる。

### 6.3 既往の裁定・完了 spec との関係

- **食い違い（要件 5.1 が覆す）**: `doc/COMPAT_ARCHITECTURE.md` §8 の 226 行目（`areka-P0-scope-zorder-pinning` 要件 5.1・8.3 の裁量）は「shell 設定由来の基底を据えるとき、既にタグ由来のグループが載っていたら、タグ由来のグループを残さず `\![reset,zorder]` の後と同じ終状態にする」と記録している。根拠は「正典の経路ではこの状況は起きない（設定の適用は起動の段だけ）」。その後 `areka-P0-shell-balloon-switch`（要件 2.7）がシェルの切り替えでこの口を呼ぶようにしたため、前提が崩れて「台本の組が落ちる」になった。要件 5.1・5.2 はこの裁量を「シェルの切り替えでは台本の組を保ち、重なる descript の組は作らずに記録する」へ改める。**要件 9.3 の書き方は「寿命の裁定を書く」で、226 行目の上書きを名指ししていない**。`areka-P0-window-link-directives` の 151 行目の「食い違う」の文と、332 行目の定義点の記述も同じく改める。
- **一致**: 完了 `areka-P0-ghost-window-zorder` 要件 8.2 は「ゴースト一式を最前面へ上げる指定（`\v` 相当）を後から追加できる構造」と書く。要件の段の裁定 1（`\1\v` でもゴーストの窓の全部）は「一式」の字面と合う。要件 4.3・8.1 の読み替えは requirements の Introduction のとおりで、完了 spec は直さない。
- **一致**: 完了 `areka-P0-window-link-directives` の裁定 1（読み直しで捨て、SHIORI だけの読み直しで保つ）と裁定 2（シェルの切り替えで台本の分を保ち descript の分だけ取り替える）に、要件の段の裁定 3・4・10 が揃っている。今のコードの寿命（3.4 節）もこれと合う。
- **注意**: 完了 `areka-P0-zorder-scope-inversion` が決めた鎖の 1 巡の順と `PairReleased` を前提にすること（brief 10-19 の申し送り）。「鎖の根だけを帯へ上げる」案は、結び目を外した組の窓が付いてこない場合がある。

## 7. 実装の選択肢

### 案 A: 今の重なりの機構へ足す（拡張）

- 読み手: `decode_bare` に `"v"` の腕。
- 受け手: `ZOrderCueSink`（`zorder_cue.rs`）に `(set, windowstate)` を拾わせ、`ZOrderDirective` に「常に手前のオン／オフ」の値を足し、`frame/zorder_drain.rs` の取り出しで処理する。状態は `Emo2Wiring`（台帳の隣）に持つ。
- wintf: 鎖の適用系の 1 巡（`zorder_chain_apply.rs`）か組の維持系（`zorder_pair_maintain.rs`）に「帯へ上げる・外す」を足し、`decide_pair_fix` の帯の避け方に条件を付ける。
- 件 1: `set_descript_base` の中身を「台本の組を残す」へ変える。
- 利点: 新しいファイルが少ない。重なりの線・取り出しの相・記録の出口を全部流用できる。
- 欠点: `zorder_chain_apply.rs` は 930 行で余地が 70 行（先に外へ出す必要）。`zorder_cue.rs`・`zorder_drain.rs`・`zorder_pair*.rs` はどれも G1・G2 の走査の対象で、守りの名簿と語の扱いを大きく書き直すことになる。`set_descript_base` は起動の段（`apply_descript_base`）とも共有なので、中身を変えると起動の意味まで動く（226 行目の裁量の根拠と別の話が混ざる）。

### 案 B: 新しい部品として足す（新設）

- 受け手: `emo2_boot/` の下に新しいファイル（例: `window_state_cue.rs`）と自分の線を置き、受け手の表に新しい担当（例: `CommandConsumer::WindowStateSink`）を 1 行。`tasktray-minimize` が `minimize` の腕を同じ受け手に足せる形にする。
- 状態と適用: UI の側に新しい相（例: `frame/window_state_*.rs`）と、wintf に新しいファイル（例: `ecs/window/band_*.rs`。`zorder_` で始めなければ G1 の名簿の検査に入らない）。帯へ上げ下げする指令はここだけが出す。帯の状態は窓の entity の印（wintf の component）か `Emo2Wiring` の欄で持つ。
- 件 1: 台帳に新しい口（例: 「基底だけを取り替え、台本の組と重なる要素は作らずに返す」）を足し、`apply_descript_rebase` だけがそれを呼ぶ。起動の `apply_descript_base` は今の口のまま。
- 既存の判断への条件付け: `decide_pair_fix` の観測に「常に手前のあいだ」を 1 欄足し、`TopOfNormalBand` を選ぶ条件にだけ使う（返り値に `TopMost` は足さない＝G4 を残せる）。
- 利点: 責務の切れ目がはっきりし、テストを分けやすい。上限に近いファイル（`zorder_chain_apply.rs`・`zorder_pair.rs`）への追記が少ない。`minimize` の後続を塞がない。
- 欠点: 新しい線・新しい相・新しい担当の分、結線（`runtime_boot.rs`・`emo2_boot/mod.rs` の `mod` 行・`frame` のスケジュール）が増える。鎖の 1 巡と別の相で帯を動かすので、同じ巡の中の順（鎖の付与の後に上げるか）を決める必要がある（R1〜R3）。

### 案 C: 組み合わせ（推奨の候補）

- 読み手の腕と受け手の表の 1 行・新しい受け手のファイル・新しい担当は案 B。
- 帯の上げ下げの働きは wintf の新しいファイルに置き、鎖の適用系の 1 巡の**最後**（後押しと直後の実測の後）か直後の相から呼ぶ。既存の判断へは「常に手前のあいだ」の 1 欄だけを足す（案 A の最小の部分）。
- 後から現れる窓（要件 2.3）は、作る前の `WindowStyle` に `WS_EX_TOPMOST` を足す道（`compute_ex_style` がそのまま渡す）と、`Added<WindowHandle>` の巡で `HWND_TOPMOST` を当てる道のどちらか（下の判断 4）。
- 件 1 は案 B の新しい口。
- 守りは G1・G2 の語の表を「本 spec の受け持ちのファイルでは許す」形へ改め、`minimize`・`iconic`・`onwindowstate`・`onfullscreenapp`・`appwindow`・`taskbar` の 6 語（`tasktray-minimize` と全画面の先送り）は残す。

## 8. 規模と危うさ

- **規模: M〜L（brief の見立て 8〜12 タスク・`zorder_chain_apply.rs` を先に外へ出すなら +1）**。読み手・受け手・台帳の 1 行・件 1 は既存の型で S 程度。重さは wintf の帯の働きと既存の判断への条件付け、守り 8 か所の書き直し、実機の確かめに在る。
- **危うさ: Medium〜High**。OS の帯と所有の木の性質（R1〜R3）は文書で読めるが、areka の鎖と組の巡の中での振る舞いは実機でしか確かめられない。帯から黙って落とす指令（R2）は見つけにくい。一方で、今の判断の形（純関数＋観測）と前例（`replace_sticky_descript`・`\+` の腕）がそろっており、読み手・受け手・寿命の部分は Low。

## 9. 設計の段への申し送り

- 推奨の候補は案 C。最初に R1〜R3 を排他テストの机で測り、「根だけ上げる」か「窓ごとに上げる」かを決めてから、帯の上げ下げの置き場（鎖の 1 巡の中か外か）を決める。
- 件 1 は起動の口（`apply_descript_base`）と切り替えの口（`apply_descript_rebase`）を分け、`t_zdb13` を書き直す。§8 の 226・151・332 行目を一緒に改める。
- `zorder_chain_apply.rs`（930 行）と `zorder_pair.rs`（892 行）に足すなら、先に外へ出すタスクを置く。
- 受け手の表の手書きの数（38→39）は並走の 3 本と 1 本ずつ。`decode.rs` は `script-small-tags` と直列。

## 10. 要件ディスカッションへ出す判断の項目

1. **要件 7.5 の「2 つの守り」の数え方**: 実際に書き直しが要る守りは 4 節の G1〜G8（語の走査 2 本・窓の既定スタイル・判断が常に手前を返さない・帯に居ても隣接・受け手の表のテスト 2 か所・見本の一致）。要件 7.5 を「先送りの語と常に手前の不在を見張る守りの全部」へ広げるか、2 本のままにして残りを設計の段で拾うか。
2. **常に手前の状態の置き場**: (a) `Emo2Wiring` の欄（ゴーストの切り替え・読み直しで作り直され、SHIORI だけの読み直しとシェルの切り替えで残る＝要件 4 の寿命が置き場だけで決まる）、(b) World の資源にして `close_windows_for_restart` で外す（`StickyGroupLedger` の前例）、(c) 窓の entity の印（wintf の component・窓の作り直しで自然に消え、後から付く HWND にも当てやすい）。
3. **受け手の担当**: 新しい担当（例: `WindowStateSink`）と新しい受け口・新しい線を作るか、重なりの受け口 `ZOrderCueSink` に `(set, windowstate)` を拾わせるか。後者は G2 の走査の対象のファイルに語が入る。`tasktray-minimize` が `minimize` の腕を足す場所としてはどちらも成り立つ。
4. **新しく現れる窓（要件 2.3）の入れ方**: (a) 作る前の `WindowStyle.ex_style` に `WS_EX_TOPMOST` を立てる（作った瞬間から帯に居る・`compute_ex_style` がそのまま渡す）、(b) `Added<WindowHandle>` の巡で `HWND_TOPMOST` を当てる（作った直後の 1 巡は普通の窓）。(b) は要件 2.3 の「後ろに出る瞬間を作らない」と、窓が見える前に当てられるか次第。今の areka では後から作られる窓はゴーストの読み直しにしか無く、そのときは状態がオフへ戻る＝実際に効くのは HWND が遅れて付く窓（起動の直後の `\v`）。
5. **帯の上げ方（要件 2.1・2.8・3.1）**: OS の所有の木の性質（6.1 節）に頼って鎖と組の根だけを上げ下げするか、ゴーストの窓を 1 枚ずつ奥から順に上げるか。R1 の実測の結果で決めてよいか。`PairReleased` の組と鎖に入らないスコープの扱いも含む。
6. **`check_script` と strict の記録の第 2 引数**: 表に `(set, windowstate)` を足すと、`\![set,windowstate,minimize]` や `\![set,windowstate,StayOnTop]`（要件 1.7 で受け付けない綴り）も `check_script` と strict の記録が黙る。受け手は記録を残す（要件 1.5・1.7）が、作者には見えない。`check_script` に第 2 引数の判定（`unreadable_argument` か `ignored`）を足すか、`check-script-arg-checks` に申し送るか。
7. **`alwaysstayontopbutton.caption` の台帳の状態（要件 9.2）**: 状態の語に「対応しない」は無い。`vocabulary-only` のまま note に理由を書くか、`not-applicable` にするか（README の定義は「areka の作りでは意味を持たない項目」で、意味はあるが作らない、とは少し違う）。
8. **§8 の既往の裁量の上書き（6.3 節）**: `doc/COMPAT_ARCHITECTURE.md` §8 の 226 行目（`areka-P0-scope-zorder-pinning` の「基底を据えるときタグ由来の組を残さない」）を本 spec が上書きすることを、要件 9.3 に名指しで書くか。起動の段の扱い（起動では台本の組が無いので今のまま）と、シェルの切り替えの扱いを分けて記録するか。
9. **沈降の観測（R4）**: 常に手前のあいだに他のアプリが活性になっても沈まないのは正しい動き。今の沈降の観測がこれを失敗として残すなら、オンのあいだは観測を止めるか、記録に常に手前の欄を足して区別するか。要件 6 に記録の決まりとして足すか、設計の段の判断に任せるか。
10. **SSP の測り（R5）の扱い**: 要件の段の裁定 1・3・4・5 は「SSP で未測」。要件を確定する前に開発者の機械の SSP で測るか、推奨の答えで確定して実機の確かめ（要件 8）の段で SSP と見比べるか。測るなら SSP の環境を変える許可が要る。
11. **件 1 の口の分け方**: 台帳の `set_descript_base` の中身を変える（起動と切り替えの両方に効く）か、切り替え専用の新しい口を足して起動の口は今のままにするか。後者なら 226 行目の裁量は起動については生きたまま残る。

## 11. SSP の測り（2026-10-30・要件ディスカッションの前）

- 道具: SSP 2.9.26（`baseware.version` は `SSP/2.9.26 (20261029-1; Windows NT 10.0.26300)`）を `C:\wintools\ssp\ssp.exe` から起こし（プロセスの中だけ `PASTA_DEBUG=0`）、SSP の MCP の `sakurascript`（strict）で台本を送り、`EnumWindows`／`GetWindowLong(GWL_EXSTYLE)` で SSP の窓の重なりの順（手前から）と `WS_EX_TOPMOST` を読んだ。比べる相手の普通の窓は Claude Desktop の窓（`SetWindowPos(HWND_TOP, SWP_NOACTIVATE)` で手前へ出してから送った）。reload・設定変更・ゴーストとシェルの切り替えはしていない。測った後は `\![set,windowstate,!stayontop]` で元の重なりへ戻した。strict の誤りの記録は 0 件。
- 対象: 「窓際のぱすたさんDEBUG」（起動の台本の `!stayontop` が効いて、測る前は全部の窓が帯の外）。窓は本体側「ぱすた」・相方側「窓」・本体側のバルーン。所有の鎖は 窓 ← バルーン ← ぱすた（`\![set,zorder,s0,b0,s1]` を SSP は所有で組んでいる）。

| # | 送った台本 | 見えたこと |
|---|---|---|
| M1 | `\![set,windowstate,stayontop]` | ゴーストの窓の全部（両スコープのキャラクターの窓・SSP の管理の窓・ゴーストの補助の窓）に即座に `WS_EX_TOPMOST` が付いた。後から出た本体側のバルーンも `WS_EX_TOPMOST` で現れた |
| M2 | （M1 のあいだ） | 重なりの組の並びが**逆になった**: 手前から 窓（s1）→ バルーン（b0）→ ぱすた（s0）。帯の外では ぱすた → バルーン → 窓 |
| M3 | `\![set,windowstate,!stayontop]` | 全部の窓から `WS_EX_TOPMOST` が外れ、並びは ぱすた → バルーン → 窓 に戻った |
| M4 | `\0\v`・`\1\v`（Claude Desktop を手前に出してから） | `WS_EX_TOPMOST` は**付かない**。ゴーストの窓の全部（`\1\v` でも本体側を含む）が、普通の窓の最上（Claude Desktop の手前）へ 1 回だけ上がった。組の並び（ぱすた → バルーン → 窓）は保たれた |
| M5 | `\0\v` の後に Claude Desktop を再び手前へ | ゴーストは 4 秒後も Claude Desktop の後ろのまま＝`\v` の効きは続かない（1 回だけの「手前へ出す」） |

- 読み: SSP 2.9.26 の `\v` は、ukadoc の字面（「常に最前面に表示されるようになる」「ゴースト終了まで有効」）と違い、**ゴーストの窓の全部を、組の並びを保って普通の窓の最上へ 1 回出す**働き。ぱすたの辞書の `\1\v`（重なりを指定した直後に奥の窓を並べ直す狙い）はこの働きを当てにしている。`stayontop` は字面どおり常に手前で、後から出る窓にも効くが、組の並びを逆にする（SSP の癖）。
- 測っていないもの: ゴーストの読み直し・シェルの切り替えの後の常に手前の状態と台本の組（reload は 1 回ごとの許可が要る）・既にオンのときのオンの重ね。
- 他のゴースト（同時に起きていた「悪役令嬢クローディア」）の窓は、ぱすたへの指令で変わらなかった（要件の段の裁定 12 と合う）。

## 12. 設計の段（2026-10-30・`/kiro-spec-design -y`）

- 調べ方: 軽い調べ（既存の作りへの拡張）。wintf の重なりの機構は調べ係 1 本（読むだけ）に任せ、areka の受け口・取り出しの相・台帳・寿命は手元で読んだ。外の資料は 6.1 節（Microsoft Learn「SetWindowPos」）のまま。新しい外部クレートは無い。
- 使ったスキル・指針: `kiro-spec-design` の規則（design-principles・discovery-light・synthesis・review-gate）。steering は `product.md`・`tech.md`・`structure.md`・`logging.md`。

### 12.1 調べて分かったこと（設計を形づくった事実）

- **wintf の本番の重なりの書き込みは 2 か所だけ**。組の是正（`zorder_pair_maintain.rs:189-217` が作り `:524` で予約の列へ積む。`After(h)`→`InsertAfter(h)`、`TopEdge`・`TopOfNormalBand`→`HWND_TOP`。旗は `SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE`。動くのは `ReassertZOrder` の付いた組だけで、付けるのは `establish_owner_links`＝窓に HWND が付いた直後の 1 回）と、鎖の後押し（`zorder_chain.rs:451-482`・予約の列を通さず `guarded_set_window_pos` で直に書く・基準は鎖の自分の窓）。`WindowPos.zorder` は既定の `NoChange` で、areka は変えない。窓を作る `SetWindowPos` は `SWP_NOZORDER`。
- **予約の列は 1 巡の終わりに流れる**（`tick_bridge.rs:261` ほか。`DeferWindowPos` でまとめる）。鎖の後押しだけが巡の中で直に書く。
- **帯の窓どうしで組の是正が帯を壊す道は 1 つ**。キャラクターが帯に居ると、その手前の窓は必ず帯の窓なので、今の判断は `TopOfNormalBand`（`HWND_TOP`）を選ぶ。帯の中のバルーンに `HWND_TOP` を当てると帯の最上（他のアプリの帯の窓より手前）へ飛び、次の巡の検証（バルーンのすぐ奥がキャラクターか）が `error!` になる。キャラクターも帯に居るときは `After(手前の窓)` で帯の中のすぐ手前へ置ける（`zorder_pair_maintain_always_on_top_tests.rs:409` が帯の中でこの形が帯を保つことを実際の窓で示している）。
- **鎖の後押しの基準は鎖の自分の窓**。鎖の窓が全部同じ帯に居れば、後押しは帯を変えない。
- **沈降の観測は `debug!` の 1 行だけで、失敗として判定する所は無い**（`zorder_pair_diag.rs:219-231`）。帯のあいだは `behind_foreground=false` が正しい答えだが、読み手には区別が付かない。
- **窓は起動のときに全部作られる**（`spawn_ghost_windows`）。作る時点で `WindowStyle.ex_style` がそのまま `CreateWindowExW` へ渡る（`compute_ex_style` は `WS_EX_TOPMOST` を残す）。窓の作りは `WS_VISIBLE` 付き。シェルとバルーンの切り替えは窓を作り直さない。
- **台本の読み手の前例**: `\j` は運搬名 `JUMP_TAG_CARRIER`（`"\j"`）の汎用コマンドへ写し、受け手の表に選別子なしで 1 行置く。`\v` は ukadoc の正典の `\![…]` と同じ意味ではない（裁定 1）ので、`\+` の前例（正典の `\![change,ghost,random]` へ写す）ではなく `\j` の前例に従う。
- **受け口と取り出しの相の前例**: `ZOrderCueSink`（talk スレッド・自己選別して `Sender` へ）→ `Emo2Wiring` の受信端 → `frame.rs` の `run_zorder_drain_phase`（`Update`）→ wintf の確定段 `FrameFinalize` の `(establish_owner_links, apply_zorder_pair_maintenance, apply_zorder_chain).chain()`。同じ巡のうちに `Update` の後に確定段が回る。
- **台帳の基底の置き直し**: `set_descript_base` は台本の組まで消す。起動の口（`apply_descript_base`）と切り替えの口（`apply_descript_rebase`）は既に分かれている。

### 12.2 設計の決め（研究の 10 節の項目への答え）

| # | 項目 | 決め | 理由 |
|---|---|---|---|
| D1 | 状態の置き場（10 節 2） | `Emo2Wiring` の欄（オン・オフの値と受信端） | ゴーストの切り替え・読み直しで作り直され、SHIORI だけの読み直しとシェル・バルーンの切り替えで残る＝要件 3.4・4.1〜4.5 の寿命が置き場だけで決まる。窓の印は wintf の部品として別に持つ（D4） |
| D2 | 受け手（10 節 3） | 新しい受け口 `WindowStateCueSink`・新しい担当 `CommandConsumer::WindowStateSink`・新しい線 | `ZOrderCueSink` に拾わせると語の見張りの対象のファイルへ語が入る。`tasktray-minimize` は同じ受け口に `minimize` の腕を足せる |
| D3 | `\v` の読み方 | `decode_bare` に `"v"` の腕。運搬名 `RAISE_TAG_CARRIER`（`"\v"`）の汎用コマンドへ写す（`\j` の前例） | `\v` は常に手前を変えない（裁定 1）＝正典の `\![set,windowstate,stayontop]` へ写すと意味がずれる |
| D4 | 新しく現れる窓（10 節 4） | wintf の印 `StayOnTop` を窓の entity に付け、窓を作る所（`window_factory`）が印を見て `WS_EX_TOPMOST` を足す。さらに HWND が付いた巡に印のある窓の帯を確かめ、外れていれば入れる | (a) 作る前に立てれば後ろに出る瞬間が無い。(b) の確かめは (a) が間に合わなかったときの保険で、どちらの順でも正しい |
| D5 | 帯の上げ方（10 節 5・R1〜R3） | ゴーストの窓を 1 枚ずつ、**今の重なりの奥から手前の順に** `SetWindowPos` する（入る＝`HWND_TOPMOST`・出る＝`HWND_NOTOPMOST`・手前へ出す＝`HWND_TOP`）。旗は `SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE`。予約の列を通さず直に書く | どの指定も「その帯の最上へ置く」なので、奥から順に当てると最後の並びは当てた順＝元の並びになる。所有の木ごと帯が動く（R1）かどうか、所有の窓が一緒に動く（`SWP_NOOWNERZORDER` の要否）かどうかに関わらず、最後の 1 回が各窓の位置を決めるので結果は同じ。持ち主が奥・持たれる窓が手前の決まりも、今の並びが既に守っているので崩れない（2026-10-31 開発者の裁定「案 A」で改め: 手前へ出すは `HWND_TOPMOST` を奥から当て、帯の外だった窓へ続けて奥から `HWND_NOTOPMOST`。帯の中は `HWND_TOPMOST` の 1 回。裏のプロセスの `HWND_TOP` は入力先の窓の手前へ出られないため・verification.md 2e〜2g 節） |
| D6 | 帯へ上げる時点 | 指令は `Update` の新しい相で取り出し、同じ巡の確定段で鎖の適用系の**後**に当てる | 同じ台本で `\![set,zorder,…]` と `\v` が並ぶ（ぱすたの「重ね順」）と、鎖を組み終えた並びを手前へ出せる。後のフレームへ持ち越さない。**→ 13 節で改めた**: 置き場は `establish_owner_links` の直後（組の是正と鎖の適用系の前）。同じ巡のまま。鎖の後押しは鎖の自分の窓の後ろへしか置かないので、`\v` で手前へ出した一式は手前に固まったまま鎖の並びに組み直される |
| D7 | 組の是正への条件（G4） | `PairObservation` に「キャラクターが帯に居るか」の 1 欄を足し、`TopOfNormalBand` を選ぶのは「手前の窓が帯に居て、キャラクターは帯に居ない」ときだけにする。キャラクターも帯に居れば `After(手前の窓)`。測れないときは「帯に居ない」へ倒す（今の避け方のまま） | 状態（オン・オフ）を wintf の判断へ持ち込まず、測った事実だけで正しい。オフのあいだの判断は 1 つも変わらない（要件 3.3）。返り値に帯へ入れる指定は足さない |
| D8 | 沈降の観測（R4・10 節 9） | 観測の記録の行に「バルーンが帯に居るか」の欄を足す。水準は `debug!` のまま | 失敗の判定は今も無い。欄があれば記録の読み手が「奥へ回らない」を正しい動きと読める（要件 6.6） |
| D9 | 件 1 の口の分け方（10 節 11） | 台帳に切り替え専用の口 `rebase_descript` を足す。台本の組を残し、基底だけを取り替え、台本の組と重なる基底は据えずに「据え損ねた基底」として持つ。`\![reset,zorder]` で台本の組を落としたとき、据え損ねた基底があれば据える | 起動の口は今のまま（§8 の 226 行目の裁量は起動について生きる）。要件 5.5「今のシェルの descript の組へ戻す」は、重なりで据えられなかった基底も含めて戻すと読む |
| D10 | 帯の書き込みの持ち主 | wintf の新しいファイル `window_band.rs` 1 つだけが `HWND_TOPMOST`・`HWND_NOTOPMOST` を書く（ツールチップの既存の 1 か所を除く） | 語の見張り（G1・G2）を「先送りの語」から「帯を書くのは持ち主の 1 ファイルだけ」へ言い直せる＝守りを捨てずに着地に合わせられる |
| D11 | 終了・読み直しの記録（要件 6.4） | 状態がオンのまま降ろすときだけ、降ろす直前に理由（終了・切り替え・読み直し）と一緒に `info!` で残す。呼ぶ所は `ghost_switch.rs` の `take_down` の 2 か所の直前と `app_exit.rs` の `quit_app` | 理由が分かるのはその場所だけ。状態がオフなら「戻した」ことが無いので書かない |
| D12 | `\v` の網羅台帳の状態 | `degraded`（動くが正典の字面どおりではない）。注記に SSP 2.9.26 の働きに合わせたことと 11 節の証拠 | 状態の 7 語の定義に合う。`stayontop`・`!stayontop` の 2 行は `implemented`（受け口に ukadoc の URL の証拠を書く） |

### 12.3 まとめの 3 つの見方（synthesis）

- **一般化**: オン・オフ・手前へ出すは、どれも「ゴーストの窓の組を、並びを保ったまま、ある帯の最上へ置く」の変種。wintf には `BandOp { Enter, Leave, Raise }` の 1 つの働きだけを作り、指定（`HWND_TOPMOST`・`HWND_NOTOPMOST`・`HWND_TOP`）だけを替える。`minimize` は帯の話ではないので、この働きには入れない（受け口の腕だけが後から足せる）。（2026-10-31 開発者の裁定「案 A」で改め: 手前へ出すは `HWND_TOPMOST` を奥から当て、帯の外だった窓へ続けて奥から `HWND_NOTOPMOST`。帯の中は `HWND_TOPMOST` の 1 回。裏のプロセスの `HWND_TOP` は入力先の窓の手前へ出られないため・verification.md 2e〜2g 節）
- **作るか借りるか**: OS の `SetWindowPos` の帯の性質をそのまま使う。重なりの並べ直しを自前で計算しない（今の並びを測って奥から当てるだけ）。新しい依存は無い。
- **削ったもの**: 帯の状態を wintf の資源に持つ案（areka の `Emo2Wiring` と二重帳簿になる）・毎フレーム帯を確かめ直す見張り（変わる時点が分かるので押し出しで足りる）・`ZOrderDirective` への相乗り（見張りの対象のファイルへ語が入る）・`zorder_chain_apply.rs` の分割（触らないので要らない）・`check_script` の第 2 引数の診断（`check-script-arg-checks` へ申し送り済み）。

### 12.4 残る危うさと確かめ方

- R1・R3 は D5 の順で結果に効かなくなった。実機で「根だけ上げたら持たれる窓も上がるか」を測って記録するのは、効かないことの確かめとして残す（タスクの確かめ）。
- R2（帯から落とす既存の書き込み）は 12.1 の 2 か所を読み切った。帯へ当てた後に各窓の帯の印を読み直し、合わなければ `warn!` を残す（要件 6.3）。実機で「オンのあいだにシェルの切り替え・バルーンの表示・ドラッグ・`\![set,zorder]` の組み替えをしても帯から落ちない」を確かめる。
- 受け手の表の手書きの数（38）は並走の spec も書き換える。着地のときの値に 2 を足す。

## 13. 設計ディスカッションの決め（2026-10-30）

- **議題 1: 帯を当てる系の置き場**（`design-validation.md` の重大な指摘 1）。確定段の並びを `establish_owner_links → apply_window_band → apply_zorder_pair_maintenance → apply_zorder_chain` にした（開発者「推奨で承認」）。
  - 理由: 組の是正は予約の列（`SetWindowPosCommand::enqueue`・`zorder_pair_maintain.rs`）に積まれて巡の終わりに流れる。帯の系を末尾に置くと、帯を当てる前の観測で決めた是正が帯を当てた後に上書きし、オンの巡でバルーンが帯から落ちる・隣り合いが崩れる、オフの巡で他のアプリの帯の窓の後ろへ引き戻される。帯の系を前に置けば、組の是正は帯を当てた後の重なりを測る。鎖の後押しは鎖の自分の窓の後ろへしか置かないので、帯の後に回っても帯の窓どうしで閉じる。
  - 退けた案: 帯の指令がある巡は組の是正を次の巡へ回す（1 フレーム遅らせない原則に触れ、組の系が帯の列を読む結び目が増える）。
  - 押さえ: 確定段の並びを固定するテストと、同じ巡に是正が要る場面の偽の OS のテストを 1 件ずつ（design.md Testing Strategy）。
