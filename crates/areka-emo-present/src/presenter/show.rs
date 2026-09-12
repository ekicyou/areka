//! `ShowSurface` の適用（`EmoPresenter::apply_show`）——k 導出・引き当て／合成＋表示記録・装着の遅延生成・
//! 表示記録＋配置＋マスク同期＋可視化・表示成立点の状態更新を 1 呼び出しで行う単一漏斗。

use wintf::ecs::window::transition_diag;

use crate::display::record_display;

use super::timing::{EmitContext, FrameTiming, Stage, compose_key_hash};
use super::transition_record::{SurfaceRecord, SurfaceStage, frame_of, stamp_of, surface_line};
use super::{
    AlphaMaskResource, BindSet, ComposeError, DPI, EmoPresenter, GraphicsCore, PatternState,
    PresentError, PresentOutcome, ReplySender, TargetId, VisibilityOwnership, VisualMount, World,
    derive_scale,
};

impl EmoPresenter {
    /// `ShowSurface` の適用（キャッシュ引き当て or 合成＋表示記録 → 表示記録・配置の書き込み → マスク同期 → 可視化）。
    ///
    /// 手順（design §System Flows・Flow 1・裁定 D）: (1) 未装着なら error! ＋ `Err(TargetNotAttached)`。
    /// (0) 窓の `DPI` component と target 政策から**この適用に使う k**を導出する（[`derive_scale`]・
    /// component 不在は `None` のまま渡して要件 1.4 の縮退へ落とす）。**k はキー要素ではない**——
    /// 合成入力（surface id＋bind 集合＋pattern）が完全一致するヒットなら再合成しない（R4.2）し、
    /// k だけが変わった適用もヒットする（要件 5.3）。(1) ミスなら合成し（native 原寸）、
    /// `SurfaceNotFound` は error! ＋表示不変＋`Err`（R3.4）、`EmptyComposition` は warn! ＋ Hide 縮退＋
    /// `Ok`（設計ディスカッション #1）、`Ok` なら原寸バイトから表示記録（閉じた `GraphicsCommandList`）を
    /// 起こし、マスクを原寸バイトから 1 回だけ生成して `cache.insert` へ面・マスク・記録の三つ組で渡す。
    /// (2) `mount` 未生成ならエントリの表示記録込みで遅延生成し、(3) 表示記録 → 配置（原寸・k）→
    /// マスク → 可視化の順に同一呼び出し内で書く（R2.4）。拡大は wintf の描画経路
    /// （`render_surface` の `SetTransform`）が `Arrangement.scale`＝k で掛ける——この漏斗は k の値で
    /// 手順を分岐しない（要件 1.3／7.3）。
    ///
    /// # 毎フレーム経路は再利用席の上を走る（`areka-P0-recompose-budget` Requirement 3.1・Flow 2）
    ///
    /// ミス経路のバッファ（native 合成先・表示バッファ・当たり判定マスク）はいずれも [`FrameBudget`] の
    /// 席か、キャッシュ追い出しエントリの容量回収で回る。定常状態（寸法不変・初回確保後）ではこの
    /// 経路の新規確保は 0 であり、確保が起きた回だけが perf サマリ行の `alloc_*` に現れる。
    ///
    /// [`FrameBudget`]: super::budget::FrameBudget
    ///
    /// # 可視化手順だけが所有権でゲートされる（`areka-P0-balloon-visibility` Requirement 1.1/6.2/6.9）
    ///
    /// [`VisibilityOwnership::External`] の target では、末尾の可視化（`set_visible(true)`／`visible=true`）
    /// と mount 遅延生成の初期可視性だけが変わり、**それ以外の全手順は所有権に依らず共通**である
    /// （k 導出・合成・キャッシュ・表示記録・配置・マスク・`applied`／`native_size`／
    /// `last_show`／`pending_resize`・下の `info!`）。表示成立点が 1 つであることを崩さないための境界であり、
    /// 「不可視のまま配置先と面が確立する」（Requirement 1.3）はこの共通部分がそのまま与える。
    /// 可視化は [`EmoPresenter::show_target`] が同じ漏斗を再通過したうえで付与する。
    pub(super) fn apply_show(
        &mut self,
        world: &mut World,
        target_id: TargetId,
        surface_id: u32,
        binds: BindSet,
        pattern: PatternState,
        reply: Option<ReplySender<PresentOutcome>>,
    ) {
        // (−1) 段階計時の起点（Requirement 1.1・design D5）。**無条件**に開始する——ログ設定を
        // 一切参照しないため、計時ログの有効・無効で表示経路が分岐しない（Requirement 1.5）ことが
        // 檻ではなく構造で成立する。`emit` は `self` を消費するため、以降の early return は
        // すべて計時器を黙って drop する＝**成立点の対でしか行が出ない**（design §FrameTiming）。
        let mut timing = FrameTiming::start();

        let Some(target) = self.targets.get_mut(&target_id) else {
            tracing::error!(
                ?target_id,
                surface_id,
                "apply(ShowSurface): 未装着ターゲット"
            );
            Self::reply(reply, Err(PresentError::TargetNotAttached(target_id)));
            return;
        };

        // (0) k 導出（show 適用ごと・design Flow 1）。窓 DPI は wintf の `DPI` component から読む
        // （consume のみ・新規依存なし）。**component 不在は `None` のまま [`derive_scale`] へ渡す**——
        // ここで 96 を捏造すると要件 1.4 の縮退（error! ＋ k=1.0）が「正常系のふり」で通ってしまう。
        let window = target.window;
        let window_dpi = world.get::<DPI>(window).map(|d| (d.dpi_x, d.dpi_y));
        let scale = derive_scale(target.policy, window_dpi);

        // (1) 引き当て: 合成入力（id＋binds＋pattern）の完全一致のみヒット＝再合成しない（R4.2/R5.2）。
        // **k はキーに参加しない**（要件 5.2）——面・マスク・表示記録はいずれも原寸で k を含まないため、
        // k だけが変わった適用は同じエントリにヒットし、変換の係数（下の `set_layout`）だけが変わる。
        // ミスのみ合成する。pattern は指令が運ぶ現在コマ集合をそのまま透過する（presenter は新しい
        // 判断を持たず輸送のみ）。空 PatternState なら拡張前と観測等価（R5.4）。
        //
        // **`touch` であって `get` ではない**（要件 7.1・容量 3 の LRU）。ここが 1 適用に 1 回の
        // 引き当て点＝最近使用順を動かす唯一の場所である。`get`（順序を動かさない読み取り）へ
        // 替えると置換は挿入順（FIFO）へ静かに退化し、容量 3 の裁定の根拠である LRU 再生の
        // 命中率と実装が対応しなくなる——表示バイトも確保計数も変わらないため、その退化は
        // 専用の檻（`presenter_cache_capacity_tests.rs`）だけが捕まえる。
        let cache_hit = target.cache.touch(surface_id, &binds, &pattern);
        timing.mark(Stage::CacheLookup);
        if !cache_hit {
            // 合成先は [`FrameBudget`] の常設席（設計 D2⑴・Flow 2）。`compose_into` は席の容量を
            // 再利用するため、外形の変わらない反復では 1 バイトも確保しない。席を**閉包で借りる**のは、
            // 合成外形が `compose_into` の内側で決まり、伸長の観測を「席を使い終えた直後」にしか
            // 置けないためである（`FrameBudget::native_scratch` の doc）。確保の計数は席メソッドの
            // 内側で起き、その意味は「席の再利用が成立せず結局確保した回」である。
            //
            // 合成結果を先に束縛して境界を作る（計時点 `Stage::Compose` を合成の直後に置くため）。
            // 分岐・呼出順序・エラー経路は下の `match` がそのまま担い、挙動は不変である。
            let composed = target.budget.native_scratch(|scratch| {
                target
                    .composer
                    // pattern を合成入力の第一級要素として合成器へ透過する（R5.1）。
                    // 合成は常に native 原寸（emo-compose の合成経路は k を知らない）。外形は席
                    // そのものが持つ——エントリの `composed` 外形として下流が読む。
                    .compose_into(
                        scratch,
                        &target.emo_world,
                        &target.atlas,
                        surface_id,
                        &binds,
                        &pattern,
                    )
            });
            timing.mark(Stage::Compose);
            match composed {
                Ok(()) => {
                    // (1a) 表示の記録（設計 Flow 1・要件 1.1／7.1）: 合成した**原寸**バイトから
                    // 閉じたコマンドリストを起こし、下の `insert` でエントリへ束ねる。記録は k を
                    // 含まない（拡大は wintf の変換行列が掛ける）。
                    //
                    // **失敗し得る GPU 呼び出しはここに集約**され、位置が回収（`take_recycled`）より
                    // **手前**であることが失敗時の前状態維持を与える（design Flow 1 キー決定）——
                    // ここで失敗すればメモ・表示記録・配置・マスク・可視性・`applied`／`native_size`／
                    // `last_show`／`pending_resize` の 1 つも動いていない。記録元は合成先席そのもの
                    // （下の交代より前ゆえ、まだ原寸が入っている）。
                    let display_list = {
                        let Some(dc) = world
                            .get_resource::<GraphicsCore>()
                            .and_then(|gfx| gfx.device_context())
                        else {
                            tracing::error!(
                                ?target_id,
                                "apply(ShowSurface): GraphicsCore 不在または DeviceContext 不在（表示を記録できない）"
                            );
                            Self::reply(
                                reply,
                                Err(PresentError::Device {
                                    hresult: 0,
                                    context: "GraphicsCore resource",
                                }),
                            );
                            return;
                        };
                        // 席は読むだけだが、貸し出し口は 1 つしかない（`native_scratch`）。伸長は
                        // 起きないため確保の計数も動かない。
                        match target
                            .budget
                            .native_scratch(|scratch| record_display(dc, scratch))
                        {
                            Ok(list) => list,
                            // record_display は内部で error! 済み（display.rs device_err）。
                            Err(e) => {
                                Self::reply(reply, Err(e));
                                return;
                            }
                        }
                    };
                    // `Stage::Upload` の区間＝原寸 D2D bitmap の生成＋描画命令の記録＋Close。
                    timing.mark(Stage::Upload);
                    // **容量回収は合成・記録の成功後に限る**（設計 Flow 2 の規律・`take_recycled` の契約）。
                    // 失敗し得る位置でこれを呼ぶとスロットが空のまま残り、「失敗時は表示もキャッシュも
                    // 適用前のまま」（R3.4・設計 §Error Handling）が崩れる——直後の同一入力の適用が
                    // ヒットせず再合成へ落ちるためである。手前の失敗経路は全て早期復帰済みゆえ、
                    // ここへ到達した時点で合成も記録も成功している。
                    let recycled = target.cache.take_recycled();
                    // 追い出しエントリの表示バッファ容量を受け取り、束ねられていたマスクは輪番の
                    // 空きスロットとして下の `regenerate_mask` へ回す（設計 D2⑵/D3）。回収が
                    // 成立しなければ空バッファから始まり、以後の伸長がそのまま計数される。
                    let (mut display, retired_mask) = target.budget.display_buffer(recycled);
                    // 合成先席と表示バッファを**交代**させる（複写も確保も起きない・設計 D2⑴）。
                    // **k の値に依らず常にこの経路**（要件 1.3・7.3）——表示へ載せるのは原寸そのもの
                    // であり、k 適用の段は存在しない。交代後は回収した容量が合成先席に入り、次の適用は
                    // そちらへ合成する。
                    target.budget.swap_native_scratch(&mut display);
                    // マスクをこの適用で 1 回だけ**原寸バイト**から生成する（R2.1/R2.4・要件 4.1）。
                    // 生成点は `cache.insert` の内側から**予算シームの輪番へ移った**（設計 D4/D3）。
                    // `retired_mask` は直前の適用で表示に使ったマスクで、これが次の空きスロットへ回り、
                    // 代わりに前々回のマスク（下流が既に手放して単独所有）が再生成先として取り出される。
                    // 挿入は表示バッファとマスクを同時に受け取るため、対が崩れないことは引き続き
                    // 構造で担保される。÷k は wintf の `alpha_mask_hit` が物理寸の境界に対する比例
                    // 写像で 1 回だけ掛ける（要件 4.2）。
                    let mask = target.budget.regenerate_mask(
                        retired_mask,
                        display.bytes(),
                        display.width(),
                        display.height(),
                        display.stride(),
                    );
                    // pattern は binds と同格のキー要素として挿入キーへ透過する（R5.2）。原寸の外形は
                    // `composed` そのものが持つ（別フィールドで二重に持たない・要件 5.1）。
                    target.cache.insert(
                        surface_id,
                        binds.clone(),
                        pattern.clone(),
                        display,
                        mask,
                        display_list,
                    );
                    // `Stage::MaskGen` の区間はマスク生成＋挿入全体（スロット置換）を含む。旧エントリは
                    // `take_recycled` で先に回収されているため、この区間から対の解放 churn（is-a A6）は
                    // 消えている。席の交代（O(1) の入れ替え）もこの区間に入る。
                    timing.mark(Stage::MaskGen);
                }
                Err(ComposeError::EmptyComposition(id)) => {
                    // 全透明退化（外形 0×0）: 許容される正常退化として Hide 縮退＋reply Ok（skip ではない）。
                    tracing::warn!(
                        ?target_id,
                        surface_id = id,
                        was_visible = target.visible,
                        "apply(ShowSurface): 全透明退化（EmptyComposition）→ Hide 縮退（reply Ok）"
                    );
                    if let Some(mount) = target.mount.as_ref() {
                        mount.set_visible(world, false);
                    }
                    target.visible = false;
                    // EmptyComposition 縮退は Hide と同じ表示結果ゆえ現サーフェス無し（R3.2・Key decisions (b)）。
                    target.current_surface_id = None;
                    Self::reply(reply, Ok(()));
                    return;
                }
                Err(e) => {
                    // 解決不能 id（SurfaceNotFound 等）: error! ＋ 表示不変 ＋ reply Err（R3.4）。
                    tracing::error!(
                        ?target_id,
                        surface_id,
                        error = %e,
                        "apply(ShowSurface): 合成失敗 → 表示は適用前のまま（reply Err）"
                    );
                    Self::reply(reply, Err(PresentError::Compose(e)));
                    return;
                }
            }
        }

        // 使えるエントリ（引き当て済み・順序は動かさない `get`）。原寸はエントリの `composed` 外形、
        // 物理寸は**丸め権威の式 1 つ**（`scaled_extent`・要件 2.1／2.2）で導く——自前供給面の寸を
        // 照会する形は消え、照会値（`target_physical_size`）と同じ式で同じ値になる。
        let entry = target
            .cache
            .get(surface_id, &binds, &pattern)
            .expect("直前に引き当て済み");
        let native = (entry.composed.width(), entry.composed.height());
        let physical = scale.scaled_extent(native.0, native.1);

        // (2) 装着の遅延生成（初回表示・原寸確定後）。surface entity はエントリの表示記録込みで
        // spawn する（純 ECS・失敗経路なし）。`WucGraphicsResource`／`Compositor` は要らない。
        if target.mount.is_none() {
            // 初期可視性は**所有権から導出**する（`areka-P0-balloon-visibility` Requirement 1.2）。
            // `CommandDriven` は従来どおり可視で構築（この漏斗の末尾で `set_visible(true)` する経路と
            // 同値）、`External` は不可視で構築する——「可視で spawn してから消す」経路を持たないことで、
            // 可視状態を component レベルでも一度も経由しないことを構造で保証する。導出をここで行う
            // （装着側の既定に委ねない）のは、可視性の所有者を知っているのがこの層だけだからである。
            let initially_visible = matches!(target.ownership, VisibilityOwnership::CommandDriven);
            target.mount = Some(VisualMount::attach(
                world,
                window,
                native,
                scale,
                &entry.display,
                initially_visible,
            ));
        }

        // (3) 表示記録 → 配置 → マスク同期 → 可視化（同一呼び出し内＝原子入替・R2.4／要件 4.4）。
        //
        // **可視化手順だけ**が所有権でゲートされる（`areka-P0-balloon-visibility` Requirement 6.2/6.9）。
        // `External` の target では表示状態の確立（合成・表示記録・配置・マスク・k・面 id・再表示入力・
        // 窓寸 reconcile 要求）は従来どおり全て完了し、可視性の付与だけを行わない——面切替もループ由来の
        // 反復指令も、経路を問わず可視状態を変えない。
        let visualize = matches!(target.ownership, VisibilityOwnership::CommandDriven);

        // (3a) 状態照合の前値導出（design Flow 1 キー決定・議題 #2 裁定）。**前値を上書きする前に**
        // 前回表示の原寸と物理寸を組み立てる。物理寸は契約式 `applied.scaled_extent(native_size)`
        // （design §State Management）に従う——別フィールドで物理寸を二重に持つと更新点が 2 つになり、
        // 片方だけが書かれる欠陥を招く。
        //
        // `resized`（遷移観測・要件 7.6）は**原寸の外形が前回表示から変わったか**——k だけの変化は
        // false で、`size_changed`（物理寸の変化・窓寸 reconcile の材料）とは別の述語である。
        //
        // 前値なし（初回表示）は `None` ≠ `Some(..)` ゆえ**必ず差分扱い**になる。これは意図した
        // 設計である——窓は起動時 k₀ 見積もり寸で生成されており実窓 DPI 由来の k と一致する保証が
        // ないため、初回を黙らせると Flow 3 手順 5 の補正が永久に走らない。
        let resized = target.native_size != Some(native);
        let prev_physical = target
            .applied
            .zip(target.native_size)
            .map(|(k, (nw, nh))| k.scaled_extent(nw, nh));
        let size_changed = prev_physical != Some(physical);

        // (3b) 遷移観測: サーフェス更新の記録（Requirement 2.2・design C3）。
        //
        // **寸が動いた回だけ**、かつ**観測が有効なときだけ**組む。前置ガードが行の組立より外側に
        // 無いと、既定 OFF の運転でも毎フレーム `String` を確保することになり、`recompose-budget` が
        // 成立させた定常状態のアロケーション 0（Requirement 10.4）が壊れる。発行は `debug!` ゆえ
        // ガードの有無で**出力は 1 文字も変わらない**——この退行を捕まえるのは
        // `transition_record_tests.rs` の本文走査だけである。
        let observe_surface = (size_changed || resized) && transition_diag::is_enabled();
        if observe_surface {
            transition_diag::emit_line(&surface_line(&SurfaceRecord {
                // 刻印は World 資源から組む（D1）。本経路は `&mut World` を持つ観測点であり、
                // スレッド局所ミラーは World を借りられない点の専用口である。
                stamp: stamp_of(world),
                stage: SurfaceStage::Upload,
                target_id,
                size: Some(physical),
                resized: Some(resized),
                reason: None,
            }));
        }

        let mount = target.mount.as_ref().expect("直上で生成済み");
        // 表示記録: 値が異なるときだけ挿す（同一エントリの再適用で `Changed` を立てない）。
        mount.set_display(world, &entry.display);
        // 配置（原寸・k）: 同値なら書かない。bounds（＝αマスク座標基準・物理寸）は wintf が
        // `GlobalArrangement` へ導く。可視性に依らず常に合わせる——不可視中の更新は安全であり
        // （`HitTest::none()` 中はマスクが判定に使われない）、ここで揃えておくことが「不可視期間中に
        // 生じた変化を取りこぼさない」（Requirement 6.6）の土台になる。
        mount.set_layout(world, native, scale);
        if let Some(mut mask_res) = world.get_mut::<AlphaMaskResource>(mount.surface_entity()) {
            // 表示記録と同一の原寸 bytes 由来のマスクを hit-test へ供給する（R2.2/R2.5・要件 4.4）。
            // 供給は**共有参照の受け渡し**（`set_shared`）＝`Arc` の参照カウント増のみで、実体の
            // 複製は起きない（設計 D3・is-a A7 の消し方）。`set` を常に呼ぶ現行の観測形はそのまま。
            mask_res.set_shared(entry.mask.clone());
        } else {
            tracing::warn!(
                ?target_id,
                entity = ?mount.surface_entity(),
                "apply(ShowSurface): surface entity に AlphaMaskResource が無い（当たり判定は矩形/前状態）"
            );
        }
        if visualize {
            mount.set_visible(world, true);
            target.visible = true;
        }
        // 可視化後の記録（design C3）。表示記録・配置・マスクを揃え終えた点＝「描画内容がこの寸で
        // 見えるようになった」瞬間であり、判定側はこの行の `t_us` と当該窓の窓書込の `t_us` の差
        // （`visualize_to_write_us`）で「窓矩形と描画内容が食い違う区間」を測る（C7・設計討議 A-2）。
        // 条件は upload の記録と同じ 1 つの札を使う—— 2 度評価すると片方だけが変わる形を作れる。
        if observe_surface {
            transition_diag::emit_line(&surface_line(&SurfaceRecord {
                stamp: stamp_of(world),
                stage: SurfaceStage::Visualize,
                target_id,
                size: Some(physical),
                // 段階の意味を持たない 2 フィールドは番兵で残す（落とさない）。
                resized: None,
                reason: None,
            }));
        }
        // 表示確立＝この id が現サーフェス（全透明でも成立・α 非依存の単一真実源・R3.1/3.3・Key decisions）。
        target.current_surface_id = Some(surface_id);
        // ここが**表示成立点**＝ k・native 原寸・再表示入力の唯一の更新点（design Flow 1 キー決定）。
        // 手前の失敗経路はすべて early return 済みゆえ、失敗時は前 k・前表示が保たれる（要件 4.4）。

        // (3.5) 状態照合＝窓寸 reconcile 要求の生成（design Flow 1 キー決定・議題 #2 裁定）。
        //
        // 判定材料（`prev_physical`／`size_changed`）は手順 (3a) で既に導いてある——遷移観測が
        // それを要るためであり、値の意味も算出式も (3a) の説明のとおりである。要求を積むのは
        // 従来どおりここ（表示成立が確定した後）で、`applied`／`native_size` の上書きより手前である。
        if size_changed {
            // 差分あり＝呼び手（frame drain フェーズ）へ新物理寸を報告する。同寸のときは**何も触らない**
            // ——`None` を書き戻すと未消費の要求を殺してしまう（取りこぼしを作らない・べき等）。
            target.pending_resize = Some(physical);
        }

        target.applied = Some(scale);
        // いま表示に使ったエントリ由来の原寸をそのまま写す（合成した回か否かで分岐しない——分岐させると
        // 「insert 済みのまま失敗 → 後からヒットで成立」の経路で照会値が画面と乖離する）。
        // 容量 3 ではヒットしたエントリが直前の挿入とは限らないため、エントリの `composed` 外形から
        // 読む（要件 7.1）。
        target.native_size = Some(native);
        // 合成キーの安定ハッシュ（perf サマリ行の `key_hash`・Requirement 7.2 の裁定材料）は
        // `last_show` への move の**直前**に取る。全段の `mark` が済んだ後なので、この走査
        // （借用のみ・確保なし）は段別所要へ混入せず `t_total_us` にだけ含まれる。
        let key_hash = compose_key_hash(surface_id, &binds, &pattern, scale);
        target.last_show = Some((surface_id, binds, pattern));

        // 表示成立点の観測ログ（設計 D10・要件 6.1/6.3 の判定素材）。実機サインオフは有界 auto-exit で
        // 起動し `RUST_LOG` を grep してここを読むため、**`info!` レベル**であることが契約である
        // （`debug!` へ落とすと既定の観測条件で消える）。k 導出値（`k`・`k_ratio`）と適用寸（`native_*`・
        // `scaled_*`＝物理寸・要件 2.8／7.5）が揃うことで、2 水準（125%/200%）の実行が「異なる物理寸で
        // 描かれた」ことをログだけで決定論的に判定できる。フィールドの名と意味は不変。
        tracing::info!(
            ?target_id,
            surface_id,
            cache_hit,
            // k の有理表現（既約 num/den）。`ScaleRatio` の num/den は非公開ゆえ `Debug` で出す。
            k_ratio = ?scale,
            k = scale.as_f32(),
            author_dpi = target.policy.author_dpi,
            // `None` は要件 1.4 の縮退（DPI component 不在 → k=1.0）そのものゆえ潰さずに出す。
            window_dpi = ?window_dpi,
            native_w = native.0,
            native_h = native.1,
            scaled_w = physical.0,
            scaled_h = physical.1,
            // 今回の表示成立が窓寸 reconcile 要求を積んだか（議題 #2 裁定の状態照合の観測点）。
            size_changed,
            "apply(ShowSurface): 表示・マスクを更新"
        );

        // perf サマリ行（Requirement 1.1/1.3・design.md §Data Models）。上の info! と**同一適用の対**
        // として隣接して出る別行である。ここが唯一の emit 点＝早期復帰の経路では 1 行も出ない。
        //
        // `take_delta` は増分を取り出して器を 0 に戻す。早期復帰で取り出されなかった増分は次の適用へ
        // 持ち越される（budget.rs の契約）——確保は実際に起きているため、成立した次の行に現れるのが
        // 正しく、累積（`cumulative`）は常に厳密である。
        let allocs = target.budget.take_delta();
        timing.emit(
            &EmitContext {
                target_id,
                surface_id,
                cache_hit,
                key_hash,
                // 遷移観測と**同じ資源**から読む（Requirement 2.8・D12）。供給元が 2 つになると
                // 「perf 行と surface 行が同一フレームで突合できる」が静かに壊れる。
                frame: frame_of(world),
            },
            allocs,
        );
        Self::reply(reply, Ok(()));
    }
}
