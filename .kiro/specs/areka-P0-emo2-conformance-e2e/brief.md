# Brief: areka-P0-emo2-conformance-e2e

> **種別**: 本坑（main）・⓪ ghost 帰属の**全エンジン統合点＝M1 完成宣言ユニット**（アプリ組み上げ三段の第三段・M-e2e）。roadmap「M-e2e ＝ 全エンジン統合・boot→talk→touch→menu→close 一周適合・M1 ゴール充足」の brief 化。
> **調査日**: 2026-07-16（再入精査⑧・体裁フェーズ棚卸）。
> **🧭 ゴール裁定を収録**: 本 brief が **M-dual（dual-surface／dual-window）の吸収・退役の正本**（2026-07-16 裁定）——M-dual は 2026-07-16 実査で「大半 M-boot 充足済み・残作業は検証と gap-fill に縮退」と確定済み（roadmap 統合点行）。独立 spec を起こさず、**本ユニットの適合検証項目として消化**する。
> **⛔ 着手ゲート**: M1 残ユニットの全完了後（最終ユニット）。ただし**適合表の維持は今から**（下記チェックリストが M1 の「迷いの発生しないゴール」の単一定義）。

> **📌 2026-08-01 追記(58)棚卸更新（棚卸⑤・本ブロックが(52)㊹より優先）**:
> - **W5 は 3/4 着地**（choice-select-events✅・kero-balloon✅・dpi-window-vanish✅）・残＝collision-dpi-hittest は **W6 へ編入**。残ウェーブ改訂＝**W6（col ∥ vis ∥ bind ∥ zorder ∥ scg の5本）→ W6.5（exact ∥ wpl）→ W6.75（budget ∥ atom+bod〔縮退時統合〕）→ W6.9（cage）→ W7=本 spec**（正本は roadmap 追記(58)）。
> - **上流列へ追補 5 本**: `ghost-window-zorder`（バルーン埋もれ＝一周走行の可視性前提）・`scope-chain-gap`（P2 幅差隙間・SSP 実測正典）・`windowposition-limit`（バルーン画面外はみ出し）・`recompose-budget`（アイドル CPU 税＝e2e 実機走行の観測品質前提）・`dpi-transition-atomicity`（適合 #1 DPI 検証時の跳ね解消・+36px 追随）。
> - アンカー: spawn.rs `GhostWindows` :115 → **:164**・`ScopeWindows` :101 → **:150**（van の despawn hook 挿入）。target_map.rs `shell_target` :19 不変。
>
> **📌 2026-07-31 追記(52)棚卸更新（W4 完走・本ブロックが㊹以下より優先）**:
> - **completed 追補（㊹以降）**: wintf-gpu-test-crash（割込）✅・position-persist✅・choice-interact✅・emo-dpi-scaling✅＝**W4 完走**。**残ウェーブ改訂**: W5（dpi-window-vanish ∥ collision-dpi-hittest ∥ choice-select-events ∥ kero-balloon）→ **W6（balloon-visibility ∥ bindoption-exclusivity の2本・追記(52)裁定）**→ **W6.5（scale-exact-rational ∥ test-cage-determinism・追記(51)起票）**→ W7=本 spec。
> - **上流列へ追補3本**: `bindoption-exclusivity`（表情固着バグ＝**適合 #3「着せ替え表情」の前提充足**・bindoption 3値正典準拠）・`scale-exact-rational`（**適合 #1 の DPI 検証を絶対値で書ける前提**＝画素演算の有理数化。**【2026-08-14 失効・下記追記(68) が正本】**——有理数化は却下され、供給面寸の判定には **+1 の許容が要る**）・`test-cage-determinism`（M1 宣言を支える檻の決定性）。
> - **着手時義務**: 本 brief の全面再監査（追記㊹時点で唯一補正無しだった経緯・調査日 2026-07-16 の実測は全面陳腐化前提で読む）・適合表へバルーン表示ライフサイクル項目追補・㉘(E)「OnFirstBoot 限定 `\![move]` の2回目起動蒸発は許容仕様」の実機判断・#7（冒頭空行＝pasta 上流未解決）は M1 完成を妨げない扱いの確認。
> - アンカー: spawn.rs `GhostWindows` :109-130 → **:115**（`ScopeWindows` :101）・target_map.rs `shell_target` :19 不変。

> **📌 2026-07-24 追記㊹棚卸更新（本ブロックが以下の本文より優先・調査日 2026-07-16 の残ユニット認識は失効）**:
> - **completed 済み**: cue-playback-duration・mayuna-compose・seriko-loop・sakura-dialogue-tags・choice-render・input-events・idle-talk・collision-geometry・sylphya（本文の「実装中/並走中」記述は全て過去形へ読み替え）。**残ウェーブ**: 割込 `wintf-gpu-test-crash`（DoD ゲート復旧）→ W4（position-persist ∥ choice-interact ∥ emo-dpi-scaling）→ W5（dpi-window-vanish ∥ collision-dpi-hittest ∥ choice-select-events ∥ kero-balloon）→ W6（balloon-visibility）→ **W7=本 spec**。
> - **上流列へ追補4本**: `choice-interact`（choice-render 2分割の対話半分・**`ChoiceSelection` 正本**＝適合 #7 hover の対話面と #8 の供給元）・`balloon-visibility`（M1 編入裁可済＝**本 spec 着手時に適合表へ「バルーン表示ライフサイクル」項目を追補**・roadmap W7 行登記済み）・`kero-balloon`（**#10 kero 一式の前提充足**＝kero が `balloonk*` 正典資産で表示・placement 採寸 scope 別）・`sylphya`✅（%username 実導出）。#1 の DPI 検証は DPI 追従込みへ格上げ（追記㉟裁定）。
> - アンカー微修正: spawn.rs の `GhostWindows` は **:109-130**（`ScopeWindows` :95-100）・target_map.rs:19-38 は不変。

> **📌 2026-08-13 追記(63)（`areka-P0-scope-chain-gap` からの申し送り・適合表の期待値更新）**: 二体の位置に関する適合項目の**期待値が 2 つ変わった**。着手時に適合表と決定論 spine の期待列を再突合すること。
> - **⑴ 二体の既定間隔＝隣接（隙間 0）が正典**（scg 要件 1/2・SSP 実測で確定）。連鎖式は `scope_n.L = scope_{n-1}.L − scope_n 自身の幅`。さらに要件 7 で「初期配置は実表示サーフェス寸が確定するまで暫定」とし、確定時に一度だけ解き直すようにした（`chain_finalize.rs`／`drain_resnap.rs` の `finalize_chain_once`）。**定常表示状態で隙間 0** が期待値。実機実測（拡大率 200%・移動指令を除いた複製ゴースト）で `1392+672 = 2064` ＝ scope0 左端 `2064` を確認済み。
> - **⑵ 項目 9（`\![move,-353,...]`）の着地座標が k≠1 で変わった**。台本オフセットが k 倍されるようになったため（`resolve_move_target_position` に `k: ScaleRatio` 追加）。式は `x' = base_pos.x + basepos(base窓).x + k·dx − basepos(対象窓).x`。**k=1 では従来と同値**ゆえ 100% の適合走行に差は出ないが、実 DPI≠96 の走行では着地が変わる。実測: 拡大率 200% で二体の重なりが **365px → 12px**（＝100% の 6px のちょうど 2 倍）。
> - **参照実装との関係**: SSP は `\![move]` を無スケールで適用するため、⑵ は**意図的な SSP 非互換**である（`ssp-oracle-notes.md` の SSP 自己不整合 #2）。適合検証で SSP と突き合わせる場合、この 1 点は差が出るのが正しい。
> - 併せて `#12`（初回ゲート）と項目 9 の相互作用（本 brief 末尾の申し送り）を判断する際は、要件 7 の「明示的に再配置されたスコープは既定連鎖へ引き戻さない」（現在位置と既定位置の一致で判定）も前提に入れること。
> - scg 側の正本: `.kiro/specs/completed/areka-P0-scope-chain-gap/`（要件 1/2/7・`real-run-signoff-2026-08-13.log` §5.5）。

> **📌 2026-08-14 追記(68)（`areka-P0-scale-exact-rational` からの申し送り・適合 #1 の DPI 判定式）**: **供給面寸（文字供給面の確保寸）を判定に使う場合、絶対値一致では書けない。** 本ブロックが上記追記(52) の「絶対値で書ける前提」記述より優先する。
> - **⑴ 供給面寸は期待値 +1 の許容が要る**。誤りが出るのは拡大率 **6/5 と 12/5 の 2 比のみ**（各 81 件・寸 1..=1200）で、残る 21 比は 0 件・差は常に **0 か 1**（−1 は起きない）——ただし**判定式は一律 +1 許容で書くのが安全**（比ごとに場合分けすると、比の集合が変わったときに黙って割れる）。
> - **⑵ 窓 client 寸は従来どおり絶対値で書ける**。こちらは丸め権威（`ScaleRatio::scaled_extent`）経由であり、⑴ の許容は**持ち込まない**。適合 #1 の判定を書く際は供給面寸と窓 client 寸を混ぜず、許容を付けるのは前者だけにすること。
> - **⑶ 失効**: 追記(52) 上流列の `scale-exact-rational`「**適合 #1 の DPI 検証を絶対値で書ける前提**＝画素演算の有理数化」は**失効**。有理数の文字層配管は 2026-08-14 に却下され、当該 spec は裁定の登記・前提の決定論テスト・申し送りへ縮小された（実行時の挙動は不変ゆえ、適合表の他項目の期待値は変わらない）。
> - **⑷ 根拠**: 再説明しない。spec **`areka-P0-scale-exact-rational`** の裁定登記（emo-text `region.rs` の `ScaleContract::physical_extent` doc）を参照。前提（差は 0 か 1・−1 は起きない・件数 81/81/0×21）は決定論テスト `crates/areka-emo-text/tests/physical_extent_arbitration_test.rs` が固定している。
> - 判定式の最終形（どの項目でどこまで許容を書くか）は**本 spec が決める**——上記は前提の申し送りであって、適合 e2e の要件裁定ではない。

> **📌 2026-08-21 追記(73)（`areka-P0-dpi-transition-atomicity` からの申し送り・一周走行で使える遷移観測チャネルが増え、上流 `ghost-window-zorder` の実機確認をここで受ける）**: **拡大率を切り替えたときの窓の動きを 1 本の時系列で機械判定できる観測チャネルとランナーが常設された。一周走行の適合表へ「拡大率切替」の項目を足せる材料が揃っている。**
> - **⑴ 恒久観測（削除しない・後続 spec が再利用する契約）**: 既定 OFF の target `wintf::transition`（`crates/wintf/src/ecs/window/transition_diag.rs:54`）に、モニタ表更新（`kind=monitor`）・メッセージ受理（`msg`）・指令の積み上げ（`enqueue`）・一括 flush（`flush`）・窓書込（`write`）・サーフェス寸（`surface`）・作業領域源の差し替え（`snapshot`）・整合待ち（`hold`）・接地点（`ground`）・連鎖の解き直し（`chain`）の 10 種が出る。**点灯手順・grep 語・判定の全文は `.kiro/specs/completed/areka-P0-dpi-transition-atomicity/signoff-procedure.md`**（手順書の語が発行側の単一定義元と一致することは `crates/areka/src/placement/transition_signoff_procedure_tests.rs` が檻で固定している＝手順書が陳腐化したら赤くなる）。
> - **⑵ 実機ログの機械判定ランナーがある**: `AREKA_TRANSITION_LOG=<絶対パス> cargo test -p areka transition_signoff -- --ignored --nocapture`（`crates/areka/src/placement/transition_signoff_tests.rs:10,57`）。ランナーは自前の判定を 1 行も持たず、決定論テストと**同一の純関数**を回す。環境変数未設定・パス不達・観測行 0 行はいずれも失敗（無言スキップで緑を偽装しない）。**一周走行のログをそのまま食わせられる。**
> - **⑶ 上流 `ghost-window-zorder` の実機確認をここで受ける（受け先の判断）**: 本 brief は追記(58) で `ghost-window-zorder` を上流列へ追補済み（「バルーン埋もれ＝一周走行の可視性前提」）。当該 spec は `.kiro/specs/completed/` にあり**申し送りを消化できない**ため、**実機の見た目の側（バルーンがキャラの手前に居続けること）は本 spec が受ける**。本 spec は既に zorder の残件を 1 件抱えている——「再表示直後の隣接が実機未確認」。**拡大率の切替は再表示を伴うので、一周走行に拡大率切替を含めれば同時に確認できる。** コード側の適用順・適用回数の前提は `areka-P0-draw-load-parity` の追記(71) が受けている（そちらが flush 経路を In-scope に持つ）。
> - **⑷ Z の適用順は変えていない（確認の観点）**: 窓書込指令は同一窓のジオメトリ指令が積む時点で合流するようになったが、**挿入位置を持つ Z 指令は合流対象外**で、畳めない指令は同一窓の仕切りとして働く（`crates/wintf/src/ecs/window/command.rs:229` の 3 連言）。ゆえに一周走行で見るべきは「Z が崩れていないか」であって「合流で順序が入れ替わったか」ではない。
> - **⑸ 適合表へ足せる項目（本 spec が裁定すること）**: 拡大率を切り替えたとき、⒜ キャラの接地点が新しい作業領域の下端に載る（タスクバーへ潜らない）、⒝ 随伴バルーンが**同一フレームで**追従し追従 offset が変わらない、⒞ 遷移の途中で中間矩形（旧下端の位置）が提示されない、⒟ 二体の隣接（隙間 0）が遷移後に解け直る、の 4 点が決定論テストで固定済みである。**実機で残るのは目視の側**（跳ねが見えないこと）で、それは本 spec の一周走行の射程に入る。
> - **⑹ 二体の隣接は遷移後に解き直される（追記(63) の期待値に条件が 1 つ増えた）**: `scope-chain-gap` 由来の追記(63)⑴ は「二体の既定間隔＝隣接（隙間 0）が正典」「初期配置は実表示サーフェス寸が確定するまで暫定・確定時に一度きり解く」と書いている。本 spec の是正で、**拡大率遷移のたびに同じ判定器で一度きり解き直す**ようになった（実機で 359px 開いていた隙間が 0 になる。決定論の対応物は `crates/areka/src/emo2_boot/frame_chain_realign_tests.rs`）。ただし**ドラッグ等で明示的に動かされたスコープは対象外のまま**（既定位置と現在位置の一致で判定）で、この規約は追記(63) から変わっていない。適合表の「二体の位置」の期待値は**遷移後も隣接**で読むこと。
> - **⑺ perf 行に `frame=` が末尾追加された**: `perf(apply_show)`（`crates/areka-emo-present/src/presenter/timing.rs:220`）。既存フィールドの順序・名前・文言は不変で `tools/perf/judge-perf.py` とは互換。一周走行のログで perf 行と遷移観測行を**同一フレームで**突合できる。
> - **⑻ 語彙の不変条件**: 窓種別は `win_kind=`（`transition_diag.rs:167`）で、`kind=` はレコード種別（:143）。**1 行に同じフィールド名を 2 度出さない**（`judge-perf.py::parse_fields` が後勝ちで潰すため）。ログを読む道具を足すときはこの規律に従うこと。
> - **⑼ 正本**: `.kiro/specs/completed/areka-P0-dpi-transition-atomicity/`（要件 2／4／5／8・`signoff-procedure.md`・`mechanism-ledger.md` が file:line の正本）。
>
> **📌 2026-08-22 追記(77)（`areka-P0-dpi-transition-atomicity` からの申し送り・追記(73) の続き。上流の実機サインオフが FAIL のまま裁定 GO で閉じたので、一周走行が見る「跳ね」の期待値が変わる）**: 追記(73) は「拡大率切替を適合表へ足せる材料が揃った」と渡したが、**その材料で実際に測った結果が出た**。適合表を書くときの期待値に直に効く。
> - **⑴ 窓は 4 枚同時に動くようになった**: 1 遷移内の窓書込の散らばりは **93,152〜157,684µs → 40〜101µs**（約 1,500〜2,000 分の 1）。task 7.2 が一括 flush を `Begin/Defer/EndDeferWindowPos` の 1 バッチへ移した効果である。**「窓が 1 枚ずつ順にずれていく」形は一周走行では観測されないはず**であり、観測されたら退行である。
> - **⑵ しかし目視では跳ねが残る**: 絵が新しい拡大率で描かれてから窓がその寸へ動くまでが **210,329〜306,301µs（0.21〜0.31 秒）**ある。**この「絵が先・窓が後」は本 spec の走行でも見える。**上流の欠陥として再起票しないこと。**なお引受先は 2026-08-28 時点で存在しない**——`areka-P0-present-write-coherence` は要件討議の裁定（見送り＋登記）に従い**是正コード 0 行**で閉じ、この量を**未達 40 件・引受先なし**として登記した（正本＝`.kiro/specs/completed/areka-P0-present-write-coherence/requirements.md` の「未達の登記」節、検証記録＝同 `verification/notes.md`）。**したがってこの跳ねは本 spec の走行でも着手前とまったく同じように見える。**適合表で拡大率遷移の見た目を判定条件に含めてはならない。将来直すには新規仕様の起票が要る。
> - **⑶ 機械判定と目視が食い違うのは正常な向きが 1 つある**: 上流の判定器は決定論系統（フレーム単位）で PASS を出しつつ、実機専用系統（µs）で FAIL を出した。**同じ症状を別の量で測っている**ためで、判定器の欠陥ではない。読み分けの手順は `signoff-procedure.md` **§6.5**（4 行の表＋3 問の分岐手順）にある。**本 spec が拡大率切替の項目を書くときは、この分岐手順をそのまま引くこと**——独自の突合規約を作ると上流と食い違う。
> - **⑷ 追記(73) ⑹「二体の隣接は遷移後に解き直される」は実機で成立した**: 8 遷移すべてで連鎖の解き直しが 1 回だけ起き（`chain_realigned=1`）、接地点差は 0 だった。**再表示直後の隣接（本 spec が抱える zorder の残件）を確認する好機は拡大率切替の直後である。**
> - **⑸ ドラッグ追従の比 1.000 を実機で確かめるのは本 spec の持ち分**（上流が明示的に未測定として渡す）: 上流の実機サインオフは**採取中のドラッグを禁じる**条件で成立しており（手順書 §4.4・`ATOM-NO-DRAG: PASS`）、ドラッグは判定の外に置かれている。一方でアンカー付きキャラ窓のドラッグ追従は `crates/areka/src/placement/follow/drag_follow.rs:89`・`:183` が指令キューへ積む＝**上流が変えた合流と一括バッチをそのまま通る**。決定論の檻（`follow_drag_tests.rs`）は指令キューまでしか測らない（一括 flush を通らない＝上流 design D11）ので、**「掴んで動かしたとき窓がカーソルに 1:1 で付いてくるか」を実機で見るのは本 spec が唯一の場所である。**一周走行の実機項目へ足すこと。根拠と経緯は上流 requirements.md 要件 10.6 の注記。
> - **⑹ 判定器の既知の限界を 1 つ承知しておくこと（ランナーを流用する側の注意）**: 「見送り窓（再表示が `invisible` で見送られた窓）への書込を数えない」規則は `frames_to_last_write` には適用されているが、`writes_per_window`（窓ごと 1 回）には**広げていない**。よって**見送り窓が 1 遷移の中で別々の tick に 2 本書いた形**では、`writes_per_window` が偽の違反を立て得る。上流の実機採取 2 回（7.1＝7 遷移・7.3＝8 遷移＝計 15 遷移）でこの形は **1 度も起きていない**ので上流は広げずに閉じた（裁定と根拠は上流 `mechanism-ledger.md`）。**一周走行で `writes_per_window` の違反を見たら、まず当該窓が `skipped_windows` に居ないかを確かめること**——居るなら判定器側の限界であって製品の欠陥ではない。
> - **⑺ 正本**: `.kiro/specs/completed/areka-P0-dpi-transition-atomicity/mechanism-ledger.md` **§11**・`signoff-procedure.md` **§6.5**／**§6.6**・requirements.md 要件 8 の注記（裁定 GO の全文）。

## Problem

M1 ゴール「emo2 が**そのまま** boot→talk→touch→menu→close まで E2E 実走する」を**証明する仕様が無所属**。各ユニットは自分の観測（決定論檻＋個別実機サインオフ）を持つが:

- **一周を貫く適合走行**（起動→自発会話→撫で反応→メニュー一周→位置調整→終了挨拶→clean exit）を単一の pass/fail として持つ檻・手順・記録が無い。
- **M-dual の残作業（kero 側検証）が宙に浮く**: kero 窓・バルーン窓は spawn 済み（`GhostWindows`＝scope 毎 char+balloon の2窓・`spawn.rs:88-123`）・target 採番偶奇（`target_map.rs:19-38`）・kero alias 解決（seriko ✅・`surfaces.txt:458-507` の `通常,[2100]`〜`ジト,[2110,2210]`）・`\p[n]`/`\1` 交替は R9.3 実機動作——だが「**kero 側の一式が揃って正しい**」（バルーン `balloonk0` 表示・kero 撫で・kero まばたき・kero 位置調整）を誰も束ねて観測していない。
- 「M1 完成」の宣言基準（DoD）が分散している（workspace テスト・License Gate・各所実機サインオフ）——**M2 再構築の起点**となる完成宣言はここで一本化する。

## Current State（2026-07-16 棚卸）

- **完了済みの土台**: M-boot 23/23 ✅（起動→OnBoot talk→close 握手・決定論 spine ＋実機 R9.3）・ghost-setup の spine e2e（`ScriptedShioriBackend`＋`RecordingSink`・S1〜S6）が決定論 conformance 走行の**拡張母体**。
- **残ユニット（本 spec の上流・2026-07-16 時点）**: `cue-playback-duration`（実装中）→`mayuna-compose`／`seriko-loop`／`sakura-dialogue-tags`→`choice-render`／`choice-select-events`＋並走中の `position-persist`／`idle-talk`／`collision-geometry`／`input-events`。**全完了で本 spec が解禁**。
- **fixture の実全体像**（2026-07-16 実査）: dic ハンドラ＝OnFirstBoot/OnBoot(Lua)/OnClose×3/OnTalk×9(内部)/時報(内部)/OnMouseMove(撫で16シーン・**Head1/Bust1＝kero 側撫でも実在**)/OnMouseDoubleClick(メニュー)/OnUpdate*4/OnBalloonChange。バルーン fixture＝balloons0/balloonk0＋arrow0/1・marker・online0-3・balloonc1-4・sstp*。

## Desired Outcome

**M1 の完成が単一の適合走行で証明され、開発者が「M1 完了」を宣言できる。**

**✔ 観測（単一 pass/fail・二層）**:
- **(a) 決定論 conformance spine（CI 常設）**: `ScriptedShioriBackend` 拡張＝boot→（Tick 注入で）自発 talk→（注入 MouseMove 列で）撫で GET→（注入 DoubleClick→ChoiceSelection で）メニュー一周→（`\![move]` cue）→OnClose 握手→clean exit、の**全 SHIORI 交信列と全表示指令列が期待一致**（sleep 不使用・実 pasta 不要）。
- **(b) 実機一周適合（人間サインオフ・M1 完成宣言）**: 実 emo2・実 pasta.dll・実 DPI（≠96）・絶対パス起動で、下記**適合検証項目表**を一周で目視確認し、記録（acceptance-record.md）に残す。

## 適合検証項目表（M1 ゴールの単一定義・迷いの発生しないゴール）

| # | 項目 | 由来ユニット |
|---|---|---|
| 1 | 起動: 実 surface 表示（first `\s` まで非表示→表示）・既定位置（右下・相方は左）・DPI 正 | emo2-boot✅/window-placement✅ |
| 2 | OnBoot 挨拶 talk が typewriter＋正しい wait/改行/表情同期で再生 | cue-playback |
| 3 | 着せ替え表情（`\![bind]`）でむらさきの表情が変わる | mayuna-compose |
| 4 | まばたき2系統（むらさき bind+random／エモ random） | seriko-loop |
| 5 | 放置で自発会話（時報系）・talk 中は割込まない | idle-talk |
| 6 | 撫で: Head/Bust ストローク→touch 反応（**sakura 側＋kero 側の両方**＝Head0/Bust0/Head1/Bust1） | collision-geometry＋input-events |
| 7 | ダブルクリック→メニュー表示（選択肢・字下げ・hover 反転） | input-events＋sakura-dialogue-tags＋choice-render |
| 8 | 選択→シーン遷移→サブメニュー→もどる→閉じる（一周） | choice-select-events |
| 9 | エモの位置調整（`\![move,-353,...]`・boot 時＋メニュー発火時） | sakura-dialogue-tags |
| 10 | **二人立ち総合（旧 M-dual の吸収先）**: kero 窓＋kero バルーン（balloonk0）表示・`\1`/`\p[n]` 交替・kero alias 表情（`\s[ジト]` 等）・両バルーン追従/単独ドラッグ | M-boot 充足＋本 spec 検証 |
| 11 | `%username` が展開されて表示（生文字列露出なし） | sakura-dialogue-tags |
| 12 | 位置永続化: 窓を動かして終了→再起動で復元・OnFirstBoot は初回のみ | position-persist |
| 13 | 終了: メニュー or 退避手段→OnClose 挨拶→`\-`→clean exit（stand-in despawn 不使用） | input-events＋kanade✅ |
| 14 | **省略可項目の縮退確認**（gap 監査）: OnUpdate*/OnBalloonChange＝未送出（M2）・arrow/marker/online/balloonc/sstp＝未描画で破綻なし・OnChoiceTimeout の裁定どおりの挙動 | 本 spec |

## Approach

1. **決定論 spine の拡張**（ghost-setup S1〜S6 の母体を conformance 台本へ）: scripted backend に「自発 talk 応答・撫で応答・メニュー script・選択遷移 script・move 込み script」を追加し、**入力注入（Tick/Mouse/ChoiceSelection）→SHIORI 交信列→表示指令列**の全経路を1本の統合テストに固定（CI 常設・実 pasta 非依存）。
2. **実機適合走行の手順書＋記録**: 上記14項目のチェックリストを acceptance-record.md 様式で固定（window-placement の前例に倣う）。実 DPI 120/192 級・マルチモニタ跨ぎを含む。
3. **gap-fill の規律（escape hatch）**: 適合走行で発見された欠陥は**本 spec で直さない**——症状を仕分けし、小さければ「この場修正」・構造的なら**個別 spec を just-in-time で切って先に完遂**（emo2-boot R9.3 の実機7件仕分けと同じ運用・spec 工場回避のまま）。本 spec は「証明」に徹する。
4. **M1 完成宣言の DoD 一本化**: `cargo test --workspace` exit 0（i686 host-32 成果物ビルド後・[[workspace-test-needs-i686-host32-artifacts]]）＋ License Gate（cargo deny＋cargo about・kiro-complete DoD 統合済み）＋ 実機14項目サインオフ（開発者・人間判断）＝**M1 完成**。完了時に roadmap の M1 節を閉じ、M2 再構築（「実物を見て組み直す」）の起点を宣言する。

## クロスユニット契約（2026-07-16）

- **M-dual の吸収・退役（本 brief が正本）**: roadmap 増分の `areka-P0-dual-surface`（⑤）・`areka-P0-dual-window`（⑥）は**ユニット名を退役**——実体（kero 窓 spawn・target 偶奇・alias 解決・`\p` 交替）は M-boot で充足済みであり、残る「kero 側の束ねた検証」は項目 #10 が所有。検証で構造的 gap が出た場合のみ、その症状に対する個別 spec を just-in-time で切る（旧名を復活させない）。
- **各ユニットの実機サインオフとの関係**: 各上流ユニットは自分の症状の実機確認を済ませて完了する（判定は各 spec 帰属）——本 spec は**相互作用と一周**（例: 撫で talk 中にメニュー・選択待ち中の自発 talk 抑止=Status: choosing）を観測対象とする。**判定を混ぜない**規律は idle-talk brief と同型。
- **決定論 spine の資産系譜**: ghost-setup `ScriptedShioriBackend`／emo2-boot spine／kanade 統合テストの拡張であり**新しいテスト機構を発明しない**。
- **画家則の適合範囲（2026-07-17 合流裁定で登記・collision-geometry research §10.6 の申し送り受領）**: collision 重なり優先は emo 合成規約＝**画家のアルゴリズム**（後定義が手前）で SSP `collision-sort`（既定 none＝先書き手前）とは**逆向き**（collision-geometry 議題1裁定）。emo2 fixture には重なり collision も `collision-sort` 宣言も無く、本 spec の適合走行はこの逸脱を**検出しない**——本 spec が証明するのは「**emo2 適合**」であって「SSP 完全適合」ではない。
- **`\![move]` の2回目起動挙動（2026-07-17 合流裁定 E の申し送り・項目 #9/#12 の相互作用）**: `\![move]` は永続値を書かない（position-persist R1.9 二層分離・sakura-dialogue-tags brief が正本）。emo2 の `\![move]` は OnFirstBoot 限定ゆえ、初回ゲート（#12）導入後は**未ドラッグの2回目起動で初回位置調整が既定配置へ戻る**＝許容仕様として裁定済みだが、**適合走行時に開発者の実機判断で最終確定**すること（違和感があれば just-in-time の個別 spec で扱う）。

## ukadoc 必読（design 着手時に ukadoc MCP で正典参照）

- `list_shiori_event` の boot/close/入力/選択の各イベント（各上流 brief で裏取り済み・本 spec は**交信列の順序**を総覧で再確認）。
- **`Status` ヘッダ 9種**（scope doc §1）——一周中の遷移（talking/choosing）が交信列に正しく現れることを spine の檻に。
- OnUpdate*/OnBalloonChange が**任意（M2）**である根拠の最終確認（項目 #14）。

## Scope

- **In**: 決定論 conformance spine（1本・CI 常設）／実機14項目の適合走行手順＋記録様式／gap 仕分け運用／M1 完成 DoD の一本化と宣言／M-dual 検証吸収（#10）。
- **Out**: 発見欠陥の修正そのもの（just-in-time 個別 spec へ）／新機能・新 UI（全て上流ユニット）／里々/YAYA・Shift_JIS 等の生態系拡張（M1 後・scope doc §7）／NAR インストーラ・選択 UI・SSTP/FMO/Plugin/更新（M2 予約）。

## Boundary Candidates

- spine 台本（scripted backend 拡張＝決定論）／実機手順・記録（人間判断の様式化）／DoD ゲート（機械検証＋宣言）。

## Out of Boundary

- 各エンジンの内部品質（各 spec の檻が正本）——本 spec は結合と一周のみ。

## Upstream / Downstream

- **Upstream（全部）**: `cue-playback-duration`→`mayuna-compose`・`seriko-loop`・`sakura-dialogue-tags`→`choice-render`・`choice-select-events`／`position-persist`・`idle-talk`・`collision-geometry`・`input-events`／完了済み全ユニット（M-boot 23）。
- **Downstream**: **M2 ロードマップ再構築**（M1 完成宣言が起点・「実物を見て組み直す」）／生態系拡張（里々/YAYA・Shift_JIS・SAORI）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-ghost-setup`（spine e2e 母体）・`completed/areka-P0-emo2-boot`（spine＋実機サインオフ様式）。
- **Supersedes（吸収・退役）**: roadmap 増分の `areka-P0-dual-surface`・`areka-P0-dual-window`（M-dual→検証項目 #10 へ縮退・2026-07-16 裁定）。
- **Adjacent**: `doc/emo2-conformance-scope.md`（M1 実物定義の正本——本 spec 完了時に「充足済み」注記で閉じる。**§1 の OnChoiceSelectEx Ref0 記述の訂正**は choice-select-events design が実施）。

## Constraints

- Rust 2024・新規依存なし・tokio 不使用。
- **決定論 spine は実 pasta 非依存**（scripted backend・注入入力のみ・[[deterministic-test-coverage-mandate]]）／**実機走行は実 pasta・実 DPI・絶対パス起動**（[[areka-placement-real-ghost-first]]・MOD_NOT_FOUND 運用注意）。
- DoD: `cargo test --workspace` exit 0（[[workspace-test-needs-i686-host32-artifacts]]）＋License Gate＋実機14項目（人間サインオフ・AI 単独で完成宣言しない）。
- 正典は ukadoc・emo2 は最小適合 fixture（[[ukadoc-mcp-preferred-source]]）。

---

## 申し送り（areka-P0-draw-load-parity・2026-08-23）

`areka-P0-draw-load-parity`（W6.9）が tick の周期・構造に加えた変更の報告（同 spec 要件 8.3）。本 spec（W7・最終）は適合 14 項目の一周走行の前提としてこれを読み、実機走行時の環境変数と観測行の扱いを決めること。

**⑴ tick に「門」が入った（既定 OFF）**

- 画面更新 1 回ごとに、変化を示す旗が 1 つも立っていなければ 13 本のスケジュールを回さない、という判断を手前に挟む。判断は純関数 `tick_gate::should_run`（`crates/wintf/src/ecs/world/tick_gate.rs:154`）・つなぎは `EcsWorld::decide_tick`（`crates/wintf/src/ecs/world/mod.rs:551`）・入口は `tick_one_frame_with`（`crates/wintf/src/runtime/tick_bridge.rs:230`）。
- **既定は OFF**（`world/mod.rs:405` の `tick_gate_enabled: false`）＝門を入れる前と同じ挙動。`AREKA_TICK_GATE=1|0`（`crates/areka/src/tick_gate_config.rs:25`）で同じ実行体のまま切り替えられ、A/B 比較と安全弁を兼ねる。既定を ON にするかは改善ループの周 1 の A/B が決める。
- 必ず回す条件＝門が無効／起動から 600 回未満（`TICK_GATE_WARMUP_FRAMES`）／旗が 1 つでも立っている／期限到来／前回回してから 30 回（`TICK_HEARTBEAT_FRAMES`＝省略 30 回の次＝31 回目が心拍で回る・約 3.9 回/秒）。未知の窓メッセージは「疑わしいときは回す」側へ倒す。

**⑵ 省略した回に何が起きないか（前提が変わる箇所）**

- `FrameCount`／`FrameTime`／`TickStart` は**進まない**し、スケジュールは 1 本も回らない（`EcsWorld::note_skipped_tick`＝`world/mod.rs:593`）。フレーム番号や `FrameTime` を時間の代わりに読む観測・判定は、門が ON のとき意味が変わる。
- `flush_window_pos_commands()` は**省略した回も必ず呼ぶ**（`tick_bridge.rs:258`）＝窓書込指令の一括 flush の駆動は不変。13 本の順序と `try_tick_world` の中身も不変（門は手前にある）。
- 変化が生じたら次の画面更新までに反映する（遅れの上限は 1 画面更新周期＝120Hz の実機で約 8.3ms）。

**⑶ 旗を立てる側（起床の生産者・全て 1 行）**

- wintf: 窓メッセージ配送点（`ecs/window_proc/mod.rs`）・ポインタ投入（`ecs/pointer/buffers.rs`）・窓書込指令の積み上げ（`ecs/window/command.rs`）・Z 順（`ecs/window/zorder_pair_maintain.rs`）・ドラッグ（`ecs/drag/systems.rs`）・dola アニメ（`ecs/dola/mod.rs`）・GraphicsCore 無効（`ecs/graphics/systems/init.rs`）・表示構成の変化（`ecs/app.rs`）。
- areka: 表示指令の到着（`emo2_boot/adapter.rs`・`move_cue.rs`・`talk_lifecycle.rs`）・文字の進行（`emo2_boot/frame/scale_text.rs`）・バルーンの待ち時間（`emo2_boot/balloon_visibility_phase.rs`）・`emo2_boot/hover_inject.rs`。旗は `tx.send` の**後**に立てる。`sinks` の順序（clocked_text_sink → lifecycle_sink）を保つこと。

**⑷ 観測の口が増えた（いずれも既定 OFF・点けなければ費用 0）**

- `[tick] kind=window frame= t_ms= ticks= skipped= heartbeat= wall_us= max_us= ui_cpu_us=` ＋ 13 本の相別 `<相>_us=`（1 秒窓で 1 行・`crates/wintf/src/ecs/world/tick_diag.rs:133`・target `wintf::tick`）。省略率はこの行の `skipped=` で読む。
- `perf(thread)`／`perf(process)`（スレッド別・プロセス全体の CPU・`crates/areka/src/perf_thread_report.rs:51`・target `areka::perf`）。
- 既存の `perf(apply_show)`（末尾 `frame`）と `[transition]` の文言・フィールド名は不変で、新しい行とは重ならない。

**⑸ 一周走行での扱い**

- 走行時の `AREKA_TICK_GATE` を**明示して記録に残す**こと（未指定＝既定 OFF＝門を入れる前と同じ挙動）。門の既定がループの結果で ON へ変わる可能性があるため、「何も指定しなかった」だけでは後から条件を復元できない。
- 門が ON の走行では、フレーム番号を時間の代わりに使う判定（「N フレーム待って現れること」の類）が成立しない場合がある——放置中は 1 秒あたり 116〜118 回ぶんの画面更新が省略される実測がある。時間で待つか、上の `[tick]` 行の `ticks=`／`skipped=` を併記すること。
- 見た目の追随（クリック透過・αマスク・バルーン追従・Z 順）は門の ON/OFF で変わらないことを dlp 側で確かめているが、**実機の目視確認は本 spec の 14 項目が最終の関門**である。門 ON で 1 周する場合はその旨を記録に残すこと。
- `[tick]`／`perf(thread)`／`perf(process)` は既定 OFF なので、点けない限り一周走行のログ量は変わらない。点ける場合の target は上記のとおり。

**dlp の合否に載せない申し送り（憶測で埋めないこと）**: 遷移フレームのうち自前の窓手続きが 1 行も走っていない**未特定区間 47.5%**（639,106／1,344,271µs）と、文字層の再構築の所要。

**着地（2026-08-23・dlp タスク 9.4 で更新）**: dlp の改善ループは周 3 で頭打ち（plateau）となり STOPPED・**採用 0**。門の既定は **OFF のまま**（`crates/wintf/src/ecs/world/mod.rs` の `tick_gate_enabled: false`）、tick 構造（13 本の順序・実行器）は着地前と同じ、`Cargo.toml` は非接触。本節の file:line は dlp のタスク 1〜8 着地時点のまま有効。未達（⑵ catch-up・⑶・⑷a 22.3%）と残る最大項（段② `unregistered_rest` 51.8%）・引受先なし（新規 spec 要）は dlp の `requirements.md` 改訂欄に登記済み。

## 申し送り（areka-P0-test-cage-determinism・2026-08-27）

> 送り元は **2026-08-27 に完了・アーカイブ済み**。台帳の本体は `.kiro/specs/completed/areka-P0-test-cage-determinism/requirements.md` の `## 申し送り台帳`。

同 spec の申し送り台帳「⑶ タスク 12.1 の登記」の **B-1／B-2／B-3** を本 brief へ転記する。**転記の理由**——同 spec は「引受先が実在しない閉ループ」を 2 度作っている（要件 7.4 がその是正だった）。**台帳にだけ書いて受け手が知らない状態は 3 度目**になるので、受け手の側にも置く。

**B-1 再表示時の重なり順の再確認（実機が要る）**

決定論的なテストで固定できる範囲が無い。理由は 3 点で、いずれも実測で確かめてある——⑴ 再断行の要求を挿す箇所は `crates/wintf/src/ecs/window/zorder_pair_establish.rs:180` の **1 箇所だけ**で、確立時にしか通らない、⑵ 再表示の経路（`crates/areka/src/emo2_boot/balloon_visibility_phase.rs:446` → `crates/areka-emo-present/src/presenter/visibility.rs:69`）は再指示を**挿さない**、⑶ 実際の隣接の確認には**実窓**が要る（既定の IME 窓が owner の直上に居座るため、隣接は「最も近い**可視**の隣」で測らねばならず、生の 1 歩だと偽の失敗を記録する）。**本 brief `:41`・`:53` が既にこの残件を自ら宣言している**ので、受け手として矛盾しない。

**B-2 `areka-ghost` の間欠赤（本仕様の要件 4 と同型の構造欠陥）**

`crates/areka-ghost/tests/ghost/spine_e2e_test_s3_helper_liveness_detected.rs` の有界スピン（`:145-163`）が待つのは**記録が非空になることだけ**で、その直後の 5 呼出の数え上げ（`:175-180`）と判定 `assert_eq!(boot_prefix_len, 5, …)`（`:181-185`）には**待機が 1 つも無い単発のスナップショット**である。ソース内のコメントは「5 本は先に完了しているはず」と論じるが、それは**仮定であって強制ではない**。負荷下で 5 本目が間に合わないと `left: 4` で落ちる。`verification/logs/` の既存全ログでは常に緑で、**赤の記録は 2026-08-27 のタスク 10.1 が初**（`cargo test --workspace` の 1 回目・2 回目は全緑・単独 5 連走も全緑）。**本 brief `:104`／`:148` の DoD が `cargo test --workspace` の exit 0 を逐語で要求しており、この間欠赤はその DoD を間欠的に破る。**

**B-3 子プロセスへの受け渡しの実走未検証**

タスク 10.3 が `areka-ghost` の一時パス 13 ファイルを共通窓口へ寄せた際、タスク文が名指しした「宛先が変わっても子プロセスへの受け渡し（環境変数・引数）が壊れていないこと」は**静的検証止まり**である。実プロセスを起こす 2 本（`tests/ghost/real_pasta_test.rs`・`snapshot_capture_test.rs`）が環境変数の門で既定では走らないため。静的には確認済み——受け渡しの配線（`crates/shiori-host32-host/src/process_host.rs:239-251` の引数・環境変数・作業ディレクトリ）は非接触、札は英数と `-` のみ、絶対パス、最長の合成名でも約 129 文字。**実機サインオフの機会に 1 度通すこと**（`HOST32_PASTA_DLL` を絶対パスで与える。相対だと `pasta.dll` の LOAD が `0x8007007E` で落ちる）。
