---
inclusion: manual
updated_at: 2026-09-02
---

# Roadmap — areka M1（最小 SSP 互換ベースウェア）

> **このロードマップは M1 のみを扱う。** M2 以降は **M1 完成後に実物を見て組み直す**（憶測で先に書かない）。
> 正本配置: 本ファイルが M1 ロードマップ正本（`.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。M1 実物スコープは [doc/emo2-conformance-scope.md](../../doc/emo2-conformance-scope.md)。
> **履歴**: 追記①〜(51)・M-boot 完了詳報等は 2026-07-31 棚卸④、W5 詳報・旧ウェーブ表・旧干渉台帳は 08-01 棚卸⑤、W6（新 W7）行と詳報・追記(58)〜(64) は 08-14 棚卸⑧、旧 W6/W6.5 行・追記(65)〜(68) は 08-15 棚卸⑨、旧 W6.75 行・追記(69)(75) は 08-22 棚卸⑩、W6.9 行・追記(79)〜(81) は 08-27 棚卸⑪、**W6.5〜W6.95 の完了詳報バレット 8 本・旧ゴール表完了行・旧 W6.95 行 4 本・旧干渉台帳・追記(79)(82)〜(87) 全文は 2026-09-02 棚卸⑫で**、いずれも [roadmap-history.md](roadmap-history.md) へ退避（history が全文正本・既知の記録欠陥〔㊻番号衝突・(55) 欠番〕も history に注記）。完了ユニットの実装詳細は各 `completed/` spec が正本。
> **ウェーブ番号の振り直し（2026-09-02 棚卸⑫・開発者指示＝小数点番号を整数へ）**: 旧 W5.95→**W6**・旧 W6→**W7**・旧 W6.5→**W8**・旧 W6.75→**W9**・旧 W6.9→**W10**・旧 W6.95→**W11**・旧 W7（e2e）→**W12**。W1〜W5 は不変。history の退避全文と completed spec 内の記述は旧番号のまま（改変しない）——旧番号を見たらこの表で読み替える。

## M1 ゴール

areka（**x64**）が最小 SSP 互換ベースウェアとして、適合対象ゴースト **emo2**（作者自作・脳=`pasta.dll`・**32bit SHIORI**）を「**そのまま**」起動→会話→撫で→メニュー→終了まで E2E 実走させる。

- emo2 が動く＝同じ汎用 32bit ブリッジで里々/YAYA も動く土台（互換＝普及の入口）。
- 「伺かっぽいマスコット」ではなく「**伺か互換系**」であること自体が長期ロードマップの起点。
- M1 スコープは emo2 が実際に使う機能で**実物定義**。完全網羅・予測実装はしない。

## 実装規律（balloon-system の失敗から得た正）

- **実装ファースト**: 各作業ユニットの成果物は「emo2 が実際に動く」検証済みコード。
- **spec 工場の禁止**: 成果物が子 spec になる構造を作らない。1ユニット＝1かたまりの動く振る舞い。
- **最小実装＋薄い拡張シーム**: emo2 が使う分だけ実装し、拡張は型/レジストリの口だけ残す。抽象は「2例目の実物」が要求してから。
- **動く資産から建てる**: ゼロから再アーキテクトしない。
- **粒度基準**: 1ユニット＝単一 pass/fail の独立観測。純粋層は fixture/mock 直入力で切る。**UI 位置決め・座標系は逆＝本番ゴースト（実 emo2）＋実 DPI（≠96）が観測条件**（dpi=96 の自己整合が欠陥を隠す・記憶 areka-placement-real-ghost-first）。
- **⚠️ 語彙完備・配線ゼロの追跡**: 先送りシームには狭い `#[allow(dead_code)]`＋実在理由の doc を義務付け、消費者ゼロの検出は棚卸の定期監査項目とする（歴史的実例 5 件のうち `ZOrder` は PR#107 で実消費に転じた）。
- **1 ファイル 1,000 行**: 機械の番人 `crates/log-capture-kit/tests/file_length_guard_test.rs`（例外表 11 件・暗黙増加不可）。**どの spec も例外表に触れない**——新規ファイルは 1,000 行未満で作る（2026-09-02 実測: 超過 11 本＝例外表と完全一致・`areka-emo-text/src/draw.rs` は **980 行＝残 20 行**で番人の射程内）。

## アーキテクチャ横断原則（要約・詳細は history＋記憶＋completed spec）

- **シェル/バルーン統一**: 描画エンジンはシェルとバルーンを区別しない。バルーン＝surface 上の文字レンダリング層。element は他サーフェス参照可・配置は D2D 変換行列が内部表現。
- **アニメエンジンは2つ**: ①さくらスクリプト再生（sakura）＋②SERIKO ループ（seriko）。両者とも dola（絶対時刻台本・CuePlayer/CueSink）上。テキストは①から emo-text へ直接。
- **並行モデル**: 各エンジン＝チャンネル通信のアクター＋独立スレッド。**render/window は UI スレッド固定**。機構=areka-actor／経路=kanade／結線=ghost の責務三分。
- **emo 合成**: 自前コンポジタ（アトラス→1枚物合成→wintf へ完成品のみ）。emo=UI 層全般（合成・マウス/さわり・バルーン文字・選択肢）。
- **DPI 追従が基本設計**: k=monitorDPI÷author_dpi で全表示経路がスケール（SSP と別思想・k=1.0 は途中状態）。**キャラ窓の原点は下端中央**（保存/復元/resize の三層で統一・Bottom 限定）。**⚠️ バルーン追従は例外＝「窓（char 左上）相対」**（`kero-balloon` 裁定）。バルーン offset の単位空間は「表示 DPI の物理 px」1 つ（`balloon-offset-dpi`・`placement/follow/offset_space.rs` が権威）。
- **スコープ窓の重なり**: 既定は非強制、`\![set,zorder]` 後は **Windows の所有関係（owner）の一直線の鎖**で構造保証（`scope-zorder-pinning` 要件 14/15・全ゴースト窓が 1 本の鎖）。

**エンジン固有名**（コード/spec/会話の参照はこの名で統一・詳細は記憶 areka-engine-names）:

| # | エンジン | 固有名 | # | エンジン | 固有名 |
|---|---|---|---|---|---|
| ⓪ | ゴーストエンジン（最上位 owner） | `ghost` | ④ | さくらスクリプト再生 | `sakura` |
| ① | SHIORI 通信層 host-32 | `shiori` | ⑤ | SERIKO アニメ | `seriko` |
| ② | parser/loader | `parsers` | ⑥ | render（surface 合成＋UI 層） | `emo` |
| ③ | conductor（SHIORI イベント循環） | `kanade` | | | |

## 完了サマリ（2026-09-02 時点・新ウェーブ番号・詳報は completed spec ＋ history）

| Wave | 完了日 | ユニット | 一行要約 |
|---|---|---|---|
| 耐力壁 | 07-01 | `pilot-shiori-host-32` | x64→32bit pasta.dll 駆動 GO |
| M-boot | 07-13 | `emo2-boot` 23/23 | 起動→表示→talk→close の可視一周。①shiori・②parsers 全完了 |
| W1〜W4 | 〜07-29 | idle-talk／collision-geometry／sakura-dialogue-tags／input-events／mayuna-compose／sylphya／seriko-loop／choice-render／割込 wintf-gpu-test-crash／position-persist ∥ choice-interact ∥ emo-dpi-scaling（DPI 追従 k）＋横断 4 本 | 増分ウェーブ全完走 |
| W5 | 07-31〜08-01 | `choice-select-events`（M-dialogue 完走）・`kero-balloon`（PR#97・SSP 裁定 2 件）・`dpi-window-vanish`（PR#98） | 4/4（col は W7 へ編入） |
| W6 | 08-10 | `file-slimming`（PR#103） | 最大 8,472→986 行・1,000 行超 54→0 |
| W7 | 08-05〜08-13 | `collision-dpi-hittest`（PR#100）・`balloon-visibility`（PR#106）・`bindoption-exclusivity`（PR#105）・`ghost-window-zorder`（PR#107）・`scope-chain-gap`（PR#108） | 5/5 完走 |
| W8 | 08-14〜08-15 | `recompose-budget`（PR#112・1 コマ 22,210→1,240µs・CPU 24.9→11%）・`scale-exact-rational`（PR#110・+1 許容の裁定登記）・`windowposition-limit`（PR#111） | 3/3 完走 |
| W9 | 08-22 | `dpi-transition-atomicity`（PR#114・書込散らばり 93〜158ms→40〜101µs・**「FAIL だが GO」**） | 単独 |
| W10 | 08-23〜08-27 | `draw-load-parity`（PR#118・22.3% 対目標 3.0% 未達・門 ON 3.30% の手がかり→`tick-gate-adoption` へ）・`test-cage-determinism`（PR#119・`log-capture-kit`／`temp-path-kit`・5,894 passed） | 2/2 完走 |
| W11 | 08-28〜09-02 | `present-write-coherence`（PR#123・**見送り＋登記＝是正 0 行・未達 40 件は引受先なし**）・`balloon-vertical-canon`（PR#124・`vertical,0/1`・origin クランプ撤去）・`balloon-offset-dpi`（PR#125・単位空間 1 つ・実機合格）・`scope-zorder-pinning`（PR#126・改訂第 2 版＝所有の鎖・目視合格・**残件 9 件→`zorder-chain-residue`**） | 4/4 完走・**M1 残＝e2e のみ** |

- **実機サインオフ発見 7 件中 #1〜#6 解決済み**。#7（冒頭空行）のみ pasta 上流（`ekicyou/pasta` 起票済み）＝areka スコープ外。
- 完了 spec 直下エントリ＝**166**（`.kiro/specs/completed/` 直下・2026-09-02 実測）。計数は**直下エントリ数**で行うこと。
- 主な申し送りの生存先: W7 ⑴ **`ReassertZOrder` 未消費**（再表示直後のバルーン隣接は実機未確認）→ **e2e 着手時義務**。⑵ 配置系 spec は `window-placement` R2.9 を正典として引用しない（正典は COMPAT §8 経由で scg へ）。W9 atom→bod 等の追記(70)〜(78) は各 brief が正本。

## M1 残工程ゴール表（2026-09-02 棚卸⑫）

| 種別 | ゴール（単一文） | ユニット | ウェーブ |
|---|---|---|---|
| M-e2e | 適合 14 項目一周＋DoD＝**M1 完成宣言** | `emo2-conformance-e2e` | **W12**（最終・M1 唯一の残ユニット） |
| 挙動バグ（直接修正候補） | 角括弧なしの 2 文字 `\_X` タグ（`\_a`・`\_q`・`\_n`・`\_b`・`\_v`…）で lexer が `_` 1 文字しか消費せず **`X` が本文へ漏れる**（`lexer.rs:172-177`・テスト 0 本・emo2 非使用＝M1 適合には無害） | **`sakura-bare-tag-lexer`**（S・2026-09-02 起票＝開発者「spec が無いと開始できない」で直接修正から S spec へ）・意味付けの所有は `anchor-tag-canon`（M2）。W12 の先頭で着地させ、`decode.rs` を触る後続 spec の rebase 源にしない | ⓪（W12 開始前） |
| 挙動バグ（直接修正候補） | `writing.rs:311` の未知 `writing_mode` 警告文言が実挙動（`:32`「指定なしとして扱う」）とずれている（表示影響なし・ログ文言のみ） | 所有＝`balloon-canon-residue` 項目 12・**S・前提なし** | ⓪（任意） |
| 見た目（引受先なし） | 遷移中に絵と窓が同じ提示フレームで揃う（可視化→書込の隙間 0.21〜0.31 秒・未達 40 件） | **引受先なし**（pwc は見送り＋登記で完了・再着手は新規起票が要る） | M2 |

> 完了済みマイルストーンのゴール表は history 参照。M-dual は退役＝e2e 適合 #10 へ吸収。

## ウェーブ編成（着手順の正本・2026-09-02 棚卸⑫改訂・整数番号）

> 各ウェーブは**フルライフサイクル**（要件→設計→タスク→実装→`/kiro-complete`＝PR squash マージ）を完走してから次へ。並走はウェーブ内のみ（1 spec = 1 worktree = 1 PR）。同居は**実測で共有ファイル 0**が原則（記憶 prefer-clean-waves-over-max-parallelism）。文書フェーズは先行可＝先行 spec はウェーブ開始時に settled main へ再突合。優先順位: **挙動バグ → 依存ツリーが長く早期着手が効くもの → その他**。申し送りの詳細は各 brief の追記ブロックが正本（roadmap は編成と条件のみ持つ）。
> **M2 ゲートの扱い**: 「M1 では着手しない」が既定。ただし W11 の zsp／bvc と同じく **emo2 非依存で e2e をブロックせず、共有ファイル 0 が実測できる spec は開発者裁定で W12 以降へ前倒し可**。下表の W12 裁定枠はその候補であり、**e2e 以外は開発者の GO が要る**。

| Wave | ユニット | 開始コマンド | 編成根拠・条件 |
|---|---|---|---|
| W1〜W11 ✅ | 完了サマリ参照 | — | 旧行全文は history（棚卸⑧〜⑫退避） |
| **⓪**（W12 開始前・S spec） | **`sakura-bare-tag-lexer`**（`\_X` bare 漏れの lexer 修正・S・2026-09-02 起票）／residue 項目 12 の文言修正（任意・spec なし） | `/kiro-start areka-P0-sakura-bare-tag-lexer` | **挙動バグ最優先**。編集集合＝`lexer.rs`＋`decode.rs`＋テスト新設（`\_a`・`\_q` 等の bare 形と `\_a[...]` 形の決定論檻・現在 0 本）・意味付けは行わない。**着地後に `ukadoc-survey-sakura-script` を開始**（同ファイルへ URL コメント）。**W12 の channels と W13 の decoration が同じ `decode.rs` を触るため、先に着地させて rebase 源を消す**。着地後は `anchor-tag-canon` brief に「lexer 修正は消化済み」を登記 |
| **W12**（M1 最終＋裁定枠） | **`emo2-conformance-e2e`**（M1 必須） | `/kiro-start areka-P0-emo2-conformance-e2e` | 全ユニット完了（W11 完走で充足）。着手時義務: ⑴ brief 全面再監査（棚卸⑫追記ブロックが実測正本＝`spawn.rs` アンカー :294/:254 等の大ドリフト・dlp 申し送り 9 点は全命中）⑵ ㉘(E) の実機判断 ⑶ #7（pasta 上流）は M1 完成を妨げない扱いの確認 ⑷ **`ReassertZOrder` 未消費の実機確認** ⑸ **DoD `cargo test --workspace` exit 0 を脅かす間欠赤 3 系統の隔離裁定**（B-2 `spine_e2e_test_s3_helper_liveness_detected.rs:175-185`・zsp §13.8 ①②＝`zorder-chain-residue` A-1/A-2）——e2e は除外/更新の裁定のみ行い根治は `zorder-chain-residue`。**分割候補（開発者裁定）**: 決定論 conformance spine（CI 常設）／実機一周＋M1 完成宣言 DoD の 2 spec——brief 自身の Boundary Candidates と一致。本 spec は「証明に徹する」（`crates/` 本番コード改変 0 が原則） |
| **W12** 裁定枠 A | `cursor-tag-canon`（挙動バグ級・L・分割禁止裁定済み） | `/kiro-start areka-P0-cursor-tag-canon` | **現行 main の本物の非互換**＝縦書き `vertical_rl` で `\_l[0,0]` が 1 列目に着地しない（`layout.rs:453` 増加方向 対 `:309` 減少方向・`:620` 列右端＝構造で裏取り済み・縦書き `\_l` テスト 0 本）。上流 bvc 完了で前提充足。編集集合＝`areka-emo-text/src/{layout,state}.rs`＋兄弟テスト＋完了 spec `emo-text-layer` 縮退表＋COMPAT §8。**e2e／channels／toolkit と共有ファイル 0**（実測）。`draw.rs` には追記しない（残 20 行） |
| **W12** 裁定枠 B → **W14 へ移動（2026-09-02 再評価）** | `property-query-channels`（依存ツリー最長の先頭・XL・分割推奨＝⑴スクリプト経路／⑵IPC＋ライブ実測／⑶台帳 S） | （W14）`/kiro-start areka-P0-property-query-channels` | **W12 から外した理由**＝survey 4 本の doc コメント接触先（`sakura/{lexer,decode}.rs`・`consumer_ledger.rs`・`kanade/schedule`・`sylphya/vocab`）が本 spec の編集集合と一致＝原則（共有ファイル 0）で同居不可。W13 は decoration が `decode.rs` を持つため ⑴ を載せられない（⑵ IPC 片＝`shiori-host32-*`・`areka/shiori_host.rs` のみなら W13 同居可）。**利得**＝survey-property の所有突合表（188 項目・三重所有の裁定案）が先に出るので要件の材料が揃う。分割案・`PropSetCueSink` 雛形・三重所有は brief 追記(88) ブロックが正本。**要件定義は Fable** |
| **W12** 枠 C（**即時開始・2026-09-02 開発者指示**） | `ukadoc-survey-toolkit`（今すぐ）→ `ukadoc-survey-{shiori,assets,sakura-script,property}`（toolkit 要件確定後・4 本同時）（別セッション起票・追記(89)・調査道具＝新規 crate＋doc/ 台帳のみ） | `/kiro-start areka-P0-ukadoc-survey-toolkit` →（要件確定後）`/kiro-start areka-P0-ukadoc-survey-shiori` ∥ `-assets` ∥ `-sakura-script` ∥ `-property` | 既存 crate 非接触＝改変集合は新規 `crates/ukadoc-survey/`＋`doc/ukadoc-coverage/`＋`Cargo.lock` のみ（root `Cargo.toml` の members は `crates/*` glob＝行変更なし・**共有は `Cargo.lock` の機械マージのみ**・W12 の他 spec は Cargo.lock 非接触）。**survey 4 本（`ukadoc-survey-{shiori,assets,sakura-script,property}`）は toolkit の要件確定後に同時開始（実装完了を待たない・開発者方針「調査段階は並行実施」）**＝W12 内で段差開始・共有は各自の `doc/ukadoc-coverage/ledger/<domain>.toml` のみ＝0。`ukadoc-coverage-roadmap` は survey 4 本＋e2e の後（W13）。M2 機能 spec は起票しない（材料作り） |
| **W13**（裁定枠・W12 完走後） | `text-decoration-canon`（`\f` 族の先頭ゲート・XL・分割推奨＝基盤相／語彙相／descript 13 キー）∥ `property-query-channels` ⑵ IPC 片のみ（任意・`decode.rs` 非接触）∥ `ukadoc-coverage-roadmap`（第一段は 4 台帳が揃い次第・第二段は e2e 完走後） | `/kiro-start areka-P0-text-decoration-canon` 他 | decoration は **`draw.rs` 分割が着手前提**（980/1,000）・`balloon-canon-residue` 項目 9 と相互登記＝単独着地不可・cursor-tag と `layout.rs`/`state.rs` を共有するため **W12 の cursor-tag 完走後**。tree・catalog・zorder-property は channels⑴⑶ の後（`dotted.rs`／`emo2_boot/mod.rs` を共有＝直列） |
| **W14**（裁定枠） | `property-query-channels` ⑴⑶（`decode.rs` の唯一の持ち手）∥ `choice-marker-styling`（S・decoration 後・`decode.rs` 非接触なら同居可）∥ `surfaces-basepos`（S・独立・需要が出たら） | `/kiro-start areka-P0-property-query-channels` 他 | `decode.rs` は 1 ウェーブ 1 spec（⓪→W13 decoration→W14 channels⑴→W15 anchor）。**緩和案（開発者裁定）**＝doc コメント 1 行接触を非干渉扱いにすれば channels は W12/W13 へ前倒し可 |
| **W15**（裁定枠） | `anchor-tag-canon`（`"_a"` 腕・lexer 修正済みなら M）∥ `currentghost-property-tree`（channels 後・三重所有裁定後）∥ `property-catalog-lists`（島単位）∥ `zorder-property`（S） | — | tree／catalog／zorder-property は `dotted.rs` を共有＝design で所有分割（行単位・後着 rebase）を確認してから同居 |
| **M1 完成後・単独** | `tick-gate-adoption` | — | 夜間/25 分/n≥3 の実測環境が前提・**開発者方針「長時間試行禁止」と正面衝突する走行時間要求**＝始める前に決着可能な A/B 設計を組むこと・e2e と並走不可（M1 状態の証明を汚す） |

**干渉台帳（2026-09-02 棚卸⑫・W12 候補 4 本＋⓪の全ペア実測・旧 W6.95〔新 W11〕台帳全文は history）**:
- **⓪ lexer 修正 ⇄ W12 channels**〔**同一ファイル `lexer.rs`／`decode.rs`**＝同居不可。⓪ を先に着地させる（XS・1 PR）。channels が先に始まる場合は channels が lexer 修正を吸収する（anchor brief への登記義務は同じ）〕
- **e2e ⇄ cursor-tag**〔ファイル素（e2e＝`areka-ghost/tests/ghost/*` 新規＋`doc/emo2-conformance-scope.md`・cursor＝`areka-emo-text/src`）。ウォッチ事項なし〕
- **e2e ⇄ channels**〔crate 同居 `areka-ghost` だが tests/ 対 src/ で別居。**保存義務（channels 側）**: 本番 sink 列（`areka-ghost/src/runtime.rs:601` 近傍）への sink 追加は spine が数える sink 数・順序を変え得る＝e2e の檻が数を固定していたら channels が更新する（挙動不変の証跡として）〕
- **e2e ⇄ toolkit**〔ファイル素。toolkit の台帳が `doc/emo2-conformance-scope.md` を読む場合は参照のみ〕
- **cursor-tag ⇄ channels**〔ファイル素（emo-text 対 parsers/kanade/sylphya/emo2_boot/ghost）。COMPAT §8 は各自の節のみ追記・後着が rebase〕
- **cursor-tag ⇄ toolkit／channels ⇄ toolkit**〔ファイル素（toolkit は新規 crate＋doc/ 台帳＋`Cargo.lock` のみ・既存 crate と `doc/emo2-conformance-scope.md` は非接触）〕
- **survey 4 本 ⇄ channels（追記(89) ⑧・2026-09-02）**〔**同一ファイル接触**＝survey は定義箇所への ukadoc URL doc コメント 1 行/項目（挙動不変）・channels は機能編集。sakura-script⇄channels＝`areka-parsers/sakura/*`＋`areka/emo2_boot/consumer_ledger.rs`／property⇄channels＝`areka-sylphya/vocab/*`／shiori⇄channels＝`areka-kanade/schedule/*`。原則どおりなら同居不可（channels を W13 以降へ）・緩和案＝doc 1 行は後着 rebase 吸収（開発者裁定）。**survey ⇄ cursor-tag／e2e／toolkit は 0**（emo-text・tests/・新規 crate）・**survey ⇄ ⓪ lexer 修正**は `parsers/sakura/lexer.rs` で接触＝⓪ 先着で解消〕
- **共有の追記先は 3 つ**＝`doc/COMPAT_ARCHITECTURE.md` §8（各自の節のみ）・sylphya `vocab/dotted.rs`（W12 では channels のみ）・`file_length_guard_test.rs` 例外表（**誰も触らない**）。
- **⚠ 三重所有（着手前に 1 度で裁定・持ち越し）**: `currentghost.seriko.zorder` の実導出＝`zorder-property`（S・brief は「dotted.rs の 21 項に入れない・本 brief が語彙の正本」）／`currentghost-property-tree`（`seriko.*` 14 項の一括・zorder-property は真部分集合）／`property-query-channels`（SET 台帳 21→26 の追随に `seriko.zorder`・`seriko.sticky-window` を含む＝SET 経路の所有者として）。**推奨＝切り出し**: 値の導出は `zorder-property` 単独・tree は `seriko.*` から `zorder` を除外・**台帳行 1 本は channels⑶ が持つ**（zorder-property は台帳に触れない）。完了 spec zsp 要件 13.3/13.4 の追跡先は `zorder-property` 単独と記録済み（tree に言及なし）。
- **退役（棚卸⑫・W11 完走で消滅）**: pwc⇄bod・zsp⇄pwc・zsp⇄bod・bvc⇄zsp・bvc⇄pwc／bod（5 ペア・全文は history）。zsp の起床旗保存義務は所有の鎖への改組で消化済み（`tick_wake.rs` 差分 15 行・PR#126）。

## 着手手順

> **brief 全数完備体制**: M1 残ユニット 1 本（e2e）＋M2 ゲート **14 本**（棚卸⑫で `zorder-chain-residue` 合流）＋調査系 **6 本**（別セッション起票・追記(89) 反映後に実在）＋⓪ `sakura-bare-tag-lexer`（S・09-02 起票）＝全 22 本 brief 済み＝着手は該当 brief を読んで `/kiro-start <unit>` へ直行。新規課題の起票は `/kiro-discovery`（再入）で brief just-in-time 生成。`/kiro-spec-batch` は使わない（一括＝工場化）。ウェーブ跨ぎの合流判断は別セッションで一括（記憶 portfolio-convergence-decided-in-separate-session）。

## 制約

- Rust 2024・マルチクレート（24 crate・一覧は structure.md）。
- **32bit 可搬性の適用範囲＝host-32 系（`shiori-host32-*`／`shiori-abi`）のみ**。wintf/areka 本体は x64＋arm64 ネイティブ。
- 透過は WUC/DComp GPU 合成上のクリックスルー機構（`WS_EX_TRANSPARENT` 動的トグル＋αマスク）で成立（ULW は撤去済み）。SHIORI 内部唯一 ABI=`IShiori`(COM, HSTRING/UTF-16)。過去互換は 32bit Rust ホスト。
- 設計判断の変更は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md) を正本として更新。
- 実機運転の定石: 絶対パス起動（相対は pasta.dll LOAD 失敗）・i686 helper を先ビルド・`AREKA_APP_SMOKE_EXIT_MS` 有界自動終了＋`RUST_LOG` grep（記憶 areka-real-machine-signoff-bounded-auto-exit）。

## M2 以降

**M1 完成後に、実物を見て組み直す。** 本ロードマップでは扱わない（pasta の native x64・`IShiori` in-proc 化、ベクトル描画・AI、owner-draw 右クリック system メニュー、互換面拡大＝Shift_JIS/SAORI/里々・YAYA 網羅/NAR 等はその時に）。組み直しの材料＝別セッション起票の `ukadoc-survey-*` 5 本→`ukadoc-coverage-roadmap`（追記(89)）。

**アプリ層の M2 予約（2026-07-05 ukadoc 裏付け・全て任意）**: SSTP ポート（9801）・FMO・DirectSSTP・Plugin/HEADLINE/SAORI・ネットワーク更新・ゴースト/バルーン選択 UI・多重ゴースト運用。
**emo テキスト進化の予約**: ①回転テキストの実挙動 ②文字装飾（→ `text-decoration-canon` が具体着地先）。
**バルーン美観配置政策の予約**（`dpi-window-vanish` task 6.2 先送り登記）: 画面端での左右反転等。縮退シーム＝`[visibility-guard] ClampX` の `warn!` の発火回数が優先度根拠。M1 では起票しない。

**M2 解禁ゲートの spec（brief 済・着手は開発者裁定・依存順）**:
- 台帳系: `status-execution-states`（残状態の源着地時に just-in-time・**brief の Current State は陳腐化＝`choosing` は実導出済み・`balloon`／`minimizing` の源は今日実在＝追記(88) 参照**）／`balloon-canon-residue`（12 項目・**XL＝3 軸分割推奨**〔系列 1〜6／表示寿命 7〜10／bvc 残 11〜12〕・項目 12 は S 単独）／**`zorder-chain-residue`**（棚卸⑫起票・zsp 残件 8 件・A 群 2 件は e2e DoD の間欠赤）。
- 互換拡充時: `surfaces-basepos`（S・ドリフト 0）／`sakura-time-directives`（L・段階 A/B/D→C）。
- 性能: `tick-gate-adoption`（M1 完成後単独）。
- カーソル: `cursor-tag-canon`（W12 裁定枠 A）。
- プロパティ系 3 本: `property-query-channels`（W12 裁定枠 B・分割推奨）→ `currentghost-property-tree`（L・balloon.scope 19 は先行スライス可）→ `property-catalog-lists`（島単位）。＋ `zorder-property`（S・三重所有裁定後）。
- 文字装飾系 3 本: `text-decoration-canon`（W13・分割推奨）→ `anchor-tag-canon`（lexer バグは ⓪ で前倒し可）＋ `choice-marker-styling`（S）。
- 調査系 **6 本**（別セッション起票・追記(89)・開発者追記で script-property を 2 分割）: `ukadoc-survey-toolkit` →（要件確定後・同時開始）`ukadoc-survey-{shiori,assets,sakura-script,property}` → `ukadoc-coverage-roadmap`（二段＝第一段は台帳が揃い次第・第二段は e2e 後）。優先度は 4 軸（壊れ方＞**伺からしさ**〔テーマ 8＝気配／触れ合い／掛け合い／装い／記憶／交わり／気配り／更新〕＞資産の広さ＞基盤共有度・追記(90)）。

**要件定義段階で深掘り（Fable）を推奨する spec**: `emo2-conformance-e2e`（DoD の定義・間欠赤の隔離裁定・分割裁定・14 項目×二層観測の設計）・`property-query-channels`（6 経路・輸送路のライブ実測・三重所有・分割）・`text-decoration-canon`（design 段階＝`draw.rs` 分割と per-run 属性の 3 層配管）。Opus で足りる: `cursor-tag-canon`（語彙は採取済み・符号の非互換は構造で裏取り済み）・`zorder-property`・`choice-marker-styling`・`surfaces-basepos`・residue 分割後の各片。

---

**追記台帳（要約・全文は history・**ウェーブ番号は起票当時の旧番号のまま**＝冒頭対応表で読み替え）**: (51)〜(78) は棚卸⑪版と同じ（history 参照）／(79) 1,000 行番人＝cage が解決・棚卸⑪初回登記の誤認を同日是正（08-22／08-27）／(80)(81) 棚卸⑩＝atom 完走後の再解決・bod 優先度低で W6.9=cage ∥ dlp・W6.95=pwc ∥ bod（08-22）／(82) `scope-zorder-pinning` 起票＝`\![set,zorder]` バルーン込み・W6.95 を 3 本へ（08-27）／(83) `balloon-vertical-canon` 起票＝縦書き SSP 正典化・W6.95 を 4 本へ・ukadoc-mcp スナップショットの陳腐化を実証（08-27）／(84) 棚卸⑪＝W6.9 完走後の再解決・`tick-gate-adoption` 起票・steering 同期（08-27）／(85) `cursor-tag-canon` 起票＝`\_l` 全仕様一括・つまみ食い禁止（08-27）／(86) プロパティ系 3 spec 起票＝照会経路 vs 値の木・二重所有の登記（08-27）／(87) 文字装飾系 3 spec 起票＝`\f` 族 43 項目・`\_a` lexer バグ発見（08-27）。

**2026-09-02 追記(88)（棚卸⑫＝W11 完走後の全面再解決・ウェーブ番号整数化・`zorder-chain-residue` 起票・W12 編成）**: `/kiro-discovery` 再入。前回⑪以降の main 差分＝W11 完走（pwc PR#123・bvc PR#124・bod PR#125・zsp PR#126）＋wintf 直接コミット 4 本（visual draw command builder／visual bounds and draw modes／hash support ×2＝`wintf/ecs/visual/**`・`numerics/aabb.rs`・**全 14 brief の編集集合と交差 0**）。①**brief 14 本のアンカーを機械＋目視で全数再測定**（サブエージェント 2 体・各 brief の追記ブロックが実測正本）——ドリフトは登記時期で割れる: 08-2x 以降の申し送り（dlp 9 点・tick-gate・choice-marker・basepos・time）は全命中、07 期の e2e 本文（`spawn.rs` :164→:294／:150→:254・`command.rs:229`→:512-516・`transition_diag.rs` :167→:193／:143→:169）と decoration の `canvas.rs`/`draw.rs`（+6〜+15）は要 rebase。事実誤認 1 件＝decoration brief「`DWRITE_TEXT_RANGE` は `viewbox_draw.rs` の 1 箇所」は誤り（`wintf/ecs/widget/text/typewriter_draw.rs:245,:259` にも 2 件）。②**現行 main の挙動バグを 3 件確定**（いずれも檻 0 本）＝⑴ `\_X` bare 漏れ（`lexer.rs:172-177`・`\_a[id]text\_a` は「text**a**」と表示・全 2 文字 `\_` 系 bare 形に一律）⑵ 縦書き `vertical_rl` の `\_l[0,0]` 列ずれ（cursor-tag 所有・分割禁止）⑶ `writing.rs:311` 文言ずれ（residue 12）。＋DoD を脅かす間欠赤 3 系統（e2e B-2・zsp §13.8 ①②）。③**所有者ゼロを 2 件解消**——zsp 残件 9 件中 8 件→新規 `zorder-chain-residue`（台帳）・#7（COMPAT §8 の `roadmap.md:132` 引用 5 行が空行を指す）→ **doc 側を直接是正**（`doc/COMPAT_ARCHITECTURE.md:160-165` を台帳 spec／residue 項目番号への引用に差替）。④**三重所有の発見**（二重ではない）＝`seriko.zorder` の SET 台帳を channels も所有主張（推奨＝切り出し・台帳行は channels⑶）。⑤**分割推奨 4 本**（brief に継ぎ目を登記・裁定は開発者）＝channels（⑴⑵⑶）・residue（3 軸）・decoration（基盤／語彙／descript 13 キー）・e2e（spine／実機 DoD）。anchor は lexer 修正を ⓪ へ切り出せば L→M。⑥**status-execution-states の brief 陳腐化**＝`choosing` 実導出済み（`kanade/status.rs:171-174`）・`balloon`/`minimizing` の源は実在・区切り文字 `/` 対 `,` の互換裁定が台帳表に不在（追記登記済み）。⑦**ウェーブ番号整数化**（冒頭対応表）と roadmap 減量＝完了サマリ 8 バレット→1 表・旧 W6.95〔新 W11〕行 4 本・干渉台帳・追記(79)(82)〜(87) 全文を history へ（68.9KB→約半減）。⑧**W12 編成**＝e2e（必須）＋裁定枠 3（cursor-tag／channels ⑴⑶／別セッションの toolkit）・⓪ 直接修正 2 件を先頭に。全 6 ペア＋⓪ の共有ファイルを実測＝⓪⇄channels の `lexer.rs`/`decode.rs` のみ衝突（⓪ 先着で解消）。⑨ 別セッション（`claude/areka-spec-discovery-a8843a`）が同時に ukadoc 調査 spec 5 本を起票中＝分担協定（そちらは追記(89) 1 ブロックのみ・本文行は本棚卸が所有・後着が rebase）。⑩ 記憶索引の陳腐化 1 件（`bindoption-exclusivity` が「完了待ち」表記＝PR#105 で完了済み）を是正。

**2026-09-02 追記(89)（ukadoc 網羅調査 5 spec 起票＝M2 ロードマップ「組み直し」の材料作り・棚卸⑫と同時進行のため本ブロックのみ追記）**: `/kiro-discovery`（Path D・開発者要望「ukadoc を読み込み SHIORI Event/Resource・プロパティ・Install/Update・nar 等を網羅的に分類し、繋がりを評価し、製品品質に必要な順に実装項目を洗い出す調査 spec を作れ。網羅調査の仕組みは Rust で作ることも検討。大規模なら分割」）。①**規模実測**——ukadoc スナップショット（`ukagaka-doc-mcp` 2026-08-24 生成・ローカル JSON）＝**1,749 項目／37 ページ／本文 364K 字**（shiori_event 637・descript 518・sakurascript 342・protocol 237・file_structure 8・dev_guide 7）。MCP 検索は 50 件上限・ページング無し＝網羅列挙にはスナップショット直読みの道具が要る。②**areka 側実測**——正典側の機械可読資産は既に 2 系統（`doc/shiori/fragments` 446 entry・sylphya 語彙表 26/10/17/21/159）ある一方、実装側は送出イベント 11／照会リソース 1／`\!` 消費者 4／ghost descript 7 系統／balloon descript 29／install・update・nar・SSTP・FMO・SAORI・HEADLINE・PLUGIN は 0。未知 descript キーは無言で捨てる（`kv/parse.rs`）。③**起票 5 本（brief のみ・着手しない・M2 ゲート扱い）**——`ukadoc-survey-toolkit`（唯一コードを書く＝新規 crate `ukadoc-survey`＋`doc/ukadoc-coverage/` 台帳・catalog／evidence／ledger／check・本文は同梱せずハッシュのみ）→ `ukadoc-survey-shiori`（677 項目）∥ `ukadoc-survey-assets`（542）∥ `ukadoc-survey-script-property`（530・既存 M2 ゲート 13 brief との突合＝無所有項目の炙り出し）→ `ukadoc-coverage-roadmap`（繋がり評価・製品品質の段階 A〜E・優先順ブリーフィング・M2+ ロードマップ草案＝`doc/ukadoc-coverage/` のみ・steering roadmap 本文と候補 spec の brief は作らない＝spec 工場化しない）。④**編成の提案**（棚卸⑫セッションへ送付済み・裁定は同セッション）——toolkit は e2e（W12）と共有ファイル 0（新規 crate＋doc のみ）・survey 3 本は toolkit 後に並走（台帳ファイル別＝共有 0）・coverage-roadmap は **e2e 完了後**（段階 A の起点＝M1 の実物）。⑤**方針との関係**——「M2 以降は M1 完成後に実物を見て組み直す」は不変。本 5 本はその組み直しの材料であり、M2 ロードマップ本文化は coverage-roadmap 完了後の `/kiro-discovery` 再入（別セッション一括裁定）で行う。brief 5 本＝`.kiro/specs/areka-P0-ukadoc-{survey-toolkit,survey-shiori,survey-assets,survey-script-property,coverage-roadmap}/brief.md`。⑥**棚卸⑫（追記(88)・別セッション `claude/epic-kepler-bdbee8` 8b71490d）との突合**——spec 名は (88) の登記どおり（`areka-P0-ukadoc-survey-toolkit`／`-survey-shiori`／`-survey-assets`／`-survey-script-property`／`-ukadoc-coverage-roadmap`・修正不要）。「着手手順」節の brief 本数は **M2 ゲート 14 本＋調査系 5 本＝計 19 本**。toolkit の改変集合＝新規 `crates/ukadoc-survey/`＋`doc/ukadoc-coverage/`＋`Cargo.lock`（root `Cargo.toml` は `crates/*` glob ゆえ members 行の変更なし・既存 crate と `doc/emo2-conformance-scope.md` は非接触）＝W12 裁定枠 C の共有ファイルは `Cargo.lock` の機械マージのみ。本ブランチは後着＝PR 前に origin/main（棚卸⑫合流後）へ rebase し、本ブロックを (88) の直後へ載せる。⑦**開発者追記（同日）による改訂＝5 本→6 本**——「さくらスクリプトも大事。ukadoc をとにかく網羅的に一度調査して仕訳すべき。最新仕様を優先し、古い仕様に新しい書式があれば新書式を優先して旧書式はエイリアスとする。網羅調査→仕訳と関連の検索→優先度決定と開発仕様立ち上げ→ロードマップ調整。調査段階は並行実施できるように」。⑴ `ukadoc-survey-script-property` を分割＝**`areka-P0-ukadoc-survey-sakura-script`（342・全数調査へ格上げ）＋`areka-P0-ukadoc-survey-property`（188・所有突合と二重所有裁定案）**（(88) の本文行の spec 名はこの 2 本へ差し替え・調査系は **6 本**・brief 本数は **M2 ゲート 14＋調査系 6＝計 20 本**）。⑵ **仕訳の規則を toolkit brief で凍結**（最新優先・新書式正典・旧書式 `alias`〔`alias_of`〕・版番号＝世代・種別付き links〔alias_of／supersedes／triggers／configures／queries／same-feature〕・状態語彙 7 種）。⑶ **並行条件**＝survey 4 本は toolkit の**要件確定（台帳形式の凍結）後に着手可・実装完了を待たない**（検査は後から追いつく）。⑷ 段③④＝coverage-roadmap 完了時に先頭ウェーブ分の開発 spec brief を `/kiro-discovery` 再入で起票し、roadmap 反映は棚卸セッションで一括裁定（先の束は名前付きのまま＝spec 工場化しない）。⑧**ブリーフィング段階の裁定 4 件（同日・チャットで 1 議題ずつ）**——⑴ 実装済みの証拠＝**ソースに置いた ukadoc の URL**（`/// ukadoc: <url>`・定義箇所のみ・1 項目 1 行・語彙表は頭にページ URL 1 つ・未実装には書かない・行番号と内部 ID は使わない＝「ソースが第一聖典」の向きで機能逆引きを可能にする）。⑵ 台帳の整合検査は**常時 `cargo test --workspace`**（純粋決定論・スナップショット不在でも緑・赤は URL の綴り違いと消失のみ）。⑶ coverage-roadmap は**二段**＝第一段（統合・束・仮順位）は台帳が揃い次第で e2e を待たない／第二段（順位確定・開発 spec 起票・roadmap 反映）は e2e 後。⑷ 「使用頻度」の参照元と段階 A の検証対象＝当面**里々／YAYA の標準テンプレート辞書**・実在ゴーストの指定は「このゴーストを動かして」という要望が来た時点で切替（候補＝開発者自作「どっとさくら」）・温度感は「当面 emo2 が動けばよい」。併せて仕訳規則を補正＝版番号なしは「世代不明」（最古と決めつけない）・alias の向きは本文注記→版番号→人手。survey 4 本の唯一のコード接触は URL の doc コメント（実行時挙動不変）。

**2026-09-02 追記(90)（ukadoc 調査の優先度軸に「伺からしさ」を追加＝開発者指針・議題 5〜7 を裁定・brief 6 本へ反映）**: `/kiro-discovery` 再入（開発者「toolkit 起点の ukadoc 調査の優先順位付けに『伺からしい価値観を持つ要素について優先度を上げる』を加えよ・深掘りして brief を洗練・議題は 1 つずつチャット」）。①**診断**——既存の優先度根拠 3 つ（壊れ方・資産の広さ・基盤共有度）は全て互換工学の軸で、「伺かとして何が失われるか」を測る軸が無かった。裁定(89)⑷の参照元（標準テンプレート辞書）は「よく使う」を測り「象徴的だが稀」を拾えない＝新軸の守備範囲。新旧が食い違う実例＝消滅 OnVanish 系 5（既存軸 低／新軸 高）・OnFileDrop2（キャラに物を渡す）・時報 OnHourTimeSignal／OnMinuteChange・コミュニケート OnCommunicate。②**議題 5＝テーマ正典 8 つ**（`doc/ukadoc-coverage/values.md`・toolkit 規則 9 で凍結）＝気配／触れ合い／掛け合い／装い／記憶／交わり／気配り／**更新**（開発者「ネットワーク更新に関する軸は優先すべきかも」で「記憶と成長」から独立＝OnUpdate 系 24・`\![updatebymyself]`・updates2.dau・`*.refresh`・delete.txt・`homeurl`＝ゴーストが自分で育ち更新を自分の台詞で語る・**新旧両軸で高い唯一の群**）。付与規則＝「この項目が無いと利用者はゴーストの何を失うか」に答えられるテーマだけ。テーマ 0＝開発者機能・Ex 168・トランスレータ・ヘッドライン・照会配管（`%property`／`system.*`）・areka 自身の更新（別軸）。「交わり」は同格（基盤の重さは基盤共有度が担う）。③**議題 6＝序列と上げ方**＝**壊れ方 ＞ 伺からしさ ＞ 資産の広さ ＞ 基盤共有度**（壊れ方＝「利用者が気づけるか」・黙って壊れるは「間違った結果を正常な顔で見せる」を含む）。テーマ 1 つ＝同段階内先頭／2 つ以上＝段階 1 つ繰り上げ可／テーマ 0 かつ見た目差以下＝E 候補。台帳行に `values[]`・検査＝テーマ名の実在・report に「テーマ別の状態分布」節。④**議題 7＝段階 A〜E の切り直し**＝段階は「利用者が体験できる節目」で名付け（A そこにいて触れて話す／B 迎えて育てて見送る／C 察してくれる／D 仲間がいる／E 周辺）・束は機構で切る・テーマが束→段階を決める。移動＝時報・OnUserInput を C→A／OnVanish・ファイル D&D を B／ヘッドラインを E／`system.*` は C 末尾。⑤**SAORI の確認**（開発者質問「areka は SAORI を実装しなくて良いか」）＝**実装しない**が正（SHIORI が `LoadLibrary` して直接呼ぶ・`doc/COMPAT_ARCHITECTURE.md:87`「同 32bit プロセスに同居」）。台帳は `not-applicable`・note に host-32 の同居条件（32bit 同一プロセス・cwd `ghost/master`・DLL 検索パス）・段階 A の検証項目として links。棚卸⑫の口頭「SAORI が A〜B 相当に上がる」は撤回。⑥**M3「伺かの冠」の受入基準候補**＝「テーマ 8 つ全てで代表束が実装済み」を coverage-roadmap の roadmap-draft に登記（マイルストーン階梯の裁定は棚卸セッション）。⑦**反映先**＝toolkit brief（規則 6・9・台帳行・検査⑸・values.md）／coverage-roadmap brief（段階表・4 軸・SAORI・M3）／survey 4 本の追記（テーマの代表 id）。**toolkit の要件確定前に入れたので台帳形式に乗る**。ウェーブ編成・干渉台帳は不変（変更は brief 本文のみ）。
