# Requirements Document

## Project Description (Input)
`\q` 選択肢の**対話面**を実装する。上流 `areka-P0-choice-render`（W3・2026-07-24 完了）が供給する選択肢**行ヒットジオメトリ**と **hover 状態 API**（emo-text `TextLayerRuntime`：`ChoiceHitRow`・`inject_choice_hover`・`choice_hit_rows`・`choice_active`）を消費し、バルーン窓の実ポインタ移動を選択肢行の hit 判定へ写像して hover を追従させ、クリック確定時に **`ChoiceSelection`（本 spec が契約正本）** を一度だけ発行する。talk 切替・choice 消滅後の stale クリックは原子性ガードで棄却する。決定論檻は注入ポインタ列（実窓不要・sleep 不使用）で全網羅し、実機は実 emo2・実 DPI・絶対パス起動でポインタ→ハイライト→クリック到達を目視サインオフする。描画・レイアウト・ハイライト描画（choice-render）、SHIORI カスケード発火・`Status: choosing`・timeout（choice-select-events W6）、`resolve_choice` の直接呼出（配送は kanade 経由の正規経路のみ）、キャラ窓側ポインタ配線の変更（input-events W2 の成果は消費のみ）はスコープ外。

## Introduction

emo2 のメニュー選択肢は上流 `areka-P0-choice-render`（W3・2026-07-24 完了）によって**見え**、注入 hover で**光る**が、実ポインタがバルーン窓からどの選択肢行へ届き、どの行が hover 中で、クリックがいつ「選択確定」になるかの**対話面**が無い。この配線が無いと M-dialogue の「ダブルクリック→メニュー→選択→遷移」の一周は完走しない。

本仕様は M-dialogue の**対話半分**である。上流 `areka-emo-text` の `TextLayerRuntime` が契約正本として供給する**選択肢行ヒットジオメトリ**（`choice_hit_rows` が返す `ChoiceHitRow`＝`ordinal`／`id`／`label`／`references`／バルーン窓物理 px の `rect`）と、**hover 状態注入 API**（`inject_choice_hover(actor, Option<usize>)`）・**選択肢表示中照会**（`choice_active(actor)`）を消費し、バルーン窓の実ポインタ移動を選択肢行の hit 判定へ写像して hover を追従駆動する。そして確定クリック時に **`ChoiceSelection`（本仕様が契約正本・下流 `areka-P0-choice-select-events` が消費）** を一度だけ発行する。talk 切替・choice 消滅後の stale クリックは原子性ガードで棄却する。

本仕様は additive 増分に徹し、上流の描画・行ヒットジオメトリ・hover API・cue 配送・emo-text 状態機械・キャラ窓ポインタ配線（`areka-P0-input-events` の成果）を改変しない。決定論檻は注入ポインタ列（実窓不要・sleep 不使用）による純粋 hit 判定＋配線の存在チェックで全網羅し、実機は実 emo2・実 pasta.dll・実 DPI（≠96）・絶対パス起動でポインタ→ハイライト追従→クリック到達を目視サインオフする。完了時、実 emo2 でメニュー選択肢行が実ポインタで**追従ハイライト**され、行クリックで選択が**一度だけ確定**する（確定後のカスケード発火・`Status: choosing`・timeout・遷移は下流の領分ゆえ本仕様では判定しない）。

## Boundary Context

- **In scope**:
  - バルーン窓ポインタ移動 → 選択肢行 hit 判定（上流 `choice_hit_rows` の行矩形を消費）。
  - hover 状態の更新駆動（上流 `inject_choice_hover` を hit 行の `ordinal`／None で駆動・自前描画はしない）。
  - 確定クリック → `ChoiceSelection` の一度きり発行（**契約正本**＝選択 id・表示ラベル・発生元 scope・`references` を保持するワイヤ形を本仕様が所有）。
  - stale クリック棄却（talk 切替・`Clear`／`ClearAll`・choice 消滅後の原子性ガード）。
  - 決定論檻（注入ポインタ列・sleep 不使用の純粋 hit 判定＋配線存在チェック）＋実機サインオフ（有界 auto-exit ＋ログ grep）。
- **Out of scope**:
  - 選択肢の描画・レイアウト・ハイライトの**描画**（`areka-P0-choice-render` W3・完了）。
  - SHIORI カスケード（`OnChoiceSelectEx`→`OnChoiceSelect`→任意名直接発火）・`Status: choosing`・timeout（`areka-P0-choice-select-events` W6）。
  - `ChoiceSelection` の**受信・配送処理**および `CuePlayer::resolve_choice` の直接呼出（配送は正規の下流経路のみ）。
  - キャラ窓側ポインタ配線の変更（`areka-P0-input-events` W2 の成果は消費のみ）。
  - ホイール・キーボードによる選択肢操作（M2）／選択肢以外のバルーン内リンク（`\_a` アンカー等・emo2 未使用）。
- **Adjacent expectations**:
  - 契約 API は上流 `areka-emo-text` の `TextLayerRuntime`（`choice_hit_rows`／`inject_choice_hover`／`choice_active`）を**消費のみ**とし、行ヒットジオメトリ・hover API・ハイライト描画は本仕様が所有しない（`areka-P0-choice-render` が正本）。hover 対象は emo-text 側の `ordinal` ベースで、本仕様は hit した行の `ordinal` を注入する。
  - `ChoiceHitRow.rect` はバルーン窓の**物理 px**、ポインタの client 座標も物理 px であり、hit 判定は既存ポインタ配線の**物理 px 素通し（DPI 変換を挟まない・k=1.0）規約**に整合させる（`areka-P0-input-events` の素通し規約を破らない）。
  - バルーン窓は既に窓マーカーとドラッグ設定を備える（窓生成側で付与済み）。本仕様はこれらを**消費**し、新たな窓ライフサイクルやドラッグ挙動は新設・改変しない。ただし選択肢表示中にバルーン窓ポインタを選択肢行へ**到達**させるために窓生成側の最小限の到達設定（`HitTest` 等）の改変が設計上必要と判明した場合、本仕様がその最小改変を負う（実機到達サインオフ〔R7〕を無条件 DoD として維持するため）。窓生成側を扱う `areka-P0-position-persist` が**同時進行中で停止できない**ため、当該改変が position-persist と衝突する場合は、委譲・先送りではなく position-persist へ **rebase/merge して統合**する（合流機構の選択＝窓 `HitTest` トグル／content `alpha_mask` 等は設計 R-1 に残す）。
  - `ChoiceSelection` の**ワイヤ形は本仕様が正本**だが、その最終配送先（受信アクター／inbox 型）と受信処理・カスケード発火は下流 `areka-P0-choice-select-events` の契約辺であり、本仕様は**発行まで**を担い `resolve_choice` を直接呼ばない。
  - 選択肢の消滅（表示・hit の原子的無効化）は `areka-P0-choice-render` が保証する契約であり、本仕様はクリック確定時に上流の**現行**行ヒットジオメトリを参照して stale 状態を作らないことで協調する。

## Requirements

### Requirement 1: バルーン窓ポインタの選択肢行 hit 判定と hover 追従駆動

**Objective:** ゴースト作者として、バルーン上でカーソルを動かすと、いま選ぼうとしている選択肢行のハイライトが実ポインタに追従することを求める。これにより、対話面がポインタと表示をつなぐ。

#### Acceptance Criteria

1. When バルーン窓上でポインタ移動イベントを受け取り、かつ選択肢が表示中（上流 `choice_active` が真）であるとき、the 対話層 shall ポインタの client 座標（物理 px）を上流の行ヒットジオメトリ（`choice_hit_rows` の各 `ChoiceHitRow.rect`）と突き合わせ、当該座標を包含する選択肢行を高々 1 つ判定する。
2. When ポインタがいずれかの選択肢行を包含判定するとき、the 対話層 shall 当該行の `ordinal` を hover 対象として上流 hover 状態注入 API（`inject_choice_hover`）へ設定する。
3. When ポインタがどの選択肢行も包含しない位置へ移動するとき、the 対話層 shall hover 対象を「ハイライト無し」（`None`）として上流 hover 状態注入 API へ設定する。
4. While 選択肢が表示中でない（上流 `choice_active` が偽）とき、the 対話層 shall バルーン窓ポインタ移動を選択肢 hit 判定・hover 注入に用いず、hover 追従を発生させない。
5. The 対話層 shall 選択肢行の hit 判定を各行矩形への点包含判定として構成し、行矩形が病的に重なる入力に対しても決定的な選択規則で高々 1 行のみを hover 対象に選ぶ。
6. The 対話層 shall hover 追従を自前描画なしで実現し、ハイライトの実描画は上流（`areka-P0-choice-render`）へ委ねる（本仕様は hover 状態の駆動に留める）。

### Requirement 2: 確定クリックによる `ChoiceSelection` の一度きり発行と契約ワイヤ形

**Objective:** ゴースト作者として、選択肢行をクリックするとその選択が一度だけ確定し、下流がカスケードを組み立てられる形で通知されることを求める。これにより、メニューから選択が**確定**する。

#### Acceptance Criteria

1. When 選択肢が表示中で、ある選択肢行を包含する位置でバルーン窓の確定クリック（左シングルクリック）を受け取るとき、the 対話層 shall 当該行に対応する `ChoiceSelection` を 1 回だけ発行する。
2. The 対話層 shall `ChoiceSelection` に少なくとも選択された選択肢の id（`\q` ID）・表示ラベル・発生元 scope・`references`（`\q` の付随引数）を保持させ、下流が表示層へ再照会せずに選択解決とカスケード発火を組み立てられる契約とする（当該ワイヤ形の正本を本仕様が所有する）。
3. When 確定クリックがどの選択肢行も包含しない位置で発生するとき、the 対話層 shall `ChoiceSelection` を発行しない。
4. When 単一の確定クリックを処理するとき、the 対話層 shall 高々 1 つの `ChoiceSelection` のみを発行し、同一クリックからの二重発行を行わない。
5. The 対話層 shall クリック時点で上流の**現行**行ヒットジオメトリを参照して選択行を確定し、過去にキャッシュした行情報のみからは発行しない（描画と hit の現行整合＝Requirement 3 の原子性に従う）。
6. The 対話層 shall `ChoiceSelection` の発行までに留め、`CuePlayer::resolve_choice` を直接呼び出さず、SHIORI カスケード・`Status: choosing`・timeout を行わない（それらは下流の領分）。

### Requirement 3: stale クリックの棄却と原子性ガード

**Objective:** ゴースト作者として、talk が切り替わったり選択肢が消えた後にクリックが届いても、古い/存在しない選択が誤って確定しないことを求める。これにより、消滅済み選択肢を誤選択する状態が生じない。

#### Acceptance Criteria

1. When talk 切替・`Clear`／`ClearAll`・新 talk 開始により選択肢が消滅（上流 `choice_active` が偽へ遷移）した後にバルーン窓クリックを受け取るとき、the 対話層 shall `ChoiceSelection` を発行しない。
2. If hover 対象として保持していた選択肢行が、クリック確定時点で上流の現行行ヒットジオメトリに存在しない（消滅・置換済み）とき、then the 対話層 shall 当該クリックを stale として棄却し `ChoiceSelection` を発行しない。
3. When 選択肢集合が新しい talk の選択肢へ置き換わるとき、the 対話層 shall 直前の hover/hit 状態を持ち越さず、新しい行ヒットジオメトリに対してのみ hit 判定・hover・確定を行う。
4. When 選択肢が消滅するとき、the 対話層 shall 自身の hover 対象を「ハイライト無し」へ整合させ、消滅済み選択肢へ hover 注入が残らないようにする（上流の表示・hit の原子的無効化と協調する）。

### Requirement 4: 上流契約 API の消費境界と DPI 素通し規約の遵守

**Objective:** 保守者として、対話面が描画・幾何・hover API を再実装せず上流契約の消費に徹し、キャラ窓配線と物理 px 素通し規約を壊さないことを求める。これにより、責務境界と座標契約が保たれる。

#### Acceptance Criteria

1. The 対話層 shall 行ヒットジオメトリ・hover 状態注入 API・選択肢表示中照会を上流 `areka-emo-text`（`TextLayerRuntime` の `choice_hit_rows`／`inject_choice_hover`／`choice_active`）から消費し、これら契約の再定義・自前保持・自前描画を行わない。
2. The 対話層 shall バルーン窓ポインタの client 座標を物理 px のまま行矩形（同じくバルーン窓物理 px）と突き合わせ、DPI 変換を挟まない素通し（k=1.0）規約に整合させる。
3. The 対話層 shall キャラ窓側のポインタ配線・hit 判定・既存メッセージ配送を変更せず、キャラ窓由来の既存挙動（ダブルクリック→メニュー等）を退行させない。
4. The 対話層 shall バルーン窓に既存で付与されている窓マーカー・ドラッグ設定を消費し、新たな窓ライフサイクルやドラッグ挙動を導入しない（選択肢行へのポインタ到達に必要な最小限の `HitTest`／到達設定の改変はこの限りでなく、同時進行の `areka-P0-position-persist` と窓生成側で衝突する場合は rebase/merge で統合する）。

### Requirement 5: M1 取り扱い範囲と対話境界

**Objective:** 保守者として、対話面の M1 実装範囲と、下流／M2 へ委ねる境界を明確化することを求める。これにより、対話半分のスコープが誤読されない。

#### Acceptance Criteria

1. The 対話層 shall 選択肢の確定操作をポインタ（左シングルクリック）に限り、ホイール・キーボードによる選択肢操作を M1 では行わない（M2）。
2. The 対話層 shall 選択肢以外のバルーン内リンク（`\_a` アンカー等・emo2 未使用）の対話を行わない。
3. The 対話層 shall 選択確定後の SHIORI カスケード（`OnChoiceSelectEx`→`OnChoiceSelect`→任意名直接発火）・`Status: choosing`・timeout を行わず、`areka-P0-choice-select-events` の領分として明示的に除外する。
4. The 対話層 shall `CuePlayer::resolve_choice` を直接呼び出さず、選択の解決配送を正規の下流経路へ委ねる（本仕様は `ChoiceSelection` 発行まで）。
5. The 対話層 shall キャラ窓側ポインタ配線の変更を行わず、`areka-P0-input-events` の成果を消費のみとする。

### Requirement 6: 決定論的なポインタ対話の観測とテスト網羅

**Objective:** 開発者として、注入ポインタ列から hit 判定・hover 追従・確定発行・stale 棄却までを実窓・sleep 無しで決定論的に観測できることを求める。これにより、対話面が回帰檻で保護される。

#### Acceptance Criteria

1. When 選択肢行を包含する／外れる注入ポインタ座標列を与えるとき、the 対話層 shall 各座標に対する hover 対象（`ordinal` または `None`）の決定を実窓不要で観測可能にする。
2. When 選択肢行上の注入クリックを与えるとき、the 対話層 shall 対応する `ChoiceSelection`（id／label／scope／references）の一度きり発行を観測可能にする。
3. When 選択肢消滅後の注入クリック、または行を外れた位置の注入クリックを与えるとき、the 対話層 shall `ChoiceSelection` が発行されない（stale 棄却・非 hit 棄却）ことを観測可能にする。
4. The 対話層 shall 上記観測を実ポインタ窓・sleep を用いず、注入したポインタ座標・クリック・選択肢状態のみで決定論的に成立させる。
5. The 対話層 shall 点包含 hit 判定・hover 遷移・確定発行・原子性/stale 棄却の入力依存の判断分岐を実行テストで網羅し、hit 判定（点 × 行矩形）を純関数として GPU 不要で全網羅する。
6. The 対話層 shall バルーン窓ポインタハンドラの配線の存在（増設された経路が結線されていること）を検証可能にし、純関数判定と配線存在チェックの二本立てで対話面を檻に入れる。

### Requirement 7: 実機による対話到達の人間サインオフ

**Objective:** 開発者として、実ゴースト・実 DPI 環境でポインタ→ハイライト追従→クリック確定まで実際に到達することを人間の目視で確認できることを求める。これにより、対話面が繋がっていない欠陥が解消されたことを保証する。

#### Acceptance Criteria

1. When 実 emo2・実 pasta.dll・実 DPI（≠96）でメニューを表示し、バルーン上でポインタを動かすとき、the 対話層 shall 選択肢行のハイライトが実ポインタに追従することを目視可能に提示する。
2. When 実機で選択肢行を実ポインタでクリックするとき、the 対話層 shall 当該選択の確定（`ChoiceSelection` 発行）に到達することを観測可能（ログ等）に提示する。
3. The 対話層 shall 実機サインオフを本番ゴースト表示を先行させたうえで行い、単発デモへの合わせ込みを判定根拠にしない。
4. The 対話層 shall 選択確定後のカスケード発火・遷移（`areka-P0-choice-select-events` の領分）を本仕様の実機判定に混ぜない（判定は発行到達まで）。
5. Where 実機起動を行うとき、the 対話層 shall pasta.dll を `LoadLibrary` 可能にするため絶対パスで起動する。
6. The 対話層 shall 実機サインオフを有界な auto-exit ＋ログ grep で決定論的に判定可能な形で構成する。

### Requirement 8: 既存資産の非退行（additive 制約）

**Objective:** 保守者として、完成済み上流エンジンおよびキャラ窓配線への本増分が、既存の全テスト緑・依存方針・ビルド制約・スレッド親和を崩さないことを求める。これにより、additive 原則が担保される。

#### Acceptance Criteria

1. The 対話層 shall 本増分の適用後も `cargo test --workspace` を exit 0 で成功させ、既存テストをすべて緑に保つ。
2. The 対話層 shall 新規の外部（crates.io）依存を追加しない。
3. The 対話層 shall Rust 2024 で構築し、tokio を導入しない。
4. The 対話層 shall ポインタ処理・上流 API 呼出を既存のスレッド親和（WUC/D2D を触る経路は UI スレッド固定）を守って行い、既存のスレッド制約を破らない。
5. The 対話層 shall 上流 `areka-emo-text` の描画経路・`areka-P0-choice-render` の契約（行ヒットジオメトリ／hover API）・既存 cue ワイヤ形・既存住人種を変更せず、新しい cue variant を新設しない（消費に留める）。
6. The 対話層 shall キャラ窓側ポインタ配線・`areka-P0-input-events` の DPI 素通し規約を退行させない。
