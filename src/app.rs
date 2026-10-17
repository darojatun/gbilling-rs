use crate::db::Database;
use egui::{Color32, RichText};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Stations,
    Tariffs,
    Sessions,
    Reports,
    Settings,
}

pub struct App {
    db: Database,
    active_tab: Tab,
    logged_in: bool,
    username: String,
    password: String,
    customer_name: String,
    selected_tariff_id: i64,
    tariff_name: String,
    tariff_price: String,
    admin_new_password: String,
    admin_confirm_password: String,
    status_message: String,
    dark_mode: bool,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext) -> Self {
        let mut db = Database::new("gbilling.db").expect("failed to initialize DB");
        let theme = db
            .get_setting("theme")
            .unwrap_or(None)
            .unwrap_or_else(|| "dark".to_string());
        let selected_tariff_id = db
            .list_tariffs()
            .ok()
            .and_then(|tariffs| tariffs.first().map(|t| t.id))
            .unwrap_or(1);

        Self {
            db,
            active_tab: Tab::Dashboard,
            logged_in: false,
            username: "admin".to_string(),
            password: "admin123".to_string(),
            customer_name: "Budi".to_string(),
            selected_tariff_id,
            tariff_name: String::new(),
            tariff_price: String::new(),
            admin_new_password: String::new(),
            admin_confirm_password: String::new(),
            status_message: "Aplikasi siap digunakan.".to_string(),
            dark_mode: theme == "dark",
        }
    }

    fn login_view(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(32.0);
                ui.heading("GBilling RS");
                ui.add_space(18.0);
                ui.group(|ui| {
                    ui.set_min_width(320.0);
                    ui.vertical(|ui| {
                        ui.label("Username");
                        ui.text_edit_singleline(&mut self.username);
                        ui.add_space(8.0);
                        ui.label("Password");
                        ui.password(&mut self.password);
                        ui.add_space(12.0);
                        if ui.button("Login").clicked() {
                            match self.db.validate_login(&self.username, &self.password) {
                                Ok(true) => {
                                    self.logged_in = true;
                                    self.status_message = format!("Selamat datang, {}.", self.username);
                                }
                                Ok(false) => {
                                    self.status_message = "Login gagal. Username atau password tidak valid.".to_string();
                                }
                                Err(_) => {
                                    self.status_message = "Terjadi error saat login.".to_string();
                                }
                            }
                        }
                        ui.add_space(8.0);
                        ui.label(RichText::new(&self.status_message).color(Color32::from_rgb(158, 174, 196)));
                    });
                });
            });
        });
    }

    fn dashboard(&mut self, ui: &mut egui::Ui) {
        ui.heading("Dashboard");
        ui.add_space(8.0);

        let total_revenue = self.db.total_revenue().unwrap_or(0.0);
        let active_count = self.db.active_sessions_count().unwrap_or(0);
        let stations = self.db.list_stations().unwrap_or_default();

        egui::Grid::new("dashboard_grid")
            .num_columns(3)
            .spacing([16.0, 12.0])
            .show(ui, |ui| {
                self.stat_card(ui, "Pendapatan", &format!("Rp {:.0}", total_revenue));
                self.stat_card(ui, "Sesi Aktif", &active_count.to_string());
                self.stat_card(ui, "Workstation", &stations.len().to_string());
            });

        ui.add_space(20.0);
        ui.group(|ui| {
            ui.label("Status Sistem");
            ui.separator();
            ui.colored_label(Color32::from_rgb(80, 160, 100), &self.status_message);
        });
    }

    fn stations(&mut self, ui: &mut egui::Ui) {
        ui.heading("Workstation");
        ui.add_space(8.0);

        let tariffs = self.db.list_tariffs().unwrap_or_default();
        if !tariffs.is_empty() && self.selected_tariff_id == 0 {
            self.selected_tariff_id = tariffs[0].id;
        }

        ui.horizontal(|ui| {
            ui.label("Nama pelanggan:");
            ui.text_edit_singleline(&mut self.customer_name);
            if !tariffs.is_empty() {
                let selected_text = tariffs
                    .iter()
                    .find(|t| t.id == self.selected_tariff_id)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| "Pilih Tarif".to_string());

                egui::ComboBox::from_label("Tarif")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        for tariff in &tariffs {
                            ui.selectable_value(&mut self.selected_tariff_id, tariff.id, &tariff.name);
                        }
                    });
            }
        });

        ui.add_space(12.0);

        let stations = self.db.list_stations().unwrap_or_default();
        egui::Grid::new("stations_grid")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .show(ui, |ui| {
                for station in stations {
                    ui.group(|ui| {
                        ui.set_min_width(250.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&station.name).strong());
                            let badge_color = if station.status == "busy" { Color32::from_rgb(210, 80, 80) } else { Color32::from_rgb(80, 170, 100) };
                            ui.colored_label(badge_color, station.status.to_uppercase());
                        });

                        if station.status == "available" {
                            if ui.button("Mulai Sesi").clicked() {
                                if self.db.start_session(station.id, &self.customer_name, self.selected_tariff_id).unwrap_or(false) {
                                    self.status_message = format!("Sesi {} dimulai untuk {}.", station.name, self.customer_name);
                                } else {
                                    self.status_message = format!("Station {} sedang digunakan.", station.name);
                                }
                            }
                        } else {
                            if ui.button("Stop Sesi").clicked() {
                                if let Some(total) = self.db.stop_session(station.id).unwrap_or(None) {
                                    self.status_message = format!("Sesi {} ditutup. Total: Rp {:.0}", station.name, total);
                                } else {
                                    self.status_message = format!("Tidak ada sesi aktif di {}.", station.name);
                                }
                            }
                        }
                    });
                    ui.end_row();
                }
            });
    }

    fn tariffs(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tarif");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Nama");
            ui.text_edit_singleline(&mut self.tariff_name);
            ui.label("Harga / jam");
            ui.text_edit_singleline(&mut self.tariff_price);
            if ui.button("Tambah Tarif").clicked() {
                if !self.tariff_name.trim().is_empty() {
                    if let Ok(price) = self.tariff_price.parse::<f64>() {
                        if self.db.add_tariff(&self.tariff_name, price).is_ok() {
                            self.status_message = format!("Tarif {} ditambahkan.", self.tariff_name);
                            self.tariff_name.clear();
                            self.tariff_price.clear();
                        }
                    }
                }
            }
        });

        ui.add_space(12.0);

        let tariffs = self.db.list_tariffs().unwrap_or_default();
        egui::Grid::new("tariff_grid")
            .num_columns(3)
            .spacing([18.0, 8.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Nama").strong());
                ui.label(RichText::new("Harga / Jam").strong());
                ui.label(RichText::new("Status").strong());
                ui.end_row();

                for tariff in tariffs {
                    ui.label(&tariff.name);
                    ui.label(format!("Rp {:.0}", tariff.price_per_hour));
                    ui.label("Aktif");
                    ui.end_row();
                }
            });
    }

    fn sessions(&mut self, ui: &mut egui::Ui) {
        ui.heading("Riwayat Sesi");
        ui.add_space(8.0);

        if ui.button("Export CSV").clicked() {
            match self.db.export_sessions_csv() {
                Ok(path) => self.status_message = format!("CSV berhasil dibuat: {}", path),
                Err(err) => self.status_message = format!("Gagal export CSV: {}", err),
            }
        }

        if ui.button("Backup Database").clicked() {
            match self.db.backup_db() {
                Ok(path) => self.status_message = format!("Backup database dibuat: {}", path),
                Err(err) => self.status_message = format!("Gagal backup: {}", err),
            }
        }

        ui.add_space(12.0);

        let sessions = self.db.recent_sessions().unwrap_or_default();
        egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
            egui::Grid::new("session_grid")
                .num_columns(6)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Pelanggan").strong());
                    ui.label(RichText::new("PC").strong());
                    ui.label(RichText::new("Mulai").strong());
                    ui.label(RichText::new("Selesai").strong());
                    ui.label(RichText::new("Total").strong());
                    ui.label(RichText::new("Aksi").strong());
                    ui.end_row();

                    for session in sessions {
                        ui.label(&session.customer_name);
                        ui.label(format!("PC-{:02}", session.station_id));
                        ui.label(&session.started_at[..10]);
                        ui.label(session.ended_at.clone().unwrap_or_else(|| "-".to_string()));
                        ui.label(format!("Rp {:.0}", session.total_amount));
                        if ui.button("Receipt").clicked() {
                            match self.db.create_receipt(session.id) {
                                Ok(path) if !path.is_empty() => self.status_message = format!("Receipt dibuat: {}", path),
                                Ok(_) => self.status_message = "Receipt gagal dibuat.".to_string(),
                                Err(err) => self.status_message = format!("Gagal membuat receipt: {}", err),
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    }

    fn reports(&mut self, ui: &mut egui::Ui) {
        ui.heading("Laporan");
        ui.add_space(8.0);

        let revenue = self.db.total_revenue().unwrap_or(0.0);
        let active = self.db.active_sessions_count().unwrap_or(0);
        let closed = self.db.recent_sessions().unwrap_or_default().len();

        ui.group(|ui| {
            ui.label("Ringkasan");
            ui.separator();
            ui.label(format!("Pendapatan total: Rp {:.0}", revenue));
            ui.label(format!("Sesi aktif: {}", active));
            ui.label(format!("Jumlah transaksi terdaftar: {}", closed));
        });

        ui.add_space(12.0);
        if ui.button("Export CSV").clicked() {
            match self.db.export_sessions_csv() {
                Ok(path) => self.status_message = format!("Laporan CSV berhasil dibuat: {}", path),
                Err(err) => self.status_message = format!("Gagal export laporan: {}", err),
            }
        }

        if ui.button("Restore Backup Terbaru").clicked() {
            match self.db.restore_latest_backup() {
                Ok(path) if !path.is_empty() => self.status_message = format!("Backup dipulihkan dari: {}", path),
                Ok(_) => self.status_message = "Tidak ada backup yang tersedia.".to_string(),
                Err(err) => self.status_message = format!("Gagal restore backup: {}", err),
            }
        }
    }

    fn settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Pengaturan");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Tema:");
            if ui.button("Dark").clicked() {
                self.dark_mode = true;
                if self.db.set_setting("theme", "dark").is_ok() {
                    self.status_message = "Tema diubah ke dark.".to_string();
                }
            }
            if ui.button("Light").clicked() {
                self.dark_mode = false;
                if self.db.set_setting("theme", "light").is_ok() {
                    self.status_message = "Tema diubah ke light.".to_string();
                }
            }
        });

        ui.add_space(12.0);
        ui.group(|ui| {
            ui.label("Ubah Password Admin");
            ui.horizontal(|ui| {
                ui.label("Password baru");
                ui.password(&mut self.admin_new_password);
            });
            ui.horizontal(|ui| {
                ui.label("Konfirmasi");
                ui.password(&mut self.admin_confirm_password);
            });
            if ui.button("Simpan Password").clicked() {
                if self.admin_new_password == self.admin_confirm_password && !self.admin_new_password.trim().is_empty() {
                    if self.db.set_password(&self.username, &self.admin_new_password).is_ok() {
                        self.status_message = "Password admin berhasil diubah.".to_string();
                        self.admin_new_password.clear();
                        self.admin_confirm_password.clear();
                    }
                } else {
                    self.status_message = "Password tidak valid atau konfirmasi tidak cocok.".to_string();
                }
            }
        });

        ui.add_space(12.0);
        ui.label("Bahasa: Bahasa Indonesia");
    }

    fn stat_card(&self, ui: &mut egui::Ui, label: &str, value: &str) {
        ui.group(|ui| {
            ui.set_min_height(80.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(label).color(Color32::from_rgb(140, 140, 140)).small());
                ui.add_space(6.0);
                ui.label(RichText::new(value).size(26.0).strong());
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_theme(if self.dark_mode { egui::Theme::Dark } else { egui::Theme::Light });

        if !self.logged_in {
            self.login_view(ctx);
            return;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(self.active_tab == Tab::Dashboard, "Dashboard").clicked() { self.active_tab = Tab::Dashboard; }
                if ui.selectable_label(self.active_tab == Tab::Stations, "Workstation").clicked() { self.active_tab = Tab::Stations; }
                if ui.selectable_label(self.active_tab == Tab::Tariffs, "Tarif").clicked() { self.active_tab = Tab::Tariffs; }
                if ui.selectable_label(self.active_tab == Tab::Sessions, "Sesi").clicked() { self.active_tab = Tab::Sessions; }
                if ui.selectable_label(self.active_tab == Tab::Reports, "Laporan").clicked() { self.active_tab = Tab::Reports; }
                if ui.selectable_label(self.active_tab == Tab::Settings, "Pengaturan").clicked() { self.active_tab = Tab::Settings; }
            });

            ui.separator();
            ui.label(RichText::new(&self.status_message).small());
            ui.add_space(8.0);

            match self.active_tab {
                Tab::Dashboard => self.dashboard(ui),
                Tab::Stations => self.stations(ui),
                Tab::Tariffs => self.tariffs(ui),
                Tab::Sessions => self.sessions(ui),
                Tab::Reports => self.reports(ui),
                Tab::Settings => self.settings(ui),
            }
        });
    }
}
