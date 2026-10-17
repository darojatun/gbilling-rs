# GBilling RS

Aplikasi warnet billing desktop yang ditulis dengan Rust dan framework egui. Proyek ini adalah versi modern dari konsep GBilling yang lebih mudah dibangun di Linux/macOS/Windows dibandingkan GTK2 legacy.

## Fitur awal

- Dashboard ringkasan pendapatan dan sesi aktif
- Manajemen workstation / komputer warnet
- Start/stop session otomatis untuk pelanggan
- Tarif per jam dan daftar tarif
- Riwayat sesi dan laporan sederhana
- Database SQLite lokal

## Stack

- Rust
- egui / eframe
- rusqlite
- SQLite

## Jalankan

```bash
cargo run --release
```

## Build untuk release

```bash
cargo build --release
```

## Lisensi

Proyek ini dilisensikan di bawah [GPL-3.0](LICENSE).
