---
inclusion: manual
updated_at: 2026-08-14
---

# Roadmap — areka M1（最小 SSP 互換ベースウェア）

> **このロードマップは M1 のみを扱う。** M2 以降は **M1 完成後に実物を見て組み直す**（憶測で先に書かない）。
> 正本配置: 本ファイルが M1 ロードマップ正本（`.kiro/steering/roadmap.md`）。`focus.md`（`inclusion: always`）から辿る。設計判断の正本は [doc/COMPAT_ARCHITECTURE.md](../../doc/COMPAT_ARCHITECTURE.md)。M1 実物スコープは [doc/emo2-conformance-scope.md](../../doc/emo2-conformance-scope.md)。
> **履歴**: 追記①〜(51)・M-boot 完了詳報等は 2026-07-31 棚卸④で、W5 完了詳報・旧ウェーブ表・旧干渉台帳・追記(53)(54)(56)(57) 全文は 2026-08-01 棚卸⑤で、**W6 行全文・W6 完了詳報バレット（col/vis/scg）・追記(58)(60)(61)(62)(63)(64) 全文は 2026-08-14 棚卸⑧で**、いずれも [roadmap-history.md](roadmap-history.md) へ退避（history が全文正本・既知の記録欠陥〔㊻番号衝突・**(55) 欠番**〕も history 冒頭と棚卸⑤節に注記）。完了ユニットの実装詳細は各 `completed/` spec が正本。

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
- **W6 完了 5/5**（2026-08-05〜08-13・**W6 完走**）:
  - `collision-dpi-hittest` ✅（08-05・PR#100）＝**当たり判定 ÷k の決着**。丸め権威 `ScaleRatio::unscale_coord`（画素中心逆写像・整数のみ）＋本番入口 `hit_region_client`・SHIORI 配信座標を**サーフェス px** へ正準化・**実 DPI 2 水準**（125% k=5/4／200% k=2/1・互いに異なる拡大寸）サインオフ合格＝`collision-geometry` 4.2 却下の観測条件を初突破。
  - `balloon-visibility` ✅（08-13・PR#106）＝**起動時の空バルーン出しっぱなしの決着**。可視性の所有権を presenter の第一級概念へ・起動時は「不可視のまま確立」・表示は可視グリフ数の増加、非表示はゼロ下降とタイムアウト（既定 30 秒・占有終端起点）・ドラッグ／ポインタ滞在／選択肢中は抑止。**相順を固定する検査が初めて入った**。実機 2 回サインオフ合格（DPI 192）。残件（明示的な非表示指令の実機確認）は `balloon-canon-residue` へ送付済み。
  - `bindoption-exclusivity` ✅（08-11・PR#105）＝**表情固着の根治・根因は 2 層**（bindoption 3 値正典〔`mustselect`／非宣言=高々1・解除可／`multiple`〕＋**bind から外れた ID の保持コマ掃除 3 段**）。**`mayuna-compose` R4.5/D11 を覆した**（同型誤仮定の 2 度目）。25 分実機 J1/J2/J3 全 PASS。**本番挙動の変化**: 非宣言 3 カテゴリ（まばたき 1400-1403・キラリ 1700/1701・髪飾り 1800/1801）が新たに排他＝正典どおりの是正。`mustselect` の起動時充足は shell `default,1` へ委譲（縮退登記・emo2 実害なし）。全文は history 追記(63)。
  - `ghost-window-zorder` ✅（08-13・PR#107）＝**バルーンを自キャラ窓の直前に維持＝案 A（Win32 owner）で構造保証確定**。実機ゲート G1〜G8・サインオフ S1〜S4 全 PASS（異なる拡大率 2 画面の往復証跡つき）。検証で本物の欠陥 3 件摘出——うち**常時最前面帯への引き込み**は `HWND_NOTOPMOST`→`HWND_TOP` へ是正（隣接保証は owner 関係が担う）。案 A ではキャラクリックで `fix` は出ない（証跡は `sink-observed`）。全文は history 追記(64)。
  - `scope-chain-gap` ✅（08-13・PR#108）＝**P2 連鎖の幅差隙間の決着**。SSP 実測オラクル 8 本で正典確定（完全隣接・gap 0・拡大率不変）・先行 `window-placement` R2.9 を COMPAT §8 で上書き・**要件 7 が差し戻しで新設**＝実表示寸確定時に連鎖を一度だけ解き直す `finalize_chain_once`（定常 52px も 0 へ）・実機 2 水準とも gap 0。`\![move]` の dx/dy k 倍＝意図的 SSP 非互換。
- **実機サインオフ発見 7件中 #1〜#6 解決済み**。**#7（冒頭空行）のみ pasta 上流（`ekicyou/pasta` 起票済み）＝areka スコープ外・未解決**。
- 完了 spec = **156**（`.kiro/specs/completed/` の**直下エントリ数**＝ディレクトリ 155 ＋ 単体ファイル `graphics-rendering-stability.md` 1・2026-08-14 実測一致確認済み）。**ディレクトリ数を採ると 1 ずれる**ので計数時は直下エントリで数えること。

## M1 残工程ゴール表（2026-08-14 棚卸⑧・完了 6 行は完了サマリへ集約済み）

| 種別 | ゴール（単一文） | ユニット | ウェーブ |
|---|---|---|---|
| 基盤 | 画素演算の f32 排除＝`ScaleRatio` 有理数を text 層まで配管 | `scale-exact-rational` | **W6.5** |
| 挙動バグ | `windowposition.limit` 正典既定 1 実装＝バルーン画面外はみ出し解消 | `windowposition-limit` | **W6.5**（scg 実形へ rebase 必達） |
| 挙動バグ | 常駐アイドルの CPU 消費＝毎フレーム全再合成のアロケーション予算是正 | `recompose-budget` | **W6.5**（**性能最優先の開発者裁定で W6.75 から前倒し**・2026-08-14） |
| 挙動バグ | 拡大率切替時の跳ね＝遷移の原子性＋work area 追随（+36px/24px 浮き） | `dpi-transition-atomicity` | **W6.75**（W6.5 着地後に**単独フルライフサイクル**・再観測は要件の research＝budget 後の実測が帰着切り分けに最良） |
| 挙動バグ | DPI 遷移時の `BalloonFollow.offset` スケール意味論確定 | `balloon-offset-dpi` | **W6.75**（atom 縮退時は atom と統合） |
| 基盤 | 檻の決定性（毒化 **95 呼出/12 モジュール**・ハーネス 2 設計の一本化・注入シーム） | `test-cage-determinism` | **W6.9**（これ以上後送しない） |
| M-e2e | 適合14項目一周＋DoD＝**M1 完成宣言** | `emo2-conformance-e2e` | **W7**（最終） |
| 性能 | 描画・フレーム駆動の負荷を SSP 同等圏へ（現状 CPU 3.6 倍） | `draw-load-parity` | **W8**（**優先度低・M1 完成を妨げない**・2026-08-15 起票） |

> 完了済みマイルストーンのゴール表は history 参照。M-dual は退役＝e2e 適合 #10 へ吸収。

## ウェーブ編成（着手順の正本・2026-08-14 棚卸⑧改訂）

> 各ウェーブは**フルライフサイクル**（要件→設計→タスク→実装→`/kiro-complete`＝PR squash マージ）を完走してから次へ。並走はウェーブ内のみ（1 spec = 1 worktree = 1 PR）。同居は**実測で共有ファイル 0**が原則。文書フェーズ（要件・設計）は先行可＝先行 spec はウェーブ開始時に settled main へ再突合。優先順位: **挙動バグ → 依存ツリーが長く早期着手が効くもの → その他**。詳細な申し送りは各 brief の追記ブロックが正本（roadmap は編成と条件のみ持つ）。

| Wave | ユニット | 開始コマンド | 編成根拠・条件 |
|---|---|---|---|
| W1〜W5.95 ✅ | （W5=4/4・W5.95=file-slimming 単独。完了サマリ参照） | — | 詳細は history |
| **W6** ✅ **完走** | ~~col ∥ vis ∥ bind ∥ zorder ∥ scg~~（5本・2026-08-05〜08-13） | — | 編成条件⑴〜⑹の消化記録は history（棚卸⑧退避）。**生存する申し送りのみ**: ⑴**⚠ vis は zorder の再表示シーム `ReassertZOrder` を消費せずに着地**（main 実測 0 件・案 A では owner 関係が隣接を保つ見込みだが**再表示直後の隣接は実機未確認**）——**e2e／cage で拾うこと** ⑵配置系 spec は `window-placement` R2.9 を正典として引用しない（正典は COMPAT §8 経由で scg へ一意に辿る）⑶atom は `ChainFinalized`／`ChainFinalizeStall` の寿命を設計判断として引き取る ⑷atom が flush 経路（`tick_bridge`／`command.rs`）を改造する場合は zorder の維持系が同経路へ指令を積む点を確認（zorder 自身は flush 非接触で完了） |
| **W6.5**（3本） | `recompose-budget` ∥ `scale-exact-rational` ∥ `windowposition-limit` | `/kiro-start areka-P0-recompose-budget`・`/kiro-start areka-P0-scale-exact-rational`・`/kiro-start areka-P0-windowposition-limit` | 実測で素（各自 1 worktree = 1 PR でフルライフサイクル並走）。**budget は性能最優先の開発者裁定（2026-08-14）で W6.75 から前倒し**——干渉は⑴exact⇄budget=slimming 緩和済み（`read.rs` ∥ `show.rs`）⑵wpl とはドメインごと素⑶atom は本ウェーブでは文書のみ＝非接触、かつ台帳の推奨順序（**budget 先着→atom が 859ms を再測**）が自然成立し編成が強化される。着手ゲート開放済み（bind 着地で (a) 同根説消滅・第 0 段計時ログ設計は budget brief の 2026-08-11 追記が正本）・キャッシュ容量は emo-present R4.1 の**承認済み要件**ゆえ変更には要件段階の裁定が要る（**2026-08-15 に裁定が下り 1 → 3・置換方式 LRU へ改訂**＝budget 要件 7.1／7.3・R4.1 も同日付で改訂済み）。**wpl は scg の実形へ rebase 必達**——干渉面は resolver.rs P2/P5（現 :154／:184）に留まらず **`placement/chain_finalize.rs`（第 2 の位置ライター・P4 クランプ非経由）／`frame/drain_resnap.rs` `finalize_chain_once`／`spawn.rs` `ScopeWindows.default_char_pos`** へ拡大（**wpl brief の scg 申し送りブロックが正本**）。逆順不可の理由（limit クランプ後の値へ scg 式が二重補正）は解消済みだが、クランプと連鎖確定の**適用順序**は wpl の要件裁定事項。**exact** は `presenter/read.rs`（f32 汚染点 **:109**・2026-08-14 現物一致確認済み）＝show.rs 系（budget/atom/cage④）と別ファイル。**exact は bod の前提**（丸め権威）ゆえ W6.75 より先。**atom の文書併走は不採用（2026-08-14 開発者裁定＝直列方針）**——atom は W6.5 の 3 本着地後に W6.75 で単独フルライフサイクル（下行）。 |
| **W6.75** | `dpi-transition-atomicity`（＋`balloon-offset-dpi`） | `/kiro-start areka-P0-dpi-transition-atomicity`（**W6.5 の 3 本着地後**） | **単独フルライフサイクル（2026-08-14 直列裁定）**——第 1 段再観測を requirements の research として budget/wpl/exact 着地済みの settled main 上で実施＝**budget 後の実測ゆえ「残る跳ねコスト＝atom の取り分」が一回の観測で確定**（旧 859ms/8 回は van S1 是正で失効済み・pre-budget 基線の再採取は不要と裁定）。**分岐は再観測で確定**: ⑴**縮退時**＝「+36px work-area 追随＋檻」へ縮退し **bod と統合して 1 spec 化**（follow 系）⑵**残存時**＝show.rs `apply_show` 域へ広がる（budget 実形へ design 前 rebase）・bod は atom に従う。`ChainFinalized`／`ChainFinalizeStall` の寿命引き取り（W6 申し送り⑶）も本 spec の設計判断 |
| **W6.9** | `test-cage-determinism` | `/kiro-start areka-P0-test-cage-determinism` | `presenter/show.rs` `apply_show` 鎖（budget→atom→④）の**最後尾**。vis 先着（推奨）は充足済み（vis 08-13 着地）。着手時に vis/exact/budget/atom の実形へ rebase・毒化 **95 呼出/12 モジュール**の再計数（後置するほどコピーが増える構造が実証済み＝**これ以上後ろへ置かない**）。④は `#[cfg(test)]` fault フラグ小案を第一候補。bind が登記した info ログ檻 3 本＋既存間欠赤 1 本（`bind_default_exclusive_replace_emits_show_and_info_marker`）も担当クラス |
| **W7** | `emo2-conformance-e2e`（最終） | `/kiro-start areka-P0-emo2-conformance-e2e` | 全ユニット完了後＝適合14項目（#1 DPI 検証は追従込み・バルーン表示ライフサイクル追補・#3 は bind・#10 は ker が前提充足）の一周走行→**M1 完成宣言**。着手時義務: brief 全面再監査・㉘(E) の実機判断・#7（pasta 上流）は M1 完成を妨げない扱いの確認・**W6 申し送り＝再表示直後のバルーン隣接（`ReassertZOrder` 未消費）の実機確認** |
| **W8**（優先度低） | `draw-load-parity` | `/kiro-start areka-P0-draw-load-parity` | **M1 完成を妨げない**（2026-08-15 開発者裁定＝優先度の低い仕様として起票・実行は後日別セッション）。budget が「削るべき対象は自分の境界の外」を実測で示したため分離起票。**apply_show は CPU の 3.3% しか占めない**（着手前でも 10.4%）ので、`presenter/show.rs` は対象外。真の最大項は **`try_tick_world` が 13 スケジュールを 120回/秒 全部回していること**（1 tick 578µs・壁時計 6.85%・**tick の 98% は表示に変化なし**）。内訳は FrameFinalize 182µs(31.5%)＋Draw 143µs(24.8%) で 56%。**wintf 中核（フレーム駆動）に手を入れる spec** なので、クリック透過・αマスク追随の「毎フレーム評価」前提（`runtime/mod.rs:231-237` が R2.4 として明記）と正面から調停が要る。**⚠ 比較の前提に未検証の穴**——SSP は 100% 描画→200% 引き伸ばしの可能性が高く（開発者観察＝文字がぼやける）、事実なら画素の仕事量が 1/4 で「3.6 倍」は画素あたり 0.9 倍＝互角。**目標を絶対値で置くか画素あたり効率で置くかは要件段階の裁定事項**。atom（W6.75）とは `tick_bridge` の flush 経路で干渉しうるので着手時に順序調停 |

**干渉台帳（生存ペアのみ・2026-08-14 棚卸⑧で W6 完走後の実形へ再解決）**:
- **budget⇄atom**〔**同ハンク＋因果・直列確定（budget 先＝W6.5 前倒しで自然成立）**: `presenter/show.rs` の `apply_show`（:32 起点）内——budget=compose/resample **:68-88**／atom=スワップ〜upload 域 **:220-270 帯**（vis 着地で旧 :55-70／:186-215 から +13〜+40 ドリフト・2026-08-14 実測）。atom design（W6.75）は budget 実形へ rebase・着手時に 859ms を再測（合成コスト帰着なら budget へ差し戻し）。atom 縮退時は解消〕
- **cage④⇄budget**〔`presenter/show.rs` 同居継続: cage④ 観測点＝upload エラー分岐 **:227-232** が budget :68-88 と同ファイル。④小案で縮退可。cage は W6.9 最後尾で解決〕
- **atom⇄bod**〔follow 系共有＝**統合候補（縮退時は統合が既定路線）**: atom の主戦場は `follow/window_move.rs`（DPI 遷移域・scg 差分 +2 行のみ）・persist は `placement/persist.rs`。分離時は atom 先着→bod rebase〕
- **atom⇄wpl**〔wpl の limit クランプが follow 側に落ちた場合のみ衝突＝wpl の SSP 観測（適用時点）確定後に再判定。**加えて両者とも scg 新設の連鎖確定機構を扱う**（wpl=クランプとの適用順序／atom=`ChainFinalized` 寿命の引き取り）＝wpl が W6.5 で先着し atom design（W6.75）は wpl 実形へ rebase〕
- **wpl⇄bod**〔windowposition.rs 異ハンク: wpl=:39 変換域／bod=:93-94 合流欄（scg 差分 +2 行のみ）。小ファイルゆえ先着後 rebase〕
- **因果のみ（コードは素）**: exact→bod（丸め権威）・atom 859ms⇄budget 143ms（帰着切り分け・budget 先着後の再測が最良）
- **軽微**: cage③の test_support 共有化で placement 系（wpl/bod）の import 行が追随＝実質共存可
- **退役（2026-08-14 棚卸⑧）**: scg⇄wpl（scg 着地→W6.5 行の rebase 義務へ転化・干渉面の拡大は wpl brief 申し送りが正本）・cage⇄vis・atom⇄vis・zorder→vis（いずれも W6 着地で解消）・atom⇄zorder（zorder が flush 非接触で完了・history 追記(64)）
- **slim→全 spec（着地済み・2026-08-10）**: `file-slimming` 着地で全 brief の file:line が一度ずれた＝各 spec は design 前 rebase（既存規律）で吸収。cage 宛アンカーは `completed/areka-P0-file-slimming/verification/notes.md` §37 が再解決済み。**W6 の 4 実装着地でさらにドリフト**（show.rs/read.rs/frame 系/placement 系）——主要アンカーは本台帳が 2026-08-14 実測で更新済み・残りは各 spec の design 前 rebase で吸収
- `status-execution-states`=台帳 spec（着手しない・源着地時に just-in-time）・`surfaces-basepos`／`sakura-time-directives`／`balloon-canon-residue`=**M2 解禁ゲート**（M1 では着手しない）

## 着手手順

> **brief 全数完備体制**: M1 残ユニット 7 本＋M2 ゲート 4 本＝全 11 本 brief 済み＝着手は該当 brief を読んで `/kiro-start <unit>` へ直行。新規課題の起票は `/kiro-discovery`（再入）で brief just-in-time 生成。`/kiro-spec-batch` は使わない（一括＝工場化）。ウェーブ跨ぎの合流判断は別セッションで一括（記憶 portfolio-convergence-decided-in-separate-session）。

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

**追記台帳（要約・全文は history）**: (51) W6.5 残件 2 spec 起票（07-30）／(52) 棚卸④＝roadmap 履歴分離・W6 に bind 編入・brief 補正 8 本（07-31）／(53) `recompose-budget` 起票＝`compose_into` 本番未配線・アイドル 1 コア 13〜22%（07-31）／(54) `ghost-window-zorder` 起票＝owner 無し・全 `SWP_NOZORDER`・「語彙完備・配線ゼロ」3 例目（07-31）／**(55) 欠番**（atom 起票の本体がマージで脱落・内容は atom brief＋(57)⑴ が正本）／(56) 未登記先送り棚卸＝孤児 17 件・新規 brief 4 本起票（08-01）／(57) van 完了＝S5 担当が atom で確定・確定台帳の裁定付き訂正運用が確立・檻の空虚性通算 9 例（08-01）／(58) 棚卸⑤＝W5 3本マージ後の全面棚卸・全配置確定・干渉行列 55 ペア再実測（08-01）／(59) col 実装中の申し送り＝cage brief ①インベントリを 95 呼出/12 モジュールへ拡大（08-05・PR#101・cage brief 内が全文正本）／(60) 棚卸⑥＝col マージ後の軽量アンカー監査・実害ドリフト 2 ファイルのみ（08-06）／(61) `file-slimming` 起票＝W5.95 新設・肥大の 7〜8 割は in-file 檻（08-06）／(62) 棚卸⑦＝slimming 後の干渉台帳全面再解決・緩和 3 ペア（08-11）／(63) `bindoption-exclusivity` 完了＝根因 2 層・mayuna R4.5/D11 覆し・25 分実機 PASS（08-11）／(64) `ghost-window-zorder` 完了＝案 A owner 確定・topmost 帯引き込み是正 `HWND_TOP`（08-13）。

**2026-08-14 追記(65)（棚卸⑧＝W6 完走後の全面再解決）**: `/kiro-discovery` 再入。前回⑦以降の main 差分は W6 の 4 実装（bind PR#105／vis PR#106／zorder PR#107／scg PR#108・計 +35,766 行）＝**W6 完走**。①**brief 無し spec=0・新規起票 0 を確認**（残 11 brief 全実在・spec.json 無し＝走行中 spec 無し・completed 直下 156 一致）。②**ウェーブ骨格は不変**（W6.5→W6.75→W6.9→W7）だが 2 点改訂: ⑴**atom の第 1 段再観測が W6 中に実施されなかった**ため、要件フェーズ（コード非接触）を W6.5 併走へ繰り下げ（design 以降 W6.75 は不変・W6.75 の形は再観測結果で確定）⑵旧「W6 併走（文書のみ）」行は W6.5 行へ統合。③**干渉台帳を W6 後の実形へ再解決**——退役 5 ペア（scg⇄wpl・cage⇄vis・atom⇄vis・zorder→vis・atom⇄zorder）・生存 5 ペア＋因果 2。show.rs アンカードリフトを実測反映（budget :68-88・atom :220-270 帯・cage④ :227-232）・exact の read.rs :109 は現物一致。**scg⇄wpl の退役は「解消」ではなく rebase 義務への転化**——干渉面が resolver 外（`chain_finalize.rs`／`drain_resnap.rs`／`spawn.rs` `default_char_pos`）へ拡大した旨は wpl brief の scg 申し送りが正本（同ブロックは brief ローカルで「追記(63)」を名乗る＝roadmap (63) とは別物・㊻型の番号衝突として改番せず注記のみ）。④**roadmap 減量**: W6 行全文（編成条件⑴〜⑹）・完了サマリの col/vis/scg 詳報バレット・追記(58)(60)(61)(62)(63)(64) 全文を history へ退避し、完了ゴール表 6 行を削除（完了サマリへ集約）。⑤負荷過大の分割候補は atom のみ（既登記・3 関心）＝分割判断は再観測後の既定路線を維持。粒度調整の新規事案なし。

**2026-08-14 追記(66)（開発者裁定＝性能要件を最優先・`recompose-budget` を W6.5 へ前倒し）**: 開発者指示「CPU パワーを食っている原因の調査は性能要件ゆえ最優先」を受け、budget を W6.75→**W6.5** へ前倒し（優先順位の第 1 項「挙動バグ」の中でも性能を筆頭へ）。干渉再確認: ⑴exact⇄budget は棚卸⑦で緩和済み（`presenter/read.rs` ∥ `presenter/show.rs`＝別ファイル）⑵budget⇄wpl はドメインごと素（presenter 系 ∥ placement 系）⑶budget⇄atom は W6.5 では非接触（atom は文書フェーズのみ）で、台帳の推奨順序「**budget 先着→atom が 859ms を再測**」が自然成立＝前倒しは編成を強化する。W6.5 は 3 実装並走（budget ∥ exact ∥ wpl）＋atom 文書併走の 4 本体制へ。W6.75 は atom（＋bod）のみへ縮小。

**2026-08-14 追記(67)（開発者裁定＝atom の文書併走を不採用・直列方針へ）**: 開発者判断「結局並走できない（実装まで流せない）なら普通に進めたい」を受け、atom の W6.5 文書併走（要件フェーズのみ・⛔ design ゲート付き）を**不採用**とし、**W6.5 の 3 本着地後に W6.75 で単独フルライフサイクル**へ改めた。直列化の利点: ⑴第 1 段再観測が budget 着地後の settled main 上になり「残る跳ねコスト＝atom の取り分」が**一回の観測で**確定（pre-budget 基線の再採取は不要と裁定・旧 859ms は van S1 是正で失効済みのため基線価値なし）⑵「要件で止める」運用ゲートが消える⑶design 前 rebase が一度で済む。代償＝W6.75 の形（bod 統合可否）の確定が atom 着手時まで遅延するのみで、bod は W6.75 内で atom に従うため実害なし。

**2026-08-15 追記(68)（`draw-load-parity` 起票＝W8 新設・優先度低）**: `/kiro-discovery` 再入（Path C）。`recompose-budget`（W6.5）の実測が「**削るべき対象は自分の境界の外にある**」ことを示したため分離起票。①**budget は目的を達したが要件 4.4 には届かない**——1 コマ適用を 22,210→1,240µs（18 分の 1）まで削り CPU を 24.9%→約 11% へ半減させたが、**`apply_show` は CPU の 3.3% しか占めない**（着手前でも 10.4%）ため、同じ手段では残り 7 ポイントが動かない。②**SSP との同一手順比較を初めて実施**（同一マシン・同一ゴースト emo2・同一拡大率 200%・25 分・1 分ごと採取）＝**areka 10.97% 対 SSP 3.05%（3.6 倍）**・底 3.60% 対 1.77%（2.0 倍）・**頂 20.42% 対 4.64%（4.4 倍）**・Private 163.4MB 対 54.2MB・スレッド 83 対 32。**要件 4.4 の「SSP 実測 2.2〜2.8%」は出所不明のまま使われていたが、今回の実測 3.05% で裏が取れた**（較正の穴が 1 つ埋まった）。③**負荷の実体を計測で特定**＝`try_tick_world` が 13 スケジュールを **120回/秒** 全部回している（1 tick 578µs・壁時計 6.85%・**tick の 98% は表示に変化なし**）。内訳は FrameFinalize 182µs(31.5%)＋Draw 143µs(24.8%) で 56%。クリック透過の毎フレーム再評価は疑ったが**実測 5.2µs＝0.07% で無罪**（文書の機序と算術の符合だけで犯人を決めてはならない実例）。④**⚠ 比較の前提に未検証の穴**——SSP は 100% 描画→200% 引き伸ばしの可能性が高く（開発者観察＝バルーンの文字がぼやける）、事実なら画素の仕事量が 1/4 で「3.6 倍」は画素あたり 0.9 倍＝互角以上。**目標を CPU の絶対値で置くか画素あたり効率で置くかは要件段階の開発者裁定事項**（「SSP 水準」が暗に品質低下を要求している可能性がある）。⑤**バグと言える挙動は解決済み**（開発者判断）——メモリリークは Private 12分→24分で **+0.1MB**・ハンドル/スレッドとも増加なし、単調上昇は **+0.84 %/分 → −0.045 %/分**（判定「単調上昇」→「頭打ち」）。**本 spec はこれらを再発させないことだけが責務**。⑥配置は **W8（M1 完成後・優先度低）**＝M1 完成を妨げない。atom（W6.75）とは `tick_bridge` の flush 経路で干渉しうるので着手時に順序調停。⑦**引き受けるのは 4.4 だけではない**——`recompose-budget` の判定式**⑵（定常状態の進行境界スキップ＝catch-up 0 件）も未達のまま送られる**（release 17 件・dev 22 件・長時間 69 件／ベースラインは dev 78・release 41・長時間 82）。⑷a と**同じ走行で同時に落ちた 2 件**で、1 コマが 1,240µs（コマ待ち 150ms／22ms に対し桁で余裕）まで下がっても消えない以上、遅れの出どころは適用経路の外＝フレーム駆動側であり、**その負荷が下がれば 1 コマがコマ待ちを超える機会が減る**。⑵ も `draw-load-parity` が引き受ける（登記は同 spec brief の `## Problem`／`## Desired Outcome`／`## Upstream` と `recompose-budget` requirements の Requirement 4 改訂欄）。
