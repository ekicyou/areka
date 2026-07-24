# Requirements Document

## Introduction

`cargo test -p wintf --test graphics` は、同一プロセス内で 2 個目の WUC（Windows.UI.Composition）グラフィックススタック（`GraphicsCore` ／ `WucGraphicsResource` の初期化一式）を生成するテストの実行中に、100% 決定論的に `STATUS_ACCESS_VIOLATION (0xc0000005)` でクラッシュする（flake ではない・特定テスト非依存）。この 1 点により `cargo test --workspace` が exit 非 0 となり、Kiro ワークフローの `kiro-complete` DoD Test Gate（workspace 緑判定）が全 spec で閉塞している。影響は wintf `graphics` バイナリに閉じず、areka bin テスト（`spine_e2e_kero_blink_one_cycle_golden`）にも同一法則で波及済みであることが実測されている（詳細な診断マトリクス・bisect 手順・タイムライン・仮説は `.kiro/specs/wintf-gpu-test-crash/brief.md` を正本とする）。

このクラッシュは単なるテスト基盤問題に留まらない可能性がある。「同一プロセスで WUC グラフィックススタックを 2 度生成すると死ぬ」という法則が本番コードにも当てはまるなら、将来のゴースト再ロードやシェル切替（プロセス内 GPU スタック再生成を伴う機能）が本番で同じクラッシュを踏む。したがって本 spec は、(1) テストスイートを決定論的に緑へ復旧させること、(2) 根本原因を特定・記録すること、(3) その原因が本番設計に持つ含意を明文化すること、(4) 同じ crash パターンが二度と静かに再発しないよう回帰の網を張ること、の 4 点を扱う。

## Boundary Context (Optional)
- **In scope**: wintf `graphics` テストスイート（91 テスト）における同一プロセス 2 個目 WUC スタック生成クラッシュの根本原因特定・修正、`cargo test --workspace` の exit 0 復旧確認（wintf・areka bin 双方の該当テストを含む）、根本原因および本番設計への含意の文書化、再発防止のための回帰テスト追加
- **Out of scope**: areka 側クレート（`areka-sylphya` / `areka-kanade` / `areka-ghost` / `areka-parsers` / `areka` bin 等）のソースコード変更、WUC 以外のレンダリング機能追加、graphics テストスイートの網羅範囲拡張（回帰テスト以外の新規テスト追加）、`areka-P0-sylphya` の完了処理（人間サインオフ待ちの別トラック）、emo2 実機系（`AREKA_EMO2_REAL_RUN`）検証、32bit SHIORI 系の変更
- **Adjacent expectations**: 本 spec の完了は、後続ウェーブ（position-persist／choice-interact／emo-dpi-scaling 等）が「素の `cargo test --workspace` 緑」を前提に DoD 判定できることを回復する前提条件である。将来のゴースト再ロード・シェル切替機能は、本 spec が明文化する「プロセス内 GPU スタック再生成の可否」という設計含意に従うことを期待される。areka bin テストの緑化はこの根本原因修正の副次効果として得られるものであり、areka クレート自体への変更を要求しない。

## Requirements

### Requirement 1: wintf graphics テストスイートの決定論的グリーン化
**Objective:** As a wintf メンテナ, I want `cargo test -p wintf --test graphics` を既定の並列実行設定で決定論的に全テスト成功させたい, so that テストスイートがクラッシュに阻まれず信頼できる検証手段であり続ける

#### Acceptance Criteria
1. When `cargo test -p wintf --test graphics` を既定（並列）設定で実行する, the wintf graphics テストバイナリ shall 91 テストすべてを `STATUS_ACCESS_VIOLATION` を発生させずに完了する
2. When `cargo test -p wintf --test graphics -- --test-threads=1` で逐次実行する, the wintf graphics テストバイナリ shall どの WUC スタック生成テストの組み合わせが連続しても `STATUS_ACCESS_VIOLATION` を発生させずに完了する
3. While 既知の最小再現ペア（`clip_sync_system_test::clip_sync_applies_all_clip_shape_variants` に続けて `clip_sync_system_test::clip_sync_clears_clip_when_clip_is_none` を同一プロセスで実行する状態にある, the wintf graphics テストバイナリ shall 両テストをクラッシュなく完了する
4. The wintf graphics テストスイート shall 少なくとも 5 回の連続フルスイート実行のすべてでクラッシュ・flake が 0 件である
5. The wintf graphics テストスイート shall 修正後も外部 CI インフラを介さずローカル開発機上の `cargo test` で実行可能なままである（実 GPU/WUC を要する既存の検証様式を維持する）

### Requirement 2: ワークスペース全体の Test Gate 復旧
**Objective:** As spec を完了させる開発者, I want `cargo test --workspace` が exit 0 で終了することを確認したい, so that `kiro-complete` の DoD Test Gate が条件付き判定（一部クレートのみ緑）ではなく素の workspace 一括緑で機能する

#### Acceptance Criteria
1. When 修正適用後に `cargo test --workspace` を実行する, the ワークスペーステストコマンド shall exit code 0 で終了する
2. When areka bin テストスイートを実行する, `spine_e2e_kero_blink_one_cycle_golden` shall areka クレートのソースコードを変更することなく `STATUS_ACCESS_VIOLATION` なく成功する
3. If wintf `graphics` および areka bin 以外のテストバイナリが同一プロセス内で 2 個以上の WUC/GPU スタックを生成する構造を持つことが判明した場合, then そのテストバイナリ shall 同種の `STATUS_ACCESS_VIOLATION` が発生しないことを検証される

### Requirement 3: 根本原因の特定と記録
**Objective:** As wintf メンテナ, I want 同一プロセス 2 個目 WUC スタック生成クラッシュの根本原因が特定・記録されることを求める, so that 将来の WUC ライフサイクル関連作業が憶測ではなく事実に基づいて行われる

#### Acceptance Criteria
1. When 根本原因調査が完了する, the 根本原因記録 shall bisect 手順（`68bd2e3e~1` での再現有無）に基づき、環境要因（H-env）とコード要因（H-code・`68bd2e3e` の MTA 常駐導入）のどちらが確定したかを明示する
2. The 根本原因記録 shall クラッシュ発生箇所（WUC スタック生成時・前 world の teardown 由来・schedule 実行中のいずれか）を、デバッガのスタックトレースまたは切り分け実験結果などの裏付け証拠とともに特定する
3. Where 根本原因が本番の WUC リソースライフサイクルに波及する場合（実際のゴースト再ロード・シェル切替シナリオが同種クラッシュを踏み得ることを意味する場合）, the 根本原因記録 shall その含意を明示的に記載する

### Requirement 4: 本番ライフサイクルへの含意の明文化
**Objective:** As 将来のゴースト再ロード／シェル切替機能の実装者, I want 同一プロセス内での WUC グラフィックススタック再生成が本番で安全かどうかが明文化されることを求める, so that その制約を知った上で機能を設計できる

#### Acceptance Criteria
1. When 根本原因が確定する, the spec の記録 shall 同一プロセス内 WUC スタック再生成のハザードが (a) 本番の WUC リソースライフサイクル是正を要する実在リスクか、(b) テストハーネス固有でありプロダクションへの影響がない事象か、のいずれであるかを明文で宣言する
2. If ハザードが本番の実在リスクと宣言された場合, then 修正 shall wintf の WUC リソースライフサイクル（生成・破棄順序等）そのものを是正する
3. If ハザードがテストハーネス固有と宣言された場合, then 修正 shall Requirement 1 および Requirement 2 の受入基準を満たす限りにおいて、本番コードではなくテストハーネス構造側の是正を選択してよい

### Requirement 5: 再発防止の回帰テスト
**Objective:** As wintf メンテナ, I want 今回の最小再現パターンを恒久的に監視する回帰テストを持ちたい, so that 将来の変更が同じクラッシュを静かに再発させない

#### Acceptance Criteria
1. The wintf graphics テストスイート shall 単一テストプロセス内で WUC グラフィックススタックを 2 回以上連続生成する回帰テストを含む
2. When この回帰テストを `cargo test -p wintf --test graphics` の一部として実行する, the 回帰テスト shall Requirement 1 が定める決定論的グリーン化の一部として、他のテストと並んで安定して成功する
3. If 将来の変更が同一プロセス 2 個目 WUC スタック生成時の `STATUS_ACCESS_VIOLATION` を再発させた場合, then この回帰テスト shall 失敗し回帰を検出する（サイレントに成功しない）
