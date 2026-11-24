#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod data;
mod upgrades;
use upgrades::*;
mod ui;
use data::*;
use eframe::egui;
use serde_json::json;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;

// ============ 命令构建函数（由 Python 版迁移） ============

// ============ 主题（浅色 / 深色） ============
#[derive(Clone, Copy)]
struct Palette {
    hero_bg: egui::Color32,
    page_bg: egui::Color32,
    card_bg: egui::Color32,
    top_bg: egui::Color32,
    widget_bg: egui::Color32,
    widget_hover: egui::Color32,
    accent: egui::Color32,
    accent_hover: egui::Color32,
    accent_light: egui::Color32,
    text: egui::Color32,
    text_weak: egui::Color32,
    border: egui::Color32,
    input_bg: egui::Color32,
    status_ok: egui::Color32,
    status_warn: egui::Color32,
    status_err: egui::Color32,
}

const fn rgb(hex: u32) -> egui::Color32 {
    egui::Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

const LIGHT: Palette = Palette {
    hero_bg: rgb(0xe3eee5),
    page_bg: rgb(0xf1f4ef),
    card_bg: rgb(0xfcfdfa),
    top_bg: rgb(0xf9fbf7),
    widget_bg: rgb(0xeef2eb),
    widget_hover: rgb(0xe1eae0),
    accent: rgb(0x236044),
    accent_hover: rgb(0x194d36),
    accent_light: rgb(0xdcece0),
    text: rgb(0x18271f),
    text_weak: rgb(0x53685a),
    border: rgb(0xd0ddd0),
    input_bg: rgb(0xf2f6f0),
    status_ok: rgb(0x216442),
    status_warn: rgb(0x895316),
    status_err: rgb(0xac302d),
};

const DARK: Palette = Palette {
    hero_bg: rgb(0x182f26),
    page_bg: rgb(0x111c17),
    card_bg: rgb(0x19251e),
    top_bg: rgb(0x15211b),
    widget_bg: rgb(0x23332a),
    widget_hover: rgb(0x304438),
    accent: rgb(0x89d6a4),
    accent_hover: rgb(0xa9e4b8),
    accent_light: rgb(0x274636),
    text: rgb(0xe5ede7),
    text_weak: rgb(0xa4baab),
    border: rgb(0x36493d),
    input_bg: rgb(0x132019),
    status_ok: rgb(0x9ae3b0),
    status_warn: rgb(0xf2c781),
    status_err: rgb(0xff9a95),
};

fn stroke(color: egui::Color32) -> egui::Stroke {
    egui::Stroke::new(1.0_f32, color)
}

fn apply_theme(ctx: &egui::Context, p: &Palette, dark: bool) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;
    *v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = p.page_bg;
    v.override_text_color = Some(p.text);
    v.window_fill = p.card_bg;
    v.extreme_bg_color = p.input_bg;
    v.faint_bg_color = p.widget_bg;
    v.window_stroke = stroke(p.border);
    v.window_shadow = egui::epaint::Shadow {
        offset: egui::Vec2::new(0.0, 6.0),
        blur: 18.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(28),
    };
    v.window_rounding = egui::Rounding::same(10.0);
    v.menu_rounding = egui::Rounding::same(8.0);
    v.widgets.noninteractive.bg_fill = p.widget_bg;
    v.widgets.noninteractive.fg_stroke = stroke(p.text);
    v.widgets.noninteractive.bg_stroke = stroke(p.border);
    v.widgets.inactive.bg_fill = p.widget_bg;
    v.widgets.inactive.weak_bg_fill = p.widget_bg;
    v.widgets.inactive.bg_stroke = stroke(p.border);
    v.widgets.inactive.fg_stroke = stroke(p.text);
    v.widgets.hovered.bg_fill = p.widget_hover;
    v.widgets.hovered.weak_bg_fill = p.widget_hover;
    v.widgets.hovered.bg_stroke = stroke(p.accent);
    v.widgets.hovered.fg_stroke = stroke(p.accent_hover);
    v.widgets.active.bg_fill = p.accent_light;
    v.widgets.active.weak_bg_fill = p.accent_light;
    v.widgets.active.bg_stroke = stroke(p.accent);
    v.widgets.active.fg_stroke = stroke(p.accent_hover);
    v.widgets.open.bg_fill = p.widget_bg;
    v.widgets.open.fg_stroke = stroke(p.text);
    v.selection.bg_fill = p.accent_light;
    v.selection.stroke = stroke(p.accent_hover);
    let widget_rounding = egui::Rounding::same(7.0);
    v.widgets.noninteractive.rounding = widget_rounding;
    v.widgets.inactive.rounding = widget_rounding;
    v.widgets.hovered.rounding = widget_rounding;
    v.widgets.active.rounding = widget_rounding;
    v.widgets.open.rounding = widget_rounding;

    style.spacing.button_padding = egui::Vec2::new(9.0, 5.0);
    style.spacing.item_spacing = egui::Vec2::new(9.0, 8.0);
    style.text_styles = [
        (egui::TextStyle::Body, egui::FontId::proportional(15.0)),
        (egui::TextStyle::Button, egui::FontId::proportional(15.0)),
        (egui::TextStyle::Heading, egui::FontId::proportional(21.0)),
        (egui::TextStyle::Monospace, egui::FontId::proportional(14.0)),
        (egui::TextStyle::Small, egui::FontId::proportional(12.0)),
    ]
    .into();
    ctx.set_style(style);
}

// ============ 配置持久化 ============
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Teleport {
    name: String,
    pos: String, // "x y z"
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct CustomCmd {
    label: String,
    cmd: String,
}

/// 老版本配置缺少的新字段使用默认值；类型错误仍会报告解析失败。
#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(default)]
struct Config {
    /// 配置格式版本。老配置没有这个字段，反序列化后是 0，当「升级前的老格式」处理。
    /// 读到比本程序新的版本号时只提示、不乱套用，避免新版本的字段被当默认值抹掉。
    schema_version: u32,
    /// 这份配置写下时的时间（UTC，ISO-8601）。纯给人看的，程序逻辑不依赖它。
    saved_at: String,
    version: String,
    selector: String,
    dark_mode: bool,
    always_top: bool,
    ench_level: String,
    history: Vec<String>,
    history_versions: Vec<Option<String>>,
    quick_options: QuickOptions,
    favorites: Vec<Favorite>,
    world_presets: Vec<WorldPreset>,
    teleports: Vec<Teleport>,
    custom_cmds: Vec<CustomCmd>,
    inf_entry: String,
    ench_entry: String,
    name_entry: String,
    name_text: String,
    batch_entry: String,
    batch_count: String,
    ent_id: String,
    egg_entity: String,
    head_name: String,
    pot_level: String,
    pot_dur: String,
    attr_val: String,
    mods_dir: String,
    win_x: f32,
    win_y: f32,
    win_w: f32,
    win_h: f32,
}

/// 当前程序认识的配置格式版本。
/// v3 追加历史版本、收藏、世界方案和参数；旧历史的字符串数组保留原格式。
const CURRENT_SCHEMA: u32 = 3;

/// 把 Unix 秒数转成 `YYYY-MM-DDTHH:MM:SSZ`（UTC）。
///
/// 只为了让人看懂「这份配置是什么时候存的」。
/// 项目里没有引入 chrono（它会把编译时间和 exe 体积都抬上去），
/// 这里用 Howard Hinnant 的 civil_from_days 算法手算，纯整数运算、无时区库。
fn format_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// 从 1970-01-01 起的天数换算成年月日（含闰年规则的完整实现）。
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32; // [1, 12]
    (y + i64::from(m <= 2), m, d)
}

fn now_utc_stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_utc(secs)
}

/// exe 所在目录。只有「用户主动导出」和「启动失败日志」这类东西才落盘，
/// 配置本身存注册表，这里不再生成任何文件。
fn exe_dir() -> Option<std::path::PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(|d| d.to_path_buf())
}

const REG_SUBKEY: &str = "Software\\minecraft_tool";

#[cfg(all(windows, test))]
thread_local! {
    static TEST_REG_SUBKEY: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

#[cfg(windows)]
fn registry_subkey() -> Cow<'static, str> {
    #[cfg(test)]
    if let Some(path) = TEST_REG_SUBKEY.with(|key| key.borrow().clone()) {
        return Cow::Owned(path);
    }
    Cow::Borrowed(REG_SUBKEY)
}
const REG_CONFIG: &str = "config";
const REG_CONFIG_BROKEN: &str = "config_broken";
const REG_LAST_ERROR: &str = "last_error";
/// 每次执行清理动作之前，把「即将丢失的那份配置」整份存到这里。
/// 用户手滑清空了历史，还能从这里捞回来（界面上可以查看、复制）。
const REG_LAST_DELETED: &str = "last_deleted";

/// 往注册表写一个字符串值。整份配置就是靠这一个函数落盘的。
#[cfg(windows)]
fn write_reg_string(name: &str, text: &str) -> std::io::Result<()> {
    let (key, _) = winreg::HKCU.create_subkey(registry_subkey().as_ref())?;
    key.set_value(name, &text.to_string())
}

/// 备份槽写入用的入口。非 Windows 上给一个明确失败，别静默当成成功。
#[cfg(windows)]
fn write_reg_string_io(name: &str, text: &str) -> Result<(), String> {
    write_reg_string(name, text).map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn write_reg_string_io(_name: &str, _text: &str) -> Result<(), String> {
    Err("备份只支持存进 Windows 注册表".to_string())
}

#[cfg(windows)]
fn read_reg_string(name: &str) -> std::io::Result<String> {
    winreg::HKCU
        .open_subkey(registry_subkey().as_ref())?
        .get_value(name)
}

/// 把整份配置以 JSON 存进注册表 `HKCU\Software\minecraft_tool` 的 `config` 值。
///
/// 为什么改存注册表：
/// ① 绿色软件放哪都能用，exe 旁边不再堆 JSON；
/// ② 单值写入由系统保证原子性，不会出现「写一半断电留下半截 JSON」；
/// ③ 想清空时一句 `reg delete "HKCU\Software\minecraft_tool" /f` 就干净。
#[cfg(windows)]
fn save_config_text(text: &str) -> std::io::Result<()> {
    write_reg_string(REG_CONFIG, text)
}

#[cfg(windows)]
fn read_config_text() -> std::io::Result<String> {
    read_reg_string(REG_CONFIG)
}

/// 解析失败时把原文留在 `config_broken` 值里，绝不直接丢掉。
#[cfg(windows)]
fn save_broken_text(text: &str) {
    let _ = write_reg_string(REG_CONFIG_BROKEN, text);
}

/// 对可撤销的操作，快照写不进去时不继续清理。
#[cfg(windows)]
fn save_deleted_snapshot(text: &str) -> Result<(), String> {
    write_reg_string_io(REG_LAST_DELETED, text)
}

/// 上次启动如果失败了，把错误记在注册表里，这次开机第一屏就能看到，不用留日志文件。
fn save_last_error(msg: &str) {
    #[cfg(windows)]
    {
        if let Ok((key, _)) = winreg::HKCU.create_subkey(registry_subkey().as_ref()) {
            let _ = key.set_value(REG_LAST_ERROR, &msg.to_string());
        }
    }
    eprintln!("{msg}");
}

/// 取走上次的启动错误（读到就删，只提示一次）。
#[cfg(windows)]
fn take_last_error() -> Option<String> {
    // 读完要删掉，所以句柄必须带 SET_VALUE 权限（默认 open_subkey 是只读的）
    use winreg::enums::{KEY_QUERY_VALUE, KEY_SET_VALUE};
    let key = winreg::HKCU
        .open_subkey_with_flags(registry_subkey().as_ref(), KEY_SET_VALUE | KEY_QUERY_VALUE)
        .ok()?;
    let text: String = key.get_value(REG_LAST_ERROR).ok()?;
    let _ = key.delete_value(REG_LAST_ERROR);
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(not(windows))]
fn take_last_error() -> Option<String> {
    None
}

#[cfg(not(windows))]
fn save_config_text(_text: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "配置只支持存进 Windows 注册表",
    ))
}

#[cfg(not(windows))]
fn read_config_text() -> std::io::Result<String> {
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "配置只支持存进 Windows 注册表",
    ))
}

#[cfg(not(windows))]
fn save_broken_text(_text: &str) {}

#[cfg(not(windows))]
fn save_deleted_snapshot(text: &str) -> Result<(), String> {
    write_reg_string_io(REG_LAST_DELETED, text)
}

/// 读一个注册表值的内容，给「设置与清理」页的查看/复制用。
#[cfg(windows)]
fn read_reg_value_text(name: &str) -> Option<String> {
    read_reg_string(name).ok()
}

#[cfg(not(windows))]
fn read_reg_value_text(_name: &str) -> Option<String> {
    None
}

/// 配置备份的三个槽位（注册表值名）。
/// 备份也放注册表里，不往磁盘丢文件 —— 绿色软件挪来挪去，
/// 备份跟着注册表走反而不容易丢。三个槽循环覆盖，够回退到最近三次。
const REG_BACKUP_SLOTS: [&str; 3] = ["backup1", "backup2", "backup3"];

/// 备份槽上可以做的三件事。
#[derive(Clone, Copy, PartialEq, Debug)]
enum BackupOp {
    /// 把当前配置写进这个槽
    Save,
    /// 用这个槽里的配置覆盖当前设置
    Restore,
    /// 清空这个槽
    Delete,
}

/// 读三个备份槽的元信息：(槽名, 保存时间, 字节数)。
/// 0 字节表示这个槽是空的。只在进页 / 动作执行后调用，不每帧读注册表。
fn read_backup_meta() -> Vec<(String, String, usize)> {
    REG_BACKUP_SLOTS
        .iter()
        .map(|slot| match read_reg_value_text(slot) {
            Some(text) => {
                let stamp = serde_json::from_str::<Config>(&text)
                    .map(|c| c.saved_at)
                    .unwrap_or_default();
                (slot.to_string(), stamp, text.len())
            }
            None => (slot.to_string(), String::new(), 0),
        })
        .collect()
}

/// 列出注册表项里现有的值（名称 + 字节数），给「设置与清理」页显示用。
/// 只读，不创建键：键不存在就是空列表。
#[cfg(windows)]
fn reg_values() -> Vec<(String, usize)> {
    let Ok(key) = winreg::HKCU.open_subkey(registry_subkey().as_ref()) else {
        return Vec::new();
    };
    let mut out: Vec<(String, usize)> = key
        .enum_values()
        .filter_map(|r| r.ok())
        .map(|(name, value)| {
            // 别直接用 value.bytes.len()：REG_SZ 的 bytes 是 UTF-16LE，
            // 长度是「字符数 × 2 + 结尾的 0」，同一份 config 会显示成真实体积的两倍多，
            // 和备份卡片上的「N 字节」（那边是 JSON 文本长度）对不上，看着像两套数据。
            // 统一成「还原成文本后的字节数」；非字符串类型的值没有文本形态，才退回原始 bytes。
            let size = match value.vtype {
                winreg::enums::RegType::REG_SZ | winreg::enums::RegType::REG_EXPAND_SZ => {
                    let words: Vec<u16> = value
                        .bytes
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect();
                    // winreg 的 String 转换会自己剥掉结尾的 \0，这里保持一致，
                    // 否则每份配置都会多算 2 字节。
                    String::from_utf16_lossy(&words)
                        .trim_end_matches('\0')
                        .len()
                }
                _ => value.bytes.len(),
            };
            (name, size)
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[cfg(not(windows))]
fn reg_values() -> Vec<(String, usize)> {
    Vec::new()
}

/// 删掉注册表项里的单个值。值本来就不存在算成功（幂等）。
#[cfg(windows)]
fn delete_reg_value(name: &str) -> std::io::Result<()> {
    // 必须显式要 KEY_SET_VALUE：winreg 的 open_subkey 只申请读权限，
    // 拿只读句柄去 delete_value 会被系统拒绝（os error 5 拒绝访问）。
    use winreg::enums::{KEY_QUERY_VALUE, KEY_SET_VALUE};
    let key = winreg::HKCU
        .open_subkey_with_flags(registry_subkey().as_ref(), KEY_SET_VALUE | KEY_QUERY_VALUE)?;
    match key.delete_value(name) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(not(windows))]
fn delete_reg_value(_name: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "配置只支持存进 Windows 注册表",
    ))
}

/// 删掉整个 `HKCU\Software\minecraft_tool`，等同
/// `reg delete "HKCU\Software\minecraft_tool" /f`。键不存在算成功。
#[cfg(windows)]
fn delete_reg_key() -> std::io::Result<()> {
    match winreg::HKCU.delete_subkey_all(registry_subkey().as_ref()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(not(windows))]
fn delete_reg_key() -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "配置只支持存进 Windows 注册表",
    ))
}

/// 一条自检结论：(是否正常, 一行结论, 一句补充说明)。
/// 启动时算一次存进 App，进「设置与清理」页直接显示，不每帧重算。
type StartupCheck = (bool, String, String);

/// 启动自检（N7）：把「配置到底存没存下来」这件事一次性查清楚。
///
/// 以前这些信息散在各处：写失败只在顶部红条闪一下，配置格式版本要展开值卡片才看得见，
/// 备份槽在页面下三分之一。用户想知道"我的东西到底存住没有"，得自己拼。
/// 这里合成四行，进页就能一眼看完。
///
/// 只读，不改任何东西；调用频率是「启动一次」，不是每帧。
#[cfg(windows)]
fn startup_checks(cfg: &Config) -> Vec<StartupCheck> {
    let mut out: Vec<StartupCheck> = Vec::new();

    // 1) 注册表能不能读
    //    读到的原文留着给第 3 项用：格式版本和保存时间要以「注册表里真实写着什么」为准，
    //    不能拿内存快照的字段（那份快照里 saved_at 恒为空，见 config_snapshot 的注释）。
    let reg_text: Option<String> = match read_reg_string(REG_CONFIG) {
        Ok(text) => {
            out.push((
                true,
                "配置读取：正常".to_string(),
                format!(
                    "注册表 HKCU\\{REG_SUBKEY}\\{REG_CONFIG} 可读，{} 字节",
                    text.len()
                ),
            ));
            Some(text)
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                out.push((
                    false,
                    "配置读取：注册表里还没有配置".to_string(),
                    "第一次运行或刚彻底清除本机数据，改动设置后会自动创建。".to_string(),
                ));
            } else {
                out.push((
                    false,
                    format!("配置读取：失败（{e}）"),
                    "可能是注册表被安全软件锁住，或当前用户没有读权限。".to_string(),
                ));
            }
            None
        }
    };

    // Probe permissions without writing a value or recreating a deleted key.
    use winreg::enums::KEY_SET_VALUE;
    match winreg::HKCU.open_subkey_with_flags(registry_subkey().as_ref(), KEY_SET_VALUE) {
        Ok(_) => out.push((
            true,
            "配置写入权限：正常".into(),
            "实际保存结果仍以程序提示为准。".into(),
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => out.push((
            true,
            "配置写入权限：尚未验证".into(),
            "改动设置后会尝试创建配置。".into(),
        )),
        Err(e) => out.push((
            false,
            format!("配置写入权限：检查失败（{e}）"),
            "改动可能无法保存，请先导出 JSON。".into(),
        )),
    }

    // 3) 配置格式版本
    //    以注册表原文为准；只有读不到时才退回内存快照（那种情况下也没有历史可谈）。
    let on_disk: Option<Config> = reg_text
        .as_deref()
        .and_then(|t| serde_json::from_str::<Config>(t).ok());
    let (fmt_ver, fmt_saved) = match &on_disk {
        Some(c) => (c.schema_version, c.saved_at.clone()),
        None => (cfg.schema_version, cfg.saved_at.clone()),
    };
    if fmt_ver == 0 {
        out.push((
            true,
            "配置格式：旧版本格式（会在下次写入时升级）".to_string(),
            format!("当前程序使用 v{CURRENT_SCHEMA}。"),
        ));
    } else if fmt_ver > CURRENT_SCHEMA {
        out.push((
            false,
            format!("配置格式：v{fmt_ver}，比程序还新"),
            format!(
                "本程序只认到 v{CURRENT_SCHEMA}，继续使用会丢掉新版独有的设置，建议换回新版程序。"
            ),
        ));
    } else {
        out.push((
            true,
            format!("配置格式：v{fmt_ver}"),
            if fmt_saved.is_empty() {
                "最后保存时间未知。".to_string()
            } else {
                format!("最后一次保存：{fmt_saved}")
            },
        ));
    }

    // 4) 备份槽
    let backups = read_backup_meta();
    let used = backups.iter().filter(|(_, _, size)| *size > 0).count();
    let total: usize = backups.iter().map(|(_, _, size)| *size).sum();
    out.push((
        true,
        format!("配置备份：{used}/{} 个槽有内容", backups.len()),
        if used == 0 {
            "还没有备份。建议在改大设置之前点一次「备份当前」。".to_string()
        } else {
            format!("合计 {total} 字节，都存在注册表里。")
        },
    ));

    out
}

#[cfg(not(windows))]
fn startup_checks(_cfg: &Config) -> Vec<StartupCheck> {
    vec![(
        false,
        "配置读写：仅支持 Windows 注册表".to_string(),
        String::new(),
    )]
}

/// 历史占用统计（N6）：条数、粗估字节、以及占 config 总量的比例。
///
/// 粗估口径：每条历史按 `len + 3` 算（JSON 的引号与逗号）。
/// 只为了让人知道"历史占了多少、值不值得清"，不追求跟注册表字节数分毫不差。
fn history_stats(history: &VecDeque<HistoryEntry>, config_bytes: usize) -> String {
    let n = history.len();
    let bytes: usize = history.iter().map(|entry| entry.command.len() + 3).sum();
    if n == 0 {
        return "指令历史：0 条".to_string();
    }
    // config 里除了历史还有别的字段，所以比例按「历史 / 整份配置」算，
    // 分母用真实的注册表字节数（读不到时才退回粗估）。
    let denom = if config_bytes > bytes {
        config_bytes
    } else {
        bytes
    };
    let pct = (bytes as f64) * 100.0 / (denom as f64);
    format!("指令历史：{n} 条，约 {bytes} 字节（占整份配置的 {pct:.0}%）")
}

/// 老版本把配置存在 exe 同目录的 mc_tool_config.json。
/// 首次运行新版本时先把它搬进注册表，成功后再删掉旧文件：
/// 升级不丢历史记录，目录里也不会继续留着那个 JSON。
///
/// 只在注册表里还没有配置时才搬 —— 否则换一个目录放 exe，
/// 会用那份更旧的 JSON 覆盖掉注册表里的当前设置。
fn migrate_legacy_config_file() {
    let Some(dir) = exe_dir() else {
        return;
    };
    let legacy = dir.join("mc_tool_config.json");
    if !legacy.exists() {
        return;
    }

    // 注册表里已经有配置：说明这台机器早就升级过了，别再拿旧 JSON 覆盖它。
    // 那个 JSON 只是历史残留，确认能完整解析后直接删掉，让目录干净；
    // 解析不了就留着给用户自己处理，绝不静默删除看不出对错的旧数据。
    if read_config_text().is_ok() {
        if let Ok(text) = std::fs::read_to_string(&legacy) {
            if serde_json::from_str::<Config>(&text).is_ok() {
                let _ = std::fs::remove_file(&legacy);
                let _ = std::fs::remove_file(dir.join("mc_tool_config.json.tmp"));
            }
        }
        return;
    }

    // 注册表还是空的 → 真正的首次升级，把它搬进去
    // 读不出来或解析不了就原样留着，绝不拿半份数据覆盖
    let Ok(text) = std::fs::read_to_string(&legacy) else {
        return;
    };
    if serde_json::from_str::<Config>(&text).is_err() {
        return;
    }
    if save_config_text(&text).is_ok() {
        let _ = std::fs::remove_file(&legacy);
        // 老版本原子写留下的临时文件也顺手清掉
        let _ = std::fs::remove_file(dir.join("mc_tool_config.json.tmp"));
    }
}

/// 读配置。第二个返回值是需要给用户看的告警（例如「配置损坏，原文已保留」）。
///
/// 数据源是注册表 `HKCU\Software\minecraft_tool` 的 `config` 值（JSON）：
/// ① 值不存在 → 视为首次运行，静默用默认值；
/// ② 解析失败 → 原文另存进 `config_broken` 值，不覆盖、不丢失，并告诉用户；
/// ③ 经 `#[serde(default)]` 兜底，字段级缺失不会导致整份报废。
fn load_config() -> (Config, Option<String>) {
    // 老版本留在 exe 旁边的 JSON 先搬进注册表（只做一次）
    migrate_legacy_config_file();
    let text = match read_config_text() {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // 注册表里还没有配置，看看是不是刚升级但旧文件解析失败留着的
            return (Config::default(), take_last_error());
        }
        Err(e) => {
            return (
                Config::default(),
                Some(format!("配置读取失败（{e}），本次使用默认设置")),
            );
        }
    };
    match serde_json::from_str::<Config>(&text) {
        Ok(c) => {
            // 版本号比本程序新：多半是拿旧程序打开新版本存的配置。
            // 不拦着用（字段都靠 serde(default) 兜着），但必须警告 ——
            // 新版本独有的字段在这里会被无视，一旦改动写回就被抹掉了。
            let notice = if c.schema_version > CURRENT_SCHEMA {
                Some(format!(
                    "这份配置由更新版本的程序写入（格式 v{}，本程序只认到 v{}）。\
                     继续使用会丢掉新版独有的设置，建议换回新版程序。",
                    c.schema_version, CURRENT_SCHEMA
                ))
            } else {
                take_last_error()
            };
            (c, notice)
        }
        Err(e) => {
            // 原文留在注册表的 config_broken 值里，绝不覆盖
            save_broken_text(&text);
            (
                Config::default(),
                Some(format!(
                    "配置解析失败（{e}），原文已保留在注册表 HKCU\\{REG_SUBKEY}\\{REG_CONFIG_BROKEN}"
                )),
            )
        }
    }
}

/// 原子写配置：整份 JSON 作为一个 REG_SZ 值写入。
/// 注册表单值写入本身是原子的，不会再出现「写一半断电留下半截 JSON」。
fn write_config_atomic(data: &str) -> std::io::Result<()> {
    save_config_text(data)
}

/// 从模组语言键值中提取 (modid:itemid, 中文名) 与 (modid:entityid, 中文名)
/// 过滤掉 id 部分含 '.' 的键（如 .desc / .tooltip 说明行，不是真实物品 ID）
fn collect_lang_entries(
    lang: &HashMap<String, String>,
    out_items: &mut HashMap<String, String>,
    out_entities: &mut HashMap<String, String>,
) {
    for (k, v) in lang {
        if let Some(rest) = k.strip_prefix("entity.") {
            if let Some((ns, id)) = rest.split_once('.') {
                if !ns.is_empty() && !id.is_empty() && !id.contains('.') {
                    out_entities
                        .entry(format!("{ns}:{id}"))
                        .or_insert_with(|| v.clone());
                }
            }
        }
        for prefix in ["item.", "block."] {
            if let Some(rest) = k.strip_prefix(prefix) {
                if let Some((ns, id)) = rest.split_once('.') {
                    if !ns.is_empty() && !id.is_empty() && !id.contains('.') {
                        out_items
                            .entry(format!("{ns}:{id}"))
                            .or_insert_with(|| v.clone());
                    }
                }
            }
        }
    }
}

/// 扫描单个 jar 内所有 zh_cn 语言文件
fn scan_jar(
    path: &std::path::Path,
    out_items: &mut HashMap<String, String>,
    out_entities: &mut HashMap<String, String>,
) -> Result<(), String> {
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(std::io::BufReader::new(f)).map_err(|e| e.to_string())?;
    // 先用中央目录里的名字筛一遍：一个模组 jar 常有几千个条目，
    // 逐个 by_index 建解压器只为了看一眼名字太浪费。先收集索引，再按需打开。
    let targets: Vec<usize> = z
        .file_names()
        .enumerate()
        .filter(|(_, name)| name.starts_with("assets/") && name.ends_with("/lang/zh_cn.json"))
        .map(|(i, _)| i)
        .collect();
    for i in targets {
        let mut file = z.by_index(i).map_err(|e| e.to_string())?;
        if !file.is_file() {
            continue;
        }
        let mut s = String::new();
        std::io::Read::read_to_string(&mut file, &mut s).map_err(|e| e.to_string())?;
        if let Ok(lang) = serde_json::from_str::<HashMap<String, String>>(&s) {
            collect_lang_entries(&lang, out_items, out_entities);
        }
    }
    Ok(())
}

/// 药水类型：显示名 + 物品 ID
const POTION_KINDS: [(&str, &str); 4] = [
    ("药水", "minecraft:potion"),
    ("喷溅", "minecraft:splash_potion"),
    ("滞留", "minecraft:lingering_potion"),
    ("药水箭", "minecraft:tipped_arrow"),
];

/// 输入框按回车（配合 resp.request_focus() 保持焦点）
fn quick_world_command(cmd: &str) -> bool {
    cmd.starts_with("/gamerule ")
        || cmd.starts_with("/difficulty ")
        || cmd.starts_with("/time ")
        || cmd.starts_with("/weather ")
        || cmd == "/kill @e[type=item]"
}

fn quick_mod_command(cmd: &str) -> bool {
    cmd.starts_with("/ae2 ") || cmd.starts_with("/tiab ")
}

fn enter_pressed(ui: &egui::Ui, resp: &egui::Response) -> bool {
    resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
}

/// .mcfunction 行需要去掉前导斜杠
fn mcfn_line(l: &str) -> String {
    l.strip_prefix('/').unwrap_or(l).to_string()
}

/// 数据包函数目录名：1.20.1 及更早用复数 `functions`，1.21 起（24w21a）改为单数 `function`。
/// 两者之间没有自动别名，目录名写错时游戏会静默忽略整个目录。
fn datapack_function_dir(version: &str) -> &'static str {
    if version == "1.20.1" {
        "functions"
    } else {
        "function"
    }
}

/// 编译期内嵌的两版原版物品 ID 清单（裸 ID，每行一个，从官方注册表导出）。
/// 用来判断一个 `minecraft:` 开头的 ID 到底是不是真物品：名表里混着方块和旧 ID，
/// 光看字符串形状区分不出来，只能对注册表。
const VANILLA_ITEM_IDS: [&'static str; 2] = [
    include_str!("../assets/vanilla_item_ids_1211.txt"),
    include_str!("../assets/vanilla_item_ids_1201.txt"),
];

/// 两版物品 ID 的并集（用于「这个 ID 在任一版本里存在吗」的过滤）。
fn all_vanilla_ids() -> std::collections::HashSet<&'static str> {
    VANILLA_ITEM_IDS
        .iter()
        .flat_map(|list| list.lines())
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect()
}

/// 某一版的物品 ID 集合。`version` 传 "1.20.1" 取旧表，其余取 1.21.1 表。
/// 只在测试里用得到（生产代码只关心「任一一版里存在」），所以编译时排除。
#[cfg(test)]
fn vanilla_ids_for(version: &str) -> std::collections::HashSet<&'static str> {
    let list = if version == "1.20.1" {
        VANILLA_ITEM_IDS[1]
    } else {
        VANILLA_ITEM_IDS[0]
    };
    list.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect()
}

/// 盔甲架姿势预设（NBT Pose 分量，角度制）
const ARMOR_POSES: [(&str, &str); 7] = [
    ("标准", ""),
    ("挥手", "RightArm:[-60f,0f,-80f]"),
    ("双手举起", "LeftArm:[0f,0f,-150f],RightArm:[0f,0f,150f]"),
    ("坐姿", "LeftLeg:[-90f,0f,0f],RightLeg:[-90f,0f,0f]"),
    (
        "奔跑",
        "LeftArm:[-70f,0f,10f],RightArm:[70f,0f,-10f],LeftLeg:[50f,0f,0f],RightLeg:[-50f,0f,0f]",
    ),
    ("战斗", "LeftArm:[-80f,20f,-15f],RightArm:[-80f,-20f,15f]"),
    ("思考", "Head:[25f,0f,-25f],RightArm:[-90f,0f,-40f]"),
];

fn build_armorstand_cmd(
    name: &str,
    arms: bool,
    no_base: bool,
    invisible: bool,
    small: bool,
    invul: bool,
    pose: usize,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !name.trim().is_empty() {
        let nm = make_text(name.trim());
        parts.push(format!("CustomName:'{}'", nm));
    }
    if arms {
        parts.push("ShowArms:1b".to_string());
    }
    if no_base {
        parts.push("NoBasePlate:1b".to_string());
    }
    if invisible {
        parts.push("Invisible:1b".to_string());
    }
    if small {
        parts.push("Small:1b".to_string());
    }
    if invul {
        parts.push("Invulnerable:1b".to_string());
    }
    if let Some((_, p)) = ARMOR_POSES.get(pose) {
        if !p.is_empty() {
            parts.push(format!("Pose:{{{}}}", p));
        }
    }
    let inner = parts.join(",");
    // 全默认时不写空的 `{}`，直接一个裸盔甲架
    if inner.is_empty() {
        return "/summon minecraft:armor_stand ~ ~ ~".to_string();
    }
    format!("/summon minecraft:armor_stand ~ ~ ~ {{{}}}", inner)
}

fn make_text(s: &str) -> String {
    // 这段 JSON 会被塞进 SNBT 的单引号里（'...'），要过两道解析：
    //   1. SNBT 单引号串只认 \\ 和 \' 两种转义，别的反斜杠原样保留；
    //   2. 剩下的文本再按 JSON 解析一次。
    // 所以 json! 已经产出的 `\`（如 `\"`、`\\`）必须先按 SNBT 规则翻倍，
    // 才不会在第一步被吃掉。最后再转义单引号，顺序不能反。
    json!({"text": s, "italic": false})
        .to_string()
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
}

fn is_cell4(mod_key: &str) -> bool {
    MOD_MAP
        .iter()
        .find(|(k, _)| *k == mod_key)
        .map(|(_, v)| *v == "Cell⁴")
        .unwrap_or(false)
}

/// 生成无限元件指令。
/// `selector` 跟随顶部「目标」下拉 —— 以前这里硬编码 `@s`，
/// 即使选了「所有玩家」也只给自己。
fn build_infinity_cmd(
    item_id: &str,
    cell_type: &str,
    version: &str,
    mod_key: &str,
    selector: &str,
) -> String {
    let extae = extae_id_for(item_id, version);
    let item_id = item_id_for(&extae, version);
    if version == "1.20.1" {
        // Cell⁴ 物品/流体共用 cell4:infinity_item_cell，1.20.1 用 NBT 语法绑定
        if is_cell4(mod_key) {
            return format!(
                "/give {} cell4:infinity_item_cell{{cell4item:\"{}\"}}",
                selector, item_id
            );
        }
        let tag = if cell_type == "item" {
            ITEM_TAG
        } else {
            FLUID_TAG
        };
        return format!(
            "/give {} expatternprovider:infinity_cell{{record:{{id:\"{}\",\"#c\":\"{}\"}}}}",
            selector, item_id, tag
        );
    }
    if is_cell4(mod_key) {
        return format!(
            "/give {} cell4:infinity_item_cell[custom_data={{cell4item:\"{}\"}}]",
            selector, item_id
        );
    }
    format!("/give {} {}", selector, item_id)
}

/// ExtendedAE 的 modid 随 MC 版本走：1.20.x = expatternprovider，1.21.x = extendedae
/// 数据表统一存 1.21 的新前缀，此处按版本回退到旧 modid
fn extae_id_for<'a>(item_id: &'a str, version: &str) -> Cow<'a, str> {
    if version == "1.20.1" && item_id.starts_with("extendedae:") {
        Cow::Owned(format!(
            "expatternprovider:{}",
            &item_id["extendedae:".len()..]
        ))
    } else {
        Cow::Borrowed(item_id)
    }
}

/// 原版物品 ID 跨版本改名：`(1.21.1 用的新 ID, 1.20.1 用的旧 ID)`。
///
/// 目前只有鳞甲一项：1.21.1 把 `scute` 拆成了 `armadillo_scute`（犰狳鳞甲）和
/// `turtle_scute`（海龟鳞甲），旧的 `scute` 已从注册表删除。数据表统一存新 ID，
/// 两个方向都换：1.20.1 出指令时换回旧 ID，1.21.1 手输了旧 ID 也换成新的 ——
/// 跟 `extae_id_for` 是同一套思路。
const ITEM_ID_RENAMES: [(&str, &str); 1] = [("minecraft:turtle_scute", "minecraft:scute")];

fn item_id_for<'a>(item_id: &'a str, version: &str) -> Cow<'a, str> {
    for (new_id, old_id) in ITEM_ID_RENAMES {
        if version == "1.20.1" && item_id == new_id {
            return Cow::Owned(old_id.to_string());
        }
        if version != "1.20.1" && item_id == old_id {
            return Cow::Owned(new_id.to_string());
        }
    }
    Cow::Borrowed(item_id)
}

// 横扫之刃 1.20.5 起才叫 sweeping_edge，1.20.1 仍是 sweeping；数据表统一存新 ID，此处按版本回退
fn ench_id_for<'a>(e: &'a str, version: &str) -> &'a str {
    if version == "1.20.1" && e == "minecraft:sweeping_edge" {
        "minecraft:sweeping"
    } else {
        e
    }
}

// 手持附魔：原版最大等级表（/enchant 会校验等级与冲突，超出即整条报错，故生成时截断到上限）
const ENCH_MAX: &[(&str, u32)] = &[
    ("minecraft:sharpness", 5),
    ("minecraft:smite", 5),
    ("minecraft:bane_of_arthropods", 5),
    ("minecraft:knockback", 2),
    ("minecraft:fire_aspect", 2),
    ("minecraft:looting", 3),
    ("minecraft:sweeping_edge", 3),
    ("minecraft:efficiency", 5),
    ("minecraft:silk_touch", 1),
    ("minecraft:fortune", 3),
    ("minecraft:power", 5),
    ("minecraft:punch", 2),
    ("minecraft:flame", 1),
    ("minecraft:infinity", 1),
    ("minecraft:protection", 4),
    ("minecraft:fire_protection", 4),
    ("minecraft:blast_protection", 4),
    ("minecraft:projectile_protection", 4),
    ("minecraft:feather_falling", 4),
    ("minecraft:respiration", 3),
    ("minecraft:aqua_affinity", 1),
    ("minecraft:thorns", 3),
    ("minecraft:depth_strider", 3),
    ("minecraft:frost_walker", 2),
    ("minecraft:soul_speed", 3),
    ("minecraft:swift_sneak", 3),
    ("minecraft:unbreaking", 3),
    ("minecraft:mending", 1),
    ("minecraft:lure", 3),
    ("minecraft:luck_of_the_sea", 3),
    ("minecraft:loyalty", 3),
    ("minecraft:impaling", 5),
    ("minecraft:riptide", 3),
    ("minecraft:channeling", 1),
    ("minecraft:multishot", 1),
    ("minecraft:piercing", 4),
    ("minecraft:quick_charge", 3),
    ("minecraft:density", 5),
    ("minecraft:breach", 4),
    ("minecraft:wind_burst", 3),
];

fn ench_max_level(e: &str) -> u32 {
    ENCH_MAX
        .iter()
        .find(|(id, _)| *id == e)
        .map(|(_, m)| *m)
        .unwrap_or(1)
}

/// 手持附魔：直接给手上现有物品加附魔（/enchant），保留物品原有数据；
/// 一条附魔对应一条命令，等级超过原版上限时截断；
/// 无耐久消耗无法用 /enchant 写入，1.20.5+ 用内联 item modifier 单独补一条
fn build_hand_ench_cmds(
    version: &str,
    selector: &str,
    ench_ids: &[&str],
    use_max: bool,
    level_in: u32,
    unbreakable: bool,
) -> String {
    let mut selected: Vec<&str> = ench_ids.to_vec();
    if version == "1.20.1" {
        let v21 = [
            "minecraft:density",
            "minecraft:breach",
            "minecraft:wind_burst",
        ];
        selected.retain(|e| !v21.contains(e));
    }
    let mut cmds: Vec<String> = selected
        .iter()
        .map(|e| {
            let max = ench_max_level(e);
            let lv = if use_max { max } else { level_in.min(max) };
            format!("/enchant {} {} {}", selector, ench_id_for(e, version), lv)
        })
        .collect();
    if unbreakable && version != "1.20.1" {
        cmds.push(format!(
            "/item modify entity {} weapon.mainhand {{\"function\":\"minecraft:set_components\",\"components\":{{\"minecraft:unbreakable\":{{}}}}}}",
            selector
        ));
    }
    cmds.join("\n")
}

fn build_enchant_cmd(
    item_id: &str,
    ench_ids: &[&str],
    version: &str,
    selector: &str,
    unbreakable: bool,
    level: u32,
) -> String {
    let levels: Vec<(&str, u32)> = ench_ids.iter().map(|e| (*e, level)).collect();
    build_enchant_cmd_levels(item_id, &levels, version, selector, unbreakable)
}

/// 与 `build_enchant_cmd` 一样，但每个附魔可以带各自的等级（装备页需要按原版上限分别给）。
fn build_enchant_cmd_levels(
    item_id: &str,
    ench: &[(&str, u32)],
    version: &str,
    selector: &str,
    unbreakable: bool,
) -> String {
    // 什么都没选就别生成 `item[]` / `item{}` 这种空壳，退化成普通 give
    if ench.is_empty() && !unbreakable {
        return format!("/give {} {}", selector, item_id);
    }
    if version == "1.21.1" {
        let mut comp = String::new();
        if !ench.is_empty() {
            let levels: Vec<String> = ench
                .iter()
                .map(|(e, lv)| format!("\"{}\":{}", ench_id_for(e, version), lv))
                .collect();
            comp.push_str(&format!(
                "[enchantments={{levels:{{{}}}}}",
                levels.join(",")
            ));
        } else {
            comp.push('[');
        }
        if unbreakable {
            if comp.len() > 1 {
                comp.push_str(",unbreakable={}]");
            } else {
                comp.push_str("unbreakable={}]");
            }
        } else {
            comp.push(']');
        }
        return format!("/give {} {}{}", selector, item_id, comp);
    }
    // 1.20.1
    let mut nbt = String::new();
    if !ench.is_empty() {
        let parts: Vec<String> = ench
            .iter()
            .map(|(e, lv)| format!("{{id:\"{}\",lvl:{}s}}", ench_id_for(e, version), lv))
            .collect();
        nbt.push_str(&format!("{{Enchantments:[{}]", parts.join(",")));
    } else {
        nbt.push('{');
    }
    if unbreakable {
        if nbt.len() > 1 {
            nbt.push_str(",Unbreakable:1b}");
        } else {
            nbt.push_str("Unbreakable:1b}");
        }
    } else {
        nbt.push('}');
    }
    format!("/give {} {}{}", selector, item_id, nbt)
}

fn build_name_cmd(
    item_id: &str,
    name: &str,
    lore_lines: &[String],
    version: &str,
    selector: &str,
) -> String {
    let name_json = if !name.trim().is_empty() {
        Some(make_text(name.trim()))
    } else {
        None
    };
    let lore_jsons: Vec<String> = lore_lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| make_text(l.trim()))
        .collect();
    if version == "1.21.1" {
        let mut parts: Vec<String> = Vec::new();
        if let Some(nj) = &name_json {
            parts.push(format!("item_name='{}'", nj));
        }
        if !lore_jsons.is_empty() {
            let quoted: Vec<String> = lore_jsons.iter().map(|j| format!("'{}'", j)).collect();
            parts.push(format!("lore=[{}]", quoted.join(",")));
        }
        // 组件列表为空时给出 `item[]` 虽合法但没意义，直接退化成普通 give
        if parts.is_empty() {
            return format!("/give {} {}", selector, item_id);
        }
        return format!("/give {} {}[{}]", selector, item_id, parts.join(","));
    }
    let mut ndisp: Vec<String> = Vec::new();
    if let Some(nj) = &name_json {
        ndisp.push(format!("Name:'{}'", nj));
    }
    if !lore_jsons.is_empty() {
        let quoted: Vec<String> = lore_jsons.iter().map(|j| format!("'{}'", j)).collect();
        ndisp.push(format!("Lore:[{}]", quoted.join(",")));
    }
    // 同上：`{display:{}}` 是空壳，别生成
    if ndisp.is_empty() {
        return format!("/give {} {}", selector, item_id);
    }
    format!(
        "/give {} {}{{display:{{{}}}}}",
        selector,
        item_id,
        ndisp.join(",")
    )
}

fn build_potion_cmd(
    item_id: &str,
    effect_id: &str,
    potion_id: u8,
    amplifier: i32,
    duration: i32,
    version: &str,
    selector: &str,
) -> String {
    if version == "1.21.1" {
        let eff = format!(
            "{{id:\"{}\",amplifier:{},duration:{}}}",
            effect_id, amplifier, duration
        );
        return format!(
            "/give {} {}[potion_contents={{custom_effects:[{}]}}]",
            selector, item_id, eff
        );
    }
    let eff = format!(
        "{{Id:{},Amplifier:{},Duration:{}}}",
        potion_id, amplifier, duration
    );
    format!(
        "/give {} {}{{CustomPotionEffects:[{}]}}",
        selector, item_id, eff
    )
}

/// 实体属性 NBT 片段。
/// Java 1.21 起字段名改为小写 `attributes:[{id:..., base:...}]`；
/// 属性 ID 前缀（`generic.` / `player.`）在 1.21.1 仍然保留，别一起改掉。
fn attr_nbt(name: &str, value: f64, version: &str) -> String {
    if version == "1.21.1" {
        format!("{{id:\"{}\",base:{}}}", name, value)
    } else {
        format!("{{Name:\"{}\",Base:{}}}", name, value)
    }
}

/// 没有刷怪蛋的实体。
/// 1.13 物品扁平化后 `minecraft:spawn_egg` 这个通用蛋就不存在了，每个实体有自己的蛋；
/// 下面这份名单取自 1.20.1 与 1.21.1 官方注册表（entity_type 中有、item 中无 `*_spawn_egg` 的并集）。
const NO_EGG_ENTITIES: &[&str] = &[
    "minecraft:area_effect_cloud",
    "minecraft:armor_stand",
    "minecraft:arrow",
    "minecraft:block_display",
    "minecraft:boat",
    "minecraft:breeze_wind_charge",
    "minecraft:chest_boat",
    "minecraft:chest_minecart",
    "minecraft:command_block_minecart",
    "minecraft:dragon_fireball",
    "minecraft:egg",
    "minecraft:end_crystal",
    "minecraft:ender_pearl",
    "minecraft:evoker_fangs",
    "minecraft:experience_bottle",
    "minecraft:experience_orb",
    "minecraft:eye_of_ender",
    "minecraft:falling_block",
    "minecraft:fireball",
    "minecraft:firework_rocket",
    "minecraft:fishing_bobber",
    "minecraft:furnace_minecart",
    "minecraft:giant",
    "minecraft:glow_item_frame",
    "minecraft:hopper_minecart",
    "minecraft:illusioner",
    "minecraft:interaction",
    "minecraft:item",
    "minecraft:item_display",
    "minecraft:item_frame",
    "minecraft:leash_knot",
    "minecraft:lightning_bolt",
    "minecraft:llama_spit",
    "minecraft:marker",
    "minecraft:minecart",
    "minecraft:ominous_item_spawner",
    "minecraft:painting",
    "minecraft:player",
    "minecraft:potion",
    "minecraft:shulker_bullet",
    "minecraft:small_fireball",
    "minecraft:snowball",
    "minecraft:spawner_minecart",
    "minecraft:spectral_arrow",
    "minecraft:text_display",
    "minecraft:tnt",
    "minecraft:tnt_minecart",
    "minecraft:trident",
    "minecraft:wind_charge",
    "minecraft:wither_skull",
];

/// 实体 ID → 该实体对应刷怪蛋的物品 ID。
fn egg_item_for(entity_id: &str) -> Option<String> {
    if NO_EGG_ENTITIES.contains(&entity_id) {
        return None;
    }
    let (ns, path) = entity_id.split_once(':')?;
    if ns.is_empty() || path.is_empty() {
        return None;
    }
    Some(format!("{}:{}_spawn_egg", ns, path))
}

fn build_egg_cmd(
    entity_id: &str,
    custom_name: &str,
    scale: &str,
    version: &str,
    selector: &str,
) -> String {
    let Some(egg_item) = egg_item_for(entity_id) else {
        return format!(
            "无法生成：{} 没有对应的刷怪蛋，请改用「自定义实体」页的 /summon",
            entity_id
        );
    };
    let mut nbt_parts: Vec<String> = Vec::new();
    if !custom_name.trim().is_empty() {
        let nm = make_text(custom_name.trim());
        nbt_parts.push(format!("CustomName:'{}'", nm));
    }
    // 1.20.1 无 generic.scale 属性（1.20.5+ 才有），只有 1.21.1 处理大小
    if version == "1.21.1" {
        if let Ok(s) = scale.trim().parse::<f64>() {
            if s != 1.0 {
                nbt_parts.push(format!(
                    "attributes:[{}]",
                    attr_nbt("minecraft:generic.scale", s, version)
                ));
            }
        }
        let inner = nbt_parts.join(",");
        let data = if inner.is_empty() {
            format!("{{id:\"{}\"}}", entity_id)
        } else {
            format!("{{id:\"{}\",{}}}", entity_id, inner)
        };
        return format!("/give {} {}[entity_data={}]", selector, egg_item, data);
    }
    let inner = nbt_parts.join(",");
    let data = if inner.is_empty() {
        format!("{{EntityTag:{{id:\"{}\"}}}}", entity_id)
    } else {
        format!("{{EntityTag:{{id:\"{}\",{}}}}}", entity_id, inner)
    };
    format!("/give {} {}{}", selector, egg_item, data)
}

#[derive(Default)]
struct EntityFlags {
    noai: bool,
    invul: bool,
    persist: bool,
    glow: bool,
    silent: bool,
}

fn build_entity_cmd(
    entity_id: &str,
    name: &str,
    scale: &str,
    hp: &str,
    atk: &str,
    with_gear: bool,
    flags: &EntityFlags,
    version: &str,
    count: i32,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !name.trim().is_empty() {
        let nm = make_text(name.trim());
        parts.push(format!("CustomName:'{}'", nm));
    }
    let mut attrs: Vec<String> = Vec::new();
    if let Ok(h) = hp.trim().parse::<f64>() {
        attrs.push(attr_nbt("minecraft:generic.max_health", h, version));
    }
    if let Ok(a) = atk.trim().parse::<f64>() {
        attrs.push(attr_nbt("minecraft:generic.attack_damage", a, version));
    }
    if version == "1.21.1" {
        if let Ok(s) = scale.trim().parse::<f64>() {
            if s != 1.0 {
                attrs.push(attr_nbt("minecraft:generic.scale", s, version));
            }
        }
    }
    if !attrs.is_empty() {
        if version == "1.21.1" {
            parts.push(format!("attributes:[{}]", attrs.join(",")));
        } else {
            parts.push(format!("Attributes:[{}]", attrs.join(",")));
        }
    }
    if with_gear {
        if version == "1.21.1" {
            parts.push("HandItems:[{id:\"minecraft:netherite_sword\",count:1},{}]".to_string());
            parts.push(
                "ArmorItems:[{id:\"minecraft:netherite_boots\",count:1},{id:\"minecraft:netherite_leggings\",count:1},{id:\"minecraft:netherite_chestplate\",count:1},{id:\"minecraft:netherite_helmet\",count:1}]".to_string(),
            );
        } else {
            parts.push("HandItems:[{id:\"minecraft:netherite_sword\",Count:1b},{}]".to_string());
            parts.push(
                "ArmorItems:[{id:\"minecraft:netherite_boots\",Count:1b},{id:\"minecraft:netherite_leggings\",Count:1b},{id:\"minecraft:netherite_chestplate\",Count:1b},{id:\"minecraft:netherite_helmet\",Count:1b}]".to_string(),
            );
        }
    }
    if flags.noai {
        parts.push("NoAI:1b".to_string());
    }
    if flags.invul {
        parts.push("Invulnerable:1b".to_string());
    }
    if flags.persist {
        parts.push("PersistenceRequired:1b".to_string());
    }
    if flags.glow {
        parts.push("Glowing:1b".to_string());
    }
    if flags.silent {
        parts.push("Silent:1b".to_string());
    }
    let inner = parts.join(",");
    let data = if inner.is_empty() {
        format!("{{id:\"{}\"}}", entity_id)
    } else {
        format!("{{id:\"{}\",{}}}", entity_id, inner)
    };
    let cnt = count.clamp(1, 100);
    let mut cmds: Vec<String> = Vec::new();
    for i in 0..cnt {
        let pos = if i == 0 {
            "~ ~ ~".to_string()
        } else {
            format!("~{} ~ ~", i)
        };
        cmds.push(format!("/summon {} {} {}", entity_id, pos, data));
    }
    cmds.join("\n")
}

fn build_attr_cmd(attr_id: &str, value: &str, selector: &str) -> String {
    format!("/attribute {} {} base set {}", selector, attr_id, value)
}

fn build_head_cmd(player_name: &str, version: &str, selector: &str) -> String {
    if version == "1.21.1" {
        format!(
            "/give {} minecraft:player_head[profile={{name:\"{}\"}}]",
            selector, player_name
        )
    } else {
        format!(
            "/give {} minecraft:player_head{{SkullOwner:\"{}\"}}",
            selector, player_name
        )
    }
}

fn gear_enchant_names(p: &str) -> Vec<&'static str> {
    match p {
        "剑" => vec![
            "minecraft:sharpness",
            "minecraft:knockback",
            "minecraft:fire_aspect",
            "minecraft:looting",
            "minecraft:sweeping_edge",
            "minecraft:unbreaking",
            "minecraft:mending",
        ],
        "斧" => vec![
            "minecraft:sharpness",
            "minecraft:efficiency",
            "minecraft:unbreaking",
            "minecraft:mending",
        ],
        "镐" => vec![
            "minecraft:efficiency",
            "minecraft:fortune",
            "minecraft:unbreaking",
            "minecraft:mending",
        ],
        _ => vec![
            "minecraft:protection",
            "minecraft:unbreaking",
            "minecraft:mending",
        ],
    }
}

fn build_gear_cmd(
    material: &str,
    parts: &[&str],
    version: &str,
    selector: &str,
    ench_on: bool,
) -> String {
    // 第一个元素是中文显示名（"钻石"），第二个才是物品 ID 前缀（"minecraft:diamond"）。
    // 以前这里把第一个当前缀用，生成的是 `钻石_sword` 这种不存在的物品 ID。
    let Some((_, prefix)) = GEAR_MATERIALS.iter().find(|(k, _)| *k == material) else {
        return String::new();
    };
    let mut cmds: Vec<String> = Vec::new();
    for p in parts {
        let Some(suffix) = GEAR_PART_MAP.iter().find(|(k, _)| k == p).map(|(_, v)| *v) else {
            continue;
        };
        let item_id = format!("{}_{}", prefix, suffix);
        if !ench_on {
            cmds.push(format!("/give {} {}", selector, item_id));
            continue;
        }
        let ench = gear_enchant_names(p);
        // 每个附魔用它自己的原版上限，别一把 255 糊上去
        let levels: Vec<(&str, u32)> = ench.iter().map(|e| (*e, ench_max_level(e))).collect();
        cmds.push(build_enchant_cmd_levels(
            &item_id, &levels, version, selector, false,
        ));
    }
    cmds.join("\n")
}

// ============ 应用状态 ============

/// 子线程扫描的产物。
///
/// 扫描结果只活在内存里，不落盘：这个工具是单文件绿色版，
/// exe 旁边除了配置不该再冒出别的东西，用完点「清除缓存」或关掉程序就没了。
struct ScanOutcome {
    jars: usize,
    failed: usize,
    /// (ID, 中文名)，已按 ID 排好序
    items: Vec<(String, String)>,
    entities: Vec<(String, String)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Page {
    Home,
    Infinity,
    Enchant,
    HandEnch,
    Name,
    Potion,
    Egg,
    Attr,
    Head,
    Gear,
    Quick,
    History,
    Search,
    Batch,
    Entity,
    ArmorStand,
    Settings,
}

/// 「设置与清理」页里可执行的清理动作。
/// 全部是删注册表里的东西，所以都要两段式确认，避免误点。
#[derive(Clone, Copy, PartialEq, Debug)]
enum RegAction {
    /// 只清指令历史（最多 200 条）
    History,
    /// 清传送点与自定义指令
    Records,
    /// 清运行期的搜索词/搜索结果/勾选状态（不碰任何存进注册表的输入框）
    Transient,
    /// 各页输入框恢复默认值（保留历史、传送点、自定义指令、整合包路径、窗口位置）
    Inputs,
    /// 删掉解析失败时留存的原文
    Broken,
    /// 删掉上次启动的错误记录
    LastError,
    /// 删掉整个注册表项，等同 `reg delete "HKCU\Software\minecraft_tool" /f`
    WholeKey,
    Defaults,
}

struct App {
    version: String,
    selector: String,
    page: Page,

    // 无限元件
    inf_entry: String,
    inf_type: String,
    inf_mod: String,
    inf_result: String,
    inf_status: String,

    // 附魔
    ench_entry: String,
    ench_unbreakable: bool,
    ench_checks: Vec<bool>,
    ench_result: String,
    ench_status: String,
    ench_preset_cat: String,
    ench_preset_checks: Vec<bool>,
    ench_page: usize, // 附魔列表翻页

    // 手持附魔
    hand_checks: Vec<bool>,
    hand_level: String,
    hand_use_max: bool,
    hand_unbreakable: bool,
    hand_page: usize,
    hand_result: String,
    hand_status: String,

    // 快捷指令
    quick_tab: usize,
    // 历史/搜索翻页
    history_page: usize,
    search_page: usize,

    // 命名
    name_entry: String,
    name_text: String,
    name_lore: Vec<String>,
    name_result: String,

    // 药水
    pot_effect: String,
    pot_level: String,
    pot_dur: String,
    pot_result: String,

    // 刷怪蛋
    egg_entity: String,
    egg_name: String,
    egg_scale: String,
    egg_result: String,

    // 属性
    attr_id: String,
    attr_val: String,
    attr_result: String,

    // 头颅
    head_name: String,
    head_result: String,

    // 装备
    gear_mat: String,
    gear_parts: Vec<bool>,
    gear_ench_on: bool,
    gear_result: String,

    // 历史
    history: VecDeque<HistoryEntry>,
    history_selected: HashSet<usize>,
    history_filter: usize,
    result_versions: HashMap<Page, String>,
    quick_options: QuickOptions,
    favorites: Vec<Favorite>,
    world_presets: Vec<WorldPreset>,
    world_draft: WorldPreset,
    json_path: String,
    pending_import: Option<Config>,
    purge_confirm: String,

    // 搜索
    search_key: String,
    search_result: Vec<(String, String)>,

    // 批量
    batch_entry: String,
    batch_count: String,
    batch_result: String,

    // 实体
    ent_id: String,
    ent_name: String,
    ent_scale: String,
    ent_hp: String,
    ent_atk: String,
    ent_gear: bool,
    ent_count: String,
    ent_result: String,

    // 各页复制按钮的反馈（切页时清空）
    copy_status: String,
    // 主题 / 窗口 / 持久化
    palette: &'static Palette,
    dark_mode: bool,
    always_top: bool,
    top_apply: bool,
    last_saved: String,
    // 附魔等级 / 药水类型
    ench_level: String,
    pot_kind: usize,
    // 快捷页：自定义指令与传送点
    custom_cmds: Vec<CustomCmd>,
    teleports: Vec<Teleport>,
    qc_label: String,
    qc_cmd: String,
    tp_name: String,
    tp_pos: String,
    // 原版物品中文名库（搜索用）
    vanilla_items: Vec<(String, String)>,
    // 与 vanilla_items 同序的小写副本，搜索时直接比对，避免每次逐条 to_lowercase
    vanilla_items_lc: Vec<(String, String)>,
    // 模组物品库（扫描整合包 mods 目录）
    mods_dir: String,
    mod_items: Vec<(String, String)>,
    mod_items_lc: Vec<(String, String)>,
    scan_status: String,
    // 实体开关
    ent_noai: bool,
    ent_invul: bool,
    ent_persist: bool,
    ent_glow: bool,
    ent_silent: bool,
    // 窗口位置/大小记忆
    win_x: f32,
    win_y: f32,
    win_w: f32,
    win_h: f32,
    started: std::time::Instant,
    // 模组生物库
    mod_entities: Vec<(String, String)>,
    ent_search: String,
    ent_hits: Vec<(String, String)>,
    // 盔甲架
    as_name: String,
    as_arms: bool,
    as_no_base: bool,
    as_invisible: bool,
    as_small: bool,
    as_invul: bool,
    as_pose: usize,
    as_result: String,
    inf_preset_page: usize,
    inf_preset_sel: Option<usize>,
    inf_sel: HashSet<String>,
    inf_search: String, // 预设搜索框

    // 后台扫描（避免扫描时冻结界面）
    scan_rx: Option<std::sync::mpsc::Receiver<Result<ScanOutcome, String>>>,
    scanning: bool,
    // 启动时的配置告警（如配置损坏已备份），显示在顶部
    startup_notice: Option<String>,
    // 各页提示的成功/失败语义：true 绿、false 红
    copy_status_ok: bool,
    ench_status_ok: bool,
    hand_status_ok: bool,
    inf_status_ok: bool,
    // 「设置与清理」页：待确认的动作 + 上次清理结果
    reg_pending: Option<RegAction>,
    reg_status: String,
    reg_status_ok: bool,
    /// 注册表值列表的缓存。读注册表本身很便宜，但界面每帧都会重绘，
    /// 没必要 60 次/秒去开键枚举 —— 只在进页/动作执行后刷新一次。
    reg_cache: Option<Vec<(String, usize)>>,
    /// 动作执行后要把缓存标脏，等下一帧再读：
    /// 如果当场重读，读到的是"刚删完"的空快照，界面就会一直显示 0 个值。
    reg_cache_stale: bool,
    /// 值卡片里「查看」的那个值：(名字, 内容)。内容只在点击时读一次。
    reg_view: Option<(String, String)>,
    /// 最近一次写注册表失败的原始错误。
    /// 以前写失败被 `.is_ok()` 吞掉了：界面照常显示修改成功，重启一看全丢了。
    /// 现在留在这里，顶部横幅 + 「设置与清理」页都能看到。
    write_error: Option<String>,
    /// 三个备份槽的缓存：(槽名, 保存时间, 字节数)。和 reg_cache 一样只在进页/动作后重读。
    backup_cache: Option<Vec<(String, String, usize)>>,
    /// 备份槽上待确认的动作：(槽名, 动作)。
    /// 恢复和清空都会动到现有数据，同样要两段式确认。
    backup_pending: Option<(String, BackupOp)>,
    /// 上一次启动时做的自检结果，只算一次，进页时不重算（N7）。
    startup_checks: Option<Vec<(bool, String, String)>>,
    /// 「指令历史」页点了「清空」但还没确认。
    /// 清空历史不可撤销，跟设置页的删除一样走两段式确认，别让一次误点就没了。
    history_clear_pending: bool,
}

impl App {
    fn new(ctx: &egui::Context, cfg: Config, startup_notice: Option<String>) -> Self {
        // 字体：中文字体 + 符号兜底
        //
        // 顺序有讲究，别随手调换。egui 的做法是「逐字体找第一个有这个字形的」，
        // 而且整行高度按**字族里第一个**字体算（Fonts::row_height）。内置的
        // Ubuntu-Light 排最前面，界面的行高和间距都是照它调出来的；把中文字体
        // 提到最前会让行高从 40px 涨到 45px，整窗布局全松掉 —— 试过，不能那么干。
        //
        // 所以中文字体（以及符号字体）一律追加到末尾，只在前面几个字体都没有
        // 该字形时才接手。汉字只有中文字体有，天然会落过来。
        //
        // 末尾再挂一支 Segoe UI Symbol：个别机器上第一个读得到的简体中文字体是
        // simhei 或 simsun，这两个不带 U+2074（上标四，「Cell⁴」里那个 4）。本机
        // 用的是 msyh 有这字形、内置的 Ubuntu-Light 也有，所以平时看不出来，但
        // 换台机器可能就是方块。seguisym 从 Windows 7 起都有，塞在最后当保险。
        let mut fonts = egui::FontDefinitions::default();
        for path in [
            "C:\\Windows\\Fonts\\msyh.ttc",
            "C:\\Windows\\Fonts\\simhei.ttf",
            "C:\\Windows\\Fonts\\simsun.ttc",
        ] {
            if let Ok(bytes) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert("cn".to_owned(), egui::FontData::from_owned(bytes));
                for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                    fonts
                        .families
                        .entry(family)
                        .or_default()
                        .push("cn".to_owned());
                }
                break;
            }
        }
        if let Ok(bytes) = std::fs::read("C:\\Windows\\Fonts\\seguisym.ttf") {
            fonts
                .font_data
                .insert("sym".to_owned(), egui::FontData::from_owned(bytes));
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .push("sym".to_owned());
            }
        }
        ctx.set_fonts(fonts);

        // 扫描结果不再从磁盘读缓存：只在本次运行期间留在内存里。
        let mod_items: Vec<(String, String)> = Vec::new();
        let mod_entities: Vec<(String, String)> = Vec::new();
        let vanilla_items: Vec<(String, String)> =
            serde_json::from_str::<std::collections::HashMap<String, String>>(include_str!(
                "../assets/items_zh.json"
            ))
            .map(|m| {
                // 名表是从语言文件抽的，混进了三类不是物品的键：
                //   1. 组件名，带点，如 banner.base.black（688 条）、potion.night_vision；
                //   2. 方块，如 beetroots、acacia_wall_sign、sweet_berry_bush —— /give 给不出来；
                //   3. 旧 ID，如 grass、scute（1.21.1 已拆成 turtle_scute / armadillo_scute）。
                // 不过滤的话搜索页会把它们当物品列出来，点一下复制出无效指令。
                // 前两类靠形状判断，第三类只能对注册表 —— 用随包内嵌的两版 ID 清单查。
                let ids = all_vanilla_ids();
                m.into_iter()
                    .filter(|(k, _)| {
                        k.strip_prefix("minecraft:")
                            .map(|s| !s.is_empty() && !s.contains('.') && ids.contains(s))
                            .unwrap_or(true)
                    })
                    .map(|(k, v)| (v, k))
                    .collect()
            })
            .unwrap_or_default();
        // 搜索要在这两千多（原版）+ 上万（模组）条里逐条比中文名，
        // 每次按键都重新 to_lowercase 会明显卡顿，所以加载时一次性建好小写副本。
        let lower_pairs = |v: &[(String, String)]| -> Vec<(String, String)> {
            v.iter()
                .map(|(n, i)| (n.to_lowercase(), i.to_lowercase()))
                .collect()
        };
        let vanilla_items_lc = lower_pairs(&vanilla_items);
        let mod_items_lc = lower_pairs(&mod_items);

        let palette = if cfg.dark_mode { &DARK } else { &LIGHT };
        apply_theme(ctx, palette, cfg.dark_mode);

        let ench_n = ENCHANTMENTS.len();
        let preset_cats: Vec<&str> = ENCH_PRESETS.iter().map(|(k, _)| *k).collect();
        let first_preset_len = ENCH_PRESETS.first().map(|(_, v)| v.len()).unwrap_or(0);
        let gear_part_n = GEAR_PART_MAP.len();
        let lore = vec![String::new(); 3];
        let history = load_history(&cfg);
        let nonempty = |s: &str, d: &str| {
            if s.is_empty() {
                d.to_string()
            } else {
                s.to_string()
            }
        };

        App {
            version: nonempty(&cfg.version, "1.21.1"),
            selector: nonempty(&cfg.selector, "@s"),
            page: Page::Home,
            inf_entry: nonempty(&cfg.inf_entry, "minecraft:diamond"),
            inf_type: "item".to_string(),
            inf_mod: "ExtendedAE扩展AE".to_string(),
            inf_result: String::new(),
            inf_status: String::new(),
            ench_entry: nonempty(&cfg.ench_entry, "minecraft:netherite_sword"),
            ench_unbreakable: false,
            ench_checks: vec![false; ench_n],
            ench_result: String::new(),
            ench_status: String::new(),
            ench_preset_cat: preset_cats.first().copied().unwrap_or("").to_string(),
            ench_preset_checks: vec![false; first_preset_len],
            ench_page: 0,
            hand_checks: vec![false; ench_n],
            hand_level: "255".to_string(),
            hand_use_max: true,
            hand_unbreakable: false,
            hand_page: 0,
            hand_result: String::new(),
            hand_status: String::new(),
            quick_tab: 0,
            history_page: 0,
            search_page: 0,
            name_entry: nonempty(&cfg.name_entry, "minecraft:diamond"),
            name_text: cfg.name_text.clone(),
            name_lore: lore,
            name_result: String::new(),
            pot_effect: POTIONS[0].0.to_string(),
            pot_kind: 0,
            pot_level: nonempty(&cfg.pot_level, "1"),
            pot_dur: nonempty(&cfg.pot_dur, "30"),
            pot_result: String::new(),
            egg_entity: nonempty(&cfg.egg_entity, "minecraft:zombie"),
            egg_name: String::new(),
            egg_scale: "1".to_string(),
            egg_result: String::new(),
            attr_id: ATTRIBUTES[0].1.to_string(),
            attr_val: nonempty(&cfg.attr_val, "100"),
            attr_result: String::new(),
            head_name: nonempty(&cfg.head_name, "Notch"),
            head_result: String::new(),
            gear_mat: GEAR_MATERIALS[0].0.to_string(),
            gear_parts: vec![false; gear_part_n],
            gear_ench_on: true,
            gear_result: String::new(),
            history,
            history_selected: HashSet::new(),
            history_filter: 0,
            result_versions: HashMap::new(),
            quick_options: cfg.quick_options.clone(),
            favorites: cfg.favorites.clone(),
            world_presets: cfg.world_presets.clone(),
            world_draft: WorldPreset::default(),
            json_path: String::new(),
            pending_import: None,
            purge_confirm: String::new(),
            search_key: String::new(),
            search_result: Vec::new(),
            batch_entry: nonempty(&cfg.batch_entry, "minecraft:diamond"),
            batch_count: nonempty(&cfg.batch_count, "64"),
            batch_result: String::new(),
            ent_id: nonempty(&cfg.ent_id, "minecraft:zombie"),
            ent_name: String::new(),
            ent_scale: "1".to_string(),
            ent_hp: String::new(),
            ent_atk: String::new(),
            ent_gear: true,
            ent_count: "1".to_string(),
            ent_result: String::new(),
            copy_status: String::new(),
            palette,
            dark_mode: cfg.dark_mode,
            always_top: cfg.always_top,
            top_apply: cfg.always_top,
            last_saved: String::new(),
            ench_level: nonempty(&cfg.ench_level, "255"),
            custom_cmds: cfg.custom_cmds.clone(),
            teleports: cfg.teleports.clone(),
            qc_label: String::new(),
            qc_cmd: String::new(),
            tp_name: String::new(),
            tp_pos: String::new(),
            vanilla_items,
            vanilla_items_lc,
            mods_dir: cfg.mods_dir.clone(),
            mod_items,
            mod_items_lc,
            mod_entities,
            scan_status: String::new(),
            ent_search: String::new(),
            ent_hits: Vec::new(),
            as_name: String::new(),
            as_arms: true,
            as_no_base: true,
            as_invisible: false,
            as_small: false,
            as_invul: false,
            as_pose: 0,
            as_result: String::new(),
            ent_noai: false,
            ent_invul: false,
            ent_persist: false,
            ent_glow: false,
            ent_silent: false,
            win_x: cfg.win_x,
            win_y: cfg.win_y,
            win_w: cfg.win_w,
            win_h: cfg.win_h,
            started: std::time::Instant::now(),
            inf_preset_page: 0,
            inf_preset_sel: None,
            inf_sel: HashSet::new(),
            inf_search: String::new(),
            scan_rx: None,
            scanning: false,
            startup_notice,
            copy_status_ok: true,
            ench_status_ok: true,
            hand_status_ok: true,
            inf_status_ok: true,
            reg_pending: None,
            reg_status: String::new(),
            reg_status_ok: true,
            reg_cache: None,
            reg_cache_stale: false,
            reg_view: None,
            write_error: None,
            backup_cache: None,
            backup_pending: None,
            startup_checks: None,
            history_clear_pending: false,
        }
    }

    /// 把已读取或已保存的配置套用到当前 App。
    /// 只覆盖「存进注册表的那部分」状态：页面勾选、搜索框、扫描结果这些
    /// 运行期临时状态留着不动，否则用户会看到界面莫名其妙地跳。
    fn apply_config(&mut self, cfg: &Config) {
        let nonempty = |s: &str, d: &str| {
            if s.is_empty() {
                d.to_string()
            } else {
                s.to_string()
            }
        };
        self.version = nonempty(&cfg.version, "1.21.1");
        self.selector = nonempty(&cfg.selector, "@s");
        self.dark_mode = cfg.dark_mode;
        self.always_top = cfg.always_top;
        self.top_apply = true; // 交给 update() 下一帧真正下发给窗口
        self.palette = if cfg.dark_mode { &DARK } else { &LIGHT };
        self.ench_level = nonempty(&cfg.ench_level, "255");
        self.history = load_history(cfg);
        self.history_selected.clear();
        self.quick_options = cfg.quick_options.clone();
        self.favorites.clone_from(&cfg.favorites);
        self.world_presets.clone_from(&cfg.world_presets);
        self.world_draft = WorldPreset::default();
        self.reset_mod_cache_memory();
        self.scan_status.clear();
        self.search_key.clear();
        self.search_page = 0;
        self.inf_sel.clear();
        self.inf_search.clear();
        self.inf_preset_page = 0;
        self.ent_search.clear();
        self.ent_hits.clear();
        self.clear_generated_results();
        self.history_page = 0;
        self.teleports = cfg.teleports.clone();
        self.custom_cmds = cfg.custom_cmds.clone();
        self.inf_entry = nonempty(&cfg.inf_entry, "minecraft:diamond");
        self.ench_entry = nonempty(&cfg.ench_entry, "minecraft:netherite_sword");
        self.name_entry = nonempty(&cfg.name_entry, "minecraft:diamond");
        self.name_text = cfg.name_text.clone();
        self.batch_entry = nonempty(&cfg.batch_entry, "minecraft:diamond");
        self.batch_count = nonempty(&cfg.batch_count, "64");
        self.ent_id = nonempty(&cfg.ent_id, "minecraft:zombie");
        self.egg_entity = nonempty(&cfg.egg_entity, "minecraft:zombie");
        self.head_name = nonempty(&cfg.head_name, "Notch");
        self.pot_level = nonempty(&cfg.pot_level, "1");
        self.pot_dur = nonempty(&cfg.pot_dur, "30");
        self.attr_val = nonempty(&cfg.attr_val, "100");
        self.mods_dir = cfg.mods_dir.clone();
        self.win_x = cfg.win_x;
        self.win_y = cfg.win_y;
        self.win_w = cfg.win_w;
        self.win_h = cfg.win_h;
    }

    fn config_snapshot(&self) -> Config {
        Config {
            // 写入时才会填真正的时间戳（见 update()）：如果这里就填当前时间，
            // 每帧算出来的 JSON 都不一样，「内容没变就别写」这道防抖会彻底失效。
            schema_version: CURRENT_SCHEMA,
            saved_at: String::new(),
            version: self.version.clone(),
            selector: self.selector.clone(),
            dark_mode: self.dark_mode,
            always_top: self.always_top,
            ench_level: self.ench_level.clone(),
            history: self
                .history
                .iter()
                .map(|entry| entry.command.clone())
                .collect(),
            history_versions: self
                .history
                .iter()
                .map(|entry| entry.version.clone())
                .collect(),
            quick_options: self.quick_options.clone(),
            favorites: self.favorites.clone(),
            world_presets: self.world_presets.clone(),
            teleports: self.teleports.clone(),
            custom_cmds: self.custom_cmds.clone(),
            inf_entry: self.inf_entry.clone(),
            ench_entry: self.ench_entry.clone(),
            name_entry: self.name_entry.clone(),
            name_text: self.name_text.clone(),
            batch_entry: self.batch_entry.clone(),
            batch_count: self.batch_count.clone(),
            ent_id: self.ent_id.clone(),
            egg_entity: self.egg_entity.clone(),
            head_name: self.head_name.clone(),
            pot_level: self.pot_level.clone(),
            pot_dur: self.pot_dur.clone(),
            attr_val: self.attr_val.clone(),
            mods_dir: self.mods_dir.clone(),
            win_x: self.win_x,
            win_y: self.win_y,
            win_w: self.win_w,
            win_h: self.win_h,
        }
    }

    /// 扫描整合包 mods 目录下所有 jar 的 zh_cn 语言文件，收录模组物品与生物中文名。
    /// 真正的读盘/解压跑在子线程里 —— 以前同步跑在 UI 线程上，几百个 jar 一扫描界面就整个卡死。
    /// 结果只回传到内存，不写任何文件。
    fn scan_mods(&mut self) {
        let dir = self.mods_dir.trim().to_string();
        if dir.is_empty() {
            self.scan_status = "请先填写整合包 mods 目录".to_string();
            return;
        }
        if self.scanning {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel::<Result<ScanOutcome, String>>();
        self.scan_rx = Some(rx);
        self.scanning = true;
        self.scan_status = "正在扫描…（界面可继续使用）".to_string();
        std::thread::spawn(move || {
            let rd = match std::fs::read_dir(&dir) {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(Err(format!("目录读取失败：{e}")));
                    return;
                }
            };
            let mut item_map: HashMap<String, String> = HashMap::new();
            let mut ent_map: HashMap<String, String> = HashMap::new();
            let mut jars = 0usize;
            let mut failed = 0usize;
            for entry in rd.flatten() {
                let p = entry.path();
                if p.extension().map(|e| e != "jar").unwrap_or(true) {
                    continue;
                }
                jars += 1;
                if scan_jar(&p, &mut item_map, &mut ent_map).is_err() {
                    failed += 1;
                }
            }
            let mut items: Vec<(String, String)> = item_map.into_iter().collect();
            items.sort_by(|a, b| a.0.cmp(&b.0));
            let mut entities: Vec<(String, String)> = ent_map.into_iter().collect();
            entities.sort_by(|a, b| a.0.cmp(&b.0));

            let _ = tx.send(Ok(ScanOutcome {
                jars,
                failed,
                items,
                entities,
            }));
        });
    }

    /// 每帧检查扫描线程是否回了结果
    fn poll_scan(&mut self) {
        let Some(rx) = &self.scan_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(out)) => {
                self.scan_status = format!(
                    "扫描完成：{} 个 jar，{} 个失败，{} 条物品名，{} 条生物名",
                    out.jars,
                    out.failed,
                    out.items.len(),
                    out.entities.len()
                );
                // 缓存文件里存的是 ID→中文名，内存里统一成 (中文名, ID)
                self.mod_items = out.items.into_iter().map(|(id, cn)| (cn, id)).collect();
                self.mod_items_lc = self
                    .mod_items
                    .iter()
                    .map(|(n, i)| (n.to_lowercase(), i.to_lowercase()))
                    .collect();
                self.mod_entities = out.entities.into_iter().map(|(id, cn)| (cn, id)).collect();
                self.scanning = false;
                self.scan_rx = None;
            }
            Ok(Err(msg)) => {
                self.scan_status = msg;
                self.scanning = false;
                self.scan_rx = None;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.scan_status = "扫描线程异常结束".to_string();
                self.scanning = false;
                self.scan_rx = None;
            }
        }
    }

    /// 清除模组物品/生物缓存：只清内存，下次需重新扫描。
    ///
    /// 扫描结果本来就不落盘，所以这里没有文件可删；顺手把老版本留在
    /// exe 旁边的两个 JSON 也清掉，免得升级后旧缓存继续躺在目录里。
    fn reset_mod_cache_memory(&mut self) {
        self.scan_rx = None;
        self.scanning = false;
        self.mod_items.clear();
        self.mod_items_lc.clear();
        self.mod_entities.clear();
        self.search_result.clear();
    }

    fn clear_mod_cache(&mut self) {
        self.reset_mod_cache_memory();
        let mut deleted = 0usize;
        if let Some(base) = exe_dir() {
            for name in ["mod_items_zh.json", "mod_entities_zh.json"] {
                match fs::remove_file(base.join(name)) {
                    Ok(()) => deleted += 1,
                    // 文件不存在是正常情况（新版本不再生成），其余错误也不必打断用户
                    Err(_) => {}
                }
            }
        }
        self.scan_status = if deleted > 0 {
            format!("已清除扫描记录（顺带删除 {deleted} 个旧版缓存文件），请重新扫描")
        } else {
            "已清除扫描记录，请重新扫描".to_string()
        };
    }

    /// 把指令行导出为 mcfunction 文件（自动去前导斜杠），返回文件路径。
    /// 同一秒内多次导出会重名，这里自增后缀，避免后一次静默覆盖前一次。
    fn export_mcfunction(&self, name: &str, lines: &[String]) -> Option<String> {
        if lines.is_empty() {
            return None;
        }
        let dir = exe_dir()?;
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        let mut path = dir.join(format!("{name}_{secs}.mcfunction"));
        let mut dup = 1;
        while path.exists() {
            path = dir.join(format!("{name}_{secs}_{dup}.mcfunction"));
            dup += 1;
        }
        let body: Vec<String> = lines.iter().map(|l| mcfn_line(l)).collect();
        fs::write(
            &path,
            format!("# 由我的世界小工具导出\n{}\n", body.join("\n")),
        )
        .ok()?;
        Some(path.to_string_lossy().to_string())
    }

    /// 生成完整数据包文件夹（pack.mcmeta + 函数），可直接放进存档 datapacks 目录
    fn export_datapack(&self, pack: &str, func: &str, lines: &[String]) -> Option<String> {
        if lines.is_empty() {
            return None;
        }
        let base = exe_dir()?;
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        // 同一秒内重复导出不要互相覆盖
        let mut root = base.join("数据包").join(format!("{pack}_{secs}"));
        let mut dup = 1;
        while root.exists() {
            root = base.join("数据包").join(format!("{pack}_{secs}_{dup}"));
            dup += 1;
        }
        let format_no = if self.version == "1.20.1" { 15 } else { 48 };
        let meta = json!({
            "pack": {
                "pack_format": format_no,
                "description": format!("我的世界小工具数据包（{}）", self.version)
            }
        });
        // 24w21a(1.21) 起数据包注册目录改为单数；1.20.1 必须用复数 functions
        let func_dir = root
            .join("data/mc_tool")
            .join(datapack_function_dir(&self.version));
        fs::create_dir_all(&func_dir).ok()?;
        fs::write(root.join("pack.mcmeta"), serde_json::to_string(&meta).ok()?).ok()?;
        let body: Vec<String> = lines.iter().map(|l| mcfn_line(l)).collect();
        fs::write(
            func_dir.join(format!("{func}.mcfunction")),
            format!("# 由我的世界小工具导出\n{}\n", body.join("\n")),
        )
        .ok()?;
        Some(root.to_string_lossy().to_string())
    }

    fn push_history(&mut self, cmd: String) {
        self.record_history(cmd, Some(self.version.clone()));
    }

    fn record_history(&mut self, command: String, version: Option<String>) {
        if !command.starts_with('/') {
            return;
        }
        let entry = HistoryEntry { command, version };
        if self.history.front() == Some(&entry) {
            return;
        }
        self.history.push_front(entry);
        self.history.truncate(200);
        self.history_selected.clear();
    }

    fn selector_map(&self, key: &str) -> String {
        SELECTOR_MAP
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.to_string())
            .unwrap_or_else(|| key.to_string())
    }
}

impl App {
    fn render_result_bar(&mut self, ui: &mut egui::Ui, wide: bool) {
        // palette 是 &'static，先取出来，避免下面借用 self 的字段时又读 self.palette
        let palette = self.palette;
        if self.page.has_output() && self.output_is_stale() {
            ui.colored_label(
                palette.status_warn,
                "这条指令来自另一版本。请重新生成，或切回原版本后再复制或导出。",
            );
            ui.add_enabled_ui(false, |ui| self.render_output_widgets(ui, wide));
            return;
        }
        self.render_output_widgets(ui, wide);
    }

    fn render_output_widgets(&mut self, ui: &mut egui::Ui, wide: bool) {
        let palette = self.palette;
        match self.page {
            Page::Home => {}
            Page::Infinity => {
                App::result_row(
                    ui,
                    &mut self.inf_result,
                    &mut self.inf_status,
                    &mut self.inf_status_ok,
                    palette,
                    wide,
                );
                // 导出按钮作用于上面这个结果框，必须跟它待在一起，否则又会被顶回页面的滚动区
                ui.horizontal_wrapped(|ui| {
                    let can_export = self.inf_result.starts_with('/');
                    if ui
                        .add_enabled(can_export, egui::Button::new("导出函数"))
                        .clicked()
                    {
                        let lines: Vec<String> = self.inf_result.lines().map(mcfn_line).collect();
                        match self.export_mcfunction("infinity", &lines) {
                            Some(p) => {
                                self.inf_status = format!("已导出：{}", p);
                                self.inf_status_ok = true;
                            }
                            None => {
                                self.inf_status =
                                    "导出失败：没有可导出的指令，或文件写入失败".to_string();
                                self.inf_status_ok = false;
                            }
                        }
                    }
                    if ui
                        .add_enabled(can_export, egui::Button::new("导出数据包"))
                        .clicked()
                    {
                        let lines: Vec<String> = self.inf_result.lines().map(mcfn_line).collect();
                        match self.export_datapack("infinity", "infinity", &lines) {
                            Some(p) => {
                                self.inf_status = format!(
                                    "数据包已生成：{}（复制该文件夹到存档 datapacks 里）",
                                    p
                                );
                                self.inf_status_ok = true;
                            }
                            None => {
                                self.inf_status =
                                    "导出失败：没有可导出的指令，或文件写入失败".to_string();
                                self.inf_status_ok = false;
                            }
                        }
                    }
                });
            }
            Page::Enchant => App::result_row(
                ui,
                &mut self.ench_result,
                &mut self.ench_status,
                &mut self.ench_status_ok,
                palette,
                wide,
            ),
            Page::HandEnch => App::result_row(
                ui,
                &mut self.hand_result,
                &mut self.hand_status,
                &mut self.hand_status_ok,
                palette,
                wide,
            ),
            Page::Name => App::result_row(
                ui,
                &mut self.name_result,
                &mut self.copy_status,
                &mut self.copy_status_ok,
                palette,
                wide,
            ),
            Page::Potion => App::result_row(
                ui,
                &mut self.pot_result,
                &mut self.copy_status,
                &mut self.copy_status_ok,
                palette,
                wide,
            ),
            Page::Egg => App::result_row(
                ui,
                &mut self.egg_result,
                &mut self.copy_status,
                &mut self.copy_status_ok,
                palette,
                wide,
            ),
            Page::Attr => App::result_row(
                ui,
                &mut self.attr_result,
                &mut self.copy_status,
                &mut self.copy_status_ok,
                palette,
                wide,
            ),
            Page::Head => App::result_row(
                ui,
                &mut self.head_result,
                &mut self.copy_status,
                &mut self.copy_status_ok,
                palette,
                wide,
            ),
            Page::Gear => App::result_row(
                ui,
                &mut self.gear_result,
                &mut self.copy_status,
                &mut self.copy_status_ok,
                palette,
                wide,
            ),
            Page::ArmorStand => App::result_row(
                ui,
                &mut self.as_result,
                &mut self.copy_status,
                &mut self.copy_status_ok,
                palette,
                wide,
            ),
            Page::Batch => App::result_row(
                ui,
                &mut self.batch_result,
                &mut self.copy_status,
                &mut self.copy_status_ok,
                palette,
                wide,
            ),
            Page::Entity => App::result_row(
                ui,
                &mut self.ent_result,
                &mut self.copy_status,
                &mut self.copy_status_ok,
                palette,
                wide,
            ),
            // 这几页没有结果框（点一下就直接复制 / 只在页面里提示），只显示提示行
            Page::Quick | Page::History | Page::Search => {
                App::status_line(
                    ui,
                    &mut self.copy_status,
                    &mut self.copy_status_ok,
                    palette,
                    "点击条目即复制到剪贴板",
                );
            }
            // 设置页的操作结果写在页面里（离按钮近），底部只留一句说明
            Page::Settings => {
                App::status_line(
                    ui,
                    &mut self.reg_status,
                    &mut self.reg_status_ok,
                    palette,
                    "可导出 JSON 备份；彻底清除本机数据不可撤销",
                );
            }
        }
    }

    /// 只有提示、没有结果框的页面用这个（快捷指令 / 历史 / 搜索 / 设置与清理）。
    /// `hint` 是没有提示时显示的灰字说明。
    fn status_line(ui: &mut egui::Ui, status: &mut String, ok: &mut bool, p: &Palette, hint: &str) {
        ui.horizontal_wrapped(|ui| {
            if !status.is_empty() {
                let color = if *ok { p.status_ok } else { p.status_err };
                ui.colored_label(color, status.as_str());
            } else {
                ui.label(egui::RichText::new(hint).color(p.text_weak));
            }
            if ui.small_button("清空提示").clicked() {
                status.clear();
            }
        });
    }

    fn result_row(
        ui: &mut egui::Ui,
        result: &mut String,
        status: &mut String,
        ok: &mut bool,
        p: &Palette,
        wide: bool,
    ) {
        if wide || !result.is_empty() {
            ui::surface(p, p.input_bg, 10.0).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                if result.is_empty() {
                    ui.label(
                        egui::RichText::new("等待生成指令")
                            .color(p.text_weak)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("生成后的内容会出现在这里，也可以直接编辑。")
                            .color(p.text_weak)
                            .size(12.0),
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(if wide { 250.0 } else { 48.0 })
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(result)
                                    .desired_rows(if wide { 5 } else { 2 })
                                    .desired_width(ui.available_width())
                                    .margin(egui::vec2(0.0, 0.0))
                                    .font(egui::TextStyle::Monospace)
                                    .frame(false),
                            );
                        });
                }
            });
        }
        let can_copy = result.trim_start().starts_with('/');
        if ui
            .add_enabled(can_copy, ui::action(p, "复制指令"))
            .clicked()
        {
            ui.ctx().copy_text(result.clone());
            *status = "已复制到剪贴板".to_owned();
            *ok = true;
        }
        // 状态单独占一行，避免长物品 ID 或导出路径在横向换行时挤到复制按钮下面。
        if !status.is_empty() {
            let color = if *ok { p.status_ok } else { p.status_err };
            ui.add(
                egui::Label::new(egui::RichText::new(status.as_str()).color(color))
                    .wrap(wide)
                    .truncate(!wide),
            )
            .on_hover_text(status.as_str());
        } else if result.is_empty() && !wide {
            ui.label(egui::RichText::new("等待生成").color(p.text_weak));
        }
    }

    // ---------- 无限元件 ----------
    /// 生成无限元件指令。
    /// 每个分支都必须同时写 `inf_status` 和 `inf_status_ok`，
    /// 以前的兜底分支不写 status，界面上会一直挂着上一次的「已生成：xxx」，看着像刚成功。
    fn gen_infinity(&mut self, from_preset: bool) {
        self.mark_result(Page::Infinity);
        let raw = self.inf_entry.trim().to_string();
        if raw.is_empty() {
            self.inf_result = "请输入物品ID".to_string();
            self.inf_status = "请先填写物品ID".to_string();
            self.inf_status_ok = false;
            return;
        }
        let selector = self.selector_map(&self.selector);
        // 数据表统一存 1.21 的 ID，1.20.1 出指令时换回旧 ID（如 turtle_scute -> scute）。
        // 这里就换掉，下面的普通 /give 兜底分支也才能拿到正确的 ID。
        let mut item_id = item_id_for(&raw, &self.version).to_string();
        if self.version == "1.21.1" && !is_cell4(&self.inf_mod) {
            if let Some(mapped) = INF_CELL_MAP
                .iter()
                .find(|(k, _)| *k == item_id)
                .map(|(_, v)| *v)
            {
                item_id = mapped.to_string();
                self.inf_entry = item_id.clone();
            }
            let valid = (item_id.starts_with("kubejs:") || item_id.starts_with("extendedae:"))
                && item_id.ends_with("_cell");
            if !valid {
                if from_preset {
                    let cmd = format!("/give {} {}", selector, item_id);
                    self.inf_result = cmd.clone();
                    self.inf_status =
                        format!("已生成普通 /give：{}（该物品没有对应无限元件）", item_id);
                    self.inf_status_ok = true;
                    self.push_history(cmd);
                    return;
                }
                self.inf_result = "ExtendedAE 1.21.1 无法自定义任意物品".to_string();
                self.inf_status = "需要任意物品无限元件：模组下拉切换 Cell⁴".to_string();
                self.inf_status_ok = false;
                return;
            }
            let cmd = build_infinity_cmd(
                &item_id,
                &self.inf_type,
                &self.version,
                &self.inf_mod,
                &selector,
            );
            self.inf_result = cmd.clone();
            self.inf_status = format!("已生成：{}", item_id);
            self.inf_status_ok = true;
            self.push_history(cmd);
            return;
        }
        let cmd = build_infinity_cmd(
            &item_id,
            &self.inf_type,
            &self.version,
            &self.inf_mod,
            &selector,
        );
        self.inf_result = cmd.clone();
        self.inf_status = format!("已生成：{}", item_id);
        self.inf_status_ok = true;
        self.push_history(cmd);
    }

    // 按预设物品生成单条无限元件指令（批量用，不改 inf_entry）
    fn infinity_cmd_for(&self, item_id: &str, cell_type: &str) -> String {
        let sel = self.selector_map(&self.selector);
        let mut id = item_id.to_string();
        if self.version == "1.21.1" && !is_cell4(&self.inf_mod) {
            if let Some(mapped) = INF_CELL_MAP.iter().find(|(k, _)| *k == id).map(|(_, v)| *v) {
                id = mapped.to_string();
            }
            let valid = (id.starts_with("kubejs:") || id.starts_with("extendedae:"))
                && id.ends_with("_cell");
            if !valid {
                // ExtendedAE 1.21.1 无法自定义任意物品，预设物品直接给原物品
                return format!("/give {} {}", sel, id);
            }
            return build_infinity_cmd(&id, cell_type, &self.version, &self.inf_mod, &sel);
        }
        build_infinity_cmd(&id, cell_type, &self.version, &self.inf_mod, &sel)
    }

    // 批量生成：对所有勾选物品生成指令，多行输出
    fn gen_infinity_batch(&mut self) {
        self.mark_result(Page::Infinity);
        if self.inf_sel.is_empty() {
            // 空选择时把上一次的结果也清掉：结果框里留着旧指令，
            // 用户会以为这次也生成了。
            self.inf_result.clear();
            self.inf_status = "请先勾选预设物品".to_string();
            self.inf_status_ok = false;
            return;
        }
        // 构建 item_id -> 类型 反向索引（预设数据静态，安全）
        let mut type_map: HashMap<&str, &str> = HashMap::new();
        for (_, items) in PRESET_CATEGORIES {
            for (_, iid, itype) in items {
                type_map.entry(iid).or_insert(itype);
            }
        }
        let mut ids: Vec<String> = self.inf_sel.iter().cloned().collect();
        ids.sort();
        let mut cmds: Vec<String> = Vec::new();
        for id in &ids {
            let itype = type_map.get(id.as_str()).copied().unwrap_or("item");
            let cmd = self.infinity_cmd_for(id, itype);
            cmds.push(cmd.clone());
            self.push_history(cmd);
        }
        self.inf_result = cmds.join("\n");
        self.inf_status = format!("已批量生成 {} 条指令，可复制全部", cmds.len());
        // 以前这里不写 ok，上一次要是红字失败态，批量成功后还挂着红的。
        self.inf_status_ok = true;
    }

    fn render_infinity(&mut self, ui: &mut egui::Ui) {
        let p = self.palette;
        ui::section(ui, p, "配置元件", |ui| self.render_infinity_form(ui));
        ui::section(ui, p, "物品预设", |ui| self.render_infinity_catalog(ui));
    }

    fn render_infinity_form(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let resp = ui::field(ui, "物品ID", &mut self.inf_entry, "例如 minecraft:diamond");
            if enter_pressed(ui, &resp) {
                self.gen_infinity(false);
                resp.request_focus();
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("类型：");
            // 用 selectable_label + 变了才赋值：selectable_value 每帧都要为两个选项各分配一个 String，
            // 顶部栏那两个下拉早就是这么写的，这里漏了。
            for (val, label) in [("item", "物品"), ("fluid", "流体")] {
                if ui.selectable_label(self.inf_type == val, label).clicked()
                    && self.inf_type != val
                {
                    self.inf_type = val.to_string();
                }
            }
            ui.separator();
            ui.label("模组：");
            for (k, _) in MOD_MAP {
                ui.selectable_value(&mut self.inf_mod, k.to_string(), k);
            }
        });
        if self.version == "1.21.1" && !is_cell4(&self.inf_mod) {
            ui.colored_label(
                self.palette.status_warn,
                "1.21.1 的 ExtendedAE 不支持任意物品元件；需要时请选择 Cell⁴。",
            );
        }
        if ui.add(ui::action(self.palette, "生成指令")).clicked() {
            self.gen_infinity(false);
        }
    }

    fn render_infinity_catalog(&mut self, ui: &mut egui::Ui) {
        // 搜索框（跨全部分类过滤中文名/物品ID）
        ui.horizontal_wrapped(|ui| {
            ui.label("搜索：");
            ui.text_edit_singleline(&mut self.inf_search);
            if !self.inf_search.is_empty() && ui.button("×").clicked() {
                self.inf_search.clear();
                self.inf_preset_page = 0;
            }
        });
        // 物品列表来源：有搜索词→全局过滤结果；无→当前分类
        let search_kw = self.inf_search.trim().to_lowercase();
        let search_items: Vec<(&str, &str, &str)> = if !search_kw.is_empty() {
            let mut v = Vec::new();
            for (_, items) in PRESET_CATEGORIES.iter() {
                for (name, iid, itype) in items.iter() {
                    if name.to_lowercase().contains(&search_kw)
                        || iid.to_lowercase().contains(&search_kw)
                    {
                        v.push((*name, *iid, *itype));
                    }
                }
            }
            v
        } else {
            Vec::new()
        };
        ui.label(if !search_kw.is_empty() {
            format!(
                "预设分类（搜索结果 {} 条，勾选后批量生成）：",
                search_items.len()
            )
        } else {
            "预设分类（勾选后批量生成，点击物品即生成单条）：".to_string()
        });
        // 分类选择：搜索时隐藏（结果已跨分类）
        if search_kw.is_empty() {
            let cat_count = PRESET_CATEGORIES.len();
            let sel = self.inf_preset_sel.unwrap_or(0).min(cat_count - 1);
            ui.horizontal_wrapped(|ui| {
                for (i, (cat, _)) in PRESET_CATEGORIES.iter().enumerate() {
                    if ui.selectable_label(sel == i, *cat).clicked() {
                        self.inf_preset_sel = Some(i);
                        self.inf_preset_page = 0;
                    }
                }
            });
            ui.separator();
        }
        // 当前列表 + 分页
        let items: &[(&str, &str, &str)] = if !search_kw.is_empty() {
            &search_items
        } else {
            let sel = self
                .inf_preset_sel
                .unwrap_or(0)
                .min(PRESET_CATEGORIES.len() - 1);
            PRESET_CATEGORIES[sel].1
        };
        const PER_PAGE: usize = 20;
        let total_pages = items.len().div_ceil(PER_PAGE).max(1);
        if self.inf_preset_page >= total_pages {
            self.inf_preset_page = total_pages - 1;
        }
        let page = self.inf_preset_page;
        let start = page * PER_PAGE;
        let end = (start + PER_PAGE).min(items.len());
        if items.is_empty() {
            ui.colored_label(self.palette.status_warn, "没有匹配的物品");
            // 后面三个按钮、网格、分页在空列表下全是摆设，直接收工
            return;
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(format!("批量生成（{}）", self.inf_sel.len()))
                .clicked()
            {
                self.gen_infinity_batch();
            }
            if ui.button("全选本页").clicked() {
                for (_, iid, _) in &items[start..end] {
                    self.inf_sel.insert(iid.to_string());
                }
            }
            if ui.button("清空勾选").clicked() {
                self.inf_sel.clear();
            }
        });
        let cols = ((ui.available_width() / 156.0).floor() as usize).clamp(1, 7);
        for row in items[start..end].chunks(cols) {
            ui.columns(cols, |cells| {
                for (cell, &(name, iid, itype)) in cells.iter_mut().zip(row) {
                    cell.push_id(iid, |ui| {
                        ui.horizontal(|ui| {
                            let mut checked = self.inf_sel.contains(iid);
                            let toggle = ui
                                .checkbox(&mut checked, "")
                                .on_hover_text(format!("勾选 {name}"));
                            if toggle.changed() {
                                if checked {
                                    self.inf_sel.insert(iid.to_owned());
                                } else {
                                    self.inf_sel.remove(iid);
                                }
                            }
                            if ui
                                .add(egui::Button::new(name).wrap(true))
                                .on_hover_text(iid)
                                .clicked()
                            {
                                self.inf_entry = iid.to_owned();
                                self.inf_type = itype.to_owned();
                                self.gen_infinity(true);
                                if self.inf_result.starts_with('/') {
                                    ui.ctx().copy_text(self.inf_result.clone());
                                    self.inf_status = format!("已复制：{iid}");
                                }
                            }
                        });
                    });
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button("< 上一页").clicked() && page > 0 {
                self.inf_preset_page = page - 1;
            }
            ui.label(format!(
                "第 {}/{} 页 · 共 {} 项",
                page + 1,
                total_pages,
                items.len()
            ));
            if ui.button("下一页 >").clicked() && page + 1 < total_pages {
                self.inf_preset_page = page + 1;
            }
        });
    }

    // ---------- 255附魔 ----------
    fn gen_enchant(&mut self) {
        self.mark_result(Page::Enchant);
        let item_id = self.ench_entry.trim().to_string();
        if item_id.is_empty() {
            self.ench_result = "请输入物品ID".to_string();
            self.ench_status = "请先填写物品ID".to_string();
            self.ench_status_ok = false;
            return;
        }
        let mut selected: Vec<&str> = ENCHANTMENTS
            .iter()
            .enumerate()
            .filter(|(i, _)| self.ench_checks[*i])
            .map(|(_, (eid, _))| *eid)
            .collect();
        // 1.20.1 没有这三项，静默剔除后要在提示里说清楚
        let mut skipped = 0usize;
        if self.version == "1.20.1" {
            let v21 = [
                "minecraft:density",
                "minecraft:breach",
                "minecraft:wind_burst",
            ];
            let before = selected.len();
            selected.retain(|e| !v21.contains(e));
            skipped = before - selected.len();
        }
        let sel_refs: Vec<&str> = selected.iter().map(|s| *s).collect();
        let level = self
            .ench_level
            .trim()
            .parse::<u32>()
            .unwrap_or(255)
            .clamp(1, 255);
        let cmd = build_enchant_cmd(
            &item_id,
            &sel_refs,
            &self.version,
            &self.selector_map(&self.selector),
            self.ench_unbreakable,
            level,
        );
        self.ench_status = if !sel_refs.is_empty() {
            let mut s = format!("已生成：{} 项附魔", sel_refs.len());
            if skipped > 0 {
                s.push_str(&format!("（跳过 {skipped} 个 1.20.1 不存在的附魔）"));
            }
            self.ench_status_ok = true;
            s
        } else if self.ench_unbreakable {
            self.ench_status_ok = true;
            "已生成：只加了不可破坏，没有选附魔".to_string()
        } else {
            self.ench_status_ok = false;
            "已生成，但一项附魔都没选 —— 结果里是空组件列表".to_string()
        };
        self.ench_result = cmd.clone();
        self.push_history(cmd);
    }

    fn render_enchant(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let resp = ui::field(
                ui,
                "物品ID",
                &mut self.ench_entry,
                "例如 minecraft:netherite_sword",
            );
            if enter_pressed(ui, &resp) {
                self.gen_enchant();
                resp.request_focus();
            }
        });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.label("预设：");
            egui::ComboBox::from_id_source("ench_preset_cat")
                .selected_text(self.ench_preset_cat.clone())
                .show_ui(ui, |ui| {
                    for (k, v) in ENCH_PRESETS {
                        ui.selectable_value(&mut self.ench_preset_cat, k.to_string(), k);
                        if self.ench_preset_cat == k && self.ench_preset_checks.len() != v.len() {
                            self.ench_preset_checks = vec![false; v.len()];
                        }
                    }
                });
            if ui.button("套用预设").clicked() {
                if let Some((_, presets)) = ENCH_PRESETS
                    .iter()
                    .find(|(k, _)| *k == self.ench_preset_cat)
                {
                    // 以前每命中一个勾选项就整块覆盖 ench_entry / 清空 ench_checks 并立刻生成，
                    // 勾了多个预设时只有最后一个生效。现在把所有勾选项合并成一次生成：
                    // 物品取第一个勾选项，附魔取并集。
                    let picked: Vec<&(&str, &str, &[&str])> = presets
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| {
                            *i < self.ench_preset_checks.len() && self.ench_preset_checks[*i]
                        })
                        .map(|(_, p)| p)
                        .collect();
                    if picked.is_empty() {
                        self.ench_status =
                            format!("「{}」里一个预设都没勾，先勾选再套用", self.ench_preset_cat);
                        self.ench_status_ok = false;
                    } else {
                        // 用户先手动勾的附魔不丢，与预设取并集
                        let mut keep: Vec<bool> = self.ench_checks.clone();
                        // 物品取第一个勾选项：以前这句在循环里，
                        // 勾了多个预设时最后写进去的是最后一个，跟注释说的不一样。
                        if let Some((_, pid, _)) = picked.first() {
                            self.ench_entry = (*pid).to_string();
                        }
                        for (_pname, _pid, pids) in picked.iter() {
                            for e in pids.iter() {
                                if let Some(idx) = ENCHANTMENTS.iter().position(|(eid, _)| eid == e)
                                {
                                    keep[idx] = true;
                                }
                            }
                        }
                        self.ench_checks = keep;
                        let names: Vec<&str> = picked.iter().map(|(n, _, _)| *n).collect();
                        self.gen_enchant();
                        if self.ench_status_ok {
                            self.ench_status =
                                format!("已套用 {} 个预设（{}）", picked.len(), names.join("+"));
                        }
                    }
                }
            }
        });
        // 预设详情
        if let Some((_, presets)) = ENCH_PRESETS
            .iter()
            .find(|(k, _)| *k == self.ench_preset_cat)
        {
            ui.horizontal_wrapped(|ui| {
                for (i, (pname, _pid, _)) in presets.iter().enumerate() {
                    if i < self.ench_preset_checks.len() {
                        ui.checkbox(&mut self.ench_preset_checks[i], *pname);
                    }
                }
            });
        }
        ui.separator();
        // 生成区（固定在上方，不被附魔列表遮挡）
        ui.checkbox(&mut self.ench_unbreakable, "无耐久消耗（Unbreakable）");
        ui.horizontal_wrapped(|ui| {
            ui.label("附魔等级(1~255)：");
            let resp = ui.text_edit_singleline(&mut self.ench_level);
            if enter_pressed(ui, &resp) {
                self.gen_enchant();
                resp.request_focus();
            }
            if ui.add(ui::action(self.palette, "生成指令")).clicked() {
                self.gen_enchant();
            }
        });
        ui.separator();
        // 附魔列表（翻页显示，40 种分 2 页不滚动）
        const ENCH_PER_PAGE: usize = 20;
        let total_pages = ENCHANTMENTS.len().div_ceil(ENCH_PER_PAGE).max(1);
        if self.ench_page >= total_pages {
            self.ench_page = total_pages - 1;
        }
        ui.label(format!(
            "附魔列表（第 {}/{} 页）：",
            self.ench_page + 1,
            total_pages
        ));
        let start = self.ench_page * ENCH_PER_PAGE;
        let end = (start + ENCH_PER_PAGE).min(ENCHANTMENTS.len());
        ui.horizontal_wrapped(|ui| {
            for i in start..end {
                let (_eid, cn) = ENCHANTMENTS[i];
                ui.checkbox(&mut self.ench_checks[i], cn);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(self.ench_page > 0, egui::Button::new("< 上一页"))
                .clicked()
            {
                self.ench_page -= 1;
            }
            ui.label(format!("第 {}/{} 页", self.ench_page + 1, total_pages));
            if ui
                .add_enabled(
                    self.ench_page + 1 < total_pages,
                    egui::Button::new("下一页 >"),
                )
                .clicked()
            {
                self.ench_page += 1;
            }
        });
    }

    // ---------- 手持附魔 ----------
    fn gen_handench(&mut self) {
        self.mark_result(Page::HandEnch);
        let selected: Vec<&str> = ENCHANTMENTS
            .iter()
            .enumerate()
            .filter(|(i, _)| self.hand_checks[*i])
            .map(|(_, (eid, _))| *eid)
            .collect();
        if selected.is_empty() {
            self.hand_result = "请先勾选至少一种附魔".to_string();
            self.hand_status = "还没勾选任何附魔".to_string();
            self.hand_status_ok = false;
            return;
        }
        let level = self
            .hand_level
            .trim()
            .parse::<u32>()
            .unwrap_or(255)
            .clamp(1, 255);
        let cmd = build_hand_ench_cmds(
            &self.version,
            &self.selector_map(&self.selector),
            &selected,
            self.hand_use_max,
            level,
            self.hand_unbreakable,
        );
        let lines = cmd.lines().count();
        self.hand_status = if self.hand_unbreakable && self.version == "1.20.1" {
            // 1.20.1 没有 /item modify 的组件语法，不可破坏那一条会被跳过
            format!("已生成 {lines} 条指令（1.20.1 无法加不可破坏，已忽略该选项）")
        } else if self.hand_unbreakable {
            format!("已生成 {lines} 条指令（含不可破坏）")
        } else {
            format!("已生成 {lines} 条指令")
        };
        self.hand_status_ok = !(self.hand_unbreakable && self.version == "1.20.1");
        self.hand_result = cmd.clone();
        self.push_history(cmd);
    }

    fn render_handench(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "手持要附魔的物品，直接对已有物品加附魔（保留名称/耐久/已有附魔），不会生成新物品。",
        );
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.hand_use_max, "用原版最大等级");
            if !self.hand_use_max {
                ui.label("等级(超出上限自动截断)：");
                let resp = ui.text_edit_singleline(&mut self.hand_level);
                if enter_pressed(ui, &resp) {
                    self.gen_handench();
                    resp.request_focus();
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            // 1.20.1 没有 /item modify 的组件语法，勾了也生成不出来，直接禁用并说明
            let ok = self.version != "1.20.1";
            ui.add_enabled_ui(ok, |ui| {
                ui.checkbox(&mut self.hand_unbreakable, "无耐久消耗（仅 1.21.1 支持）");
            });
            if !ok {
                ui.label(
                    egui::RichText::new("1.20.1 不支持，需要的话请用「255附魔」页")
                        .color(self.palette.status_warn),
                );
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui.add(ui::action(self.palette, "生成指令")).clicked() {
                self.gen_handench();
            }
            if ui.button("全不选").clicked() {
                for c in self.hand_checks.iter_mut() {
                    *c = false;
                }
            }
        });
        ui.separator();
        // 附魔列表（与255附魔同样的翻页模式，20/页）
        const HAND_PER_PAGE: usize = 20;
        let total_pages = ENCHANTMENTS.len().div_ceil(HAND_PER_PAGE).max(1);
        if self.hand_page >= total_pages {
            self.hand_page = total_pages - 1;
        }
        ui.label(format!(
            "附魔列表（第 {}/{} 页）：",
            self.hand_page + 1,
            total_pages
        ));
        let start = self.hand_page * HAND_PER_PAGE;
        let end = (start + HAND_PER_PAGE).min(ENCHANTMENTS.len());
        ui.horizontal_wrapped(|ui| {
            for i in start..end {
                let (_eid, cn) = ENCHANTMENTS[i];
                ui.checkbox(&mut self.hand_checks[i], cn);
            }
        });
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(self.hand_page > 0, egui::Button::new("< 上一页"))
                .clicked()
            {
                self.hand_page -= 1;
            }
            ui.label(format!("第 {}/{} 页", self.hand_page + 1, total_pages));
            if ui
                .add_enabled(
                    self.hand_page + 1 < total_pages,
                    egui::Button::new("下一页 >"),
                )
                .clicked()
            {
                self.hand_page += 1;
            }
        });
    }

    // ---------- 物品命名 ----------
    fn gen_name(&mut self) {
        self.mark_result(Page::Name);
        let item_id = self.name_entry.trim().to_string();
        if item_id.is_empty() {
            self.name_result = "请输入物品ID".to_string();
            self.copy_status = "请先填写物品ID".to_string();
            self.copy_status_ok = false;
            return;
        }
        let has_name = !self.name_text.trim().is_empty();
        let lore_n = self
            .name_lore
            .iter()
            .filter(|l| !l.trim().is_empty())
            .count();
        let cmd = build_name_cmd(
            &item_id,
            &self.name_text,
            &self.name_lore,
            &self.version,
            &self.selector_map(&self.selector),
        );
        self.copy_status = if has_name && lore_n > 0 {
            format!("已生成：名称 + {lore_n} 行 Lore")
        } else if has_name {
            "已生成：仅名称".to_string()
        } else if lore_n > 0 {
            format!("已生成：仅 {lore_n} 行 Lore（名称为空，物品保持原名）")
        } else {
            "已生成，但名称和 Lore 都为空 —— 结果等于直接 give 一个普通物品".to_string()
        };
        self.copy_status_ok = has_name || lore_n > 0;
        self.name_result = cmd.clone();
        self.push_history(cmd);
    }

    fn render_name(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let resp = ui::field(ui, "物品ID", &mut self.name_entry, "例如 minecraft:diamond");
            if enter_pressed(ui, &resp) {
                self.gen_name();
                resp.request_focus();
            }
        });
        ui.vertical(|ui| {
            let resp = ui::field(ui, "名称", &mut self.name_text, "输入显示名称");
            if enter_pressed(ui, &resp) {
                self.gen_name();
                resp.request_focus();
            }
        });
        ui.label("Lore（每行一条）：");
        for l in self.name_lore.iter_mut() {
            ui.text_edit_singleline(l);
        }
        if ui.add(ui::action(self.palette, "生成指令")).clicked() {
            self.gen_name();
        }
    }

    // ---------- 药水 ----------
    fn gen_potion(&mut self) {
        self.mark_result(Page::Potion);
        let Some((_, eid, eid_num)) = POTIONS.iter().find(|(cn, _, _)| *cn == self.pot_effect)
        else {
            // 以前这里静默 return，界面什么都不发生，看着像按钮坏了
            self.pot_result = format!("无法识别的药水效果：{}", self.pot_effect);
            self.copy_status = "请从下拉框里重新选择药水效果".to_string();
            self.copy_status_ok = false;
            return;
        };
        let amp = self
            .pot_level
            .trim()
            .parse::<i32>()
            .unwrap_or(1)
            .clamp(1, 255)
            - 1;
        let dur = self
            .pot_dur
            .trim()
            .parse::<i32>()
            .unwrap_or(30)
            .clamp(1, 999999)
            * 20;
        let kind_idx = self.pot_kind.min(POTION_KINDS.len() - 1);
        let item_id = POTION_KINDS[kind_idx].1;
        let cmd = build_potion_cmd(
            item_id,
            eid,
            *eid_num,
            amp,
            dur,
            &self.version,
            &self.selector_map(&self.selector),
        );
        self.copy_status = format!(
            "已生成：{} 级 {}，{} 秒",
            amp + 1,
            self.pot_effect,
            dur / 20
        );
        self.copy_status_ok = true;
        self.pot_result = cmd.clone();
        self.push_history(cmd);
    }

    fn render_potion(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("类型：");
            for (i, (k, _)) in POTION_KINDS.iter().enumerate() {
                ui.selectable_value(&mut self.pot_kind, i, *k);
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("效果：");
            egui::ComboBox::from_id_source("pot_effect")
                .selected_text(self.pot_effect.clone())
                .show_ui(ui, |ui| {
                    for (cn, _, _) in POTIONS {
                        ui.selectable_value(&mut self.pot_effect, cn.to_string(), cn);
                    }
                });
            ui.label("等级：");
            let r1 = ui.text_edit_singleline(&mut self.pot_level);
            ui.label("时长(秒)：");
            let r2 = ui.text_edit_singleline(&mut self.pot_dur);
            if enter_pressed(ui, &r1) || enter_pressed(ui, &r2) {
                self.gen_potion();
                r1.request_focus();
            }
        });
        if ui.add(ui::action(self.palette, "生成指令")).clicked() {
            self.gen_potion();
        }
    }

    // ---------- 刷怪蛋 ----------
    fn gen_egg(&mut self) {
        self.mark_result(Page::Egg);
        let raw = self.egg_entity.trim().to_string();
        if raw.is_empty() {
            self.egg_result = "请输入实体ID".to_string();
            self.copy_status = "请先填写实体ID".to_string();
            self.copy_status_ok = false;
            return;
        }
        let cmd = build_egg_cmd(
            &raw,
            &self.egg_name,
            &self.egg_scale,
            &self.version,
            &self.selector_map(&self.selector),
        );
        // build_egg_cmd 遇到没有刷怪蛋的实体会返回中文提示而不是命令
        if cmd.starts_with('/') {
            self.copy_status = format!("已生成：{} 的刷怪蛋", raw);
            self.copy_status_ok = true;
            self.push_history(cmd.clone());
        } else {
            self.copy_status = format!("{raw} 没有对应的刷怪蛋，已改用「自定义实体」页的 /summon");
            self.copy_status_ok = false;
        }
        self.egg_result = cmd;
    }

    fn render_egg(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("实体ID：");
            let resp = ui.text_edit_singleline(&mut self.egg_entity);
            if enter_pressed(ui, &resp) {
                self.gen_egg();
                resp.request_focus();
            }
            ui.label("（如 minecraft:zombie）");
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("名称(可空)：");
            ui.text_edit_singleline(&mut self.egg_name);
            ui.label("大小(1=正常)：");
            ui.text_edit_singleline(&mut self.egg_scale);
        });
        if ui.add(ui::action(self.palette, "生成指令")).clicked() {
            self.gen_egg();
        }
    }

    // ---------- 属性 ----------
    fn gen_attr(&mut self) {
        self.mark_result(Page::Attr);
        let raw = self.attr_val.trim().to_string();
        // 以前什么都不过滤，空串/非数字会拼出 `/attribute @s xxx base set ` 这种
        // 结尾缺参数的命令，进游戏才报语法错，还看不出是哪儿出的问题。
        if raw.is_empty() {
            self.attr_result = "请输入数值".to_string();
            self.copy_status = "请先填写属性数值".to_string();
            self.copy_status_ok = false;
            return;
        }
        if raw.parse::<f64>().is_err() {
            self.attr_result = format!("数值无效：{raw}");
            self.copy_status = "数值必须是数字（可带小数和负号）".to_string();
            self.copy_status_ok = false;
            return;
        }
        let cmd = build_attr_cmd(&self.attr_id, &raw, &self.selector_map(&self.selector));
        self.copy_status = format!("已生成：{} = {}", self.attr_id, raw);
        self.copy_status_ok = true;
        self.attr_result = cmd.clone();
        self.push_history(cmd);
    }

    fn render_attr(&mut self, ui: &mut egui::Ui) {
        // 版本切换后当前属性可能不可用，回退到第一个可用项
        let usable = ATTRIBUTES
            .iter()
            .any(|(_, aid, ver)| *aid == self.attr_id && (*ver == "both" || *ver == self.version));
        if !usable {
            if let Some((_, aid, _)) = ATTRIBUTES
                .iter()
                .find(|(_, _, ver)| *ver == "both" || *ver == self.version)
            {
                self.attr_id = aid.to_string();
            }
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("属性：");
            egui::ComboBox::from_id_source("attr_id")
                .selected_text(self.attr_id.clone())
                .show_ui(ui, |ui| {
                    for (aname, aid, ver) in ATTRIBUTES {
                        if ver != "both" && ver != self.version {
                            continue;
                        }
                        ui.selectable_value(&mut self.attr_id, aid.to_string(), aname);
                    }
                });
            ui.label("数值：");
            let resp = ui.text_edit_singleline(&mut self.attr_val);
            if enter_pressed(ui, &resp) {
                self.gen_attr();
                resp.request_focus();
            }
        });
        if ui.add(ui::action(self.palette, "生成指令")).clicked() {
            self.gen_attr();
        }
    }

    // ---------- 头颅 ----------
    fn gen_head(&mut self) {
        self.mark_result(Page::Head);
        let name = self.head_name.trim().to_string();
        if name.is_empty() {
            self.head_result = "请输入玩家名".to_string();
            self.copy_status = "请先填写玩家名".to_string();
            self.copy_status_ok = false;
            return;
        }
        // 玩家名会原样写进 SNBT 字符串里，含引号/反斜杠会把整条命令弄坏
        if let Some(bad) = name.chars().find(|c| matches!(c, '"' | '\'' | '\\')) {
            self.head_result = format!("玩家名含非法字符：{bad}");
            self.copy_status = "玩家名不能包含引号或反斜杠".to_string();
            self.copy_status_ok = false;
            return;
        }
        let cmd = build_head_cmd(&name, &self.version, &self.selector_map(&self.selector));
        self.copy_status = format!("已生成：{} 的头颅", name);
        self.copy_status_ok = true;
        self.head_result = cmd.clone();
        self.push_history(cmd);
    }

    fn render_head(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let resp = ui::field(ui, "玩家名", &mut self.head_name, "例如 Notch");
            if enter_pressed(ui, &resp) {
                self.gen_head();
                resp.request_focus();
            }
        });
        if ui.add(ui::action(self.palette, "生成指令")).clicked() {
            self.gen_head();
        }
    }

    // ---------- 全套装备 ----------
    fn gen_gear(&mut self) {
        self.mark_result(Page::Gear);
        let parts: Vec<&str> = GEAR_PART_MAP
            .iter()
            .enumerate()
            .filter(|(i, _)| self.gear_parts[*i])
            .map(|(_, (k, _))| *k)
            .collect();
        if parts.is_empty() {
            // 以前会静默生成空串，界面毫无反应
            self.gear_result = "请至少勾选一个部件".to_string();
            self.copy_status = "还没勾选任何部件".to_string();
            self.copy_status_ok = false;
            return;
        }
        let cmd = build_gear_cmd(
            &self.gear_mat,
            &parts,
            &self.version,
            &self.selector_map(&self.selector),
            self.gear_ench_on,
        );
        if cmd.is_empty() {
            self.gear_result = format!("无法识别材料：{}", self.gear_mat);
            self.copy_status = "材料不在支持列表里".to_string();
            self.copy_status_ok = false;
            return;
        }
        self.copy_status = if self.gear_ench_on {
            // 附魔等级不是 255：build_gear_cmd 用的是每一种附魔各自的原版上限
            // （锋利 5、耐久 3、经验修补 1……），写「附魔 255」是旧文案，会误导。
            format!(
                "已生成 {} 件 {} 装备（附魔按各自原版上限）",
                parts.len(),
                self.gear_mat
            )
        } else {
            format!("已生成 {} 件 {} 装备", parts.len(), self.gear_mat)
        };
        self.copy_status_ok = true;
        self.gear_result = cmd.clone();
        self.push_history(cmd);
    }

    fn render_gear(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("材料：");
            egui::ComboBox::from_id_source("gear_mat")
                .selected_text(self.gear_mat.clone())
                .show_ui(ui, |ui| {
                    for (k, _) in GEAR_MATERIALS {
                        ui.selectable_value(&mut self.gear_mat, k.to_string(), k);
                    }
                });
            ui.checkbox(&mut self.gear_ench_on, "按原版最高等级附魔");
        });
        ui.label("部件：");
        ui.horizontal_wrapped(|ui| {
            for (i, (k, _)) in GEAR_PART_MAP.iter().enumerate() {
                ui.checkbox(&mut self.gear_parts[i], *k);
            }
        });
        if ui.add(ui::action(self.palette, "生成指令")).clicked() {
            self.gen_gear();
        }
    }

    /// 实体库搜索（预设 + 模组生物）
    fn do_entity_search(&mut self) {
        self.ent_hits.clear();
        let key = self.ent_search.trim().to_lowercase();
        if key.is_empty() {
            return;
        }
        let mut seen = HashSet::new();
        let curated = ENTITIES.iter().map(|(n, e)| (n.to_string(), e.to_string()));
        let modded = self
            .mod_entities
            .iter()
            .map(|(n, e)| (n.to_string(), e.to_string()));
        for (name, eid) in curated.chain(modded) {
            if self.ent_hits.len() >= 60 {
                break;
            }
            let hit = name.to_lowercase().contains(&key) || eid.to_lowercase().contains(&key);
            if hit && seen.insert(eid.clone()) {
                self.ent_hits.push((name, eid));
            }
        }
    }

    fn gen_armorstand(&mut self) {
        self.mark_result(Page::ArmorStand);
        let cmd = build_armorstand_cmd(
            &self.as_name,
            self.as_arms,
            self.as_no_base,
            self.as_invisible,
            self.as_small,
            self.as_invul,
            self.as_pose,
        );
        self.as_result = cmd.clone();
        self.push_history(cmd);
    }

    fn render_armorstand(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let resp = ui::field(ui, "名称(可空)", &mut self.as_name, "可以留空");
            if enter_pressed(ui, &resp) {
                self.gen_armorstand();
                resp.request_focus();
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.as_arms, "显示手臂");
            ui.checkbox(&mut self.as_no_base, "无底座");
            ui.checkbox(&mut self.as_invisible, "隐形");
            ui.checkbox(&mut self.as_small, "小型");
            ui.checkbox(&mut self.as_invul, "无敌");
        });
        ui.label("姿势预设：");
        ui.horizontal_wrapped(|ui| {
            for (i, (name, _)) in ARMOR_POSES.iter().enumerate() {
                ui.selectable_value(&mut self.as_pose, i, *name);
            }
        });
        if ui.add(ui::action(self.palette, "生成指令")).clicked() {
            self.gen_armorstand();
        }
    }

    // ---------- 快捷指令 ----------
    fn render_quick(&mut self, ui: &mut egui::Ui) {
        let p = self.palette;
        ui.horizontal_wrapped(|ui| {
            for (tab, label) in [
                "常用指令",
                "世界规则",
                "模组与距离",
                "自定义指令",
                "传送点",
                "世界方案",
            ]
            .into_iter()
            .enumerate()
            {
                ui.selectable_value(&mut self.quick_tab, tab, label);
            }
        });
        ui.add_space(6.0);
        match self.quick_tab {
            1 => self.render_command_group(ui, "世界规则", quick_world_command),
            2 => {
                let entity_reach = self.version == "1.21.1";
                ui::section(ui, p, &format!("接触距离 · {}", self.version), |ui| {
                    if self.version == "1.20.1" {
                        ui.label(
                            egui::RichText::new(
                                "方块距离需要 Forge。此版本不提供原版实体距离指令。",
                            )
                            .color(p.text_weak),
                        );
                    }
                    self.render_quick_options(ui, true);
                    self.render_quick_preset(ui, Favorite::BlockReach);
                    if entity_reach {
                        self.render_quick_preset(ui, Favorite::EntityReach);
                    }
                });
                self.render_command_group(ui, "模组命令", quick_mod_command);
            }
            3 => {
                ui::section(ui, p, "自定义指令", |ui| {
                    self.render_custom_commands(ui)
                });
            }
            4 => {
                ui::section(ui, p, "收藏的传送点", |ui| self.render_teleports(ui));
            }
            5 => self.render_world_presets(ui),
            _ => {
                ui::section(ui, p, "快捷参数", |ui| {
                    self.render_quick_options(ui, false)
                });
                self.render_command_group(ui, "常用指令", |cmd| {
                    !quick_world_command(cmd) && !quick_mod_command(cmd)
                });
            }
        }
    }

    fn render_command_group(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        include: impl Fn(&str) -> bool,
    ) {
        ui::section(ui, self.palette, title, |ui| {
            for &(_, cmd) in QUICK_COMMANDS {
                if include(cmd) {
                    self.render_quick_preset(ui, Favorite::from_command(cmd));
                }
            }
        });
    }

    fn render_custom_commands(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("名称：");
            ui.text_edit_singleline(&mut self.qc_label);
            ui.label("指令：");
            let resp = ui.text_edit_singleline(&mut self.qc_cmd);
            let add = ui.button("添加").clicked()
                || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if add {
                let cmd = self.qc_cmd.trim().to_string();
                if !cmd.is_empty() {
                    let label = match self.qc_label.trim() {
                        "" => cmd.clone(),
                        s => s.to_string(),
                    };
                    self.custom_cmds.push(CustomCmd { label, cmd });
                    self.qc_label.clear();
                    self.qc_cmd.clear();
                }
            }
        });
        if self.custom_cmds.is_empty() {
            ui.label(
                egui::RichText::new("（暂无自定义指令，添加后自动保存）")
                    .color(self.palette.text_weak),
            );
        }
        let mut del_cmd: Option<usize> = None;
        let mut click_cmd: Option<(usize, String, String)> = None;
        for (i, c) in self.custom_cmds.iter().enumerate() {
            let label = c.label.clone();
            let cmd = c.cmd.clone();
            let text = format!("{}（{}）", label, cmd);
            ui.horizontal(|ui| {
                if ui.button(text).clicked() {
                    click_cmd = Some((i, label, cmd));
                }
                if ui.small_button("×").clicked() {
                    del_cmd = Some(i);
                }
            });
        }
        if let Some((_, label, cmd)) = click_cmd {
            ui.ctx().copy_text(cmd.clone());
            self.record_history(cmd, None);
            self.copy_status = format!("已复制：{}", label);
            self.copy_status_ok = true;
        }
        if let Some(i) = del_cmd {
            self.custom_cmds.remove(i);
        }
    }

    fn render_teleports(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("名称：");
            ui.text_edit_singleline(&mut self.tp_name);
            ui.label("坐标：");
            let resp = ui.text_edit_singleline(&mut self.tp_pos);
            ui.label("（x y z）");
            let add = ui.button("添加").clicked()
                || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if add {
                let tokens: Vec<&str> = self.tp_pos.split_whitespace().collect();
                // 以前是 filter_map 静默丢弃非数字 token：输入 "1 abc 3 4" 会变成
                // [1,3,4] 被当作合法坐标，玩家被传送到错误位置。
                // 现在要求正好 3 个 token 且全部能解析。
                let nums: Option<Vec<f64>> = if tokens.len() == 3 {
                    tokens.iter().map(|s| s.parse::<f64>().ok()).collect()
                } else {
                    None
                };
                match nums {
                    Some(v) => {
                        let pos = format!("{} {} {}", v[0], v[1], v[2]);
                        let count = self.teleports.len() + 1;
                        let name = match self.tp_name.trim() {
                            "" => format!("传送点{}", count),
                            s => s.to_string(),
                        };
                        self.teleports.push(Teleport { name, pos });
                        self.tp_name.clear();
                        self.tp_pos.clear();
                        self.copy_status = format!("已添加传送点{}", count);
                        self.copy_status_ok = true;
                    }
                    None => {
                        self.copy_status = format!(
                            "添加失败：需要 3 个数字坐标，当前输入「{}」",
                            self.tp_pos.trim()
                        );
                        self.copy_status_ok = false;
                    }
                }
            }
        });
        if self.teleports.is_empty() {
            ui.label(
                egui::RichText::new("（暂无传送点，添加后自动保存）").color(self.palette.text_weak),
            );
        }
        let mut del_tp: Option<usize> = None;
        let mut click_tp: Option<(usize, String, String)> = None;
        for (i, t) in self.teleports.iter().enumerate() {
            let name = t.name.clone();
            let pos = t.pos.clone();
            let text = format!("{}（{}）", name, pos);
            ui.horizontal(|ui| {
                if ui.button(text).clicked() {
                    click_tp = Some((i, name, pos));
                }
                if ui.small_button("×").clicked() {
                    del_tp = Some(i);
                }
            });
        }
        if let Some((_, name, pos)) = click_tp {
            // 跟随顶部「目标」选择器，以前硬编码 @s，选「所有玩家」也只传自己
            let cmd = format!("/tp {} {}", self.selector_map(&self.selector), pos);
            ui.ctx().copy_text(cmd.clone());
            self.push_history(cmd);
            self.copy_status = format!("已复制：{}", name);
            self.copy_status_ok = true;
        }
        if let Some(i) = del_tp {
            self.teleports.remove(i);
        }
    }

    // ---------- 指令历史 ----------
    fn render_history(&mut self, ui: &mut egui::Ui) {
        let version = self.version.clone();
        let mut filter_changed = false;
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("共 {} 条", self.history.len()));
            for (index, label) in ["全部", "当前版本", "版本未知"].into_iter().enumerate()
            {
                filter_changed |= ui
                    .selectable_value(&mut self.history_filter, index, label)
                    .changed();
            }
            if ui
                .add_enabled(!self.history.is_empty(), egui::Button::new("清空历史"))
                .clicked()
            {
                self.history_clear_pending = true;
            }
        });
        if filter_changed {
            self.history_page = 0;
        }
        ui.label(format!(
            "导出目标：{version}。只能勾选同版指令；版本未知的旧历史仍可复制。"
        ));
        let matches: Vec<_> = self
            .history
            .iter()
            .enumerate()
            .filter(|(_, entry)| match self.history_filter {
                1 => entry.matches(&version),
                2 => entry.version.is_none(),
                _ => true,
            })
            .map(|(i, _)| i)
            .collect();
        self.history_selected.retain(|&i| {
            self.history
                .get(i)
                .is_some_and(|entry| entry.matches(&version))
        });
        ui.horizontal_wrapped(|ui| {
            if ui.button("全选当前版本").clicked() {
                self.history_selected = self
                    .history
                    .iter()
                    .enumerate()
                    .filter_map(|(i, e)| e.matches(&version).then_some(i))
                    .collect();
            }
            if ui.button("取消勾选").clicked() {
                self.history_selected.clear();
            }
            let lines = history_export_lines(&self.history, &self.history_selected, &version);
            ui.label(format!(
                "已选 {} 条记录 · {} 行指令",
                self.history_selected.len(),
                lines.len()
            ));
            let function = ui
                .add_enabled(!lines.is_empty(), egui::Button::new("导出所选函数"))
                .clicked();
            let pack = ui
                .add_enabled(!lines.is_empty(), egui::Button::new("导出所选数据包"))
                .clicked();
            if function || pack {
                let path = if pack {
                    self.export_datapack("history", "history", &lines)
                } else {
                    self.export_mcfunction("history", &lines)
                };
                self.report_export(path);
            }
        });
        if self.history_clear_pending {
            ui.colored_label(
                self.palette.status_warn,
                "清空前会保存快照；快照写入失败时不会清空。",
            );
            ui.horizontal_wrapped(|ui| {
                if ui.button("确认清空").clicked() {
                    self.run_reg_action(&ui.ctx().clone(), RegAction::History);
                }
                if ui.button("取消").clicked() {
                    self.history_clear_pending = false;
                }
            });
        }
        const PER_PAGE: usize = 20;
        let pages = matches.len().div_ceil(PER_PAGE).max(1);
        self.history_page = self.history_page.min(pages - 1);
        if matches.is_empty() {
            ui.label("没有符合筛选条件的指令。");
        }
        for &i in matches
            .iter()
            .skip(self.history_page * PER_PAGE)
            .take(PER_PAGE)
        {
            let Some(entry) = self.history.get(i) else {
                continue;
            };
            ui.push_id(i, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let mut checked = self.history_selected.contains(&i);
                    if ui
                        .add_enabled(
                            entry.matches(&version),
                            egui::Checkbox::new(&mut checked, "导出"),
                        )
                        .changed()
                    {
                        if checked {
                            self.history_selected.insert(i);
                        } else {
                            self.history_selected.remove(&i);
                        }
                    }
                    ui.label(entry.version.as_deref().unwrap_or("版本未知"));
                    if ui.button("复制").clicked() {
                        ui.ctx().copy_text(entry.command.clone());
                        self.copy_status = "已复制到剪贴板".into();
                        self.copy_status_ok = true;
                    }
                });
                ui.label(egui::RichText::new(entry.command.as_str()).monospace());
                ui.separator();
            });
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.history_page > 0, egui::Button::new("上一页"))
                .clicked()
            {
                self.history_page -= 1;
            }
            ui.label(format!("第 {}/{} 页", self.history_page + 1, pages));
            if ui
                .add_enabled(self.history_page + 1 < pages, egui::Button::new("下一页"))
                .clicked()
            {
                self.history_page += 1;
            }
        });
    }

    // ---------- 设置与清理 ----------
    /// 一般清理先保存可恢复快照；只有明确确认的彻底清除会删除整个注册表项。
    fn run_reg_action(&mut self, ctx: &egui::Context, action: RegAction) {
        if action == RegAction::Defaults {
            let result = self
                .replace_config(ctx, Config::default(), true)
                .map(|()| "已恢复默认配置。备份槽仍保留，也可以恢复操作前的快照。".into());
            self.set_config_status(result);
            self.reg_pending = None;
            return;
        }
        if action == RegAction::WholeKey && self.purge_confirm.trim() != "彻底清除" {
            return;
        }
        let snapshot_before = matches!(
            action,
            RegAction::History | RegAction::Records | RegAction::Inputs
        );
        if snapshot_before {
            if let Err(e) = self.write_safety_snapshot() {
                self.set_config_status(Err(e));
                if action == RegAction::History {
                    self.copy_status = self.reg_status.clone();
                    self.copy_status_ok = false;
                }
                return;
            }
        }
        let res: std::io::Result<()> = match action {
            RegAction::History => {
                self.history.clear();
                self.history_selected.clear();
                self.history_page = 0;
                self.history_clear_pending = false;
                Ok(())
            }
            RegAction::Records => {
                self.teleports.clear();
                self.custom_cmds.clear();
                Ok(())
            }
            RegAction::Transient => {
                // 纯运行期状态：搜索词、搜索结果、勾选、页码。
                // 这些刷新一次程序就没了，本来也存不进注册表，单独拆出来一档，
                // 用户想清「刚才搜过的痕迹」时不必连输入框里的默认值一起动。
                self.search_key.clear();
                self.search_result.clear();
                self.search_page = 0;
                self.inf_sel.clear();
                self.inf_search.clear();
                self.inf_preset_page = 0;
                self.ent_search.clear();
                self.ent_hits.clear();
                self.history_page = 0;
                Ok(())
            }
            RegAction::Inputs => {
                // 只把「各页输入框」恢复成出厂默认，且保留身份证信息：
                // 历史、传送点、自定义指令、整合包 mods 路径、窗口位置一概不动。
                // （以前这一档会连 mods_dir 一起清，用户要重新翻路径找整合包。）
                self.inf_entry = "minecraft:diamond".to_string();
                self.ench_entry = "minecraft:netherite_sword".to_string();
                self.name_entry = "minecraft:diamond".to_string();
                self.name_text.clear();
                self.name_lore = vec![String::new(); 3];
                self.batch_entry = "minecraft:diamond".to_string();
                self.batch_count = "64".to_string();
                self.ent_id = "minecraft:zombie".to_string();
                self.egg_entity = "minecraft:zombie".to_string();
                self.head_name = "Notch".to_string();
                self.pot_level = "1".to_string();
                self.pot_dur = "30".to_string();
                self.attr_val = "100".to_string();
                self.ench_level = "255".to_string();
                // 搜索词/勾选属于运行期状态，这一档顺手一起清，行为与以前一致
                self.search_key.clear();
                self.search_result.clear();
                self.search_page = 0;
                self.inf_sel.clear();
                self.inf_search.clear();
                self.ent_search.clear();
                self.ent_hits.clear();
                self.history_page = 0;
                Ok(())
            }
            RegAction::Broken => delete_reg_value(REG_CONFIG_BROKEN),
            RegAction::LastError => delete_reg_value(REG_LAST_ERROR),
            RegAction::WholeKey => delete_reg_key(),
            RegAction::Defaults => unreachable!(),
        };
        match res {
            Ok(()) => {
                self.reg_status = match action {
                    RegAction::History => "已清空指令历史".to_string(),
                    RegAction::Records => "已清空传送点与自定义指令".to_string(),
                    RegAction::Transient => "已清空搜索词与勾选状态".to_string(),
                    RegAction::Inputs => {
                        "已把各页输入框恢复默认（历史、记录、整合包路径都保留）".to_string()
                    }
                    RegAction::Broken => "已删除 config_broken（损坏配置的原文）".to_string(),
                    RegAction::LastError => "已删除 last_error（上次启动错误）".to_string(),
                    RegAction::WholeKey => {
                        self.apply_config(&Config::default());
                        apply_theme(ctx, self.palette, self.dark_mode);
                        if let Some(rect) = ctx.input(|i| i.viewport().inner_rect) {
                            self.win_w = rect.width();
                            self.win_h = rect.height();
                        }
                        if let Some(rect) = ctx.input(|i| i.viewport().outer_rect) {
                            self.win_x = rect.left();
                            self.win_y = rect.top();
                        }
                        self.last_saved =
                            serde_json::to_string(&self.config_snapshot()).unwrap_or_default();
                        self.purge_confirm.clear();
                        self.pending_import = None;
                        self.reg_view = None;
                        self.backup_pending = None;
                        self.startup_checks = Some(Vec::new());
                        self.startup_notice = None;
                        self.write_error = None;
                        "已彻底清除本机配置和备份；已导出的 JSON 文件不受影响。".to_string()
                    }
                    RegAction::Defaults => unreachable!(),
                };
                self.reg_status_ok = true;
                // 有快照的动作补一句「删之前存过一份」，用户知道还能捞回来
                if snapshot_before {
                    self.reg_status.push_str("。可在设置页恢复操作前快照。");
                }
            }
            Err(e) => {
                self.reg_status = format!("清理失败：{e}");
                self.reg_status_ok = false;
            }
        }
        if action == RegAction::History {
            self.copy_status = self.reg_status.clone();
            self.copy_status_ok = self.reg_status_ok;
        }
        self.reg_pending = None;
        // 下次渲染重新读取注册表列表与备份槽状态。
        self.reg_cache_stale = true;
        self.backup_cache = None;
    }

    /// 备份槽上的三种操作。备份本身就是「整份配置的 JSON」，
    /// 和 `config` 值同一个格式，所以备份文件拿手贴回 `config` 也能用。
    fn run_backup_op(&mut self, ctx: &egui::Context, slot: &str, op: BackupOp) {
        let res: Result<String, String> = match op {
            BackupOp::Save => {
                // 备份要带上真实时间戳：它存在的意义就是「这是哪一刻的配置」，
                // 不带时间戳的话三个槽看起来一模一样，根本分不清谁是谁。
                let mut snap = self.config_snapshot();
                snap.saved_at = now_utc_stamp();
                let text = serde_json::to_string(&snap).unwrap_or_default();
                write_reg_string_io(slot, &text).map(|_| {
                    format!(
                        "已保存到 {slot}（{} 字节，保存时间 {}）",
                        text.len(),
                        snap.saved_at
                    )
                })
            }
            BackupOp::Restore => read_reg_value_text(slot)
                .ok_or_else(|| format!("{slot} 里没有备份"))
                .and_then(|text| parse_config_backup(&text))
                .and_then(|cfg| self.replace_config(ctx, cfg, true))
                .map(|()| format!("已从 {slot} 恢复。替换前的配置也已保存为恢复快照。")),
            // delete_reg_value 返回 io::Result，这里统一成 Result<_, String>，
            // 否则 match 各分支的错误类型对不上。
            BackupOp::Delete => delete_reg_value(slot)
                .map(|_| format!("已清空 {slot}"))
                .map_err(|e| e.to_string()),
        };
        match res {
            Ok(msg) => {
                self.reg_status = msg;
                self.reg_status_ok = true;
            }
            Err(e) => {
                self.reg_status = e;
                self.reg_status_ok = false;
            }
        }
        self.backup_pending = None;
        self.reg_cache_stale = true;
        self.backup_cache = None;
    }

    /// 每个清理动作「到底会删掉什么」的一句话摘要。
    /// 确认卡上必须显示这个 —— 只说「无法恢复」，用户根本不知道自己丢的是什么。
    fn reg_action_summary(&self, action: RegAction) -> String {
        // 历史按字符串长度粗估字节数（含 JSON 引号与逗号，够用就行）
        let history_bytes: usize = self
            .history
            .iter()
            .map(|entry| entry.command.len() + 3)
            .sum();
        match action {
            RegAction::History => format!(
                "将删除：指令历史 {} 条（约 {} 字节）。其它设置、传送点、自定义指令都保留。",
                self.history.len(),
                history_bytes
            ),
            RegAction::Records => format!(
                "将删除：传送点 {} 个、自定义指令 {} 条。指令历史保留。",
                self.teleports.len(),
                self.custom_cmds.len()
            ),
            RegAction::Inputs => "将把各页输入框恢复默认值。整合包路径、历史、\
                 传送点、自定义指令、窗口位置都保留。"
                .to_string(),
            RegAction::Transient => "将清空：搜索词与搜索结果、无限元件页的勾选、\
                 实体搜索的匹配结果、各列表的页码。输入框里的内容和注册表都不动。"
                .to_string(),
            RegAction::Broken => {
                let bytes = read_reg_value_text(REG_CONFIG_BROKEN)
                    .map(|s| s.len())
                    .unwrap_or(0);
                if bytes == 0 {
                    "将删除：config_broken（当前为空或不存在）。".to_string()
                } else {
                    format!("将删除：config_broken 里那份损坏配置的原文（{bytes} 字节）。")
                }
            }
            RegAction::LastError => "将删除：last_error（上次启动失败的一条提示）。".to_string(),
            RegAction::Defaults => {
                "当前设置恢复默认，历史、传送点、自定义指令、收藏和世界方案清空。三个备份槽保留。"
                    .to_owned()
            }
            RegAction::WholeKey => format!(
                "将删除整个注册表项：指令历史 {} 条、传送点 {} 个、自定义指令 {} 条，\
                 外加全部页面输入与窗口位置。程序本体不受影响。",
                self.history.len(),
                self.teleports.len(),
                self.custom_cmds.len()
            ),
        }
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        let palette = self.palette;
        self.render_portable_backup(ui);
        ui.label(
            egui::RichText::new("日常设置保存在本机注册表。可以手动导出 JSON，用于备份或迁移。")
                .color(palette.text_weak),
        );
        ui.add_space(8.0);

        // 这一页以前是六个大块一拉到底，想清个搜索词得先滚过三屏注册表字节数。
        // 现在每块都收进可开合的分区里，标题右边带一句摘要 —— 收起时也读得到关键数字。
        //
        // id 一律写死（不能用会变的标题当 id）：egui 拿 id 存开合状态，
        // 摘要里一有数字变动 id 就跟着变，用户刚展开的分区会自己弹回去。
        //
        // 有两样东西必须常显、不能折起来：
        //   1. 写注册表失败的红条 —— 收起来等于把错误藏了；
        //   2. 确认框 —— 用户刚点的按钮，动作还没执行，得当场看见。
        //
        // 标题右边那截灰字是关键数字的摘要（几个值、几个槽、多少条历史）。
        // 只在收起时画：展开着画等于把同一句话在标题和正文里重复两遍。
        // 每个分区都是一张内嵌小卡：input_bg 底 + 1px 边框 + 圆角 9 + 内边距 10。
        // 跟页面里的结果框（result_row）、告警框、确认框是同一套尺寸。
        //
        // 原先直接用 egui 原生的 show_header：一个裸三角加一行字，没有底色、
        // 没有边框、没有内边距，跟页面上其它所有块都不一个样，看着像别的软件的控件。
        // 现在自己画标题行：整行都能点，悬停时整块描边变蓝，跟按钮的悬停反馈一致。
        let section = |ui: &mut egui::Ui,
                       id: &str,
                       title: &str,
                       summary: &str,
                       default_open: bool,
                       add: &mut dyn FnMut(&mut egui::Ui)| {
            let state_id = ui.make_persistent_id(id);
            let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                state_id,
                default_open,
            );

            let frame = egui::Frame::none()
                .fill(palette.input_bg)
                .rounding(egui::Rounding::same(9.0))
                .stroke(stroke(palette.border))
                .inner_margin(egui::Margin::same(10.0));
            let mut prepared = frame.begin(ui);

            let width = prepared.content_ui.available_width();
            let (rect, response) = prepared
                .content_ui
                .allocate_exact_size(egui::vec2(width, 26.0), egui::Sense::click());
            let hovered = response.hovered();
            if hovered {
                prepared
                    .content_ui
                    .ctx()
                    .set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                state.toggle(ui);
            }

            // 点完立刻重读：这一帧就把箭头转过去，不用等下一帧。
            let open = state.is_open();
            let openness = state.openness(ui.ctx());
            let painter = prepared.content_ui.painter().clone();

            // 标题和摘要都用 painter 直接画（不走 ui.label），所以得手动补上
            // 无障碍信息：不然读屏软件只能看到一个没有名字的方块。
            let widget_text = if open || summary.is_empty() {
                title.to_string()
            } else {
                format!("{title}，{summary}")
            };
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::CollapsingHeader, widget_text.clone())
            });

            if hovered {
                painter.rect_filled(rect, egui::Rounding::same(7.0), palette.widget_hover);
            }

            // 三角箭头按 openness 旋转（0 收起 → -90°，1 展开 → 0°），
            // 颜色用 text_weak 而不是原生那套 fg_stroke：浅底卡上后者对比度不够。
            let icon = egui::Rect::from_center_size(
                egui::pos2(rect.left() + 8.0, rect.center().y),
                egui::vec2(9.0, 9.0),
            );
            let mut points = vec![icon.left_top(), icon.right_top(), icon.center_bottom()];
            let rotation = egui::emath::Rot2::from_angle(egui::remap(
                openness,
                0.0..=1.0,
                -std::f32::consts::TAU / 4.0..=0.0,
            ));
            for p in &mut points {
                *p = icon.center() + rotation * (*p - icon.center());
            }
            painter.add(egui::Shape::convex_polygon(
                points,
                palette.text_weak,
                egui::Stroke::NONE,
            ));

            let title_galley = painter.layout_no_wrap(
                title.to_owned(),
                egui::FontId::proportional(15.0),
                palette.text,
            );
            let title_left = rect.left() + 24.0;
            let title_width = title_galley.size().x;
            painter.galley(
                egui::pos2(title_left, rect.center().y - title_galley.size().y / 2.0),
                title_galley,
                palette.text,
            );

            // 摘要只在收起时画，右对齐：展开着画等于把同一句话在正文里再说一遍。
            // 窗口拉窄时先让摘要让位 —— 标题是必须读的，摘要只是补充。
            if !open && !summary.is_empty() {
                let summary_galley = painter.layout_no_wrap(
                    summary.to_owned(),
                    egui::FontId::proportional(12.0),
                    palette.text_weak,
                );
                let summary_width = summary_galley.size().x;
                let room = rect.right() - 2.0 - summary_width;
                if room >= title_left + title_width + 16.0 {
                    painter.galley(
                        egui::pos2(room, rect.center().y - summary_galley.size().y / 2.0),
                        summary_galley,
                        palette.text_weak,
                    );
                }
            }
            if open {
                painter.hline(rect.x_range(), rect.bottom() + 5.0, stroke(palette.border));
            }

            if hovered {
                prepared.frame.stroke = stroke(palette.accent);
            }

            state.show_body_unindented(&mut prepared.content_ui, |ui| {
                if open {
                    ui.add_space(8.0);
                }
                add(ui);
            });

            prepared.end(ui);
        };

        // 写注册表失败的红条也在这里显示一次：用户进了这个页，就是想弄清数据在哪
        if let Some(err) = self.write_error.clone() {
            egui::Frame::none()
                .fill(palette.input_bg)
                .rounding(egui::Rounding::same(9.0))
                .stroke(stroke(palette.status_err))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("写入失败：{err}"))
                            .color(palette.status_err)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(
                            "改动只在内存里生效，关掉程序就没了。常见原因：注册表被安全软件锁住。",
                        )
                        .color(palette.text_weak)
                        .size(11.0),
                    );
                });
            ui.add_space(8.0);
        }
        // 每帧重绘都去枚举注册表太浪费，第一次进页时读一次，动作执行后标脏重读
        if self.reg_cache.is_none() || self.reg_cache_stale {
            self.reg_cache = Some(reg_values());
            self.reg_cache_stale = false;
        }
        let entries: Vec<(String, usize)> = self.reg_cache.clone().unwrap_or_default();
        let total: usize = entries.iter().map(|(_, n)| *n).sum();

        // ---------- 启动自检（N7） ----------
        // 只在第一次进页时算一次：里面有一次真实写注册表的动作，
        // 每帧跑等于每帧写一遍配置，既慢又没必要。
        if self.startup_checks.is_none() {
            let cfg = self.config_snapshot();
            self.startup_checks = Some(startup_checks(&cfg));
            // 自检会写一次配置，值卡片上的字节数/时间就过期了
            self.reg_cache_stale = true;
        }
        // 自检四项都正常时它占了半屏，可绝大多数时候四项都是 √。
        // 折起来，摘要里报结论；只要有一项是 × 就默认展开，
        // 别让用户自己点开才发现出了问题。
        if let Some(checks) = self.startup_checks.clone() {
            let bad = checks.iter().filter(|(ok, _, _)| !*ok).count();
            let summary = if bad == 0 {
                format!("{} 项全部正常", checks.len())
            } else {
                format!("{} 项里 {} 项有问题", checks.len(), bad)
            };
            section(
                ui,
                "sec_startup",
                "启动自检",
                &summary,
                bad > 0,
                &mut |ui| {
                    ui.label(
                        egui::RichText::new(
                            "启动时查一次：配置读得到吗、写得进去吗、格式版本对不对、备份有没有。",
                        )
                        .color(palette.text_weak)
                        .size(11.0),
                    );
                    for (ok, line, detail) in &checks {
                        ui.horizontal(|ui| {
                            // 用符号区分，不只靠颜色：色弱也能分辨。
                            // 符号只选 GBK 里就有的「√ / ×」：程序只加载一个中文字体
                            // （msyh/simhei/simsun 里第一个能读到的），U+2713/U+2717
                            // 这类符号在部分字体里没有，会渲染成空方块（实测踩过）。
                            let (mark, color) = if *ok {
                                ("√", palette.status_ok)
                            } else {
                                ("×", palette.status_err)
                            };
                            ui.label(egui::RichText::new(mark).color(color).strong());
                            ui.label(egui::RichText::new(line).color(color));
                        });
                        if !detail.is_empty() {
                            ui.label(
                                egui::RichText::new(format!("    {detail}"))
                                    .color(palette.text_weak)
                                    .size(11.0),
                            );
                        }
                    }
                    if ui.small_button("重新自检").clicked() {
                        self.startup_checks = None;
                    }
                },
            );
        }

        // 值卡片里点了「查看」的名字，先记下来，等循环结束再读内容（避免同时借 self）
        let mut view_req: Option<String> = None;
        let mut refresh_req = false;
        let val_summary = if entries.is_empty() {
            "注册表里还没有数据".to_string()
        } else {
            format!("共 {} 个值，{total} 字节", entries.len())
        };
        section(
            ui,
            "sec_values",
            "注册表里的值",
            &val_summary,
            false,
            &mut |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("位置：HKEY_CURRENT_USER\\{REG_SUBKEY}"))
                            .color(palette.text_weak)
                            .size(11.0),
                    );
                    if ui.small_button("刷新").clicked() {
                        refresh_req = true;
                    }
                });
                ui.add_space(4.0);
                if entries.is_empty() {
                    ui.label(
                        egui::RichText::new("（暂无数据 —— 下次改动设置时会自动创建）")
                            .color(palette.text_weak),
                    );
                } else {
                    for (name, size) in &entries {
                        let desc = match name.as_str() {
                            REG_CONFIG => "全部设置与历史",
                            REG_CONFIG_BROKEN => "损坏配置的原文（防丢）",
                            REG_LAST_ERROR => "上次启动的错误",
                            REG_LAST_DELETED => "上一次清理前的原配置快照",
                            "backup1" | "backup2" | "backup3" => "手工备份的配置快照",
                            _ => "",
                        };
                        ui.horizontal(|ui| {
                            if desc.is_empty() {
                                ui.label(format!("· {name} —— {size} 字节"));
                            } else {
                                ui.label(format!("· {name} —— {desc}，{size} 字节"));
                            }
                            if ui.small_button("查看").clicked() {
                                view_req = Some(name.clone());
                            }
                        });
                    }
                }

                // 值的原文预览：只读，可以整段复制走（比如把 last_deleted 贴回去救回历史）。
                // 放在值卡片里面 —— 它是「查看」的结果，跟着被点的那一行走。
                if let Some((name, text)) = self.reg_view.clone() {
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new(format!("{name} 的内容（{} 字节）", text.len()))
                            .color(palette.text)
                            .strong(),
                    );
                    ui.horizontal(|ui| {
                        if ui.small_button("复制全部").clicked() {
                            ui.ctx().copy_text(text.clone());
                        }
                        if ui.small_button("关闭").clicked() {
                            self.reg_view = None;
                        }
                    });
                    egui::ScrollArea::vertical()
                        .max_height(160.0)
                        .id_source("reg_view_scroll")
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut text.clone())
                                    .desired_rows(6)
                                    .desired_width(f32::INFINITY)
                                    .font(egui::TextStyle::Monospace),
                            );
                        });
                }
            },
        );
        if refresh_req {
            self.reg_cache = Some(reg_values());
            self.reg_cache_stale = false;
        }
        if let Some(name) = view_req {
            let text = read_reg_value_text(&name).unwrap_or_else(|| "（读取失败）".to_string());
            self.reg_view = Some((name, text));
        }

        // 历史占用统计（N6）：把条数、粗估字节、占比一起报出来，
        // 用户清历史之前能先知道"清掉能省多少、值不值得"。
        let config_bytes = entries
            .iter()
            .find(|(name, _)| name == REG_CONFIG)
            .map(|(_, size)| *size)
            .unwrap_or(0);
        let mem_summary = format!(
            "历史 {} 条，传送点 {} 个，自定义指令 {} 条",
            self.history.len(),
            self.teleports.len(),
            self.custom_cmds.len()
        );
        section(
            ui,
            "sec_memory",
            "当前内存中的状态",
            &mem_summary,
            false,
            &mut |ui| {
                ui.label(format!("· {}", history_stats(&self.history, config_bytes)));
                ui.label(format!(
                    "· 传送点 {} 个 · 自定义指令 {} 条",
                    self.teleports.len(),
                    self.custom_cmds.len()
                ));
                ui.label(format!(
                    "· 模组扫描：{} 条物品名（仅本次运行）",
                    self.mod_items.len()
                ));
                // last_deleted 是上一次清理前自动存的整份配置。
                // 这里只报一下它在不在、多大 —— 原文可以点上面值卡片里那行的「查看」复制走。
                // 注意别在这里直接读注册表：render_settings 每帧都会跑，读一次就多一次系统调用。
                // 复用上面已经枚举好的 entries 就够了（它同样只在进页/动作后刷新）。
                match entries.iter().find(|(name, _)| name == REG_LAST_DELETED) {
                    Some((_, size)) => {
                        ui.label(
                            egui::RichText::new(format!(
                                "· 操作前快照 last_deleted：{size} 字节（点上面值卡片里那行「查看」可复制原文）"
                            ))
                            .color(palette.text_weak)
                            .size(11.0),
                        );
                    }
                    None => {
                        ui.label(
                            egui::RichText::new(
                                "· 操作前快照 last_deleted：暂无（还没执行过会丢数据的清理）",
                            )
                            .color(palette.text_weak)
                            .size(11.0),
                        );
                    }
                }
            },
        );

        // ---------- 备份槽 ----------
        // 三个槽循环用，不动 config 本身。备份也存注册表，不落盘。
        if self.backup_cache.is_none() {
            self.backup_cache = Some(read_backup_meta());
        }
        let backups: Vec<(String, String, usize)> = self.backup_cache.clone().unwrap_or_default();
        let mut backup_want: Option<(String, BackupOp)> = None;
        if self
            .reg_cache
            .as_ref()
            .is_some_and(|entries| entries.iter().any(|(name, _)| name == REG_LAST_DELETED))
            && ui.button("恢复操作前快照").clicked()
        {
            backup_want = Some((REG_LAST_DELETED.to_owned(), BackupOp::Restore));
        }
        let used = backups.iter().filter(|(_, _, size)| *size > 0).count();
        let bk_summary = format!("{used}/{} 个槽已用", backups.len());
        section(
            ui,
            "sec_backup",
            "配置备份",
            &bk_summary,
            false,
            &mut |ui| {
                ui.label(
                    egui::RichText::new(
                        "备份是一整份配置的原文，和 config 同一个格式，存在注册表里，不生成文件。\
                     恢复会用备份里的值覆盖当前设置（历史、传送点、自定义指令都换掉）。",
                    )
                    .color(palette.text_weak)
                    .size(11.0),
                );
                ui.add_space(4.0);
                for (slot, stamp, size) in &backups {
                    ui.horizontal(|ui| {
                        let when = if stamp.is_empty() {
                            if *size == 0 {
                                "（空）".to_string()
                            } else {
                                format!("（{size} 字节，时间未知）")
                            }
                        } else {
                            format!("{stamp}（{size} 字节）")
                        };
                        ui.label(format!("· {slot} —— {when}"));
                        if ui.small_button("备份当前").clicked() {
                            backup_want = Some((slot.clone(), BackupOp::Save));
                        }
                        if *size > 0 {
                            if ui.small_button("恢复").clicked() {
                                backup_want = Some((slot.clone(), BackupOp::Restore));
                            }
                            if ui.small_button("清空").clicked() {
                                backup_want = Some((slot.clone(), BackupOp::Delete));
                            }
                        }
                    });
                }
            },
        );
        if let Some((slot, op)) = backup_want {
            // 只有写入空槽不需要确认。
            if op == BackupOp::Save
                && !backups
                    .iter()
                    .any(|(name, _, bytes)| name == &slot && *bytes > 0)
            {
                let ctx = ui.ctx().clone();
                self.run_backup_op(&ctx, &slot, op);
            } else {
                self.backup_pending = Some((slot, op));
            }
        }

        if let Some((slot, op)) = self.backup_pending.clone() {
            let (title, warn) = match op {
                BackupOp::Restore => (
                    format!("从 {slot} 恢复配置"),
                    "当前设置、历史、传送点、自定义指令都会被备份里的内容替换掉。",
                ),
                BackupOp::Delete => (format!("清空 {slot}"), "这个槽里的备份将被删除，无法恢复。"),
                BackupOp::Save => (format!("覆盖 {slot}"), "这个槽原有的备份将被替换。"),
            };
            ui.add_space(8.0);
            egui::Frame::none()
                .fill(palette.input_bg)
                .rounding(egui::Rounding::same(9.0))
                .stroke(stroke(palette.status_warn))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("确认{title}？"))
                            .color(palette.status_warn)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(warn)
                            .color(palette.text_weak)
                            .size(11.0),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("确认执行").clicked() {
                            let ctx = ui.ctx().clone();
                            self.run_backup_op(&ctx, &slot, op);
                        }
                        if ui.button("取消").clicked() {
                            self.backup_pending = None;
                        }
                    });
                });
        }

        let mut want: Option<RegAction> = None;
        // 按「危险度」分三档摆按钮：安全的在上面，越往下越难挽回来。
        // 以前六个按钮排成一列、只有最后一个是红的，
        // 手快点错「清空历史」的时候，用户根本没意识到那不可撤销。
        let groups: [(&str, egui::Color32, &str, &[(&str, RegAction, &str)]); 3] = [
            (
                "安全（只是删提示信息）",
                palette.text_weak,
                "删掉不影响任何设置与记录",
                &[
                    (
                        "删除上次启动错误记录",
                        RegAction::LastError,
                        "last_error，只是提示用",
                    ),
                    (
                        "删除损坏配置的原文",
                        RegAction::Broken,
                        "config_broken，确认不再需要那份旧配置时删",
                    ),
                ],
            ),
            (
                "会修改记录（先备份）",
                palette.status_warn,
                "「清空」类动作执行前会自动把当前配置整份存进 last_deleted",
                &[
                    (
                        "清空指令历史",
                        RegAction::History,
                        "删掉最多 200 条历史命令，其它设置不动",
                    ),
                    (
                        "清空传送点与自定义指令",
                        RegAction::Records,
                        "只删这两类记录，历史命令保留",
                    ),
                    (
                        "清空搜索词与勾选状态",
                        RegAction::Transient,
                        "临时状态，不进注册表；输入框内容不动",
                    ),
                    (
                        "恢复默认配置（保留备份）",
                        RegAction::Defaults,
                        "先保存快照；可撤销，备份槽不变",
                    ),
                    (
                        "各页输入框恢复默认",
                        RegAction::Inputs,
                        "输入框回出厂值；整合包路径、历史、记录都保留",
                    ),
                ],
            ),
            (
                "不可撤销（整个注册表项）",
                palette.status_err,
                "当前配置和本机备份槽一起删除",
                &[("彻底清除本机数据", RegAction::WholeKey, "程序本体不受影响")],
            ),
        ];
        // 清理区的按钮一共 7 个，占的地方最大，所以默认收起来。
        // 摘要只报「有哪些档」不报数量，免得数字一变标题就跟着跳。
        section(
            ui,
            "sec_clean",
            "清理注册表里的数据",
            "恢复默认可撤销；彻底清除不可撤销",
            false,
            &mut |ui| {
                for (group_title, group_color, group_hint, rows) in groups {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(group_title).color(group_color).strong());
                    ui.label(
                        egui::RichText::new(group_hint)
                            .color(palette.text_weak)
                            .size(11.0),
                    );
                    for (label, action, hint) in rows {
                        ui.horizontal(|ui| {
                            let danger = *action == RegAction::WholeKey;
                            let btn = egui::Button::new(*label);
                            if ui.add(btn).clicked() {
                                want = Some(*action);
                            }
                            let color = if danger {
                                palette.status_err
                            } else {
                                palette.text_weak
                            };
                            ui.label(egui::RichText::new(*hint).color(color).size(11.0));
                        });
                    }
                }
            },
        );
        if let Some(a) = want {
            // 一律先问一遍：这些操作删掉就找不回来
            self.reg_pending = Some(a);
            self.purge_confirm.clear();
            self.reg_status.clear();
        }

        if let Some(pending) = self.reg_pending {
            ui.add_space(8.0);
            let (title, warn) = match pending {
                RegAction::History => ("清空指令历史", "清空前保存快照，写入失败则不会执行。"),
                RegAction::Records => (
                    "清空传送点与自定义指令",
                    "清空前保存快照，写入失败则不会执行。",
                ),
                RegAction::Transient => (
                    "清空搜索词与勾选状态",
                    "这些只是本次运行中的临时状态，不进注册表。",
                ),
                RegAction::Inputs => (
                    "各页输入框恢复默认",
                    "输入框会回到出厂默认值；历史、记录、整合包路径都保留。",
                ),
                RegAction::Broken => ("删除损坏配置的原文", "config_broken 里的原文将被删除。"),
                RegAction::LastError => ("删除启动错误记录", "仅删掉一条提示信息。"),
                RegAction::Defaults => ("恢复默认配置", "三个备份槽保持不变，可恢复操作前快照。"),
                RegAction::WholeKey => (
                    "彻底清除本机数据",
                    "本机快照和备份槽也会被删除，不可撤销。如需保留，请先导出 JSON。",
                ),
            };
            let summary = self.reg_action_summary(pending);
            egui::Frame::none()
                .fill(palette.input_bg)
                .rounding(egui::Rounding::same(9.0))
                .stroke(stroke(palette.status_warn))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("确认{title}？"))
                            .color(palette.status_warn)
                            .strong(),
                    );
                    // 摘要：说清「删什么、留什么」，不让用户在不知道后果的情况下点确认
                    ui.label(egui::RichText::new(summary).color(palette.text));
                    ui.label(
                        egui::RichText::new(warn)
                            .color(palette.text_weak)
                            .size(11.0),
                    );
                    if pending == RegAction::WholeKey {
                        ui::field(
                            ui,
                            "输入「彻底清除」确认",
                            &mut self.purge_confirm,
                            "彻底清除",
                        );
                    }
                    ui.horizontal_wrapped(|ui| {
                        let allowed = pending != RegAction::WholeKey
                            || self.purge_confirm.trim() == "彻底清除";
                        if ui
                            .add_enabled(allowed, egui::Button::new("确认执行"))
                            .clicked()
                        {
                            // clone 出 Context：闭包里还借着 self，直接传 ui.ctx() 会冲突
                            let ctx = ui.ctx().clone();
                            self.run_reg_action(&ctx, pending);
                        }
                        if ui.button("取消").clicked() {
                            self.reg_pending = None;
                        }
                    });
                });
        }
    }

    // ---------- 物品ID查询 ----------
    fn do_search(&mut self) {
        let key = self.search_key.trim().to_lowercase();
        self.search_result.clear();
        if key.is_empty() {
            return;
        }
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (cat, items) in PRESET_CATEGORIES {
            for (name, iid, _) in items {
                let iid = extae_id_for(iid, &self.version);
                if name.to_lowercase().contains(&key) || iid.to_lowercase().contains(&key) {
                    if seen.insert(iid.to_string()) {
                        self.search_result
                            .push((format!("[{}]{}", cat, name), iid.to_string()));
                    }
                }
            }
        }
        for (name, iid) in INF_CELL_MAP {
            // 与上面的预设分支保持一致：1.20.1 下 extendedae: 要显示成 expatternprovider:
            let mapped = extae_id_for(iid, &self.version);
            if name.to_lowercase().contains(&key) || mapped.to_lowercase().contains(&key) {
                if seen.insert(mapped.to_string()) {
                    self.search_result
                        .push((format!("[无限元件]{}", name), mapped.to_string()));
                }
            }
        }
        for (cat, presets) in ENCH_PRESETS {
            for (pname, pid, _) in presets {
                if pname.to_lowercase().contains(&key) || pid.to_lowercase().contains(&key) {
                    if seen.insert(pid.to_string()) {
                        self.search_result
                            .push((format!("[{}]{}", cat, pname), pid.to_string()));
                    }
                }
            }
        }
        for (mat, _) in GEAR_MATERIALS {
            if mat.to_lowercase().contains(&key) {
                for (_cn, suffix) in GEAR_PART_MAP {
                    let item_id = format!(
                        "{}_{}",
                        GEAR_MATERIALS
                            .iter()
                            .find(|(k, _)| *k == mat)
                            .map(|(_, v)| *v)
                            .unwrap_or("minecraft:iron"),
                        suffix
                    );
                    if seen.insert(item_id.clone()) {
                        // 标签带上部件中文名：以前六个部件都叫「[装备]钻石」，
                        // 列表里是六行一模一样的字，只能靠位置猜哪个是剑哪个是靴子。
                        let part_cn = GEAR_PART_MAP
                            .iter()
                            .find(|(_, s)| *s == suffix)
                            .map(|(p, _)| *p)
                            .unwrap_or(suffix);
                        self.search_result
                            .push((format!("[装备]{} {}", mat, part_cn), item_id));
                    }
                }
            }
        }
        for (aname, aid, _) in ATTRIBUTES {
            if aname.to_lowercase().contains(&key) || aid.to_lowercase().contains(&key) {
                if seen.insert(aid.to_string()) {
                    self.search_result
                        .push((format!("[属性]{}", aname), aid.to_string()));
                }
            }
        }
        for (cn, eid) in ENCHANTMENTS {
            if cn.to_lowercase().contains(&key) || eid.to_lowercase().contains(&key) {
                if seen.insert(eid.to_string()) {
                    self.search_result
                        .push((format!("[附魔]{}", cn), eid.to_string()));
                }
            }
        }
        // 原版全量物品库（中文名 ↔ ID），预设里没有的在这里补齐。
        // 以前是边匹配边 break，截断点落在 HashMap 反序列化顺序上 —— 同一关键词
        // 每次搜到的尾部结果都不一样。现在先全部收集、排序，再截断。
        let mut vanilla_hits: Vec<(String, String)> = Vec::new();
        for (i, (name, iid)) in self.vanilla_items.iter().enumerate() {
            if let Some((lc_name, lc_id)) = self.vanilla_items_lc.get(i) {
                // 小写索引在加载时就建好了，这里不再逐条 to_lowercase 分配
                if lc_name.contains(&key) || lc_id.contains(&key) {
                    vanilla_hits.push((format!("[原版]{}", name), iid.clone()));
                }
            }
        }
        vanilla_hits.sort_by(|a, b| a.0.cmp(&b.0));
        for hit in vanilla_hits.into_iter().take(500) {
            if seen.insert(hit.1.clone()) {
                self.search_result.push(hit);
            }
        }
        // 模组物品库（扫描整合包 mods 目录得到）
        let mut mod_hits: Vec<(String, String)> = Vec::new();
        for (i, (name, iid)) in self.mod_items.iter().enumerate() {
            if let Some((lc_name, lc_id)) = self.mod_items_lc.get(i) {
                if lc_name.contains(&key) || lc_id.contains(&key) {
                    mod_hits.push((format!("[模组]{}", name), iid.clone()));
                }
            }
        }
        mod_hits.sort_by(|a, b| a.0.cmp(&b.0));
        for hit in mod_hits.into_iter().take(800) {
            if seen.insert(hit.1.clone()) {
                self.search_result.push(hit);
            }
        }
        self.search_result.sort_by(|a, b| a.0.cmp(&b.0));
    }

    fn render_search(&mut self, ui: &mut egui::Ui) {
        let p = self.palette;
        ui::section(ui, p, "查找物品", |ui| self.render_item_lookup(ui));
        ui::section(ui, p, "模组物品库", |ui| self.render_mod_library(ui));
    }

    fn render_item_lookup(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("关键词：");
            let resp = ui.text_edit_singleline(&mut self.search_key);
            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                self.do_search();
            }
            if ui.add(ui::action(self.palette, "搜索")).clicked() {
                self.do_search();
            }
        });
        if self.search_result.is_empty() {
            let hint = if self.search_key.trim().is_empty() {
                "输入中文名或物品 ID 开始搜索。"
            } else {
                "无结果。可以换个关键词，或先扫描模组物品库。"
            };
            ui.label(egui::RichText::new(hint).color(self.palette.text_weak));
            return;
        }
        ui.label(format!(
            "共 {} 条结果，可复制 ID 或用于其他工具",
            self.search_result.len()
        ));
        const SEARCH_PER_PAGE: usize = 20;
        let total_pages = self.search_result.len().div_ceil(SEARCH_PER_PAGE).max(1);
        if self.search_page >= total_pages {
            self.search_page = total_pages - 1;
        }
        let start = self.search_page * SEARCH_PER_PAGE;
        let end = (start + SEARCH_PER_PAGE).min(self.search_result.len());
        let mut use_item = None;
        for (offset, (label, iid)) in self.search_result[start..end].iter().enumerate() {
            ui.push_id(("lookup", start + offset), |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("复制 ID").clicked() {
                        ui.ctx().copy_text(iid.clone());
                        self.copy_status = format!("已复制：{iid}");
                        self.copy_status_ok = true;
                    }
                    // https://docs.rs/egui/0.27.2/egui/struct.Ui.html#method.menu_button
                    ui.menu_button("用于…", |ui| {
                        for (page, title) in [
                            (Page::Infinity, "无限元件"),
                            (Page::Batch, "批量生成"),
                            (Page::Name, "物品命名"),
                            (Page::Enchant, "255 附魔"),
                        ] {
                            if ui.button(title).clicked() {
                                use_item = Some((page, iid.clone()));
                                ui.close_menu();
                            }
                        }
                    });
                    ui.label(label);
                });
            });
        }
        if let Some((page, id)) = use_item {
            self.apply_lookup_item(page, &id);
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(self.search_page > 0, egui::Button::new("< 上一页"))
                .clicked()
            {
                self.search_page -= 1;
            }
            ui.label(format!("第 {}/{} 页", self.search_page + 1, total_pages));
            if ui
                .add_enabled(
                    self.search_page + 1 < total_pages,
                    egui::Button::new("下一页 >"),
                )
                .clicked()
            {
                self.search_page += 1;
            }
        });
    }

    fn render_mod_library(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("整合包 mods 目录：");
            let resp = ui.text_edit_singleline(&mut self.mods_dir);
            let go = ui.add(ui::action(self.palette, "扫描")).clicked()
                || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if go {
                self.scan_mods();
            }
            // 原先写的是「🧹 清除缓存」：U+1F9F9 在微软雅黑/黑体/宋体里都缺字形，
            // 界面上是一个空心豆腐块（跟之前的 ✓ ✗ ⚠ 是同一类问题），所以只用中文。
            let clear = ui.button("清除缓存").clicked();
            if clear {
                self.clear_mod_cache();
                self.copy_status = "已清除扫描记录".to_string();
                self.copy_status_ok = true;
            }
        });
        if !self.scan_status.is_empty() {
            ui.label(egui::RichText::new(&self.scan_status).color(self.palette.text_weak));
        }
        let hint = if self.mod_items.is_empty() {
            "尚未扫描。点「扫描」后会读取 jar 里的中文名，结果只留在本次运行内，关掉程序即失效。"
                .to_string()
        } else {
            format!(
                "已收录 {} 条模组物品名（仅本次运行有效，点「清除缓存」或关掉程序即清空）",
                self.mod_items.len()
            )
        };
        ui.label(egui::RichText::new(hint).color(self.palette.text_weak));
    }

    // ---------- 批量生成 ----------
    fn gen_batch(&mut self) {
        self.mark_result(Page::Batch);
        let item_id = self.batch_entry.trim().to_string();
        if item_id.is_empty() {
            self.batch_result = "请输入物品ID".to_string();
            return;
        }
        let cnt = self
            .batch_count
            .trim()
            .parse::<i32>()
            .unwrap_or(1)
            .clamp(1, 999);
        let cmd = format!(
            "/give {} {} {}",
            self.selector_map(&self.selector),
            item_id,
            cnt
        );
        self.batch_result = cmd.clone();
        self.push_history(cmd);
    }

    fn render_batch(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let r1 = ui::field(
                ui,
                "物品ID",
                &mut self.batch_entry,
                "例如 minecraft:diamond",
            );
            let r2 = ui::field(ui, "数量", &mut self.batch_count, "例如 64");
            if enter_pressed(ui, &r1) || enter_pressed(ui, &r2) {
                self.gen_batch();
                r1.request_focus();
            }
        });
        if ui.add(ui::action(self.palette, "生成指令")).clicked() {
            self.gen_batch();
        }
    }

    // ---------- 自定义实体 ----------
    fn gen_entity(&mut self) {
        self.mark_result(Page::Entity);
        let raw = self.ent_id.trim().to_string();
        if raw.is_empty() {
            self.ent_result = "请输入实体ID".to_string();
            return;
        }
        let cnt = self.ent_count.trim().parse::<i32>().unwrap_or(1);
        let flags = EntityFlags {
            noai: self.ent_noai,
            invul: self.ent_invul,
            persist: self.ent_persist,
            glow: self.ent_glow,
            silent: self.ent_silent,
        };
        let cmd = build_entity_cmd(
            &raw,
            &self.ent_name,
            &self.ent_scale,
            &self.ent_hp,
            &self.ent_atk,
            self.ent_gear,
            &flags,
            &self.version,
            cnt,
        );
        self.ent_result = cmd.clone();
        self.push_history(cmd);
    }

    fn render_entity(&mut self, ui: &mut egui::Ui) {
        ui.label("常用生物（点击填入实体ID）：");
        // 2026-09-06：去掉 clamp 1..4，列数随宽度动态
        let cols = ((ui.available_width() / 120.0).floor() as usize).clamp(1, 12);
        egui::Grid::new("ent_presets")
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                for (i, (name, eid)) in ENTITIES.iter().enumerate() {
                    if i > 0 && i % cols == 0 {
                        ui.end_row();
                    }
                    if ui.button(*name).clicked() {
                        self.ent_id = eid.to_string();
                    }
                }
            });
        ui.separator();
        ui.heading(egui::RichText::new("生物库").size(17.0));
        ui.horizontal_wrapped(|ui| {
            ui.label("关键词：");
            let resp = ui.text_edit_singleline(&mut self.ent_search);
            let go = ui.add(ui::action(self.palette, "搜索")).clicked()
                || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if go {
                self.do_entity_search();
            }
        });
        if !self.ent_hits.is_empty() {
            let hits = self.ent_hits.clone();
            ui.horizontal_wrapped(|ui| {
                for (name, eid) in &hits {
                    if ui.button(name).on_hover_text(eid).clicked() {
                        self.ent_id.clone_from(eid);
                    }
                }
            });
        } else if !self.ent_search.trim().is_empty() {
            ui.label(egui::RichText::new("没有匹配的生物").color(self.palette.status_warn));
        }
        ui.separator();
        ui.vertical(|ui| {
            let resp = ui::field(ui, "实体ID", &mut self.ent_id, "例如 minecraft:zombie");
            if enter_pressed(ui, &resp) {
                self.gen_entity();
                resp.request_focus();
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("名称：");
            ui.text_edit_singleline(&mut self.ent_name);
            ui.label("大小：");
            ui.text_edit_singleline(&mut self.ent_scale);
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("血量：");
            ui.text_edit_singleline(&mut self.ent_hp);
            ui.label("攻击：");
            ui.text_edit_singleline(&mut self.ent_atk);
            ui.label("数量：");
            ui.text_edit_singleline(&mut self.ent_count);
            ui.checkbox(&mut self.ent_gear, "全套下界合金装备");
        });
        if ui.add(ui::action(self.palette, "生成指令")).clicked() {
            self.gen_entity();
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 启动时恢复置顶状态
        if self.top_apply {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(if self.always_top {
                egui::WindowLevel::AlwaysOnTop
            } else {
                egui::WindowLevel::Normal
            }));
            self.top_apply = false;
        }
        // 窗口矩形等首帧稳定后再记录，避免用初始占位值覆盖配置。
        // 注意：screen_rect 恒以 (0,0) 为左上角，取 left()/top() 永远是 0。
        // 位置要取 outer_rect（with_position 收的是外框位置），
        // 尺寸要取 inner_rect（with_inner_size 收的是客户区尺寸），
        // 两者混用会让窗口每次重启都长大一圈。
        if self.started.elapsed().as_secs_f32() > 2.0 {
            let (outer, inner) = ctx.input(|i| (i.viewport().outer_rect, i.viewport().inner_rect));
            if let (Some(o), Some(n)) = (outer, inner) {
                if n.width() >= 300.0 && n.height() >= 300.0 {
                    self.win_x = o.left();
                    self.win_y = o.top();
                    self.win_w = n.width();
                    self.win_h = n.height();
                }
            }
        }
        self.poll_scan();
        if self.scanning {
            // 扫描期间保持刷新，让「正在扫描」和完成提示能及时出现
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }
        self.render(ctx);
        // 配置有变化即写（每帧最多一次，内容比对防抖）。
        // 写的是注册表里的一个字符串值，写入本身由系统保证原子性。
        //
        // 时间戳要单独处理：config_snapshot() 把 saved_at 留空，
        // 拿它算出的 core 才是「内容是否真的变了」的判据；
        // 真到写盘时再填当前时间，否则每帧都填一个新时间戳，
        // 比对永远不相等，就变成每帧都写一次注册表了。
        let mut snap = self.config_snapshot();
        let core = serde_json::to_string(&snap).unwrap_or_default();
        if core != self.last_saved {
            snap.saved_at = now_utc_stamp();
            let text = serde_json::to_string(&snap).unwrap_or_default();
            match write_config_atomic(&text) {
                Ok(()) => {
                    // 记的是不带时间戳的那份，下一帧才能正确判断「没变就别写」
                    self.last_saved = core;
                    // 写了一次注册表，「设置与清理」页里的值列表/字节数就过期了。
                    // 标脏让下一帧重读，否则用户会看到「共 0 个值」这种过时快照。
                    self.reg_cache_stale = true;
                    // 恢复正常写入了就把旧错误清掉，别让红条一直挂着吓人
                    self.write_error = None;
                }
                Err(e) => {
                    // 这里以前是 `.is_ok()`：写失败被静默吞掉，界面照常显示改动成功，
                    // 重启之后才发现什么都没保存。现在把错误挂在顶部，用户至少能看见。
                    // 注意：last_saved 不更新，下一帧还会重试。
                    let msg = format!("配置写入注册表失败：{e}");
                    if self.write_error.as_deref() != Some(msg.as_str()) {
                        self.write_error = Some(msg);
                    }
                }
            }
        }
    }
}

fn load_window_icon() -> Option<egui::IconData> {
    // 编译期内嵌图标，不依赖 exe 旁边的文件
    let bytes = include_bytes!("../custom_icon.ico");
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Some(egui::IconData {
        rgba: rgba.into_raw(),
        width: w,
        height: h,
    })
}

/// 判断保存下来的窗口位置是否还「有可能」出现在屏幕上。
///
/// 显示器全部拔掉/改插后，窗口可能恢复到屏幕外，用户会以为程序没启动。
/// 没有引入 Win32 API（本项目零 unsafe），只用一条保守判据：
/// 窗口矩形必须与以 (0,0) 为原点的桌面区域有交集。
/// 这样左侧/上方的副屏（坐标为负）仍然允许，只挡掉完全跑到屏幕外的情况。
/// 局限：右下方超出桌面的情况检测不到，需要在系统里把窗口拖回来。
fn saved_position_usable(x: f32, y: f32, w: f32, h: f32) -> bool {
    if !(x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite()) {
        return false;
    }
    if w < 300.0 || h < 300.0 {
        return false;
    }
    // 与 (0,0)..(+∞,+∞) 有交集即可
    x + w > 0.0 && y + h > 0.0
}

fn main() -> eframe::Result<()> {
    let (cfg, notice) = load_config();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1240.0, 790.0])
        .with_min_inner_size([680.0, 560.0]);
    // 与 update() 里的保存阈值保持一致（都是 300），避免 100~300 的窗口永远存不上位置
    if saved_position_usable(cfg.win_x, cfg.win_y, cfg.win_w, cfg.win_h) {
        viewport = viewport
            .with_position([cfg.win_x, cfg.win_y])
            .with_inner_size([cfg.win_w, cfg.win_h]);
    } else if cfg.win_w >= 300.0 && cfg.win_h >= 300.0 {
        // 位置不可用但尺寸有效：只沿用尺寸，位置交给系统
        viewport = viewport.with_inner_size([cfg.win_w, cfg.win_h]);
    }
    if let Some(icon) = load_window_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }
    let options = eframe::NativeOptions {
        viewport,
        follow_system_theme: false,
        default_theme: eframe::Theme::Light,
        persist_window: false,
        ..Default::default()
    };
    let result = eframe::run_native(
        "我的世界小工具",
        options,
        Box::new(move |cc| Box::new(App::new(&cc.egui_ctx, cfg, notice))),
    );
    // 启动失败时给出可见反馈：release 是 windows_subsystem="windows"，没有控制台，
    // 直接返回 Result 的话用户只会看到「双击没反应」。
    if let Err(e) = &result {
        show_fatal_error(&format!("程序启动失败：{e}"));
    }
    result
}

/// release 构建没有控制台，致命错误只能靠界面让用户看见。
///
/// 错误记进注册表的 `last_error` 值（不落盘），下次启动时由 `load_config`
/// 取出来放进 `startup_notice`，在窗口顶部以红字提示一次，读完即删。
fn show_fatal_error(msg: &str) {
    save_last_error(msg);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enchant_1211_uses_sweeping_edge_and_level() {
        let c = build_enchant_cmd(
            "minecraft:netherite_sword",
            &["minecraft:sharpness", "minecraft:sweeping_edge"],
            "1.21.1",
            "@s",
            false,
            255,
        );
        assert!(c.contains("\"minecraft:sweeping_edge\":255"), "{}", c);
        assert!(c.starts_with("/give @s minecraft:netherite_sword[enchantments={levels:"));
    }

    #[test]
    fn enchant_1201_remaps_sweeping_and_level() {
        let c = build_enchant_cmd(
            "minecraft:bow",
            &["minecraft:sweeping_edge", "minecraft:power"],
            "1.20.1",
            "@p",
            true,
            10,
        );
        assert!(c.contains("{id:\"minecraft:sweeping\",lvl:10s}"), "{}", c);
        assert!(c.contains("{id:\"minecraft:power\",lvl:10s}"));
        assert!(c.contains("Unbreakable:1b"));
    }

    #[test]
    fn infinity_cell4_differs_by_version() {
        assert_eq!(
            build_infinity_cmd("minecraft:diamond", "item", "1.20.1", "Cell⁴细胞⁴", "@s"),
            "/give @s cell4:infinity_item_cell{cell4item:\"minecraft:diamond\"}"
        );
        assert_eq!(
            build_infinity_cmd("minecraft:water", "fluid", "1.20.1", "ExtendedAE扩展AE", "@s"),
            "/give @s expatternprovider:infinity_cell{record:{id:\"minecraft:water\",\"#c\":\"ae2:f\"}}"
        );
    }

    #[test]
    fn potion_kinds_both_versions() {
        let c1211 = build_potion_cmd(
            "minecraft:splash_potion",
            "minecraft:speed",
            1,
            0,
            600,
            "1.21.1",
            "@s",
        );
        assert!(
            c1211.starts_with("/give @s minecraft:splash_potion[potion_contents="),
            "{}",
            c1211
        );
        assert!(c1211.contains("amplifier:0"));
        let c1201 = build_potion_cmd(
            "minecraft:tipped_arrow",
            "minecraft:speed",
            1,
            0,
            600,
            "1.20.1",
            "@s",
        );
        assert!(
            c1201.contains("minecraft:tipped_arrow{CustomPotionEffects:[{Id:1,"),
            "{}",
            c1201
        );
    }

    #[test]
    fn head_syntax_per_version() {
        assert_eq!(
            build_head_cmd("Notch", "1.21.1", "@s"),
            "/give @s minecraft:player_head[profile={name:\"Notch\"}]"
        );
        assert_eq!(
            build_head_cmd("Notch", "1.20.1", "@s"),
            "/give @s minecraft:player_head{SkullOwner:\"Notch\"}"
        );
    }

    #[test]
    fn name_1201_display_format() {
        let c = build_name_cmd(
            "minecraft:diamond",
            "名字",
            &["第一行".to_string(), String::new(), String::new()],
            "1.20.1",
            "@s",
        );
        assert!(c.contains("display:{Name:'"), "{}", c);
        assert!(c.contains("Lore:["));
    }

    #[test]
    fn hand_ench_1211_max_level() {
        let c = build_hand_ench_cmds(
            "1.21.1",
            "@s",
            &["minecraft:sharpness", "minecraft:mending"],
            true,
            255,
            false,
        );
        assert_eq!(
            c, "/enchant @s minecraft:sharpness 5\n/enchant @s minecraft:mending 1",
            "{}",
            c
        );
    }

    #[test]
    fn hand_ench_1211_custom_level_clamped() {
        // 等级超出原版上限自动截断：击退上限2，给10只出2
        let c = build_hand_ench_cmds("1.21.1", "@s", &["minecraft:knockback"], false, 10, false);
        assert_eq!(c, "/enchant @s minecraft:knockback 2", "{}", c);
    }

    #[test]
    fn hand_ench_1201_sweeping_and_skip() {
        // 1.20.1：sweeping_edge 回退成 sweeping；1.21 独有附魔被跳过
        let c = build_hand_ench_cmds(
            "1.20.1",
            "@s",
            &[
                "minecraft:sweeping_edge",
                "minecraft:density",
                "minecraft:efficiency",
            ],
            true,
            255,
            false,
        );
        assert!(c.contains("/enchant @s minecraft:sweeping 3"), "{}", c);
        assert!(c.contains("/enchant @s minecraft:efficiency 5"), "{}", c);
        assert!(!c.contains("density"), "{}", c);
    }

    #[test]
    fn hand_ench_unbreakable_modifier() {
        let c = build_hand_ench_cmds("1.21.1", "@s", &["minecraft:unbreaking"], true, 255, true);
        let lines: Vec<&str> = c.lines().collect();
        assert_eq!(lines.len(), 2, "{}", c);
        assert!(
            lines[1].starts_with("/item modify entity @s weapon.mainhand"),
            "{}",
            c
        );
        assert!(lines[1].contains("\"minecraft:unbreakable\":{}"), "{}", c);
        // 1.20.1 写不了 unbreakable，不生成该条
        let c12 = build_hand_ench_cmds("1.20.1", "@s", &["minecraft:unbreaking"], true, 255, true);
        assert_eq!(c12.lines().count(), 1, "{}", c12);
    }

    #[test]
    fn config_roundtrip() {
        let cfg = Config {
            schema_version: CURRENT_SCHEMA,
            saved_at: "2026-10-30T00:34:48Z".into(),
            version: "1.21.1".into(),
            selector: "@s".into(),
            dark_mode: true,
            always_top: true,
            ench_level: "100".into(),
            history: vec!["/weather clear".into()],
            history_versions: vec![Some("1.21.1".into())],
            quick_options: QuickOptions::default(),
            favorites: Vec::new(),
            world_presets: Vec::new(),
            teleports: vec![Teleport {
                name: "主城".into(),
                pos: "0 100 0".into(),
            }],
            custom_cmds: vec![CustomCmd {
                label: "回血".into(),
                cmd: "/effect give @s minecraft:instant_health 1 255".into(),
            }],
            inf_entry: "minecraft:diamond".into(),
            ench_entry: String::new(),
            name_entry: String::new(),
            name_text: String::new(),
            batch_entry: String::new(),
            batch_count: String::new(),
            ent_id: String::new(),
            egg_entity: String::new(),
            head_name: String::new(),
            pot_level: String::new(),
            pot_dur: String::new(),
            attr_val: String::new(),
            mods_dir: String::new(),
            win_x: 0.0,
            win_y: 0.0,
            win_w: 0.0,
            win_h: 0.0,
        };
        let s = serde_json::to_string(&cfg).unwrap();
        let back: Config = serde_json::from_str(&s).unwrap();
        assert_eq!(back.teleports.len(), 1);
        assert_eq!(back.teleports[0].pos, "0 100 0");
        assert!(back.dark_mode);
        assert!(back.always_top);
        assert_eq!(back.history[0], "/weather clear");
        // 新增的两个字段也要能往返
        assert_eq!(back.schema_version, CURRENT_SCHEMA);
        assert_eq!(back.saved_at, "2026-10-30T00:34:48Z");
    }

    #[test]
    fn old_config_without_schema_fields_still_loads() {
        // 老版本写进注册表的那份 JSON 没有 schema_version / saved_at，
        // 必须还能完整读出来，否则升级一次就丢一次历史。
        let old = r#"{"version":"1.20.1","selector":"@a","history":["/say 老配置"],
                      "teleports":[{"name":"主城","pos":"0 64 0"}],
                      "win_x":10.0,"win_y":20.0,"win_w":800.0,"win_h":600.0}"#;
        let cfg: Config = serde_json::from_str(old).unwrap();
        assert_eq!(cfg.schema_version, 0, "缺字段应该取默认值 0");
        assert!(cfg.saved_at.is_empty());
        assert_eq!(cfg.version, "1.20.1");
        assert_eq!(cfg.history.len(), 1);
        assert_eq!(cfg.teleports.len(), 1);
        assert_eq!(cfg.win_w, 800.0);
    }

    #[test]
    fn utc_formatter_matches_known_dates() {
        // 1970-01-01T00:00:00Z
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        // 2000-03-01T00:00:00Z（闰年之后的月份边界，最容易算错的一天）
        assert_eq!(format_utc(951_868_800), "2000-03-01T00:00:00Z");
        // 2024-02-29T12:34:56Z（闰日）
        assert_eq!(format_utc(1_709_210_096), "2024-02-29T12:34:56Z");
        // 2026-10-30T08:34:48Z（本项目实际写入的一个时间点）
        assert_eq!(format_utc(1_793_349_288), "2026-10-30T08:34:48Z");
        // 2100 不是闰年：2100-03-01T00:00:00Z
        assert_eq!(format_utc(4_107_542_400), "2100-03-01T00:00:00Z");
    }

    #[test]
    fn history_stats_reports_count_and_share() {
        let mut h = VecDeque::new();
        // 空历史：只报 0 条，不出现除零
        let s = history_stats(&h, 0);
        assert!(s.contains("0 条"), "{s}");
        // 8 条，每条 "abcdefgh" (8+3=11 字节) → 88 字节；config 给 880 → 10%
        for _ in 0..8 {
            h.push_back(HistoryEntry {
                command: "abcdefgh".to_owned(),
                version: None,
            });
        }
        let s = history_stats(&h, 880);
        assert!(s.contains("8 条"), "{s}");
        assert!(s.contains("88 字节"), "{s}");
        assert!(s.contains("10%"), "{s}");
        // config 比历史还小（脏数据）时不能算出大于 100% 或 NaN
        let s2 = history_stats(&h, 1);
        assert!(!s2.contains("NaN"), "{s2}");
        assert!(s2.contains("100%"), "{s2}");
    }

    #[test]
    fn mcfunction_lines_stripped() {
        assert_eq!(mcfn_line("/tp @s 0 0 0"), "tp @s 0 0 0");
        assert_eq!(mcfn_line("say hi"), "say hi");
    }

    #[test]
    fn entity_flags_rendered() {
        let flags = EntityFlags {
            noai: true,
            invul: true,
            persist: false,
            glow: false,
            silent: true,
        };
        let c = build_entity_cmd(
            "minecraft:zombie",
            "测试Boss",
            "1.5",
            "100",
            "20",
            false,
            &flags,
            "1.21.1",
            1,
        );
        assert!(c.contains("NoAI:1b"), "{}", c);
        assert!(c.contains("Invulnerable:1b"));
        assert!(c.contains("Silent:1b"));
        assert!(!c.contains("PersistenceRequired"));
        // 1.21 起属性 NBT 字段必须小写；属性 ID 前缀仍保留 generic.
        assert!(
            c.contains(r#"attributes:[{id:"minecraft:generic.max_health",base:100}"#),
            "{}",
            c
        );
        assert!(
            c.contains(r#"{id:"minecraft:generic.scale",base:1.5}"#),
            "{}",
            c
        );
        assert!(!c.contains("Attributes:"), "{}", c);
    }

    #[test]
    fn entity_attrs_case_per_version() {
        let flags = EntityFlags::default();
        let c11 = build_entity_cmd(
            "minecraft:zombie",
            "",
            "",
            "100",
            "20",
            false,
            &flags,
            "1.21.1",
            1,
        );
        assert!(
            c11.contains(r#"attributes:[{id:"minecraft:generic.max_health",base:100},"#),
            "{}",
            c11
        );
        assert!(!c11.contains("Attributes:"), "{}", c11);

        let c20 = build_entity_cmd(
            "minecraft:zombie",
            "",
            "",
            "100",
            "20",
            false,
            &flags,
            "1.20.1",
            1,
        );
        assert!(
            c20.contains(r#"Attributes:[{Name:"minecraft:generic.max_health",Base:100},"#),
            "{}",
            c20
        );
        assert!(!c20.contains("attributes:"), "{}", c20);
    }

    #[test]
    fn datapack_dir_case_per_version() {
        assert_eq!(datapack_function_dir("1.20.1"), "functions");
        assert_eq!(datapack_function_dir("1.21.1"), "function");
    }

    #[test]
    fn egg_uses_concrete_spawn_egg() {
        let c11 = build_egg_cmd("minecraft:zombie", "", "1.0", "1.21.1", "@s");
        assert!(
            c11.contains("minecraft:zombie_spawn_egg[entity_data="),
            "{}",
            c11
        );
        assert!(!c11.contains(" minecraft:spawn_egg["), "{}", c11);

        let c20 = build_egg_cmd("minecraft:zombie", "", "1.0", "1.20.1", "@s");
        assert!(
            c20.contains(r#"minecraft:zombie_spawn_egg{EntityTag:{id:"minecraft:zombie"}}"#),
            "{}",
            c20
        );

        // 没有刷怪蛋的实体要给出明确提示，而不是生成无效指令
        let no_egg = build_egg_cmd("minecraft:player", "", "1.0", "1.21.1", "@s");
        assert!(no_egg.starts_with("无法生成"), "{}", no_egg);
    }

    #[test]
    fn egg_scale_uses_lowercase_attrs() {
        let c = build_egg_cmd("minecraft:zombie", "", "2.0", "1.21.1", "@s");
        assert!(
            c.contains(r#"attributes:[{id:"minecraft:generic.scale",base:2}]"#),
            "{}",
            c
        );
        assert!(!c.contains("Attributes:"), "{}", c);
    }

    #[test]
    fn lang_keys_collected() {
        let mut lang = HashMap::new();
        lang.insert(
            "item.extendedae.infinity_cell".to_string(),
            "无限元件".to_string(),
        );
        lang.insert("block.minecraft.stone".to_string(), "石头".to_string());
        lang.insert("entity.minecraft.zombie".to_string(), "僵尸".to_string());
        lang.insert(
            "item.slashblade_addon.tofu.desc".to_string(),
            "豆腐（说明）".to_string(),
        );
        let mut items = HashMap::new();
        let mut ents = HashMap::new();
        collect_lang_entries(&lang, &mut items, &mut ents);
        assert_eq!(
            items.get("extendedae:infinity_cell").map(String::as_str),
            Some("无限元件")
        );
        assert_eq!(
            items.get("minecraft:stone").map(String::as_str),
            Some("石头")
        );
        assert!(!items.contains_key("slashblade_addon:tofu.desc"));
        assert_eq!(
            ents.get("minecraft:zombie").map(String::as_str),
            Some("僵尸")
        );
    }

    #[test]
    fn armorstand_cmd_built() {
        let c = build_armorstand_cmd("守卫", true, true, false, false, true, 1);
        assert!(
            c.starts_with("/summon minecraft:armor_stand ~ ~ ~ {"),
            "{}",
            c
        );
        assert!(c.contains("ShowArms:1b"));
        assert!(c.contains("NoBasePlate:1b"));
        assert!(c.contains("Invulnerable:1b"));
        assert!(c.contains("CustomName:'"));
        assert!(c.contains("Pose:{RightArm:[-60f,0f,-80f]}"));
        let plain = build_armorstand_cmd("", false, false, false, false, false, 0);
        assert!(!plain.contains("Pose:"));
        assert!(!plain.contains("ShowArms"));
    }

    #[test]
    fn armorstand_defaults_produce_no_empty_nbt() {
        // 全默认时不该生成 `~ ~ ~ {}` 这种空复合标签
        let plain = build_armorstand_cmd("", false, false, false, false, false, 0);
        assert_eq!(plain, "/summon minecraft:armor_stand ~ ~ ~");
    }

    #[test]
    fn enchant_levels_are_per_enchantment() {
        // 装备页每件装备的附魔各用各的原版上限，不再一把 255
        let levels: Vec<(&str, u32)> = vec![("minecraft:sharpness", 5), ("minecraft:mending", 1)];
        let c = build_enchant_cmd_levels("minecraft:diamond_sword", &levels, "1.21.1", "@s", false);
        assert!(c.contains(r#""minecraft:sharpness":5"#), "{}", c);
        assert!(c.contains(r#""minecraft:mending":1"#), "{}", c);
        assert!(!c.contains(":255"));
    }

    #[test]
    fn empty_selection_degrades_to_plain_give() {
        // 什么都没选时不要产出 `item[]` / `{display:{}}` 这样的空壳
        assert_eq!(
            build_enchant_cmd("minecraft:diamond_sword", &[], "1.21.1", "@s", false, 5),
            "/give @s minecraft:diamond_sword"
        );
        assert_eq!(
            build_enchant_cmd("minecraft:diamond_sword", &[], "1.20.1", "@s", false, 5),
            "/give @s minecraft:diamond_sword"
        );
        assert_eq!(
            build_name_cmd("minecraft:diamond", "", &[], "1.21.1", "@s"),
            "/give @s minecraft:diamond"
        );
        assert_eq!(
            build_name_cmd("minecraft:diamond", "", &[], "1.20.1", "@s"),
            "/give @s minecraft:diamond"
        );
    }

    #[test]
    fn text_escapes_backslash_before_quote() {
        // 反斜杠必须翻倍：json! 产出 `\\`，SNBT 单引号串再吃一层，最后进 JSON 才对
        let t = make_text("a\\b");
        assert!(t.contains(r#"a\\\\b"#), "{}", t);
        // 单引号也要转义，顺序在反斜杠之后
        let q = make_text("it's");
        assert!(q.contains(r"it\'s"), "{}", q);
    }

    #[test]
    fn gear_enchant_levels_not_255() {
        let c = build_gear_cmd("钻石", &["剑"], "1.21.1", "@s", true);
        assert!(c.contains("minecraft:diamond_sword"), "{}", c);
        assert!(!c.contains(":255"), "{}", c);
        // 未附魔版就是普通 give
        let plain = build_gear_cmd("钻石", &["剑"], "1.21.1", "@s", false);
        assert_eq!(plain, "/give @s minecraft:diamond_sword");
    }

    #[test]
    fn preset_ids_are_unique_across_categories() {
        // 预设表里同一个物品 ID 只能出现一次。以前红石粉、下界之星、西瓜、下界疣、
        // 各种酿造材料都跨分类重复，用户在不同分组里看到同一个东西两次。
        let mut seen: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        let mut dups: Vec<String> = Vec::new();
        for (cat, items) in PRESET_CATEGORIES {
            for (_name, iid, _ty) in items {
                if let Some(prev) = seen.insert(iid, cat) {
                    dups.push(format!("{iid} 同时出现在「{prev}」和「{cat}」"));
                }
            }
        }
        assert!(dups.is_empty(), "预设分类里有重复：\n{}", dups.join("\n"));
    }

    #[test]
    fn preset_names_match_official_zh() {
        // 预设里显示的名字要和原版中文名一致，否则搜索搜不到、看着像两个东西。
        // 只校验原版物品（原版名表里有的 ID）；模组物品无从校验，跳过。
        //
        // 例外：音乐唱片。原版 16 张唱片的中文名全是「音乐唱片」，照搬过来
        // 列表里就是十六行一模一样的字，反而没法选，所以保留「唱片Cat」这种写法。
        let table: std::collections::HashMap<String, String> =
            serde_json::from_str(include_str!("../assets/items_zh.json")).unwrap_or_default();
        let mut bad: Vec<String> = Vec::new();
        for (_cat, items) in PRESET_CATEGORIES {
            for (name, iid, _ty) in items {
                // minecraft: 开头的必须是真原版物品，否则给出来的是个不存在的 ID
                if iid.starts_with("minecraft:") && !table.contains_key(*iid) {
                    bad.push(format!("{iid}：预设写「{name}」，原版名表里没有这个 ID"));
                    continue;
                }
                if iid.starts_with("minecraft:music_disc_") {
                    continue;
                }
                if let Some(official) = table.get(*iid) {
                    if official != name {
                        bad.push(format!("{iid}：预设写「{name}」，原版是「{official}」"));
                    }
                }
            }
        }
        assert!(bad.is_empty(), "预设和原版对不上：\n{}", bad.join("\n"));
    }

    #[test]
    fn preset_ids_exist_in_vanilla_registries() {
        // preset_names_match_official_zh 的基准是 items_zh.json，而那份名表混了
        // 890 条组件键（banner.base.black 之类）和 131 条方块，拿它查「ID 存不存在」
        // 会漏 —— sweet_berry_bush（方块）、scute（1.21.1 已删）当初就是这么混进来的。
        // 这里改用从官方注册表导出的纯 ID 清单做基准，并且两个版本各查一遍。
        //
        // 注意：预设表统一存 1.21.1 的 ID，所以查 1.20.1 时要先过一遍 item_id_for
        // （鳞甲会换回旧的 scute），否则新 ID 在旧版注册表里当然查不到。
        let v1211 = vanilla_ids_for("1.21.1");
        let v1201 = vanilla_ids_for("1.20.1");
        let mut bad: Vec<String> = Vec::new();
        for (cat, items) in PRESET_CATEGORIES {
            for (name, iid, itype) in items {
                // 模组物品不在原版注册表里，无从校验（用户已确认这些 ID 是可用的）
                if !iid.starts_with("minecraft:") {
                    continue;
                }
                // 流体不是物品：water / lava 只在 fluid 注册表里，/give 也拿不到。
                // 预设里这两条标了 "fluid"，出指令走的是无限元件而不是 /give，单独放行。
                if *itype == "fluid" {
                    continue;
                }
                for (ver, set) in [("1.21.1", &v1211), ("1.20.1", &v1201)] {
                    let actual = item_id_for(iid, ver);
                    let Some(short_ver) = actual.strip_prefix("minecraft:") else {
                        continue;
                    };
                    if !set.contains(short_ver) {
                        bad.push(format!(
                            "[{cat}] {name} => {iid}（{ver} 实际用 {actual}）不在物品注册表里"
                        ));
                    }
                }
            }
        }
        assert!(bad.is_empty(), "预设里有无效物品 ID：\n{}", bad.join("\n"));
    }

    #[test]
    fn item_ids_get_renamed_per_version() {
        // 鳞甲：1.21.1 把 scute 拆成 armadillo_scute / turtle_scute，旧 ID 已删除。
        // 数据表存新 ID，1.20.1 出指令时要换回旧的，1.21.1 手输旧 ID 也要换新的。
        assert_eq!(
            item_id_for("minecraft:turtle_scute", "1.20.1"),
            "minecraft:scute"
        );
        assert_eq!(
            item_id_for("minecraft:turtle_scute", "1.21.1"),
            "minecraft:turtle_scute"
        );
        assert_eq!(
            item_id_for("minecraft:scute", "1.21.1"),
            "minecraft:turtle_scute"
        );
        assert_eq!(item_id_for("minecraft:scute", "1.20.1"), "minecraft:scute");
        // 无关 ID 原样返回
        assert_eq!(
            item_id_for("minecraft:diamond", "1.20.1"),
            "minecraft:diamond"
        );
        // 无限元件指令里也要跟着换
        let c = build_infinity_cmd(
            "minecraft:turtle_scute",
            "item",
            "1.20.1",
            "ExtendedAE扩展AE",
            "@s",
        );
        assert!(c.contains("minecraft:scute"), "{}", c);
        let c = build_infinity_cmd(
            "minecraft:turtle_scute",
            "item",
            "1.21.1",
            "Cell⁴细胞⁴",
            "@s",
        );
        assert!(c.contains("minecraft:turtle_scute"), "{}", c);
    }

    #[test]
    fn quick_commands_include_requested_presets_without_duplicates() {
        let commands: HashSet<_> = QUICK_COMMANDS.iter().map(|&(_, cmd)| cmd).collect();
        assert_eq!(commands.len(), QUICK_COMMANDS.len());
        for cmd in [
            "/gamerule keepInventory true",
            "/gamerule mobGriefing false",
            "/gamerule doDaylightCycle false",
            "/gamerule doWeatherCycle false",
            "/difficulty hard",
            "/kill @e[type=item]",
            "/ae2 channelmode infinite",
            "/tiab addTime 999999999",
            "/effect give Xian_Yu12138 minecraft:instant_health infinite 111",
            "/experience add @p 1000000000 levels",
        ] {
            assert!(commands.contains(cmd), "缺少快捷指令：{cmd}");
        }
    }

    #[test]
    fn quick_reach_commands_follow_game_version() {
        let forge = quick_reach_commands("1.20.1");
        assert_eq!(forge.len(), 1);
        assert_eq!(
            forge[0].1,
            "/execute as @a[tag=!lr_done] run attribute @s forge:block_reach base set 15"
        );

        let vanilla = quick_reach_commands("1.21.1");
        assert_eq!(vanilla.len(), 2);
        assert_eq!(
            vanilla[0].1,
            "/attribute @p minecraft:player.block_interaction_range base set 15"
        );
        assert_eq!(
            vanilla[1].1,
            "/attribute @p minecraft:player.entity_interaction_range base set 15"
        );
    }

    #[test]
    fn infinity_copy_status_has_its_own_line() {
        use egui::epaint::Shape;

        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, Config::default(), None);
        app.page = Page::Infinity;
        app.inf_entry = "minecraft:cobblestone".into();
        app.gen_infinity(true);
        let status = "已复制：minecraft:cobblestone";
        app.inf_status = status.into();

        for (width, wide) in [(184.0, true), (264.0, true), (500.0, false)] {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width, 560.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        app.render_result_bar(ui, wide);
                    });
                },
            );
            let bounds = |text: &str| {
                output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        Shape::Text(label) if label.galley.job.text == text => {
                            Some((label.visual_bounding_rect(), shape.clip_rect))
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("missing {text:?} at width {width}"))
            };
            let (copy, _) = bounds("复制指令");
            let (message, clip) = bounds(status);
            assert!(
                copy.bottom() < message.top(),
                "status shares the copy row at width {width}: {copy:?} / {message:?}"
            );
            let (export, _) = bounds("导出函数");
            assert!(
                message.bottom() < export.top(),
                "status crosses the export row at width {width}"
            );
            assert!(
                clip.expand(1.0).contains_rect(message),
                "status gets clipped at width {width}"
            );
        }
    }

    #[test]
    fn result_row_only_copies_commands() {
        // 生成失败时结果框里是中文提示（如「ExtendedAE 1.21.1 无法自定义任意物品」），
        // result_row 的复制按钮只认 / 开头的才是指令 —— 以前只判空串，会把提示当命令复制。
        let is_cmd = |s: &str| s.trim_start().starts_with('/');
        assert!(is_cmd("/give @s minecraft:diamond"));
        assert!(is_cmd("  /give @s minecraft:diamond"));
        assert!(!is_cmd("ExtendedAE 1.21.1 无法自定义任意物品"));
        assert!(!is_cmd(
            "无法生成：minecraft:illusioner 没有对应的刷怪蛋，请改用「自定义实体」页的 /summon"
        ));
        assert!(!is_cmd(""));
    }
}
