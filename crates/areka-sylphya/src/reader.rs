//! `SylphyaReader`: `resolve_flat` / `resolve_dotted` / `talk_snapshot`（同期・無待機）。
//!
//! 掲示板（マテリアライズド・ビュー）モデルの読み側。共有読みハンドル
//! [`crate::mirror::SharedMirror`] の現在鏡像を都度 `Arc` clone し、その不変像に対して
//! map 参照だけで解決する。読み口 2 形（per-talk 凍結スナップショット／逐次同期解決）を、
//! 問い合わせ元コンテキスト（[`AskerContext`]）第一級・由来非露出（R2.4）・無待機で提供する
//! （design §「鏡像＋SylphyaReader」Service Interface）。
//!
//! ## 無待機の読み不変（R2.7 / R8.3・観測条件）
//!
//! [`SylphyaReader::resolve_flat`] / [`SylphyaReader::resolve_dotted`] /
//! [`SylphyaReader::resolve_dotted_str`] / [`SylphyaReader::talk_snapshot`] が行うのは
//! **鏡像の read lock 取得 → `Arc` clone → 不変像への map 参照** のみである。
//! チャネルの送受信（send/recv）・ファイル IO・OS 呼出・他アクター/他スレッドへの
//! ブロッキング照会は一切行わない（`SharedMirror::load` 自体がその不変を持つ）。
//! ゆえに読み経路はシステム全体の直列化点にならない。

use crate::asker::AskerContext;
use crate::key::{parse_dotted, PropPath};
use crate::mirror::SharedMirror;
use crate::value::{DottedResolution, FlatResolution};
use crate::vocab::flat::FLAT_VOCAB;
use crate::vocab::DegradePolicy;
use std::collections::BTreeMap;

/// 共有読みハンドル: 鏡像（[`SharedMirror`]）を包み、同期・無待機の読み口 2 形を提供する。
///
/// `Clone` は内部 [`SharedMirror`]（`Arc` 共有）の clone のみ——複数の読者・消費エンジンが
/// 同一鏡像を安価に共有する。構築は [`SylphyaReader::new`]（Task 5.x のアクター結線が
/// 鏡像ハンドルを渡して生成する）。
#[derive(Clone)]
pub struct SylphyaReader {
    shared: SharedMirror,
}

impl SylphyaReader {
    /// 共有鏡像ハンドルから読み口を構築する。
    pub fn new(shared: SharedMirror) -> Self {
        SylphyaReader { shared }
    }

    /// フラット窓の逐次解決（`%` 抜きの名を与える・R3.1）。
    ///
    /// 解決順は **per-asker → global**: 当該 asker の per-asker 区画→大域区画の順に
    /// 名前を引き、値が実在すれば [`FlatResolution::Value`]。不在なら台帳
    /// （[`FLAT_VOCAB`]）の縮退政策を返す（[`FlatResolution::Degraded`]）。台帳外の名は
    /// `Degraded(PassThroughRaw)`（design「台帳外の名は Degraded(PassThroughRaw)」）。
    ///
    /// 由来非露出（R2.4）: 返す型に backing 層情報は載らない。無待機（R2.7）。
    pub fn resolve_flat(&self, asker: &AskerContext, name: &str) -> FlatResolution {
        let img = self.shared.load();
        // per-asker → global の順で値を引く。
        let hit = img
            .flat_per_asker
            .get(&asker.asker)
            .and_then(|m| m.get(name))
            .or_else(|| img.flat_global.get(name));
        if let Some(v) = hit {
            return FlatResolution::Value(v.clone());
        }
        // 不在 → 台帳の縮退政策（台帳外は PassThroughRaw）。
        let policy = flat_degrade_policy(name);
        tracing::debug!(
            target: "areka_sylphya::reader",
            asker = asker.asker.as_str(),
            name = name,
            policy = ?policy,
            "flat resolution degraded"
        );
        FlatResolution::Degraded(policy)
    }

    /// 点付き窓の逐次解決（パース済み [`PropPath`]・R3.2 / R5.2）。
    ///
    /// `PropPath` を正準文字列形（[`PropPath::to_canonical_string`]）へ正規化し、
    /// per-asker → global の順で点付き区画を引く。値が実在すれば
    /// [`DottedResolution::Value`]、不在なら [`DottedResolution::NotFound`]（決定論）。
    ///
    /// 無待機（R2.7）・由来非露出（R2.4）。
    pub fn resolve_dotted(&self, asker: &AskerContext, path: &PropPath) -> DottedResolution {
        let key = path.to_canonical_string();
        self.resolve_dotted_canonical(asker, &key)
    }

    /// 文字列 key の便宜口（sink 用・R3.2）。
    ///
    /// [`parse_dotted`] に失敗した key は `tracing::warn!` を記録して
    /// [`DottedResolution::NotFound`] へ決定論的に縮退する（panic しない）。成功時は
    /// [`SylphyaReader::resolve_dotted`] へ委譲する。
    pub fn resolve_dotted_str(&self, asker: &AskerContext, key: &str) -> DottedResolution {
        match parse_dotted(key) {
            Ok(path) => self.resolve_dotted(asker, &path),
            Err(e) => {
                tracing::warn!(
                    target: "areka_sylphya::reader",
                    asker = asker.asker.as_str(),
                    key = key,
                    error = %e,
                    "dotted key parse failed; degrading to NotFound"
                );
                DottedResolution::NotFound
            }
        }
    }

    /// talk 凍結スナップショット素材（R2.1）。
    ///
    /// 鏡像に **値が実在するフラット名のみ** を `BTreeMap<String, String>` として返す。
    /// per-asker → global のマージ（per-asker が global を隠す）で、縮退・不在の名や
    /// 既定値は含めない（値実在分のみ）。戻り値は所有コピー——取得後に新しい鏡像が
    /// publish されても、返した写像は不変（per-talk 凍結）。
    ///
    /// 無待機（R2.7）。ghost がこれを `SystemVarSnapshot` へ写像する（sylphya は sakura 非依存）。
    pub fn talk_snapshot(&self, asker: &AskerContext) -> BTreeMap<String, String> {
        let img = self.shared.load();
        // global を土台に per-asker を上書きマージ（per-asker → global 優先順に対応）。
        let mut out: BTreeMap<String, String> = img.flat_global.clone();
        if let Some(per) = img.flat_per_asker.get(&asker.asker) {
            for (k, v) in per {
                out.insert(k.clone(), v.clone());
            }
        }
        out
    }

    /// 正準文字列 key で点付き区画を引く（per-asker → global）。内部共通経路。
    fn resolve_dotted_canonical(&self, asker: &AskerContext, key: &str) -> DottedResolution {
        let img = self.shared.load();
        let hit = img
            .dotted_per_asker
            .get(&asker.asker)
            .and_then(|m| m.get(key))
            .or_else(|| img.dotted_global.get(key));
        match hit {
            Some(v) => DottedResolution::Value(v.clone()),
            None => {
                tracing::debug!(
                    target: "areka_sylphya::reader",
                    asker = asker.asker.as_str(),
                    key = key,
                    "dotted resolution not found"
                );
                DottedResolution::NotFound
            }
        }
    }
}

/// フラット名の縮退政策を台帳から引く。台帳外の名は `PassThroughRaw`
/// （design「台帳外の名は Degraded(PassThroughRaw)」）。
fn flat_degrade_policy(name: &str) -> DegradePolicy {
    FLAT_VOCAB
        .iter()
        .find(|e| e.token == name)
        .map(|e| e.degrade)
        .unwrap_or(DegradePolicy::PassThroughRaw)
}

#[cfg(test)]
mod tests {
    use crate::asker::{AskerContext, AskerId};
    use crate::key::parse_dotted;
    use crate::mirror::{MirrorImage, SharedMirror};
    use crate::reader::SylphyaReader;
    use crate::value::{DottedResolution, FlatResolution};
    use crate::vocab::DegradePolicy;
    use std::sync::Arc;

    fn ctx(id: &str) -> AskerContext {
        AskerContext { asker: AskerId::new(id) }
    }

    #[test]
    fn flat_value_present_per_asker() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.flat_per_asker
            .entry(a.clone())
            .or_default()
            .insert("selfname".into(), "Alice".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/a"), "selfname"),
            FlatResolution::Value("Alice".into())
        );
    }

    #[test]
    fn flat_absent_username_is_consumer_default() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/a"), "username"),
            FlatResolution::Degraded(DegradePolicy::ConsumerDefault)
        );
    }

    #[test]
    fn flat_absent_passthrough_token_is_passthrough() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        // selfname is a table entry with PassThroughRaw
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/a"), "selfname"),
            FlatResolution::Degraded(DegradePolicy::PassThroughRaw)
        );
    }

    #[test]
    fn flat_absent_table_outside_is_passthrough() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/a"), "not_a_real_token"),
            FlatResolution::Degraded(DegradePolicy::PassThroughRaw)
        );
    }

    #[test]
    fn flat_per_asker_shadows_global() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.flat_global.insert("selfname".into(), "GLOBAL".into());
        img.flat_per_asker
            .entry(a.clone())
            .or_default()
            .insert("selfname".into(), "PERASKER".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/a"), "selfname"),
            FlatResolution::Value("PERASKER".into())
        );
    }

    #[test]
    fn flat_global_found_when_per_asker_lacks() {
        let mut img = MirrorImage::empty();
        img.flat_global.insert("selfname".into(), "GLOBAL".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/a"), "selfname"),
            FlatResolution::Value("GLOBAL".into())
        );
    }

    #[test]
    fn flat_other_asker_does_not_leak() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.flat_per_asker
            .entry(a.clone())
            .or_default()
            .insert("selfname".into(), "Alice".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        // different asker sees no value -> degrade
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/b"), "selfname"),
            FlatResolution::Degraded(DegradePolicy::PassThroughRaw)
        );
    }

    #[test]
    fn dotted_value_present_per_asker() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.dotted_per_asker
            .entry(a.clone())
            .or_default()
            .insert("areka.window.scope(0).x".into(), "10".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        let path = parse_dotted("areka.window.scope(0).x").unwrap();
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/a"), &path),
            DottedResolution::Value("10".into())
        );
    }

    #[test]
    fn dotted_value_present_global() {
        let mut img = MirrorImage::empty();
        img.dotted_global
            .insert("baseware.name".into(), "areka".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        let path = parse_dotted("baseware.name").unwrap();
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/a"), &path),
            DottedResolution::Value("areka".into())
        );
    }

    #[test]
    fn dotted_per_asker_precedes_global() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.dotted_global.insert("x.y".into(), "GLOBAL".into());
        img.dotted_per_asker
            .entry(a.clone())
            .or_default()
            .insert("x.y".into(), "PERASKER".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        let path = parse_dotted("x.y").unwrap();
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/a"), &path),
            DottedResolution::Value("PERASKER".into())
        );
    }

    #[test]
    fn dotted_absent_is_not_found() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        let path = parse_dotted("system.no_such").unwrap();
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/a"), &path),
            DottedResolution::NotFound
        );
    }

    #[test]
    fn dotted_str_present() {
        let mut img = MirrorImage::empty();
        img.dotted_global
            .insert("baseware.version".into(), "0.1".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        assert_eq!(
            reader.resolve_dotted_str(&ctx("ghost/a"), "baseware.version"),
            DottedResolution::Value("0.1".into())
        );
    }

    #[test]
    fn dotted_str_malformed_is_not_found_no_panic() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        // "a..b" -> EmptySegment parse error -> NotFound (deterministic, no panic)
        assert_eq!(
            reader.resolve_dotted_str(&ctx("ghost/a"), "a..b"),
            DottedResolution::NotFound
        );
        assert_eq!(
            reader.resolve_dotted_str(&ctx("ghost/a"), ""),
            DottedResolution::NotFound
        );
    }

    #[test]
    fn dotted_str_roundtrips_with_canonical_stored_key() {
        // key stored canonically by publisher/persist; resolve_dotted_str must find it
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.dotted_per_asker
            .entry(a.clone())
            .or_default()
            .insert("areka.window.scope(2).y".into(), "512".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        assert_eq!(
            reader.resolve_dotted_str(&ctx("ghost/a"), "areka.window.scope(2).y"),
            DottedResolution::Value("512".into())
        );
    }

    #[test]
    fn talk_snapshot_contains_only_real_values() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.flat_global.insert("gkey".into(), "gval".into());
        img.flat_per_asker
            .entry(a.clone())
            .or_default()
            .insert("selfname".into(), "Alice".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        let snap = reader.talk_snapshot(&ctx("ghost/a"));
        assert_eq!(snap.get("selfname").map(String::as_str), Some("Alice"));
        assert_eq!(snap.get("gkey").map(String::as_str), Some("gval"));
        // degraded/absent names are NOT present
        assert!(!snap.contains_key("username"));
        assert!(!snap.contains_key("month"));
    }

    #[test]
    fn talk_snapshot_per_asker_shadows_global() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.flat_global.insert("selfname".into(), "GLOBAL".into());
        img.flat_per_asker
            .entry(a.clone())
            .or_default()
            .insert("selfname".into(), "PERASKER".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        let snap = reader.talk_snapshot(&ctx("ghost/a"));
        assert_eq!(snap.get("selfname").map(String::as_str), Some("PERASKER"));
    }

    #[test]
    fn talk_snapshot_is_frozen_after_publish() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.flat_per_asker
            .entry(a.clone())
            .or_default()
            .insert("selfname".into(), "Alice".into());
        let shared = SharedMirror::new(Arc::new(img));
        let reader = SylphyaReader::new(shared.clone());
        let snap = reader.talk_snapshot(&ctx("ghost/a"));
        // publish a new image that changes the value
        let mut next = shared.load().successor();
        next.flat_per_asker
            .entry(a.clone())
            .or_default()
            .insert("selfname".into(), "CHANGED".into());
        shared.publish(Arc::new(next));
        // already-returned snapshot is unchanged (per-talk freeze)
        assert_eq!(snap.get("selfname").map(String::as_str), Some("Alice"));
        // but a fresh snapshot sees the new value
        let snap2 = reader.talk_snapshot(&ctx("ghost/a"));
        assert_eq!(snap2.get("selfname").map(String::as_str), Some("CHANGED"));
    }

    #[test]
    fn determinism_same_epoch_same_result() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.flat_per_asker
            .entry(a.clone())
            .or_default()
            .insert("selfname".into(), "Alice".into());
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));
        let r1 = reader.resolve_flat(&ctx("ghost/a"), "selfname");
        let r2 = reader.resolve_flat(&ctx("ghost/a"), "selfname");
        assert_eq!(r1, r2);
        let d1 = reader.resolve_dotted_str(&ctx("ghost/a"), "system.none");
        let d2 = reader.resolve_dotted_str(&ctx("ghost/a"), "system.none");
        assert_eq!(d1, d2);
        assert_eq!(d1, DottedResolution::NotFound);
    }

    #[test]
    fn reader_is_clone() {
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(MirrorImage::empty())));
        let clone = reader.clone();
        assert_eq!(
            clone.resolve_dotted_str(&ctx("g"), "a.b"),
            DottedResolution::NotFound
        );
    }
}

/// 鏡像・読み口 決定論単体テスト群（Task 3.3・consolidated criterion-mapped cage）。
///
/// design §Testing Strategy → Unit Tests 3（解決と縮退）・4（鏡像と凍結）の 4 判断分岐を
/// 決定論檻として集約する。x64 純粋（no I/O・no std::env・no clock——注入シーム不要の
/// 純関数的解決）。既存の強い檻は重複させず、監査で判明した **点付き per-asker 非混同**
/// （フラット側は `flat_other_asker_does_not_leak` 済・点付き側は本群が初出）を主眼に補う。
///
/// ## 判断分岐 → 檻 の対応（criterion → test）
///
/// - **値あり `Value`／政策別 `Degraded`／点付き `NotFound`**（R2.3・3.1・3.2）:
///   `super::tests::{flat_value_present_per_asker, flat_absent_username_is_consumer_default,
///   flat_absent_passthrough_token_is_passthrough, flat_absent_table_outside_is_passthrough,
///   dotted_absent_is_not_found}`（既存・全政策——ConsumerDefault／PassThroughRaw／NotFound——を網羅）。
/// - **per-asker 非混同（フラット＋点付き 両窓・両方向）**（R2.6）:
///   フラット片方向は `super::tests::flat_other_asker_does_not_leak`（既存）。
///   両窓・両方向の集約檻は本群 [`per_asker_non_mixing_flat_and_dotted`]（点付き per-asker
///   非混同の初出檻）。
/// - **per-talk 凍結**（R2.1）: `super::tests::talk_snapshot_is_frozen_after_publish`（既存）。
/// - **決定論（同一鏡像 epoch×同一 asker×同一名→同一結果）**（R2.5・9.1）:
///   フラット Value＋点付き NotFound は `super::tests::determinism_same_epoch_same_result`（既存）。
///   点付き **Value** 分岐は本群 [`determinism_dotted_value_present_stable`] が補う。
#[cfg(test)]
mod mirror_reader_criteria_cage {
    use crate::asker::{AskerContext, AskerId};
    use crate::key::parse_dotted;
    use crate::mirror::{MirrorImage, SharedMirror};
    use crate::reader::SylphyaReader;
    use crate::value::{DottedResolution, FlatResolution};
    use crate::vocab::DegradePolicy;
    use std::sync::Arc;

    fn ctx(id: &str) -> AskerContext {
        AskerContext { asker: AskerId::new(id) }
    }

    /// per-asker 区画が **フラット・点付きとも** asker 間で混ざらないこと（R2.6・両方向）。
    ///
    /// 2 asker（A=ghost/a・B=ghost/b）へ各々固有のフラット値（selfname／keroname）と
    /// 固有の点付き値（同名 key の別値＋各自だけが持つ key）を据える。検証:
    /// - フラット両方向: A の selfname=Alice・B の selfname=Bob が独立（相互に混ざらない）。
    /// - フラット非漏洩: A のみが持つ keroname は B からは見えず政策縮退へ落ちる。
    /// - 点付き両方向: 同一 key `areka.window.scope(0).x` が A→10・B→99 と asker ごとに独立
    ///   （点付き per-asker 非混同の核——本檻の初出主眼）。
    /// - 点付き非漏洩: A 固有 key は B から `NotFound`、B 固有 key は A から `NotFound`
    ///   （per-asker 区画を取り違え／併合すれば別 asker の値が漏れて破れる）。
    #[test]
    fn per_asker_non_mixing_flat_and_dotted() {
        let a = AskerId::new("ghost/a");
        let b = AskerId::new("ghost/b");
        let mut img = MirrorImage::empty();
        // asker A: 固有フラット（selfname・keroname）＋固有点付き（共有 key の別値＋A 専用 key）。
        {
            let pa = img.flat_per_asker.entry(a.clone()).or_default();
            pa.insert("selfname".into(), "Alice".into());
            pa.insert("keroname".into(), "KeroA".into());
        }
        {
            let da = img.dotted_per_asker.entry(a.clone()).or_default();
            da.insert("areka.window.scope(0).x".into(), "10".into());
            da.insert("areka.only_a".into(), "A1".into());
        }
        // asker B: 固有フラット（selfname のみ・keroname は持たない）＋固有点付き。
        {
            let pb = img.flat_per_asker.entry(b.clone()).or_default();
            pb.insert("selfname".into(), "Bob".into());
        }
        {
            let db = img.dotted_per_asker.entry(b.clone()).or_default();
            db.insert("areka.window.scope(0).x".into(), "99".into());
            db.insert("areka.only_b".into(), "B2".into());
        }
        let reader = SylphyaReader::new(SharedMirror::new(Arc::new(img)));

        // --- フラット: 両方向で独立 ---
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/a"), "selfname"),
            FlatResolution::Value("Alice".into())
        );
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/b"), "selfname"),
            FlatResolution::Value("Bob".into())
        );
        // A のみが持つ keroname は B へ漏れない（政策縮退＝PassThroughRaw）。
        assert_eq!(
            reader.resolve_flat(&ctx("ghost/b"), "keroname"),
            FlatResolution::Degraded(DegradePolicy::PassThroughRaw)
        );

        // --- 点付き: 同一 key が asker ごとに独立（本檻の主眼） ---
        let shared_key = parse_dotted("areka.window.scope(0).x").unwrap();
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/a"), &shared_key),
            DottedResolution::Value("10".into())
        );
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/b"), &shared_key),
            DottedResolution::Value("99".into())
        );
        // A 専用 key は B からは不在・B 専用 key は A からは不在（per-asker 非漏洩）。
        let only_a = parse_dotted("areka.only_a").unwrap();
        let only_b = parse_dotted("areka.only_b").unwrap();
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/b"), &only_a),
            DottedResolution::NotFound
        );
        assert_eq!(
            reader.resolve_dotted(&ctx("ghost/a"), &only_b),
            DottedResolution::NotFound
        );
    }

    /// 決定論（点付き **Value** 分岐）: 同一鏡像 epoch×同一 asker×同一名 → 同一結果（R2.5・9.1）。
    ///
    /// 既存 `determinism_same_epoch_same_result` はフラット Value＋点付き NotFound を檻化する。
    /// 本檻は publish を挟まず（epoch 不変）点付き Value を反復解決し、値が実在する分岐でも
    /// 結果が安定（同一 `Value`）であることを補完する。
    #[test]
    fn determinism_dotted_value_present_stable() {
        let a = AskerId::new("ghost/a");
        let mut img = MirrorImage::empty();
        img.dotted_per_asker
            .entry(a.clone())
            .or_default()
            .insert("areka.window.scope(1).y".into(), "256".into());
        let shared = SharedMirror::new(Arc::new(img));
        let reader = SylphyaReader::new(shared.clone());
        // epoch は publish を挟まないので不変（同一鏡像世代）。
        let epoch_before = shared.load().epoch;
        let path = parse_dotted("areka.window.scope(1).y").unwrap();
        let r1 = reader.resolve_dotted(&ctx("ghost/a"), &path);
        let r2 = reader.resolve_dotted(&ctx("ghost/a"), &path);
        let r3 = reader.resolve_dotted_str(&ctx("ghost/a"), "areka.window.scope(1).y");
        assert_eq!(r1, DottedResolution::Value("256".into()));
        assert_eq!(r1, r2);
        assert_eq!(r2, r3);
        assert_eq!(shared.load().epoch, epoch_before);
    }
}
