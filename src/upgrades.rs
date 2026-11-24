use super::*;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HistoryEntry {
    pub command: String,
    pub version: Option<String>,
}

impl HistoryEntry {
    pub(super) fn matches(&self, version: &str) -> bool {
        VERSIONS.contains(&version) && self.version.as_deref() == Some(version)
    }
}

pub(super) fn load_history(cfg: &Config) -> VecDeque<HistoryEntry> {
    // The legacy string array stays readable by the earlier executables.
    // Ignore misaligned metadata instead of assigning unverified versions.
    let versions =
        (cfg.history_versions.len() == cfg.history.len()).then_some(&cfg.history_versions);
    cfg.history
        .iter()
        .take(200)
        .enumerate()
        .map(|(i, command)| HistoryEntry {
            command: command.clone(),
            version: versions
                .and_then(|values| values[i].as_ref())
                .filter(|v| VERSIONS.contains(&v.as_str()))
                .cloned(),
        })
        .collect()
}

pub(super) fn history_export_lines(
    history: &VecDeque<HistoryEntry>,
    selected: &HashSet<usize>,
    version: &str,
) -> Vec<String> {
    // Replay actions in the order they were recorded; keep multiline groups intact.
    history
        .iter()
        .enumerate()
        .rev()
        .filter(|(i, entry)| selected.contains(i) && entry.matches(version))
        .flat_map(|(_, entry)| entry.command.lines())
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(super) struct QuickOptions {
    xp_levels: i32,
    reach: f64,
    heal_target: String,
}

impl Default for QuickOptions {
    fn default() -> Self {
        Self {
            xp_levels: 1_000_000_000,
            reach: 15.0,
            heal_target: "Xian_Yu12138".into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) enum Favorite {
    BuiltIn(String),
    Experience,
    Healing,
    BlockReach,
    EntityReach,
}

impl Favorite {
    pub(super) fn from_command(cmd: &str) -> Self {
        if cmd.starts_with("/experience add @p ") {
            Self::Experience
        } else if cmd.starts_with("/effect give Xian_Yu12138 minecraft:instant_health infinite 111")
        {
            Self::Healing
        } else {
            Self::BuiltIn(cmd.to_string())
        }
    }

    fn label(&self) -> &str {
        match self {
            Self::BuiltIn(cmd) => QUICK_COMMANDS
                .iter()
                .find(|(_, original)| *original == cmd)
                .map_or("未知的快捷指令", |(label, _)| label),
            Self::Experience => "增加经验等级",
            Self::Healing => "无限瞬间治疗",
            Self::BlockReach => "方块接触距离",
            Self::EntityReach => "实体接触距离",
        }
    }

    fn command(&self, options: &QuickOptions, version: &str) -> Option<String> {
        match self {
            Self::BuiltIn(cmd) => QUICK_COMMANDS
                .iter()
                .any(|(_, original)| *original == cmd)
                .then(|| cmd.clone()),
            Self::Experience => (options.xp_levels > 0)
                .then(|| format!("/experience add @p {} levels", options.xp_levels)),
            Self::Healing => {
                let target = options.heal_target.trim();
                let valid = matches!(target, "@s" | "@p" | "@a" | "@r")
                    || ((1..=16).contains(&target.len())
                        && target
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || c == b'_'));
                valid
                    .then(|| format!("/effect give {target} minecraft:instant_health infinite 111"))
            }
            reach => {
                let index = usize::from(*reach == Self::EntityReach);
                let (_, template) = quick_reach_commands(version).get(index)?;
                ((0.0..=64.0).contains(&options.reach)).then(|| {
                    format!(
                        "{} {}",
                        template.strip_suffix(" 15").unwrap_or(template),
                        options.reach
                    )
                })
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
enum Difficulty {
    Easy,
    Normal,
    Hard,
    Peaceful,
    Unchanged,
}
impl Difficulty {
    fn name(self) -> &'static str {
        match self {
            Self::Easy => "简单",
            Self::Normal => "普通",
            Self::Hard => "困难",
            Self::Peaceful => "和平",
            Self::Unchanged => "不修改",
        }
    }
    fn command(self) -> Option<&'static str> {
        match self {
            Self::Easy => Some("/difficulty easy"),
            Self::Normal => Some("/difficulty normal"),
            Self::Hard => Some("/difficulty hard"),
            Self::Peaceful => Some("/difficulty peaceful"),
            Self::Unchanged => None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(super) struct WorldPreset {
    name: String,
    keep_inventory: bool,
    difficulty: Difficulty,
    lock_day: bool,
    lock_clear: bool,
    prevent_mob_griefing: bool,
}
impl Default for WorldPreset {
    fn default() -> Self {
        Self {
            name: "新存档初始化".into(),
            keep_inventory: true,
            difficulty: Difficulty::Hard,
            lock_day: true,
            lock_clear: true,
            prevent_mob_griefing: false,
        }
    }
}
impl WorldPreset {
    fn commands(&self) -> Vec<&'static str> {
        let mut commands = Vec::new();
        if self.keep_inventory {
            commands.push("/gamerule keepInventory true");
        }
        commands.extend(self.difficulty.command());
        if self.lock_day {
            commands.extend(["/time set day", "/gamerule doDaylightCycle false"]);
        }
        if self.lock_clear {
            commands.extend(["/weather clear", "/gamerule doWeatherCycle false"]);
        }
        if self.prevent_mob_griefing {
            commands.push("/gamerule mobGriefing false");
        }
        commands
    }
}

pub(super) fn parse_config_backup(text: &str) -> Result<Config, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("JSON 格式错误：{e}"))?;
    let obj = value.as_object().ok_or("请选择本工具导出的 JSON 配置。")?;
    if !obj.keys().any(|s| {
        matches!(
            s.as_str(),
            "schema_version" | "version" | "history" | "teleports" | "custom_cmds"
        )
    }) {
        return Err("未找到本工具的配置字段。".into());
    }
    let cfg: Config = serde_json::from_value(value).map_err(|e| format!("配置字段无效：{e}"))?;
    if cfg.schema_version > CURRENT_SCHEMA {
        return Err(format!(
            "文件格式 v{} 比本程序支持的 v{CURRENT_SCHEMA} 更新，未覆盖当前配置。",
            cfg.schema_version
        ));
    }
    if !cfg.version.is_empty() && !VERSIONS.contains(&cfg.version.as_str()) {
        return Err(format!("配置包含不支持的游戏版本：{}", cfg.version));
    }
    Ok(cfg)
}

fn read_backup_file(path: &Path) -> Result<Config, String> {
    const MAX: u64 = 16 * 1024 * 1024;
    if !fs::metadata(path)
        .map_err(|e| format!("无法读取文件：{e}"))?
        .is_file()
    {
        return Err("请输入 JSON 文件路径，而不是目录。".into());
    }
    let mut text = String::new();
    fs::File::open(path)
        .and_then(|f| f.take(MAX + 1).read_to_string(&mut text))
        .map_err(|e| format!("读取失败：{e}"))?;
    if text.len() as u64 > MAX {
        return Err("JSON 备份不能超过 16 MiB。".into());
    }
    parse_config_backup(&text)
}

fn write_backup_file(path: &Path, text: &str) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("无法创建新文件（不会覆盖现有文件）：{e}"))?;
    if let Err(err) = file
        .write_all(text.as_bytes())
        .and_then(|_| file.sync_all())
    {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(format!("导出失败：{err}"));
    }
    Ok(())
}

impl App {
    pub(super) fn output_is_stale(&self) -> bool {
        self.result_versions
            .get(&self.page)
            .is_some_and(|version| version != &self.version)
    }

    pub(super) fn mark_result(&mut self, page: Page) {
        self.result_versions.insert(page, self.version.clone());
    }

    pub(super) fn clear_generated_results(&mut self) {
        self.result_versions.clear();
        for result in [
            &mut self.inf_result,
            &mut self.ench_result,
            &mut self.hand_result,
            &mut self.name_result,
            &mut self.pot_result,
            &mut self.egg_result,
            &mut self.attr_result,
            &mut self.head_result,
            &mut self.gear_result,
            &mut self.batch_result,
            &mut self.ent_result,
            &mut self.as_result,
        ] {
            result.clear();
        }
        for status in [
            &mut self.inf_status,
            &mut self.ench_status,
            &mut self.hand_status,
            &mut self.copy_status,
        ] {
            status.clear();
        }
    }

    pub(super) fn apply_lookup_item(&mut self, page: Page, id: &str) {
        match page {
            Page::Infinity => {
                self.inf_entry = id.into();
                self.inf_type = "item".into();
                self.inf_result.clear();
                self.inf_status.clear();
            }
            Page::Batch => {
                self.batch_entry = id.into();
                self.batch_result.clear();
            }
            Page::Name => {
                self.name_entry = id.into();
                self.name_result.clear();
            }
            Page::Enchant => {
                self.ench_entry = id.into();
                self.ench_result.clear();
                self.ench_status.clear();
            }
            _ => return,
        }
        self.result_versions.remove(&page);
        self.navigate(page);
    }

    pub(super) fn report_export(&mut self, path: Option<String>) {
        self.copy_status_ok = path.is_some();
        self.copy_status = path.map_or_else(
            || "导出失败：无法写入程序目录或没有有效指令".into(),
            |p| format!("已导出：{p}"),
        );
    }

    pub(super) fn write_safety_snapshot(&self) -> Result<(), String> {
        let mut snap = self.config_snapshot();
        snap.saved_at = now_utc_stamp();
        let text = serde_json::to_string(&snap).map_err(|e| e.to_string())?;
        save_deleted_snapshot(&text).map_err(|e| format!("无法保存恢复快照，未执行操作：{e}"))
    }

    pub(super) fn replace_config(
        &mut self,
        ctx: &egui::Context,
        mut cfg: Config,
        make_snapshot: bool,
    ) -> Result<(), String> {
        cfg.schema_version = CURRENT_SCHEMA;
        if cfg.saved_at.is_empty() {
            cfg.saved_at = now_utc_stamp();
        }
        let text = serde_json::to_string(&cfg).map_err(|e| e.to_string())?;
        if make_snapshot {
            self.write_safety_snapshot()?;
        }
        write_config_atomic(&text).map_err(|e| format!("写入配置失败，当前界面未更改：{e}"))?;
        self.apply_config(&cfg);
        apply_theme(ctx, self.palette, self.dark_mode);
        self.last_saved = serde_json::to_string(&self.config_snapshot()).unwrap_or_default();
        self.reg_cache_stale = true;
        self.startup_checks = None;
        self.write_error = None;
        Ok(())
    }

    pub(super) fn render_portable_backup(&mut self, ui: &mut egui::Ui) {
        ui::section(ui, self.palette, "JSON 配置备份", |ui| {
            ui.label(
                "备份包括设置、历史、传送点、收藏、自定义指令和世界方案。不包括三个本机备份槽。",
            );
            if ui::field(
                ui,
                "文件路径",
                &mut self.json_path,
                "留空导出到程序目录；导入请填 JSON 文件完整路径",
            )
            .changed()
            {
                self.pending_import = None;
            }
            ui.horizontal_wrapped(|ui| {
                if ui.button("导出新 JSON").clicked() {
                    let result = (|| -> Result<PathBuf, String> {
                        let raw = self.json_path.trim().trim_matches('"');
                        let path = if raw.is_empty() {
                            let dir = exe_dir().ok_or("找不到程序目录")?;
                            let stamp = now_utc_stamp().replace(':', "-");
                            dir.join(format!("mc-tool-backup-{stamp}.json"))
                        } else {
                            PathBuf::from(raw)
                        };
                        let mut cfg = self.config_snapshot();
                        cfg.saved_at = now_utc_stamp();
                        let text = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
                        write_backup_file(&path, &text)?;
                        Ok(path)
                    })();
                    self.set_config_status(result.map(|p| format!("已导出：{}", p.display())));
                }
                if ui
                    .add_enabled(
                        !self.json_path.trim().is_empty(),
                        egui::Button::new("预览导入"),
                    )
                    .clicked()
                {
                    self.pending_import = None;
                    let path = PathBuf::from(self.json_path.trim().trim_matches('"'));
                    match read_backup_file(&path) {
                        Ok(cfg) => self.pending_import = Some(cfg),
                        Err(e) => self.set_config_status(Err(e)),
                    }
                }
            });
            if let Some(cfg) = &self.pending_import {
                ui.colored_label(self.palette.status_warn, format!("将替换当前配置：{} 条历史、{} 个传送点、{} 条自定义指令。替换前会保存恢复快照。", cfg.history.len(), cfg.teleports.len(), cfg.custom_cmds.len()));
                ui.horizontal_wrapped(|ui| {
                    if ui.button("确认导入并替换").clicked() {
                        if let Some(cfg) = self.pending_import.take() {
                            let result = self
                                .replace_config(&ui.ctx().clone(), cfg, true)
                                .map(|()| "导入成功。可在此页恢复替换前的快照。".into());
                            self.set_config_status(result);
                        }
                    }
                    if ui.button("取消预览").clicked() {
                        self.pending_import = None;
                    }
                });
            }
        });
    }

    pub(super) fn set_config_status(&mut self, result: Result<String, String>) {
        self.reg_status_ok = result.is_ok();
        self.reg_status = result.unwrap_or_else(|e| e);
    }

    pub(super) fn render_quick_options(&mut self, ui: &mut egui::Ui, show_reach: bool) {
        ui.horizontal_wrapped(|ui| {
            if show_reach {
                ui.label("距离（格）");
                ui.add(
                    egui::DragValue::new(&mut self.quick_options.reach)
                        .clamp_range(0.0..=64.0)
                        .speed(0.25),
                );
            } else {
                ui.label("增加经验等级");
                ui.add(
                    egui::DragValue::new(&mut self.quick_options.xp_levels)
                        .clamp_range(1..=i32::MAX)
                        .speed(1),
                );
                ui.label("治疗对象");
                ui.add(
                    egui::TextEdit::singleline(&mut self.quick_options.heal_target)
                        .desired_width(120.0)
                        .hint_text("玩家名或 @p"),
                );
            }
            if ui.button("恢复默认参数").clicked() {
                let defaults = QuickOptions::default();
                if show_reach {
                    self.quick_options.reach = defaults.reach;
                } else {
                    self.quick_options.xp_levels = defaults.xp_levels;
                    self.quick_options.heal_target = defaults.heal_target;
                }
            }
        });
    }

    pub(super) fn render_quick_preset(&mut self, ui: &mut egui::Ui, favorite: Favorite) {
        let p = self.palette;
        ui::surface(p, p.input_bg, 10.0).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let cmd = favorite.command(&self.quick_options, &self.version);
            let pinned = self.favorites.contains(&favorite);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(cmd.is_some(), egui::Button::new("复制"))
                    .clicked()
                {
                    if let Some(command) = &cmd {
                        ui.ctx().copy_text(command.clone());
                        self.push_history(command.clone());
                        self.copy_status = format!("已复制：{}", favorite.label());
                        self.copy_status_ok = true;
                    }
                }
                ui.label(egui::RichText::new(favorite.label()).strong());
                if ui
                    .small_button(if pinned {
                        "取消收藏"
                    } else {
                        "收藏到首页"
                    })
                    .clicked()
                {
                    if pinned {
                        self.favorites.retain(|f| f != &favorite);
                    } else {
                        self.favorites.push(favorite.clone());
                    }
                }
            });
            ui.label(
                egui::RichText::new(
                    cmd.as_deref()
                        .unwrap_or("当前版本不支持此指令，或参数无效。"),
                )
                .monospace()
                .size(12.0)
                .color(p.text_weak),
            );
        });
    }

    pub(super) fn render_favorites(&mut self, ui: &mut egui::Ui) {
        if self.favorites.is_empty() {
            return;
        }
        let favorites = self.favorites.clone();
        ui::section(ui, self.palette, "收藏指令", |ui| {
            for favorite in favorites {
                self.render_quick_preset(ui, favorite);
            }
            if !self.copy_status.is_empty() {
                ui.colored_label(self.palette.status_ok, &self.copy_status);
            }
        });
    }

    pub(super) fn render_world_presets(&mut self, ui: &mut egui::Ui) {
        ui::section(ui, self.palette, "世界规则方案", |ui| {
            ui.label("用于初始化新存档。只生成和复制指令，不会直接修改世界。");
            ui.horizontal_wrapped(|ui| {
                if ui.button("载入新档默认方案").clicked() {
                    self.world_draft = WorldPreset::default();
                }
                for preset in &self.world_presets {
                    if ui.button(&preset.name).clicked() {
                        self.world_draft = preset.clone();
                    }
                }
            });
            ui::field(ui, "方案名称", &mut self.world_draft.name, "例如 生存服");
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut self.world_draft.keep_inventory, "死亡不掉落");
                ui.checkbox(&mut self.world_draft.lock_day, "切到白天并锁定");
                ui.checkbox(&mut self.world_draft.lock_clear, "切到晴天并锁定");
                ui.checkbox(
                    &mut self.world_draft.prevent_mob_griefing,
                    "禁止生物破坏地形",
                );
                egui::ComboBox::from_id_source("world_difficulty")
                    .selected_text(self.world_draft.difficulty.name())
                    .show_ui(ui, |ui| {
                        for difficulty in [
                            Difficulty::Easy,
                            Difficulty::Normal,
                            Difficulty::Hard,
                            Difficulty::Peaceful,
                            Difficulty::Unchanged,
                        ] {
                            ui.selectable_value(
                                &mut self.world_draft.difficulty,
                                difficulty,
                                difficulty.name(),
                            );
                        }
                    });
            });
            let name = self.world_draft.name.trim().to_owned();
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(!name.is_empty(), egui::Button::new("保存方案（同名更新）"))
                    .clicked()
                {
                    let mut preset = self.world_draft.clone();
                    preset.name = name.clone();
                    if let Some(existing) = self.world_presets.iter_mut().find(|p| p.name == name) {
                        *existing = preset;
                    } else {
                        self.world_presets.push(preset);
                    }
                    self.copy_status = format!("已保存方案：{name}");
                    self.copy_status_ok = true;
                }
                if ui
                    .add_enabled(
                        self.world_presets.iter().any(|p| p.name == name),
                        egui::Button::new("移除此方案"),
                    )
                    .clicked()
                {
                    self.world_presets.retain(|p| p.name != name);
                    self.copy_status = format!("已移除方案：{name}");
                    self.copy_status_ok = true;
                }
            });
            let commands = self.world_draft.commands();
            if ui
                .add_enabled(
                    !commands.is_empty(),
                    egui::Button::new("导出方案为 .mcfunction"),
                )
                .clicked()
            {
                let lines: Vec<_> = commands.iter().map(|cmd| cmd.to_string()).collect();
                let path = self.export_mcfunction("world_rules", &lines);
                self.report_export(path);
            }
            for cmd in commands {
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("复制").clicked() {
                        ui.ctx().copy_text(cmd.to_owned());
                        self.push_history(cmd.to_owned());
                        self.copy_status = "已复制世界规则指令".into();
                        self.copy_status_ok = true;
                    }
                    ui.label(egui::RichText::new(cmd).monospace());
                });
            }
            ui.label(
                egui::RichText::new("清除掉落物是单独的快捷指令，不包含在任何初始化方案内。")
                    .color(self.palette.text_weak),
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_history_does_not_inherit_current_game_version() {
        let cfg =
            parse_config_backup(r#"{"version":"1.20.1","history":["/weather clear"]}"#).unwrap();
        let history = load_history(&cfg);
        assert_eq!(history[0].command, "/weather clear");
        assert_eq!(history[0].version, None);
    }

    #[test]
    fn history_export_excludes_unknown_unselected_and_other_versions() {
        let mut app = App::new(&egui::Context::default(), Config::default(), None);
        app.push_history("/time set day\n/gamerule doDaylightCycle false".into());
        app.record_history("/say old".into(), None);
        app.record_history("/say other version".into(), Some("1.20.1".into()));
        app.push_history("/weather clear".into());
        app.push_history("/say not selected".into());
        let cfg =
            parse_config_backup(&serde_json::to_string(&app.config_snapshot()).unwrap()).unwrap();
        let history = load_history(&cfg);
        assert_eq!(
            history_export_lines(&history, &HashSet::from([1, 2, 3, 4]), "1.21.1"),
            [
                "/time set day",
                "/gamerule doDaylightCycle false",
                "/weather clear"
            ]
        );
        assert_eq!(
            history_export_lines(&history, &HashSet::from([1, 2, 3, 4]), "1.20.1"),
            ["/say other version"]
        );
    }

    #[test]
    fn version_switch_preserves_result_provenance_and_lookup_replaces_only_target_input() {
        let mut app = App::new(&egui::Context::default(), Config::default(), None);
        app.page = Page::Batch;
        app.gen_batch();
        app.version = "1.20.1".into();
        assert!(app.output_is_stale());
        assert_eq!(app.result_versions[&Page::Batch], "1.21.1");
        app.gen_batch();
        assert!(!app.output_is_stale());
        assert_eq!(app.history[0].version.as_deref(), Some("1.20.1"));
        assert_eq!(app.history[1].version.as_deref(), Some("1.21.1"));
        for page in [Page::Infinity, Page::Batch, Page::Name, Page::Enchant] {
            app.apply_lookup_item(page, "minecraft:stone");
            assert_eq!(app.page, page);
        }
        assert_eq!(app.inf_entry, "minecraft:stone");
        assert_eq!(app.batch_entry, "minecraft:stone");
        assert_eq!(app.name_entry, "minecraft:stone");
        assert_eq!(app.ench_entry, "minecraft:stone");
        assert!(app.batch_result.is_empty());
        assert!(!app.result_versions.contains_key(&Page::Batch));
    }

    #[test]
    fn favorites_keep_default_commands_and_follow_parameters_and_version() {
        let mut options = QuickOptions::default();
        for &(_, command) in QUICK_COMMANDS {
            assert_eq!(
                Favorite::from_command(command)
                    .command(&options, "1.21.1")
                    .as_deref(),
                Some(command)
            );
        }
        for version in VERSIONS {
            assert_eq!(
                Favorite::BlockReach.command(&options, version).as_deref(),
                Some(quick_reach_commands(version)[0].1)
            );
        }
        options.xp_levels = 20;
        options.reach = 7.5;
        options.heal_target = "@p".into();
        assert_eq!(
            Favorite::Experience.command(&options, "1.21.1").unwrap(),
            "/experience add @p 20 levels"
        );
        assert_eq!(
            Favorite::Healing.command(&options, "1.21.1").unwrap(),
            "/effect give @p minecraft:instant_health infinite 111"
        );
        assert!(Favorite::EntityReach
            .command(&options, "1.21.1")
            .unwrap()
            .ends_with("base set 7.5"));
        assert!(Favorite::EntityReach.command(&options, "1.20.1").is_none());
        options.heal_target = "Player extra".into();
        assert!(Favorite::Healing.command(&options, "1.21.1").is_none());
    }

    #[test]
    fn world_preset_sets_day_and_weather_before_locking_without_dropping_items() {
        let commands = WorldPreset::default().commands();
        assert_eq!(
            commands,
            [
                "/gamerule keepInventory true",
                "/difficulty hard",
                "/time set day",
                "/gamerule doDaylightCycle false",
                "/weather clear",
                "/gamerule doWeatherCycle false"
            ]
        );
        assert!(!commands.iter().any(|cmd| cmd.starts_with("/kill")));
    }

    fn isolated_name() -> String {
        format!(
            "minecraft_tool_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    }

    #[test]
    fn portable_backup_round_trips_and_fails_closed() {
        let cfg = Config {
            schema_version: CURRENT_SCHEMA,
            version: "1.21.1".into(),
            favorites: vec![Favorite::BlockReach],
            world_presets: vec![WorldPreset::default()],
            quick_options: QuickOptions {
                xp_levels: 42,
                ..QuickOptions::default()
            },
            ..Config::default()
        };
        let path = std::env::temp_dir()
            .join(isolated_name())
            .with_extension("json");
        let text = serde_json::to_string(&cfg).unwrap();
        write_backup_file(&path, &text).unwrap();
        let opened = read_backup_file(&path);
        let duplicate = write_backup_file(&path, "must not overwrite");
        let unchanged = fs::read_to_string(&path);
        fs::remove_file(&path).unwrap();
        let opened = opened.unwrap();
        assert_eq!(opened.favorites, cfg.favorites);
        assert_eq!(opened.world_presets, cfg.world_presets);
        assert_eq!(opened.quick_options, cfg.quick_options);
        assert!(duplicate.is_err());
        assert_eq!(unchanged.unwrap(), text);
        for bad in [
            "[]",
            "{}",
            "broken",
            r#"{"schema_version":999}"#,
            r#"{"version":"invalid"}"#,
        ] {
            assert!(parse_config_backup(bad).is_err(), "accepted {bad}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn defaults_snapshot_and_undo_preserve_slots_until_explicit_purge() {
        let isolated_key = format!("Software\\{}", isolated_name());
        TEST_REG_SUBKEY.with(|key| key.replace(Some(isolated_key)));
        struct Cleanup;
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = delete_reg_key();
                TEST_REG_SUBKEY.with(|key| key.replace(None));
            }
        }
        let _cleanup = Cleanup;
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, Config::default(), None);
        app.push_history("/weather rain".into());
        app.favorites.push(Favorite::Experience);
        app.run_backup_op(&ctx, "backup1", BackupOp::Save);
        let slot = read_reg_string("backup1").unwrap();
        app.run_reg_action(&ctx, RegAction::Defaults);
        assert!(app.reg_status_ok, "{}", app.reg_status);
        assert!(app.history.is_empty());
        assert_eq!(read_reg_string("backup1").unwrap(), slot);
        assert!(read_reg_string(REG_LAST_DELETED).is_ok());
        app.run_backup_op(&ctx, REG_LAST_DELETED, BackupOp::Restore);
        assert!(app.reg_status_ok, "{}", app.reg_status);
        assert_eq!(app.history[0].command, "/weather rain");
        assert_eq!(app.favorites, vec![Favorite::Experience]);
        assert_eq!(read_reg_string("backup1").unwrap(), slot);
        app.run_reg_action(&ctx, RegAction::WholeKey);
        assert!(read_reg_string("backup1").is_ok());
        app.purge_confirm = "彻底清除".into();
        app.run_reg_action(&ctx, RegAction::WholeKey);
        assert!(app.reg_status_ok, "{}", app.reg_status);
        assert!(reg_values().is_empty());
        let _ = startup_checks(&app.config_snapshot());
        assert!(
            reg_values().is_empty(),
            "read-only diagnostics recreated configuration"
        );
    }
}
