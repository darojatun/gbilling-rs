use crate::db::Database;
use egui::{Align, Color32, RichText, Slider};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Stations,
    Tariffs,
    Sessions,
    Reports,
}

pub struct App {
    db: Database,
    active_tab: Tab,
    customer_name: String,
    tariff_name: String,
    tariff_price: String,
    status_message: String,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let _ = cc;
        let db = Database::new("gbilling.db").expect("Failed to initialize SQLite database");

        Self {
            db,
            active_tab: Tab::Dashboard,
            customer_name: String::from("Budi"),
            tariff_name: String::new(),
            tariff_price: String::new(),
            status_message: String::from("Aplikasi siap digunakan."),
        }
    }

    fn show_dashboard(&mut self, ui: &mut egui::Ui) {
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

        ui.add_space(18.0);
        ui.label("Status sistem");
        ui.colored_label(Color32::from_rgb(86, 139, 91), &self.status_message);
    }

    fn show_stations(&mut self, ui: &mut egui::Ui) {
        ui.heading("Workstation");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Nama pelanggan:");
            ui.text_edit_singleline(&mut self.customer_name);
        });

        ui.add_space(12.0);

        let stations = self.db.list_stations().unwrap_or_default();
        egui::Grid::new("stations_grid")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .show(ui, |ui| {
                for station in stations {
                    ui.group(|ui| {
                        ui.set_min_width(220.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&station.name).strong());
                            let badge = if station.status == "busy" {
                                Color32::from_rgb(208, 72, 72)
                            } else {
                                Color32::from_rgb(64, 150, 84)
                            };
                            ui.colored_label(badge, station.status.to_uppercase());
                        });

                        if station.status == "available" {
                            if ui.button("Mulai Sesi").clicked() {
                                if let Ok(_) = self.db.start_session(station.id, &self.customer_name) {
                                    self.status_message = format!("Sesi {} dimulai untuk {}.", station.name, self.customer_name);
                                } else {
                                    self.status_message = "Gagal memulai sesi.".to_string();
                                }
                            }
                        } else {
                            if ui.button("Stop Sesi").clicked() {
                                if let Ok(total) = self.db.stop_session(station.id) {
                                    self.status_message = format!("Sesi {} ditutup. Total: Rp {:.0}", station.name, total);
                                } else {
                                    self.status_message = "Gagal menutup sesi.".to_string();
                                }
                            }
                        }
                    });
                    ui.end_row();
                }
            });
    }

    fn show_tariffs(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tarif");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Nama");
            ui.text_edit_singleline(&mut self.tariff_name);
            ui.label("Harga/jam");
            ui.text_edit_singleline(&mut self.tariff_price);
            if ui.button("Tambah Tarif").clicked() {
                if let Ok(price) = self.tariff_price.parse::<f64>() {
                    if !self.tariff_name.trim().is_empty() {
                        if let Ok(_) = self.db.add_tariff(&self.tariff_name, price) {
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
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Nama").strong());
                ui.label(RichText::new("Harga/Jam").strong());
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

    fn show_sessions(&mut self, ui: &mut egui::Ui) {
        ui.heading("Riwayat Sesi");
        ui.add_space(8.0);

        let sessions = self.db.recent_sessions().unwrap_or_default();
        egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
            egui::Grid::new("session_grid")
                .num_columns(5)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Pelanggan").strong());
                    ui.label(RichText::new("Lokasi").strong());
                    ui.label(RichText::new("Mulai").strong());
                    ui.label(RichText::new("Total").strong());
                    ui.label(RichText::new("Status").strong());
                    ui.end_row();

                    for session in sessions {
                        ui.label(&session.customer_name);
                        ui.label(format!("PC-{:02}", session.station_id));
                        ui.label(&session.started_at[..10]);
                        ui.label(format!("Rp {:.0}", session.total_amount));
                        ui.label(&session.status);
                        ui.end_row();
                    }
                });
        });
    }

    fn show_reports(&mut self, ui: &mut egui::Ui) {
        ui.heading("Laporan");
        ui.add_space(8.0);

        let revenue = self.db.total_revenue().unwrap_or(0.0);
        let active = self.db.active_sessions_count().unwrap_or(0);

        ui.group(|ui| {
            ui.label("Ringkasan laporan");
            ui.separator();
            ui.label(format!("Pendapatan total: Rp {:.0}", revenue));
            ui.label(format!("Sesi aktif: {}", active));
            ui.label("Catatan: modul laporan akan dikembangkan secara lebih detail pada versi berikutnya.");
        });
    }

    fn stat_card(&self, ui: &mut egui::Ui, label: &str, value: &str) {
        ui.group(|ui| {
            ui.set_min_height(80.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(label).color(Color32::from_rgb(160, 160, 160)).small());
                ui.add_space(6.0);
                ui.label(RichText::new(value).size(26.0).strong());
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(self.active_tab == Tab::Dashboard, "Dashboard").clicked() {
                    self.active_tab = Tab::Dashboard;
                }
                if ui.selectable_label(self.active_tab == Tab::Stations, "Workstation").clicked() {
                    self.active_tab = Tab::Stations;
                }
                if ui.selectable_label(self.active_tab == Tab::Tariffs, "Tarif").clicked() {
                    self.active_tab = Tab::Tariffs;
                }
                if ui.selectable_label(self.active_tab == Tab::Sessions, "Sesi").clicked() {
                    self.active_tab = Tab::Sessions;
                }
                if ui.selectable_label(self.active_tab == Tab::Reports, "Laporan").clicked() {
                    self.active_tab = Tab::Reports;
                }
            });

            ui.separator();

            match self.active_tab {
                Tab::Dashboard => self.show_dashboard(ui),
                Tab::Stations => self.show_stations(ui),
                Tab::Tariffs => self.show_tariffs(ui),
                Tab::Sessions => self.show_sessions(ui),
                Tab::Reports => self.show_reports(ui),
            }
        });
    }
}
