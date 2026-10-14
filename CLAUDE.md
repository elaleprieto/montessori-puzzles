# montessori-puzzles

App de escritorio en Rust (egui) que convierte una Ilustración en un rompecabezas Montessori exportable a SVG para corte láser.

- Leé `GLOSSARY.md` antes de tocar código: los nombres de tipos, funciones y docs usan ese vocabulario, en español (sin tildes en identificadores: `pestana`, `mascara`).
- Decisiones de arquitectura en `docs/adr/`; ideas fuera de alcance en `docs/roadmap.md`.
- El núcleo (`src/lib.rs` y sus módulos) no depende de la UI y se testea sin ventana. Tests unitarios en `#[cfg(test)] mod tests` del mismo módulo, con figuras sintéticas generadas en código.
- Chequeo completo antes de commitear: `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`.
- Commit messages: `<type>: <resumen>` en español, sin trailers de atribución.
