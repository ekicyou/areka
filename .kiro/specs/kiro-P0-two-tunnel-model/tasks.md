# Implementation Plan

> プロセス支援仕様。成果物は steering 文書群＋`crates/pilot` 検疫所クレート（空 lib ＋ examples-only による命綱の構造的担保）。専用機械ゲートは契約外（design ディスカッションで defer）。
>
> **並列の地図**: Task 1（crates/pilot 境界）・Task 2（two-tunnel.md 境界）・Task 3（roadmap.md 境界）は相互に独立で並列可。Task 4 は Task 2 を参照するため後続。Task 5 は検証。

- [ ] 1. crates/pilot 検疫所クレートの新設（命綱の構造的担保）
  - 前提（全 1.x・5.1 共通）: worktree では submodule（`vendors/pasta`）が未populate のため、pilot の全 cargo 操作（`build`/`run`/`metadata`）の前段で `git submodule update --init --recursive` が必要（design §Architecture・要件 4 補足）。これは環境アクションでありコード成果物ではない。
- [x] 1.1 pilot クレート骨格（Cargo.toml・空 lib・クレート README）
  - `crates/pilot/Cargo.toml` を作成（`name="pilot"`, `publish=false`, version/edition/license 等は workspace 継承, `[dependencies]` は空）。`crates/shiori-abi/Cargo.toml` を構造範例とする。
  - `crates/pilot/src/lib.rs` を空 lib（`//! pilot quarantine crate` のドキュメントコメントのみ・公開 item なし）で作成する。これが命綱（公開 API を露出しない）の構造的担保の核心。
  - `crates/pilot/README.md` にクレートの役割（検疫所・空 lib ＝命綱の構造的担保）と `two-tunnel.md` への参照を記載。
  - 完了状態: submodule init 後に `cargo metadata` が `pilot` をワークスペースメンバー（`publish=false`）として解決し、`cargo build -p pilot` が成功する。`src/lib.rs` に公開 API が存在しない。
  - _Requirements: 2.1, 2.2, 2.3, 2.7, 4.1, 4.2, NFR-2, NFR-3_
  - _Boundary: crates/pilot_
- [x] 1.2 テンプレ example ＋ examples 配置規約
  - `crates/pilot/examples/_template/main.rs` を依存ゼロの最小コード（`println!` 程度）で作成。
  - `crates/pilot/examples/_template/README.md` を 3 幕（動機→概要→検証結果）の雛形で作成（対応本坑 spec 名指し欄・実行法 `cargo run -p pilot --example <spec>`・判定/学び/日付欄を含む）。
  - 1 仕様=1 フォルダ（`examples/<spec-name>/`・`main.rs` 必須）の配置規約を確立し crate README に明記（`two-tunnel.md` の README 規約と整合・並列時の merge 衝突ゼロ）。
  - 完了状態: submodule init 後 `cargo build -p pilot --example _template` がビルド成功し、`cargo run -p pilot --example _template` が template メッセージを出力する。
  - _Requirements: 2.4, 2.5, 2.6, 3.1, 3.2, 3.3, 3.4_
  - _Boundary: crates/pilot_

- [x] 2. (P) two-tunnel.md（二坑規律の詳細正本）を作成
  - `.kiro/steering/two-tunnel.md` を `inclusion: manual` で新規作成する（Task 1 のクレートとは別境界ゆえ並列可）。
  - 章立てに全規律を含める: ①概要＋可逆性最優先方針、②先進坑と本坑（定義・役割分担・何を掘るかの判断基準・直行許容）、③命綱と削除/隔離規律（葉ノード隔離・隔離保全許可・掘り直し禁止＝コピペ donor 禁止・品質基準・検疫所効果・空 lib＋examples-only の構造的担保・唯一の inbound 経路を変更レビューで捕捉する人手レビュー規律・機械チェックは defer で将来別途依頼可）、④ハードゲート（go 前提依存・BLOCKED・人間判断・記法 `_Depends(confirmed):`〈宿主は roadmap.md〉・直行許容）、⑤依存マップ重点検証の手動チェックリスト（被覆/孤児なし/DAG/各エッジ合否基準/不適合時 not-ready/適用タイミング: discovery・`/kiro-spec-batch`）、⑥先進坑の一次記録 README 3 幕規約（本坑 design は README 検証結果を参照し二重化しない・subagent が `.md` を書けない制約の代替手順）。
  - 完了状態: `two-tunnel.md` が `inclusion: manual` を持ち、上記 6 領域の全規律へ見出しから到達できる（要件 1.4 の「各規律へ到達できる参照」を内部見出しで担保）。
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 3.5, 3.6, 4.3, 4.4, 5.1, 5.2, 5.3, 5.4, 5.5, 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.2, 7.3, 7.4, 7.5, 7.6_
  - _Boundary: two-tunnel.md_

- [x] 3. (P) roadmap.md に go ゲート記法凡例を追記
  - `.kiro/steering/roadmap.md` に `_Depends(confirmed): <pilot-spec>` の凡例節を追記する（既存 `Dependencies:` 自由テキスト慣行の拡張・`spec.json` の `dependencies` 配列とは別レイヤで二重管理を回避）。
  - 凡例は self-contained（記法の正規の意味＝先進坑 go 必須の確定前提依存は `two-tunnel.md` ハードゲート節を相互参照）ゆえ Task 2 に実装依存せず並列可。記法の文字列・宿主は本仕様で確定。
  - 完了状態: `roadmap.md` に `_Depends(confirmed):` の意味（先進坑 go 必須の確定前提依存）と既存 `Dependencies:` との区別を示す凡例が存在する。
  - _Requirements: 6.4_
  - _Boundary: roadmap.md_

- [ ] 4. workflow 統合 ＋ 常駐ポインタ
- [ ] 4.1 workflow.md に二坑統合節を追記
  - `workflow.md` に二坑統合節を追記する: 先進坑フェーズと既存フロー（requirements→design→tasks→implementation→complete）の関係、go ハードゲートを本坑着手の前提条件として、依存マップ重点検証ルール、削除/隔離規律、先進坑の多重並列運用（既存 Agent/Workflow 機構を用い新規基盤を開発しない）。二坑詳細は `two-tunnel.md` へ委譲し要約＋参照に留める（常駐コスト抑制）。
  - 機械ゲート/DoD ゲート統合は追加しない。既存の「ブランチ＆マージ戦略」「実装完了時のアクション」「仕様フェーズフロー」は不変（追記のみ）。
  - 完了状態: `workflow.md` の diff が追記のみで既存節を改変せず、二坑統合節から `two-tunnel.md` への参照があり、機械ゲート/DoD 統合の記述を含まない。
  - _Requirements: 1.5, 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, NFR-1_
  - _Depends: 2_
  - _Boundary: workflow.md_
- [ ] 4.2 (P) focus.md に two-tunnel.md 参照を追記（常駐 lean ポインタ・任意最小）
  - `focus.md` の「参照先」節に `two-tunnel.md`（二坑規律正本・`inclusion: manual`）への参照を 1 行追記する。Task 4.1 とは別ファイル境界ゆえ並列可。
  - 完了状態: `focus.md` から `two-tunnel.md` への参照行が存在する。
  - _Requirements: 1.4, 1.5_
  - _Depends: 2_
  - _Boundary: focus.md_

- [ ] 5. 整合検証
- [ ] 5.1 (P) 構造的隔離・クレート構造の検証
  - submodule init 後、`cargo metadata` で `pilot`=ワークスペースメンバー/`publish=false`、`src/lib.rs` が空 lib（公開 API なし）、探索コードが `examples/` のみに存在、`cargo build --examples -p pilot`（`_template` 含む）通過、`cargo run -p pilot --example _template` 実行を確認する。
  - 「Cargo の `examples/` は他クレートから依存できず、空 lib は API 露出なし」ゆえ inbound edge が構造的に発生し得ないこと、および他クレートの `Cargo.toml` に `pilot` 依存が無いことを確認する（命綱の構造的担保の検証）。
  - 完了状態: 上記 cargo コマンドが全て成功し、`pilot` への inbound 依存が存在しないことを確認できる。
  - _Requirements: 2.1, 2.2, 2.6, 3.4, 4.1, 4.2, NFR-2, NFR-3_
  - _Depends: 1.1, 1.2_
  - _Boundary: crates/pilot_
- [ ] 5.2 (P) ドキュメント整合検証
  - 次を確認する: `two-tunnel.md` に全規律（命綱・ハードゲート・依存マップ・削除/隔離・README 規約・inbound 人手レビュー規律・defer 方針）が到達可能、常駐側（`workflow.md`/`focus.md`）から `two-tunnel.md` へ参照が辿れる、`workflow.md` は追記のみで既存規約（PR ベース・main 直 push 禁止・完了手順）が不変、`roadmap.md` に go 記法凡例が存在、`completed/kiro-P0-roadmap-management` が未改変、全成果物がテキストベースで Git 追跡可能。
  - `(P)` は Task 5.1（`crates/pilot` 境界）との並列を指す。自身の依存（Task 2/3/4 と同一ドキュメント群）とは並列しない。
  - 完了状態: 上記整合チェックが全てパスし、`workflow.md` の既存規約に対する変更が無い（追記のみ）ことを確認できる。
  - _Requirements: 1.4, 1.5, 4.3, 4.4, 5.1, 6.4, 7.1, 8.5, NFR-1, NFR-4, NFR-5_
  - _Depends: 2, 3, 4_
  - _Boundary: steering docs_
