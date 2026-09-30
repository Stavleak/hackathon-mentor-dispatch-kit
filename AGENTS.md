<p><a href="https://stavleak.com/ru"><picture><source media="(prefers-color-scheme: dark)" srcset="assets/stavleak-logo-dark.svg"><img src="assets/stavleak-logo.svg" alt="Stavleak" width="180"></picture></a></p>

# Поддержка приложения Stavleak

Цель репозитория: Менторские запросы теряются, очередь непрозрачна, консультации заканчиваются без следующего шага.

Сначала прочитайте README, CONTRIBUTING, docs/application.md и docs/local-operation.md. Проверьте git status; сохраняйте работу других людей. Предметный модуль Rust находится в backend/src/domain.rs, синхронные типы в app/domain.ts, рабочие формы в app/workbench.tsx. Проверяйте серверные правила близко к каждой записи; браузерная проверка не является авторизацией.

- Backend — Rust, frontend — Next.js. Перед изменением Next.js API читайте установленную документацию node_modules/next/dist/docs. Используйте pnpm 10.33.0 и существующие lockfiles.
- Не помещайте серверные ключи в браузер, NEXT_PUBLIC_ переменные, репозиторий или ответы API. Сохраняйте loopback-привязку, проверку сессии, Origin, версий, размера и предметного жизненного цикла.
- Не ослабляйте неизменяемость закрытых записей или проверку свидетельств ради демо. Новые примеры помечайте вымышленными; начальная среда должна оставаться пустой.
- Данные реального события и контакты храните приватно. Не собирайте лишние персональные данные и не публикуйте их в Issues или screenshots.
- Для интерфейса изучайте https://stavleak.com/ru и https://www.instagram.com/stavleak/. Сохраняйте точный логотип, ссылку на сайт, чёрный/белый/красный #ff2200, Inter, доступные названия полей и состояния. Используйте общие компоненты app/ui.tsx.
- Не обещайте автоматическую отправку сообщений, проверку личности или интеграцию, которых нет в коде. Не придумывайте клиентские результаты и партнёрства.
- Перед завершением запускайте pnpm check, pnpm build и pnpm audit. Проверяйте изменённый рабочий путь в браузере, конфликт двух вкладок и связанные организационные примеры. Не называйте невыполненную проверку успешной.
- Генерируемые .next, next-env.d.ts, tsbuildinfo, target и рабочие data/ не редактируйте и не коммитьте. Для большого изменения добавляйте предметные тесты, а не копии реализации.

<!-- stavleak:brand-footer -->
[Stavleak](https://stavleak.com/ru) · [Материалы организатора](https://org.stavleak.com/ru#toolkit)

<!-- BEGIN:nextjs-agent-rules -->

# This is NOT the Next.js you know

This version has breaking changes — APIs, conventions, and file structure may all differ from your training data. Read the relevant guide in `node_modules/next/dist/docs/` (resolved from this file's directory; in monorepos the `next` package may not be visible from the repo root) before writing any code. Heed deprecation notices.

This block is written and re-added by `next dev` — verify at `node_modules/next/dist/server/lib/generate-agent-files.js`. Removing it from a diff only re-creates the uncommitted change; committing it with your work keeps the tree clean.

<!-- END:nextjs-agent-rules -->
