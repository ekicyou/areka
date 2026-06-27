# Implementation Plan

> 本仕様の成果物は単一 TOML 正本 `doc/shiori/shiori_protocol.toml`（＋ 正本宣言 README・生成 doc/Web アプローチ・ピン留め ukadoc スナップショット）。
> 契約データの全タスクは**単一正本ファイルを書き換える**ため、共有ファイル競合により直列実行する（`(P)` は read-only 検証のみ）。
> Rust 型・codegen・パーサ・doc/Web 生成器の実装は下流（スコープ外）。

- [ ] 1. Foundation: 資産ルートと典拠ベースライン
- [x] 1.1 doc/shiori 資産ルートの確立（骨格 TOML ＋ 正本宣言 README）
  - `doc/shiori/shiori_protocol.toml` を新設し、`[meta]`/`[envelope]`/`[reserved_headers]`/`[[entry]]`（＋`[[entry.field]]`）/`[[silence_ruling]]` の table 階層と各 table の必須キーを骨格として確立する
  - 型語彙を小文字 Rust 準拠（`i32`/`u32`/`i64`/`bool`/`str`、大文字混在禁止）に固定し、各 table・entry・field に `description` をデータフィールドとして必須化する
  - `doc/shiori/README.md` に「TOML=単一正本(SSOT)・doc/Web=TOML からの派生・Rust 型/codegen は下流生成・契約を本ファイル以外へ分散しない」を明記する
  - Observable: 骨格 TOML が valid な TOML としてパースでき design.md の table 階層・必須キーを満たし、README が正本/派生関係を宣言している
  - _Requirements: 3.1, 11.1, 11.2, 11.3, 11.4, 11.5_
- [x] 1.2 ukadoc 典拠スナップショットのベースライン確定
  - ピン留め3ページ（`list_shiori_event.html`/`list_shiori_resource.html`/`spec_shiori3.html`）の on-disk sha256 が `ukadoc/SOURCES.md` の記録と一致し、各ページの出典 URL・取得日・sha256 が SOURCES.md に網羅されていることを典拠ベースラインとして確立する
  - Observable: SOURCES.md が3ページの URL/取得日/sha256 を網羅し、ディスク上の sha256 と完全一致する典拠ベースラインが確立される
  - _Requirements: 7.3, 11.6_

- [ ] 2. Core: 契約固定部の符号化（meta / envelope / 予約ヘッダ）
- [x] 2.1 契約メタ・バージョニング・不変条件の符号化
  - `[meta]` に `contract_version`/`prerelease`/`internal_encoding="utf-16"`/`content_opaque=true`/`legacy_coemit_default=true`/`reference_immutable=true`/`high_rate_safe` を符号化する
  - 正準 content は charset を持たず内部=UTF-16 とし、レガシー wire の charset 符号化は host-32 委譲である旨を `description` に明記する
  - Observable: `[meta]` が全必須キーを持ち、バージョニング（D7 流動・lockstep）・content 不透明性・レガシー併載既定・Reference 不変・高レート方針を表現する
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 8.1, 8.2, 8.3, 9.1, 9.2, 9.3, 10.1_
  - _Depends: 1.1_
  - _Boundary: shiori_protocol.toml_
- [x] 2.2 json-rpc 封筒マッピングの符号化
  - `[envelope]` に method=event_id・params=named・correlation=token_eq_id・immediate=result・failure=error・deferred=id_then_result・raise=notification_no_id・batch を符号化する
  - 各写像が `areka-P0-shiori-com` の ABI 意味論（`Request`/`Complete`/`Raise`・`CorrelationToken`・`SHIORI_S_PENDING`）へ一意対応することを `description` に明記する
  - Observable: `[envelope]` が即時/失敗/遅延/Raise/相関の5写像を全て規定し、ABI 意味論へ一意対応する
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 9.3_
  - _Depends: 1.1_
  - _Boundary: shiori_protocol.toml_
- [x] 2.3 予約 SHIORI ヘッダ集合の確定
  - ピン留め `ukadoc/spec_shiori3.html` を解析し、ukadoc で使われる範疇の request/response 予約ヘッダ集合を `[reserved_headers]` へ確定（seed でなく確定集合）し、`collision_policy` を符号化する
  - Observable: `[reserved_headers].request/response` が spec_shiori3 由来の確定集合を保持し、典拠が `description` に記録される
  - _Requirements: 6.1, 6.2_
  - _Depends: 1.1, 1.2_
  - _Boundary: shiori_protocol.toml_

- [ ] 3. Core: ukadoc カタログ・フィールドスキーマ抽出
  - 本グループの全タスクは単一正本 `shiori_protocol.toml` を書き換えるため、共有ファイル競合により直列実行する（`(P)` 不可）
- [x] 3.1 イベント抽出（ライフサイクル/時刻/通信系）
  - `ukadoc/list_shiori_event.html` から起動・終了・時刻・通信系イベントを `[[entry]]`（kind=event）へ抽出し、各 entry に id/category/dispatch/response/provenance、各 field に name(意味名)/reference/type/required/response_meaning/provenance/description を符号化する
  - GET/NOTIFY を `[NOTIFY]` マーカーで分類し、ukadoc が応答期待や意味を沈黙する箇所は `silence_ref` で裁定へ紐付ける
  - Observable: 当該カテゴリ群のイベントが意味名フィールド＋`ReferenceN`対応＋GET/NOTIFY分類付きで TOML に存在する
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 3.2, 3.3_
  - _Depends: 1.1_
  - _Boundary: shiori_protocol.toml_
- [x] 3.2 イベント抽出（入力/マウス/サーフェス/キー系）
  - 同形式で入力・マウス・サーフェス・キー系イベントを抽出する
  - Observable: 当該カテゴリ群のイベントが必須キー充足で TOML に網羅される
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3_
  - _Depends: 3.1_
  - _Boundary: shiori_protocol.toml_
- [x] 3.3 イベント抽出（インストール/更新/システム/残カテゴリ網羅）
  - 残る全 ukadoc イベントカテゴリを抽出し、Event ページの網羅（約 261 件）を完成する
  - Observable: ukadoc Event ページの全カテゴリが TOML に網羅され、欠落カテゴリがゼロ
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3_
  - _Depends: 3.2_
  - _Boundary: shiori_protocol.toml_
- [x] 3.4 リソースカタログ抽出
  - `ukadoc/list_shiori_resource.html` から全リソース（約 158 件）を `[[entry]]`（kind=resource）へ抽出し、response 意味・field schema・provenance を符号化する
  - Observable: ukadoc Resource ページの全リソースが TOML に網羅される
  - _Requirements: 1.1, 1.4, 2.1, 2.2, 2.3_
  - _Depends: 3.3_
  - _Boundary: shiori_protocol.toml_

- [ ] 4. Integration: 2投影整合・沈黙裁定の確立
- [x] 4.1 意味名⇔ReferenceN 2投影と予約非衝突の確立
  - 全 field 行で canonical(`name`)/alias(`reference`) が単一行由来で同一値を指すことを保証し、canonical 優先規則を明記、別の対応表テーブルを作らない（R3 構造担保）
  - 全意味名が `[reserved_headers]` 確定集合と非衝突であることを保証し、衝突を検出した場合は `collision_policy` に従い是正する（是正の所有はこのタスク）
  - Observable: 全 field が予約非衝突かつ2投影が単一 field 由来で、対応表が field 行以外に存在しない
  - _Requirements: 3.1, 3.2, 3.4, 6.1, 6.2, 10.2, 10.3, 10.4_
  - _Depends: 2.3, 3.4_
  - _Boundary: shiori_protocol.toml_
- [x] 4.2 沈黙裁定ログの記録
  - GET/NOTIFY 沈黙・意味割り当て・追加ヘッダ可否等の areka 裁量裁定を `[[silence_ruling]]` へ記録し、各裁定に `basis`(典拠区分)/`ruling`/`ukadoc_anchor` を付与、entry/field の `silence_ref`（配列）から参照する
  - Observable: 全沈黙箇所が `[[silence_ruling]]` に記録され、`basis` 付きで entry/field から参照される
  - _Requirements: 1.3, 7.1, 7.2_
  - _Depends: 3.4_
  - _Boundary: shiori_protocol.toml_

- [ ] 5. Validation: 構造・整合・生成アプローチ検証
- [x] 5.1 (P) 構造検証
  - 必須キー存在・型語彙（小文字 Rust 準拠・大文字混在なし）・全 table/entry/field の `description` 非空・意味名の予約非衝突（ゼロ違反のアサート）を検証する
  - Observable: 構造検証が全 entry/field で pass し、型語彙違反・大文字混在・description 欠落・予約衝突がゼロ
  - _Requirements: 6.1, 11.3, 11.4_
  - _Depends: 4.1_
  - _Boundary: 構造検証（read-only）_
- [ ] 5.2 (P) 契約整合検証
  - 2投影が単一 field 由来であること、`[envelope]` の5写像が ABI 意味論へ一意対応すること、`silence_ref` 参照先が存在し `basis` を持つこと、ukadoc スナップショット sha256 が `SOURCES.md` と一致し provenance が ukadoc 記述有無と整合することを検証する
  - Observable: 整合検証が pass し、封筒被覆・2投影・沈黙参照・典拠同値が確認される
  - _Requirements: 3.2, 4.1, 4.2, 4.3, 4.4, 4.5, 7.1, 7.2, 7.3, 10.2, 11.6_
  - _Depends: 4.1, 4.2_
  - _Boundary: 整合検証（read-only）_
- [ ] 5.3 doc/Web 生成アプローチの受け入れ基準化
  - 正本から派生 doc/Web を生成する射（各 `description` をデータから本文へ展開・正本に無い記述を含まない・生成毎に正本と同値）を受け入れ基準として確定する（生成器実装は下流・本タスクは基準の確定のみ）
  - Observable: 生成アプローチが「入力=正本/出力=派生/同値保持」で文書化され受け入れ基準が定義される
  - _Requirements: 11.2, 11.5_
  - _Depends: 4.1_

## Implementation Notes
- 3.1: 意味名はスネークケース英語で ukadoc の Reference 説明から導出。カテゴリ slug は lowercase 英語固定（lifecycle/time/clock/network_update/mailcheck/rss/calendar/sstp/comm_other/send_failure/network_state）。Reference* 可変長は `reference_variadic=true`。予約ヘッダは PascalCase なので snake_case 意味名と構造的に非衝突。
- 3.1→4.2 引き継ぎ: dispatch 文脈依存/沈黙ケースは 4.2 で `[[silence_ruling]]`(topic=dispatch_class) を付与すべし — (a) OnDressupChanged は `[GET]/[NOTIFY]` 文脈依存（現状 get 採用・裁定未付与）、(b) OnCacheSuspend/OnCacheRestore は ukadoc 無印だが notify 採用（既定 get からの逸脱）、(c) 骨格由来の `sr_dispatch_onboot` は 3.1 で OnBoot から参照解除され孤立 — 4.2 で適切に再割当 or 整理する。
- 4.1: 802 field を機械検証（342 distinct 意味名 vs 18 予約ヘッダ＝衝突ゼロ・大小区別あり/なし両方）。canonical 優先規則は新設 top-level `[mapping]` policy table（`single_source="entry.field"`/`canonical_key="name"`/`alias_key="reference"`/`canonical_priority=true`/`separate_mapping_table=false`/`reference_backed_by`＋description）に符号化。**design は5テーブル想定だが policy-only の6番目を追加**（reviewer 許容判定・データ分散せず R3 補強）→ 5.1 構造検証の top-level keys 期待集合は **{meta, mapping, envelope, reserved_headers, entry, silence_ruling}** の6。ReferenceN 無し6 field は `reference_variadic=true`（pure `Reference*` 可変長尾）の正規例外。
- 3.4: Resource ページ 159件 全 kind=resource 抽出（event 287 と合わせ entry=446）。1:1 アサーション（159 `<dl>` ⇔ 159 resource id, MISSING/SURPLUS=0）。category slug=shiori_info(5)/ghost_info(40)/update_info(1)/ownerdraw_menu_image(3)/ownerdraw_menu_color(15)/shortcut_key(93)/tooltip(2)。`*.caption` は実リソース（widget でない）。全 resource dispatch="get"、tooltip/balloon_tooltip は response="sakura_script"・他 scalar は "text"。ReferenceN 引数を持つ 4件（tooltip/balloon_tooltip/getaistateex/other_homeurl_override）のみ field 化、残 155 は引数なし scalar。→ 4.1(2投影) は event+resource 全 446 entry の field を対象に canonical/alias 整合を検証すること。
- 3.3: Event ページ網羅は **`<dl id=...>` アンカー集合 ⇔ TOML event id の 1:1 アサーション**で機械検証（caption/widget アンカー除外: ghost_*/supported_*/caption_*）。ドット形式 ID（`OnUpdate.OnDownloadBegin`/`OnUpdateOther.*`/`OnHeadlinesense.OnFind`）は **verbatim 保持**しカテゴリは兄弟（network_update/rss）へ寄せる。`OnUpdateOther.OnMD5CompareComplete/Failure` は ukadoc に Reference0 が無く field は Reference1 始まり（捏造禁止・原文忠実）。列挙意味は `response_meaning` 不使用の既存形に倣い field.description へ符号化。→ 3.4(resource) も同様に list_shiori_resource.html の `<dl>` アンカー 1:1 で網羅検証すべし。
- 3.2→4.2 引き継ぎ: 文脈依存 dispatch を 4.2 で `[[silence_ruling]]`(topic=dispatch_class) に裁定 — OnSpeechSynthesisStatus/OnVoiceRecognitionStatus は `[NOTIFY/他GET]`（現状 notify 採用）、OnOtherSurfaceChange は無印だが通知意味で notify 採用、*InputCancel 系（OnTeachInputCancel/OnCommunicateInputCancel/OnUserInputCancel）は応答期待が沈黙（現状 get/sakura_script 既定）。OnMouseClick/ClickEx の R2 は ukadoc「常に0」だが位置整合で wheel_delta 命名（OnMouseWheel/Move では always_zero）。
