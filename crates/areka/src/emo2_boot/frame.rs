//! 毎フレーム三相結線（attach／drain／text）の排他 system と NonSend 配線状態。
//!
//! `Emo2Wiring`（NonSend resource・presenter／rx／runtime／clock／assets／attached を保持）と
//! 排他 system `emo2_frame_system(world: &mut World)`（donor パターン: remove→3 フェーズ→insert）を
//! 所有する。三フェーズ:
//! - attach: GPU 資源＋`GhostWindows` 到達ゲート→`plan_attachments`（DD-12）→初回 `ShowSurface`→
//!   文字層スロット取得→`register_actor_view`（`Option::take` で高々 1 回消費）。
//! - drain: attach 完了後のみ `Receiver::try_iter` で `PresentCommand` を FIFO で `presenter.apply` へ適用。
//! - text: `TalkClock::talk_time` が `Some` のとき `present_frame` を呼ぶ（`Err` は `error!`＋継続）。
//!
//! `plan_attachments`（`GhostWindows::scopes()` を正とする純関数・DD-12）も本モジュールに属する。
//!
//! 本 task（tasks.md task 3）は純関数 `plan_attachments`（＋`AttachPlan`／`PlannedAttach`）のみを
//! 実装する。`Emo2Wiring`／`emo2_frame_system`／三フェーズ（attach／drain／text）の実装は後続
//! task 4.1（attach）／task 4.2（drain・text・`emo2_frame_system`）が担い、本ファイルにはまだ存在しない。

use areka_emo_present::TargetId;

use super::assets::BootAssets;
use super::target_map::{balloon_target, shell_target};

/// 窓×資産の scope 突き合わせ結果（DD-12・純関数 [`plan_attachments`] の戻り値）。
///
/// `GhostWindows::scopes()`（`usize`・昇順・**正**）を [`BootAssets`] の資産 scope と照合した
/// 装着計画。三分類は排他:
/// - `items`: 窓と資産の双方が揃った scope の装着項目（`window_scopes` の出現順を保つ）。
/// - `missing_assets`: 窓はあるが対応資産が無い scope（呼び手が `warn!`＋skip＝表示なし縮退）。
/// - `unused_assets`: 資産はあるが窓が無い scope（呼び手が `debug!`＋破棄）。
///
/// 純粋・決定論（GPU 不要）。呼び手（attach フェーズ）は `items.len()` が期待窓数と一致することを
/// 積極 assert し、warn+skip 縮退が scope 導出バグを隠さないことを檻に入れる（DD-12・spine S1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachPlan {
    /// 窓と資産の双方が揃った scope の装着項目（`window_scopes` 出現順）。
    pub items: Vec<PlannedAttach>,
    /// 窓はあるが対応資産が無い scope（`usize` のまま・呼び手が `warn!`＋skip 縮退）。
    pub missing_assets: Vec<usize>,
    /// 資産はあるが窓が無い scope（`u32`・呼び手が `debug!`＋破棄）。
    pub unused_assets: Vec<u32>,
}

/// 1 scope 分の装着計画項目（attach フェーズ＝task 4.1 が消費）。
///
/// scope の shell／balloon 表示対象（`target_map` の正本・DD-3 の `2*scope`／`2*scope+1` 採番）と
/// 初期表示 surface id（DD-9・task 2.6 が [`super::assets::ScopeAssets`] へ焼き込み済み）を運ぶ。
/// `static_binds` は [`BootAssets`] 単一共有ゆえ本項目には複製しない（attach フェーズが
/// `assets.static_binds` を直接読む）。
///
/// `shell_index`／`balloon_index` は attach フェーズが非 Clone な `EmoWorld` を
/// `assets.shells`／`assets.balloons` から添字で move 消費するための添字（DD-12 の突き合わせで
/// 自然に確定する）。当該 scope に balloon 資産が無い場合 `balloon_index` は `None`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedAttach {
    /// 対応 scope 番号（`usize`→`u32` 変換済み）。
    pub scope: u32,
    /// シェル表示対象（`shell_target(scope)`＝偶数・DD-3）。
    pub shell_target: TargetId,
    /// バルーン表示対象（`balloon_target(scope)`＝奇数・DD-3）。
    pub balloon_target: TargetId,
    /// 初期表示 surface id（`ScopeAssets.initial_surface_id`・DD-9）。
    pub initial_surface_id: u32,
    /// `assets.shells` 内の対応添字（attach フェーズの add 消費用）。
    pub shell_index: usize,
    /// `assets.balloons` 内の対応添字（同 scope・無ければ `None`）。
    pub balloon_index: Option<usize>,
}

/// 窓一覧と構築資産を突き合わせ装着計画を返す純関数（DD-12・GPU 不要の決定論単体テスト対象）。
///
/// **`GhostWindows::scopes()`（`window_scopes`・`usize`・正）が唯一の正**（DD-12）。窓一覧を
/// 走査し、各窓 scope を `u32` へ変換（`u32::try_from`）してから `assets.shells` を `scope`
/// フィールドで照合する:
/// - 一致 → `items` へ [`PlannedAttach`] を積む（`window_scopes` の出現順を保つ）。
/// - 不一致、または `u32` に収まらない `usize`（`u32::MAX` 超過＝如何なる資産 scope とも一致し得ない）
///   → 当該窓 scope（`usize`）を `missing_assets` へ。
///
/// 加えて、`assets.shells` の各 scope のうち `window_scopes` に現れないもの（資産あり窓なし）を
/// `unused_assets`（`u32`）へ集める（`u32`→`usize` は無損失ゆえ `as` で比較）。
///
/// # 純粋性
/// 状態・I/O・GPU なし。戻り値は入力順に決定論的（`window_scopes`／`assets.shells` の順序を保つ）。
/// `window_scopes` は `GhostWindows` 契約により一意昇順が期待されるが、本関数は分類の正しさを
/// その前提に依存しない（重複窓は重複項目として忠実に写す）。
///
/// # scope 整合（DD-12）
/// 窓あり資産なしは `warn!`＋skip 縮退、資産あり窓なしは `debug!`＋破棄を呼び手（attach フェーズ）が
/// 行う。呼び手は `items.len()` が期待窓数と一致することを積極 assert し、縮退が scope 導出バグを
/// 隠さないことを担保する（spine S1）。
pub fn plan_attachments(window_scopes: &[usize], assets: &BootAssets) -> AttachPlan {
    let mut items = Vec::new();
    let mut missing_assets = Vec::new();

    // GhostWindows::scopes()（正）を走査し、usize→u32 の吸収をここで一度だけ行う（DD-12）。
    for &window_scope in window_scopes {
        // u32 に収まらない usize は如何なる資産 scope（u32）とも一致し得ない → missing 分類。
        let Ok(scope) = u32::try_from(window_scope) else {
            missing_assets.push(window_scope);
            continue;
        };
        // 資産存在の正は shells の scope フィールド。一致した添字を attach フェーズへ運ぶ。
        match assets.shells.iter().position(|s| s.scope == scope) {
            Some(shell_index) => {
                // balloon 資産は同 scope で引く（build_boot_assets は shell と同 scope 集合で組むが、
                // 万一不揃いなら None として運び、attach フェーズが文字層接続を縮退できるようにする）。
                let balloon_index = assets.balloons.iter().position(|b| b.0 == scope);
                items.push(PlannedAttach {
                    scope,
                    shell_target: shell_target(scope),
                    balloon_target: balloon_target(scope),
                    initial_surface_id: assets.shells[shell_index].initial_surface_id,
                    shell_index,
                    balloon_index,
                });
            }
            // 窓はあるが対応資産が無い → warn!＋skip 縮退の対象（呼び手が観測）。
            None => missing_assets.push(window_scope),
        }
    }

    // 資産あり窓なし: shells の scope で window_scopes に不在のものを集める。
    // u32→usize は無損失（対象プラットフォームの usize は 32bit 以上）ゆえ as で比較する。
    let mut unused_assets = Vec::new();
    for shell in &assets.shells {
        if !window_scopes.contains(&(shell.scope as usize)) {
            unused_assets.push(shell.scope);
        }
    }

    AttachPlan {
        items,
        missing_assets,
        unused_assets,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use areka_emo_atlas::AtlasTable;
    use areka_emo_compose::{BindSet, EmoWorld};
    use areka_seriko::SurfaceResolver;

    use super::*;
    use crate::emo2_boot::assets::{BootAssets, ScopeAssets};

    /// throwaway な `EmoWorld`（空 shell から build・`plan_attachments` は emo_world を読まない）。
    ///
    /// `EmoWorld::build` は空 `Shell` に対し entity ゼロの空 World を返す寛容契約（COM/GPU 不要）。
    fn empty_world() -> EmoWorld {
        EmoWorld::build(&areka_parsers::shell::parse(""))
    }

    /// 空アトラス（headless 構築・`plan_attachments` は atlas を読まない）。
    fn empty_atlas() -> AtlasTable {
        AtlasTable::new(Vec::new(), Vec::new(), Vec::new())
    }

    /// 合成 `BootAssets`（shell scope 集合と同一の balloon scope 集合を持つ標準形）。
    ///
    /// `plan_attachments` が実際に読むのは `shells[*].scope`／`shells[*].initial_surface_id`／
    /// `balloons[*].0` のみ。残りのフィールド（emo_world／atlas／balloon_model／resolver／
    /// static_binds）は最小の headless 値で埋める（COM/GPU/fixture 不要の純合成）。
    fn synth_assets(shells: &[(u32, u32)]) -> BootAssets {
        let balloon_scopes: Vec<u32> = shells.iter().map(|&(scope, _)| scope).collect();
        synth_assets_with_balloons(shells, &balloon_scopes)
    }

    /// balloon scope 集合を shell と独立に指定できる版（`balloon_index` の `None` 経路検証用）。
    fn synth_assets_with_balloons(shells: &[(u32, u32)], balloon_scopes: &[u32]) -> BootAssets {
        BootAssets {
            shells: shells
                .iter()
                .map(|&(scope, initial_surface_id)| ScopeAssets {
                    scope,
                    emo_world: empty_world(),
                    atlas: empty_atlas(),
                    initial_surface_id,
                })
                .collect(),
            balloons: balloon_scopes
                .iter()
                .map(|&scope| (scope, empty_world(), empty_atlas()))
                .collect(),
            balloon_model: areka_parsers::balloon::parse_str("", None),
            resolver: SurfaceResolver::new(BTreeMap::new()),
            static_binds: BindSet::default(),
        }
    }

    /// DD-12 完全一致: `window_scopes == 資産 scope` → 計画件数＝窓数・missing/unused 空。
    ///
    /// 各項目の shell/balloon target（DD-3 採番）と初期面（DD-9: scope0→0／scope1→10）・添字も検証する。
    #[test]
    fn plan_attachments_exact_match_plans_all_windows() {
        // 資産 scope [0,1]（DD-9: scope0→初期 surface 0／scope1→初期 surface 10）。
        let assets = synth_assets(&[(0, 0), (1, 10)]);
        let window_scopes = [0usize, 1];

        let plan = plan_attachments(&window_scopes, &assets);

        // 計画件数＝窓数（DD-12 の積極 assert・完全一致の核）。
        assert_eq!(
            plan.items.len(),
            window_scopes.len(),
            "完全一致では計画件数＝窓数"
        );
        assert!(plan.missing_assets.is_empty(), "窓あり資産なしは無い");
        assert!(plan.unused_assets.is_empty(), "資産あり窓なしは無い");

        // scope0 の項目: shell=TargetId(0)・balloon=TargetId(1)・初期面 0（DD-9）。
        assert_eq!(plan.items[0].scope, 0);
        assert_eq!(plan.items[0].shell_target, shell_target(0));
        assert_eq!(plan.items[0].balloon_target, balloon_target(0));
        assert_eq!(plan.items[0].initial_surface_id, 0, "scope0 初期面 0（DD-9）");
        assert_eq!(plan.items[0].shell_index, 0);
        assert_eq!(plan.items[0].balloon_index, Some(0));

        // scope1 の項目: shell=TargetId(2)・balloon=TargetId(3)・初期面 10（DD-9）。
        assert_eq!(plan.items[1].scope, 1);
        assert_eq!(plan.items[1].shell_target, shell_target(1));
        assert_eq!(plan.items[1].balloon_target, balloon_target(1));
        assert_eq!(plan.items[1].initial_surface_id, 10, "scope1 初期面 10（DD-9）");
        assert_eq!(plan.items[1].shell_index, 1);
        assert_eq!(plan.items[1].balloon_index, Some(1));
    }

    /// DD-12 窓あり資産なし: 窓 scope に対応資産が無ければ `missing_assets` へ（`items` には載らない）。
    #[test]
    fn plan_attachments_window_without_asset_goes_to_missing() {
        // 資産 [0,1]・窓 [0,1,2] → 窓 scope 2 は資産なし。
        let assets = synth_assets(&[(0, 0), (1, 10)]);
        let window_scopes = [0usize, 1, 2];

        let plan = plan_attachments(&window_scopes, &assets);

        assert_eq!(plan.missing_assets, vec![2usize], "窓あり資産なしは missing 検出");
        assert_eq!(plan.items.len(), 2, "資産のある 0,1 のみ装着計画に載る");
        assert!(
            plan.items.iter().all(|it| it.scope != 2),
            "scope2 は items に載らない"
        );
        assert!(plan.unused_assets.is_empty(), "全資産に窓がある");
    }

    /// DD-12 資産あり窓なし: 窓を持たない資産 scope は `unused_assets`（`u32`）へ（`items` には載らない）。
    #[test]
    fn plan_attachments_asset_without_window_goes_to_unused() {
        // 資産 [0,1]・窓 [0] のみ → 資産 scope 1 は窓なし。
        let assets = synth_assets(&[(0, 0), (1, 10)]);
        let window_scopes = [0usize];

        let plan = plan_attachments(&window_scopes, &assets);

        assert_eq!(plan.unused_assets, vec![1u32], "資産あり窓なしは unused 検出（u32）");
        assert_eq!(plan.items.len(), 1, "窓のある scope0 のみ装着計画に載る");
        assert_eq!(plan.items[0].scope, 0);
        assert!(plan.missing_assets.is_empty(), "全窓に資産がある");
    }

    /// DD-12 `usize`→`u32` 変換境界: 小 usize scope は u32 資産 scope と一致・`u32::MAX` 超過は missing。
    #[test]
    fn plan_attachments_usize_to_u32_conversion_boundary() {
        let assets = synth_assets(&[(0, 0), (1, 10)]);

        // 小さい usize scope (0,1) は u32 資産 scope と正しく一致する。
        let plan = plan_attachments(&[0usize, 1], &assets);
        assert_eq!(plan.items.len(), 2, "小 usize scope は u32 資産と一致");
        assert_eq!(plan.items[0].scope, 0);
        assert_eq!(plan.items[1].scope, 1);

        // usize > u32::MAX は如何なる u32 資産 scope とも一致し得ず missing 分類（64bit 環境でのみ表現可能）。
        #[cfg(target_pointer_width = "64")]
        {
            let overflow = (u32::MAX as usize) + 1; // u32 に収まらない境界超過値
            let plan = plan_attachments(&[0usize, overflow], &assets);
            assert_eq!(plan.items.len(), 1, "overflow scope は装着計画に載らない");
            assert_eq!(plan.items[0].scope, 0, "収まる 0 のみ計画に載る");
            assert_eq!(
                plan.missing_assets,
                vec![overflow],
                "u32 超過 usize は missing 分類（変換不能）"
            );
        }
    }

    /// `balloon_index` の `None` 経路: shell 資産はあるが同 scope の balloon 資産が無い場合。
    #[test]
    fn plan_attachments_marks_missing_balloon_index_none() {
        // shell scope [0,1]・balloon scope [0] のみ → scope1 は shell だけで balloon なし。
        let assets = synth_assets_with_balloons(&[(0, 0), (1, 10)], &[0]);
        let plan = plan_attachments(&[0usize, 1], &assets);

        assert_eq!(plan.items.len(), 2, "shell 資産の揃う 0,1 が計画に載る");
        assert_eq!(plan.items[0].balloon_index, Some(0), "scope0 は balloon あり");
        assert_eq!(plan.items[1].balloon_index, None, "scope1 は balloon なし → None");
    }
}
