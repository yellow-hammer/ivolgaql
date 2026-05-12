# ИволгаQL

[![Лицензия MIT](https://img.shields.io/badge/лицензия-MIT-blue.svg)](LICENSE)
[![чат Telegram](https://raw.githubusercontent.com/yellow-hammer/ivolgaql/main/resources/badges/telegram-chat.png)](https://t.me/wonder_yellow)
[![DeepWiki](https://raw.githubusercontent.com/yellow-hammer/ivolgaql/main/resources/badges/deepwiki-badge.png)](https://deepwiki.com/yellow-hammer/ivolgaql)

🐦 **ИволгаQL** — по смыслу **игрушечная / учебная СУБД в памяти** с русским **SQL-подобным** синтаксисом (`ВЫБРАТЬ … ИЗ … ГДЕ …`): по TCP уходит **одна строка** запроса, приходит **одна строка** JSON (`ok`, данные, **`bird`**). Это **не** PostgreSQL/MySQL по протоколу и по языку — своё подмножество команд.

**Зачем:** разобраться на практике с лексером/парсером и простым протоколом «текст ↔ JSON» поверх TCP; быстро накидать демо «запросы на русском к таблицам»; подцепить из скриптов без драйвера БД. Для реальной нагрузки и надёжного хранения данных не предназначен.

## Старт

Нужен **Rust**: [rustup](https://rustup.rs) или `winget install Rustlang.Rustup`.

- **`cargo run`** — слушает **`127.0.0.1:15432`**, поднимает демо-таблицу **`склад`**.
- **`cargo run -- client -q "выбрать * из склад"`** — проверка (сервер должен быть запущен в другом терминале): в ответе `"ok": true` и `rows`.
- **`cargo run -- client`** — интерактив построчно; **`помощь`** — полный синтаксис.

```text
выбрать * из склад
выбрать товар, цена из склад где цена >= 89
создать таблицу демо (id, имя)
вставить в демо значения (1, 'Тест')
```

## Для разработчиков

- [CONTRIBUTING.md](CONTRIBUTING.md) — вклад в проект, сборка, PR и issues.
- [`.cursor/rules/`](.cursor/rules/) — правила для агента в Cursor.

## Лицензия

MIT. Подробности см. в файле [LICENSE](LICENSE).

## Автор

Ivan Karlo ([i.karlo@outlook.com](mailto:i.karlo@outlook.com))

При желании, отблагодарить автора можно по ссылке:

- [Boosty](https://boosty.to/1carlo/donate)
- [Чаевые](https://pay.cloudtips.ru/p/d752cb43)
