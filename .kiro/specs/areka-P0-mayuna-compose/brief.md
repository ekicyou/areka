# Brief: areka-P0-mayuna-compose

> **種別**: 本坑（main）増分。⑤ seriko 帰属だが**②parsers・④sakura・⑤seriko・⑥emo を貫く垂直スライス**（`areka-P0-balloon-face-cue` と同型）。
> 由来: 2026-07-13 M-boot（`areka-P0-emo2-boot`）R9.3 実機サインオフの実機欠陥 **#2「着せ替え（bind）で表情変化せず」**。roadmap「M-boot 実機サインオフ発見」節 #2 が本 spec を名指し（「登録済み増分 `mayuna-compose`（⑤seriko）を②④⑤垂直スライスへ再スコープ」）。
> **調査日**: 2026-07-13（実 emo2 murasaki スクリプト接地＋実装現況精査）。
> **⚠️ 調停（2026-07-18・`areka-P0-sakura-dialogue-tags` 要件ディスカッション帰結・受領済み）**: `\![...]` ベースウェアコマンドは **typed variant を個別新設せず、W1 が確立する単一の汎用コマンド cue**（コマンド名＋生引数列の不透明転写・空トークン保持・消費側のコマンド名選別・未知名は全消費者が良性スキップ）**に乗る**。根拠＝正典実測でコマンド 183 個（下限・52 族・非有界）に対し emo2 実使用は move/bind の 2 個（1%）＝型付き個別実装は破綻。境界原則: **コンテンツタグ（`\s`/`\b` 等）＝typed cue／`\!` コマンド名前空間＝汎用キャリア**——本 brief の「balloon-face-cue と完全同型」引用はコンテンツタグ側にのみ有効で `\!` へは延長しない。これに伴い下記 Approach を読み替える:
> - **step 2（dola `CueCommand::Bind` variant 追加）＝廃止**。bind は W1 汎用キャリアに乗る（dola 無改修）。
> - **step 3（compile bind アーム・`cue_target_of` Bind→Shell・emo-text 無視列挙追加）＝廃止**。`\!` の転写・emo-text 側の無視アームは W1 が汎用キャリアに対し一度だけ実装済みとなる（以後コマンドが増えても共有 4 ファイルは不変）。
> - **step 1（parsers 名前解決表）・step 4（seriko 動的 bind 状態——ただし入力照合は typed variant でなく「汎用 cue のコマンド名 `bind`」の名前ゲート＝`SerikoSink` の BalloonSurface 処理と同型）・step 5（emo-present 回帰）＝存続**。
> - 帰結: 本 spec は共有編集面 4 ファイル（dola command.rs／sakura compile.rs／dola sink.rs／emo-text state.rs）を**触らなくなり**、下記クロスユニット契約の「近接編集」警告は「W1 汎用キャリア契約への一方向依存」へ解消される。1 コマンド名の担当消費者は高々 1（単一権威表）＝`bind` の担当は seriko として同表に登記する。

## Problem

emo2 のむらさきは表情を **`\s[1000]`（本体サーフェス）＋ `\![bind,category,partname,1/0]`（名前キーによる着せ替え bind の on/off）連打**で作る（実機観測: `\![bind,腕,伸び,1]` 等 6 連）。ところが areka はこの **名前キー bind を一切処理せず捨てている**ため、実機でスクリプトが表情を変えても**むらさきの見た目が変化しない**（bind パーツが乗らない＝素の surface1000 のまま）。「着せ替えで表情が変わる」は emo2 の中核表現であり、この欠落は M-boot の目視品質を損なう。

**三重の配管欠落（実装現況で確定・2026-07-13）:**
- **②parser**: `\![bind,腕,伸び,1]` は `Instruction::GenericCommand{name:"bind", raw_args:["腕","伸び","1"]}` へ**転記までは成功**（`crates/areka-parsers/src/sakura/decode.rs:301`・fixture 適合 test `validation_tests.rs:381-397` が emo2 murasaki の6連を固定）。だが**名前キー（category＋partname 文字列）→ bindgroup ID/animation ID の解決経路が存在しない**（`sakura.bindgroupN.name` 解決は parsers/mount のどこにも無い＝`.default,1` のみ転記〔`package/resolve.rs:107-149`〕）。
- **④sakura**: `GenericCommand` は compile の catch-all `other =>`（`tracing::debug!` 無視・#60 後の現行 `compile.rs:120-122`〔旧 :92〕）で**drop され cue 化されない**（dola へ届かない）。
- **dola**: `CueCommand` enum（`crates/dola/src/cue/command.rs`）に **bind variant が不在**（#60 後は **10 variant**＝旧8＋`Wait`/`ClearAll`〔追記㉗〕。`BalloonSurface`/`Wait`/`ClearAll` の additive 追加実績＝**本 spec の追加テンプレート**）。
- **⑤seriko**: `static_binds: BindSet`（`state.rs:56`・構築時一度きり恒等写像 `bind.rs:18-20`）は在るが、**実行時の動的 bind 切替 API は不在**。seriko は本 spec のためのシームを**既に明記予約**している（`state.rs:44-46`「per-scope マップと静的 `BindSet` を同居させ…後続の動的切替ユニット（`mayuna-compose`）が `static_binds` の置き場のみを差し替えられる形にする。本ユニットは bind の切替 API を持たず」）。

## Current State

- **⑥emo-compose は bind を静的合成済み**（M-boot で所有）: 合成は `(surface_id, BindSet)` の純関数。有効 bind の pattern0 を animation ID 昇順で静的合成する経路は**実装済み**（emo2 surface1000＝全 bind 合成が M-boot で動いている）。**欠けているのは「実行時に BindSet を差し替える」動的側だけ**。
- **⑥emo-present の再合成経路は実在**（まばたきの前例）: `ComposeCache` は `(surface_id, BindSet)` 完全一致キーの容量1メモ化（emo-present 追記⑫で是正済み）。**新 BindSet で `ShowSurface` を発行すればキャッシュミス→再合成→再表示**が自然に流れる（まばたき開閉で実証済み）。ゆえに本 spec は「正しい動的 BindSet を seriko が生成し `Show{binds}` へ載せる」だけで表示が変わる。
- **`DisplayCommand::Show{scope,surface_id,binds}`**（`crates/areka-seriko/src/output.rs:30`）は既に `binds` を運ぶ。動的切替 variant は不要＝**bind 変化時に新 `BindSet` を載せて `Show` を再発行**すれば足る（emo2-boot adapter `map_display_command` が `PresentCommand::ShowSurface{binds}` へ写像・`adapter.rs:37-46`）。
- **balloon-face-cue（✅完了）が垂直スライスの完全な前例**: parser variant→dola `CueCommand` variant→sakura compile 写像→`cue_target_of` で seriko 行き→seriko per-scope 状態→表示指令、の背骨を `\b` で実証済み。本 spec は**同じ背骨を bind（名前キー＋on/off）で再演**する。

## Desired Outcome

実機でスクリプトの `\![bind,category,partname,on/off]` が**むらさきの表情パーツを実際に着脱**し、着せ替えが目視で反映される。名前キー（category/partname）は bindgroup へ正しく解決され、on/off が per-scope の bind 状態へ積算され、変化時に emo が再合成して表示する。

**✔ 観測（単一 pass/fail）**: fixture script 直入力（`\![bind,腕,伸び,1]`／`,0` の on/off 列）→ **mock 表示 sink** で per-scope bind 状態の積算と `Show{binds}` 再発行の**決定論観測**（sleep 不使用・注入 Tick）。加えて **実 emo2・実 pasta・実 DPI** で表情変化の人間サインオフ（R9.3 系・本番ゴースト先行）。

## Approach

**balloon-face-cue と完全同型の垂直スライス**（`\s` / `\b` に次ぐ第三の第一級 cue 語彙化）。左→右の5層 additive:

1. **②parsers — 名前キー解決基盤**（唯一の真の新規設計）: `sakura.bindgroupN.name`（＋`kero.bindgroupN.name`）宣言を descript から転記し、**(category, partname) → bindgroup ID** の解決表を `MountModel` へ増設（既存 `BindGroupDefaults` に隣接・`.default,1` 転記〔`resolve.rs:107-149`〕と同じ層）。ukadoc `descript_shell` の bindgroup 命名規約を design で確定（**必読**下記）。
2. **dola — `CueCommand::Bind` variant 追加**（additive・`BalloonSurface` の line 144 追加を踏襲・serde ワイヤ形は既存不変＝test `command.rs:314-329` に倣い新 variant を追記）。ペイロードは不透明側に寄せる（category/partname/on-off を String/bool で持つ＝id 解決は seriko 下流〔記憶 areka-surface-args-opaque-string-downstream-resolve〕）。
3. **④sakura compile — bind アーム新設**: `GenericCommand{name:"bind",…}` を `CueCommand::Bind` へ写像（compile catch-all〔現行 `compile.rs:120-122`〕から救出）。**分類**: bind は表示系＝`cue_target_of`（**#60 で dola へ移動**＝現行 `dola/src/cue/sink.rs:50-67`・2026-07-17 実測）へ **Bind→`CueTarget::Shell`** アームを追加（seriko 行き・`\s`/`\b` と同じ棚・broadcast＋演者側 relevance の settled 配送モデルに従う）。emo-text `apply_cue`（catch-all 無しの網羅 match・Choice 分岐 `state.rs:224-229` 実測＝旧 :196-199 からドリフト）へ **Bind を明示無視列挙に追加**（bind は text でない＝誤配線防止）。
4. **⑤seriko — 動的 bind 状態**: 予約シーム（`state.rs:44-46`）を活かし **per-scope 動的 bind マップ**を導入（`static_binds` を初期値に）。`Bind` cue 受理で (category,partname) を bindgroup ID へ解決〔①の表〕→ on/off を積算→ **新 `BindSet` を載せた `Show` を再発行**（冪等ガード＝状態不変なら再発行しない・DD6 同様の決定論規律）。名前解決失敗は warn/error skip（balloon-face-cue の `resolve_balloon_key` 同型・シェル経路無改変）。
5. **⑥emo-present — additive 回帰のみ**: 動的 BindSet での再 Show が既存 `ComposeCache`（容量1・まばたき前例）を正しくミス→再合成することを test で固定（**本体無改変**が原則・balloon-face-cue 同様 test-only）。

## クロスユニット契約（並走を詰ませない事前考慮・2026-07-13）

- **`cue-playback-duration` との共有編集面**（相互調整・**最重要**）: 本 spec と `cue-playback-duration` は **dola `CueCommand`（command.rs）・sakura `compile.rs`・`contract.rs` cue_target_of・emo-text `state.rs` apply_cue の4ファイルを共有**する。ただし**第一次 locus は素（disjoint）**で、衝突は「catch-all を持たない網羅 match」2箇所（現行＝dola `sink.rs:50-67` `cue_target_of`／emo-text `state.rs` 網羅 match〔Choice 分岐 :224-229 実測〕・#60 で移動）への**別アーム追加**に限られる（本 spec は Bind アーム／cue-playback は Text アーム挙動変更）＝**マージ可能な近接編集**。**先決すべき契約**: cue-playback-duration が `Cue` 構造体へ duration を足すか schedule を duration 認識にするか（＝emit()/Cue 形の変更）を**先に確定**し、本 spec の `CueCommand::Bind` は**その確定形に載る**（bind は瞬時＝duration 0 でよい）。**推奨順序: cue-playback-duration 先行 → 本 spec が settled cue モデルへ additive**（下記 Upstream/Downstream）。契約先決なしの完全並走は emit() 署名のドリフトで齟齬るため非推奨。**【✅ 2026-07-17 充足＝即着手可】** cue-playback-duration 完了（追記㉗）＝settled cue モデル（envelope 一律 duration・瞬時は明示 0・単一 `CueSink`・broadcast・`cue_target_of` 単一権威）が main 着地済み——本 brief の branch 実測引用は最終形＝main を正として読み替える。**下流順序（2026-07-17 合流裁定）**: `seriko-loop` の実機サインオフ（sakura 側まばたき）は本 spec の `\![bind]` 貫通が前提＝**seriko-loop の完了ゲート**（同 brief に登記済み）。
- **emo-compose の再合成は不変**（消費のみ）: 動的 BindSet は emo-compose の純関数 `(surface_id, BindSet)` をそのまま駆動。本 spec は**新しい合成メソッドを足さない**（有効 bind の pattern0 静的合成は M-boot 所有）。
- **名前キー vs 番号キー**: `\![bind,ID,1]`（番号直指定）と `\![bind,category,partname,1]`（名前キー）の両形を design で確定（emo2 は名前キー使用が実測・番号形の要否は ukadoc で裏取り）。番号形は解決不要・名前形のみ①の表を引く。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-13）

- **必読**: `descript_shell` の **bindgroup 系キー全容**——`sakura.bindgroup{N}.name,カテゴリ,パーツ名`（名前宣言）／`.default,0|1`（既定 on/off）／`.addid` / `.mustselect` 等の関連キー。**MAYUNA** の項（`\![bind,…]` の category/partname/on-off 意味論・複数 bind の重ね順・排他グループ〔同一 category 内 mustselect〕）。`kero.bindgroup{N}.*`（相方側）。
- **必読**: さくらスクリプト `\![bind,…]` の**全引数形**（`get_doc` で確認: category+partname 名前形／ID 番号形／`\![bind,…,1]`=on・`,0`=off・トグル形の有無）。emo2 は名前形＋明示 on/off が実測だが正典で全形を押さえる。
- **具体指示**: design 冒頭で `get_doc('descript_shell')` の bindgroup 節と `\![bind]` の項を読み、**「名前解決に効くキー（→②parsers 増設）」「on/off・排他の意味論（→⑤seriko 積算）」「M1 対象外（addid/mustselect 等 emo2 未使用なら型シームのみ）」の3分類表**を design.md に載せること。emo2 fixture（murasaki の `sakura.bindgroupN.name` 実宣言）と正典の突合も。**正典は ukadoc・emo2 は最小適合 fixture**（記憶 ukadoc-mcp-preferred-source）。

## Scope

- **In**: ②parsers 名前キー解決表（`bindgroupN.name`→(category,partname)→id・`MountModel` 増設）／dola `CueCommand::Bind` variant／④sakura compile bind アーム＋`cue_target_of` Bind→Shell＋emo-text apply_cue Bind 無視追加／⑤seriko per-scope 動的 bind 状態＋on/off 積算＋新 BindSet で `Show` 再発行＋冪等ガード＋解決失敗 skip／⑥emo-present 再合成の test-only 回帰。決定論 mock sink 観測＋実 emo2 実 DPI サインオフ。
- **Out**: bind の**静的合成適用**（emo-compose が M-boot で所有済み＝再定義しない）／SERIKO ループ（blink＝`seriko-loop`・M-life）／二人立ちの kero 側 bind 本格結線（M-dual・シームのみ）／テキスト再生 duration（`cue-playback-duration`）／実行時 resize（`surface-resize-resnap`）／着せ替え**メニュー UI**（`\![bind]` の script 発火のみ扱う・選択 UI は M-dialogue/M2）。

## Boundary Candidates

- 名前解決（②parsers・唯一の新規設計）／cue 語彙化（dola variant＋④sakura 写像＝balloon-face-cue 同型・機械的）／動的 bind 状態（⑤seriko・予約シーム消費）／再合成回帰（⑥emo-present・test-only）。

## Out of Boundary

- 静的 bind 合成（emo-compose・完了済み）／blink ループ（seriko-loop・M-life）／メニュー選択 UI（M-dialogue）。

## Upstream / Downstream

- **Upstream**: `completed/areka-P0-seriko-engine`（予約シーム `static_binds`／`ScopeStates`／`DisplayCommand::Show{binds}`）／`completed/areka-P0-emo-compose`（`(surface_id,BindSet)` 純合成）／`completed/areka-P0-emo-present`（`ComposeCache` 再合成経路・まばたき前例）／`completed/areka-P0-balloon-face-cue`（垂直スライス背骨の完全前例）／`completed/areka-P0-emo2-boot`（#2 症状の出所＝実 pasta スクリプト接地）／**`areka-P0-cue-playback-duration`（推奨先行＝共有 cue モデルの settled 形へ載せる）**。
- **Downstream**: M-mayuna（マイルストーン＝本 spec 単独で構成）／将来の着せ替えメニュー（M-dialogue の選択肢が本 spec の bind 発火を駆動）／M-dual（kero 側 bind が同機構を再利用）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-P0-seriko-engine`（`static_binds` 置き場を動的マップへ差し替え＝予約どおり）。
- **Adjacent（相互調整）**: `areka-P0-cue-playback-duration`（**共有編集面4ファイル＝上記クロスユニット契約で先決・推奨は cue-playback 先行**）／`areka-P0-surface-resize-resnap`（**交差面ゼロ**＝placement/emo-present-size と bind は無関係＝完全並走可）／`completed/areka-P0-balloon-face-cue`（同型テンプレート・`resolve_balloon_key`→`resolve_bind_key` の対応）／**`areka-P0-sakura-dialogue-tags`**（2026-07-16 新 brief・**compile.rs catch-all 近接**＝別 variant 救出の additive アーム〔本 spec=`GenericCommand{bind}`・同 spec=Choice/Cursor/Move/SystemVar〕＝マージ可能・同時着手時は rebase 注意）／**`areka-P0-seriko-loop`**（2026-07-16 新 brief・**seriko state.rs 近接**——本 spec の動的 bind マップが seriko-loop の bind+random 発火ゲートの **read-only 読み口**になる契約＝推奨順序 mayuna→seriko-loop。fixture のまばたき bindgroup1400-1402 は `default,1` 無し＝**既定 OFF** ゆえ、本 spec の動的 bind 無しでは sakura まばたきが観測不能＝先行推奨の実質根拠）。
- **Supersedes（スコープ吸収）**: roadmap 増分の旧「⑤seriko のみ `mayuna-compose`（bind 状態の動的管理）」表記を**②④⑤垂直スライスへ拡張**（#2 仕分けで確定・roadmap line 151 の再スコープ注記が根拠）。

## Constraints

- Rust 2024・新規 crates.io 依存なし・tokio 不使用・**additive**（既存 variant/API のワイヤ形不変）。
- **決定論維持**: 時刻注入・実時間 sleep/`Instant` 不使用（[[deterministic-test-coverage-mandate]]）。名前解決・on/off 積算は純関数化し GPU 不要で全網羅（[[test-only-decision-branches-not-proven-wiring]]）。
- **面引数は不透明 String・id 解決は下流**（[[areka-surface-args-opaque-string-downstream-resolve]]＝parser/compile は名前を転写、bindgroup 解決は seriko）。
- **実機受け入れ**: 実 emo2・実 pasta.dll・実 DPI（≠96）で表情変化を人間サインオフ（[[areka-placement-real-ghost-first]]）。**起動は絶対パス必須**（相対だと helper が pasta.dll を LoadLibrary できず MOD_NOT_FOUND＝2026-07-13 運用注意）。
- 正典は ukadoc・emo2 は最小適合 fixture（[[ukadoc-mcp-preferred-source]]）。
