# 1681

Open source шутер от первого лица, вдохновлённый ULTRAKILL. Ты — бракованное изделие подземного завода, который десятилетиями работает без людей. Скорость, агрессия, стиль, ретро-графика.

> Проект на ранней стадии. `ultrafun` — внутреннее рабочее имя репозитория.

## Сборка

Нужен [Rust](https://rustup.rs); нужная версия поставится сама из `rust-toolchain.toml`.

```bash
cargo run
```

На Linux нужны системные библиотеки: `libasound2-dev libudev-dev libxkbcommon-dev libwayland-dev` (Debian/Ubuntu).

## Для разработчиков и агентов

Начинать с [AGENTS.md](AGENTS.md).

## Лицензия

- Код — [GPL-3.0-or-later](LICENSE).
- Ассеты (модели, текстуры, звуки, музыка, уровни) — [CC BY-SA 4.0](LICENSE-ASSETS). Авторы сторонних ассетов — в [assets/CREDITS.md](assets/CREDITS.md).

ULTRAKILL — игра Arsi «Hakita» Patala и New Blood Interactive. Этот проект с ними не связан.
