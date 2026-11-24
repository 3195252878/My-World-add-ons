use super::*;

#[derive(Clone, Copy)]
struct Destination {
    page: Page,
    code: &'static str,
    title: &'static str,
    subtitle: &'static str,
}

const fn dest(
    page: Page,
    code: &'static str,
    title: &'static str,
    subtitle: &'static str,
) -> Destination {
    Destination {
        page,
        code,
        title,
        subtitle,
    }
}

const GROUPS: &[(&str, &[Destination])] = &[
    (
        "工作台",
        &[
            dest(
                Page::Home,
                "00",
                "总览",
                "按用途选择工具，或直接从常用任务开始。",
            ),
            dest(
                Page::Quick,
                "01",
                "快捷指令",
                "常用游戏规则、模组命令和传送点，点击即可复制。",
            ),
            dest(
                Page::Search,
                "02",
                "物品 ID 查询",
                "按中文名或 ID 查找原版物品，也可以扫描整合包。",
            ),
            dest(
                Page::History,
                "03",
                "指令历史",
                "查找最近使用的指令，复制或导出为游戏函数。",
            ),
        ],
    ),
    (
        "物品与装备",
        &[
            dest(
                Page::Infinity,
                "04",
                "无限元件",
                "手动生成元件，或从预设目录选择多种物品。",
            ),
            dest(
                Page::Batch,
                "05",
                "批量生成",
                "指定物品和数量，生成一条发放指令。",
            ),
            dest(
                Page::Name,
                "06",
                "物品命名",
                "设置物品显示名称、样式和描述文本。",
            ),
            dest(
                Page::Enchant,
                "07",
                "255 附魔",
                "生成已附魔的物品，也可以从预设起步。",
            ),
            dest(
                Page::HandEnch,
                "08",
                "手持附魔",
                "对手上的物品生成附魔指令，无需重新发放。",
            ),
            dest(Page::Gear, "09", "全套装备", "一次生成多件指定材质的装备。"),
            dest(
                Page::Potion,
                "10",
                "药水生成",
                "设置效果、等级、时长和药水形态。",
            ),
        ],
    ),
    (
        "实体与玩家",
        &[
            dest(
                Page::Entity,
                "11",
                "自定义实体",
                "选择原版或模组生物，设置属性和生成数量。",
            ),
            dest(Page::Egg, "12", "刷怪蛋", "生成指定实体的刷怪蛋。"),
            dest(
                Page::ArmorStand,
                "13",
                "盔甲架",
                "配置姿势、外观和显示选项。",
            ),
            dest(Page::Attr, "14", "属性修改", "为目标玩家设置属性的基础值。"),
            dest(Page::Head, "15", "玩家头颅", "输入玩家名称，生成其头颅。"),
        ],
    ),
    (
        "应用管理",
        &[dest(
            Page::Settings,
            "16",
            "设置与清理",
            "检查本机配置，管理备份与历史数据。",
        )],
    ),
];

fn destination(page: Page) -> (&'static str, Destination) {
    GROUPS
        .iter()
        .find_map(|&(group, destinations)| {
            destinations
                .iter()
                .find(|d| d.page == page)
                .copied()
                .map(|d| (group, d))
        })
        .expect("all pages must be in the navigation")
}

pub(super) fn surface(p: &Palette, fill: egui::Color32, inset: f32) -> egui::Frame {
    egui::Frame::none()
        .fill(fill)
        .stroke(stroke(p.border))
        .rounding(10.0)
        .inner_margin(egui::Margin::same(inset))
}

pub(super) fn section<R>(
    ui: &mut egui::Ui,
    p: &Palette,
    title: &str,
    body: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    surface(p, p.card_bg, 16.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            if !title.is_empty() {
                ui.label(egui::RichText::new(title).strong().size(17.0).color(p.text));
                ui.add_space(6.0);
            }
            body(ui)
        })
        .inner
}

pub(super) fn action(p: &Palette, label: &str) -> egui::Button<'static> {
    let ink = if p.page_bg == DARK.page_bg {
        egui::Color32::from_rgb(13, 31, 22)
    } else {
        egui::Color32::WHITE
    };
    egui::Button::new(egui::RichText::new(label.to_owned()).strong().color(ink))
        .fill(p.accent)
        .stroke(egui::Stroke::NONE)
        .min_size(egui::vec2(0.0, 32.0))
}

pub(super) fn field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    hint: &str,
) -> egui::Response {
    ui.label(egui::RichText::new(label).strong().size(13.0));
    ui.add(
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .font(egui::TextStyle::Monospace)
            .desired_width(ui.available_width().min(480.0))
            .margin(egui::vec2(10.0, 8.0)),
    )
}

fn badge(ui: &mut egui::Ui, p: &Palette, text: &str) {
    egui::Frame::none()
        .fill(p.accent_light)
        .rounding(5.0)
        .inner_margin(egui::Margin::symmetric(7.0, 4.0))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text)
                    .monospace()
                    .size(11.0)
                    .color(p.accent_hover),
            );
        });
}

fn mark(ui: &mut egui::Ui, p: &Palette) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
    ui.painter().rect_filled(r, 7.0, p.accent);
    let ink = if p.page_bg == DARK.page_bg {
        p.page_bg
    } else {
        egui::Color32::WHITE
    };
    for (x, y) in [(8.0, 8.0), (17.0, 8.0), (8.0, 17.0)] {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(r.min + egui::vec2(x, y), egui::vec2(7.0, 7.0)),
            1.0,
            ink,
        );
    }
}

impl App {
    pub(super) fn navigate(&mut self, page: Page) {
        if self.page != page {
            self.page = page;
            self.copy_status.clear();
            if page == Page::Settings {
                self.reg_cache = None;
                self.reg_cache_stale = false;
                self.backup_cache = None;
            }
        }
    }

    pub(super) fn render(&mut self, ctx: &egui::Context) {
        let p = self.palette;
        let width = ctx.screen_rect().width();
        let compact = width < 820.0;

        egui::TopBottomPanel::top("workshop_toolbar")
            .frame(
                egui::Frame::none()
                    .fill(p.top_bg)
                    .inner_margin(egui::Margin::symmetric(18.0, 10.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    mark(ui, p);
                    if !compact {
                        ui.label(egui::RichText::new("我的世界小工具").strong().size(16.0));
                        ui.separator();
                    }
                    ui.label(egui::RichText::new("版本").color(p.text_weak));
                    egui::ComboBox::from_id_source("workshop_version")
                        .width(82.0)
                        .selected_text(&self.version)
                        .show_ui(ui, |ui| {
                            for version in VERSIONS {
                                if ui
                                    .selectable_label(self.version == version, version)
                                    .clicked()
                                {
                                    if self.version != version {
                                        self.version = version.to_owned();
                                        self.history_selected.clear();
                                        if !self.search_result.is_empty() {
                                            self.do_search();
                                        }
                                    }
                                }
                            }
                        });
                    let target = self.selector_map(&self.selector);
                    let target_label = SELECTOR_MAP
                        .iter()
                        .find(|(_, id)| *id == target)
                        .map(|(label, _)| *label)
                        .unwrap_or(self.selector.as_str());
                    ui.label(egui::RichText::new("目标").color(p.text_weak));
                    egui::ComboBox::from_id_source("workshop_target")
                        .width(105.0)
                        .selected_text(target_label)
                        .show_ui(ui, |ui| {
                            for (label, id) in SELECTOR_MAP {
                                if ui
                                    .selectable_label(target == id, format!("{label}  {id}"))
                                    .clicked()
                                {
                                    self.selector = label.to_owned();
                                }
                            }
                        });
                    ui.separator();
                    if ui.selectable_label(self.always_top, "窗口置顶").clicked() {
                        self.always_top = !self.always_top;
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                            if self.always_top {
                                egui::WindowLevel::AlwaysOnTop
                            } else {
                                egui::WindowLevel::Normal
                            },
                        ));
                    }
                    if ui
                        .button(if self.dark_mode {
                            "浅色主题"
                        } else {
                            "深色主题"
                        })
                        .clicked()
                    {
                        self.dark_mode = !self.dark_mode;
                        self.palette = if self.dark_mode { &DARK } else { &LIGHT };
                        apply_theme(ctx, self.palette, self.dark_mode);
                    }
                });
                self.render_notices(ui, p);
            });

        let nav_width = if compact { 156.0 } else { 195.0 };
        egui::SidePanel::left(egui::Id::new(("workshop_navigation", compact)))
            .default_width(nav_width)
            .width_range((nav_width - 20.0)..=nav_width)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(p.top_bg)
                    .inner_margin(egui::Margin::symmetric(10.0, 12.0)),
            )
            .show(ctx, |ui| {
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    egui::ScrollArea::vertical()
                        .id_source("workshop_nav_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for &(group, destinations) in GROUPS {
                                ui.add_space(9.0);
                                ui.label(egui::RichText::new(group).size(11.0).color(p.text_weak));
                                for d in destinations {
                                    let selected = self.page == d.page;
                                    let label = if compact {
                                        d.title.to_owned()
                                    } else {
                                        format!("{}   {}", d.code, d.title)
                                    };
                                    let mut button = egui::Button::new(
                                        egui::RichText::new(label).color(if selected {
                                            p.accent_hover
                                        } else {
                                            p.text
                                        }),
                                    )
                                    .stroke(egui::Stroke::NONE)
                                    .frame(selected);
                                    if selected {
                                        button = button.fill(p.accent_light);
                                    }
                                    let row = ui
                                        .with_layout(
                                            egui::Layout::top_down_justified(egui::Align::Min),
                                            |ui| ui.add(button.min_size(egui::vec2(0.0, 25.0))),
                                        )
                                        .inner;
                                    if row.on_hover_text(d.subtitle).clicked() {
                                        self.navigate(d.page);
                                    }
                                }
                            }
                        });
                });
            });

        if self.page.has_output() && width >= 1110.0 {
            egui::SidePanel::right("workshop_console")
                .default_width(292.0)
                .width_range(264.0..=292.0)
                .resizable(false)
                .frame(
                    egui::Frame::none()
                        .fill(p.top_bg)
                        .inner_margin(egui::Margin::symmetric(14.0, 16.0)),
                )
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .id_source("workshop_console_scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| self.render_console(ui, true));
                });
        } else if self.page.has_output() {
            egui::TopBottomPanel::bottom("workshop_console_compact")
                .frame(
                    egui::Frame::none()
                        .fill(p.top_bg)
                        .inner_margin(egui::Margin::symmetric(14.0, 8.0)),
                )
                .show(ctx, |ui| self.render_console(ui, false));
        } else if self.page != Page::Home {
            egui::TopBottomPanel::bottom("workshop_status")
                .frame(
                    egui::Frame::none()
                        .fill(p.top_bg)
                        .inner_margin(egui::Margin::symmetric(14.0, 6.0)),
                )
                .show(ctx, |ui| self.render_result_bar(ui, false));
        }

        let page = self.page;
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(p.page_bg))
            .show(ctx, |ui| {
                // Panel layout: https://docs.rs/egui/0.27.2/egui/containers/panel/index.html
                egui::ScrollArea::vertical()
                    .id_source(("workshop_page", page as u8))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::none()
                            .inner_margin(egui::Margin::symmetric(
                                if compact { 12.0 } else { 22.0 },
                                18.0,
                            ))
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                self.render_page_heading(ui, p);
                                ui.add_space(10.0);
                                if matches!(
                                    page,
                                    Page::Home | Page::Quick | Page::Search | Page::Infinity
                                ) {
                                    self.render_page_body(ui, page);
                                } else {
                                    section(ui, p, "", |ui| self.render_page_body(ui, page));
                                }
                            });
                    });
            });
    }

    fn render_notices(&mut self, ui: &mut egui::Ui, p: &Palette) {
        if let Some(notice) = &self.startup_notice {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(p.status_warn, notice);
            });
            if ui.small_button("知道了").clicked() {
                self.startup_notice = None;
            }
        }
        if let Some(err) = &self.write_error {
            ui.colored_label(p.status_err, format!("{err}。变更暂未保存。"));
            if ui.small_button("关闭提示").clicked() {
                self.write_error = None;
            }
        }
    }

    fn render_page_heading(&self, ui: &mut egui::Ui, p: &Palette) {
        let (group, d) = destination(self.page);
        if self.page == Page::Home {
            surface(p, p.hero_bg, 24.0).show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                badge(ui, p, "MINECRAFT / JAVA EDITION");
                ui.add_space(8.0);
                ui.label(egui::RichText::new("指令工作台").strong().size(34.0));
                ui.label(
                    egui::RichText::new("生成指令、查物品，整理常用命令。")
                        .color(p.text_weak)
                        .size(15.0),
                );
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    badge(ui, p, &format!("JAVA {}", self.version));
                    badge(ui, p, "仅复制 · 不会自动执行");
                });
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                badge(ui, p, &d.code);
                ui.label(egui::RichText::new(group).size(12.0).color(p.text_weak));
            });
            ui.label(egui::RichText::new(d.title).size(27.0).strong());
            ui.label(egui::RichText::new(d.subtitle).color(p.text_weak));
        }
    }

    fn render_page_body(&mut self, ui: &mut egui::Ui, page: Page) {
        // Older form rows can wrap rather than clipping when the result column or window narrows.
        ui.spacing_mut().text_edit_width = (ui.available_width() * 0.46).clamp(84.0, 218.0);
        match page {
            Page::Home => self.render_home(ui),
            Page::Infinity => self.render_infinity(ui),
            Page::Enchant => self.render_enchant(ui),
            Page::HandEnch => self.render_handench(ui),
            Page::Name => self.render_name(ui),
            Page::Potion => self.render_potion(ui),
            Page::Egg => self.render_egg(ui),
            Page::Attr => self.render_attr(ui),
            Page::Head => self.render_head(ui),
            Page::Gear => self.render_gear(ui),
            Page::Quick => self.render_quick(ui),
            Page::History => self.render_history(ui),
            Page::Search => self.render_search(ui),
            Page::Batch => self.render_batch(ui),
            Page::Entity => self.render_entity(ui),
            Page::ArmorStand => self.render_armorstand(ui),
            Page::Settings => self.render_settings(ui),
        }
    }

    fn render_home(&mut self, ui: &mut egui::Ui) {
        let p = self.palette;
        self.render_favorites(ui);
        ui.label(egui::RichText::new("开始使用").size(19.0).strong());
        let columns = if ui.available_width() >= 820.0 {
            3
        } else if ui.available_width() >= 560.0 {
            2
        } else {
            1
        };
        let featured = [
            Page::Quick,
            Page::Search,
            Page::Infinity,
            Page::Enchant,
            Page::Entity,
            Page::History,
        ];
        for row in featured.chunks(columns) {
            ui.columns(columns, |cells| {
                for (cell, &page) in cells.iter_mut().zip(row) {
                    let (_, d) = destination(page);
                    let clicked = section(cell, p, "", |ui| {
                        ui.horizontal(|ui| {
                            badge(ui, p, d.code);
                            ui.label(egui::RichText::new(d.title).strong().size(17.0));
                        });
                        ui.allocate_ui_with_layout(
                            egui::vec2(ui.available_width(), 37.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.label(egui::RichText::new(d.subtitle).color(p.text_weak));
                            },
                        );
                        ui.button("打开工具 ›").clicked()
                    });
                    if clicked {
                        self.navigate(page);
                    }
                }
            });
        }
        section(ui, p, "本机工作区", |ui| {
            ui.horizontal_wrapped(|ui| {
                badge(ui, p, &format!("{} 条历史", self.history.len()));
                badge(ui, p, &format!("{} 条自定义指令", self.custom_cmds.len()));
                badge(ui, p, &format!("{} 个传送点", self.teleports.len()));
                if ui.button("管理配置 ›").clicked() {
                    self.navigate(Page::Settings);
                }
            });
        });
    }

    fn render_console(&mut self, ui: &mut egui::Ui, wide: bool) {
        let p = self.palette;
        ui.horizontal_wrapped(|ui| {
            badge(ui, p, ">_");
            ui.label(
                egui::RichText::new("指令输出")
                    .size(if wide { 19.0 } else { 15.0 })
                    .strong(),
            );
            if let Some(version) = self.result_versions.get(&self.page) {
                badge(ui, p, &format!("生成于 {version}"));
            }
        });
        if wide {
            ui.label(egui::RichText::new("复制后粘贴到游戏中使用").color(p.text_weak));
        }
        ui.push_id(wide, |ui| self.render_result_bar(ui, wide));
    }
}

impl Page {
    pub(super) fn has_output(self) -> bool {
        !matches!(
            self,
            Self::Home | Self::Quick | Self::History | Self::Search | Self::Settings
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_has_unique_entries_and_covers_every_page() {
        let pages: Vec<Page> = GROUPS
            .iter()
            .flat_map(|(_, entries)| entries.iter().map(|entry| entry.page))
            .collect();
        assert_eq!(pages.len(), 17);
        for (i, &page) in pages.iter().enumerate() {
            assert!(!pages[..i].contains(&page));
            assert_eq!(destination(page).1.page, page);
        }
    }

    #[test]
    fn every_page_renders_in_light_and_dark_at_both_window_sizes() {
        for dark_mode in [false, true] {
            let ctx = egui::Context::default();
            let mut app = App::new(
                &ctx,
                Config {
                    dark_mode,
                    ..Config::default()
                },
                None,
            );
            // Settings normally populate these lazily by reading the user's registry.
            // Use in-memory data so this smoke test is deterministic.
            app.startup_checks = Some(Vec::new());
            app.reg_cache = Some(Vec::new());
            app.backup_cache = Some(Vec::new());
            app.gen_batch();
            for size in [egui::vec2(680.0, 560.0), egui::vec2(1240.0, 790.0)] {
                for page in GROUPS
                    .iter()
                    .flat_map(|(_, group)| group.iter().map(|d| d.page))
                {
                    app.page = page;
                    for tab in 0..if page == Page::Quick { 6 } else { 1 } {
                        app.quick_tab = tab;
                        let output = ctx.run(
                            egui::RawInput {
                                screen_rect: Some(egui::Rect::from_min_size(
                                    egui::Pos2::ZERO,
                                    size,
                                )),
                                ..Default::default()
                            },
                            |ctx| app.render(ctx),
                        );
                        assert!(!output.shapes.is_empty(), "empty UI for {page:?}");
                    }
                }
            }
        }
    }
}
