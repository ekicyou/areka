# Brief: areka-P0-recompose-budget

> **起票 2026-07-31**（`/kiro-discovery` 再入・`areka-P0-dpi-window-vanish` の task 4.5 実機セッション中に開発者が「CPU を食う・描画が重い」と発見）。
> 本 brief は**実測証拠と所有権判断を全て内包**する。別セッションはこの brief 単体で再開できる（会話ログは不要）。

> **📌 2026-08-06 追記(60)ドリフト補正（棚卸⑥・col=collision-dpi-hittest PR#100 マージ後の実測）**:
> - col が presenter.rs 冒頭へ +17 行＝`apply_show` 一族が一様シフト（形は全て不変）: fn `apply_show` :343 → **:360**（終端 :600 → **:614**）・毎フレーム `compose` 経路 :377 → **:394**・resample 再確保 :386-392 → **:403-409**（:406 `ComposedSurface::new(0,0)`・:407 `resample(...)`）。`Target` 構造体 :76-105 は**無シフト**。
> - exact との異ハンク判定は col 後も成立（本 spec=:386-417 ∥ exact=:676-683）。atom との同ハンク級（apply_show :360-614 同一関数内）・cage④（:527-531 近接）は不変＝W6.75 の編成条件に変更なし。

> **📌 2026-08-11 追記（bindopt 完了セッションでの実測更新・症状の格上げ・ループ設計）**:
>
> **⑴ bind（W6）着地＝Open Questions ③は解消・仮説 (a) は構造的に消滅**
> `areka-P0-bindoption-exclusivity` が本日実装完了（validation GO・実機サインオフ合格）。非宣言カテゴリは排他置換（高々 1 個）になり、さらに bind から外れた ID の残留コマ掃除が 3 段（state/looper/plan）で入った。**「bind 蓄積で合成要素数が単調増加する」機序は構造的に存在しなくなった**——今後の測定で CPU の時間上昇が残るなら (b)（活性集合の定常到達）か未知の機序であり、切り分けは roadmap:90 の予告どおり今すぐ実施可能。
>
> **⑵ 実測更新（2026-08-11・dev ビルド・実機サインオフ実走 4 本から採取）**
> - `apply(ShowSurface)` は **1 コマあたり約 500ms**（dev・3 群で同型再現: 493/484ms・527/515ms・543/417ms）。7/31 の release 実測 143ms と整合（dev ≒ 3x）
> - 7 分実走で ShowSurface **404 件すべて `cache_hit=false`**（アニメ再生中はキー不一致で必ずミス＝原因②の実走裏付け）
> - `areka-emo-present` に throttle/debounce は**存在しない**（grep 0 件）＝スロットリングではなく実コスト
> - **seriko 側の進行は壁時計どおり**（発火→末尾残留 183ms・定義値 172ms に一致）＝遅いのは presenter 単独
> - 開発者観測: 実走中 CPU **20% 超**
> - **【症状の格上げ】まばたきアニメーションのスロー再生が目視できる**——1 コマ約 500ms 保持のため、まばたきがコマ送りで見える。開発者の言葉:「このシェルでこんなに重いのは異常」。**アイドル税だけでなくユーザー可視の描画劣化**であることが確定
> - 測定材料（保全済み・md5 付き・`%LOCALAPPDATA%\areka-diag\`）: `bindopt-signoff-20260811-181137\`（7 分）・`bindopt-signoff2-20260811-195927\`（7 分）・`bindopt-j3-long-20260811-202648\`（**25 分**・1.3MB）・対照 `bindopt-20260811-101835\`
>
> **⑶ アンカー補正**: bindopt は presenter.rs を触っていない（毎フレーム経路は不変）。ただし `plan.rs` の合流ループに bind 所属ゲート 1 条件が入った（O(1)/コマ・微小だがホットパスに実在——本 spec の測定では bindopt 着地後の今日のログが新ベースライン）。`state.rs::commit_bind` に残留コマ除去（bind 変化時のみ・毎フレームではない）。
>
> **⑷ 問題解決のループ設計（本 spec の要件フェーズが具体化する・開発者依頼 2026-08-11・同日改訂）**
>
> **自律の原則（開発者裁定 2026-08-11）**: 「何が重いか」の特定から是正の効果判定まで、**開発者の介在なしに agent が自走で完結**させること。bindopt の J3 のような目視必須項目を**作らない**——本 spec の観測対象は全てログへ写せる量であり、目視に相当する「スロー再生の解消」も機械判定できる（コマ適用間隔 ≒ SERIKO 定義値 172ms への収束を閾値判定）。実走は `AREKA_APP_SMOKE_EXIT_MS` 有界 auto-exit＋`RUST_LOG` grep の確立済み流儀（steering）で、CPU サンプリングも `Get-Counter` の 15 秒刻みで自動採取——**人が画面を見る工程はゼロ**。
>
> 0. **第 0 段＝計時ログの新設（最初の実装タスク・これが無いと「何が」に答えられない）**:
>    現状のログでは 1 コマ約 500ms の**内訳が原理的に分からない**（本日の 500ms も既存ログの行間隔からの間接推定）。`presenter.rs` の `apply_show` ホットパスへ**段階別計時ログ**を新設する——最低限の分解は ⒜ cache 照会、⒝ `compose`（さらに `build_plan`／`blit::execute` を分けられれば理想）、⒞ リサンプル（`resample`・:403-409）、⒟ `AlphaMask` 生成、⒠ WUC/GPU 面への転写・描画呼出、⒡ 合計。**grep 可能な固定文言＋数値フィールド**（`signoff-scan.py` 方式の判定スクリプトが機械集計できる形）で、水準は debug（常設・`RUST_LOG` でオンオフ・恒久資産として残す）。アロケーション回数（バッファ新規確保の発生点）も同時に数えられる形が望ましい。**計時ログ自体の檻**（文言・水準・全段の出現）も同時に入れる——道具が壊れていたら診断が全部嘘になる（較正の規律）。
> 1. **第 1 段＝ベースライン採取（自動）**: 第 0 段のログ入りで有界実走（7 分＋20 分超の 2 水準）→ 判定スクリプトで段階別 p50/p95・cache_hit 率・ticker catch-up 件数・コマ適用間隔・CPU 時系列を自動集計。**ここで初めて「何が重いか」が数値で確定する**（本日の保全ログ 4 本は計時なしの対照として保存済み）
> 2. **第 2 段＝是正**（要件変更なし・仮説は第 1 段の実測が選ぶ）: 現時点の最有力は `compose_into` 切替＋`Target` 再利用バッファ＋`AlphaMask` 再確保見直し（brief 本文 Approach）だが、**第 1 段の内訳が別の犯人（例: リサンプルや GPU 転写が支配的）を指したらそちらを先に潰す**——推測でなく実測が順序を決める
> 3. **第 3 段＝再測（自動）**: 第 1 段と同一手順・同一 fixture で採取し差分を機械判定。合格判定式（要件フェーズで確定）の候補: コマ適用間隔の p95 ≦ SERIKO 定義値＋許容率／ticker catch-up 0 件／定常アロケーション 0／アイドル CPU ≦ 目標値
> 4. **判定分岐（自動）**: 合格 → アロケーション回数・呼出回数の決定論檻で固定して完了 ／ 不合格 → 残る最大内訳に対して 2 へ戻る。**キャッシュ容量の変更だけは要件変更ゆえ開発者裁定が要る**（Open Questions ①・自律ループの唯一の例外）
> 5. 檻は実時間でなく**アロケーション回数・呼出回数**で表現（brief 本文 Constraints どおり）。実時間の閾値判定は実機実走の判定スクリプト側にのみ置く（マシン差は較正値として明記——bindopt J2 の前例）

## Problem

**誰の問題か**: エンドユーザー（ゴーストを常駐させる利用者）と、実機サインオフを行う開発者。

**症状（実機観測・2026-07-31）**: ゴーストを起動して**放置しているだけ**で CPU を食い続ける。開発者の言葉では「アニメーションしてないときも負荷があるのはおかしい」「しばらく見ないと（分かるほど時間とともに上がる）」。

**痛み**:
1. デスクトップマスコットは**常駐**が前提。放置で 1 コアの 2 割を食うのは製品として成立しない。
2. **実機サインオフの妨げ**になる。dev ビルドでは 1 コアの 45% を消費し、ドラッグ操作が困難で `areka-P0-dpi-window-vanish` の task 4.5 採取が実施しづらかった（release へ切り替えて回避）。以後どの spec の実機検証にも同じ税がかかる。

## Current State

### 実測（2026-07-31・実機 2 モニタ混在 DPI 192/144・fixture emo2）

| 指標 | dev ビルド | release ビルド |
|---|---|---|
| アイドル時 CPU（1 コア換算） | **45.3%** | **13.4% → 21.6%**（時間とともに上昇） |
| `apply(ShowSurface)`＝全再合成 | 1.7 回/秒 | 1.4〜1.6 回/秒 |
| 1 合成あたり CPU | — | **約 143ms**（764×1094 px） |
| `cache_hit=true` / `false` | **0 件 / 410 件** | **0 件 / 18 件**（同傾向） |
| WorkingSet・ハンドル・GDI | — | 横ばい（**リークではない**） |

- SERIKO のアイドルアニメ（`animation_id=1400`／`1402`）が **60 秒あたり約 50 回**発火し、その各コマが全再合成を引く。目視で「止まって見える」瞬間もまばたき等が回っている。
- **`loop ticker catch-up: skipped multiple boundaries, firing once`** が出ている＝**ticker が追いつけず境界をスキップしている**。これが「描画が重い」の直接の現れ。
- 証跡ログ: `%LOCALAPPDATA%\areka-diag\20260731-163422-rel\session1-drag.log`（release）／`...\20260731-162340\session1-drag.log`（dev・24 分走行）。

### 構造的原因 ①: 毎フレーム用 API が本番で一度も呼ばれていない（**コード読解で確定**）

`crates/areka-emo-compose/src/lib.rs:117` の `Composer::compose_into` は doc にこう明記されている:

> 合成先バッファ再利用形（**毎フレーム経路**・定常状態アロケーションなし・**要件 10.3**）

対する同 `:149` の `compose`:

> 新規割り当て形（**初回・テスト向け便宜**）… **毎フレーム経路では代わりに `compose_into` でバッファを再利用すること（要件 10.3）**

**`compose_into` の本番呼出点はゼロ**（`grep -rn "compose_into" crates/ --include=*.rs` の一致は lib.rs の定義と blit.rs:67・plan.rs:35,505 の doc 参照のみ）。毎フレーム経路である `crates/areka-emo-present/src/presenter.rs:377` は**確保版の `compose` を呼んでいる**。

さらに `presenter.rs:386-392` のリサンプル経路も毎回 `ComposedSurface::new(0, 0)` を新規確保する。`Target` 構造体（`presenter.rs:76-105`）は `composer`／`cache` を保持するが、**合成先・リサンプル先の再利用バッファの席が無い**（`Composer` の doc「状態非保持・スクラッチのみ再利用」が指すのは内部 `ops`／`visited` であって出力バッファではない）。

結果、1 フレームごとに native 原寸バッファ（382×547×4 ≒ 836KB）＋リサンプル先（200% DPI で 764×1094×4 ≒ 3.3MB）＋ `AlphaMask` を新規確保している。

**上流要件の原文**（`completed/areka-P0-emo-compose/requirements.md:153`・Requirement 10 AC 3）:
> When 合成を実行するとき, the emo-compose Compositor shall 合成先バッファを再利用し、アトラス転写を O(elements) で行い、**途中アロケーションを発生させない**。

※ この AC は Compositor 側の契約であり、`compose_into` はそれを満たしている。**破れているのは消費側（emo-present）の呼び方**である。`emo-present` 側には毎フレームのアロケーション予算を定める AC が**存在しない**——これが構造的な穴。

### 構造的原因 ②: 合成キャッシュ容量 1 は**承認済み要件**であって実装バグではない

`crates/areka-emo-present/src/cache.rs:100` の `ComposeCache` は `slot: Option<(ComposeKey, CacheEntry)>` ＝直前 1 件のみ保持。キーに SERIKO の `PatternState` が含まれるため、**アニメ再生中は毎コマ必ずミスする**。

これは仕様どおりである。`completed/areka-P0-emo-present/requirements.md:89`（Requirement 4 AC 1）:
> The emo-present レイヤ shall 合成入力（surface id と bind 集合）をキーに、直前の合成結果を保持するキャッシュ（**容量 1 のメモ化スロット**）を備える。

つまり**容量を広げることは承認済み要件の変更**であり、勝手にやってはならない。本 spec の要件フェーズで裁定を仰ぐ（下記 Open Questions ①）。

### 未解明（仮説段階・断定しないこと）

**なぜ CPU が時間とともに上がるのか（13.4% → 21.6%）は突き止めていない。** 候補:
- (a) bind 集合の蓄積で合成要素数が増える＝`areka-P0-bindoption-exclusivity`（W6）の表情固着と**同根**の可能性。非宣言カテゴリが加算飽和してパーツが積み上がるなら、`build_plan` の O(elements) が単調増加する
- (b) 起動直後は活性アニメが少なく、会話が進むにつれ活性アニメ集合が増えるだけ（＝定常に達すれば頭打ち・欠陥ではない）
- 計測は 15 秒刻みで 1 分強しか取れていない（`%LOCALAPPDATA%\areka-diag\cpu-samples.csv`）。**長時間サンプリングが要件フェーズの最初の仕事**

## Desired Outcome

- ゴーストを放置したとき、アイドル CPU が**実用水準**に収まる（数値目標は要件フェーズで確定・現状 1 コア 13〜22%）
- `loop ticker catch-up`（境界スキップ）が定常状態で出ない
- 毎フレーム経路が定常状態でアロケーションを起こさない＝`emo-compose` 要件 10.3 の意図が**消費側でも成立**する
- 実機サインオフが CPU 税なしで実施できる

## Approach

**採用**: 段階的に、**測ってから広げる**。

1. **第 1 段（原因①の是正・要件変更なし）**: `presenter.rs` の毎フレーム経路を `compose_into` へ切り替え、`Target` へ合成先・リサンプル先の再利用バッファを持たせる。`AlphaMask` の再確保も同様に見直す。**承認済み要件を 1 つも変えずに実施できる**（むしろ上流 AC 10.3 の意図へ近づく）。
2. **第 2 段（効果測定）**: 同一手順で CPU を測り直す。第 1 段で足りるかを**実測で判定**する。
3. **第 3 段（足りない場合のみ・要件変更）**: キャッシュ容量をアニメのコマ数に見合う形へ広げる設計変更を、**選択肢として開発者へ提示してから**行う（emo-present R4.1 の改訂）。

**なぜこの順か**: ①は要件変更を伴わず、②の測定基盤がそのまま第 3 段の判断材料になる。逆順（先にキャッシュを広げる）だと、承認済み要件を変えたうえで「実は①だけで足りた」となる危険がある（[[analyze-ideal-form-not-minimal]] の「解決/未解決を明示」）。

**棄却した案**:
- **dev ビルドを諦めて release 前提にする**: 症状を隠すだけ。release でも 1 コア 13〜22% を消費しており、常駐アプリとして成立しない。
- **アイドルアニメの発火頻度を落とす**: 正典（SERIKO のアニメ定義）はゴースト作者の意図であり、baseware が勝手に間引くのは互換性違反（[[areka-compat-baseware-strategy]]）。
- **既存 spec へ相乗り**: 下記「Existing Spec Touchpoints」のとおり所有者が全て completed で消化不能（[[deferral-requires-verified-owner]]）。

## Scope

- **In**: `areka-emo-present` の毎フレーム経路のアロケーション是正（`compose_into` への切替＋再利用バッファ）。効果の実測手順と数値目標。定常状態アロケーションを固定する檻。CPU 上昇機序の切り分け（原因(a)/(b) の判別）。
- **Out**:
  - キャッシュ容量の変更（第 3 段・**要件変更ゆえ別途裁定**。裁定が下りれば本 spec 内で実施）
  - `bindoption` の 3 値意味論是正（`areka-P0-bindoption-exclusivity` の所有）
  - `ScaleRatio` の f32 排除（`areka-P0-scale-exact-rational` の所有）
  - SERIKO のアニメ発火頻度・正典解釈（`completed/areka-P0-seriko-loop` の領分・触らない）
  - GPU 側（swap chain・WUC 更新）の最適化——**まず CPU 側の確定分を潰してから測り直す**

## Boundary Candidates

- **合成の呼び方**（`presenter.rs` の compose/resample/mask 経路）＝本 spec の中核
- **キャッシュ容量政策**（`cache.rs`＋emo-present R4.1）＝要件変更を伴う独立の裁定点
- **計測手順**（アイドル CPU の再現可能な測り方）＝本 spec が新設し、以後の性能 spec が再利用する資産

## Out of Boundary

- SERIKO・ghost・kanade の駆動側（発火頻度・スケジュール）
- `emo-compose` の合成アルゴリズム本体（`build_plan`／`blit::execute` の中身）——O(elements) 契約は既に満たされている前提で、破れていたら**別途起票**
- 実機 GPU 性能・ドライバ差

## Upstream / Downstream

- **Upstream**:
  - `completed/areka-P0-emo-compose`（要件 10.3 の原文所有・`compose_into` の提供元）
  - `completed/areka-P0-emo-present`（要件 4.1 のキャッシュ容量政策所有・改修対象コードの所有）
  - `areka-P0-bindoption-exclusivity`（W6）— **CPU 上昇機序の候補 (a) の所有者**。bind 蓄積が合成要素数を押し上げているなら、bind 側の是正が本 spec の負荷も下げる。**bind 着地後に測り直すのが最も切り分けやすい**
- **Downstream**:
  - 以後の全 spec の実機サインオフ（CPU 税の除去）
  - `areka-P0-emo2-conformance-e2e`（W7）— 適合一周走行の実施環境

## Existing Spec Touchpoints

- **Extends**: なし。**所有者 `emo-compose`・`emo-present` はいずれも `completed/` にあり消化不能**（[[deferral-requires-verified-owner]]「completed は消化不能」）。ゆえに新規 spec が要る。
- **Adjacent（干渉台帳へ登記すること）**:
  - **`areka-P0-scale-exact-rational`（W6.5）— 同一ファイル `crates/areka-emo-present/src/presenter.rs`・異ハンク**。exact は `:659-666`（`TextSlotView` の scale 供給）、本 spec は `:369-400`（compose/cache/resample ブロック）。**先着後 rebase**。責務も別（exact＝f32 排除の正確性／本 spec＝アロケーション予算）
  - **`areka-P0-test-cage-determinism`（W6.5）— `areka-emo-present/src/scale.rs` の `mod tests` を触る**。本 spec は `presenter.rs`／`cache.rs` の `mod tests` ゆえ**別ファイル＝素の見込み**だが、着手時に実測再突合すること
  - `areka-P0-balloon-visibility`（W6）— emo-present を消費するが presenter 内部は触らない見込み
- **合流しない相手（判断の記録）**:
  - **`areka-P0-dpi-window-vanish`（W5・実装中）とは合流しない**。①ドメインが別（窓の位置権威／可視性 vs 合成の実行予算）②`tasks-generated`＋3 フェーズ承認済みで task 4.5 の実機ゲート待ち＝承認済み成果物を実装中に膨らませることになる ③ファイル集合が素（van＝`placement/*`・`emo2_boot/frame.rs` の DPI/resnap/drain ハンク・`wintf` window_proc／本 spec＝`areka-emo-present/{presenter,cache}.rs`）

## Constraints

- **承認済み要件を勝手に変えない**: emo-present R4.1（容量 1）の変更は裁定必須
- **ゴースト fixture・SERIKO 定義の改変は禁じ手**（症状を隠すだけ）
- 常時テストは x86 を避け純 x64 決定論で（[[prefer-x64-fake-boundary-tests-not-x86]]）
- `cargo test -p areka` に `--bins` を付けない（examples が `#[path]` include）
- `cargo clippy -p wintf` は `com/d2d/command_sink.rs` の既存不良で失敗＝DoD に使わない
- 性能の檻は**時間ではなくアロケーション回数・呼出回数**など決定論的な量で表現する（[[deterministic-test-coverage-mandate]]・実時間はマシン差で非決定）

## Open Questions（要件フェーズで裁定・本節が正本）

1. **キャッシュ容量（emo-present R4.1）を変更するか。** 第 1 段の実測で足りなければ提案する。広げる場合の容量根拠（アニメのコマ数？ LRU？）と、R4.1 の「容量 1」を書き換える手続き。
2. **数値目標をどこに置くか。** 「アイドル時 1 コアの N% 未満」か、「毎フレーム経路の定常アロケーション 0 バイト」か、両方か。後者は決定論的に檻へ入る（推奨）が、前者はユーザー体感に直結する。
3. **CPU 上昇機序 (a)/(b) の切り分けをいつ行うか。** `bindoption-exclusivity`（W6）着地後に測り直すと切り分けが最も綺麗だが、それまで待つと本 spec の着手が W6 以降に固定される。
4. **ウェーブ配置**（下記「Wave 提案」）。

## Wave 提案（開発者裁定要）

**推奨: W6.5**（`scale-exact-rational` ∥ `test-cage-determinism` と同居）。理由:
- `bindoption-exclusivity`（W6）着地後なら CPU 上昇機序 (a) の切り分けが綺麗になる
- exact とは `presenter.rs` 異ハンクで先着後 rebase・cage とは別ファイルで素の見込み＝W6.5 の 3 本目として収まる

**対抗案: 前倒し（W5 の後・W6 と並行）**。理由: **実機サインオフの税**であり、W6/W6.5/W7 の全ての実機検証がこの負荷の下で行われる。早く潰すほど後続が楽になる。ただし bind 未着地ゆえ (a)/(b) の切り分けは持ち越しになる。
