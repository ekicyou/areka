---
inclusion: manual
updated_at: 2026-08-15
---

# Roadmap — areka M1（最小 SSP 互換ベースウェア）

> **このロードマップは M1 のみを扱う。** M2 以降は **M1 完成後に実物を見て組み直す**（憶測で先に書かない）。
> 正本配置: 本ファイルが M1 ロードマップ正本（`.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。M1 実物スコープは [doc/emo2-conformance-scope.md](../../doc/emo2-conformance-scope.md)。
> **履歴**: 追記①〜(51)・M-boot 完了詳報等は 2026-07-31 棚卸④で、W5 完了詳報・旧ウェーブ表・旧干渉台帳・追記(53)(54)(56)(57) 全文は 2026-08-01 棚卸⑤で、**W6 行全文・W6 完了詳報バレット（col/vis/scg）・追記(58)(60)(61)(62)(63)(64) 全文は 2026-08-14 棚卸⑧で**、**W6 完了詳報バレット全文・旧 W6/W6.5 行全文・旧干渉台帳・追記(65)(66)(67)(68) 全文は 2026-08-15 棚卸⑨で**、いずれも [roadmap-history.md](roadmap-history.md) へ退避（history が全文正本・既知の記録欠陥〔㊻番号衝突・**(55) 欠番**〕も history 冒頭と棚卸⑤節に注記）。完了ユニットの実装詳細は各 `completed/` spec が正本。

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
- **⚠️ 語彙完備・配線ゼロの追跡**: 語彙先送りの規律は効いているが配線の追跡が弱い（実例 5 件: `compose_into`・`ZOrder`・`PlacementRoute::SpawnInitial`/`Restore`・`capture_logs` 複製増殖）。先送りシームには狭い `#[allow(dead_code)]`＋実在理由の doc を義務付け、消費者ゼロの検出は棚卸の定期監査項目とする。

## アーキテクチャ横断原則（要約・詳細は history＋記憶＋completed spec）

- **シェル/バルーン統一**: 描画エンジンはシェルとバルーンを区別しない。バルーン＝surface 上の文字レンダリング層。element は他サーフェス参照可（入れ子・再帰合成）・配置は D2D 変換行列が内部表現。
- **アニメエンジンは2つ**: ①さくらスクリプト再生（talk timeline・sakura）＋②SERIKO ループ（seriko）。両者とも dola（絶対時刻台本・duration 権威・CuePlayer/CueSink）上。テキストは①から emo-text へ直接。
- **並行モデル**: 各エンジン＝チャンネル通信のアクター＋独立スレッド。**render/window は UI スレッド固定**。機構=areka-actor／経路=kanade／結線=ghost の責務三分。
- **emo 合成**: 自前コンポジタ（アトラス→1枚物ビットマップ合成→wintf へ完成品のみ）。emo=UI 層全般（合成・マウス/さわり・バルーン文字・選択肢）＝「見える・触れる」の窓口。
- **DPI 追従が基本設計**: k=monitorDPI÷author_dpi で全表示経路がスケール（W4 で実装済み・SSP と別思想・k=1.0 は途中状態）。**キャラ窓の原点は下端中央（足元の中心）**——**キャラ窓位置の保存/復元/resize の三層**で統一済み（Bottom 限定）・左上基準で計算しないこと。**⚠️ バルーン追従は例外＝「窓（char 左上）相対」**（2026-07-31 実機裁定・`kero-balloon`）——全アンカーで `balloon_pos − char_pos ≡ offset` 不変・リサイズで offset を補正しない・保存も生 offset。旧記述「四層で統一」は誤り（詳細は history）。

**エンジン固有名**（コード/spec/会話の参照はこの名で統一・詳細は記憶 areka-engine-names）:

| # | エンジン | 固有名 | # | エンジン | 固有名 |
|---|---|---|---|---|---|
| ⓪ | ゴーストエンジン（最上位 owner） | `ghost` | ④ | さくらスクリプト再生 | `sakura` |
| ① | SHIORI 通信層 host-32 | `shiori` | ⑤ | SERIKO アニメ | `seriko` |
| ② | parser/loader | `parsers` | ⑥ | render（surface 合成＋UI 層） | `emo` |
| ③ | conductor（SHIORI イベント循環） | `kanade` | | | |

## 完了サマリ（2026-08-14 時点・詳細は completed spec ＋ history）

- **耐力壁突破**（2026-07-01 `pilot-shiori-host-32`）: x64→32bit pasta.dll 駆動 GO。
- **M-boot 23/23 完了**（2026-07-13 `emo2-boot`）: 起動→表示→talk→close の可視一周。①shiori・②parsers トラック全完了。
- **増分ウェーブ W1〜W4＋割込 全完走**: W1〜W3（idle-talk／collision-geometry／sakura-dialogue-tags／input-events／mayuna-compose／sylphya／seriko-loop／choice-render）→ 割込（wintf-gpu-test-crash）→ W4（position-persist ∥ choice-interact ∥ emo-dpi-scaling＝DPI 追従 k の全表示経路適用）。横断: cue-playback-duration・surface-resize-resnap・balloon-face-cue・emo-text-viewbox ✅。
- **W5 完了 4/4**（col は W6 へ編入）: `choice-select-events` ✅（07-31・**M-dialogue 4/4 完走**・実 DPI 120 人間サインオフ）・`kero-balloon` ✅（08-01・PR#97・**SSP 裁定 2 件**＝`windowposition.x` 符号非反転／**バルーン追従は窓相対**）・`dpi-window-vanish` ✅（08-01・PR#98・S1〜S4 是正・実機全判定 PASS・確定台帳の裁定付き訂正運用が実例確立）。詳報バレット全文は history。
- **W5.95 完了**: `file-slimming` ✅（08-10・PR#103）＝ソース肥大の全域是正。**最大 8,472→986 行・1,000 行超 54→0 本・ファイル内テスト 500 行超 49→0 本**・三不変量（テスト総数 4,790 不変・本文一致 61/61・対応表全単射）成立。一度 GO を撤回して群 8 を追加した経緯は `completed/areka-P0-file-slimming/verification/notes.md` §39/§54。
- **W6 完了 5/5**（2026-08-05〜08-13・**完走**）: `collision-dpi-hittest` ✅（PR#100・当たり判定 ÷k＝丸め権威 `unscale_coord`・実 DPI 2 水準サインオフ）・`balloon-visibility` ✅（PR#106・可視性を presenter 第一級概念へ＝空バルーン根治・残件は `balloon-canon-residue` へ送付済み）・`bindoption-exclusivity` ✅（PR#105・表情固着根治＝bindoption 3 値正典＋保持コマ掃除 3 段・`mayuna-compose` R4.5/D11 覆し）・`ghost-window-zorder` ✅（PR#107・案 A Win32 owner 構造保証・topmost 帯引き込みは `HWND_TOP` へ是正）・`scope-chain-gap` ✅（PR#108・P2 幅差隙間根治＝SSP 実測 8 本正典＋一度きりの `finalize_chain_once`）。詳報バレット全文は history（棚卸⑨退避）。
- **W6.5 完了 3/3**（**完走**・2026-08-14〜08-15）:
  - `recompose-budget` ✅（08-15）＝**常駐アイドルの CPU 消費の決着**。要件 3.1「定常アロケーション完全ゼロ」を実機で成立（4 発生点すべて 0・長時間 1503 適用/1425 秒でも 0）。**1 コマ適用 22,210 → 1,240µs（18 分の 1）**・p95 77,880 → 7,152µs・**CPU 24.9% → 約 11%**。四段＝定常アロケーション 0／冗長ゼロ埋め除去／リサンプル計算の作り直し（乗算 24→12・範囲検査 16→4・恒等性は横方向 42.9 億組の**全数検査**で確定）／**キャッシュ容量 1 → 3・LRU（開発者裁定・上流 R4.1 まで追随改訂）**。**メモリリークと負荷の単調上昇はいずれも実測で消滅**（Private 12→24 分で +0.1MB・回帰の傾き +0.84 → −0.045 %/分）。**ビルド設定は裁定済＝`opt-level='z'` 据え置きが正しい**（是正後は O3 が 26〜33% *遅い*）。⚠**最大の成果は「どこを削っても無駄か」を確定させたこと**——`apply_show` は着手前でも CPU の 10.4%・現在 3.3% しか占めず、真の最大項は `try_tick_world` が 13 スケジュールを 120回/秒 全部回していること（1 tick 578µs・**tick の 98% は表示に変化なし**）。**SSP との同一手順比較を初実施**（areka 10.97% 対 SSP 3.05%）し、要件 4.4 の出所不明だった較正値に初めて裏を取った。残る未達（⑵ 進行境界スキップ・⑷a CPU）は **`draw-load-parity`（W8・優先度低）へ引受け済み**。
  - `scale-exact-rational` ✅（08-14・PR#110）＝**f32 供給面寸の +1 を許容する裁定の登記**。厳密化（`ScaleRatio` num/den の文字層配管）は**却下**され、着地物は⑴既知欠陥登記を裁定済みへ書き換え⑵「寸法演算に f32 を使わない」絶対規則の 4 箇所へ唯一の例外を明記⑶前提の決定論テスト⑷下流申し送り。**実行時の挙動は不変**——製品コード 3 ファイルから doc 行を除くと main と byte 一致（式・署名・use・属性の不変が構造として成立）。裁定の土台を檻に入れた実測: 到達 23 比 × 寸 1..=1200 ＝ 27,600 組で差は常に 0 か 1（**−1 は 1 件も出ない**＝文字が切れる方向に転ばない）・差 1 は 162 件＝**6/5 と 12/5 で各 81 件**・残る 21 比は 0 件。正体は「1.2 の f32 表現」一点。**下流の宿題**: 適合 e2e は供給面寸の判定に **+1 許容**が要る（窓 client 寸は丸め権威経由ゆえ従来どおり絶対値・両 brief へ追記(68) で申し送り済み）。
- **実機サインオフ発見 7件中 #1〜#6 解決済み**。**#7（冒頭空行）のみ pasta 上流（`ekicyou/pasta` 起票済み）＝areka スコープ外・未解決**。
- 完了 spec 直下エントリ = **159**（`.kiro/specs/completed/` 直下＝ディレクトリ 158 ＋ 単体ファイル `graphics-rendering-stability.md`・2026-08-15 棚卸⑨実測）。計数は**直下エントリ数**で行うこと（ディレクトリ数だけ数えると 1 ずれる）。

## M1 残工程ゴール表（2026-08-15 棚卸⑨・完了行は完了サマリへ集約済み）

| 種別 | ゴール（単一文） | ユニット | ウェーブ |
|---|---|---|---|
| 挙動バグ | 拡大率切替時の跳ね＝遷移の原子性＋work area 追随（+36px/24px 浮き） | `dpi-transition-atomicity` | **W6.75**（**次ウェーブ・単独**・再観測は要件の research＝budget 後の実測が帰着切り分けに最良） |
| 挙動バグ | DPI 遷移時の `BalloonFollow.offset` スケール意味論確定 | `balloon-offset-dpi` | **W6.75**（~~atom 縮退時は atom と統合~~＝**2026-08-21 失効**。atom は残存確定で 37 タスクを完走したため統合の前提が消えた。bod 側 brief は 4 箇所とも失効済み） |
| 基盤 | 檻の決定性（毒化 **95 呼出/12 モジュール**・ハーネス 2 設計の一本化・注入シーム） | `test-cage-determinism` | **W6.9**（これ以上後送しない） |
| M-e2e | 適合14項目一周＋DoD＝**M1 完成宣言** | `emo2-conformance-e2e` | **W7**（最終） |
| 性能 | 描画・フレーム駆動の負荷を SSP 同等圏へ（現状 CPU 3.6 倍） | `draw-load-parity` | **W8**（**優先度低・M1 完成を妨げない**・2026-08-15 起票） |
| 見た目 | 遷移中に絵と窓が同じ提示フレームで揃う（要件 4.2 の実機側・可視化→書込の隙間 0.21〜0.31 秒） | `present-write-coherence` | **W8**（**優先度低・M1 完成を妨げない**・2026-08-22 起票＝atom から分離） |

> 完了済みマイルストーンのゴール表は history 参照。M-dual は退役＝e2e 適合 #10 へ吸収。

## ウェーブ編成（着手順の正本・2026-08-15 棚卸⑨改訂）

> 各ウェーブは**フルライフサイクル**（要件→設計→タスク→実装→`/kiro-complete`＝PR squash マージ）を完走してから次へ。並走はウェーブ内のみ（1 spec = 1 worktree = 1 PR）。同居は**実測で共有ファイル 0**が原則。文書フェーズ（要件・設計）は先行可＝先行 spec はウェーブ開始時に settled main へ再突合。優先順位: **挙動バグ → 依存ツリーが長く早期着手が効くもの → その他**。詳細な申し送りは各 brief の追記ブロックが正本（roadmap は編成と条件のみ持つ）。

| Wave | ユニット | 開始コマンド | 編成根拠・条件 |
|---|---|---|---|
| W1〜W6.5 ✅ | （W5=4/4・W5.95=slimming・W6=5本・W6.5=budget ∥ exact ∥ wpl。完了サマリ参照） | — | 詳細は history（旧 W6/W6.5 行全文は棚卸⑨退避）。**生存する W6 申し送り**: ⑴**⚠ vis は zorder の再表示シーム `ReassertZOrder` を消費せずに着地**（再表示直後のバルーン隣接は実機未確認）——**e2e／cage で拾うこと** ⑵配置系 spec は `window-placement` R2.9 を正典として引用しない（正典は COMPAT §8 経由で scg へ一意に辿る） |
| **W6.75**（次ウェーブ・単独） | `dpi-transition-atomicity`（＋`balloon-offset-dpi`） | `/kiro-start areka-P0-dpi-transition-atomicity` | **単独フルライフサイクル（2026-08-14 直列裁定・着手ゲート全開＝W6.5 完走済み）**——第 1 段再観測を requirements の research として settled main 上で実施＝**budget 後の実測ゆえ「残る跳ねコスト＝atom の取り分」が一回の観測で確定**（旧 859ms/8 回は van S1 是正で失効済み・budget が 1 コマ適用を 1,240µs へ是正済み＝合成コスト帰着説はほぼ消滅・観測は budget 新設の `presenter/timing.rs` Stage 計時の流用が第一候補）。**分岐は再観測で確定**: ⑴**縮退時**＝「+36px work-area 追随＋檻」へ縮退し **bod と統合して 1 spec 化**（follow 系）⑵**残存時**＝show.rs `apply_show` 域へ広がる（budget 実形へ design 前 rebase・関心域 :280-330）・bod は atom に従う。rebase 必達: **wpl step 5a（`resize_window_to` 内側）＋`follow/keyword_base.rs` の one-shot 再導出**（scg `ChainFinalized`／`ChainFinalizeStall` 寿命引き取りと同じ問い・**申し送りの正本＝atom brief 棚卸⑨ブロック**）。flush 経路（`tick_bridge`／`command.rs`）改造時は zorder 維持系が同経路へ指令を積む点を確認（W6 申し送り⑷）。**【2026-08-22 確定】分岐は ⑵ 残存で決着し atom は単独で完走した**——**37 タスク全完了**（群 6 で `6.5`・最終ゲートで `7.5` を追加起票）・ワークスペース 5,636 passed / 0 failed（86 バイナリ）。実機サインオフは **`ATOM-SIGNOFF: FAIL` のまま開発者裁定 GO** で閉じ（決定論 8 遷移すべて PASS／実機専用 µs 2 系統が違反 40 件）、未達は `present-write-coherence`（W8）が引き受ける。**bod は統合せず単独 2 本**（atom 着地後の rebase 前提・`window_move.rs` は 965→1,223 行）。**残りは `/kiro-complete areka-P0-dpi-transition-atomicity`（開発者の明示承認が要る）のみ。** |
| **W6.9** | `test-cage-determinism` | `/kiro-start areka-P0-test-cage-determinism` | `presenter/show.rs` `apply_show` 鎖（budget→atom→④）の**最後尾**。vis 先着（推奨）は充足済み。着手時に vis/exact/budget/atom の実形へ rebase・毒化 **95 呼出/12 モジュール**の再計数（後置するほどコピーが増える構造が実証済み＝**これ以上後ろへ置かない**・slim 後の cage 宛アンカーは `completed/areka-P0-file-slimming/verification/notes.md` §37 が再解決済み）。④は `#[cfg(test)]` fault フラグ小案を第一候補・観測点＝upload エラー分岐 **:297-301**（budget 改稿後・2026-08-15 実測）。bind が登記した info ログ檻 3 本＋既存間欠赤 1 本（`bind_default_exclusive_replace_emits_show_and_info_marker`）も担当クラス。W6 申し送り⑴（`ReassertZOrder` 未消費）は檻で拾える範囲を検討 |
| **W7** | `emo2-conformance-e2e`（最終） | `/kiro-start areka-P0-emo2-conformance-e2e` | 全ユニット完了後＝適合14項目（#1 DPI 検証は追従込み・バルーン表示ライフサイクル追補・#3 は bind・#10 は ker が前提充足）の一周走行→**M1 完成宣言**。着手時義務: brief 全面再監査・㉘(E) の実機判断・#7（pasta 上流）は M1 完成を妨げない扱いの確認・**W6 申し送り＝再表示直後のバルーン隣接（`ReassertZOrder` 未消費）の実機確認** |
| **W8**（優先度低） | `draw-load-parity` | `/kiro-start areka-P0-draw-load-parity` | **M1 完成を妨げない**（2026-08-15 開発者裁定＝優先度の低い仕様として起票・実行は後日別セッション）。budget が「削るべき対象は自分の境界の外」を実測で示したため分離起票。**apply_show は CPU の 3.3% しか占めない**（着手前でも 10.4%）ので、`presenter/show.rs` は対象外。真の最大項は **`try_tick_world` が 13 スケジュールを 120回/秒 全部回していること**（1 tick 578µs・壁時計 6.85%・**tick の 98% は表示に変化なし**）。内訳は FrameFinalize 182µs(31.5%)＋Draw 143µs(24.8%) で 56%。**wintf 中核（フレーム駆動）に手を入れる spec** なので、クリック透過・αマスク追随の「毎フレーム評価」前提（`runtime/mod.rs:231-237` が R2.4 として明記）と正面から調停が要る。**⚠ 比較の前提に未検証の穴**——SSP は 100% 描画→200% 引き伸ばしの可能性が高く（開発者観察＝文字がぼやける）、事実なら画素の仕事量が 1/4 で「3.6 倍」は画素あたり 0.9 倍＝互角。**目標を絶対値で置くか画素あたり効率で置くかは要件段階の裁定事項**。atom（W6.75）とは `tick_bridge` の flush 経路で干渉しうるので着手時に順序調停 |
| **W8**（優先度低） | `present-write-coherence` | `/kiro-start areka-P0-present-write-coherence` | **M1 完成を妨げない**（2026-08-22 開発者裁定＝「大改造が必要なら無理に治さなくて良い」）。atom が要件 4.2 を**決定論では満たし実機では満たさずに**閉じたぶんの引受先。出発点は atom 設計 **C8 の B-3（可視化の 2 相化・第一候補）／B-4（窓内下端中央補償・緩和）** で、候補表の外へは広げない（atom 要件 3.4 を継承）。**実測の起点**（`atom-73-signoff-1`・8 遷移・全遷移を走査した値）: `visualize_to_write_us` **210,329〜306,301µs**（上限 16,667µs の **12.6〜18.4 倍**・違反 32 件）／`flush_total_us` 143,231〜231,910µs（同 8 件）＝**実機専用系統の違反は計 40 件**。**B-2b は隙間を縮めなかった**（`flush_total` 平均 192,247→**188,711**µs＝**−1.8%**・OS 側が過半＝L7）。**窓ごとの隙間はむしろ +27% 伸びた**（全窓の書込がバッチ末尾へ揃った帰結・台帳 §11.6）。接触面は `presenter/show.rs` の可視化の段（`apply_show`:46 の末尾＝`set_visible`:375／`set_bounds`:381／`Visualize` 発行:392）。**cage（W6.9）の後**に置く（同じ `apply_show` 鎖を触るため）。B-4 を採る場合は当たり判定の原点（`collision-dpi-hittest`）と `mount.rs` の配置契約に触れるので atom 要件 10.1 の再確認が要る。**tick 構造の大改造に及ぶなら atom 要件 9.3 に従い分割を再裁定する**（要件段階でまず規模を見積もること）。判定器・観測語彙・サインオフ手順書は atom の着地物を流用（新設不要） |

**干渉台帳（生存ペアのみ・2026-08-15 棚卸⑨で W6.5 完走後の実形へ再解決・旧全文は history）**:
- **atom⇄bod**〔follow 系共有＝~~統合候補（縮退時は統合が既定路線）~~ **2026-08-21 失効・単独 2 本で確定**: atom の主戦場は `follow/window_move.rs`・persist は `placement/persist.rs`。**atom は 2026-08-22 に 37 タスクを完走した**（`window_move.rs` は 965→1,223 行）ので、bod は atom 着地後の rebase 前提で読むこと。いずれにせよ W6.75 内で完結〕
- **atom⇄dlp（W8）**〔`tick_bridge` flush 経路で干渉しうる: 編成上 atom（W6.75）が dlp（W8＝M1 後）へ必ず先着するため直列は構造で成立。dlp 着手時に atom 実形へ rebase・順序調停〕
- **軽微**: cage③の test_support 共有化で placement 系（bod）の import 行が追随＝実質共存可
- **show.rs アンカー（2026-08-15 実測・budget 改稿で全面ドリフト）**: `apply_show` :43 起点・budget 域（compose/resample/mask/insert）:95-170・atom 関心域（chain 生成〜upload〜可視化）:280-330・cage④ :297-301。旧 :32／:68-88／:220-270／:227-232 は全て失効
- **退役（2026-08-15 棚卸⑨・W6.5 の 3 本着地により後続 spec の design 前 rebase 義務へ転化）**: budget⇄atom・cage④⇄budget・atom⇄wpl・wpl⇄bod・exact→bod（rebase 点の正本＝atom brief 棚卸⑨ブロック／bod brief 追記(68)。実装着地の file:line ドリフトは各 spec の design 前 rebase＝既存規律で吸収）
- `status-execution-states`=台帳 spec（着手しない・源着地時に just-in-time）・`surfaces-basepos`／`sakura-time-directives`／`balloon-canon-residue`=**M2 解禁ゲート**（M1 では着手しない）

## 着手手順

> **brief 全数完備体制**: M1 残ユニット 5 本（atom・bod・cage・e2e・dlp）＋M2 ゲート 4 本＝全 9 本 brief 済み＝着手は該当 brief を読んで `/kiro-start <unit>` へ直行。新規課題の起票は `/kiro-discovery`（再入）で brief just-in-time 生成。`/kiro-spec-batch` は使わない（一括＝工場化）。ウェーブ跨ぎの合流判断は別セッションで一括（記憶 portfolio-convergence-decided-in-separate-session）。

## 制約

- Rust 2024・マルチクレート（wintf/dola/areka ＋ `areka-parsers` ＋最小依存 `shiori-abi` ＋ host-32 3クレート `shiori-host32-ipc`/`-host`/`-helper`）。
- **32bit 可搬性の適用範囲＝host-32 系（`shiori-host32-*`／`shiori-abi`）のみ**。wintf/areka 本体は x64＋arm64 ネイティブ（i686 検証を本体 spec に課さない）。
- 透過は WUC/DComp GPU 合成上のクリックスルー機構（`WS_EX_TRANSPARENT` 動的トグル＋αマスク）で成立（ULW は撤去済み）。SHIORI 内部唯一 ABI=`IShiori`(COM, HSTRING/UTF-16)。過去互換は 32bit Rust ホスト。
- 設計判断の変更は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md) を正本として更新。
- 実機運転の定石: 絶対パス起動（相対は pasta.dll LOAD 失敗）・i686 helper を先ビルド・`AREKA_APP_SMOKE_EXIT_MS` 有界自動終了＋`RUST_LOG` grep（記憶 areka-real-machine-signoff-bounded-auto-exit）。

## M2 以降

**M1 完成後に、実物を見て組み直す。** 本ロードマップでは扱わない（pasta の native x64・`IShiori` in-proc 化、ベクトル描画・AI、**owner-draw 右クリック system メニュー（ゴースト管理 chrome）**、互換面拡大＝Shift_JIS/SAORI/里々・YAYA 網羅/NAR 等はその時に）。

**アプリ層の M2 予約（2026-07-05 ukadoc 裏付け・全て任意＝emo2 単体起動に不要）**: SSTP ポート（9801）ホスティング・FMO・DirectSSTP・Plugin/HEADLINE/SAORI ホスティング・ネットワーク更新（`\![update,platform]`・OnBasewareUpdating/Updated 系）・ゴースト/バルーン選択 UI（OnGhostChanging 系）・多重ゴースト運用。

**emo テキスト進化の予約（M2 候補）**: ①回転テキストの実挙動（M1 で行列変換領域を内部表現として持ち込み済み）②ポップアート級の文字装飾（text effects——1枚物自前合成ゆえ文字も合成レイヤ）。

**バルーン美観配置政策の予約（M2・2026-08-01 `dpi-window-vanish` task 6.2 が先送り登記）**: 画面端でのバルーン**左右反転**をはじめとする SSP 互換の美観配置政策。M1 が持つのは遷移ガードによる「完全不可視への遷移を防ぐ安全網」まで（*見えない会話*より*重なった会話*を優先する裁定）。**縮退シーム＝`[visibility-guard] ClampX` の `warn!`**（`route=BalloonFollow` 行）——安全網の発火回数が M2 着手時の優先度根拠。**M1 では起票しない**。

**M2 解禁ゲートの spec（brief 済・M1 では着手しない）**: `areka-P0-surfaces-basepos`・`areka-P0-sakura-time-directives`（互換拡充時に解禁）／`areka-P0-status-execution-states`（残状態の源サブシステム着地時に just-in-time・台帳）／`areka-P0-balloon-canon-residue`（balloon 正典残語彙 10 項目の受け皿＝kero-balloon 由来 6 件＋balloon-visibility 由来 4 件）。

---

**追記台帳（要約・全文は history）**: (51) W6.5 残件 2 spec 起票（07-30）／(52) 棚卸④＝roadmap 履歴分離・W6 に bind 編入・brief 補正 8 本（07-31）／(53) `recompose-budget` 起票＝`compose_into` 本番未配線・アイドル 1 コア 13〜22%（07-31）／(54) `ghost-window-zorder` 起票＝owner 無し・全 `SWP_NOZORDER`・「語彙完備・配線ゼロ」3 例目（07-31）／**(55) 欠番**（atom 起票の本体がマージで脱落・内容は atom brief＋(57)⑴ が正本）／(56) 未登記先送り棚卸＝孤児 17 件・新規 brief 4 本起票（08-01）／(57) van 完了＝S5 担当が atom で確定・確定台帳の裁定付き訂正運用が確立・檻の空虚性通算 9 例（08-01）／(58) 棚卸⑤＝W5 3本マージ後の全面棚卸・全配置確定・干渉行列 55 ペア再実測（08-01）／(59) col 実装中の申し送り＝cage brief ①インベントリを 95 呼出/12 モジュールへ拡大（08-05・PR#101・cage brief 内が全文正本）／(60) 棚卸⑥＝col マージ後の軽量アンカー監査・実害ドリフト 2 ファイルのみ（08-06）／(61) `file-slimming` 起票＝W5.95 新設・肥大の 7〜8 割は in-file 檻（08-06）／(62) 棚卸⑦＝slimming 後の干渉台帳全面再解決・緩和 3 ペア（08-11）／(63) `bindoption-exclusivity` 完了＝根因 2 層・mayuna R4.5/D11 覆し・25 分実機 PASS（08-11）／(64) `ghost-window-zorder` 完了＝案 A owner 確定・topmost 帯引き込み是正 `HWND_TOP`（08-13）／(65) 棚卸⑧＝W6 完走後の全面再解決・退役 5 ペア・W6 行/詳報/追記(58)(60)-(64) を history 退避（08-14）／(66) 開発者裁定＝性能最優先・budget を W6.5 へ前倒し（08-14）／(67) 開発者裁定＝atom 文書併走不採用・W6.5 着地後の単独直列へ（08-14）／(68) `draw-load-parity` 起票＝W8 新設・優先度低・SSP 同一手順比較初実施（areka 10.97% 対 SSP 3.05%・**⚠SSP は 100% 描画→引き伸ばしの疑い＝目標を絶対値か画素あたり効率かは要件裁定事項**）・真の最大項＝`try_tick_world` 120回/秒 全走・budget 判定式⑵（catch-up）も引受け（08-15）。

**2026-08-15 追記(69)（棚卸⑨＝W6.5 完走後の全面再解決）**: `/kiro-discovery` 再入。前回⑧以降の main 差分は W6.5 の 3 実装（exact PR#110／wpl PR#111／budget・計 +34,344 行）＝**W6.5 完走**。①**brief 無し spec=0・新規起票 0・分割の新規事案 0 を確認**（残 9 brief 全実在＝M1 残 5＋M2 ゲート 4・spec.json 無し＝走行中 spec 無し・completed 直下 159 実測一致）。②**ウェーブ骨格は不変**（W6.75→W6.9→W7→W8）。**次ウェーブ＝W6.75 は atom 単独**（直列裁定(67) どおり。並走候補は厳しめ精査でもゼロ: bod は atom 従属〔統合候補〕・cage は apply_show 鎖最後尾＋atom 実形 rebase 必達・e2e は全ユニット後・dlp は M1 後＋flush 経路干渉＋開発者裁定「後日別セッション」）。③**干渉台帳を W6.5 後の実形へ再解決**——退役 5（budget⇄atom・cage④⇄budget・atom⇄wpl・wpl⇄bod・exact→bod＝全て後続の design 前 rebase 義務へ転化）・生存 2（atom⇄bod・atom⇄dlp）。show.rs は budget 改稿で全アンカードリフト（`apply_show` :32→:43・atom 域 :220-270→:280-330・cage④ :227-232→:297-301・2026-08-15 実測）。④**atom brief へ W6.5 申し送りを登記**（棚卸⑨ブロック＝着手ゲート全開・859ms 帰着材料〔budget 1,240µs 是正で合成コスト説ほぼ消滅〕・`presenter/timing.rs` Stage 計時の流用第一候補・wpl step 5a・`keyword_base` one-shot 再導出が scg に続く 2 例目で `ChainFinalized` 寿命と同じ問い）。⑤**roadmap 減量**: 陳腐 wpl 残行の削除＋ゴール表の分断修復・W6 詳報バレット・旧 W6/W6.5 行全文・旧干渉台帳・追記(65)(66)(67)(68) 全文を history へ退避。

**2026-08-22 追記(79)（1 ファイル 1,000 行の目安が漂流している——`atom` の最終ゲートで実測。開発者の裁定が要る）**: `.kiro/steering/structure.md:176` は「**1 ファイル 1,000 行以下の目安は本番ファイル・テストファイルの双方に適用する**」と定めており、`file-slimming`（W5.95・08-10・PR#103）が **1,000 行超 54→0 本**まで落として達成した。**その後、機械的な番人が 1 つも無いまま漂流している。** 2026-08-22 の実測（`crates/**/*.rs` 全数）:

| 行数 | ファイル | 出自 |
|---|---|---|
| 1,604 | `crates/areka-emo-present/src/cache_tests.rs` | **main に既存** |
| 1,330 | `crates/areka-seriko/src/actor_bind_loop_tests.rs` | **main に既存** |
| 1,255 | `crates/areka/src/emo2_boot/frame_transition_branch_tests.rs` | atom（新規） |
| 1,223 | `crates/areka/src/placement/follow/window_move.rs` | atom（**965 → 1,223**） |
| 1,076 | `crates/areka-emo-compose/src/plan_ops_tests.rs` | **main に既存** |
| 1,047 | `crates/areka-emo-present/src/presenter/budget_tests.rs` | **main に既存** |
| 1,039 | `crates/areka/src/placement/transition_judge_tests.rs` | atom（新規） |
| 1,037 | `crates/areka/src/placement/transition_judge_verdict_tests.rs` | atom（新規） |
| 1,033 | `crates/areka-seriko/src/bind.rs` | **main に既存** |

**要点は「atom が壊した」ではなく「既に 5 本破れていて atom が 4 本足した」ことである。** 5 本は本ブランチの分岐点（`git merge-base origin/main HEAD`）の時点で既に超えており、`recompose-budget`・`kero-balloon`・`bindoption-exclusivity` 等の後続 spec が積み上げたものである。**目安は「目安」と書かれており、テストも lint も守っていない。**

- **atom は自分の申し送りを消化しなかった**——`tasks.md` の「task 5.5 への申し送り: `window_move.rs` は 974 行で 1,000 行の目安まで残り 26 行・着手前にファイル分割が要る」は実行されず、裁定の記録も残らないまま群 5〜7 が進んだ。**最終ゲートまで誰も気づかなかった。**
- **atom の内側で今これを直すのは高くつく**——`window_move.rs` を割ると spec 文書 7 本に散る **41 箇所**の `window_move.rs:<行>` が全部動く。**file:line の陳腐化はこの spec が 4 度踏んだ失敗そのもの**であり、実機サインオフを閉じた直後にそれを自ら仕込むのは筋が悪い。ゆえに **atom の内側では割らず、数量つきで報告して裁定を仰ぐ**（本追記がその報告である）。
- **引受先は現時点で存在しない。** `file-slimming` は `.kiro/specs/completed/` にあり申し送りを消化できない。上の 9 本のいずれかを名指ししている**生存 spec は 1 本も無い**（`.kiro/specs/` の completed を除く全ディレクトリを 5 本のファイル名で走査して 0 件・2026-08-22 実測）。**「ウェーブ名は担当者ではない」の規律に従い、実在の引受先が決まるまで先送りとは呼ばない——本追記が唯一の記録である。**
- **開発者に問うべきは 3 択**: ⒜ 目安を機械で守る（行数の番人テストを 1 本置き、既存 9 本は例外表に載せて漸減させる。置き場所の候補は `test-cage-determinism`＝W6.9）／⒝ 掃除の spec を 1 本起票する（`file-slimming` 第 2 期）／⒞ 目安のままとし `structure.md:176` に「番人は無い・漂流は許容する」と明記して**期待値を実態へ合わせる**。**現状は「規則があるのに誰も測っていない」＝最も悪い形**である。

**2026-08-22 追記(75)（`present-write-coherence` 起票＝W8 に 2 本目・atom からの分離）**: `/kiro-discovery` 再入（Path C＝新規の単独 spec）。**atom は要件 4.2 を決定論では満たし、実機では満たさずに閉じる**——その差分の引受先として起票した。**分離の根拠**: ⑴ atom の段階裁定は task 7.1 で **B-2b 単独採用**が確定済みで、要件 3.4 が「採用候補を候補表の外へ広げない」と縛る＝atom の内側で B-3 へ進むには裁定のやり直しが要る。⑵ 接触面が `presenter/show.rs` の**提示の順序**で、atom の主戦場（配置・窓書込）とは別の関心事。⑶ `draw-load-parity` の brief が「`presenter/show.rs` は対象外」（`apply_show` は CPU の 3.3%）と明記しており責務が重ならない。**atom が実際に取った成果**（誤読を防ぐため明記）: 逐次適用は消えた——1 遷移で 4 窓の書込が散らばる幅が **93〜158ms → 40〜101µs**（約 1,500〜2,000 分の 1・B-2a 合流＋B-2b バッチ）。決定論 8 遷移すべて PASS（同一フレーム・窓ごと 1 回・経路 A 0・接地点差 0・連鎖 1 回・随伴の同一フレーム性）。**残ったのは「絵が先・窓が後」の 0.21〜0.31 秒**（`visualize_to_write_us` 210,329〜306,301µs）で、これは OS 側のコスト（L7）と提示の順序の問題であり、窓書込の同時性とは別の軸である。**⚠ 実機の目視が判定器より先に見つけた**——判定器は `mismatch_frames=0` を出すが、それはアプリ側の tick 内順序を測っており、合成器が画面へ出す順序は測っていない。**目視と機械が「食い違う」のではなく「別のものを測っている」**。**この読み分けは手順書 §6.5 が持つようになった**（atom task 7.4 で 4 行の表＋3 問の分岐手順へ拡張。起票時点では §6 の突合が区別を持たなかったので、その旨は本追記の初版の記述であり現況ではない）。**開発者裁定 2026-08-22**＝「大改造が必要なら無理に治さなくて良い」・M1 完成を妨げない優先度（dlp と同格）。**atom 側の閉じ方（2026-08-22 確定）**: 実機サインオフは `ATOM-SIGNOFF: FAIL` のまま**開発者裁定 GO** で完了した——合否は書き換えていないので、以後この採取を引くときは「FAIL だが GO」と読むこと（全文は atom requirements.md 要件 8 の注記・台帳 §11.4・記録票 §9）。干渉: **cage（W6.9）の後**に置く（同じ `apply_show` 鎖）・B-4 を採る場合は `collision-dpi-hittest`（completed）の当たり判定原点に触れる。
