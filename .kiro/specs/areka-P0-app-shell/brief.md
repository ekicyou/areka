# Brief: areka-P0-app-shell

> **種別**: 本坑（main）。⓪ ghost 帰属・**アプリ組み上げ三段の第一段**（**app-shell（骨格）→ ghost-setup（エンジン結線）→ emo2-conformance-e2e（適合証明）**）。早期着手可・小粒。
> **調査日**: 2026-07-05（areka デモ実見＋ukadoc ベースウェアアプリ要素の体系調査）。

## Problem

エンジン群（shiori/parsers/kanade/sakura/seriko/emo）のユニットは揃いつつあるが、**「アプリとしての areka」＝ `crates/areka` バイナリの main.rs を所有する仕様が存在しない**。現 main.rs はモック UI（ハードコード窓・固定テキスト）が占有しており、①本番アプリの骨格（構成入力・初期化・終了）の置き場が無い ②モックデモはエンジン結線時に上書きで消える運命（動く資産の喪失）③emo-present と window-placement が同じ main.rs を取り合う構造的衝突、の三重苦になっている。

## Current State

- **main.rs（500行）＝混成**: モック UI（`create_shell_window`/`create_balloon_window`・初期位置 (400,200)・追従 (335,0) 決め打ち・DPI 処理ゼロ）＋ **本物の資産**（`shiori_host`/`shiori_session`/`reference_brain` モジュール＝completed shiori 系 specs の成果・e2e テスト群が依存）＋ `shiori_demo`（env-gate の実走デモ）。
- **example の前例あり**: `crates/areka/examples/clickthrough_two_rects.rs`（検証台の退避先パターン）。
- **アプリ組み上げの所有マップ（本 brief で確定）**: 骨格＝**本ユニット**／エンジン結線・lifecycle 統括＝`ghost-setup`／boot・close の**イベント発火順序（運行表）＝kanade**／適合証明＝`emo2-conformance-e2e`／位置・vanish count 永続化＝`position-persist`（M-life）。

## Desired Outcome

main.rs が**本番アプリの骨格**になり、モックデモは**別名の example として保全**され、`crates/areka` の取り合いが構造ごと解消される。

**✔ 観測（単一 pass/fail）**: (a) `cargo run --example mock-shell` が**従来のデモと同一挙動**（窓2枚・ドラッグ・ダブルクリック終了）で動く ＋ (b) 新 main.rs（骨格）が構成入力（ghost path）を受けて起動→初期化ログ→（エンジン未結線のまま）正常終了する ＋ (c) 既存 shiori e2e テスト群が green のまま。

## Approach

1. **デモ保全（別名 example 化）**: main.rs のモック UI 部分を `examples/mock-shell.rs` へ移設（`clickthrough_two_rects` 前例に倣う）。**挙動不変が受け入れ基準**。デモ資産（shell/base.png・座標定数・Typewriter 詩文）は example 側の私物として保持——**本番コードへの持ち込み禁止**（07-05 window-placement リジェクトの教訓）。
2. **骨格 main**: tracing Subscriber 初期化（logging.md 準拠・現 demo の RUST_LOG フォールバックパターン流用）・human_panic・`WinApp` 起動・**構成入力の解決**（ghost root path＝引数 or 既定 fixture パス／balloon root path＝同・ukadoc 上ハードコード/引数で正当〔下記〕）→ ghost-setup 未実装の間は「構成を解決してログに出し正常終了」まで。
3. **shiori 系モジュールの帰属整理**: `shiori_host`/`shiori_session`/`reference_brain`/`shiori_demo`＋テスト群は src 残留（本物の資産）。`shiori_demo` の呼び口は骨格 main に残す（env-gate 不変）。
4. **結線の口だけ**: ghost-setup が後で差し込む「エンジン起動→boot 指示→close 待ち」の呼び口（関数1個の空実装 or feature 分岐）を置く——**中身は作らない**（ghost-setup の領分）。

## ukadoc 必読（design 着手時に ukadoc MCP `get_doc`/`search_docs` で正典参照・2026-07-05 調査）

- **アプリ構成の正当性（C）**: ゴースト/バルーンの実行時選択は**仕様上の探索義務なし**——`install.txt` はインストール時 oneshot（起動時不使用・確定済み）・**最小ベースウェアは引数/固定規約で正当**。バルーンは ghost descript の `balloon` 宣言・`sakura.balloon.defaultsurface` を読むのが望ましいが必須でない（選択 UI は M2）。
- **boot/close の発火順序は kanade の領分（申し送り）**: `list_shiori_event` の **OnInitialize**（NOTIFY・起動時必須）→ **OnFirstBoot**（Reference0=vanish count）/ **OnGhostChanged**/**OnGhostCalled**/**OnVanished** の 204 フォールスルー → **OnBoot**（Reference0=shell 名・Ref6/7=crash 検出は任意）→ **basewareversion NOTIFY**（自己申告・必須）。close は **OnClose**（Reference0=理由）→ 応答スクリプト**再生完了待ち**（`\-` 終端）→ 終了・204 なら **OnCloseAll** フォールバック・タイムアウトは de-facto（spec 無指定＝design で決める）。**本ユニットはこれらを実装しない**——kanade brief 化時に転記すること（roadmap の kanade 行に注記済み）。
- **M2 送りの裏付け（E/F）**: SSTP ポート（9801）・FMO・DirectSSTP・Plugin/HEADLINE/SAORI ホスティング・ネットワーク更新（OnBasewareUpdating 系）は**全て任意**＝emo2 単体起動に不要。M1 骨格はこれらの口を持たない。
- **永続化（D）**: vanish count（OnFirstBoot 判定用）と窓位置が de-facto 必須の永続状態——**position-persist（M-life）の領分**（状態ファイル形式は spec 無指定＝baseware 自由）。M-boot の骨格は「毎回 OnBoot」で開始してよい。

## Scope

- **In**: モックデモの `examples/mock-shell.rs` 保全（挙動不変）／骨格 main（tracing・panic・WinApp・構成入力解決＝ghost/balloon path）／shiori 系モジュール帰属整理／ghost-setup 差し込み口（空）／Cargo.toml example 登録。
- **Out**: エンジンの起動・結線・lifecycle（**ghost-setup**）／boot・close イベント順序の実装（**kanade**）／窓・描画（emo チェーン／window-placement）／vanish count・窓位置の永続化（**position-persist**）／SSTP・FMO・Plugin・自動更新・ゴースト/バルーン選択 UI（**M2**）。

## Boundary Candidates

- デモ退避（example・純機械的）／骨格 main（初期化・構成・終了）／差し込み口（ghost-setup との接続点）の三片。

## Out of Boundary

- 本番の窓生成・配置（window-placement が骨格の上で行う）／surface 表示（emo-present が example ベースで観測）。

## Upstream / Downstream

- **Upstream**: なし（既存 demo と wintf 基盤のみ・**即着手可**）。
- **Downstream**: **ghost-setup**（骨格の差し込み口にエンジン結線を実装）／**emo-present**（保全された mock-shell example を観測土台の donor に・main.rs 不触）／**window-placement**（骨格上で窓機構を実装＝main.rs 衝突が構造ごと解消）／**emo2-conformance-e2e**（完成アプリで一周適合）。

## Existing Spec Touchpoints

- **Extends**: `completed/areka-mock-shell`（デモ実体の保全）・completed shiori 系（モジュール帰属の現状維持）。
- **Adjacent**: `areka-P0-emo-present`／`areka-P0-window-placement`（両 brief の `crates/areka` 衝突注記は**本ユニット完了で解消**——申し開き整合済み）／`wintf-ulw-removal`（`CompositionMode` collapse の追随対象に example も含まれる）。

## Constraints

- Rust 2024・tokio 禁止・新規依存なし。**デモ挙動不変**＋**shiori e2e green 維持**が受け入れ基準。
- 骨格は「本番ゴースト先行の原則」に服する——**骨格自身は窓を作らない**（座標・配置ロジックを持たない＝リジェクト教訓の再発防止）。
