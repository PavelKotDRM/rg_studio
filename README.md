# rg studio

Кроссплатформенный визуальный конструктор команд для
[ripgrep](https://github.com/BurntSushi/ripgrep), написанный на Rust и egui.
Приложение формирует команду в реальном времени и копирует её в буфер обмена.

## Возможности

- Поля основного поискового шаблона и пути.
- Визуальный regex-конструктор с литералами, классами, группами, альтернативами,
  якорями и квантификаторами.
- Проверка корректности собранного выражения в реальном времени.
- 95+ параметров ripgrep 15.2.0, распределённых по четырём вкладкам.
- Фильтр по названию и имени флага.
- Дополнительные аргументы для повторяемых и появившихся в новых версиях `rg` опций.
- Корректное экранирование шаблонов, путей и значений для текущей ОС.
- Светлая и тёмная темы.
- Окно About с версией, Git SHA, временем сборки, Rust и target от vergen 10.

## Требования и запуск

Установите [Rustup](https://rustup.rs/) и `ripgrep`, затем выполните:

```powershell
rustup update stable
cargo run --release
```

Проект использует stable toolchain, заданный в `rust-toolchain.toml`. Сам `rg`
нужен только для выполнения сгенерированной команды, приложение его не запускает.

## Progress Tracker



## Quality Assurance

Перед изменениями обновите stable toolchain и установите аудит при необходимости:

```powershell
rustup update stable
cargo install cargo-audit --locked
```

Полный локальный цикл проверки:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo doc --no-deps
cargo audit
```

Публичные элементы документируются через `///`; предупреждения Clippy считаются
ошибками. `cargo audit` проверяет зафиксированное дерево из `Cargo.lock` по RustSec.

## Технологии

- Rust 1.98.0 stable, edition 2024.
- egui и eframe 0.36.1.
- vergen и vergen-git2 10.0.2.
- ripgrep 15.2.0 использован как источник актуального каталога опций.

## Лицензия

[MIT](LICENSE)
