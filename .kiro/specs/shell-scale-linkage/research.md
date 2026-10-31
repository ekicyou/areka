# ギャップ分析: shell-scale-linkage

> 2026-10-31・本ブランチ（main `9604677a` の上の `dba36c6c`）のコードで確かめた。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。
> 入力: `requirements.md`（確定）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の C15 の行）。SSP は動かしていない（測る点は 6 節に挙げた）。

## 1. 要約

- **読む口・送る口・descript の読み・受け手の表は、どれも先例の形がそのまま使える。** 値は ✅ `mcp-scope-rect-property` の読み手の登録（`SylphyaReader::register_pull`）に載せれば MCP の `get_property` とゴーストの道の両方に同時に出る。`OnShellScaling`・`OnBalloonScaling` は UI の側から `KanadeMsg::RaiseEvent` で送る形（`frame/switch.rs` の `raise_changed`）があり、許可の表 `ALLOWED_EVENT_IDS` に 2 語を足すだけで通る。`balloon.syncscale` は `read_balloon_dontmove` とほぼ同じ形で読める。
- **足りないのは 3 つ。** ⑴ 名前の自由なイベントを出どころ `internal,property` で送る kanade の種類（`EventId`・`KanadeMsg`・`Input` に 1 つずつ、出どころの定数 1 つ）。⑵ 「絵にかかった倍率が変わった」を知る点（今は `EmoPresenter` の `applied` が変わったことを外へ知らせる口が無い）。⑶ 倍率を百分率の正確な文字にする口（`ScaleRatio` は分子・分母を外へ出さず、`as_f32` は使ってよい先が 2 つに限られている）。
- **思っていたより難しい点が 2 つ。** ⑴ シェル・バルーンの切り替えでは `ReplaceTarget` が `applied` を引き継ぐので、作者の基準の DPI が違う絵に替わると同じ target の倍率が跳ぶ＝「変わった」と取り違える（要件 5.6 を守るには切り替えの印が要る）。⑵ ukadoc は `\![get,property]` を「このスクリプトの再生後、SHIORIイベント…が通知される」と書いており、要件 1.2・1.6 の「タグに届いた時点で送る」と食い違う読みがある（SSP で測る）。
- **案は 3 つ（7 節）。** 受け手を ⒜ `areka-ghost` の中（`PropSetCueSink` の隣）に置くか、⒝ `emo2_boot` に置くか、倍率の変わりを ⒜ 毎フレーム比べるか、⒝ 表示層の更新点から押し出すかの組み合わせ。推奨の組（案 C）は「受け手は `areka-ghost`・倍率の変わりは表示層から押し出す・`syncscale` は資源の変わりで読む」。
- **規模 M（11〜16 タスク）・危険 中。** 先例どおりの所が大半で、新しいのは kanade の 1 種類と表示層の変わりの知らせ。切り替えの取り違えと SSP の往復の測りが残る。

## 2. 今あるもの（調べた場所）

### 2.1 台本の命令の受け口と受け手の表

| もの | 場所 | 分かったこと |
|---|---|---|
| 受け手の表 | `crates/areka/src/emo2_boot/consumer_ledger.rs` の `ConsumerLedger::canonical` | 42 件。`(get, property)` はまだ無く、`get` の名前の登記も他に無い（選別子つきで 1 行足せる）。`check_script` は `mcp/check_script_judge.rs` で `ledger.consumer_of(name, first)` を引くので、行を足せば要件 1.8 は自動で満たす |
| emo2 の外の受け口の先例 | `CommandConsumer::PropertyWrite`（`areka_ghost::prop_sink::PropSetCueSink`） | 受け口が `areka-ghost` に在っても表に行を持てる。合意の檻 `consumer_ledger_agreement_tests.rs` の `emo2_sink_of` は `PropertyWrite` を `None`（emo2 の受け口でない）として扱う＝同じ扱いで足せる |
| 受け口の組み立て | `emo2_boot/runtime_boot.rs` の `runtime_sinks`（13 本）／`areka-ghost/src/runtime.rs` の `boot_with_origin` | `boot_with_origin` は `spawn_dispatcher` の直前の「単一登録点」で `PropSetCueSink` を足す。その場所には `sylphya_reader`・`ghost_asker`・`kanade_tx` が全部揃っている |
| 運び手 | `dola::cue::TalkCue`（`at`・`actor`・`command`・`duration`） | 出どころ（`ScriptOrigin`）の欄は無い＝受け口は「どの出どころの台本のタグか」を知らない（要件には要らないが、返しのイベントの `SecurityLevel` を元の台本から継ぐ案は取れない） |
| `get` の特別扱い | `areka-parsers`・`areka-sakura`・`dola` | `"get"` の特別扱いは無い＝汎用の運び手で `("get", ["property", イベント名, 名前…])` として届く |
| 出どころで止めるタグ | `emo2_boot/script_restriction.rs` の `RESTRICTIONS`（5 行） | `\![get,property]` は ukadoc が制限しないので載っていない（足す必要なし） |

### 2.2 値の読み口

| もの | 場所 | 分かったこと |
|---|---|---|
| 読み手の登録 | `areka-sylphya/src/pull.rs` の `PullSource`・`PullShape`／`reader.rs` の `register_pull` | 形 `currentghost.scope(*).scaling` などで登録できる。写しに無い点付きの名前だけが読み手に届く。今、`scaling`・`currentmonitor` を写しへ出している所は無い |
| 手本 | `crates/areka/src/ghost_session_scope_rect.rs`（279 行） | 実行系ごとに `wire_reader` が `Arc<RwLock<表>>` を作って読み手に渡し、`FrameFinalize` の系 `sync_scope_window_table` が変わったときだけ表を差し替える（プッシュ）。スコープの選択子の読み `scope_selector` は私有の関数（使い回すなら `pub(crate)` に上げる） |
| 登録の場所 | `ghost_session_runtime.rs` の `rewire_runtime_consumers`（`ghost_session_scope_rect::wire` を呼ぶ）・`ghost_session.rs` の `register_systems` | ゴーストの切り替え・読み直しで表が新しくなる＝要件 4.5 は形で満たす |
| MCP | `mcp/get_property.rs` の `handle` | `resolve_dotted_str(&asker, name)` で引き、無ければ `NG:Cannot find such property name.`。`\![get,property]` の受け手も同じ関数を同じ問い手（`ghost_asker_id`）で呼べば、要件 1.1（名前の規則・大小の扱い）・1.5（名前を問わない）・3.10（同じ値）は同じ道を通るので揃う |
| ゴーストの道 | `crates/areka/src/shiori_host.rs`（`GetProperty`） | 同じ置き場を読む。本番は 32bit の中継越しで届かない＝決定論のテストで確かめる（`mcp-scope-rect-property` の確かめた点 4 と同じ） |

### 2.3 倍率の真実源

| もの | 場所 | 分かったこと |
|---|---|---|
| 実効の倍率 | `areka-emo-present/src/presenter/read.rs` の `applied_ratio`（`ScaleRatio`）・`applied_scale`（`f32`） | 「いま画面に載っている絵に実際に掛かった k」。一度も表示していない target は `None`。**更新点は `presenter/show.rs` の `target.applied = Some(scale)` の 1 か所**（と `presenter/replace.rs` の引き継ぎ） |
| k の式 | `areka-emo-present/src/scale.rs` の `derive_scale`・`ScalePolicy { author_dpi, app_scale }` | k ＝ `app_scale × (窓の dpi_x ÷ author_dpi)`。**「アプリ管理拡大率」`app_scale` はもう在り、今は `ScaleRatio::ONE`**（`ScalePolicy::new` の呼び手が 1 を渡す）。要件 3.3 の「DPI 追従の倍率 × 表示の倍率」はこの値そのもの。縦横で DPI が違えば `dpi_x` を採る（単一のスカラー）＝要件 3.6 の「横と縦に同じ数」はこの規約に合う |
| 正確な値 | `areka-emo-compose/src/scale.rs` の `ScaleRatio`（既約の分子・分母） | 分子・分母を返す公開の関数が無い。`as_f32` は doc に「裁定済みの消費者は 2 つだけ・他の用途へ拡大してはならない」とある＝百分率の文字をここから作るのは規約に反する |
| 更新が走る相 | `emo2_boot/frame.rs` の `emo2_frame_system` | k が変わる点は 1 フレームに 2 つ（`run_dpi_phase` の `refresh_scale` と `run_drain_phase` の `apply_show`）。両方の下流に `run_text_scale_phase`（毎フレーム走査・D11-4 で理由づけ済み）と、相の並びの最後の `status_report::run_status_report_phase`（「前に送った組と違うときだけ kanade へ送る」台帳つき）がある |
| `refresh_scale` の戻り値 | `frame/dpi.rs` の `dpi_phase_with` | `None` は「k が変わったが丸めた寸が同じ」「見えていない」も含む＝**戻り値は「倍率が変わった」の印に使えない**（doc が明記） |
| DPI | 窓の `wintf::ecs::DPI`（`dpi_x`・`dpi_y`） | 窓ごとの値。整合待ちの札 `DpiSyncHold`（`placement/dpi_sync.rs`）が掛かっている間や、見えていない間は、DPI が新しく k が古いフレームがある |

### 2.4 イベントを送る口（kanade）

| もの | 場所 | 分かったこと |
|---|---|---|
| 汎用の通知の入口 | `areka-kanade/src/msg.rs` の `KanadeMsg::RaiseEvent`・`schedule/change.rs` の `on_raise_event` | 許可の表に在る名前を、定常（`Phase::Steady`）でだけ送る。定常以外は `warn!` で捨てる（積まない）。UI から送る手本は `emo2_boot/frame/switch.rs` の `raise_changed`（`OnShellChanged` などを GET・返信なしで送る） |
| 許可の表 | `schedule/events.rs` の `ALLOWED_EVENT_IDS`（64 語） | `OnShellScaling`・`OnBalloonScaling` は無い。数の檻 `schedule/events_change_tests.rs` の `assert_eq!(ALLOWED_EVENT_IDS.len(), 64)` を 66 に直す |
| 名前の自由なイベントの先例 | `EventId::Choice`（`On` で始まる名前だけ）・`EventId::Sstp`（`sstp::validate_notify`） | `EventId::Sstp` は出どころが `MCP`／`external,sstp` に固定（`request_origin.rs` の `of_event`）。**`internal,property` を名乗れる種類は無い** |
| 種類が増えたときに要る腕 | `request_origin.rs` の `of_event`（既定の腕なし＝コンパイラが求める）・`actor.rs` の送出前の名札（`EventId::Sstp(_) => NOTIFY_ORIGIN`）と許可の判定（`EventId::Sstp(name) => validate_notify(...)`）・`schedule/mod.rs` の `Input` の腕・`schedule/types.rs` の `Input` | `EventId::Sstp` を足したときと同じ 5〜6 か所 |
| 出どころの語 | `areka-talk/src/origin.rs` の `SenderWord::Property`・`ScriptOrigin` の定数（`GHOST`・`MCP_SEND`・`MCP_NOTIFY`） | 語はあるが、組の定数 `PROPERTY`（`internal,property`・SSTP でない・`local`）は無い |
| 名前の検査 | `areka-kanade/src/sstp.rs` の `validate_notify` | 空・空白だけ → `EmptyEvent`、名前か Reference に CR・LF・NUL → `ControlCharacter`。要件 2.2・2.3 の規則とちょうど同じ＝そのまま使える |

### 2.5 descript

| もの | 場所 | 分かったこと |
|---|---|---|
| 手本 | `placement/descript_window_keys.rs` の `read_balloon_dontmove` | シェル `<スコープ>.balloon.X`（`sakura.`＞`char0.`・`kero.`＞`char1.`・`char<n>.`）とゴースト `balloon.X` を読む。ただし**優先の順**（シェル＞ゴースト）で、値は `0`・`1`・`true`・`false`。要件 6.3・6.4 は **論理和**（どちらかが `true`）で `true` だけが有効＝読み方の規則が違う |
| 値の流れ | `placement/mod.rs` の `prepare_stages_for_shell` → `PreparedStages`・`PreparedPlacement` の欄 → `ghost_session.rs` → `placement::balloon_lock::seed_balloon_lock`／シェルの切り替えは `frame/switch.rs` が読み直し | `balloon_dontmove` と同じ形で持つと `placement/mod.rs` に 5 行前後（読み・2 つの構造の欄・受け渡し）＝ウェーブの約束「1〜2 行」を超える |
| 抜け道 | `placement/reset_pos.rs` の `DefaultPlacementBasis { source: DescriptSource, .. }` | **起動の窓を作る閉包（`ghost_session.rs`）とシェルの切り替え（`frame/switch.rs`）の両方が、読み込んだ descript の生の KV（`ghost_kv`・`shell_kv`）ごと資源として入れ直す。** この資源の変わりを見る系を足せば、`placement/mod.rs` にも `switch.rs` にも触らずに読める |

### 2.6 網羅台帳

`doc/ukadoc-coverage/ledger/`: `property.toml` の 3 行（`currentghost.scope_28ID_29.scaling`・`currentghost.balloon.scope_28ID_29.scaling`・`currentghost.scope_28ID_29.currentmonitor.dpi`・どれも `vocabulary-only`・持ち主 `shell-scale-linkage`）・`shiori.toml` の `OnShellScaling`・`OnBalloonScaling`・`sakura-script.toml` の `\![get,property,…]`・`assets.toml` のゴーストの `balloon.syncscale` とシェルの 3 行。**`property.toml` の 2 つの `scaling` の行の注記は「利用者がベースウェアで決める拡大率」と SSP の意味で書いてある**＝実装済みへ改めるときに DPI 追従の意味へ書き直す必要がある。

### 2.7 ぱすたの実物（`target\pasta-verify\root\ghost\pasta-in-windows`・読むだけ）

- `\![get,property]` を出すのは `OnSecondChange` の応答（喋っていないとき＝Reference3 が 1）と `OnShellScaling`・`OnBalloonScaling` の応答だけ。起動の挨拶の中では出さない。どれもタグの直後が `\e`。
- `OnPastaShellScale` は何も返さない（`return nil`）。`OnPastaScaleCheck` は横の値がずれたときだけ台詞（`＊バルーン倍率のずれ`）を返す＝**返しのイベントの応答の台本が再生されないと、この台詞は出ない**（要件 1.7 に効く）。
- 値の読みは `^%s*([%d%.]+)` で小数も読む。ぱすたの注記は「Windows の窓の影はモニタの DPI で大きくなる・ぱすたの窓の影はシェルの拡大率で大きくなる」＝areka の DPI 追従ではどちらも DPI で大きくなるので、`scaling × 96 ÷ DPI` が 100 になり面 10 を選ぶのが見た目の上で正しい（要件の Introduction のとおり）。

## 3. 要件ごとの対応表

| 要件 | 使えるもの | 足りないもの（Missing／Unknown／Constraint） |
|---|---|---|
| 1.1・1.5・3.10 | `resolve_dotted_str`＋`ghost_asker_id`（MCP と同じ関数） | — |
| 1.2・1.6 | 汎用の運び手・受け口の自己選別 | **Unknown**: 送る時点（タグの時点か台本の再生の後か・ukadoc は「再生後」）。**Missing**: 受け口 |
| 1.3 | `SenderWord::Property` | **Missing**: `ScriptOrigin` の定数・`EventId` の種類・`of_event` の腕 |
| 1.4・2.4 | Reference は `Vec<String>`（欠番は空文字列・詰めない＝`RaiseEvent` と同じ規約） | **Unknown**: SSP の答え（無い名前・名前なし） |
| 1.7 | 返しの応答を再生する道（`on_shiori_reply`／`sstp::on_notify_reply`） | **Unknown**: GET か NOTIFY か・話している最中の扱い。**Missing**: 応答の受け方の腕 |
| 1.8 | `ConsumerLedger`・`check_script_judge` | **Missing**: 表に 1 行（42→43）・見本の行 |
| 2.1〜2.3 | `sstp::validate_notify` | — |
| 2.5 | — | 記録の 1 行（`debug!`） |
| 3.1〜3.3・3.6・3.8 | `applied_ratio`（`app_scale × DPI ÷ author_dpi`） | **Missing**: 分子・分母を外へ出す口（`ScaleRatio`）。**Constraint**: `as_f32` の禁止 |
| 3.5 | 窓の `DPI` 部品 | **Unknown**: k と一緒に出すか、DPI の変わりで出すか（要件 4.2 との兼ね合い） |
| 3.7・5.3 | — | **Unknown**: 小数の桁（SSP で測る）。areka の作者の基準が 96 で Windows の DPI（24 の倍数）なら百分率は必ず整数＝小数が出るのは作者の基準が 96 でないときか、利用者の独自の拡大率（例 110%＝DPI 106）のとき |
| 3.9 | `ScopeRectSource::no_window` の形（値なし） | — |
| 4.1〜4.4 | 表のプッシュの手本（`sync_scope_window_table`）・`PullSource` の無待機の約束 | **Missing**: k が変わった点の知らせ（2.3 の更新点から押し出すか、毎フレーム比べるか） |
| 4.5・4.6 | `rewire_runtime_consumers` で表が作り直される・読み手の答えは写しにも保存にも書かない | — |
| 5.1〜5.3・5.8 | `RaiseEvent`・`raise_changed` の手本 | **Missing**: 許可の表に 2 語（64→66）・送る相 |
| 5.4 | 相の並び（`emo2_frame_system`） | **Constraint**: 表を書いてからイベントを送る順を同じ相の中で守る（1 フレーム遅らせない） |
| 5.5・5.7 | — | 本体の窓（スコープ 0）の k の前後の比較 |
| 5.6 | ゴーストの切り替え・読み直しは表が新品 | **Missing**: シェル・バルーンの切り替えの印（`ReplaceTarget` が `applied` を引き継ぐため・5 節の危険 1） |
| 5.9 | `OnMouseDragEnd` は今も送っている | 実機で確かめるだけ |
| 6.1〜6.5・6.7 | `read_balloon_dontmove`・`scoped_entry`・`DefaultPlacementBasis` | **Missing**: 論理和の読み・置き場の資源・記録。**Constraint**: `placement/mod.rs` は 970 行（上限 1000）でウェーブの約束は 1〜2 行 |
| 6.6 | — | 持つだけ（働きを変えない） |
| 7.x | 偽の読み手・偽の DPI を差す手本（`ScopeRectReadSeam`・`frame_test_support.rs`・`presenter_test_support.rs`） | 新しいテスト |
| 8.5 | 台帳の 4 本 | `property.toml` の注記の書き直し |

## 4. 新しく足すものの見積り（どの案でも要るもの）

1. kanade: `ScriptOrigin::PROPERTY`（`areka-talk/src/origin.rs` に定数 1 つ）・`EventId::Property(String)`・`KanadeMsg` の新しい知らせ（`RaiseEvent` は送り手が 9 か所なので欄を足さない）・`Input` の腕・`of_event` の腕（送り手 `areka`・出どころ `PROPERTY`）・`actor.rs` の 2 腕（名札と許可の判定＝`validate_notify`）・受け付けと応答の腕（`schedule/sstp.rs` の `on_notify`／`on_notify_reply` と同じ形の新しい兄弟か `change.rs`）。
2. 許可の表に `OnShellScaling`・`OnBalloonScaling`（数の檻 64→66）。
3. 倍率と DPI の読み手（新しい `ghost_session_scope_scale.rs`・`ghost_session_scope_rect.rs` の形）と登録（`ghost_session_runtime.rs` に 1 行・`ghost_session.rs` の `register_systems` に 1 行）。
4. 受け手の表に `(get, property)` の 1 行と説明の一覧・数の檻 42→43・合意の見本。
5. `balloon.syncscale` の論理和の読み（`descript_window_keys.rs` に関数 1 つ）と置き場（資源 1 つ）と記録。
6. 網羅台帳 4 本の行（`property.toml` の注記の書き直しを含む）。

## 5. 危険と難しい点

1. **切り替えで倍率が跳ぶのを「変わった」と取り違える（要件 5.6）。** `presenter/replace.rs` の `apply_replace` は `DetachedState.applied` を新しい装着へ引き継ぎ、同じフレームの `apply_show` が新しい作者の基準の DPI で k を書き直す。シェルの `seriko.dpi`・バルーンの `dpi` が前と違えば、同じ target の k が前後で変わって見える。区別の手がかりの候補: シェルの切り替えは `frame/switch.rs` が `DefaultPlacementBasis` を入れ直す（資源の変わり）・置き換えの世代 `NEXT_EPOCH`・`SkinSwitchInFlight` の段。バルーンの切り替えには `DefaultPlacementBasis` の入れ直しが無い＝別の印が要る。どのフレームで置き換えと表示が起きるか（同じフレームか）は設計で確かめる（**Research Needed**）。
2. **「倍率が変わった」を知る点。** `refresh_scale` の戻り値は使えない（2.3）。⒜ 毎フレーム、装着済みの target の `applied_ratio` を前の値と比べる（`run_text_scale_phase`・`status_report` の先例があり、数個の比較で軽い）か、⒝ 表示層の唯一の更新点（`show.rs` の `target.applied = Some(scale)`）で前後が違うときに知らせを積み、フレームの相で取り出す（変わった時点が分かる＝プッシュ）。開発者の基準「変わった時点が分かるならプッシュ・毎フレーム比べる案は推さない」（2026-10-23）に合うのは ⒝。⒝ は emo-present を触る（同じウェーブの ⑤ `emo-bake-log-overlap` は emo-present の `shell_target.rs`・`balloon.rs` だけ＝ファイルは重ならない）。
3. **DPI の値と k の食い違い（要件 4.2）。** 整合待ちの札や見えていない間は、窓の `DPI` が先に新しくなり k は古い。`currentmonitor.dpi` を k と同じ時点で出す（絵と食い違わない）か、`Changed<DPI>` で出す（画面には正しい）かで、ぱすたの `scaling × 96 ÷ DPI` が一瞬 100 以外になりうる。前者なら食い違いは起きない。
4. **SSP の往復の意味（要件 1.2・1.6・1.7）。** ukadoc の記述例は「このスクリプトの再生後、SHIORIイベント…が通知される」。`\![raise]` は「raiseが再生された時」と書き分けてある。タグの時点で送るか台本の終わりで送るか、GET か NOTIFY か（NOTIFY なら `OnPastaScaleCheck` の台詞は捨てられる）が答えで変わる＝SSP の MCP で測る（6 節）。
5. **ファイルの行の上限。** `placement/mod.rs` 970・`ghost_session.rs` 940（上限 1000）。`balloon_dontmove` と同じ運び方は約束を超える（2.5 の抜け道で避けられる）。
6. **ウェーブの表に無いファイル。** 案によって `areka-ghost/src/runtime.rs`＋新しい受け口のファイル・`areka-emo-compose/src/scale.rs`（分子・分母の口）・`areka-emo-present/src/presenter/show.rs`（変わりの知らせ）・`emo2_boot/frame/status_report.rs`・`areka-kanade/src/schedule/events_change_tests.rs` を触る。C15 の行の ②〜⑥ の触るファイルと突き合わせて**重なりは 0**（⑤ は emo-compose `fold.rs`・emo-present `shell_target.rs`・`balloon.rs`・`placement/measure.rs`、⑥ は kanade `schedule/resources.rs`・`menu/captions.rs`、③ は `alert.rs` だけ）。ただし `roadmap.md` の ① の欄と食い違うので、設計で挙げ直して開発者に示す。

## 6. SSP で測る点（Research Needed・開発者の機械の SSP の MCP）

要件の末尾の表の 2〜5 に、今回見つけた点を足した。

| # | 点 | 変わる受け入れ基準 |
|---|---|---|
| 1 | `\![get,property]` の返しのイベントが届く時点（タグの時点か台本の再生の後か・後なら `\e` までか、後ろの文字の表示が終わるまでか） | 1.2・1.6 |
| 2 | 返しのイベントは GET か NOTIFY か・応答の台本は再生されるか・話している最中に返った台本は今の台本を置き換えるか | 1.7 |
| 3 | 返しのイベントの `SenderType`・`SecurityLevel`・`Sender`（`ssp_shiori_log.txt` で読める） | 1.3 |
| 4 | 無い名前・名前なし・窓の無いスコープの `scaling`・`currentmonitor.dpi` の答え | 1.4・2.4・3.9 |
| 5 | `OnShellScaling` を送る場面（利用者がシェルの倍率を変えたとき・画面を移ったとき・シェルの切り替え）と、`OnBalloonScaling` との順 | 5.1・5.6・5.8 |
| 6 | 割り切れない倍率の小数の桁（例 シェル倍率 110% かつ作者の基準 144） | 3.7・5.3 |
| 7 | 返しのイベントが SSP の起動の挨拶の最中・終了の台本の最中に届くか（areka の `RaiseEvent` は定常以外で捨てる） | 1.2・1.7 |

## 7. 実装の案

### 案 A: 既存の場所に足す（受け手は `emo2_boot`・倍率は毎フレーム比べる）

- 受け手: 新しい `emo2_boot/property_get_cue.rs`（`WindowStateCueSink` の形・送出端 1 本）→ `runtime_boot.rs` の `CueSinkSources`／`runtime_sinks` を 13→14 本 → UI の側の相（新しい `frame/property_get.rs`＋`frame.rs`・`frame/wiring.rs`）で受け取り、`GhostSlot` の実行系の `sylphya_reader` で引き、kanade へ送る。
- 倍率: `frame/status_report.rs` の相に「本体の窓とバルーンの `applied_ratio` を前と比べ、違えば表を書いてから `RaiseEvent`」を足す（台帳は `Emo2Wiring` の中・ゴーストごとに新品）。
- `syncscale`: `balloon_dontmove` と同じ運び方（`placement/mod.rs`・`ghost_session.rs`・`frame/switch.rs`）。
- ✅ ウェーブの表の ① の欄にほぼ沿う。表示層に触らない。
- ❌ 受け手のために線・相・結線が増える（台本のスレッド → UI → kanade の 2 段の跨ぎ）。値はタグの時点でなく UI のフレームで引く（台本のスレッドの順と最大 1 フレームずれる）。毎フレーム比べる＝開発者の基準に反する。`placement/mod.rs` の約束を超える。

### 案 B: 新しい部品で分ける（受け手は `areka-ghost`・倍率は表示層から押し出す）

- 受け手: 新しい `areka-ghost/src/prop_get_sink.rs`（`PropSetCueSink` の兄弟）を `boot_with_origin` の単一登録点で足す。そこに `sylphya_reader`・`ghost_asker`・`kanade_tx` が揃っているので、台本のスレッドでタグに届いた時点で引き、検査し、新しい kanade の知らせで送る（UI を通らない）。受け手の表は `PropertyWrite` と同じく emo2 の外の担当として 1 行（`consumer_ledger.rs` の「11 の受け口」は 11 のまま）。
- 倍率: 表示層の更新点（`presenter/show.rs`）で前後が違うときに「target・前・後」を積み、取り出す口を足す。新しいフレームの相（新しい兄弟のファイル・`frame.rs` に 1 行・`wiring.rs` に台帳 1 欄）が取り出し、本体の窓とバルーンについて表を書いてから `RaiseEvent` を送る。切り替えの印で起きた変わりは送らない。
- `syncscale`: 新しい読み手のモジュールの中の系が `DefaultPlacementBasis` の変わりを見て読み、資源に持ち、スコープごとに記録する。
- ✅ 受け手の道が最短（線も相も足さない）。変わった時点のプッシュ。`placement/mod.rs`・`frame/switch.rs` に触らない。
- ❌ ウェーブの表に無い `areka-ghost`・`areka-emo-present` を触る。表示層の公開の口が 1 つ増える。

### 案 C: 組み合わせ（推奨の候補）

- 受け手は案 B（`areka-ghost` の兄弟）。
- 倍率の変わりは案 B（表示層から押し出す）。表の書き込みとイベントの送出は 1 つの相の中で「表 → イベント」の順。
- 正確な百分率は `ScaleRatio` に分子・分母（か百分率の文字）を返す口を 1 つ足す（`as_f32` を使わない）。
- `syncscale` は案 B（資源の変わりで読む）。
- `OnShellScaling`・`OnBalloonScaling` の送出は案 A と同じ `RaiseEvent`（許可の表に 2 語）。
- ✅ 先例の形に一番多く乗り、行の上限と約束を守る。❌ ウェーブの表との食い違いを設計で示す必要がある（5 節の 6）。

## 8. 規模と危険

- **規模: M（11〜16 タスク）。** kanade の 1 種類（5〜6 か所の腕）と表示層の変わりの知らせが新しく、残りは先例の写し。
- **危険: 中。** 形は既知だが、切り替えの取り違え（5 節の 1）と SSP の往復の意味（5 節の 4）が答えで作業を変える。

## 9. 設計の段へ持ち越す決めごと（要件討議の材料）

1. `scaling` に何を出すか（要件の表の 1）: コードの根拠は `applied_ratio`＝`app_scale × DPI ÷ 作者の基準` がもう「実効拡大率」そのもので、`app_scale` は今 1。
2. 受け手の置き場: `areka-ghost`（案 B・C）か `emo2_boot`（案 A）か。
3. 返しのイベントの送る時点と GET／NOTIFY（6 節の 1・2）。
4. 返しのイベントの kanade の受け付け: 定常だけか・起動の挨拶の最中も受けるか・`SecurityLevel` は常に `local` か（受け口は元の台本の出どころを知らない）。
5. 倍率の変わりを知る点: 毎フレーム比べるか、表示層から押し出すか。
6. 百分率の正確な文字の作り方と小数の桁（`ScaleRatio` に口を足すか・6 節の 6）。
7. 切り替えで倍率が跳んだときの見分け方（シェルとバルーンで別の印が要る）。
8. `currentmonitor.dpi` を k と同じ時点で出すか、DPI の変わりで出すか。
9. `syncscale` の持ち方（資源の変わりで読むか、`balloon_dontmove` と同じ運び方か）と、`char0.`・`char1.` の別名も読むか。
10. 2 つのイベントの代表のスコープ（要件の表の 6）: ukadoc の 2 つのイベントはスコープを持たず、areka は窓ごとに k を持つ。ぱすたは `scope(0)` だけを聞く。
11. 網羅台帳 `property.toml` の `scaling` の 2 行の注記（SSP の意味で書いてある）の書き直し。
12. ウェーブの表に無いファイルを触ること（5 節の 6）を `roadmap.md` の ① の欄へ足すか。
