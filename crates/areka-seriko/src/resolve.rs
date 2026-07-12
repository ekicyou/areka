//! サーフェス解決モジュール（解決層・純粋・所有 alias 表）。
//!
//! `Emote{key}` の不透明文字列（数値 id／`-1` 非表示／alias・name）を [`SurfaceTarget`] へ
//! 写す純粋層。実行時 `EmoWorld` に依存せず、構築時に emo-compose から取り出した所有
//! スナップショット（`BTreeMap<String, Vec<u32>>`）のみで解決する（design 決定1・DD2）。
//! ログや状態変更などの副作用は一切持たず、失敗（[`SurfaceTarget::Unresolved`]）の
//! error ログ＋skip は呼び手（actor）の責務（要件 2.4/6.1）。

use std::collections::BTreeMap;

/// 解決結果（非表示センチネル・未解決を型で区別する・要件 2.1/2.2/3.3/2.4）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SurfaceTarget {
    /// 表示すべき surface id（数値 id または alias／name 解決・要件 2.1/2.2）。
    Show(u32),
    /// 非表示（`\s[-1]` 相当・要件 3.3）。
    Hide,
    /// 解決不能（未知 alias・範囲外数値など。呼び手が error ログ＋skip・要件 2.4/6.1）。
    Unresolved,
}

/// surface 引数の解釈（解決層）。
///
/// emo-compose の alias 解決表を所有スナップショットとして保持し、`Emote{key}` を
/// 決定論的に [`SurfaceTarget`] へ写す。同一入力に対し常に同一出力（要件 7・純粋）。
pub struct SurfaceResolver {
    /// alias／name → surface id 群（emo-compose の `alias_snapshot()` 由来・正本二重定義しない）。
    aliases: BTreeMap<String, Vec<u32>>,
}

impl SurfaceResolver {
    /// emo-compose スナップショット（所有）から構築する。
    ///
    /// `aliases` は emo-compose の `EmoWorld::alias_snapshot()` 由来を想定する（正本・
    /// 二重定義しない・design 決定1）。
    pub fn new(aliases: BTreeMap<String, Vec<u32>>) -> Self {
        Self { aliases }
    }

    /// `Emote{key}` の文字列を解決結果へ写す（純粋・副作用なし・ログは呼び手）。
    ///
    /// 分岐（design 決定5・DD1）:
    /// 1. `key` を `i64` として parse
    ///    - 成功かつ `== -1` → [`SurfaceTarget::Hide`]（非表示センチネル・要件 3.3）
    ///    - 成功かつ非負で `u32` に収まる → [`SurfaceTarget::Show`]（要件 2.1）
    ///    - 成功だが負の非 `-1`（`-2` 等）→ [`SurfaceTarget::Unresolved`]（防御）
    ///    - 成功だが `u32` 範囲外（`4294967296` 等）→ [`SurfaceTarget::Unresolved`]（防御）
    /// 2. parse 失敗（非数値）→ 所有 alias 表を引く（要件 2.2・alias/name 同一経路 2.3）
    ///    - 見つかり非空 `Vec<u32>` → [`SurfaceTarget::Show`]`(ids[0])`（**先頭固定・決定論**・DD6）
    ///    - 見つからない（または空 Vec）→ [`SurfaceTarget::Unresolved`]（要件 2.4）
    pub fn resolve(&self, key: &str) -> SurfaceTarget {
        // 数値枝: `-1` は i64 で受けて判定し、それ以外の非負を u32 へ写す。
        if let Ok(value) = key.parse::<i64>() {
            if value == -1 {
                return SurfaceTarget::Hide;
            }
            return match u32::try_from(value) {
                // 非負かつ u32 に収まる（負の非 -1 と範囲外は Err→Unresolved）。
                Ok(id) => SurfaceTarget::Show(id),
                Err(_) => SurfaceTarget::Unresolved,
            };
        }

        // 非数値枝: alias／name を所有表から引く（複数 id は先頭固定・DD6）。
        match self.aliases.get(key) {
            Some(ids) => match ids.first() {
                Some(&id) => SurfaceTarget::Show(id),
                None => SurfaceTarget::Unresolved,
            },
            None => SurfaceTarget::Unresolved,
        }
    }
}

/// バルーン面 key 解決結果（seriko バルーン専用・シェルの [`SurfaceTarget`] とは別型ゆえ
/// 既存シェル経路に非干渉・要件 4.6）。名前形と破損数値をログ水準のために類別する。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BalloonResolve {
    /// `0..=u32::MAX` の数値 id（→ 後段 [`SurfaceTarget::Show`] で `apply_balloon`・要件 4.1/4.4）。
    Show(u32),
    /// 非表示センチネル `-1`（→ 後段 [`SurfaceTarget::Hide`] で `apply_balloon`・要件 4.2）。
    Hide,
    /// 非数値（名前形 `\b[バルーン１]`）＝M-boot 未対応の正当構文。
    /// alias 表は引かず（数値解決のみ・要件 4.4）、呼び手（actor）が warn!＋skip する（要件 4.5）。
    NameForm,
    /// 数値だが不正（`-2`・負の非 `-1`・`u32` 超過）＝破損入力。
    /// 呼び手（actor）が error!＋skip する（シェル経路と同水準・要件 4.5）。
    Invalid,
}

/// バルーン面 key の数値解決（M-boot: alias／名前解決なし・要件 4.4/4.5）。
///
/// 純関数（`self` 不要・alias 表を一切引かない・決定論）。既存 [`SurfaceResolver::resolve`] は
/// 非数値を alias 表で引くが、本関数は**引かず** [`BalloonResolve::NameForm`] で止める
/// （M-boot は数値のみ・名前解決は将来 additive）。
///
/// 分岐（design「seriko バルーン面契約」Service Interface・要件 4.4/4.5）:
/// 1. `key` を `i64` として parse
///    - 成功かつ `== -1` → [`BalloonResolve::Hide`]（非表示センチネル・要件 4.2）
///    - 成功かつ非負で `u32` に収まる → [`BalloonResolve::Show`]（要件 4.1）
///    - 成功だが負の非 `-1`（`-2` 等）・`u32` 範囲外（`4294967296` 等）→ [`BalloonResolve::Invalid`]（破損入力・要件 4.5）
/// 2. parse 失敗（非数値・名前形）→ [`BalloonResolve::NameForm`]（要件 4.5・alias 非適用）
pub fn resolve_balloon_key(key: &str) -> BalloonResolve {
    // 数値枝: `-1` は i64 で受けて判定し、それ以外の非負を u32 へ写す
    //（SurfaceResolver::resolve の数値枝と同型の判定・ただし帰結の型が異なる）。
    if let Ok(value) = key.parse::<i64>() {
        if value == -1 {
            return BalloonResolve::Hide;
        }
        return match u32::try_from(value) {
            // 非負かつ u32 に収まる（負の非 -1 と範囲外は Err→Invalid）。
            Ok(id) => BalloonResolve::Show(id),
            Err(_) => BalloonResolve::Invalid,
        };
    }

    // 非数値枝: alias 表は引かない（数値のみ・M-boot）。名前解決は将来 additive。
    BalloonResolve::NameForm
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    /// 手組みの小さな alias 表（emo2 実測値に一致）。
    fn hand_built() -> SurfaceResolver {
        let mut aliases: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        aliases.insert("通常".to_string(), vec![2100]);
        aliases.insert("静観".to_string(), vec![2106, 2206]);
        SurfaceResolver::new(aliases)
    }

    #[test]
    fn numeric_positive_is_show() {
        let r = hand_built();
        assert_eq!(r.resolve("2100"), SurfaceTarget::Show(2100));
    }

    #[test]
    fn numeric_zero_is_show() {
        let r = hand_built();
        assert_eq!(r.resolve("0"), SurfaceTarget::Show(0));
    }

    #[test]
    fn minus_one_is_hide() {
        let r = hand_built();
        assert_eq!(r.resolve("-1"), SurfaceTarget::Hide);
    }

    #[test]
    fn negative_non_minus_one_is_unresolved() {
        let r = hand_built();
        assert_eq!(r.resolve("-2"), SurfaceTarget::Unresolved);
    }

    #[test]
    fn out_of_u32_range_is_unresolved() {
        let r = hand_built();
        // u32::MAX + 1 は i64 で parse 成功するが u32 に収まらない。
        assert_eq!(r.resolve("4294967296"), SurfaceTarget::Unresolved);
    }

    #[test]
    fn u32_max_is_show() {
        let r = hand_built();
        assert_eq!(r.resolve("4294967295"), SurfaceTarget::Show(u32::MAX));
    }

    #[test]
    fn single_candidate_alias_resolves() {
        let r = hand_built();
        assert_eq!(r.resolve("通常"), SurfaceTarget::Show(2100));
    }

    #[test]
    fn multi_candidate_alias_picks_first_deterministically() {
        let r = hand_built();
        // 静観→[2106,2206]。先頭固定（DD6・ランダム選択しない）。
        assert_eq!(r.resolve("静観"), SurfaceTarget::Show(2106));
    }

    #[test]
    fn unknown_key_is_unresolved() {
        let r = hand_built();
        assert_eq!(r.resolve("知らない名前"), SurfaceTarget::Unresolved);
    }

    #[test]
    fn empty_key_is_unresolved() {
        let r = hand_built();
        assert_eq!(r.resolve(""), SurfaceTarget::Unresolved);
    }

    #[test]
    fn empty_vec_alias_is_unresolved() {
        // 空 Vec を持つ alias（防御）→ Unresolved。
        let mut aliases: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        aliases.insert("空".to_string(), Vec::new());
        let r = SurfaceResolver::new(aliases);
        assert_eq!(r.resolve("空"), SurfaceTarget::Unresolved);
    }

    #[test]
    fn resolve_is_deterministic_same_input_same_output() {
        let r = hand_built();
        assert_eq!(r.resolve("静観"), r.resolve("静観"));
        assert_eq!(r.resolve("通常"), r.resolve("通常"));
    }

    /// emo2 fixture（実上流データ）の alias スナップショットで全パターンを追験（要件 7.3）。
    ///
    /// emo-compose の `EmoWorld::alias_snapshot()`（task 1.3 で追加）由来の所有 `BTreeMap` を
    /// そのまま `SurfaceResolver::new` へ渡し、単一候補・複数候補・未知キーを確認する。
    #[test]
    fn resolves_with_real_emo2_snapshot() {
        use areka_emo_compose::EmoWorld;

        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../pilot/examples/shiori-host-32/fixtures/emo2/shell/master/surfaces.txt");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("emo2 surfaces.txt を読めること: {}: {e}", path.display()));
        let shell = areka_parsers::shell::parse(&content);
        let world = EmoWorld::build(&shell);
        let snapshot = world.alias_snapshot();

        // 非自明性: emo2 は alias を含む（空マップの vacuous pass を防ぐ）。
        assert!(!snapshot.is_empty(), "emo2 fixture は alias を含む");

        let resolver = SurfaceResolver::new(snapshot);

        // 単一候補。
        assert_eq!(resolver.resolve("通常"), SurfaceTarget::Show(2100));
        // 複数候補は先頭固定（[2106,2206] → 2106）。
        assert_eq!(resolver.resolve("静観"), SurfaceTarget::Show(2106));
        // 未知キー。
        assert_eq!(
            resolver.resolve("存在しないエイリアス"),
            SurfaceTarget::Unresolved
        );
        // 非表示センチネルは alias 表に依らず常に Hide。
        assert_eq!(resolver.resolve("-1"), SurfaceTarget::Hide);
        // 数値はそのまま id。
        assert_eq!(resolver.resolve("0"), SurfaceTarget::Show(0));
    }

    // ---- resolve_balloon_key（バルーン面 key・数値のみ・alias 非適用・純関数） ----

    #[test]
    fn balloon_numeric_positive_is_show() {
        // 数値 id → Show（→ 後段 SurfaceTarget::Show で apply_balloon・4.4）。
        assert_eq!(resolve_balloon_key("2"), BalloonResolve::Show(2));
    }

    #[test]
    fn balloon_zero_is_show() {
        // 0 も正当な数値 id（境界）。
        assert_eq!(resolve_balloon_key("0"), BalloonResolve::Show(0));
    }

    #[test]
    fn balloon_minus_one_is_hide() {
        // 非表示センチネル `-1` → Hide（4.2）。
        assert_eq!(resolve_balloon_key("-1"), BalloonResolve::Hide);
    }

    #[test]
    fn balloon_name_form_is_name_form() {
        // 名前形（非数値 `\b[バルーン１]`）は alias 表を引かず NameForm で止める
        //（M-boot は数値のみ・名前解決は将来 additive・4.5）。
        assert_eq!(resolve_balloon_key("バルーン１"), BalloonResolve::NameForm);
    }

    #[test]
    fn balloon_negative_non_minus_one_is_invalid() {
        // 負の非 `-1`（破損数値）→ Invalid（4.5）。
        assert_eq!(resolve_balloon_key("-2"), BalloonResolve::Invalid);
    }

    #[test]
    fn balloon_out_of_u32_range_is_invalid() {
        // u32::MAX + 1 は i64 で parse 成功するが u32 に収まらない → Invalid（4.5）。
        assert_eq!(resolve_balloon_key("4294967296"), BalloonResolve::Invalid);
    }

    #[test]
    fn balloon_u32_max_is_show() {
        // 上端境界 u32::MAX は Show（範囲内）。
        assert_eq!(resolve_balloon_key("4294967295"), BalloonResolve::Show(u32::MAX));
    }

    #[test]
    fn balloon_resolve_does_not_consult_alias_table() {
        // 純関数・alias 非適用の担保: SurfaceResolver では alias で引ける名前
        //（"通常"）も、resolve_balloon_key では表を引かず NameForm で止まる。
        assert_eq!(resolve_balloon_key("通常"), BalloonResolve::NameForm);
    }

    #[test]
    fn balloon_resolve_is_deterministic() {
        // 同一入力に対し常に同一出力（純関数・決定論）。
        assert_eq!(resolve_balloon_key("2"), resolve_balloon_key("2"));
        assert_eq!(resolve_balloon_key("バルーン１"), resolve_balloon_key("バルーン１"));
    }
}
