# 設計の検証: stay-on-top-directives

- 対象: `design.md`（2026-10-30 生成）・`requirements.md`（承認済み）・`research.md`
- 手順: `kiro-validate-design`（分析 → 重大な指摘 → 良い所 → GO/NO-GO）。非対話で実施。
- 確かめたコード: `crates/wintf/src/ecs/world/mod.rs`（1 巡の段の並び）・`crates/areka/src/emo2_boot/frame.rs`（`run_zorder_drain_phase` の置き場）・`crates/areka/src/placement/spawn.rs`（`wire_zorder_pair` の確定段の並び）・`crates/wintf/src/ecs/window/zorder_pair.rs`／`zorder_pair_maintain.rs`／`zorder_pair_establish.rs`（組の是正の判断・予約の列・`ReassertZOrder` の入れ所）・`zorder_chain.rs`／`zorder_chain_apply.rs`（鎖の後押しの挿入位置と直書き）・`window_system.rs`／`runtime/window_factory.rs`（窓を作る段と `compute_ex_style`）・`graphics/systems/window_pos.rs`（位置の書き込みの `ZOrder::NoChange`）・`zorder_group_ledger.rs`／`frame/zorder_descript.rs`（基底の据え方）・`ghost_switch.rs`／`app_exit.rs`（降ろす道）・`zorder_pair_deferred_vocabulary_tests.rs`（G1 の走査の範囲）。

## 設計の要約

全体の型（受け口 → 線 → 取り出しの相 → wintf の確定段）は既存の `\![set,zorder]` と `\![move]` の流儀にそろっており、境目（状態は areka、帯の働きは wintf）もはっきりしている。「奥から順に、帯の最上へ置く指定を 1 枚ずつ直に当てる」手順は、持ち主の伝播（R1・R3）に寄りかからずに並びを保つ正しい形である。ただし、組の是正が**予約の列（巡の後の flush）**へ積む書き込みと、帯の系の**直書き**の前後を設計が扱っておらず、同じ巡に両方が出ると帯の出入りが崩れうる。これを直す小さな手当てを足せば実装へ進める。

### 確かめた点（設計の主張と実物）

- **同じ巡か**: 正しい。`try_tick_world` は `Input → Update → … → FrameFinalize` を 1 巡で固定の順に回す（`wintf/src/ecs/world/mod.rs` 684〜707 行）。`run_window_state_phase` は `Update` の `emo2_frame_system` の中、`apply_window_band` は同じ巡の `FrameFinalize` の末尾。1 フレーム遅れにはならない。
- **直書きが既存の流れとぶつかるか**: 鎖の適用系も `guarded_set_window_pos` を直に呼ぶ前例がある（`zorder_chain_apply.rs` 863 行）。位置の書き込み（`apply_window_pos_changes`）は `ZOrder::NoChange` で重なりを動かさない。本番で `ZOrder::Top`／`InsertAfter` を書くのは組の是正と鎖の後押しだけ（設計の主張どおり）。
- **奥から順の手順**: `HWND_TOPMOST`／`HWND_NOTOPMOST`／`HWND_TOP` はどれも「その帯の最上へ置く」ので、奥から当てれば最後の並び＝当てた順＝元の並び。持ち主が持たれる窓を連れて動いても、続く窓への 1 回が上書きする。`PairReleased` の組・鎖の外の窓も同じ列に入るので扱いは変わらない。持ち主が奥・持たれる窓が手前の決まりも今の並びが既に満たす。
- **鎖の後押し**: 挿入位置は鎖の自分の窓だけ（`nudge_command` の `members[0]`／`anchor`）。オンのあいだは鎖の窓が全部帯に居るので、帯から落とさない。帯の系は鎖の適用系の後に回るので、同じ巡の組み替えの後の並びを測って当てる。
- **組の是正の 1 条件**: 表の 4 行は正しい。キャラクターも帯に居るとき `After(手前の帯の窓)` は帯の中に留まる。G1 の走査は `zorder_*` の本番ファイルの「コードだけ」に `topmost` が無いことを見るので、欄名 `measured_below_is_always_on_top` は語の守りに触れない。
- **2.3（後ろに出る瞬間を作らない）**: 窓を作るのは `UISetup` の `create_windows`（`Update` の後）。取り出しの相が `Update` で印を付ければ、同じ巡で作る窓は `WS_EX_TOPMOST` 付きで生まれる。
- **5.1〜5.5**: 今の `apply_descript_rebase` は `set_descript_base(Vec::new())` で台本の組まで落としている（申し送りの件 1 の原因を確認）。`rebase_descript`＋据え損ねた基底（`pending_base`）は 5.1・5.2・5.5 を満たし、起動の口を変えない。
- **1,000 行**: 触るファイルは上限内に収まる見込み（`zorder_pair.rs` 892・`ghost_switch.rs` 938・`spawn.rs` 810・`zorder_pair_maintain_always_on_top_tests.rs` 838 に 1 件）。`zorder_chain_apply.rs`（930）と `zorder_pair_maintain_tests.rs`（977）に足さない方針は正しい。テストはどれも兄弟のファイル。

## 重大な指摘

🔴 **重大な指摘 1**: 予約の列へ積んだ組の是正が、同じ巡の帯の直書きの**後**に流れて帯を崩す

- **懸念**: 組の是正は観測を測って `SetWindowPosCommand::enqueue` へ積み、実際の書き込みは**巡の後の flush**で起きる（`zorder_pair_maintain.rs` 524〜530 行）。帯の系は確定段の末尾で**直に**書くので、同じ巡では「帯の前に測った観測から決めた是正」が帯の後に当たる。(a) オンの巡: 是正が `After(普通の窓)` なら、帯に入れたばかりのバルーンが普通の窓の後ろへ置かれて帯から落ちる（`TopOfNormalBand` ならバルーンが帯の最上へ飛び、隣が崩れる）。(b) オフの巡: 是正が `After(他のアプリの常に手前の窓)` なら、外したばかりのバルーンが帯へ引き戻される（`zorder_pair.rs` 264〜268 行が書く毀損そのもの）。是正の要求は、2 組以上あると `fix_issued_this_pass` で次の巡へ持ち越される（503〜510 行）ので、起動直後の指令と重なりうる。手順 3 の読み直しは flush より前なので、この崩れを記録できない。
- **影響**: 2.1・2.4・3.1 の破れが、決定論のテストにも手順 3 の記録にも出ない。設計の「R2 は本番の書き込み 2 か所を読み切った」は、書き込みの**時点**（予約の列か直書きか）を見ていない。
- **提案**: 次のどちらかを設計に書き、決定論のテストを 1 件足す。① 確定段の並びを `establish_owner_links → apply_window_band → apply_zorder_pair_maintenance → apply_zorder_chain` にする（是正の観測が帯の後の状態を測る。鎖は自分の窓だけを挿入位置にするので帯の窓どうしで閉じる。帯の手順の測りは鎖の前の並びになるが、鎖が帯の中で並べ直す）。② 並びは今のまま、維持の系が `WindowBandQueue` に指令がある巡は是正を積まずに要求を次の巡へ残す（`fix_issued_this_pass` と同じ型）。①は 1 行で済み、②は維持の系が帯の列を読む依存が増える。どちらでも「同じ巡に是正と帯の指令が出たとき、帯が崩れない」テストを偽の OS で固定する。
- **該当の要件**: 2.1, 2.4, 3.1, 6.3, 7.3
- **根拠（設計）**: 「Existing Architecture Analysis」（組の是正は予約の列、と書きながら帯の系との前後を扱わない）・「帯を当てる手順」の R2・「WindowBand」の Responsibilities（確定段の並びの末尾）

（重大な指摘は上の 1 件だけ。2 件目以降に挙げる値打ちのある崩れは見つからなかった。）

### 重大ではない注記（タスクで拾えばよい）

- 帯の系の ⑵（印のある窓に HWND が付いた巡の `Enter`）は 1 枚だけを帯の最上へ置くので、持ち主を結ばない組（`PairReleased`）のキャラクターがここを通るとバルーンの上へ出る。作る時点で `WS_EX_TOPMOST` が付く道が主で ⑵ は保険なので、記録（`debug!`）で足りる。実機の確かめ（R2）で見る。
- `zorder_pair_maintain_always_on_top_tests.rs`（838 行）へ足す 1 件は 160 行を超えると上限に掛かる。超えそうなら兄弟のファイルを分ける。

## 設計の良い所

1. **測っていない OS の振る舞いに寄りかからない帯の当て方**: 奥から順に「その帯の最上へ置く」指定を 1 枚ずつ当てる形は、所有の伝播（R1）・`SWP_NOOWNERZORDER` の要否（R3）・`PairReleased` の組・鎖の外の窓のどれでも同じ最後の並びになる。`\v`（`HWND_TOP`）も同じ手順で帯ごとの出し先が自然に決まり、状態を持たない wintf と、状態と寿命を持つ areka の境目がきれいに分かれている。
2. **寿命を既存の作り直しの単位に乗せた**: 常に手前の状態を `Emo2Wiring` に置くことで、終了・切り替え・読み直しで消え、SHIORI だけの読み直し・シェルとバルーンの切り替えで残る、という 4.1〜4.5 の分岐が新しい仕組みなしに満たされる。台帳の口も起動用（`set_descript_base`）と切り替え用（`rebase_descript`）を分け、起動の意味を変えずに件 1 を直している。

## 最終の判断

- **判断: GO**（重大な指摘 1 の手当てを設計の討議で決めてから、タスクへ進む）
- **理由**: 境目・依存の向き・寿命・要件の追跡はそろっており、実物のコードと照らして主張は正しかった。崩れうるのは予約の列と直書きの前後 1 点だけで、確定段の並びを 1 行替えるか、維持の系に 1 条件を足せば閉じる。
- **次の手**: `/kiro-design-discussion stay-on-top-directives` で指摘 1 の案（①並びを替える／②是正を持ち越す）を決め、`design.md` の「WindowBand」「PairFixBandCondition」「Testing Strategy」に反映する。その後 `/kiro-spec-tasks stay-on-top-directives`。
